//! The P2P broadcast **card** — implements `docs/card-format.md`.
//!
//! `card.md` at the vault root: a lean, curated **key/query** signal
//! (what you offer / what you're looking for), *not* a profile. This
//! module owns the **format** (parse/render, the stable
//! `## Offering` / `## Looking for` heading contract) only.
//!
//! Out of scope here, on purpose: the **transport** that actually
//! moves a card between peers is the highest open question in the doc
//! and the format must not assume it — so there is no exchange/relay
//! code in this module. Identity resolution lives in `crate::identity`
//! (a card's `handle` is a canonical handle, currently GitHub).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::vault::{compose, parse_dt, rfc3339, split, PrmDigest};

pub(crate) fn default_visibility() -> String {
    "public".into()
}

/// A user's broadcast card. `handle` is the identity bootstrap
/// (`github:<user>` today, swappable). `offering` is the **key**,
/// `looking_for` is the **query**. There is deliberately no `value`
/// field — V is the real-world outcome, never stored or scored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Card {
    pub handle: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub offering: Vec<String>,
    #[serde(default)]
    pub looking_for: Vec<String>,
    pub updated: DateTime<Utc>,
    #[serde(default = "default_visibility")]
    pub visibility: String,
}

/// Frontmatter (stable key order). `name`/`bio`/`offering`/
/// `looking_for` live in the human body, not here.
#[derive(Serialize, Deserialize)]
struct CardFm {
    id: String,
    handle: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    website: String,
    updated: String,
    #[serde(default = "default_visibility")]
    visibility: String,
}

/// `card:<localpart>` derived from the handle (`github:jsmith` →
/// `card:jsmith`); falls back to the whole handle if unprefixed.
fn card_id(handle: &str) -> String {
    let local = handle.split_once(':').map(|(_, l)| l).unwrap_or(handle);
    format!("card:{local}")
}

/// Render a Card to the canonical `card.md` text. The body shape is
/// fixed so write→read→write is stable; the `## Offering (key)` /
/// `## Looking for (query)` headings are the machine-parseable
/// contract (kept stable on purpose — peers' parsers key off them).
pub fn render(c: &Card) -> String {
    let fm = CardFm {
        id: card_id(&c.handle),
        handle: c.handle.clone(),
        website: c.website.clone(),
        updated: rfc3339(&c.updated),
        visibility: c.visibility.clone(),
    };
    let yaml = serde_yaml::to_string(&fm).unwrap_or_default();

    let mut body = String::new();
    let name = if c.name.is_empty() { &c.handle } else { &c.name };
    body.push_str(&format!("# {name} — card\n"));
    if !c.bio.is_empty() {
        body.push_str(&format!("\n> {}\n", c.bio));
    }
    body.push_str("\n## Offering (key)\n");
    for o in &c.offering {
        body.push_str(&format!("- {o}\n"));
    }
    body.push_str("\n## Looking for (query)\n");
    for q in &c.looking_for {
        body.push_str(&format!("- {q}\n"));
    }
    compose(&yaml, &body)
}

#[derive(PartialEq)]
enum Section {
    None,
    Offering,
    LookingFor,
}

/// Parse `card.md` text back into a Card. Robust to the heading
/// suffixes (`(key)`/`(query)`) and to extra prose — only the bullet
/// lists under the Offering / Looking-for headings are contractual.
pub fn parse(content: &str) -> Option<Card> {
    let (yaml, body) = split(content);
    let fm: CardFm = serde_yaml::from_str(&yaml).ok()?;

    let mut name = String::new();
    let mut bio_lines: Vec<String> = Vec::new();
    let mut offering: Vec<String> = Vec::new();
    let mut looking_for: Vec<String> = Vec::new();
    let mut sec = Section::None;

    for raw in body.lines() {
        let line = raw.trim_end();
        if let Some(h2) = line.strip_prefix("## ") {
            let l = h2.to_ascii_lowercase();
            sec = if l.contains("offering") {
                Section::Offering
            } else if l.contains("looking for") {
                Section::LookingFor
            } else {
                Section::None
            };
        } else if let Some(h1) = line.strip_prefix("# ") {
            // "Emma — card" → "Emma" (em dash or hyphen separator).
            name = h1
                .split(" — ")
                .next()
                .unwrap_or(h1)
                .split(" - ")
                .next()
                .unwrap_or(h1)
                .trim()
                .to_string();
        } else if let Some(q) = line.strip_prefix("> ") {
            bio_lines.push(q.trim().to_string());
        } else if let Some(item) = line.strip_prefix("- ") {
            let item = item.trim().to_string();
            match sec {
                Section::Offering => offering.push(item),
                Section::LookingFor => looking_for.push(item),
                Section::None => {}
            }
        }
    }

    Some(Card {
        handle: fm.handle,
        name,
        website: fm.website,
        bio: bio_lines.join(" "),
        offering,
        looking_for,
        updated: parse_dt(&fm.updated),
        visibility: fm.visibility,
    })
}

