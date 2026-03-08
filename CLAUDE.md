# Secretarybird Pivot

## Workflow Rules
- **Commit early and often.** Every meaningful change gets a commit with a clear message explaining *why*, not just what.
- **Do not enter planning-only modes.** All thinking must produce files and commits. If scope is unclear, create a `planning/` directory and write `.md` files there instead of using an internal planning mode.
- **Keep this file up to date.** As the project takes shape, record architectural decisions, conventions, and anything needed to work effectively in this repo.
- **Update README.md regularly.** It should always reflect the current state of the project for human readers.

## Project Description
AI secretary that ingests unstructured input (Discord chats, voice notes, screenshots, pasted text), extracts tasks/events/contradictions via OpenClaw, and actively follows up with team members to clarify and resolve. Discord bot is the primary interaction surface. Flutter app provides richer features (task boards, calendars, audio recording).

## Architecture and Conventions
- **Framework**: Flutter — single codebase for Windows (current focus), macOS, Linux, Web, iOS, Android
- **AI Engine**: OpenClaw — entity extraction, task/event detection, contradiction detection, follow-up generation
- **Server**: Secretarybird Server (local or cloud) — ingestion, knowledge graph, real-time WebSocket sync
- **Primary integration**: Discord bot (DM-first, hourly batch processing)
- **Data model**: See `docs/data-model.md` — Person, Task, Event, Message, Conflict, FollowUp, etc.
- **Architecture**: See `docs/architecture.md`
- **Roadmap**: See `todo.md`

### Key Design Decisions
- **Task vs Event**: Tasks are time-flexible (optional deadline). Events are time-fixed (start + end time). If you can move it without asking permission, it's a task.
- **OpenClaw tone**: Secretary, not consultant. Short, direct messages. Never wordy.
- **Epistemic humility**: Confidence scores on extracted data. Ask when unsure rather than guess silently.
- **Cross-platform identity**: Same person tracked across Discord, Slack, phone, voice — single Person entity with multiple handles.

# currentDate
Today's date is 2026-03-08.
