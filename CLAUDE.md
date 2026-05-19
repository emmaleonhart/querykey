# QueryKey

## Workflow Rules
- **Commit early and often.** Every meaningful change gets a commit with a clear message explaining *why*, not just what.
- **Do not enter planning-only modes.** All thinking must produce files and commits. If scope is unclear, create a `planning/` directory and write `.md` files there instead of using an internal planning mode.
- **Keep this file up to date.** As the project takes shape, record architectural decisions, conventions, and anything needed to work effectively in this repo.
- **Update README.md regularly.** It should always reflect the current state of the project for human readers.

## Queue & longer-horizon work (the flow — read this)

Work flows in one direction and never leaves residue behind:

**`todo.md` (abstract horizons) → `queue.md` (concrete steps) → task tool (in-flight) → `git log` (history).**

- **`queue.md`** — *concrete, executable, in-flight steps only.* Plan-first: when any non-trivial multi-step work is planned, the plan is written here **before** execution so an interrupted session resumes from the queue, not from chat. When an item is done, **delete it in the same commit as the work** — no `[x]` checkmarks, no "DONE/COMPLETE" blocks, no per-Round logs. If it's in `queue.md` it is *not done*; if it's not in `queue.md` it is *not in scope* this session. (Round 14 pruned 882 lines of accumulated completed-Round logs that violated this — do not regrow it.)
- **`todo.md`** — the **long-horizon, abstract** roadmap: multi-session goals, architectural ambitions, future capabilities. Items are *destinations, not steps*. When work begins, pull an item from `todo.md`, decompose it into concrete steps in `queue.md`, mirror into the task tool, execute, delete. As `queue.md` drains, refill from `todo.md`.
- **`git log`** — the narrative history. **Finished work lives here** (each change is its own commit whose message is the record). Never keep a "recently completed" section anywhere else.
- New ideas surfacing mid-work go to the **bottom** of `queue.md` (or to `todo.md` if long-horizon), never silently into the in-flight task.

## Project Description
QueryKey is a **rationalist social network** that doubles as a local-first personal relationship manager (PRM) / lightweight CRM / JIRA-style task tracker, run from your own desktop. It ingests the unstructured streams of how you actually communicate (Discord chats, voice notes, screenshots, pasted text) and uses a **local AI agent** to build a private model of the people and commitments in your life. It then helps you, proactively and quietly, keep those relationships and commitments in good standing.

**Why "QueryKey":** the name references the **Q / K / V** (query, key, value) projections of a transformer attention matrix. Your day, relationships, and tasks are a body of *values*; the local agent attends over them by computing *queries* from your current intent against *keys* built from your markdown notes, chat logs, and prior conversations.

The engine grew out of an earlier prototype (a different, broader product) that has been deleted from the tree — see [`docs/versions-comparison.md`](docs/versions-comparison.md) for what was salvaged and why. Some scaffolding from that lineage is still being reoriented toward the QueryKey vision; see `queue.md` (authoritative plan), `todo.md`, and the **Status** section of `README.md`.

## Architecture and Conventions

> Settled, not up for debate (see `queue.md`): **Electron** for the
> desktop UI (the project owner replaced Flutter with Electron on
> 2026-05-17 after sustained launcher/tooling friction — counsel was
> given twice and heard; this is final, do **not** revert or
> relitigate); **Rust** is the server target; the **local AI agent is
> model-agnostic with Gemma as the default**. Do not relitigate these.

- **UI Framework**: **Electron** (`app-electron/`) — user-directed
  2026-05-17, replacing Flutter. Stack is deliberately minimal
  (Electron + `marked`; renderer `fetch()`s the local Rust server
  directly; no bundler/framework/IPC-for-data) because the owner's
  failure mode here was "stuff doesn't run". Electron also *manages*
  the Rust server (spawn + health-poll + teardown) so there is no
  fragile `.bat` launcher. The retired Flutter app lived at `app/`
  (recoverable from git history); the migration is queue Round 20.
