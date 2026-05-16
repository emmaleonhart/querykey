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

use crate::vault::{compose, parse_dt, rfc3339, split};

fn default_visibility() -> String {
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
