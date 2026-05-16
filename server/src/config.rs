//! Port of server-go-old/internal/config/config.go.
//! Fuseki config replaced by Loca (embedded; a path to the .sdb dir).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Config {
    // Server
    pub host: String,
    pub port: u16,

    // Local AI agent gateway (OpenClaw bridge runs in WSL today)
    pub openclaw_gateway_url: String,
    pub openclaw_agent_id: String,
    pub openclaw_token: String,

    // Canonical markdown vault (the store of record).
    pub vault_dir: String,

    // Loca / SutraDB graph store (embedded; DERIVED from the vault).
    // Replaces the old Fuseki URL/dataset. Path to the .sdb directory.
    pub loca_db_path: String,

    // Discord bot
    pub discord_token: String,
    pub discord_guild_ids: Vec<String>,
    pub discord_batch_interval: i64, // minutes between batch processing

    // General
    pub log_level: String,
}

impl Config {
    /// Reads configuration from environment variables with sensible
    /// defaults. Mirrors config.go's Load().
    pub fn load() -> Self {
        let mut cfg = Config {
            host: env_or("SB_HOST", "127.0.0.1"),
            port: env_int("SB_PORT", 8000) as u16,
            openclaw_gateway_url: env_or("OPENCLAW_GATEWAY_URL", "http://127.0.0.1:18789"),
            openclaw_agent_id: env_or("OPENCLAW_AGENT_ID", "main"),
            openclaw_token: std::env::var("OPENCLAW_GATEWAY_TOKEN").unwrap_or_default(),
            vault_dir: resolve_vault_dir(),
            loca_db_path: env_or("LOCA_DB_PATH", "./querykey.sdb"),
            discord_token: std::env::var("DISCORD_TOKEN").unwrap_or_default(),
            discord_guild_ids: Vec::new(),
            discord_batch_interval: env_int("DISCORD_BATCH_INTERVAL", 60),
            log_level: env_or("SB_LOG_LEVEL", "info"),
        };

        if let Ok(guilds) = std::env::var("DISCORD_GUILD_IDS") {
            for g in guilds.split(',') {
                let g = g.trim();
                if !g.is_empty() {
                    cfg.discord_guild_ids.push(g.to_string());
                }
            }
        }

        if cfg.openclaw_token.is_empty() {
            cfg.openclaw_token = read_openclaw_token_from_wsl();
        }

        cfg
    }
}

fn env_or(key: &str, fallback: &str) -> String {
    match std::env::var(key) {
        Ok(v) if !v.is_empty() => v,
        _ => fallback.to_string(),
    }
}

fn env_int(key: &str, fallback: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(fallback)
}

// ---------- querykey.toml vault-root resolution (R15-1) ----------
//
// A git repo IS a QueryKey vault when it contains a `querykey.toml`
// somewhere; that file's directory is the vault root. So a repo can
// also hold non-QueryKey data — the toml file is the marker, not a
// whole-repo convention. Resolution precedence:
//
//   1. `VAULT_DIR` env override (back-compat + explicit override)
//   2. walk up from cwd to the nearest directory containing
//      `querykey.toml` (deterministic, like git's `.git` discovery or
//      cargo's `Cargo.toml` walk)
//   3. fallback `./vault` (matches the prior default)

const VAULT_ROOT_MARKER: &str = "querykey.toml";

/// Minimal parse of `querykey.toml`. v1 schema is intentionally tiny:
/// a `[querykey]` table with `version = 1` and an optional `name`.
/// Forward-extensible — unknown fields are ignored by serde defaults.
#[derive(Debug, Default, Deserialize)]
pub struct QuerykeyToml {
    #[serde(default)]
    pub querykey: QuerykeyTable,
}

#[derive(Debug, Default, Deserialize)]
pub struct QuerykeyTable {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub name: String,
}

