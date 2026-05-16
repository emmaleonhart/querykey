//! Canonical markdown source of truth — implements
//! `docs/markdown-schema.md`.
//!
//! Markdown files on disk ARE the store of record. The Loca graph is a
//! derived, rebuildable index *projected from* this vault, never the
//! other way round. Layout (R15 onward):
//!
//! ```text
//! <root>/                       (vault root — contains `querykey.toml`)
//!   card.md                     your broadcast card (tracked)
//!   agents.md                   optional, governs the local agent
//!   peers/                      others' cards (READ-ONLY, git-ignored)
//!   .querykey/                  derived cache + propagation state (ignored)
//!   wiki/                       the graph subtree
//!     people/   tasks/   events/   notes/
//!     conflicts/ questions/ followups/
//!     instructions/ voiceprofiles/
//! ```
//!
//! **Back-compat:** vaults that pre-date R15 keep their entity dirs at
//! `<root>/<entity>/` (no `wiki/`). Reads union `wiki/<entity>/` and
//! `<entity>/`; writes go to `wiki/<entity>/` and *migrate-on-write* —
//! after a successful write the legacy `<root>/<entity>/<slug>.md` is
//! removed so the two paths never diverge. `card.md`, `agents.md`,
//! `peers/`, `.querykey/`, and the privacy `.gitignore` stay at the
//! vault root (they are not graph entities).
//!
//! Each file is `--- <yaml frontmatter> ---` + a freeform markdown
//! body. Round-trips are lossless: every model field is either in the
//! frontmatter (stable key order via the struct definition) or is the
//! body (the human description). Editable by hand in any editor.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{
    Conflict, ConflictResolution, ConflictType, DeliveryAttempt, Event, FollowUp, FollowUpStatus,
    Handle, Instruction, OpenQuestion, Person, QuestionStatus, Task, TaskStatus, TriggerType,
    Urgency, VoiceProfile,
};

pub struct Vault {
    root: PathBuf,
}

// ---------- frontmatter structs (key order = field order) ----------

#[derive(Serialize, Deserialize)]
struct PersonFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    display_name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    handles: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    role: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    contact_cascade: Vec<String>,
    created: String,
}

#[derive(Serialize, Deserialize)]
struct TaskFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    title: String,
    status: TaskStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    person: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    assigned_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deadline: Option<String>,
    confidence: f64,
    ambiguity_score: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    source: Vec<String>,
    created: String,
    updated: String,
}

#[derive(Serialize, Deserialize)]
struct EventFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    title: String,
    start: String,
    end: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    people: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recurrence: Option<String>,
    confidence: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    source: Vec<String>,
    created: String,
}

#[derive(Serialize, Deserialize)]
struct ConflictFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    conflict_type: ConflictType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    message_a: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    message_b: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    task: String,
    resolution: ConflictResolution,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    resolved_by: String,
    created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolved: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OpenQuestionFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    target: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    context: String,
    urgency: Urgency,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    urgency_deadline: Option<String>,
    trigger_type: TriggerType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    trigger_id: String,
    status: QuestionStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    resolution: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    resolved_by: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    resolved_via: String,
    created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolved: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct FollowUpFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    trigger_type: TriggerType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    trigger_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    target: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    context: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    delivery_attempts: Vec<DeliveryAttempt>,
    status: FollowUpStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    response: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    response_channel: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    response_at: Option<String>,
    created: String,
}

#[derive(Serialize, Deserialize)]
struct InstructionFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    speaker: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    audience: Vec<String>,
    is_task: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    task: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    source_message: String,
    created: String,
}

#[derive(Serialize, Deserialize)]
struct VoiceProfileFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    person: String,
    // Raw embedding bytes as a YAML int sequence — lossless, no extra
    // dep, and omitted entirely until the audio pipeline fills it
    // (the common case today). Not meant for hand-editing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    embedding: Vec<u8>,
    sample_count: i64,
    confidence: f64,
    last_updated: String,
    created: String,
}

// ---------- frontmatter (de)serialization ----------

pub(crate) fn compose(yaml: &str, body: &str) -> String {
    format!("---\n{}---\n\n{}\n", yaml, body.trim_end())
}

/// Split `--- yaml --- body`. Returns (yaml, body). If there is no
/// frontmatter the whole input is treated as body.
pub(crate) fn split(content: &str) -> (String, String) {
    let c = content.strip_prefix('\u{feff}').unwrap_or(content);
    if let Some(rest) = c.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            let yaml = rest[..end + 1].to_string();
            // Body is canonicalized (leading blank lines + trailing
            // whitespace trimmed) so write→read is idempotent and the
            // model's description round-trips losslessly. `compose`
            // trims the end on write to match.
            let body = rest[end + 4..]
                .trim_start_matches(['\n', '\r'])
                .trim_end()
                .to_string();
            return (yaml, body);
        }
    }
    (String::new(), c.trim_end().to_string())
}

pub(crate) fn rfc3339(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}
pub(crate) fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_else(Utc::now))
}

/// A derived relationship edge extracted from a `[[wikilink]]` in an
/// entity body. The graph is built from these; the markdown is canon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkEdge {
    pub from_kind: String,
    pub from_id: String,
    pub predicate: String,
    pub to_kind: String,
    pub to_id: String,
    /// The target exactly as written (useful when `resolved` is false).
    pub to_label: String,
    pub resolved: bool,
}

/// A compact, model-agnostic summary of the PRM the local agent
/// attends over when drafting a card. Deliberately small: counts +
/// the relations that actually matter, not a dump of the graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrmDigest {
    pub person_count: usize,
    pub task_count: usize,
    pub event_count: usize,
    pub note_count: usize,
    /// People you reference most (resolved inbound wikilinks).
    pub top_people: Vec<DigestPerson>,
    /// Distinct wikilink predicates in use — your relation vocabulary.
    pub predicates: Vec<String>,
    /// Targets linked with an "offering" predicate (offers/teaches/…)
    /// — explicit key signal harvested from the graph.
    pub offers: Vec<String>,
    /// Targets linked with a "looking-for" predicate
    /// (wants/needs/seeking/…) — explicit query signal.
    pub wants: Vec<String>,
    /// Titles of not-`done` tasks (capped).
    pub active_tasks: Vec<String>,
    /// Your current card, so a draft refines rather than resets it.
    pub current_card: Option<crate::card::Card>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DigestPerson {
    pub id: String,
    pub display_name: String,
    pub role: String,
    pub mentions: usize,
}

/// One row of the merged agenda: a fixed event occurrence or a
/// time-flexible task that has a deadline in the window. `movable`
/// encodes the Task-vs-Event distinction ("if you can move it without
/// asking anyone, it's a task").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgendaItem {
    pub kind: String, // "event" | "task"
    pub id: String,
    pub title: String,
    pub start: String,        // rfc3339 (event occurrence start | task deadline)
    pub end: Option<String>,  // events only
    pub movable: bool,        // task → true, event → false
    pub recurring: bool,      // this row is one occurrence of a rule
}

/// Resolution/comparison slug: lowercase, non-alphanumeric runs become
/// a single `-`, trimmed. So `John  Smith`, `john-smith`, and
/// `John-Smith` all compare equal.
/// Map an API-side entity key to its canonical on-disk dir name.
/// Most entities use the same name in both forms; "people" is the
/// exception — it lives at `wiki/contacts/` on disk (R15-3, the
/// user's term). Code outside this module still says "people".
fn canonical_dir_name(sub: &str) -> &str {
    match sub {
        "people" => "contacts",
        other => other,
    }
}

/// Read every `*.md` in `dir` into `out` keyed by file stem. Later
/// calls overwrite earlier ones for the same slug — `read_files` uses
/// this so the canonical dir read *last* wins when a migration-in-
/// progress slug exists across multiple paths.
fn read_md_into(dir: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let Some(slug) = p.file_stem().and_then(|x| x.to_str()) else {
            continue;
        };
        if let Ok(s) = fs::read_to_string(&p) {
            out.insert(slug.to_string(), s);
        }
    }
}

pub(crate) fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

// ---------- Vault ----------

// Graph entity subdirs. These move under `<root>/wiki/` in R15 (with a
// back-compat read fallback at `<root>/<entity>/`). Defined in one
// place so the resolver helpers stay honest.
const ENTITY_SUBDIRS: &[&str] = &[
    "people",
    "tasks",
    "events",
    "notes",
    "conflicts",
    "questions",
    "followups",
    "instructions",
    "voiceprofiles",
];

// Dirs that stay AT the vault root (not graph entities).
const ROOT_ONLY_SUBDIRS: &[&str] = &[
    "peers",     // others' cards — git-ignored (asymmetry)
    ".querykey", // derived cache / propagation state — git-ignored
];

