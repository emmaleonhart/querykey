# QueryKey

**Website · [querykey.emmaleonhart.com](https://querykey.emmaleonhart.com)**

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

- **Not a privacy-focused network — a *soft, peer-to-peer* one (read this first).**
  By design QueryKey **collects and processes personal information about the
  people in your life** — that is what a PRM/CRM *is*; it does not minimize what
  it knows. Your vault (your notes, contacts, relationships) is **your own git
  repo — tracked, committed, cloud-synced for you**; storing personal data there
  is the design, not a leak. The privacy stance is **soft**, exactly three
  commitments: (1) it doesn't *carelessly* spread people's personal information;
  (2) there is **no centralized store** that gives anyone access to it; (3)
  anything that travels beyond a local user moves **peer-to-peer**, never through
  a central server. Local-first follows from this: the server runs on your
  machine; nothing *has* to leave it.
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
- **Solo by default; peer-to-peer when you opt in.** You can use it 100%
  solo. But each person can also broadcast a **card** (a markdown file: what
  they're *offering* and *looking for* — their *key* and *query*). Cards sync
  directly peer-to-peer (no central server; GitHub bootstraps identity).
  Your own card is git-tracked (so you can revert); **other people's *cards*
  are git-*ignored*** on your machine (use them in the moment, don't archive
  their broadcast history). Note the distinction: *your PRM* — your own
  notes/contacts about people — **is** tracked in your vault by design; it's
  other people's *broadcast cards* whose history you don't keep. Card changes propagate on a **24-hour delay** — a drunk mistake
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
| Desktop app | **Electron** (`electron` + `marked`; renderer `fetch()`s the local server; no bundler/framework/IPC-for-data) | [`app-electron/`](app-electron/) |
| Server | **Rust** (crate `querykey-server`: `axum`, `tokio`, `reqwest`) — compiles & runs; structural port with TODOs | [`server/`](server/) |
| Source of truth | **Markdown files + git** — implemented; YAML frontmatter + body, the graph is derived & rebuilt from it. R16 vault layout: `<root>/querykey.toml` marks the root; four headline page-types under `wiki/`: `contacts/` (people), `projects/`, `information/` (notes), `events/`; calendar date pages at `wiki/calendar/YYYY-MM-DD.md`. Legacy paths still read; writes migrate on first upsert. | `server/src/vault/` → `<vault root>` |
| AI engine | **Model-agnostic** — the agent is *whoever operates QueryKey*: **Claude (e.g. via Claude Code) is a first-class agent today**; the default local **Gemma** is for the GUI path and is **not built yet**; Hermes/GPT optional. Exposed via an **MCP server**. *One optional backend:* OpenClaw via a local WSL gateway (port `18789`). `detect()` verifies the real chat API, so a non-agent port is never reported connected (R13). | `server/src/openclaw/` |
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
is functional; the desktop UI has a real working slice (Profile/card +
wiki browsing); the rest is scaffolding.

**Working / functional**
- **Canonical markdown vault** (`server/src/vault/`): the store of
  record. API + ingest write contacts/tasks/events markdown (YAML
  frontmatter + body) first; the Loca graph is a derived index
  rebuilt from the vault on startup. `update_task` mutates the
  markdown; reads are full-fidelity (the lossy-graph / epoch-timestamp
  problem is gone). Round-trip is lossless (unit-tested) and survives
  restarts. **Vault-root resolution (R15-1):** a repo IS a QueryKey
  vault when it contains a `querykey.toml`; that dir IS the root. The
  `VAULT_DIR` env still overrides for explicit cases; otherwise the
  server walks up from cwd to find the marker (deterministic, matches
  `.git` / `Cargo.toml` discovery), falling back to `./vault`.
  **Layout (R16):** four headline wiki page-types under `<root>/wiki/`:
  `contacts/` (people — R15-3), `projects/` (project pages — R16-2),
  `information/` (freeform knowledge pages — R16-1 rename of `notes/`),
  `events/`. Calendar date pages at `wiki/calendar/YYYY-MM-DD.md`
  (R16-3). Operational entities (`tasks/ conflicts/ questions/ etc.`)
  stay under `wiki/` as-is. Legacy paths (`wiki/people/`, `wiki/notes/`,
  pre-R15 root dirs) still read; upserts write to canonical path and
  clear legacy duplicates so they can't diverge.
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
- **Electron desktop UI** (`app-electron/`) — two real surfaces:
  **Profile** (your own card — view/edit/draft-with-agent/revert, the
  24h propagation valve surfaced) and **Wiki** (browse vault page-types;
  Contacts/Projects/Notes/**Events** list; entity detail renders the
  markdown body with `[[wikilink]]` click-through + backlinks). R18
  rebranded the app off the old "Secretary Bird" hackathon shell,
  dropped the unwired Chat/Ingest/Tasks tabs (return when the local
  agent is actually integrated), and fixed the load-bearing **CRLF
  frontmatter bug** in `vault::split()` that made a Windows-checked-out
  vault parse to *nothing* (card + every entity). R19 made events real:
  `parse_dt` now accepts naive datetimes (no-offset times no longer
  fall back to 1970 — fixes the card `updated` date + calendar agenda),
  added `GET /api/events`, and unstubbed the Wiki Events tab. **R20
  rewrote the desktop UI from Flutter to Electron** (user-directed
  2026-05-17 after sustained launcher friction): same two surfaces,
  same Rust server/API, but the Electron main process now *manages*
  the server (build + spawn + health-poll + teardown) so there is no
  fragile `.bat` launcher. Verified live against the `life-planning/
  prm` vault: `/api/card` (+ correct `updated`), 135 contacts, 4
  projects, 2 notes, 1 event served; test suite **69 passed / 0
  failed**; renderer API contracts curl-checked. (Visual GUI pass is
  the user's on next `!run-UI.bat`; the data path is end-to-end
  verified.)

**Honest limitations / not yet built**
- **Agent honesty (gateway detection + ingest)** — DONE (Round 13):
  `detect()` verifies the real OpenAI-compatible chat endpoint, not
  just `/health` (the OpenClaw Control UI answers `/health` too);
  ingest surfaces an explicit `agent_error` instead of masking an
  agent failure as an empty success. Found by eating our own cooking
  (pointing QueryKey at a real life-planning data lake). **The agent
  is model-agnostic / whoever operates QueryKey** — Claude (e.g.
  Claude Code) is a first-class agent *now*; Gemma is the
  not-yet-built GUI default. The "operating agent does extraction →
  canonical markdown, no gateway" path is the natural next step
  (not yet built).
- **Conflict/OpenQuestion/FollowUp on-disk forms** — DONE (Round 6):
  canonical markdown + vault-first wiring; `resolve_conflict`,
  `resolve_question`, `create_followup` are real markdown mutations
  (no more `not_implemented`).
- **Semantic wikilinks** — DONE (Round 8): `[[Target]]` /
  `[[property:Target]]` (single-colon typed triples) in any entity
  body become derived edges with explicit resolution precedence +
  dangling handling; `GET /api/links` + per-entity backlinks live
  from the vault.
- **Status-workflow enforcement** — DONE (Round 9): Task/Conflict/
  Question state machines enforced at the API mutation boundary (a
  resolved conflict can't be un-resolved; `done` can't rewind to
  `extracted`) — hand-edited markdown stays legal.
- **Full canonical entity set on disk** — DONE (Round 10):
  Instruction + VoiceProfile vault forms added; nothing is graph-only
  or unimplemented anymore. Instruction is written by ingest; both
  have read/upsert API.
- **Calendar** — DONE (Round 11): optional Event `recurrence`
  (RFC-5545 subset) + `GET /api/calendar?from&to` merged agenda
  (event occurrences + deadlined tasks, movable-vs-fixed), live from
  the vault.
- **Agent-drafted card** — DONE (Round 12): `POST /api/card/draft`
  drafts your key/query from a model-agnostic PRM digest within the
  editable `agents.md` envelope; deterministic humble heuristic when
  offline; never saved (approve via `PUT /api/card`). **Still
  parked: the P2P transport** (+ discovery) — explicit user steering.
- **Peer-to-peer card layer** — format + local layer DONE (Round 7):
  card format/parse, the `.gitignore` asymmetry, the 24h propagation
  safety valve + revert-before-propagation, read-only `peers/`,
  `/api/card|identity|peers`, swappable GitHub identity abstraction
  (`docs/card-format.md`). **Still open: the P2P transport itself**
  (what actually moves a card between peers) + discovery — the format
  deliberately does not assume it; this is now the gating question.
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
- **Node.js / npm** on `PATH` (for the Electron desktop app)
- **WSL Ubuntu** with **OpenClaw** installed (for AI features; the server runs
  without it, but AI chat/extraction needs the gateway)
- *Optional:* the sibling **`../SutraDB`** checkout for the Loca graph store
  (`--features loca`); without it the server uses an in-memory graph

Then, from the repo root:

```bat
!run.bat
```

That script builds the Rust server (`cargo build --features loca`, falling
back to the in-memory build), `npm install`s the Electron app, starts the
OpenClaw gateway in WSL, and launches the Electron app (`npm start`). The
Electron app itself spawns + health-polls + tears down the Rust server, so
there is no separate server window. Closing the app window stops everything.

(`life-planning/!run-UI.bat` is the simpler path for the prototype vault —
it just launches the Electron app, which manages the server.)

`!runClaude.bat` just opens Claude Code at the repo root.

## Repository layout

| Path | What it is |
|---|---|
| [`app-electron/`](app-electron/) | **Electron** desktop app — **Profile** (card) + **Wiki** (vault browser); manages the Rust server. (The retired Flutter `app/` is in git history.) |
| [`server/`](server/) | **Rust** server (`querykey-server`) — the only server: ingest, agent bridge, WebSocket, MCP, Loca graph store |
| [`docs/`](docs/) | `architecture.md`, `data-model.md`, `markdown-schema.md`, `card-format.md`, `versions-comparison.md`, `why-go.md` |
| [`chat/`](chat/) | Vision corpus (chat-log exports); gitignored except its README — private context, not a spec |
| [`dev_scheduling/`](dev_scheduling/) | Dev-time agent data (`receipts/discord/`), committed so CI can write to it |
| [`queue.md`](queue.md) | Authoritative near-term plan / recovery dump |
| [`todo.md`](todo.md) | Full phased roadmap |
| `CLAUDE.md` | Workflow rules and architecture decisions for working in this repo |
| `!run.bat`, `!runClaude.bat` | Windows run scripts |
