# QueryKey

**A social network you run locally from your own desktop.** QueryKey has the
elements of a personal relationship manager (PRM) and uses local AI agents to
help you keep up with the people in your life — while respecting your privacy
and the privacy of everyone you talk to.

🌐 **Website: <https://querykey.emmaleonhart.com>**

> **Status: early.** The QueryKey vision below is the target; the codebase is
> an in-progress engine being built and reoriented toward it. See **Status**
> for what is real today versus planned.

---

## What QueryKey is (the vision)

QueryKey runs on *your* machine. It watches the messy, unstructured streams of
how you actually communicate — chat logs, pasted conversations, screenshots,
voice notes — and uses local AI to build a private model of the people and
commitments in your life. It then helps you, proactively and quietly, keep
those relationships in good standing.

Principles:

- **Local-first for privacy.** The server runs on your own machine. Nothing has
  to leave your desktop. The privacy that matters is not just yours — it's the
  privacy of the people you talk about too.
- **The tool serves you.** You never reformat your life to fit a form. You
  communicate the way you already do; the system meets you there.
- **AI does the hard work, and admits when it's unsure.** Extraction carries
  confidence scores. When the system isn't sure, it asks instead of guessing
  silently. Everything it records is visible and auditable — nothing hidden.
- **A relationship knowledge graph, not a dashboard to maintain.** People,
  conversations, commitments, and the links between them are stored as a graph
  you own.

The PRM / social-network framing (people-first) is the *direction*; the engine
is still being reoriented toward it. See **Status** below for what is real
today versus planned.

## Architecture (what's actually in the tree)

| Component | Stack | Where |
|---|---|---|
| Desktop/mobile app | Flutter (Dart `sdk ^3.10.8`); `provider`, `web_socket_channel`, `http`, `uuid`, `intl` | [`app/`](app/) |
| Server | Go 1.23 (`discordgo`, `gorilla/websocket`, `google/uuid`) | [`server/`](server/) |
| AI engine | **OpenClaw** via a local gateway running in WSL Ubuntu (port `18789`) | `server/internal/openclaw/` |
| Knowledge graph | Apache Jena **Fuseki** triple store (planned; client is currently a stub) | `server/internal/graph/` |
| Ingest surface | Discord bot (DM-first, hourly batch) + pasted text / screenshots / voice notes | `server/internal/discord/`, `server/internal/ingest/` |
| Real-time | WebSocket hub | `server/internal/ws/` |

Local endpoints when running: server `http://127.0.0.1:8000`, health
`/health`, WebSocket `ws://127.0.0.1:8000/ws/chat`, OpenClaw gateway
`http://127.0.0.1:18789`.

## Status — what works today

This is early. Roughly: planning and data models are complete; the AI bridge
is functional; most product behavior is scaffolding.

**Working / functional**
- OpenClaw bridge: detects/auto-starts the WSL gateway, streams completions,
  retries, health-polls.
- Data models: the full entity set (Person, Handle, Task, Event, Message,
  Conflict, Instruction, OpenQuestion, FollowUp, VoiceProfile, …) defined in
  both Go and Dart, and aligned.
- Config loading, REST route/handler skeleton, WebSocket connection layer
  (auto-reconnect, streaming protocol), Flutter navigation shell with Chat /
  Tasks / Ingest screens, Discord bot connection + message buffering.

**Scaffolded / partial**
- Ingestion pipeline (accepts input, calls OpenClaw; result parsing is basic).
- Fuseki graph store (ping/dataset-ensure only; no real SPARQL yet).
- WebSocket hub (clients tracked; graph-diff broadcast minimal).
- Discord bot (connected and buffering; no follow-up or contradiction logic).

**Planned / not started**
- The follow-up engine (detect contradiction → open question → message the
  person), conflict resolution, daily check-ins.
- Calendar/scheduling, the audio/voice pipeline, external tool sync.
- The full QueryKey re-frame: the PRM / social-network model and the personal
  (single-user, relationship-centric) reorientation of the engine.

See [`todo.md`](todo.md) for the full roadmap.

## Running it

Windows + WSL is the current target. Prerequisites:

- **Go** (1.23+) — `winget install GoLang.Go`
- **Flutter** (Dart SDK 3.10.8+) on `PATH`
- **WSL Ubuntu** with **OpenClaw** installed (for AI features; the server runs
  without it, but AI chat/extraction needs the gateway)

Then, from the repo root:

```bat
!run.bat
```

That script builds the Go server, runs `flutter pub get`, starts the OpenClaw
gateway in WSL, launches the server, and runs the Flutter app on Windows
(`flutter run -d windows`). Closing the app window tears everything back down.

`!runClaude.bat` just opens Claude Code at the repo root.

## Repository layout

| Path | What it is |
|---|---|
| [`app/`](app/) | Flutter app (Dart) — desktop-first; Chat / Tasks / Ingest screens |
| [`server/`](server/) | Go server — ingest, OpenClaw bridge, WebSocket, (planned) graph store |
| [`docs/`](docs/) | `architecture.md`, `data-model.md`, `why-go.md` — design and entity model |
| [`dev_scheduling/`](dev_scheduling/) | Dev-time agent data (`receipts/discord/`), committed so CI can write to it |
| [`todo.md`](todo.md) | Roadmap |
| `CLAUDE.md` | Workflow rules and architecture decisions for working in this repo |
| `README_cleanvibe.md` | cleanvibe scaffolding placeholder |
| `!run.bat`, `!runClaude.bat` | Windows run scripts |
