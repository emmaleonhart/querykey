//! Port of server-go-old/internal/config/config.go.
//! Fuseki config replaced by Loca (embedded; a path to the .sdb dir).

use std::process::Command;

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
            vault_dir: env_or("VAULT_DIR", "./vault"),
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