impl Vault {
    pub fn open(root: &str) -> anyhow::Result<Self> {
        let root = PathBuf::from(root);
        // Graph entity dirs live under <root>/wiki/<canonical_name>/
        // from R15 onward. Some subs have a canonical on-disk name
        // different from the API key (e.g. "people" → "contacts").
        for sub in ENTITY_SUBDIRS {
            fs::create_dir_all(root.join("wiki").join(canonical_dir_name(sub)))?;
        }
        for sub in ROOT_ONLY_SUBDIRS {
            fs::create_dir_all(root.join(sub))?;
        }
        let v = Self { root };
        v.ensure_privacy_gitignore()?;
        Ok(v)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    // ----- entity dir resolution (R15) -----
    //
    // Writes go to `<root>/wiki/<canonical_name>/<slug>.md` — and for
    // people that canonical name is `contacts/` (R15-3, the user's
    // term). Reads check the canonical dir first, then *every* legacy
    // dir in turn: that's `<root>/wiki/people/` (R15-2 intermediate
    // form) and `<root>/people/` (pre-R15). Same logic for every
    // other entity — the only one with a renamed canonical dir
    // currently is "people" → "contacts"; all others use the same
    // name in both API and on-disk forms, so their "legacy dirs" is
    // just the pre-R15 root path.
    //
    // `migrate_legacy_on_write` removes the slug from *all* legacy
    // dirs after a successful write to the canonical path, so the
    // copies cannot silently diverge.

    /// Canonical (write) dir for a graph entity subdir.
    fn entity_dir(&self, sub: &str) -> PathBuf {
        self.root.join("wiki").join(canonical_dir_name(sub))
    }

    /// Every legacy dir to check on a read for this sub, in fall-back
    /// order. Empty if the sub's canonical dir is also its only dir.
    fn legacy_entity_dirs(&self, sub: &str) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        // R15-2 intermediate form: wiki/<api-key>/ when the canonical
        // name differs (i.e. people → contacts, contacts has a prior
        // form of `wiki/people/`).
        if canonical_dir_name(sub) != sub {
            out.push(self.root.join("wiki").join(sub));
        }
        // Pre-R15 form: <root>/<api-key>/
        out.push(self.root.join(sub));
        out
    }

    /// Locate a single entity file by slug, preferring the canonical
    /// path and falling back to each legacy path in order. Returns
    /// `None` if no copy exists.
    fn find_entity_file(&self, sub: &str, slug: &str) -> Option<PathBuf> {
        let p = self.entity_dir(sub).join(format!("{slug}.md"));
        if p.is_file() {
            return Some(p);
        }
        for legacy in self.legacy_entity_dirs(sub) {
            let lp = legacy.join(format!("{slug}.md"));
            if lp.is_file() {
                return Some(lp);
            }
        }
        None
    }

    /// After a successful write to the canonical dir, remove the same
    /// slug from every legacy dir. Idempotent and non-fatal — a stale
    /// legacy file is bad data, but failing to remove one is not
    /// worth surfacing as an error to the caller.
    fn migrate_legacy_on_write(&self, sub: &str, slug: &str) {
        for legacy in self.legacy_entity_dirs(sub) {
            let lp = legacy.join(format!("{slug}.md"));
            if lp.is_file() {
                let _ = fs::remove_file(lp);
            }
        }
    }

    // ----- privacy asymmetry (docs/card-format.md) -----
    //
    // Load-bearing: your own card.md is git-TRACKED (you need history
    // to revert a bad edit); other people's cards and the derived
    // cache are git-IGNORED (no surveillance archive of how someone's
    // card changed over time). Enforced by writing a `.gitignore` into
    // the vault root so the asymmetry holds even if QueryKey isn't run.

    const PRIVACY_MARK: &'static str = "# QueryKey vault privacy asymmetry";

    fn ensure_privacy_gitignore(&self) -> anyhow::Result<()> {
        let block = format!(
            "{mark} — see docs/card-format.md.\n\
             # Your own card.md is TRACKED (history = undo). Other\n\
             # people's cards and the derived cache are NOT.\n\
             /peers/\n\
             /.querykey/\n",
            mark = Self::PRIVACY_MARK,
        );
        let path = self.root.join(".gitignore");
        match fs::read_to_string(&path) {
            Ok(existing) if existing.contains(Self::PRIVACY_MARK) => {} // idempotent
            Ok(existing) => {
                let sep = if existing.ends_with('\n') { "\n" } else { "\n\n" };
                fs::write(&path, format!("{existing}{sep}{block}"))?;
            }
            Err(_) => fs::write(&path, block)?,
        }
        Ok(())
    }

    // ----- your broadcast card -----

    pub fn card_path(&self) -> PathBuf {
        self.root.join("card.md")
    }

    pub fn get_card(&self) -> Option<crate::card::Card> {
        let s = fs::read_to_string(self.card_path()).ok()?;
        crate::card::parse(&s)
    }

    pub fn upsert_card(&self, c: &crate::card::Card) -> anyhow::Result<()> {
        fs::write(self.card_path(), crate::card::render(c))?;
        Ok(())
    }

    // ----- 24h propagation delay (the privacy safety valve) -----
    //
    // docs/card-format.md: a card edit does NOT go out immediately. A
    // drunk 11pm mistake can be corrected by morning and *nobody ever
    // saw it*. We model the LOCAL side honestly:
    //
    //   card.md                      working/tracked (what you edit)
    //   .querykey/card.pending.md    the staged edit, not yet out
    //   .querykey/card.eligible_at   when it becomes eligible
    //   .querykey/card.published.md  the frozen snapshot a transport
    //                                would actually broadcast
    //
    // What *moves* card.published.md to peers is the deliberately
    // unresolved TRANSPORT question — there is no transport code here;
    // this is purely the local working↔published state + revert.

    const PROPAGATION_DELAY_HOURS: i64 = 24;

    fn pending_md(&self) -> PathBuf {
        self.root.join(".querykey").join("card.pending.md")
    }
    fn eligible_at_file(&self) -> PathBuf {
        self.root.join(".querykey").join("card.eligible_at")
    }
    fn published_md(&self) -> PathBuf {
        self.root.join(".querykey").join("card.published.md")
    }

    /// The snapshot a peer would currently receive (None until the
    /// first edit has propagated). Call `promote_due_card` first.
    pub fn card_published(&self) -> Option<crate::card::Card> {
        let s = fs::read_to_string(self.published_md()).ok()?;
        crate::card::parse(&s)
    }

    /// The staged-but-not-yet-out edit and when it becomes eligible.
    pub fn card_pending(&self) -> Option<(crate::card::Card, DateTime<Utc>)> {
        let c = crate::card::parse(&fs::read_to_string(self.pending_md()).ok()?)?;
        let at = parse_dt(fs::read_to_string(self.eligible_at_file()).ok()?.trim());
        Some((c, at))
    }

    /// Write the working (tracked) card.md AND stage it as pending
    /// with a 24h eligibility window. Does NOT touch the published
    /// snapshot — the edit is invisible to peers until it propagates.
    pub fn stage_card_edit(&self, c: &crate::card::Card) -> anyhow::Result<()> {
        self.upsert_card(c)?;
        fs::write(self.pending_md(), crate::card::render(c))?;
        let eligible = Utc::now() + chrono::Duration::hours(Self::PROPAGATION_DELAY_HOURS);
        fs::write(self.eligible_at_file(), rfc3339(&eligible))?;
        Ok(())
    }

    /// If a pending edit's window has elapsed, promote it to the
    /// published snapshot and clear the pending state. Idempotent;
    /// returns true iff it promoted this call.
    pub fn promote_due_card(&self) -> bool {
        let Some((_, eligible)) = self.card_pending() else {
            return false;
        };
        if Utc::now() < eligible {
            return false;
        }
        if let Ok(md) = fs::read_to_string(self.pending_md()) {
            if fs::write(self.published_md(), md).is_ok() {
                let _ = fs::remove_file(self.pending_md());
                let _ = fs::remove_file(self.eligible_at_file());
                return true;
            }
        }
        false
    }

    /// Revert a not-yet-propagated edit immediately (no delay): drop
    /// the pending edit and restore card.md from the published
    /// snapshot. Returns false if nothing was pending. If there is no
    /// published snapshot yet (first card never propagated), the
    /// pending edit is simply un-staged (card.md is left as-is — there
    /// is no prior state to roll back to).
    pub fn revert_pending_card(&self) -> bool {
        if self.card_pending().is_none() {
            return false;
        }
        let _ = fs::remove_file(self.pending_md());
        let _ = fs::remove_file(self.eligible_at_file());
        if let Ok(published) = fs::read_to_string(self.published_md()) {
            let _ = fs::write(self.card_path(), published);
        }
        true
    }

    // ----- peers (others' cards — read-only, git-ignored) -----
    //
    // We only ever READ what is locally present under peers/. Nothing
    // here FETCHES a peer's card — that is the transport question.
    // Dir name is a filesystem-safe slug; the true handle is the
    // card's own frontmatter `handle`, so ':' never hits the FS.