// ---- agent-drafted card (PRM → key/query) ----
//
// The local agent, which has been building the PRM by observing the
// user, drafts the card's key/query; the user reviews and approves
// (the existing PUT /api/card — plus the 24h propagation valve). The
// agent only proposes bio/offering/looking_for; identity fields are
// carried from `base` (the user's, not the model's, to set). No
// model is named here — any agent, governed by `agents.md`.

/// The model-agnostic drafting instruction: attend over the PRM
/// digest within the (optional) `agents.md` envelope, reply JSON.
pub fn draft_prompt(digest: &PrmDigest, agents_md: Option<&str>) -> String {
    let mut p = String::new();
    if let Some(a) = agents_md {
        p.push_str("# agents.md (your governing envelope)\n");
        p.push_str(a.trim());
        p.push_str("\n\n");
    }
    p.push_str(
        "You are drafting the user's QueryKey card from the private \
         relationship/task graph below. The card is a lean signal: a \
         `key` (what they can offer) and a `query` (what they're \
         looking for). Be concrete, humble and brief; do NOT invent \
         facts unsupported by the digest. Reply with ONLY a JSON \
         object: {\"bio\": one-line string, \"offering\": [string], \
         \"looking_for\": [string]}.\n\n# PRM digest\n",
    );
    p.push_str(&format!(
        "people: {}, tasks: {}, events: {}, notes: {}\n",
        digest.person_count, digest.task_count, digest.event_count, digest.note_count
    ));
    if !digest.offers.is_empty() {
        p.push_str(&format!("explicit offers: {}\n", digest.offers.join("; ")));
    }
    if !digest.wants.is_empty() {
        p.push_str(&format!("explicit wants: {}\n", digest.wants.join("; ")));
    }
    if !digest.predicates.is_empty() {
        p.push_str(&format!(
            "relation vocabulary: {}\n",
            digest.predicates.join(", ")
        ));
    }
    if !digest.top_people.is_empty() {
        let tp: Vec<String> = digest
            .top_people
            .iter()
            .map(|x| {
                if x.role.is_empty() {
                    x.display_name.clone()
                } else {
                    format!("{} ({})", x.display_name, x.role)
                }
            })
            .collect();
        p.push_str(&format!("most-referenced people: {}\n", tp.join("; ")));
    }
    if !digest.active_tasks.is_empty() {
        p.push_str(&format!("active tasks: {}\n", digest.active_tasks.join("; ")));
    }
    if let Some(c) = &digest.current_card {
        p.push_str(&format!(
            "\n# current card (refine, don't reset)\nbio: {}\noffering: {}\nlooking_for: {}\n",
            c.bio,
            c.offering.join("; "),
            c.looking_for.join("; ")
        ));
    }
    p
}

#[derive(Deserialize)]
struct DraftReply {
    #[serde(default)]
    bio: String,
    #[serde(default)]
    offering: Vec<String>,
    #[serde(default)]
    looking_for: Vec<String>,
}

/// Parse the agent's JSON reply into a draft card. Identity fields
/// (handle/name/website/visibility) are carried from `base` — the
/// agent only proposes bio/offering/looking_for. `None` if the reply
/// has no JSON object.
pub fn parse_draft_reply(reply: &str, base: &Card) -> Option<Card> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    if end < start {
        return None;
    }
    let dr: DraftReply = serde_json::from_str(&reply[start..=end]).ok()?;
    Some(Card {
        handle: base.handle.clone(),
        name: base.name.clone(),
        website: base.website.clone(),
        bio: if dr.bio.trim().is_empty() {
            base.bio.clone()
        } else {
            dr.bio.trim().to_string()
        },
        offering: dr.offering,
        looking_for: dr.looking_for,
        updated: Utc::now(),
        visibility: base.visibility.clone(),
    })
}

