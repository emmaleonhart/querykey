# Why Go: How the Server Rewrite Solves the Socket Problem

> **Framing note (updated 2026-05).** Read this as **"why not the old
> Electron + Python + WSL-socket stack"** — that argument still holds.
> But **Go is no longer the target: the server language is now Rust**,
> and the current Go `server/` is deprecated (kept compilable until the
> Rust rewrite supersedes it). The earlier prototype this doc compares
> against has been deleted from the tree; its salvage analysis now
> lives in [`versions-comparison.md`](versions-comparison.md). Mentions
> of the old project name below are historical, not QueryKey.

## The Old Architecture Was Broken

The original secretarybird (tojo-assistant) had a **three-process, two-socket chain** that constantly broke:

```
Electron (Node/TS, port 3000)
    │
    ├── spawns ──→ Python FastAPI (port 8000)
    │                  │
    │                  └── HTTP client ──→ OpenClaw Gateway in WSL (port 18789)
    │
    └── spawns ──→ wsl -e openclaw gateway (port 18789)
```

Every arrow in that diagram was a point of failure. The Electron main process had to:

1. Find a Python interpreter on the user's machine
2. Spawn Python as a child process running FastAPI on port 8000
3. Spawn `wsl -d Ubuntu -e bash -lc "openclaw gateway"` as another child process
4. Poll `http://127.0.0.1:8000/health` with a `setInterval` timer to know when Python was ready
5. Poll `http://127.0.0.1:18789/` with a separate timer to know when OpenClaw was ready
6. Maintain a shared `http.Agent({ keepAlive: true, maxSockets: 4 })` to prevent TIME_WAIT socket exhaustion on Windows (every health check created a TCP socket that lingered for 2-4 minutes)
7. Manage retry logic with `setTimeout` chains when OpenClaw got stuck
8. Kill stale lock files in WSL when the gateway died without cleanup
9. Bridge IPC channels between Electron renderer and the Python backend

### The #1 Runtime Issue: OpenClaw Getting Stuck

This was documented in `secretarybird-old/planning/openclaw-gateway-issues.md`. The cycle:

1. OpenClaw gateway crashes or gets killed by Windows
2. Lock file at `/tmp/openclaw-<UID>/gateway.<hash>.lock` becomes stale
3. `openclaw gateway stop` fails because WSL doesn't have systemd
4. `openclaw gateway` fails because the lock file says it's "already running"
5. User is completely stuck — no AI, no chat, app is useless
6. Only fix: `pkill -f openclaw; rm -f /tmp/openclaw-*/gateway.*.lock` (requires terminal knowledge)

The Electron app had a STOP button, but it only killed the process — it didn't clean the lock file. So even the in-app recovery was broken.

### Why It Was So Fragile

The fundamental issue was **too many layers of indirection**:

- Electron couldn't talk to OpenClaw directly — it had to go through Python
- Python couldn't manage the OpenClaw process — Electron spawned it
- Health checks happened in Electron, but recovery logic was split between Electron and Python
- The WebSocket connection went: **Browser → Electron IPC → Python FastAPI → OpenClaw HTTP**, crossing four process boundaries

When any link in this chain broke, the error messages were confusing and the recovery was manual.

## The Go Server Collapses This to Two Layers

```
Flutter app (any platform)
    │
    └── WebSocket/HTTP ──→ Go server (port 8000)
                               │
                               ├── HTTP client ──→ OpenClaw Gateway in WSL (port 18789)
                               ├── WebSocket hub (real-time sync)
                               ├── Discord bot
                               └── Fuseki SPARQL client
```

**The Python middle layer is gone.** The Go server IS the backend — one binary, one process, everything built in.

### What Changed Specifically

| Problem | Old (Electron + Python) | New (Go) |
|---------|------------------------|----------|
| **Process count** | 3 processes (Electron, Python, OpenClaw) | 2 processes (Go server, OpenClaw) |
| **Python dependency** | Need Python 3.13 installed, correct PATH, correct virtualenv | No Python needed for server |
| **Startup sequence** | Electron spawns Python, waits for health check, then spawns OpenClaw, waits again | Go binary starts instantly, checks OpenClaw on first request |
| **Health checking** | Two separate `setInterval` timers in Electron, one for Python, one for OpenClaw | One goroutine checks OpenClaw; Go server itself needs no health polling since it IS the server |
| **Socket exhaustion** | Required `http.Agent({ keepAlive: true })` to prevent TIME_WAIT buildup from health check polling | Go's `http.Client` reuses connections by default |
| **WebSocket path** | Browser → Electron IPC → Python FastAPI WebSocket → OpenClaw HTTP | Flutter → Go WebSocket → OpenClaw HTTP (2 hops instead of 4) |
| **Lock file recovery** | STOP button killed process but forgot the lock file | `wsl.go` always cleans lock files before starting: `CleanStaleLockFiles()` runs on every gateway start |
| **Retry logic** | `setTimeout` chains in Electron with `openclawRetries` counter | Goroutine with `time.AfterFunc`, cleaner and doesn't block the event loop |
| **Deployment** | Need Node.js + Python + electron-builder → NSIS installer | `go build` → single `.exe`, no runtime needed |
| **Cross-platform** | Electron worked everywhere but the Python+WSL chain was Windows-specific pain | Go binary cross-compiles to any OS from any OS |