/// Resolve the vault directory using `current_dir()` as the walk-up
/// start. Public wrapper around [`resolve_vault_dir_from`] for prod.
pub fn resolve_vault_dir() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    resolve_vault_dir_from(&cwd)
}

/// Resolution rule (see module doc). Takes the walk-up start as a
/// parameter so it can be exercised in tests without mutating the
/// process-global cwd.
pub fn resolve_vault_dir_from(start: &Path) -> String {
    if let Ok(v) = std::env::var("VAULT_DIR") {
        if !v.is_empty() {
            return v;
        }
    }
    if let Some(root) = find_vault_root(start) {
        return root.to_string_lossy().into_owned();
    }
    "./vault".to_string()
}

/// Walk up from `start` looking for `querykey.toml`. Returns the
/// directory that contains it (the vault root), not the file path.
pub fn find_vault_root(start: &Path) -> Option<PathBuf> {
    let mut cur: Option<&Path> = Some(start);
    while let Some(dir) = cur {
        if dir.join(VAULT_ROOT_MARKER).is_file() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }
    None
}

/// Parse a `querykey.toml`. Returns `Err` if the file is unreadable or
/// not valid TOML; returns `Ok(default)` for an empty file (schema is
/// permissive — every field has a default).
pub fn parse_querykey_toml(path: &Path) -> anyhow::Result<QuerykeyToml> {
    let s = std::fs::read_to_string(path)?;
    let cfg: QuerykeyToml = toml::from_str(&s)?;
    Ok(cfg)
}

/// Reads the OpenClaw gateway token from `~/.openclaw/openclaw.json`,
/// going through WSL on Windows. Mirrors config.go.
fn read_openclaw_token_from_wsl() -> String {
    if std::env::var("OS").as_deref() != Ok("Windows_NT") {
        if let Some(home) = dirs_home() {
            return read_token_from_file(&format!("{home}/.openclaw/openclaw.json"));
        }
        return String::new();
    }

    match Command::new("wsl")
        .args(["-d", "Ubuntu", "--", "bash", "-lc", "cat ~/.openclaw/openclaw.json"])
        .output()
    {
        Ok(out) if out.status.success() => parse_token(&out.stdout),
        Ok(_) | Err(_) => {
            tracing::warn!("[config] could not read OpenClaw config from WSL");
            String::new()
        }
    }
}

fn dirs_home() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
}

fn read_token_from_file(path: &str) -> String {
    match std::fs::read(path) {
        Ok(data) => parse_token(&data),
        Err(_) => String::new(),
    }
}

