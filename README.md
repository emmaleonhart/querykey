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
- **Markdown on your disk is the source of truth.** Tasks, events, people,
  and notes live as markdown files (YAML frontmatter for structured fields +
  freeform body — Obsidian-style, useful even without QueryKey installed),
  tracked in a **git repository**. The AI operates *on* those files; it does
  not lock your life inside an app database.
- **The knowledge graph is derived, not canonical.** People, commitments, and
  their links are projected *out of* the markdown into an embedded graph
  (Loca / SutraDB — see Architecture) for fast structured queries. The graph
  is a secondary index you can always rebuild from the files.
- **Agent-agnostic via MCP.** QueryKey exposes itself as an MCP server, so
  any agent (local Gemma by default — cheap and private — or Claude/GPT if
  you choose) can attend over your graph and act on your files.
- **Rationalist by disposition.** Confidence scores, "I'm not sure, want me
  to ask?", and an auditable record are the central UX, not a footnote.
- **Optionally peer-to-peer — and private by default.** You can use it 100%
  solo. But each person can also broadcast a **card** (a markdown file: what
  they're *offering* and *looking for* — their *key* and *query*). Cards sync
  directly peer-to-peer (no central server; GitHub bootstraps identity).
  Your own card is git-tracked (so you can revert); other people's cards are
  git-*ignored* on your machine (use them in the moment, don't archive their
  history). Card changes propagate on a **24-hour delay** — a drunk mistake
  at 11pm is fixable by morning and no one ever saw it. The network inverts
  the usual model: **absence of history is the default; persistence takes
  deliberate effort by an observer.**

It is **PRM + lightweight CRM + JIRA-style task tracker in one**, for one
person — and then, by selectively surfacing nodes you've *already* built,
an opt-in positive-sum social network. **Sequencing:** the private PRM is
built first (useful solo, zero network effects, and it builds the graph the
cards are later a window into); the peer-to-peer card layer comes second;
the MCP server is present from day one. See **Status** for what is real
today versus planned.

## Architecture (what's actually in the tree)

| Component | Stack | Where |
|---|---|---|
| Desktop/mobile app | Flutter (Dart `sdk ^3.10.8`); `provider`, `web_socket_channel`, `http`, `uuid`, `intl` | [`app/`](app/) |
| Server | **Rust** (crate `querykey-server`: `axum`, `tokio`, `reqwest`) — compiles & runs; structural port with TODOs | [`server/`](server/) |
| Source of truth | Markdown files + git history (planned on-disk model) | user's disk / git repo |
| AI engine | **Model-agnostic** via an **MCP server** (default agent: local **Gemma** — cheap & private; Claude/GPT optional). *Today's implementation:* OpenClaw via a local WSL gateway (port `18789`) | `server/src/openclaw/` |
| Knowledge graph | **Loca** (formerly **SutraDB**) — the author's embedded Rust graph-vector-time DB; the graph is **derived from the markdown**, not canonical. Wired via `loka-core` behind `--features loca`; in-memory fallback otherwise. Fuseki is **not** used (removed with the Go server) | `server/src/graph/` + [`../SutraDB`] |
| Ingest surface | Local markdown + pasted text / screenshots / voice notes (Discord deprioritized — todo.md Phase Z) | `server/src/ingest.rs` |
| Identity / sync | **GitHub** (usernames as identity, repo as sync) — a thin, swappable abstraction | (planned) |
| Peer-to-peer | **Card** exchange — pure P2P, no central server, 24h propagation delay | (planned) |
| Real-time | WebSocket hub | `server/src/ws.rs` |

Local endpoints when running: server `http://127.0.0.1:8000`, health
`/health`, WebSocket `ws://127.0.0.1:8000/ws/chat`, OpenClaw gateway
`http://127.0.0.1:18789`.

## Status — what works today

This is early. Roughly: planning and data models are complete; the AI bridge
is functional; most product behavior is scaffolding.

**Working / functional**
- **Rust server (`server/`) is the only server** — Go fully ported then
  deleted (recoverable from git history). Compiles in all three configs
  (`cargo build`, `--features loca`, `--features discord`), zero warnings;
  boots, detects the OpenClaw gateway, opens a **Loca** `.sdb` store,
  serves the HTTP API + `/health` + WebSocket + SPARQL passthrough + an
  **MCP endpoint** (`/mcp`).
- OpenClaw bridge: gateway detect, **incremental SSE streaming**, analyze,
  supervised retry + health-check gateway lifecycle, graceful stop.
- Data models: full entity set ported to Rust, JSON contract preserved.
- **Loca/SutraDB** derived graph (`--features loca`): person/task/message/
  conflict persisted with full fields; SPARQL **query** bridge works;
  typed read-back of persons & tasks (smoke-verified); `insert_triples`
  via N-Triples.
- Ingest pipeline: relaxed-schema parse → typed models → store + typed
  GraphDiff broadcast over the WebSocket hub.
- **MCP server** (`/mcp`): JSON-RPC `initialize`/`tools/list`/`tools/call`.

**Honest limitations / not yet built**
- **Canonical markdown write path** (YAML frontmatter + body; git-tracked)
  — the derived graph is a rebuildable index; task/conflict *mutations*
  and full hydration go through markdown, which is **not built yet** (the
  load-bearing next piece — `docs/markdown-schema.md`).
- **Peer-to-peer card layer** (offer/looking-for cards, asymmetric
  git-tracking, 24h delay) — *after* the solo PRM (`docs/card-format.md`).
- **GitHub identity/sync** bootstrap behind a swappable handle abstraction.
- MCP stdio/SSE transports + `agents.md`-governed write tools.
- The follow-up engine, conflict resolution, daily check-ins;
  calendar/scheduling; audio/voice pipeline; external tool sync.
- **Discord** is deprioritized (feature-gated serenity skeleton only) —
  see `todo.md` Phase Z.

See [`queue.md`](queue.md) for the authoritative near-term plan and
[`todo.md`](todo.md) for the full phased roadmap.

## Running it

Windows + WSL is the current target. Prerequisites:

- **Rust** (stable, via [rustup](https://rustup.rs)) — `cargo` on `PATH`
- **Flutter** (Dart SDK 3.10.8+) on `PATH`
- **WSL Ubuntu** with **OpenClaw** installed (for AI features; the server runs
  without it, but AI chat/extraction needs the gateway)
- *Optional:* the sibling **`../SutraDB`** checkout for the Loca graph store
  (`--features loca`); without it the server uses an in-memory graph

Then, from the repo root:

```bat
!run.bat
```

That script builds the Rust server (`cargo build --features loca`, falling
back to the in-memory build), runs `flutter pub get`, starts the OpenClaw
gateway in WSL, launches the server, and runs the Flutter app on Windows
(`flutter run -d windows`). Closing the app window tears everything back down.

`!runClaude.bat` just opens Claude Code at the repo root.

## Repository layout

| Path | What it is |
|---|---|
| [`app/`](app/) | Flutter app (Dart) — desktop-first; Chat / Tasks / Ingest screens |
| [`server/`](server/) | **Rust** server (`querykey-server`) — the only server: ingest, agent bridge, WebSocket, MCP, Loca graph store |
| [`docs/`](docs/) | `architecture.md`, `data-model.md`, `markdown-schema.md`, `card-format.md`, `versions-comparison.md`, `why-go.md` |
| [`chat/`](chat/) | Vision corpus (chat-log exports); gitignored except its README — private context, not a spec |
| [`dev_scheduling/`](dev_scheduling/) | Dev-time agent data (`receipts/discord/`), committed so CI can write to it |
| [`queue.md`](queue.md) | Authoritative near-term plan / recovery dump |
| [`todo.md`](todo.md) | Full phased roadmap |
| `CLAUDE.md` | Workflow rules and architecture decisions for working in this repo |
| `!run.bat`, `!runClaude.bat` | Windows run scripts |