### Lock File Recovery Is Automatic Now

The old code had this in `main.ts` (but it was incomplete — didn't clean locks):

```typescript
function forceKillOpenClaw(): { ok: boolean; message: string } {
  stopOpenClawGateway();
  execSync(`wsl -d ${distro} -- bash -c "pkill -f openclaw-gateway; pkill -f openclaw; true"`);
  // BUG: No lock file cleanup!
}
```

The Go server does it right in `openclaw/wsl.go`:

```go
func (m *WSLManager) CleanStaleLockFiles() error {
    cmd := exec.Command("wsl", "-d", m.distro, "--", "bash", "-c",
        "rm -f /tmp/openclaw-*/gateway.*.lock")
    // ...
}

func (m *WSLManager) StartGateway() (*exec.Cmd, error) {
    // Always clean stale locks before starting
    m.CleanStaleLockFiles()
    cmd := exec.Command("wsl", "-d", m.distro, "-e", "bash", "-lc", "openclaw gateway")
    // ...
}
```

And `ForceKillOpenClaw()` does both kill AND cleanup:

```go
func (m *WSLManager) ForceKillOpenClaw() error {
    cmd := exec.Command("wsl", "-d", m.distro, "--", "bash", "-c",
        "pkill -f openclaw-gateway; pkill -f openclaw; rm -f /tmp/openclaw-*/gateway.*.lock; true")
    // ...
}
```

### Goroutines Replace the Timer Mess

The old Electron code had this pattern everywhere:

```typescript
// Electron: nested timers, promise chains, manual state tracking
let openclawRetries = 0;
let openclawHealthTimer: ReturnType<typeof setInterval> | null = null;

function startOpenClawHealthCheck(): void {
  if (openclawHealthTimer) return;
  openclawHealthTimer = setInterval(async () => {
    const running = await isOpenClawGatewayRunning();
    if (running) { openclawRetries = 0; }
  }, 10000);
}

function retryOpenClawGateway(): void {
  openclawRetries++;
  if (openclawRetries >= OPENCLAW_MAX_RETRIES) { return; }
  setTimeout(() => { startOpenClawGateway(); }, OPENCLAW_RETRY_DELAY);
}
```

The Go equivalent is straightforward:

```go
func (b *Bridge) startHealthCheck() {
    b.healthTicker = time.NewTicker(10 * time.Second)
    go func() {
        for {
            select {
            case <-b.healthTicker.C:
                if b.Detect().Available {
                    b.retries = 0
                }
            case <-b.stopHealth:
                return
            }
        }
    }()
}
```

No callback nesting, no timer ID tracking, no manual cleanup. The `select` statement handles everything, and the goroutine exits cleanly when `stopHealth` is closed.

## Why Go Specifically (Not Python, Rust, or Node)

The server's job is **network I/O coordination**: shuttle messages between WebSocket clients, Discord, OpenClaw, and Fuseki. That's what Go was designed for.

| Criterion | Go | Python | Node/TS | Rust |
|-----------|-----|--------|---------|------|
| Concurrency model | Goroutines (lightweight, native) | asyncio (complex, single-threaded) | Event loop (single-threaded) | Tokio (powerful but verbose) |
| Deployment | Single binary, no runtime | Needs Python + virtualenv | Needs Node.js + node_modules | Single binary |
| Memory at idle | ~10 MB | ~50-100 MB | ~40-80 MB | ~5 MB |
| Cross-compile | Built-in (`GOOS=linux go build`) | Not applicable | Not applicable | Built-in |
| WebSocket handling | gorilla/websocket, one goroutine per client | FastAPI WebSocket works but async complexity | ws/socket.io, callback-based | tungstenite, works well |
| Discord bot library | discordgo (mature) | discord.py (mature) | discord.js (mature) | serenity (mature) |
| Startup time | Instant | 1-3 seconds (import overhead) | 0.5-1 second | Instant |

**Python** was the old choice and it worked, but it required spawning a separate process and added a layer. The AI work is in OpenClaw, not in the server, so Python's ML ecosystem advantage doesn't apply here.

**Node/TypeScript** would have been familiar from the Electron code, but it brings the same event loop limitations and `node_modules` weight. The old code's callback complexity was partly Node's fault.

**Rust** would work but is overkill — the server doesn't need maximum performance, it needs simple concurrency and easy deployment.

## The OpenClaw Gateway Problem Isn't Fully Solved

To be clear: the underlying WSL + OpenClaw lock file issue still exists. OpenClaw's gateway still uses lock files, WSL still doesn't have systemd by default, and the gateway can still get stuck.

What Go solves is **how we deal with it**:

- Lock files are always cleaned before starting (`CleanStaleLockFiles()`)
- Force-kill does both process kill AND lock cleanup in one atomic operation
- Health checking and retry are clean goroutines, not nested timer callbacks
- The recovery path is `/api/openclaw/restart` — one HTTP call from Flutter, handled entirely in Go
- No more IPC bridging through Electron to reach the WSL process

The remaining fix is upstream: OpenClaw should fall back to `kill <pid>` when systemd is unavailable, and clean its own lock files on crash. But until that happens, the Go server handles recovery correctly.
