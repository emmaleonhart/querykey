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
- **Source of truth**: **markdown files on the user's disk, tracked in a git repo** — **IMPLEMENTED** in `server/src/vault/` (Round 5). YAML frontmatter (structured fields) + freeform body (the description); Obsidian-usable without QueryKey. `VAULT_DIR` selects the root (default `./vault`); layout `people/ tasks/ events/ notes/`. API + ingest write the vault first, then project a **derived** index into Loca; the graph is rebuilt from the vault on startup; `update_task` mutates the markdown. Round-trips are lossless (unit-tested). Spec + as-built notes: `docs/markdown-schema.md`. Conflict/OpenQuestion/FollowUp on-disk forms are still TBD (graph-only for now).
- **Knowledge graph**: **derived from the markdown, not canonical.** RDF/graph is generated *out of* the files; it is a rebuildable secondary index. Store: **Loca** (formerly **SutraDB**) — the author's own embedded **Rust** graph-vector-time DB (separate project at `../SutraDB`; the time dimension matters — you care about a relationship's history). Wired in the Rust server via `loka-core` behind `--features loca` (`src/graph/loca.rs`); an in-memory backend is the default so the crate builds without the SutraDB checkout. **Fuseki is NOT used at all** (the old Fuseki stub was removed with the Go server).
- **AI engine**: model-agnostic. QueryKey exposes an **MCP server** (present from day one) so *any* agent can attend over the graph and act on the files. Default agent: local **Gemma** (cheap, private); Claude/GPT optional for power users. *Implementation note:* today's bridge is OpenClaw via a WSL gateway (`127.0.0.1:18789`) under `server/src/openclaw/` — an implementation detail behind a model-agnostic interface. Callers must never name a specific model/engine.
- **`agents.md`** (name/shape flexible — may become an `agents/` dir): the local agent's behavior, heuristics, and prompts live as an **editable, version-controlled markdown file in the user's repo**, not baked into code. Transparent and auditable (not a black box); two users' `agents.md` produce radically different behavior from the same system. The MCP server executes within whatever envelope `agents.md` defines. It is also what drives **agent-drafted cards** (the agent writes a first draft of your key/query from the PRM it built; you approve).
- **Server**: **Rust** (`server/`, crate `querykey-server`) — the *only* server; local-first: ingestion, the derived graph (Loca), MCP, real-time sync. **There is no more Go.** The previous Go implementation was fully ported then deleted (Round 4); it is recoverable from git history if ever needed (the last commit containing `server-go-old/` — same disposition as `secretarybird-old/`). Build: `cargo build` (in-memory), `--features loca` (Loca graph), `--features discord` (serenity bot). Remaining in-code TODOs are non-blocking (markdown-canonical write path, deprioritized Discord deep logic per todo.md Phase Z, MCP stdio/SSE transports).
- **Identity / sync**: **GitHub** bootstraps identity (usernames) and sync (the git repo). Design it as a thin abstraction — "a user is a canonical handle that currently resolves via GitHub" — so it can be swapped (DIDs/Nostr) later without baking GitHub in everywhere.
- **Peer-to-peer card layer** (built *after* the solo PRM): each user broadcasts a **card** — a markdown file of what they're *offering* and *looking for* (their *key* and *query*). Pure P2P, no central server. **Your own card is git-tracked** (enables revert/undo); **other people's cards are git-ignored on your machine** (usable in the moment, not archived — intentional asymmetry, no surveillance). Card changes propagate on a **24-hour delay** (privacy safety valve); a revert before propagation is immediate. Soft, non-cryptographic guarantee — sufficient for a community that doesn't assume bad actors. Private-vs-public cards: planned, not now.
- **Ingest surfaces**: local markdown + pasted text / screenshots / voice notes are primary. **Discord is deprioritized** to `todo.md` Phase Z (feature-gated serenity skeleton only; user is unsure how much they'll use it). WhatsApp/Instagram/Slack ride behind Discord.
- **Data model**: See `docs/data-model.md` — Person, Handle, Task, Event, Message, Conflict, FollowUp, etc.
- **Architecture**: See `docs/architecture.md`
- **Stack-history note**: `docs/why-go.md` / `docs/versions-comparison.md` are **historical** — they argue against the old Electron+Python stack (lessons still hold) and describe a then-current Go server. Go has since been fully replaced by Rust and deleted; treat any "Go server" wording in those docs as history, not current state.
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
- **Markdown is canonical; the graph is derived.** Never treat the graph as the store of record — it must be rebuildable from the files.
- **Adoption sequencing**: private PRM first (useful solo, builds the graph), peer-to-peer card layer second (a selective window into the already-built graph), MCP server from day one. The P2P layer is what makes using a relationship tracker socially legitimate — it's not just a feature, it changes the meaning of the tool.
- **Positive-sum & opt-in.** The social layer is selectively surfaced nodes you already built privately — not a separate publishing chore, not a surveillance archive. Absence of history is the default; persistence requires a deliberate observer.
- **Card asymmetry is load-bearing.** Own card tracked (for undo); others' cards git-ignored (no archive). Get the card format right early as a spec — it ossifies fast once cards are exchanged.
- **Auditable, not hidden.** Everything the AI records is visible and inspectable. No surveillance, no paper trail you can't see.

# currentDate
Today's date is 2026-05-15.
