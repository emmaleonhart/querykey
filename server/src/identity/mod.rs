//! Identity abstraction. Per `docs/card-format.md` ("Identity &
//! discovery"): *"a user is a canonical handle that currently resolves
//! via GitHub."* Kept deliberately thin so GitHub is **not baked into
//! call sites** and can be swapped for DIDs/Nostr later without a
//! rewrite — call sites depend on the trait, never on the concrete
//! provider.
//!
//! Scope note: *discovery* (whose cards you pull — "follow on
//! GitHub") is part of the unresolved P2P **transport** question and
//! is intentionally NOT implemented here. This module is pure local
//! handle normalization/derivation — no network calls.

use std::fmt;

/// `scheme:localpart`, e.g. `github:jsmith`. The single canonical
/// identifier for a user across QueryKey (cards, peers, refs).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalHandle(String);

impl CanonicalHandle {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn scheme(&self) -> &str {
        self.0.split_once(':').map(|(s, _)| s).unwrap_or("")
    }
    pub fn localpart(&self) -> &str {
        self.0.split_once(':').map(|(_, l)| l).unwrap_or(&self.0)
    }
}

impl fmt::Display for CanonicalHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Pluggable identity backend. Swap this (DID/Nostr) without touching
/// call sites — they depend on the trait, never on `GitHubIdentity`.
pub trait IdentityProvider: Send + Sync {
    fn scheme(&self) -> &str;
    /// Normalize raw user input — `jsmith`, `github:jsmith`, `@jsmith`,
    /// `https://github.com/jsmith` — to one `CanonicalHandle`.
    fn normalize(&self, raw: &str) -> CanonicalHandle;
    /// Best-effort public profile URL, derived locally (no fetch).
    fn profile_url(&self, h: &CanonicalHandle) -> Option<String>;
}

/// GitHub bootstrap: username = handle (the swappable default).
pub struct GitHubIdentity;

impl IdentityProvider for GitHubIdentity {
    fn scheme(&self) -> &str {
        "github"
    }

    fn normalize(&self, raw: &str) -> CanonicalHandle {
        let r = raw.trim();
        let local = r
            .strip_prefix("https://github.com/")
            .or_else(|| r.strip_prefix("http://github.com/"))
            .or_else(|| r.strip_prefix("github.com/"))
            .or_else(|| r.strip_prefix("github:"))
            .or_else(|| r.strip_prefix('@'))
            .unwrap_or(r)
            .trim_matches('/')
            .trim();
        CanonicalHandle(format!("github:{local}"))
    }

    fn profile_url(&self, h: &CanonicalHandle) -> Option<String> {
        if h.scheme() == "github" && !h.localpart().is_empty() {
            Some(format!("https://github.com/{}", h.localpart()))
        } else {
            None
        }
    }
}

static GITHUB: GitHubIdentity = GitHubIdentity;

/// The provider call sites use. This is the **only** place the
/// concrete choice (GitHub, today) is named.
pub fn default_provider() -> &'static dyn IdentityProvider {
    &GITHUB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_every_input_form_to_one_handle() {
        let p = default_provider();
        for raw in [
            "jsmith",
            "github:jsmith",
            "@jsmith",
            "https://github.com/jsmith",
            "github.com/jsmith/",
            "  jsmith  ",
        ] {
            let h = p.normalize(raw);
            assert_eq!(h.as_str(), "github:jsmith", "input: {raw:?}");
            assert_eq!(h.scheme(), "github");
            assert_eq!(h.localpart(), "jsmith");
        }
    }

    #[test]
    fn profile_url_is_local_only() {
        let p = default_provider();
        let h = p.normalize("jsmith");
        assert_eq!(
            p.profile_url(&h).as_deref(),
            Some("https://github.com/jsmith")
        );
        // Unknown scheme → no URL (don't guess).
        let other = CanonicalHandle("did:example:123".into());
        assert_eq!(p.profile_url(&other), None);
        assert_eq!(other.scheme(), "did");
    }
}