- **Source of truth**: **markdown files on the user's disk, tracked in a git repo** — **IMPLEMENTED** in `server/src/vault/` (Round 5; layout extended in Rounds 15–16). YAML frontmatter (structured fields) + freeform body (the description); Obsidian-usable without QueryKey. **Vault-root resolution (R15-1):** a directory IS a QueryKey vault when it contains a `querykey.toml`. Precedence: (1) `VAULT_DIR` env override, (2) walk up from cwd to nearest `querykey.toml`, (3) fallback `./vault`. **Layout (R16):** graph entities live under `<root>/wiki/`. **Four headline wiki page-types:** `contacts/` (people — R15-3, API key still "people"), `projects/` (project pages — R16-2, new), `information/` (freeform knowledge pages — R16-1 rename of `notes/`, API key still "notes"), `events/`. Calendar date pages at `wiki/calendar/YYYY-MM-DD.md` (R16-3). Operational entities (`tasks/ conflicts/ questions/ followups/ instructions/ voiceprofiles/`) stay under `wiki/` as machinery, not headline types. `card.md`, `agents.md`, `peers/`, `.querykey/`, `.gitignore` stay at the vault root. Legacy paths (`wiki/people/`, `wiki/notes/`, pre-R15 `<root>/<entity>/`) are still readable; writes always go to canonical path and clear legacy duplicates (migrate-on-write). Wikilinks in any entity body (contacts/projects/information/tasks/events) become derived graph edges. Spec + as-built notes: `docs/markdown-schema.md`. Person/Task/Event/Conflict/OpenQuestion/FollowUp/Instruction/VoiceProfile/Project now have canonical on-disk forms — the **full entity set is on disk**, nothing graph-only. **Semantic wikilinks (Round 8):** `[[Target]]` / `[[property:Target]]` — resolved (precedence + dangling) into derived edges; live at `/api/links`. **Status-workflow (Round 9):** Task/Conflict/Question transitions enforced at the API boundary. **Calendar (Round 11):** RFC-5545-subset recurrence + `GET /api/calendar` merged agenda. **Agent-drafted card (Round 12):** `POST /api/card/draft`. **Calendar date pages (R16-3):** `POST /api/calendar/generate` — idempotent generator for [today−6mo, today+6mo] window; machine-delimited events section, no-clobber of user content. **Projects (R16-2):** `GET/POST /api/projects`, `GET /api/projects/:id`. **Calendar grid (R21):** `GET /api/calendar/dates` (sorted ids of dates that have a `wiki/calendar/<date>.md` page) + a `calendar` kind on `get_entity`/`GET /api/entities/calendar/:date` (reads that date page) — both additive read-only, no existing route changed; powers the Electron Calendar surface.
- **Knowledge graph**: **derived from the markdown, not canonical.** RDF/graph is generated *out of* the files; it is a rebuildable secondary index. Store: **Loca** (formerly **SutraDB**) — the author's own embedded **Rust** graph-vector-time DB (separate project at `../SutraDB`; the time dimension matters — you care about a relationship's history). Wired in the Rust server via `loka-core` behind `--features loca` (`src/graph/loca.rs`); an in-memory backend is the default so the crate builds without the SutraDB checkout. **Fuseki is NOT used at all** (the old Fuseki stub was removed with the Go server).
- **AI engine**: model-agnostic. QueryKey exposes an **MCP server** (present from day one) so *any* agent can attend over the graph and act on the files. Default agent: local **Gemma** (cheap, private); Claude/GPT optional for power users. *Implementation note:* today's bridge is OpenClaw via a WSL gateway (`127.0.0.1:18789`) under `server/src/openclaw/` — an implementation detail behind a model-agnostic interface. Callers must never name a specific model/engine. **The agent is whoever operates QueryKey:** Claude (e.g. via Claude Code) is a first-class agent *now*; the default local **Gemma** is the GUI path and is **not built yet**; Hermes/GPT optional. The OpenClaw WSL gateway is just *one optional backend*, not "the agent". **Agent availability must be reported honestly** (Round 13): `detect()` verifies the real chat API (not just `/health` — the OpenClaw Control UI answers that), and ingest surfaces an explicit `agent_error` rather than a silent empty result. A misconfigured/absent agent is *said*, never masked — this is the epistemic-humility principle, not optional.
- **`agents.md`** (name/shape flexible — may become an `agents/` dir): the local agent's behavior, heuristics, and prompts live as an **editable, version-controlled markdown file in the user's repo**, not baked into code. Transparent and auditable (not a black box); two users' `agents.md` produce radically different behavior from the same system. The MCP server executes within whatever envelope `agents.md` defines. It is also what drives **agent-drafted cards** (the agent writes a first draft of your key/query from the PRM it built; you approve).
- **Server**: **Rust** (`server/`, crate `querykey-server`) — the *only* server; local-first: ingestion, the derived graph (Loca), MCP, real-time sync. **There is no more Go.** The previous Go implementation was fully ported then deleted (Round 4); it is recoverable from git history if ever needed (the last commit containing `server-go-old/` — same disposition as `secretarybird-old/`). Build: `cargo build` (in-memory), `--features loca` (Loca graph), `--features discord` (serenity bot). Remaining in-code TODOs are non-blocking (markdown-canonical write path, deprioritized Discord deep logic per todo.md Phase Z, MCP stdio/SSE transports).
- **Identity / sync**: **GitHub** bootstraps identity (usernames) and sync (the git repo). Design it as a thin abstraction — "a user is a canonical handle that currently resolves via GitHub" — so it can be swapped (DIDs/Nostr) later without baking GitHub in everywhere. **IMPLEMENTED** in `server/src/identity/` (Round 7): `CanonicalHandle` + `IdentityProvider` trait; `default_provider()` is the only site that names GitHub. *Discovery* (whose cards you pull) is part of the unresolved transport question — not built (no network).
- **Peer-to-peer card layer** (built *after* the solo PRM): each user broadcasts a **card** — a markdown file of what they're *offering* and *looking for* (their *key* and *query*). Pure P2P, no central server. **Your own card is git-tracked** (enables revert/undo); **other people's cards are git-ignored on your machine** (usable in the moment, not archived — intentional asymmetry, no surveillance). Card changes propagate on a **24-hour delay** (privacy safety valve); a revert before propagation is immediate. Soft, non-cryptographic guarantee — sufficient for a community that doesn't assume bad actors. Private-vs-public cards: planned, not now. **Format + local layer IMPLEMENTED** in `server/src/card/` (Round 7): format/parse, the `.gitignore` asymmetry, the 24h propagation safety valve + revert, read-only `peers/`, and `/api/card|identity|peers`. The **transport** that moves a card between peers is deliberately NOT built — it remains *the* open question; the format does not assume it.
- **Ingest surfaces**: local markdown + pasted text / screenshots / voice notes are primary. **Discord is deprioritized** to `todo.md` Phase Z (feature-gated serenity skeleton only; user is unsure how much they'll use it). WhatsApp/Instagram/Slack ride behind Discord.
- **Data model**: See `docs/data-model.md` — Person, Handle, Task, Event, Message, Conflict, FollowUp, etc.
- **Architecture**: See `docs/architecture.md`
- **Stack-history note**: `docs/why-go.md` / `docs/versions-comparison.md` are **historical** — they argue against the old Electron+Python stack (lessons still hold) and describe a then-current Go server. Go has since been fully replaced by Rust and deleted; treat any "Go server" wording in those docs as history, not current state.
- **Roadmap**: `queue.md` (authoritative, near-term) and `todo.md` (phased)

### Development Data (`dev_scheduling/`)
Provisional directory for agent data during development. Committed to the repo so GitHub Actions can write to it.
- `dev_scheduling/receipts/discord/` — JSON message logs extracted by the Discord bot via GitHub Actions

### Key Design Decisions
- **NOT privacy-focused — soft & peer-to-peer (read this).** By design QueryKey collects/processes personal information about other people (that is what a PRM *is*; it does not minimize what it knows). The soft stance is exactly three things: (1) no *careless* spreading of personal info; (2) **no centralized store** of it; (3) anything beyond a local user moves **peer-to-peer**, never via a central server. Local-first follows: server on your machine, nothing *has* to leave.
- **A user's vault is their own tracked git repo — committing personal data into it is the design, NOT a leak.** Never gitignore / `git reset` / uncommit / "privacy-scrub" a user's vault PRM. The *software project* repo (this one) must contain **zero** personal data; a *user-vault* repo (e.g. the `life-planning` prototype) tracks people's data by design. Conflating the two, or reverting a user's committed vault data on a privacy guess, is a serious error — it happened once and the user had to force-push to defend her work.
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

## Writing
- Do not use "honest", "honesty", or "honestly" — and do not swap in "frank", "frankly", "candid", "candidly", or "transparently", which are the same self-congratulatory move in a different coat. When something failed, name the failure: "it didn't work", "I got that wrong", "this failed" — flat, no qualifier. Tagging a report "honest" implies the rest aren't, and couching a failure as honesty asks for credit for the admission, which is worse than the failure itself. Use a precise positive word ("accurate", "plainly", "truly") only when that is genuinely the meaning — never as a halo on a bad outcome.
