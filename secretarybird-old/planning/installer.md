# Secretary Bird Assistant - Installer Architecture

## Goal
A single `.exe` that takes a non-technical Windows user from zero to a working
Secretary Bird Assistant with full OpenClaw + browser relay capabilities. No command line
knowledge required.

## What Must Be Installed

The full stack requires these components, in dependency order:

```
1. WSL (Windows Subsystem for Linux)     ← requires admin + reboot
2. Ubuntu distro inside WSL              ← auto-installed with WSL
3. Node.js inside WSL                    ← needed for OpenClaw
4. OpenClaw CLI inside WSL               ← npm install -g openclaw
5. Chromium inside WSL                   ← for browser relay
6. OpenClaw config                       ← gateway, browser, auth
7. Python 3.13 on Windows               ← for the FastAPI backend
8. Secretary Bird Assistant Electron app         ← the actual product
```

## Installation Phases

### Phase 1: NSIS Installer (runs as admin)
The electron-builder NSIS installer handles:
- Install Electron app + bundled backend to Program Files or user directory
- Create desktop/Start Menu shortcuts
- Register uninstaller

### Phase 2: First-Run Setup (runs on first app launch)
The Electron app detects missing dependencies and runs setup:

```
First Launch
  │
  ├─ Check: Is WSL installed?
  │   NO → Prompt for admin elevation
  │         Run: wsl --install (installs WSL + Ubuntu)
  │         Show: "Reboot required" dialog
  │         Set: registry flag "setup_phase = wsl_installed"
  │         Trigger: reboot (or ask user to reboot)
  │
  ├─ After reboot, app launches again
  │   Check: registry flag "setup_phase"
  │
  ├─ Check: Is Ubuntu distro ready?
  │   NO → Wait for WSL to finish Ubuntu setup
  │         (first boot after install takes 1-2 minutes)
  │
  ├─ Check: Is Node.js installed in WSL?
  │   NO → Run: wsl -d Ubuntu -e bash -c 'curl -fsSL https://...|bash && nvm install --lts'
  │
  ├─ Check: Is OpenClaw installed in WSL?
  │   NO → Run: wsl -d Ubuntu -e bash -c 'npm install -g openclaw'
  │
  ├─ Check: Is Chromium installed in WSL?
  │   NO → Run: wsl -d Ubuntu -e bash -c 'sudo apt install -y chromium-browser'
  │
  ├─ Check: Is OpenClaw configured?
  │   NO → Write ~/.openclaw/openclaw.json with:
  │         - gateway config (port 18789, loopback, auth token)
  │         - browser config (managed Chromium profile)
  │         - chatCompletions endpoint enabled
  │
  ├─ Check: Is OpenClaw onboarded?
  │   NO → Run: wsl -d Ubuntu -e bash -c 'openclaw onboard'
  │         (user chooses their LLM provider — this is interactive)
  │
  └─ Setup complete → normal app launch
```

### Phase 3: Normal Launch (every subsequent start)
```
App Start
  ├─ Start OpenClaw gateway (wsl -d Ubuntu -e bash -lc 'openclaw gateway')
  │   - Retry up to 5 times on failure
  │   - Health poll every 10 seconds
  │   - Skip if already running on port 18789
  │
  ├─ Start Python backend (python -m backend.server)
  │   - Reuse if already running on port 8000
  │   - Health poll every 2 seconds until ready
  │
  ├─ Create Electron window
  │   - WebSocket connects to backend
  │   - Backend streams OpenClaw responses
  │
  └─ System tray (minimize to tray, restart options)
```

## Challenges & Solutions

### 1. WSL Install Requires Admin + Reboot
**Problem**: `wsl --install` needs administrator privileges and a system reboot.
**Solution**: Two-phase install. Phase 1 handles admin operations. The app
detects incomplete setup on next launch and resumes from where it left off.
Use a registry key or local file to track setup state.

