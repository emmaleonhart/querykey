# QueryKey

**A rationalist social network you run locally from your own desktop** — and,
underneath, a personal relationship manager (PRM), a lightweight CRM, and a
JIRA-style task tracker for your own life. QueryKey uses a local AI agent to
help you keep up with the people and commitments in your life, while
respecting your privacy and the privacy of everyone you talk to.

> **The name.** "QueryKey" comes from the **Q / K / V** (query, key, value)
> projections in a transformer's attention matrix. Your day, relationships,
> and tasks are the *values*; the local agent attends over them by computing
> *queries* from your current intent against *keys* built from your notes,
> chat logs, and prior conversations.

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
- **Your data is plain markdown on your disk.** Tasks, events, and notes are
  intended to live as markdown files you can open and edit in any editor —
  the AI operates *on* those files; it does not lock your life inside an
  app database. (On-disk format is still being designed — see `queue.md`.)
- **Rationalist by disposition.** Confidence scores, "I'm not sure, want me
  to ask?", and an auditable record are the central UX, not a footnote.

It is **PRM + lightweight CRM + JIRA-style task tracker in one**, for one
person: the people-first social-network framing is the *direction*; the
engine is still being reoriented toward it. See **Status** below for what is
real today versus planned.

## Architecture (what's actually in the tree)

| Component | Stack | Where |
|---|---|---|
| Desktop/mobile app | Flutter (Dart `sdk ^3.10.8`); `provider`, `web_socket_channel`, `http`, `uuid`, `intl` | [`app/`](app/) |
| Server | Go 1.23 (`discordgo`, `gorilla/websocket`, `google/uuid`) — **deprecated; the target language is Rust** (no rewrite yet) | [`server/`](server/) |
| AI engine | A **model-agnostic local agent** (default model: **Gemma**; switchable). *Today's implementation:* OpenClaw via a local WSL gateway (port `18789`) — an implementation detail to be superseded by the Rust rewrite | `server/internal/openclaw/` |
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
- **Server rewrite in Rust** (the Go server is deprecated; not started).
- **Local-first markdown task model** (on-disk format not yet designed).
- Making the local agent genuinely model-agnostic with **Gemma** as the
  default (OpenClaw is today's bridge, to be superseded).

See [`queue.md`](queue.md) for the authoritative near-term plan and
[`todo.md`](todo.md) for the full phased roadmap.

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
| [`docs/`](docs/) | `architecture.md`, `data-model.md`, `versions-comparison.md`, `why-go.md` — design, entity model, history |
| [`chat/`](chat/) | Vision corpus (chat-log exports); gitignored except its README — private context, not a spec |
| [`dev_scheduling/`](dev_scheduling/) | Dev-time agent data (`receipts/discord/`), committed so CI can write to it |
| [`queue.md`](queue.md) | Authoritative near-term plan / recovery dump |
| [`todo.md`](todo.md) | Full phased roadmap |
| `CLAUDE.md` | Workflow rules and architecture decisions for working in this repo |
| `!run.bat`, `!runClaude.bat` | Windows run scripts |
