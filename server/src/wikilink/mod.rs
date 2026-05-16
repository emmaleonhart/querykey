//! Semantic wikilinks — the `[[link]]` / `[[property:link]]` syntax
//! that turns a person's (or note's) freeform markdown body into
//! derived graph triples.
//!
//! - `[[Target]]` — an **untyped reference** to an entity.
//! - `[[property:Target]]` — a **semantic** link: the part before the
//!   first `:` is the *predicate* ("the triple type"), the rest is the
//!   target entity. So `[[employer:Acme Corp]]` ⇒ a triple
//!   `(this entity) —employer→ (Acme Corp)`.
//! - Single colon **on purpose** — NOT Semantic-MediaWiki's ugly `::`
//!   (an accidental `[[p::T]]` is still parsed forgivingly).
//! - `[[Target|Alias]]` — the Obsidian display alias is kept off the
//!   edge (display-only); the link target is the left side.
//!
//! This module is the pure parser. Resolving a target to a concrete
//! entity (and dangling-link handling) is the vault's job — it knows
//! the entities.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// The predicate. `None` for a plain `[[X]]` (untyped reference).
    pub property: Option<String>,
    /// The target as written (an entity name/slug); resolved later.
    pub target: String,
}

/// Schemes that must NOT be mistaken for a predicate, so a bare
/// `[[https://example.com]]` is an (untyped) link, not `https:`.
const URI_SCHEMES: &[&str] = &["http", "https", "ftp", "mailto", "file", "data", "tel"];

/// A predicate token is a lowercase word: `[a-z][a-z0-9_-]*` and not a
/// URI scheme. Anything else before the `:` means "no predicate"
/// (treat the whole inner text as the target).
fn is_property_token(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty()
        && s.chars()
            .next()
            .map(|c| c.is_ascii_lowercase())
            .unwrap_or(false)
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        && !URI_SCHEMES.contains(&s)
}

/// Extract every `[[…]]` from a markdown body, in document order.
/// Tolerant of surrounding prose. Nested brackets and code-fence
/// exclusion are documented future refinements (rare in practice).
pub fn parse(body: &str) -> Vec<WikiLink> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else { break };
        if let Some(link) = parse_inner(&after[..end]) {
            out.push(link);
        }
        rest = &after[end + 2..];
    }
    out
}

fn parse_inner(inner: &str) -> Option<WikiLink> {
    // Drop an Obsidian display alias: [[target|Alias]].
    let link_part = inner.split('|').next().unwrap_or(inner).trim();
    if link_part.is_empty() {
        return None;
    }
    if let Some((pre, post)) = link_part.split_once(':') {
        // Forgive an accidental `::` (SMW habit) by stripping the
        // extra leading colons from the target.
        let target = post.trim_start_matches(':').trim();
        if is_property_token(pre) && !target.is_empty() {
            return Some(WikiLink {
                property: Some(pre.trim().to_string()),
                target: target.to_string(),
            });
        }
    }
    Some(WikiLink {
        property: None,
        target: link_part.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(s: &str) -> WikiLink {
        let v = parse(s);
        assert_eq!(v.len(), 1, "expected exactly one link in {s:?}: {v:?}");
        v.into_iter().next().unwrap()
    }

    #[test]
    fn plain_link_is_untyped() {
        let l = one("met [[John Smith]] at the gym");
        assert_eq!(l.property, None);
        assert_eq!(l.target, "John Smith");
    }

    #[test]
    fn property_link_is_a_typed_triple() {
        let l = one("works at [[employer:Acme Corp]] now");
        assert_eq!(l.property.as_deref(), Some("employer"));
        assert_eq!(l.target, "Acme Corp");
    }

    #[test]
    fn obsidian_alias_is_display_only() {
        let l = one("see [[john-smith|John]]");
        assert_eq!(l.property, None);
        assert_eq!(l.target, "john-smith");

        let l = one("[[knows:john-smith|John S.]]");
        assert_eq!(l.property.as_deref(), Some("knows"));
        assert_eq!(l.target, "john-smith");
    }

    #[test]
    fn urls_are_not_predicates() {
        let l = one("[[https://example.com]]");
        assert_eq!(l.property, None);
        assert_eq!(l.target, "https://example.com");
    }

    #[test]
    fn forgives_accidental_double_colon() {
        let l = one("[[knows::John]]");
        assert_eq!(l.property.as_deref(), Some("knows"));
        assert_eq!(l.target, "John");
    }

    #[test]
    fn multiple_and_empty() {
        let v = parse("[[a]] noise [[rel:b]] and [[]] and [[ | x]]");
        assert_eq!(
            v,
            vec![
                WikiLink { property: None, target: "a".into() },
                WikiLink { property: Some("rel".into()), target: "b".into() },
            ]
        );
    }

    #[test]
    fn uppercase_or_spaced_prefix_is_not_a_predicate() {
        // "See" is capitalized → not a predicate; whole thing is target.
        let l = one("[[See: the appendix]]");
        assert_eq!(l.property, None);
        assert_eq!(l.target, "See: the appendix");
    }
}