    fn peer_dirname(handle: &str) -> String {
        handle
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            })
            .collect()
    }

    pub fn list_peers(&self) -> Vec<crate::card::Card> {
        let dir = self.root.join("peers");
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let card = e.path().join("card.md");
                if let Ok(s) = fs::read_to_string(&card) {
                    if let Some(c) = crate::card::parse(&s) {
                        out.push(c);
                    }
                }
            }
        }
        out
    }

    pub fn get_peer_card(&self, handle: &str) -> Option<crate::card::Card> {
        let p = self
            .root
            .join("peers")
            .join(Self::peer_dirname(handle))
            .join("card.md");
        crate::card::parse(&fs::read_to_string(p).ok()?)
    }

    // ----- notes (freeform; the [[wikilink]] carrier) -----

    /// `(slug, body)` for every `notes/*.md`. Notes may have no
    /// frontmatter — `split` returns the whole file as the body then.
    pub fn list_notes(&self) -> Vec<(String, String)> {
        self.read_files("notes")
            .into_iter()
            .map(|(slug, c)| (slug, split(&c).1))
            .collect()
    }

    // ----- semantic wikilink edges (derived from the bodies) -----
    //
    // The canonical, backend-independent core. Every entity body is
    // scanned for [[Target]] / [[property:Target]]; the target is
    // resolved to a concrete entity by this vault (it knows them).
    // Resolution PRECEDENCE (resolves the docs/markdown-schema.md open
    // question) — first match wins:
    //   1. an explicit `kind:id` ref (symmetry with frontmatter refs)
    //   2. person  by id/slug, then by display name
    //   3. task    by uuid, then by title
    //   4. event   by uuid, then by title
    //   5. note    by slug
    //   6. DANGLING → kind "thing", id = slug(target); resolved=false
    //      (the edge is never dropped — dangling links are queryable;
    //      the raw target is kept as the label).
    // Untyped `[[X]]` uses the predicate `references`.

    pub fn collect_links(&self) -> Vec<LinkEdge> {
        let persons = self.list_persons();
        let tasks = self.list_tasks();
        let events = self.list_events();
        let notes = self.list_notes();

        let resolve = |raw: &str| -> (String, String, bool) {
            let t = raw.trim();
            let key = slug(t);
            // 1. explicit kind:id
            if let Some((k, id)) = t.split_once(':') {
                if matches!(k, "person" | "task" | "event" | "note") {
                    return (k.to_string(), id.trim().to_string(), true);
                }
            }
            // 2. person
            for p in &persons {
                if slug(&p.id) == key || slug(&p.display_name) == key {
                    return ("person".into(), p.id.clone(), true);
                }
            }
            // 3. task
            for x in &tasks {
                if x.id.to_string() == t || slug(&x.title) == key {
                    return ("task".into(), x.id.to_string(), true);
                }
            }
            // 4. event
            for x in &events {
                if x.id.to_string() == t || slug(&x.title) == key {
                    return ("event".into(), x.id.to_string(), true);
                }
            }
            // 5. note
            for (s, _) in &notes {
                if slug(s) == key {
                    return ("note".into(), s.clone(), true);
                }
            }
            // 6. dangling
            ("thing".into(), key, false)
        };

        let mut out = Vec::new();
        let mut emit = |from_kind: &str, from_id: &str, body: &str| {
            for wl in crate::wikilink::parse(body) {
                let (to_kind, to_id, resolved) = resolve(&wl.target);
                out.push(LinkEdge {
                    from_kind: from_kind.to_string(),
                    from_id: from_id.to_string(),
                    predicate: wl.property.unwrap_or_else(|| "references".into()),
                    to_kind,
                    to_id,
                    to_label: wl.target.trim().to_string(),
                    resolved,
                });
            }
        };

        for p in &persons {
            // Person body is a "# Name" heading by design — still scan
            // it (a user may add freeform relations under the heading).
            if let Some(c) = self.read_entity_body("people", &p.id) {
                emit("person", &p.id, &c);
            }
        }
        for t in &tasks {
            emit("task", &t.id.to_string(), &t.description);
        }
        for e in &events {
            emit("event", &e.id.to_string(), &e.description);
        }
        for (s, body) in &notes {
            emit("note", s, body);
        }
        out
    }

    fn read_entity_body(&self, sub: &str, slug: &str) -> Option<String> {
        let p = self.find_entity_file(sub, slug)?;
        Some(split(&fs::read_to_string(p).ok()?).1)
    }

    /// Outgoing links from an entity.
    pub fn links_from(&self, kind: &str, id: &str) -> Vec<LinkEdge> {
        self.collect_links()
            .into_iter()
            .filter(|e| e.from_kind == kind && e.from_id == id)
            .collect()
    }

    /// Backlinks: who points *at* this entity (high PRM value).
    pub fn links_to(&self, kind: &str, id: &str) -> Vec<LinkEdge> {
        self.collect_links()
            .into_iter()
            .filter(|e| e.resolved && e.to_kind == kind && e.to_id == id)
            .collect()
    }

    // ----- agent-drafted card (PRM → key/query) -----
    //
    // The agent's drafting behavior is governed by an editable,
    // version-controlled `agents.md` at the vault root (the vision's
    // "transparent, not a black box" envelope) — None if absent.

    pub fn agents_md(&self) -> Option<String> {
        fs::read_to_string(self.root.join("agents.md")).ok()
    }

    /// Build the compact PRM summary the drafter attends over.
    pub fn prm_digest(&self) -> PrmDigest {
        let persons = self.list_persons();
        let tasks = self.list_tasks();
        let links = self.collect_links();

        const OFFER_PREDS: &[&str] = &[
            "offers", "offering", "offer", "can-help", "can_help", "teaches", "provides",
            "mentors",
        ];
        const WANT_PREDS: &[&str] = &[
            "wants", "want", "looking-for", "looking_for", "needs", "need", "seeking", "seeks",
        ];
        let mut mentions: BTreeMap<String, usize> = BTreeMap::new();
        let mut preds: std::collections::BTreeSet<String> = Default::default();
        let (mut offers, mut wants): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
        for e in &links {
            preds.insert(e.predicate.clone());
            if e.resolved && e.to_kind == "person" {
                *mentions.entry(e.to_id.clone()).or_insert(0) += 1;
            }
            let p = e.predicate.as_str();
            if OFFER_PREDS.contains(&p) && !offers.contains(&e.to_label) {
                offers.push(e.to_label.clone());
            } else if WANT_PREDS.contains(&p) && !wants.contains(&e.to_label) {
                wants.push(e.to_label.clone());
            }
        }
        offers.truncate(15);
        wants.truncate(15);

        let mut top_people: Vec<DigestPerson> = persons
            .iter()
            .map(|p| DigestPerson {
                id: p.id.clone(),
                display_name: p.display_name.clone(),
                role: p.role.clone(),
                mentions: mentions.get(&p.id).copied().unwrap_or(0),
            })
            .collect();
        top_people.sort_by(|a, b| {
            b.mentions
                .cmp(&a.mentions)
                .then_with(|| a.display_name.cmp(&b.display_name))
        });
        top_people.truncate(10);

        let active_tasks: Vec<String> = tasks
            .iter()
            .filter(|t| t.status != TaskStatus::Done)
            .map(|t| t.title.clone())
            .take(20)
            .collect();

        PrmDigest {
            person_count: persons.len(),
            task_count: tasks.len(),
            event_count: self.list_events().len(),
            note_count: self.list_notes().len(),
            top_people,
            predicates: preds.into_iter().collect(),
            offers,
            wants,
            active_tasks,
            current_card: self.get_card(),
        }
    }

    fn write(&self, sub: &str, slug: &str, yaml: String, body: &str) -> anyhow::Result<()> {
        let dir = self.entity_dir(sub);
        fs::create_dir_all(&dir)?; // R15-3 dirs aren't pre-created
        let path = dir.join(format!("{slug}.md"));
        fs::write(path, compose(&yaml, body))?;
        self.migrate_legacy_on_write(sub, slug);
        Ok(())
    }

    fn read_files(&self, sub: &str) -> Vec<(String, String)> {
        // Union the canonical dir with every legacy dir, deduped by
        // slug. Legacy dirs are read in fall-back order *first* so
        // the canonical write last and wins on conflict. Same slug
        // appearing in multiple paths is the migration-in-progress
        // state — the lister returns each slug exactly once.
        let mut by_slug: BTreeMap<String, String> = BTreeMap::new();
        // Iterate legacy in reverse — they're listed in fall-back
        // (preference) order, so reverse to insert least-preferred
        // first; the canonical dir read last is the absolute winner.
        for legacy in self.legacy_entity_dirs(sub).into_iter().rev() {
            read_md_into(&legacy, &mut by_slug);
        }
        read_md_into(&self.entity_dir(sub), &mut by_slug);
        by_slug.into_iter().collect()
    }

    // ----- Person -----

    pub fn upsert_person(&self, p: &Person) -> anyhow::Result<()> {
        let fm = PersonFm {
            id: format!("person:{}", p.id),
            kind: "person".into(),
            display_name: p.display_name.clone(),
            handles: p
                .handles
                .iter()
                .map(|h| (h.platform.clone(), h.identifier.clone()))
                .collect(),
            role: p.role.clone(),
            contact_cascade: p.contact_cascade.clone(),
            created: rfc3339(&p.created_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        let body = format!("# {}\n", p.display_name);
        self.write("people", &p.id, yaml, &body)
    }

    fn person_from(&self, slug: &str, content: &str) -> Option<Person> {
        let (yaml, _body) = split(content);
        let fm: PersonFm = serde_yaml::from_str(&yaml).ok()?;
        let id = fm
            .id
            .strip_prefix("person:")
            .unwrap_or(slug)
            .to_string();
        Some(Person {
            id,
            display_name: fm.display_name,
            handles: fm
                .handles
                .into_iter()
                .map(|(platform, identifier)| Handle {
                    platform,
                    identifier,
                })
                .collect(),
            role: fm.role,
            contact_cascade: fm.contact_cascade,
            created_at: parse_dt(&fm.created),
        })
    }

    pub fn get_person(&self, id: &str) -> Option<Person> {
        let s = fs::read_to_string(self.find_entity_file("people", id)?).ok()?;
        self.person_from(id, &s)
    }

    pub fn list_persons(&self) -> Vec<Person> {
        self.read_files("people")
            .into_iter()
            .filter_map(|(slug, c)| self.person_from(&slug, &c))
            .collect()
    }

    // ----- Task -----

    pub fn upsert_task(&self, t: &Task) -> anyhow::Result<()> {
        let fm = TaskFm {
            id: format!("task:{}", t.id),
            kind: "task".into(),
            title: t.title.clone(),
            status: t.status,
            person: if t.assigned_to.is_empty() {
                String::new()
            } else {
                format!("person:{}", t.assigned_to)
            },
            assigned_by: t.assigned_by.clone(),
            deadline: t.deadline.map(|d| rfc3339(&d)),
            confidence: t.confidence,
            ambiguity_score: t.ambiguity_score,
            source: t.source_messages.clone(),
            created: rfc3339(&t.created_at),
            updated: rfc3339(&t.updated_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("tasks", &t.id.to_string(), yaml, &t.description)
    }

    fn task_from(&self, content: &str) -> Option<Task> {
        let (yaml, body) = split(content);
        let fm: TaskFm = serde_yaml::from_str(&yaml).ok()?;
        let id = uuid::Uuid::parse_str(fm.id.strip_prefix("task:").unwrap_or(&fm.id)).ok()?;
        Some(Task {
            id,
            title: fm.title,
            description: body,
            status: fm.status,
            assigned_to: fm
                .person
                .strip_prefix("person:")
                .unwrap_or(&fm.person)
                .to_string(),
            assigned_by: fm.assigned_by,
            deadline: fm.deadline.as_deref().map(parse_dt),
            confidence: fm.confidence,
            ambiguity_score: fm.ambiguity_score,
            source_messages: fm.source,
            created_at: parse_dt(&fm.created),
            updated_at: parse_dt(&fm.updated),
        })
    }

    pub fn get_task(&self, id: &uuid::Uuid) -> Option<Task> {
        let s = fs::read_to_string(self.find_entity_file("tasks", &id.to_string())?).ok()?;
        self.task_from(&s)
    }

    pub fn list_tasks(&self) -> Vec<Task> {
        self.read_files("tasks")
            .into_iter()
            .filter_map(|(_, c)| self.task_from(&c))
            .collect()
    }

    pub fn tasks_for_person(&self, person_id: &str) -> Vec<Task> {
        self.list_tasks()
            .into_iter()
            .filter(|t| t.assigned_to == person_id)
            .collect()
    }

    // ----- Event -----

    pub fn upsert_event(&self, e: &Event) -> anyhow::Result<()> {
        let fm = EventFm {
            id: format!("event:{}", e.id),
            kind: "event".into(),
            title: e.title.clone(),
            start: rfc3339(&e.start_time),
            end: rfc3339(&e.end_time),
            people: e
                .participants
                .iter()
                .map(|p| format!("person:{p}"))
                .collect(),
            recurrence: e.recurrence.clone(),
            confidence: e.confidence,
            source: e.source_messages.clone(),
            created: rfc3339(&e.created_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("events", &e.id.to_string(), yaml, &e.description)
    }

    fn event_from(&self, content: &str) -> Option<Event> {
        let (yaml, body) = split(content);
        let fm: EventFm = serde_yaml::from_str(&yaml).ok()?;
        let id = uuid::Uuid::parse_str(fm.id.strip_prefix("event:").unwrap_or(&fm.id)).ok()?;
        Some(Event {
            id,
            title: fm.title,
            description: body,
            start_time: parse_dt(&fm.start),
            end_time: parse_dt(&fm.end),
            participants: fm
                .people
                .into_iter()
                .map(|p| p.strip_prefix("person:").unwrap_or(&p).to_string())
                .collect(),
            recurrence: fm.recurrence,
            confidence: fm.confidence,
            source_messages: fm.source,
            created_at: parse_dt(&fm.created),
        })
    }

    pub fn list_events(&self) -> Vec<Event> {
        self.read_files("events")
            .into_iter()
            .filter_map(|(_, c)| self.event_from(&c))
            .collect()
    }

    /// Merged, time-ordered agenda for `[from, to]` (inclusive):
    /// every event occurrence (recurrence expanded) plus every task
    /// whose deadline falls in the window. Backend-independent — read
    /// straight from the canonical vault.
    pub fn agenda(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<AgendaItem> {
        let mut items: Vec<(DateTime<Utc>, AgendaItem)> = Vec::new();

        for e in self.list_events() {
            let dur = e.end_time - e.start_time;
            let recurring = e.recurrence.is_some();
            for occ in
                crate::calendar::occurrences(e.start_time, e.recurrence.as_deref(), from, to)
            {
                items.push((
                    occ,
                    AgendaItem {
                        kind: "event".into(),
                        id: e.id.to_string(),
                        title: e.title.clone(),
                        start: rfc3339(&occ),
                        end: Some(rfc3339(&(occ + dur))),
                        movable: false,
                        recurring,
                    },
                ));
            }
        }

        for t in self.list_tasks() {
            if let Some(d) = t.deadline {
                if d >= from && d <= to {
                    items.push((
                        d,
                        AgendaItem {
                            kind: "task".into(),
                            id: t.id.to_string(),
                            title: t.title.clone(),
                            start: rfc3339(&d),
                            end: None,
                            movable: true, // time-flexible by definition
                            recurring: false,
                        },
                    ));
                }
            }
        }

        items.sort_by(|a, b| a.0.cmp(&b.0));
        items.into_iter().map(|(_, it)| it).collect()
    }

    // ----- Conflict -----
    //
    // Body = the human `explanation` (why these two messages clash).
    // Everything structured/queryable is frontmatter.

    pub fn upsert_conflict(&self, c: &Conflict) -> anyhow::Result<()> {
        let fm = ConflictFm {
            id: format!("conflict:{}", c.id),
            kind: "conflict".into(),
            conflict_type: c.conflict_type,
            message_a: c.message_a.clone(),
            message_b: c.message_b.clone(),
            task: if c.task_id.is_empty() {
                String::new()
            } else {
                format!("task:{}", c.task_id)
            },
            resolution: c.resolution,
            resolved_by: c.resolved_by.clone(),
            created: rfc3339(&c.created_at),
            resolved: c.resolved_at.map(|d| rfc3339(&d)),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("conflicts", &c.id.to_string(), yaml, &c.explanation)
    }

    fn conflict_from(&self, content: &str) -> Option<Conflict> {
        let (yaml, body) = split(content);
        let fm: ConflictFm = serde_yaml::from_str(&yaml).ok()?;
        let id = uuid::Uuid::parse_str(fm.id.strip_prefix("conflict:").unwrap_or(&fm.id)).ok()?;
        Some(Conflict {
            id,
            conflict_type: fm.conflict_type,
            message_a: fm.message_a,
            message_b: fm.message_b,
            task_id: fm
                .task
                .strip_prefix("task:")
                .unwrap_or(&fm.task)
                .to_string(),
            explanation: body,
            resolution: fm.resolution,
            resolved_by: fm.resolved_by,
            created_at: parse_dt(&fm.created),
            resolved_at: fm.resolved.as_deref().map(parse_dt),
        })
    }

    pub fn get_conflict(&self, id: &uuid::Uuid) -> Option<Conflict> {
        let s = fs::read_to_string(self.find_entity_file("conflicts", &id.to_string())?).ok()?;
        self.conflict_from(&s)
    }

    pub fn list_conflicts(&self) -> Vec<Conflict> {
        self.read_files("conflicts")
            .into_iter()
            .filter_map(|(_, c)| self.conflict_from(&c))
            .collect()
    }

    // ----- OpenQuestion -----
    //
    // Body = the human-facing `question`. The id is a readable slug
    // (string), not a UUID, so it doubles as the filename like Person.

    pub fn upsert_question(&self, q: &OpenQuestion) -> anyhow::Result<()> {
        let fm = OpenQuestionFm {
            id: format!("question:{}", q.id),
            kind: "question".into(),
            target: q.target.clone(),
            context: q.context.clone(),
            urgency: q.urgency,
            urgency_deadline: q.urgency_deadline.map(|d| rfc3339(&d)),
            trigger_type: q.trigger_type,
            trigger_id: q.trigger_id.clone(),
            status: q.status,
            resolution: q.resolution.clone(),
            resolved_by: q.resolved_by.clone(),
            resolved_via: q.resolved_via.clone(),
            created: rfc3339(&q.created_at),
            resolved: q.resolved_at.map(|d| rfc3339(&d)),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("questions", &q.id, yaml, &q.question)
    }

    fn question_from(&self, slug: &str, content: &str) -> Option<OpenQuestion> {
        let (yaml, body) = split(content);
        let fm: OpenQuestionFm = serde_yaml::from_str(&yaml).ok()?;
        let id = fm
            .id
            .strip_prefix("question:")
            .unwrap_or(slug)
            .to_string();
        Some(OpenQuestion {
            id,
            target: fm.target,
            question: body,
            context: fm.context,
            urgency: fm.urgency,
            urgency_deadline: fm.urgency_deadline.as_deref().map(parse_dt),
            trigger_type: fm.trigger_type,
            trigger_id: fm.trigger_id,
            status: fm.status,
            resolution: fm.resolution,
            resolved_by: fm.resolved_by,
            resolved_via: fm.resolved_via,
            created_at: parse_dt(&fm.created),
            resolved_at: fm.resolved.as_deref().map(parse_dt),
        })
    }

    pub fn get_question(&self, id: &str) -> Option<OpenQuestion> {
        let s = fs::read_to_string(self.find_entity_file("questions", id)?).ok()?;
        self.question_from(id, &s)
    }

    pub fn list_questions(&self) -> Vec<OpenQuestion> {
        self.read_files("questions")
            .into_iter()
            .filter_map(|(slug, c)| self.question_from(&slug, &c))
            .collect()
    }

    // ----- FollowUp -----
    //
    // Body = the human-facing `question`. Delivery attempts are a
    // nested frontmatter list (small, structured, round-trips via serde).

    pub fn upsert_followup(&self, f: &FollowUp) -> anyhow::Result<()> {
        let fm = FollowUpFm {
            id: format!("followup:{}", f.id),
            kind: "followup".into(),
            trigger_type: f.trigger_type,
            trigger_id: f.trigger_id.clone(),
            target: f.target.clone(),
            context: f.context.clone(),
            delivery_attempts: f.delivery_attempts.clone(),
            status: f.status,
            response: f.response.clone(),
            response_channel: f.response_channel.clone(),
            response_at: f.response_at.map(|d| rfc3339(&d)),
            created: rfc3339(&f.created_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("followups", &f.id, yaml, &f.question)
    }

    fn followup_from(&self, slug: &str, content: &str) -> Option<FollowUp> {
        let (yaml, body) = split(content);
        let fm: FollowUpFm = serde_yaml::from_str(&yaml).ok()?;
        let id = fm
            .id
            .strip_prefix("followup:")
            .unwrap_or(slug)
            .to_string();
        Some(FollowUp {
            id,
            trigger_type: fm.trigger_type,
            trigger_id: fm.trigger_id,
            target: fm.target,
            question: body,
            context: fm.context,
            delivery_attempts: fm.delivery_attempts,
            status: fm.status,
            response: fm.response,
            response_channel: fm.response_channel,
            response_at: fm.response_at.as_deref().map(parse_dt),
            created_at: parse_dt(&fm.created),
        })
    }

    pub fn get_followup(&self, id: &str) -> Option<FollowUp> {
        let s = fs::read_to_string(self.find_entity_file("followups", id)?).ok()?;
        self.followup_from(id, &s)
    }

    pub fn list_followups(&self) -> Vec<FollowUp> {
        self.read_files("followups")
            .into_iter()
            .filter_map(|(slug, c)| self.followup_from(&slug, &c))
            .collect()
    }

    // ----- Instruction -----
    //
    // Who said what to whom. Body = the human `content` (the actual
    // utterance); everything structured is frontmatter.

    pub fn upsert_instruction(&self, i: &Instruction) -> anyhow::Result<()> {
        let fm = InstructionFm {
            id: format!("instruction:{}", i.id),
            kind: "instruction".into(),
            speaker: i.speaker.clone(),
            audience: i.audience.clone(),
            is_task: i.is_task,
            task: if i.task_id.is_empty() {
                String::new()
            } else {
                format!("task:{}", i.task_id)
            },
            source_message: i.source_message.clone(),
            created: rfc3339(&i.created_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("instructions", &i.id.to_string(), yaml, &i.content)
    }

    fn instruction_from(&self, content: &str) -> Option<Instruction> {
        let (yaml, body) = split(content);
        let fm: InstructionFm = serde_yaml::from_str(&yaml).ok()?;
        let id =
            uuid::Uuid::parse_str(fm.id.strip_prefix("instruction:").unwrap_or(&fm.id)).ok()?;
        Some(Instruction {
            id,
            content: body,
            speaker: fm.speaker,
            audience: fm.audience,
            is_task: fm.is_task,
            task_id: fm.task.strip_prefix("task:").unwrap_or(&fm.task).to_string(),
            source_message: fm.source_message,
            created_at: parse_dt(&fm.created),
        })
    }

    pub fn get_instruction(&self, id: &uuid::Uuid) -> Option<Instruction> {
        let p = self.find_entity_file("instructions", &id.to_string())?;
        self.instruction_from(&fs::read_to_string(p).ok()?)
    }

    pub fn list_instructions(&self) -> Vec<Instruction> {
        self.read_files("instructions")
            .into_iter()
            .filter_map(|(_, c)| self.instruction_from(&c))
            .collect()
    }

    // ----- VoiceProfile -----
    //
    // Speaker identity (diarization). The most "machine" entity — the
    // body is a cosmetic heading; everything is frontmatter, and the
    // embedding is omitted until the audio pipeline fills it.

    pub fn upsert_voice_profile(&self, v: &VoiceProfile) -> anyhow::Result<()> {
        let fm = VoiceProfileFm {
            id: format!("voiceprofile:{}", v.id),
            kind: "voice_profile".into(),
            person: if v.person_id.is_empty() {
                String::new()
            } else {
                format!("person:{}", v.person_id)
            },
            embedding: v.embedding.clone(),
            sample_count: v.sample_count,
            confidence: v.confidence,
            last_updated: rfc3339(&v.last_updated),
            created: rfc3339(&v.created_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        let body = format!("# Voice profile — person:{}\n", v.person_id);
        self.write("voiceprofiles", &v.id.to_string(), yaml, &body)
    }

    fn voice_profile_from(&self, content: &str) -> Option<VoiceProfile> {
        let (yaml, _body) = split(content);
        let fm: VoiceProfileFm = serde_yaml::from_str(&yaml).ok()?;
        let id =
            uuid::Uuid::parse_str(fm.id.strip_prefix("voiceprofile:").unwrap_or(&fm.id)).ok()?;
        Some(VoiceProfile {
            id,
            person_id: fm
                .person
                .strip_prefix("person:")
                .unwrap_or(&fm.person)
                .to_string(),
            embedding: fm.embedding,
            sample_count: fm.sample_count,
            confidence: fm.confidence,
            last_updated: parse_dt(&fm.last_updated),
            created_at: parse_dt(&fm.created),
        })
    }

    pub fn get_voice_profile(&self, id: &uuid::Uuid) -> Option<VoiceProfile> {
        let p = self.find_entity_file("voiceprofiles", &id.to_string())?;
        self.voice_profile_from(&fs::read_to_string(p).ok()?)
    }

    pub fn list_voice_profiles(&self) -> Vec<VoiceProfile> {
        self.read_files("voiceprofiles")
            .into_iter()
            .filter_map(|(_, c)| self.voice_profile_from(&c))
            .collect()
    }

    pub fn voice_profile_for_person(&self, person_id: &str) -> Option<VoiceProfile> {
        self.list_voice_profiles()
            .into_iter()
            .find(|v| v.person_id == person_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn person_and_task_round_trip_losslessly() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        let p = Person {
            id: "ada-lovelace".into(),
            display_name: "Ada Lovelace".into(),
            handles: vec![Handle {
                platform: "github".into(),
                identifier: "ada".into(),
            }],
            role: "mathematician".into(),
            contact_cascade: vec!["app".into(), "discord".into()],
            created_at: DateTime::parse_from_rfc3339("2026-05-15T00:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
        };
        v.upsert_person(&p).unwrap();
        let got = v.get_person("ada-lovelace").unwrap();
        assert_eq!(got.id, p.id);
        assert_eq!(got.display_name, p.display_name);
        assert_eq!(got.handles.len(), 1);
        assert_eq!(got.handles[0].platform, "github");
        assert_eq!(got.role, p.role);
        assert_eq!(got.contact_cascade, p.contact_cascade);
        assert_eq!(got.created_at, p.created_at);

        let id = uuid::Uuid::new_v4();
        let t = Task {
            id,
            title: "Return the book".into(),
            description: "Ada lent me the Rust book.\n\nMulti-line body.".into(),
            status: TaskStatus::InProgress,
            assigned_to: "ada-lovelace".into(),
            assigned_by: String::new(),
            deadline: Some(
                DateTime::parse_from_rfc3339("2026-05-20T00:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            confidence: 0.7,
            ambiguity_score: 0.2,
            source_messages: vec!["ingest:abc".into()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        v.upsert_task(&t).unwrap();
        let gt = v.get_task(&id).unwrap();
        assert_eq!(gt.id, t.id);
        assert_eq!(gt.title, t.title);
        assert_eq!(gt.description, t.description);
        assert_eq!(gt.status, t.status);
        assert_eq!(gt.assigned_to, "ada-lovelace");
        assert_eq!(gt.deadline, t.deadline);
        assert_eq!(gt.confidence, 0.7);
        assert_eq!(v.tasks_for_person("ada-lovelace").len(), 1);
        assert_eq!(v.list_persons().len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conflict_question_followup_round_trip_losslessly() {
        use crate::models::{
            ConflictResolution, ConflictType, DeliveryAttempt, DeliveryStatus, FollowUpStatus,
            QuestionStatus, TriggerType, Urgency,
        };
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        // --- Conflict ---
        let cid = uuid::Uuid::new_v4();
        let c = Conflict {
            id: cid,
            conflict_type: ConflictType::DeadlineChange,
            message_a: "msg-a".into(),
            message_b: "msg-b".into(),
            task_id: "11111111-1111-1111-1111-111111111111".into(),
            explanation: "Alice said Friday, Bob said Monday.\n\nNeeds a human call.".into(),
            resolution: ConflictResolution::AWins,
            resolved_by: "immanuelle".into(),
            created_at: DateTime::parse_from_rfc3339("2026-05-15T09:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
            resolved_at: Some(
                DateTime::parse_from_rfc3339("2026-05-15T10:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
        };
        v.upsert_conflict(&c).unwrap();
        let gc = v.get_conflict(&cid).unwrap();
        assert_eq!(gc.id, c.id);
        assert_eq!(gc.conflict_type, c.conflict_type);
        assert_eq!(gc.message_a, "msg-a");
        assert_eq!(gc.task_id, c.task_id);
        assert_eq!(gc.explanation, c.explanation);
        assert_eq!(gc.resolution, c.resolution);
        assert_eq!(gc.resolved_by, "immanuelle");
        assert_eq!(gc.created_at, c.created_at);
        assert_eq!(gc.resolved_at, c.resolved_at);
        assert_eq!(v.list_conflicts().len(), 1);

        // --- OpenQuestion ---
        let q = OpenQuestion {
            id: "deadline-for-johns-book".into(),
            target: "person:john-smith".into(),
            question: "When does John actually need the book back?".into(),
            context: "Casual mention; deadline is a guess.".into(),
            urgency: Urgency::ByTime,
            urgency_deadline: Some(
                DateTime::parse_from_rfc3339("2026-05-20T00:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            trigger_type: TriggerType::Ambiguity,
            trigger_id: "task:return-johns-book".into(),
            status: QuestionStatus::Open,
            resolution: String::new(),
            resolved_by: String::new(),
            resolved_via: String::new(),
            created_at: DateTime::parse_from_rfc3339("2026-05-15T09:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
            resolved_at: None,
        };
        v.upsert_question(&q).unwrap();
        let gq = v.get_question("deadline-for-johns-book").unwrap();
        assert_eq!(gq.id, q.id);
        assert_eq!(gq.target, q.target);
        assert_eq!(gq.question, q.question);
        assert_eq!(gq.context, q.context);
        assert_eq!(gq.urgency, q.urgency);
        assert_eq!(gq.urgency_deadline, q.urgency_deadline);
        assert_eq!(gq.trigger_type, q.trigger_type);
        assert_eq!(gq.status, q.status);
        assert_eq!(gq.created_at, q.created_at);
        assert_eq!(gq.resolved_at, None);
        assert_eq!(v.list_questions().len(), 1);

        // --- FollowUp ---
        let f = FollowUp {
            id: "ping-john-about-book".into(),
            trigger_type: TriggerType::UnconfirmedTask,
            trigger_id: "task:return-johns-book".into(),
            target: "person:john-smith".into(),
            question: "Still good to drop the book off Saturday?".into(),
            context: "No reply to the first nudge.".into(),
            delivery_attempts: vec![DeliveryAttempt {
                channel: "discord".into(),
                status: DeliveryStatus::Delivered,
                sent_at: DateTime::parse_from_rfc3339("2026-05-15T11:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            }],
            status: FollowUpStatus::Sent,
            response: String::new(),
            response_channel: String::new(),
            response_at: None,
            created_at: DateTime::parse_from_rfc3339("2026-05-15T10:30:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
        };
        v.upsert_followup(&f).unwrap();
        let gf = v.get_followup("ping-john-about-book").unwrap();
        assert_eq!(gf.id, f.id);
        assert_eq!(gf.trigger_type, f.trigger_type);
        assert_eq!(gf.trigger_id, f.trigger_id);
        assert_eq!(gf.target, f.target);
        assert_eq!(gf.question, f.question);
        assert_eq!(gf.context, f.context);
        assert_eq!(gf.delivery_attempts.len(), 1);
        assert_eq!(gf.delivery_attempts[0].channel, "discord");
        assert_eq!(gf.delivery_attempts[0].status, DeliveryStatus::Delivered);
        assert_eq!(gf.delivery_attempts[0].sent_at, f.delivery_attempts[0].sent_at);
        assert_eq!(gf.status, f.status);
        assert_eq!(gf.created_at, f.created_at);
        assert_eq!(v.list_followups().len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn card_propagation_delay_revert_and_peers() {
        use crate::card::Card;
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        // Privacy asymmetry: .gitignore written, ignores peers/ +
        // .querykey/ but NOT card.md.
        let gi = fs::read_to_string(dir.join(".gitignore")).unwrap();
        let rules: Vec<&str> = gi
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        assert!(rules.contains(&"/peers/"));
        assert!(rules.contains(&"/.querykey/"));
        // card.md must never be an ignore *rule* (comments may mention
        // it — that's the documentation of the asymmetry).
        assert!(!rules.iter().any(|r| r.contains("card.md")));

        let mk = |bio: &str| Card {
            handle: "github:emma".into(),
            name: "Emma".into(),
            website: "https://emmaleonhart.com".into(),
            bio: bio.into(),
            offering: vec!["Rust help".into()],
            looking_for: vec!["Flutter reviewers".into()],
            updated: Utc::now(),
            visibility: "public".into(),
        };

        // Edit → working card.md written, staged pending, NOT published.
        v.stage_card_edit(&mk("first bio")).unwrap();
        assert_eq!(v.get_card().unwrap().bio, "first bio");
        assert!(v.card_pending().is_some());
        assert!(v.card_published().is_none());
        assert!(!v.promote_due_card()); // 24h not elapsed

        // Backdate eligibility → propagation promotes it.
        let elig = dir.join(".querykey").join("card.eligible_at");
        fs::write(&elig, rfc3339(&(Utc::now() - chrono::Duration::hours(1)))).unwrap();
        assert!(v.promote_due_card());
        assert!(v.card_pending().is_none());
        assert_eq!(v.card_published().unwrap().bio, "first bio");

        // A bad edit, reverted before propagation → published
        // unchanged, working restored to the published snapshot.
        v.stage_card_edit(&mk("drunk 11pm mistake")).unwrap();
        assert_eq!(v.get_card().unwrap().bio, "drunk 11pm mistake");
        assert!(v.revert_pending_card());
        assert!(v.card_pending().is_none());
        assert_eq!(v.get_card().unwrap().bio, "first bio"); // rolled back
        assert_eq!(v.card_published().unwrap().bio, "first bio"); // never saw it
        assert!(!v.revert_pending_card()); // nothing pending now

        // Peers: read-only, ':' in handle never hits the FS.
        let peer = mk("a peer");
        let pdir = dir.join("peers").join(Vault::peer_dirname("github:bob"));
        fs::create_dir_all(&pdir).unwrap();
        fs::write(pdir.join("card.md"), crate::card::render(&peer)).unwrap();
        assert!(!Vault::peer_dirname("github:bob").contains(':'));
        assert_eq!(v.list_peers().len(), 1);
        assert_eq!(v.get_peer_card("github:bob").unwrap().handle, "github:emma");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn semantic_wikilinks_resolve_with_precedence_and_dangling() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        assert_eq!(slug("John  Smith!!"), "john-smith");
        assert_eq!(slug("person-john-smith"), "person-john-smith");

        let ada = Person {
            id: "ada-lovelace".into(),
            display_name: "Ada Lovelace".into(),
            handles: vec![],
            role: String::new(),
            contact_cascade: vec![],
            created_at: Utc::now(),
        };
        v.upsert_person(&ada).unwrap();

        let tid = uuid::Uuid::new_v4();
        v.upsert_task(&Task {
            id: tid,
            title: "Read the Analytical Engine notes".into(),
            // typed link by DISPLAY NAME + an untyped one + a dangling.
            description: "Per [[mentor:Ada Lovelace]], also see [[Ada Lovelace]] \
                          and the missing [[Babbage Difference Engine]]."
                .into(),
            status: TaskStatus::Extracted,
            assigned_to: String::new(),
            assigned_by: String::new(),
            deadline: None,
            confidence: 0.5,
            ambiguity_score: 0.0,
            source_messages: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .unwrap();

        // A note: explicit kind:id ref takes precedence (rule 1).
        // Notes live under wiki/notes/ from R15 onward; Vault::open
        // created the dir already.
        fs::write(
            dir.join("wiki").join("notes").join("salon.md"),
            "# Salon\n\nIntroduced [[knows:person:ada-lovelace]] to the group.\n",
        )
        .unwrap();

        let edges = v.collect_links();

        // Task → Ada by display name, typed predicate "mentor".
        assert!(edges.iter().any(|e| e.from_kind == "task"
            && e.from_id == tid.to_string()
            && e.predicate == "mentor"
            && e.to_kind == "person"
            && e.to_id == "ada-lovelace"
            && e.resolved));
        // Untyped link → predicate defaults to "references".
        assert!(edges.iter().any(|e| e.from_kind == "task"
            && e.predicate == "references"
            && e.to_id == "ada-lovelace"));
        // Dangling link kept, resolved=false, label preserved.
        let dangling = edges
            .iter()
            .find(|e| !e.resolved)
            .expect("dangling edge kept");
        assert_eq!(dangling.to_kind, "thing");
        assert_eq!(dangling.to_id, "babbage-difference-engine");
        assert_eq!(dangling.to_label, "Babbage Difference Engine");
        // Note's explicit `person:ada-lovelace` resolves (rule 1).
        assert!(edges.iter().any(|e| e.from_kind == "note"
            && e.from_id == "salon"
            && e.predicate == "knows"
            && e.to_kind == "person"
            && e.to_id == "ada-lovelace"));

        // Backlinks: things pointing AT Ada (PRM payoff).
        let back = v.links_to("person", "ada-lovelace");
        assert!(back.len() >= 3);
        assert!(back.iter().all(|e| e.resolved));
        assert!(v.links_from("note", "salon").iter().any(|e| e.predicate == "knows"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn instruction_and_voice_profile_round_trip() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        let iid = uuid::Uuid::new_v4();
        let inst = Instruction {
            id: iid,
            content: "Ship the report by Friday.\n\nNo extensions.".into(),
            speaker: "alice".into(),
            audience: vec!["bob".into(), "carol".into()],
            is_task: true,
            task_id: "11111111-1111-1111-1111-111111111111".into(),
            source_message: "ingest:abc".into(),
            created_at: DateTime::parse_from_rfc3339("2026-05-16T09:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
        };
        v.upsert_instruction(&inst).unwrap();
        let gi = v.get_instruction(&iid).unwrap();
        assert_eq!(gi.id, inst.id);
        assert_eq!(gi.content, inst.content);
        assert_eq!(gi.speaker, "alice");
        assert_eq!(gi.audience, vec!["bob", "carol"]);
        assert!(gi.is_task);
        assert_eq!(gi.task_id, inst.task_id);
        assert_eq!(gi.source_message, "ingest:abc");
        assert_eq!(gi.created_at, inst.created_at);
        assert_eq!(v.list_instructions().len(), 1);

        let vid = uuid::Uuid::new_v4();
        let vp = VoiceProfile {
            id: vid,
            person_id: "ada-lovelace".into(),
            embedding: vec![0, 17, 255, 128, 64],
            sample_count: 12,
            confidence: 0.83,
            last_updated: DateTime::parse_from_rfc3339("2026-05-16T10:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
            created_at: DateTime::parse_from_rfc3339("2026-05-16T08:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
        };
        v.upsert_voice_profile(&vp).unwrap();
        let gv = v.get_voice_profile(&vid).unwrap();
        assert_eq!(gv.id, vp.id);
        assert_eq!(gv.person_id, "ada-lovelace");
        assert_eq!(gv.embedding, vec![0, 17, 255, 128, 64]);
        assert_eq!(gv.sample_count, 12);
        assert_eq!(gv.confidence, 0.83);
        assert_eq!(gv.last_updated, vp.last_updated);
        assert_eq!(gv.created_at, vp.created_at);
        assert_eq!(
            v.voice_profile_for_person("ada-lovelace").unwrap().id,
            vid
        );

        // Empty embedding (the common case until audio exists) is
        // omitted from frontmatter and still round-trips.
        let vid2 = uuid::Uuid::new_v4();
        let vp2 = VoiceProfile {
            id: vid2,
            person_id: "grace-hopper".into(),
            embedding: vec![],
            sample_count: 0,
            confidence: 0.0,
            last_updated: Utc::now(),
            created_at: Utc::now(),
        };
        v.upsert_voice_profile(&vp2).unwrap();
        assert!(v.get_voice_profile(&vid2).unwrap().embedding.is_empty());
        assert_eq!(v.list_voice_profiles().len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn event_recurrence_round_trips_and_expands() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        let id = uuid::Uuid::new_v4();
        let start = DateTime::parse_from_rfc3339("2026-05-04T09:00:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        v.upsert_event(&Event {
            id,
            title: "Weekly 1:1".into(),
            description: "Recurring sync.".into(),
            start_time: start,
            end_time: start + chrono::Duration::minutes(30),
            participants: vec!["ada-lovelace".into()],
            recurrence: Some("FREQ=WEEKLY;COUNT=4".into()),
            confidence: 0.9,
            source_messages: vec![],
            created_at: Utc::now(),
        })
        .unwrap();

        let got = v.list_events().into_iter().find(|e| e.id == id).unwrap();
        assert_eq!(got.recurrence.as_deref(), Some("FREQ=WEEKLY;COUNT=4"));
        assert_eq!(got.participants, vec!["ada-lovelace"]);

        // Non-recurring still round-trips as None (back-compat).
        let id2 = uuid::Uuid::new_v4();
        v.upsert_event(&Event {
            id: id2,
            title: "One-off".into(),
            description: String::new(),
            start_time: start,
            end_time: start,
            participants: vec![],
            recurrence: None,
            confidence: 0.5,
            source_messages: vec![],
            created_at: Utc::now(),
        })
        .unwrap();
        let g2 = v.list_events().into_iter().find(|e| e.id == id2).unwrap();
        assert_eq!(g2.recurrence, None);

        // The expander sees the rule end-to-end.
        let occ = crate::calendar::occurrences(
            got.start_time,
            got.recurrence.as_deref(),
            DateTime::parse_from_rfc3339("2026-05-01T00:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
            DateTime::parse_from_rfc3339("2026-12-31T00:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(occ.len(), 4); // COUNT=4

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn agenda_merges_events_and_deadlined_tasks_in_order() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();
        let d = |s: &str| {
            DateTime::parse_from_rfc3339(s)
                .unwrap()
                .with_timezone(&Utc)
        };

        // Weekly event starting Mon May 4.
        v.upsert_event(&Event {
            id: uuid::Uuid::new_v4(),
            title: "Standup".into(),
            description: String::new(),
            start_time: d("2026-05-04T09:00:00+00:00"),
            end_time: d("2026-05-04T09:15:00+00:00"),
            participants: vec![],
            recurrence: Some("FREQ=WEEKLY;COUNT=6".into()),
            confidence: 0.9,
            source_messages: vec![],
            created_at: Utc::now(),
        })
        .unwrap();

        // Task due Tue May 5 (in window) and one due far outside.
        for (title, dl) in [
            ("Send invoice", "2026-05-05T17:00:00+00:00"),
            ("Out of range", "2026-09-01T00:00:00+00:00"),
        ] {
            v.upsert_task(&Task {
                id: uuid::Uuid::new_v4(),
                title: title.into(),
                description: String::new(),
                status: TaskStatus::Confirmed,
                assigned_to: String::new(),
                assigned_by: String::new(),
                deadline: Some(d(dl)),
                confidence: 0.8,
                ambiguity_score: 0.0,
                source_messages: vec![],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .unwrap();
        }

        let ag = v.agenda(d("2026-05-04T00:00:00+00:00"), d("2026-05-12T00:00:00+00:00"));
        // May 4 standup, May 5 invoice, May 11 standup. Out-of-range
        // task excluded.
        assert_eq!(ag.len(), 3);
        assert_eq!(ag[0].kind, "event");
        assert!(!ag[0].movable && ag[0].recurring);
        assert_eq!(ag[0].end.as_deref(), Some("2026-05-04T09:15:00+00:00"));
        assert_eq!(ag[1].kind, "task");
        assert_eq!(ag[1].title, "Send invoice");
        assert!(ag[1].movable && ag[1].end.is_none());
        assert_eq!(ag[2].kind, "event"); // May 11 occurrence
        assert!(ag[2].start.starts_with("2026-05-11"));
        // Strictly time-ordered.
        assert!(ag[0].start <= ag[1].start && ag[1].start <= ag[2].start);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- R15-2: wiki/ subtree + legacy back-compat ----

    /// New writes land under wiki/<entity>/, not at the legacy root.
    /// People specifically live at `wiki/contacts/` (R15-3 rename) —
    /// that path-specific assertion is covered in the R15-3 tests
    /// below; here we use a non-renamed entity (notes) to exercise
    /// the generic wiki/-vs-root behavior.
    #[test]
    fn r15_writes_go_under_wiki() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let _v = Vault::open(dir.to_str().unwrap()).unwrap();

        // Vault::open creates the canonical entity dirs under wiki/.
        assert!(
            dir.join("wiki").join("notes").is_dir(),
            "wiki/notes/ should be created on open"
        );
        assert!(
            dir.join("wiki").join("tasks").is_dir(),
            "wiki/tasks/ should be created on open"
        );
        assert!(
            dir.join("wiki").join("events").is_dir(),
            "wiki/events/ should be created on open"
        );
        // Non-graph dirs stay at the root.
        assert!(dir.join("peers").is_dir(), "peers/ stays at root");
        assert!(dir.join(".querykey").is_dir(), ".querykey/ stays at root");
        // Legacy non-wiki entity dirs are NOT pre-created.
        assert!(!dir.join("notes").is_dir(), "no legacy notes/ at root");
        assert!(!dir.join("tasks").is_dir(), "no legacy tasks/ at root");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A pre-R15 vault has its entity dirs at <root>/<sub>/ with no
    /// wiki/. Opening it must not break those files; reads still see
    /// them; lists still surface them.
    #[test]
    fn r15_reads_legacy_entity_dir() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        // Simulate a pre-R15 vault: hand-write a person file at the
        // legacy path BEFORE Vault::open creates wiki/.
        std::fs::create_dir_all(dir.join("people")).unwrap();
        let legacy_person = compose(
            "id: person:legacy-larry\n\
             type: person\n\
             display_name: Legacy Larry\n\
             created: 2026-01-01T00:00:00Z\n",
            "# Legacy Larry\n",
        );
        std::fs::write(dir.join("people").join("legacy-larry.md"), &legacy_person).unwrap();

        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        // get_person finds the legacy file via fallback.
        let p = v.get_person("legacy-larry").expect("legacy person readable");
        assert_eq!(p.display_name, "Legacy Larry");

        // list_persons surfaces it too.
        let all = v.list_persons();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "legacy-larry");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// When the same slug exists in both wiki/ and the legacy dir
    /// (mid-migration), the canonical wiki/ version wins for reads,
    /// and it appears exactly once in lists (not duplicated).
    #[test]
    fn r15_wiki_wins_over_legacy_on_slug_collision() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        // Place a stale legacy version directly.
        std::fs::create_dir_all(dir.join("people")).unwrap();
        let legacy = compose(
            "id: person:dup\n\
             type: person\n\
             display_name: STALE NAME\n\
             created: 2026-01-01T00:00:00Z\n",
            "# stale\n",
        );
        std::fs::write(dir.join("people").join("dup.md"), &legacy).unwrap();

        // Upsert via the API — this writes to wiki/contacts/ AND
        // removes the legacy duplicate (R15-3: people→contacts).
        v.upsert_person(&Person {
            id: "dup".into(),
            display_name: "Fresh Name".into(),
            handles: vec![],
            role: String::new(),
            contact_cascade: vec![],
            created_at: Utc::now(),
        })
        .unwrap();

        // Migrate-on-write removed the legacy copy.
        assert!(
            !dir.join("people").join("dup.md").is_file(),
            "migrate-on-write should remove the legacy duplicate"
        );
        assert!(
            dir.join("wiki").join("contacts").join("dup.md").is_file(),
            "canonical write lives under wiki/contacts/"
        );

        // Single result, with the fresh data.
        let all = v.list_persons();
        assert_eq!(all.len(), 1, "no double-count of the same slug");
        assert_eq!(all[0].display_name, "Fresh Name");
        let g = v.get_person("dup").unwrap();
        assert_eq!(g.display_name, "Fresh Name");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Without an upsert, the same slug appearing in BOTH wiki/ and
    /// the legacy dir is still de-duplicated by the lister (wiki/
    /// wins on the read side). This guards the lister independently of
    /// migrate-on-write.
    #[test]
    fn r15_list_dedupes_when_both_paths_have_same_slug() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        let canonical = compose(
            "id: person:fork\n\
             type: person\n\
             display_name: Canonical\n\
             created: 2026-01-01T00:00:00Z\n",
            "# canonical\n",
        );
        let legacy = compose(
            "id: person:fork\n\
             type: person\n\
             display_name: Legacy\n\
             created: 2026-01-01T00:00:00Z\n",
            "# legacy\n",
        );
        // After R15-3 the canonical dir is wiki/contacts/; the prior
        // R15-2 form `wiki/people/` is treated as legacy.
        std::fs::create_dir_all(dir.join("wiki").join("contacts")).unwrap();
        std::fs::create_dir_all(dir.join("people")).unwrap();
        std::fs::write(
            dir.join("wiki").join("contacts").join("fork.md"),
            &canonical,
        )
        .unwrap();
        std::fs::write(dir.join("people").join("fork.md"), &legacy).unwrap();

        let all = v.list_persons();
        assert_eq!(all.len(), 1, "canonical wins — not duplicated");
        assert_eq!(all[0].display_name, "Canonical");
    }

    // ---- R15-3: people → contacts rename ----

    /// New person writes land at `wiki/contacts/`, not `wiki/people/`
    /// and not the legacy root `people/`.
    #[test]
    fn r15_3_writes_go_to_wiki_contacts() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        v.upsert_person(&Person {
            id: "ada-lovelace".into(),
            display_name: "Ada Lovelace".into(),
            handles: vec![],
            role: String::new(),
            contact_cascade: vec![],
            created_at: Utc::now(),
        })
        .unwrap();

        assert!(
            dir.join("wiki")
                .join("contacts")
                .join("ada-lovelace.md")
                .is_file(),
            "person should be written under wiki/contacts/"
        );
        assert!(
            !dir.join("wiki")
                .join("people")
                .join("ada-lovelace.md")
                .is_file(),
            "person must NOT be written to the prior wiki/people/ path"
        );
        assert!(
            !dir.join("people").join("ada-lovelace.md").is_file(),
            "person must NOT be written to the pre-R15 root people/ path"
        );

        // The canonical dir was created by Vault::open.
        assert!(dir.join("wiki").join("contacts").is_dir());
    }

    /// A vault that's at the R15-2 intermediate form (entities under
    /// wiki/, but people still in `wiki/people/` rather than the
    /// renamed `wiki/contacts/`) keeps working — reads transparently
    /// pick up the prior path.
    #[test]
    fn r15_3_reads_intermediate_wiki_people_dir() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("wiki").join("people")).unwrap();
        std::fs::write(
            dir.join("wiki").join("people").join("intermediate-ivy.md"),
            compose(
                "id: person:intermediate-ivy\n\
                 type: person\n\
                 display_name: Intermediate Ivy\n\
                 created: 2026-01-01T00:00:00Z\n",
                "# Intermediate Ivy\n",
            ),
        )
        .unwrap();

        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        let p = v
            .get_person("intermediate-ivy")
            .expect("intermediate wiki/people/ file still readable");
        assert_eq!(p.display_name, "Intermediate Ivy");
        let all = v.list_persons();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "intermediate-ivy");
    }

    /// Migrate-on-write removes BOTH legacy copies (the pre-R15 root
    /// `people/` AND the R15-2 intermediate `wiki/people/`) when the
    /// same slug is written through the canonical path.
    #[test]
    fn r15_3_upsert_clears_both_legacy_copies() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        let stale = compose(
            "id: person:two-headed\n\
             type: person\n\
             display_name: STALE\n\
             created: 2026-01-01T00:00:00Z\n",
            "# stale\n",
        );
        std::fs::create_dir_all(dir.join("wiki").join("people")).unwrap();
        std::fs::create_dir_all(dir.join("people")).unwrap();
        std::fs::write(
            dir.join("wiki").join("people").join("two-headed.md"),
            &stale,
        )
        .unwrap();
        std::fs::write(dir.join("people").join("two-headed.md"), &stale).unwrap();

        v.upsert_person(&Person {
            id: "two-headed".into(),
            display_name: "Fresh".into(),
            handles: vec![],
            role: String::new(),
            contact_cascade: vec![],
            created_at: Utc::now(),
        })
        .unwrap();

        assert!(
            dir.join("wiki")
                .join("contacts")
                .join("two-headed.md")
                .is_file(),
            "canonical copy at wiki/contacts/ exists"
        );
        assert!(
            !dir.join("wiki")
                .join("people")
                .join("two-headed.md")
                .is_file(),
            "intermediate wiki/people/ copy removed"
        );
        assert!(
            !dir.join("people").join("two-headed.md").is_file(),
            "pre-R15 root people/ copy removed"
        );

        let all = v.list_persons();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].display_name, "Fresh");
    }

    /// Other entities are NOT renamed — they keep using their API key
    /// as the canonical dir name. This guards the rename helper from
    /// accidentally generalising to entities it doesn't apply to.
    #[test]
    fn r15_3_non_people_entities_unchanged() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        let tid = uuid::Uuid::new_v4();
        v.upsert_task(&Task {
            id: tid,
            title: "Probe task".into(),
            description: "body".into(),
            status: TaskStatus::Extracted,
            assigned_to: String::new(),
            assigned_by: String::new(),
            deadline: None,
            confidence: 1.0,
            ambiguity_score: 0.0,
            source_messages: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .unwrap();

        // Task lives at wiki/tasks/, not wiki/contacts/ or some weird
        // remap. (Sanity check on the canonical-name helper.)
        assert!(dir
            .join("wiki")
            .join("tasks")
            .join(format!("{tid}.md"))
            .is_file());
    }
}