fn parse_token(data: &[u8]) -> String {
    let v: serde_json::Value = match serde_json::from_slice(data) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    v.get("gateway")
        .and_then(|g| g.get("auth"))
        .and_then(|a| a.get("token"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    //! Tests for `resolve_vault_dir_from` precedence and walk-up.
    //!
    //! Honesty: `VAULT_DIR` is process-global; setting it inside one
    //! test could leak into another that runs concurrently and assumes
    //! it is unset. The two tests that touch it serialize on a mutex
    //! and clear it on entry + exit (RAII guard). Walk-up tests do not
    //! touch any env var, so they run freely.
    //!
    //! Tempdirs are built with the existing `uuid` dep (no new
    //! dev-dependency) and cleaned up via Drop.
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }
    impl EnvGuard {
        fn new() -> Self {
            let lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
            std::env::remove_var("VAULT_DIR");
            Self { _lock: lock }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var("VAULT_DIR");
        }
    }

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let p = std::env::temp_dir()
                .join(format!("querykey-cfg-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&p).expect("mk tempdir");
            Self(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write(p: &Path, body: &str) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("mk parent");
        }
        std::fs::write(p, body).expect("write");
    }

    #[test]
    fn env_override_beats_walk_up_and_fallback() {
        let _g = EnvGuard::new();
        let td = TempDir::new();
        // A querykey.toml exists at the cwd start — walk-up WOULD find it.
        write(&td.path().join("querykey.toml"), "[querykey]\nversion = 1\n");
        std::env::set_var("VAULT_DIR", "C:/explicit/override");
        let got = resolve_vault_dir_from(td.path());
        assert_eq!(got, "C:/explicit/override");
    }

    #[test]
    fn empty_env_is_ignored() {
        // An empty VAULT_DIR must NOT count as "set" — it should fall
        // through to walk-up (mirrors how env_or treats empty strings).
        let _g = EnvGuard::new();
        let td = TempDir::new();
        write(&td.path().join("querykey.toml"), "[querykey]\nversion = 1\n");
        std::env::set_var("VAULT_DIR", "");
        let got = resolve_vault_dir_from(td.path());
        assert_eq!(PathBuf::from(&got), td.path().to_path_buf());
    }

    #[test]
    fn walk_up_finds_marker_at_start_dir() {
        let td = TempDir::new();
        write(&td.path().join("querykey.toml"), "[querykey]\nversion = 1\n");
        let root = find_vault_root(td.path()).expect("found");
        assert_eq!(root, td.path());
    }

    #[test]
    fn walk_up_finds_marker_several_levels_up() {
        let td = TempDir::new();
        write(&td.path().join("querykey.toml"), "[querykey]\nversion = 1\n");
        let deep = td.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&deep).unwrap();
        let root = find_vault_root(&deep).expect("found from deep cwd");
        assert_eq!(root, td.path());
    }

    #[test]
    fn walk_up_picks_nearest_marker_when_nested() {
        // If both a parent and an intermediate dir contain a marker,
        // the NEAREST (nested) one wins — matches git/cargo semantics.
        let td = TempDir::new();
        write(&td.path().join("querykey.toml"), "[querykey]\nversion = 1\n");
        let nested = td.path().join("inner");
        write(&nested.join("querykey.toml"), "[querykey]\nversion = 1\n");
        let deep = nested.join("x").join("y");
        std::fs::create_dir_all(&deep).unwrap();
        let root = find_vault_root(&deep).expect("found");
        assert_eq!(root, nested);
    }

    #[test]
    fn fallback_when_neither_env_nor_marker() {
        let _g = EnvGuard::new();
        let td = TempDir::new(); // no querykey.toml written
        let got = resolve_vault_dir_from(td.path());
        assert_eq!(got, "./vault");
    }

    #[test]
    fn parse_querykey_toml_minimal() {
        let td = TempDir::new();
        let p = td.path().join("querykey.toml");
        write(&p, "[querykey]\nversion = 1\nname = \"my-vault\"\n");
        let cfg = parse_querykey_toml(&p).expect("parse");
        assert_eq!(cfg.querykey.version, 1);
        assert_eq!(cfg.querykey.name, "my-vault");
    }

    #[test]
    fn parse_querykey_toml_empty_is_ok() {
        // Empty file = all-defaults. The marker's presence is the
        // load-bearing signal; field contents are optional.
        let td = TempDir::new();
        let p = td.path().join("querykey.toml");
        write(&p, "");
        let cfg = parse_querykey_toml(&p).expect("parse empty");
        assert_eq!(cfg.querykey.version, 0);
        assert_eq!(cfg.querykey.name, "");
    }

    #[test]
    fn parse_querykey_toml_ignores_unknown_fields() {
        // Forward-extensibility: adding fields later must not break
        // older binaries trying to read newer files.
        let td = TempDir::new();
        let p = td.path().join("querykey.toml");
        write(
            &p,
            "[querykey]\nversion = 1\nfuture_field = true\n\n[other]\nx = 42\n",
        );
        let cfg = parse_querykey_toml(&p).expect("parse with extras");
        assert_eq!(cfg.querykey.version, 1);
    }

    #[test]
    fn parse_querykey_toml_rejects_malformed() {
        let td = TempDir::new();
        let p = td.path().join("querykey.toml");
        write(&p, "this is = = not toml [[[");
        assert!(parse_querykey_toml(&p).is_err());
    }
}
