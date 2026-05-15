# QueryKey

## Workflow Rules
- **Commit early and often.** Every meaningful change gets a commit with a clear message explaining *why*, not just what.
- **Do not enter planning-only modes.** All thinking must produce files and commits. If scope is unclear, create a `planning/` directory and write `.md` files there instead of using an internal planning mode.
- **Keep this file up to date.** As the project takes shape, record architectural decisions, conventions, and anything needed to work effectively in this repo.
- **Update README.md regularly.** It should always reflect the current state of the project for human readers.

## Project Description
QueryKey is a **rationalist social network** that doubles as a local-first personal relationship manager (PRM) / lightweight CRM / JIRA-style task tracker, run from your own desktop. It ingests the unstructured streams of how you actually communicate (Discord chats, voice notes, screenshots, pasted text) and uses a **local AI agent** to build a private model of the people and commitments in your life. It then helps you, proactively and quietly, keep those relationships and commitments in good standing.

**Why "QueryKey":** the name references the **Q / K / V** (query, key, value) projections of a transformer attention matrix. Your day, relationships, and tasks are a body of *values*; the local agent attends over them by computing *queries* from your current intent against *keys* built from your markdown notes, chat logs, and prior conversations.

The engine grew out of an earlier prototype (a different, broader product) that has been deleted from the tree — see [`docs/versions-comparison.md`](docs/versions-comparison.md) for what was salvaged and why. Some scaffolding from that lineage is still being reoriented toward the QueryKey vision; see `queue.md` (authoritative plan), `todo.md`, and the **Status** section of `README.md`.

## Architecture and Conventions

> Settled, not up for debate (see `queue.md`): **Flutter** for UI;
> **Rust** is the server target; the **local AI agent is
> model-agnostic with Gemma as the default**. Do not relitigate these.

- **UI Framework**: Flutter — single codebase for Windows (current focus), macOS, Linux, Web, iOS, Android. Locked in.
- **AI engine**: a **model-agnostic local agent**, default model **Gemma**, switchable to other local (or, optionally, hosted) models. It does entity extraction, task/event detection, contradiction detection, and follow-up generation. *Implementation note:* the current bridge is OpenClaw via a WSL gateway (`127.0.0.1:18789`), living under `server/internal/openclaw/`. Treat OpenClaw as an implementation detail to be superseded by the Rust rewrite — keep the agent interface model-agnostic so callers never name a specific model/engine.
- **Server**: local-first by design — ingestion, knowledge graph, real-time WebSocket sync. **Target language: Rust.** The current `server/` is Go and is **deprecated** (kept compilable until the Rust rewrite supersedes it; no rewrite this round).
- **Local task model**: tasks/events/notes are intended to live as **plain markdown files on the user's disk** that they can read and edit by hand. The on-disk schema is an open decision (see `queue.md`); document the model before implementing it.
- **Graph store**: Apache Jena Fuseki was the prior pick (client is a stub). Reconsidered for the local-first/single-user reorientation — an embedded store or SQLite + application relations may be preferable. Open decision in `queue.md`.
- **Primary ingest surface**: Discord (DM-first, hourly batch processing)
- **Future ingest surfaces**: WhatsApp, Instagram, Slack, voice notes, screenshots
- **Data model**: See `docs/data-model.md` — Person, Handle, Task, Event, Message, Conflict, FollowUp, etc.
- **Architecture**: See `docs/architecture.md`
- **Stack-history note**: `docs/why-go.md` argues against the old Electron+Python stack. Its anti-Electron/Python lessons still hold, but Go is no longer the target — Rust is.
- **Roadmap**: `queue.md` (authoritative, near-term) and `todo.md` (phased)

### Development Data (`dev_scheduling/`)
Provisional directory for agent data during development. Committed to the repo so GitHub Actions can write to it.
- `dev_scheduling/receipts/discord/` — JSON message logs extracted by the Discord bot via GitHub Actions

### Key Design Decisions
- **Local-first for privacy.** The server runs on your own machine. Nothing has to leave your desktop. The privacy that matters is not just yours — it's the privacy of the people you talk about too.
- **The tool serves you.** You never reformat your life to fit a form. You communicate the way you already do; the system meets you there.
- **Node IDs**: Human-readable aliases, not just opaque UUIDs.
- **Unified inbox**: The app has its own DM thread with the bot, and replies from any platform (Discord, WhatsApp, Instagram, etc.) show up in the same conversation view.
- **Task vs Event**: Tasks are time-flexible (optional deadline). Events are time-fixed (start + end time). If you can move it without asking permission, it's a task.
- **Confidence indicators**: Every extracted task/event/claim shows degree of certainty. The system doesn't pretend to know things perfectly.
- **Agent tone**: Secretary, not consultant. Short, direct messages. Never wordy.
- **Epistemic humility**: Confidence scores on extracted data. Ask when unsure rather than guess silently.
- **Cross-platform identity**: Same person tracked across Discord, Slack, WhatsApp, Instagram, phone, voice — single Person entity with multiple handles.
- **Open questions**: A queue of things the system needs answered, with urgency levels (ASAP, by [time], end of day, whenever). Resolved on any platform → disappears from queue.
- **Auditable, not hidden.** Everything the AI records is visible and inspectable. No surveillance, no paper trail you can't see.

# currentDate
Today's date is 2026-05-15.
