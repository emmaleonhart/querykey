//! Canonical markdown source of truth — implements
//! `docs/markdown-schema.md`.
//!
//! Markdown files on disk ARE the store of record. The Loca graph is a
//! derived, rebuildable index *projected from* this vault, never the
//! other way round. Layout:
//!
//! ```text
//! <root>/people/<id>.md   tasks/<uuid>.md   events/<uuid>.md   notes/
//! ```
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
    Handle, OpenQuestion, Person, QuestionStatus, Task, TaskStatus, TriggerType, Urgency,
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

/// Resolution/comparison slug: lowercase, non-alphanumeric runs become
/// a single `-`, trimmed. So `John  Smith`, `john-smith`, and
/// `John-Smith` all compare equal.
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

impl Vault {
    pub fn open(root: &str) -> anyhow::Result<Self> {
        let root = PathBuf::from(root);
        for sub in [
            "people",
            "tasks",
            "events",
            "notes",
            "conflicts",
            "questions",
            "followups",
            "peers",     // others' cards — git-ignored (asymmetry)
            ".querykey", // derived cache / propagation state — git-ignored
        ] {
            fs::create_dir_all(root.join(sub))?;
        }
        let v = Self { root };
        v.ensure_privacy_gitignore()?;
        Ok(v)
    }

    pub fn root(&self) -> &Path {
        &self.root
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
        let p = self.root.join(sub).join(format!("{slug}.md"));
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

    fn write(&self, sub: &str, slug: &str, yaml: String, body: &str) -> anyhow::Result<()> {
        let path = self.root.join(sub).join(format!("{slug}.md"));
        fs::write(path, compose(&yaml, body))?;
        Ok(())
    }

    fn read_files(&self, sub: &str) -> Vec<(String, String)> {
        let dir = self.root.join(sub);
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("md") {
                    if let Ok(s) = fs::read_to_string(&p) {
                        let slug = p
                            .file_stem()
                            .and_then(|x| x.to_str())
                            .unwrap_or("")
                            .to_string();
                        out.push((slug, s));
                    }
                }
            }
        }
        out
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
        let p = self.root.join("people").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
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
        let p = self.root.join("tasks").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
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
        let p = self.root.join("conflicts").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
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
        let p = self.root.join("questions").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
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
        let p = self.root.join("followups").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
        self.followup_from(id, &s)
    }

    pub fn list_followups(&self) -> Vec<FollowUp> {
        self.read_files("followups")
            .into_iter()
            .filter_map(|(slug, c)| self.followup_from(&slug, &c))
            .collect()
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
        fs::write(
            dir.join("notes").join("salon.md"),
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
}

