//! Port of server-go-old/internal/openclaw/wsl.go.
//! WSL detection + OpenClaw gateway process management.

use std::process::Command;
use std::time::Duration;

pub struct WslManager {
    distro: String,
}

impl Default for WslManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WslManager {
    pub fn new() -> Self {
        let distro = find_distro();
        WslManager { distro }
    }

    pub fn is_available(&self) -> bool {
        if self.distro.is_empty() {
            return false;
        }
        Command::new("wsl")
            .args(["-l", "-q"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn distro(&self) -> &str {
        &self.distro
    }

    /// Run a command inside WSL. Returns (stdout, stderr, exit_code).
    pub fn run_command(&self, command: &str, _timeout: Duration) -> (String, String, i32) {
        match Command::new("wsl")
            .args(["-d", &self.distro, "--", "bash", "-lc", command])
            .output()
        {
            Ok(o) => (
                String::from_utf8_lossy(&o.stdout).into_owned(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
                o.status.code().unwrap_or(-1),
            ),
            Err(e) => (String::new(), e.to_string(), -1),
        }
    }

    /// Port of wsl.go CleanStaleLockFiles().
    pub fn clean_stale_lock_files(&self) -> anyhow::Result<()> {
        if self.distro.is_empty() {
            anyhow::bail!("no WSL distro found");
        }
        let status = Command::new("wsl")
            .args([
                "-d",
                &self.distro,
                "--",
                "bash",
                "-c",
                "rm -f /tmp/openclaw-*/gateway.*.lock",
            ])
            .status();
        match status {
            Ok(s) if s.success() => {
                tracing::info!("[wsl] cleaned stale lock files");
                Ok(())
            }
            Ok(s) => anyhow::bail!("lock cleanup exited {s}"),
            Err(e) => anyhow::bail!("lock cleanup failed: {e}"),
        }
    }

    /// Port of wsl.go ForceKillOpenClaw() — the big red button.
    pub fn force_kill_openclaw(&self) -> anyhow::Result<()> {
        if self.distro.is_empty() {
            anyhow::bail!("no WSL distro found");
        }
        let status = Command::new("wsl")
            .args([
                "-d",
                &self.distro,
                "--",
                "bash",
                "-c",
                "pkill -f openclaw-gateway; pkill -f openclaw; \
                 rm -f /tmp/openclaw-*/gateway.*.lock; true",
            ])
            .status();
        match status {
            Ok(_) => {
                tracing::info!("[wsl] force-killed all OpenClaw processes");
                Ok(())
            }
            Err(e) => anyhow::bail!("force kill failed: {e}"),
        }
    }

    /// Port of wsl.go StartGateway(): clean locks, spawn the gateway,
    /// return the child so the caller manages its lifecycle.
    pub fn start_gateway(&self) -> anyhow::Result<tokio::process::Child> {
        if self.distro.is_empty() {
            anyhow::bail!("no WSL distro found");
        }
        let _ = self.clean_stale_lock_files();
        let child = tokio::process::Command::new("wsl")
            .args(["-d", &self.distro, "-e", "bash", "-lc", "openclaw gateway"])
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| anyhow::anyhow!("failed to start OpenClaw gateway: {e}"))?;
        tracing::info!("[wsl] spawned OpenClaw gateway (pid {:?})", child.id());
        Ok(child)
    }
}

/// Port of wsl.go findDistro(): `wsl --list --quiet`, strip the null
/// bytes Windows emits, prefer Ubuntu, else first non-docker distro.
fn find_distro() -> String {
    if !cfg!(windows) {
        return String::new();
    }
    let out = match Command::new("wsl").args(["--list", "--quiet"]).output() {
        Ok(o) => o.stdout,
        Err(_) => return String::new(),
    };
    let text = String::from_utf8_lossy(&out);
    let distros: Vec<String> = text
        .lines()
        .map(|l| l.trim().replace('\u{0}', ""))
        .filter(|l| !l.is_empty())
        .collect();
    if let Some(u) = distros.iter().find(|d| d.to_lowercase().contains("ubuntu")) {
        return u.clone();
    }
    if let Some(d) = distros
        .iter()
        .find(|d| !d.to_lowercase().contains("docker"))
    {
        return d.clone();
    }
    String::new()
}
