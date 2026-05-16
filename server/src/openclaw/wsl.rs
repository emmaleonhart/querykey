//! Port of server-go-old/internal/openclaw/wsl.go.
//! WSL detection + command execution helpers. Heavy gateway lifecycle
//! is skeletal with TODOs against the Go reference.

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
        let mut m = WslManager {
            distro: "Ubuntu".to_string(),
        };
        m.distro = m.find_distro();
        m
    }

    pub fn is_available(&self) -> bool {
        Command::new("wsl")
            .args(["-l", "-q"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn distro(&self) -> &str {
        &self.distro
    }

    fn find_distro(&self) -> String {
        // TODO(port): parse `wsl -l -q` like wsl.go findDistro().
        std::env::var("WSL_DISTRO").unwrap_or_else(|_| "Ubuntu".to_string())
    }

    /// Run a command inside WSL. Returns (stdout, stderr, exit_code).
    pub fn run_command(&self, command: &str, timeout: Duration) -> (String, String, i32) {
        let _ = timeout; // TODO(port): enforce timeout (wsl.go RunCommand)
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

    pub fn clean_stale_lock_files(&self) -> anyhow::Result<()> {
        // TODO(port): wsl.go CleanStaleLockFiles()
        Ok(())
    }

    pub fn force_kill_openclaw(&self) -> anyhow::Result<()> {
        let (_, _, _) = self.run_command("pkill -f openclaw || true", Duration::from_secs(10));
        Ok(())
    }

    pub fn start_gateway(&self) -> anyhow::Result<()> {
        // TODO(port): wsl.go StartGateway() — spawn the gateway process
        // and return a handle. Best-effort fire-and-forget for now.
        let _ = Command::new("wsl")
            .args([
                "-d",
                &self.distro,
                "--",
                "bash",
                "-lc",
                "nohup openclaw gateway >/dev/null 2>&1 &",
            ])
            .spawn();
        Ok(())
    }
}
