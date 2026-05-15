# QueryKey

## Workflow Rules
- **Commit early and often.** Every meaningful change gets a commit with a clear message explaining *why*, not just what.
- **Do not enter planning-only modes.** All thinking must produce files and commits. If scope is unclear, create a `planning/` directory and write `.md` files there instead of using an internal planning mode.
- **Keep this file up to date.** As the project takes shape, record architectural decisions, conventions, and anything needed to work effectively in this repo.
- **Update README.md regularly.** It should always reflect the current state of the project for human readers.

## Project Description
QueryKey is a **local-first social network and personal relationship manager (PRM)** you run from your own desktop. It ingests the unstructured streams of how you actually communicate (Discord chats, voice notes, screenshots, pasted text) and uses local AI (OpenClaw) to build a private model of the people and commitments in your life. It then helps you, proactively and quietly, keep those relationships in good standing.

The engine in this repo grew out of an earlier team-coordination tool ("Secretarybird"). Most of the entity model, ingestion pipeline, and AI bridge carry over; the framing is being reoriented from *team coordination* toward *personal relationships*. Expect the codebase to still carry some team-coordination scaffolding while that reorientation is in progress — see `todo.md` and the **Status** section of `README.md`.

## Architecture and Conventions
- **Framework**: Flutter — single codebase for Windows (current focus), macOS, Linux, Web, iOS, Android
- **AI Engine**: OpenClaw (local, via WSL gateway on `127.0.0.1:18789`) — entity extraction, task/event detection, contradiction detection, follow-up generation
- **Graph Store**: Apache Jena Fuseki — triple store for the relationship knowledge graph (client is currently a stub)
- **Server**: QueryKey Server (local) — ingestion, knowledge graph, real-time WebSocket sync. Local-first by design.
- **Primary ingest surface**: Discord bot (DM-first, hourly batch processing)
- **Future ingest surfaces**: WhatsApp, Instagram, Slack, voice notes, screenshots
- **Data model**: See `docs/data-model.md` — Person, Handle, Task, Event, Message, Conflict, FollowUp, etc.
- **Architecture**: See `docs/architecture.md`
- **Why Go**: See `docs/why-go.md`
- **Roadmap**: See `todo.md`

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
- **OpenClaw tone**: Secretary, not consultant. Short, direct messages. Never wordy.
- **Epistemic humility**: Confidence scores on extracted data. Ask when unsure rather than guess silently.
- **Cross-platform identity**: Same person tracked across Discord, Slack, WhatsApp, Instagram, phone, voice — single Person entity with multiple handles.
- **Open questions**: A queue of things the system needs answered, with urgency levels (ASAP, by [time], end of day, whenever). Resolved on any platform → disappears from queue.
- **Auditable, not hidden.** Everything the AI records is visible and inspectable. No surveillance, no paper trail you can't see.

# currentDate
Today's date is 2026-05-15.