/// Deterministic fallback when no agent is reachable: surface the
/// graph's explicit offer/want signals verbatim — epistemically
/// humble, proposes only what the user themselves tagged
/// (`[[offers:…]]` / `[[wants:…]]`) — plus a stub bio to edit.
pub fn heuristic_draft(digest: &PrmDigest, base: &Card) -> Card {
    let bio = if !base.bio.is_empty() {
        base.bio.clone()
    } else if digest.offers.is_empty() && digest.wants.is_empty() {
        "(draft stub — add your key/query, or tag relations with \
         [[offers:…]] / [[wants:…]] so the agent can draft them)"
            .to_string()
    } else {
        String::new()
    };
    Card {
        handle: base.handle.clone(),
        name: base.name.clone(),
        website: base.website.clone(),
        bio,
        offering: digest.offers.clone(),
        looking_for: digest.wants.clone(),
        updated: Utc::now(),
        visibility: base.visibility.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_round_trips_losslessly() {
        let c = Card {
            handle: "github:jsmith".into(),
            name: "Emma".into(),
            website: "https://emmaleonhart.com".into(),
            bio: "builds local-first tools; rationalist-adjacent.".into(),
            offering: vec![
                "Rust / embedded DB help (built a graph-vector-time DB)".into(),
                "Intros into the rationalist/EA Vancouver scene".into(),
            ],
            looking_for: vec![
                "Flutter desktop reviewers".into(),
                "A co-author for a NeurIPS-style writeup".into(),
            ],
            updated: DateTime::parse_from_rfc3339("2026-05-15T22:10:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
            visibility: "public".into(),
        };
        let text = render(&c);
        // The heading contract must be present and stable.
        assert!(text.contains("## Offering (key)"));
        assert!(text.contains("## Looking for (query)"));
        assert!(text.contains("id: card:jsmith"));

        let back = parse(&text).unwrap();
        assert_eq!(back, c);

        // Idempotent: render(parse(render(c))) == render(c).
        assert_eq!(render(&parse(&text).unwrap()), text);
    }

    fn empty_card() -> Card {
        Card {
            handle: "github:emma".into(),
            name: "Emma".into(),
            website: String::new(),
            bio: String::new(),
            offering: vec![],
            looking_for: vec![],
            updated: Utc::now(),
            visibility: "public".into(),
        }
    }

    fn digest(offers: &[&str], wants: &[&str]) -> PrmDigest {
        PrmDigest {
            person_count: 4,
            task_count: 2,
            event_count: 1,
            note_count: 3,
            top_people: vec![],
            predicates: vec!["offers".into(), "knows".into()],
            offers: offers.iter().map(|s| s.to_string()).collect(),
            wants: wants.iter().map(|s| s.to_string()).collect(),
            active_tasks: vec!["Finish R12".into()],
            current_card: None,
        }
    }

    #[test]
    fn heuristic_uses_explicit_signals_only() {
        let d = digest(&["Rust / embedded DB help"], &["Flutter reviewers"]);
        let c = heuristic_draft(&d, &empty_card());
        assert_eq!(c.offering, vec!["Rust / embedded DB help"]);
        assert_eq!(c.looking_for, vec!["Flutter reviewers"]);
        assert_eq!(c.handle, "github:emma"); // identity carried, not invented
        assert!(c.bio.is_empty()); // had signals, no stub needed

        // No signals → an explicit stub, no fabricated claims.
        let c2 = heuristic_draft(&digest(&[], &[]), &empty_card());
        assert!(c2.offering.is_empty() && c2.looking_for.is_empty());
        assert!(c2.bio.contains("draft stub"));
    }

    #[test]
    fn parse_draft_reply_extracts_json_and_carries_identity() {
        let base = empty_card();
        let reply = "Sure! Here is the draft:\n\
            {\"bio\": \"builds local-first tools\", \
             \"offering\": [\"Rust help\"], \"looking_for\": [\"reviewers\"]}\n\
            Hope that works.";
        let c = parse_draft_reply(reply, &base).unwrap();
        assert_eq!(c.bio, "builds local-first tools");
        assert_eq!(c.offering, vec!["Rust help"]);
        assert_eq!(c.looking_for, vec!["reviewers"]);
        assert_eq!(c.handle, "github:emma"); // never the model's to set
        assert!(parse_draft_reply("no json here", &base).is_none());
    }

    #[test]
    fn draft_prompt_is_model_agnostic_and_includes_envelope() {
        let p = draft_prompt(&digest(&["X"], &[]), Some("Be terse. Prefer verbs."));
        assert!(p.contains("agents.md"));
        assert!(p.contains("Be terse. Prefer verbs."));
        assert!(p.contains("JSON")); // structured reply contract
        assert!(p.contains("people: 4, tasks: 2"));
        assert!(p.contains("explicit offers: X"));
        // No model/engine named anywhere.
        let low = p.to_lowercase();
        assert!(!low.contains("gemma") && !low.contains("openclaw") && !low.contains("gpt"));
    }

    #[test]
    fn parse_tolerates_missing_bio_and_empty_lists() {
        let c = Card {
            handle: "github:nobody".into(),
            name: String::new(),
            website: String::new(),
            bio: String::new(),
            offering: vec![],
            looking_for: vec![],
            updated: Utc::now(),
            visibility: "public".into(),
        };
        let back = parse(&render(&c)).unwrap();
        assert_eq!(back.handle, "github:nobody");
        // Name falls back to the handle in the heading on render.
        assert_eq!(back.name, "github:nobody");
        assert!(back.bio.is_empty());
        assert!(back.offering.is_empty());
        assert!(back.looking_for.is_empty());
    }
}