### 2. WSL First Boot Is Slow
**Problem**: After install + reboot, the first `wsl` command triggers Ubuntu
setup (unpacking, creating user) which takes 1-2 minutes.
**Solution**: Show a progress UI: "Setting up Linux environment..." with a
spinner. Poll `wsl --list --quiet` until Ubuntu appears.

### 3. OpenClaw Onboarding Is Interactive
**Problem**: `openclaw onboard` is a TUI wizard where users pick their LLM
provider (OpenAI, Anthropic, local models, etc.).
**Solution**: Options:
- (A) Embed a simplified LLM provider picker in the Electron UI that writes
  the config directly — avoids the TUI entirely.
- (B) Open a terminal window for the user to complete onboarding.
- (C) Pre-configure with a default provider and let users change later.

### 4. Chromium in WSL Needs a Display
**Problem**: WSL2 supports WSLg (GUI apps) but not all systems have it working.
**Solution**: The managed browser uses headless Chromium by default for web
research. No display needed. If the user wants visible browser automation,
they can use the Chrome extension relay on Windows Chrome instead.

### 5. Multiple Python Installations
**Problem**: Windows may have multiple Python installations (different users,
different versions). The wrong one won't have required packages.
**Solution**: The app bundles a PyInstaller-compiled backend executable for
production. In dev mode, `findPython()` prefers the known-good installation
path. The NSIS installer can also bundle a Python embeddable distribution.

### 6. Disk Space
**Problem**: Full stack needs significant disk space:
- WSL + Ubuntu: ~1.5 GB
- Node.js: ~200 MB
- OpenClaw: ~100 MB
- Chromium: ~300 MB
- Python + backend: ~200 MB
- Electron app: ~150 MB
- **Total: ~2.5 GB**

**Solution**: Warn user about disk space requirement before installation.
Show progress for each component being installed.

## Setup State Machine

```
STATES:
  fresh           → No WSL, nothing installed
  wsl_installing  → WSL install triggered, waiting for reboot
  wsl_ready       → WSL + Ubuntu available
  deps_installing → Installing Node.js, OpenClaw, Chromium
  deps_ready      → All dependencies installed
  configuring     → Writing OpenClaw config
  onboarding      → User picking LLM provider
  ready           → Full stack operational

TRANSITIONS:
  fresh → wsl_installing           (admin elevation + wsl --install)
  wsl_installing → wsl_ready       (reboot + Ubuntu unpacked)
  wsl_ready → deps_installing      (apt + npm installs)
  deps_installing → deps_ready     (all packages installed)
  deps_ready → configuring         (write openclaw.json)
  configuring → onboarding         (openclaw onboard)
  onboarding → ready               (LLM provider configured)

PERSISTENCE:
  State stored in: %APPDATA%/tojo-assistant/setup-state.json
  {
    "state": "deps_ready",
    "wsl_distro": "Ubuntu",
    "openclaw_version": "2026.2.6-3",
    "completed_steps": ["wsl", "ubuntu", "nodejs", "openclaw", "chromium"],
    "last_error": null
  }
```

## Current Installer (What Exists Today)

The current NSIS installer (via electron-builder) handles:
- Installing the Electron app
- Bundling the PyInstaller backend executable
- Creating shortcuts

It does NOT handle:
- WSL installation
- Ubuntu setup
- OpenClaw installation
- Chromium installation
- OpenClaw configuration
- First-run setup wizard

These are all handled manually by the developer today, and need to be
automated for non-technical users.

## Implementation Priority

1. **First-run dependency checker** — detect what's missing, show clear status
2. **WSL installer with reboot handling** — the hardest piece
3. **Automated OpenClaw + Chromium install** — straightforward apt/npm
4. **Config writer** — generate openclaw.json programmatically
5. **LLM provider setup UI** — either embed in Electron or open terminal
6. **Progress UI** — show installation progress with time estimates
