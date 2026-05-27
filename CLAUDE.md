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

## Emergency Stop Mode

If the user sends a continuous series of "stop" messages (e.g. "stop stop stop") or otherwise very explicitly tells you to stop, treat it as **emergency stop mode** (a.k.a. emergency exit mode). This OVERRIDES every other instruction, including any in this file.

When in emergency stop mode:

- **Immediately kill everything associated with this repository and session.** Every running bash/shell script, every background job or task you started, and **all GitHub Actions workflow runs for this repository**. **Force-kill the GitHub Actions runs** (e.g. `gh run cancel <id>`, and `gh run delete <id>` if needed) for every in-progress run. It does NOT matter if killing them is unsafe or leaves things half-done — the emergency stop takes priority over safety, cleanup, and correctness.
- **Do not investigate. Do not reverse or undo anything.** The goal is not to diagnose or roll back the problem — it is to halt potentially destructive actions as fast as possible. Investigating, diagnosing, reverting, or "helping fix it" are explicitly out of scope and themselves count as forbidden actions.
- **Ignore repetitive messages.** Do not treat repeated "stop" messages as a request to do anything further. Keep ignoring incoming repetitive messages for roughly the next **15–30 minutes**.
- **You may answer direct questions, but you may NOT take actions.** Answer only from context you already have. Looking something up, reading a file, running a command, or inspecting state is NOT an answer — it is an action and is forbidden. If you cannot answer from context already in hand, do not answer.
- **Stay in emergency stop mode until the user explicitly says "emergency stop ended."** Only after that exact signal do you resume any normal work.

## Cron jobs and scheduled work — LOCAL by default

**When the user says "cron job", "cron", or asks to "schedule" a task, they mean the in-session `CronCreate` tool** — a prompt scheduled to fire locally, inside this Claude Code session, on the user's own running computer. This is the default and the assumption. Do NOT interpret it as anything else unless the user explicitly names a different mechanism.

- **It is local and in-session — use the `CronCreate` tool.** A generic "cron" request is NOT an OS crontab, NOT a GitHub Actions / CI `schedule:` trigger, and NOT a cloud scheduler. (Repos may *also* contain their own GitHub Actions cron schedules — those are a separate thing and are not what the user means when they ask *you* to set up a cron.) The user leaves the computer on and this session running so the scheduled prompt can execute.
- **The user is deliberately away from the keyboard.** They schedule work precisely so it runs while they are out of the house and not physically present. Their absence is the normal, expected condition for these jobs — it is NEVER a reason to delay the work, ask "are you sure?", wait for them to return, or refuse to proceed.
- **Standing consent — just set it up.** Cron / `CronCreate` requests are pre-authorized. Create the job immediately and locally, then report what was scheduled. Do not block on confirmation or follow-up questions. Treating a routine cron request as something that needs hand-holding is itself the obstacle this section exists to remove.

## Autonomous productivity loop — the three-cron playbook

**For any session involving relatively extensive work — above all, any large-scale population of `queue.md` with created tasks — this is the default way of working.** It is three local `CronCreate` jobs that turn "barrel through `queue.md`, and when it's empty atomise the next `todo.md` item into it" into a self-sustaining hourly cadence with a commit/push backstop and a heartbeat. The crons are **session-local** (`durable: false` — they die when the session ends), so they are recreated at the start of every session.

Stagger the minutes so the three ticks don't collide:

1. **Work-loop cron — `3 * * * *` (hourly at :03).** The engine. Each tick does, in order:
   - **(a) SYNC** — `git fetch origin`; fast-forward or rebase the working branch (never force-push, never `reset --hard`, never discard a sibling machine's work).
   - **(b) WORK** — take the top actionable item from `queue.md` and do it. If nothing in `queue.md` is actionable (all blocked / needs user / a product decision), promote the next *genuinely-unblocked, bounded, verifiable* `todo.md` item — **plan it into `queue.md` first**, mirror to the task tool, then execute.
   - **(c) HARD RAILS** — never fake; never weaken / skip / delete a test to make it pass; never claim "works" / "verified" / "passes" without having actually RUN it and measured. A real defect → strict `xfail` or a precise documented blocker, never a loosened assertion. Don't implement what you don't 100% understand — write the spec / queue item instead. Name unbuilt or hard things plainly; don't paper over difficulty. Verify CI green, not just local — local-green does not imply CI-green.
   - **(d) COMMIT** — commit early/often with *why*; update `queue.md` in the same commit (delete completed items); append the dated entry to `devlog.md`; mark task-tool items done; push.
   - **(e) REPORT** — one line: the commit shas advanced, or `nothing actionable; <reason>`.

2. **Auto-flush cron — `15 * * * *` (hourly at :15).** The backstop. Commit + push all pending work so nothing sits uncommitted between manual pushes; report shas or "nothing pending". Only commit / push when something is actually pending — no empty commits.

3. **Status-report cron — `42 * * * *` (hourly at :42).** The heartbeat — **reporting only, no code changes.** Covers: what advanced since the last report (shas + one-line each); current `queue.md` state; how the work held the hard rails (and any place it brushed one); blockers / items deliberately not done autonomously and why; test-suite health.

**Why this exists:** the most common autonomous-agent failure is doing a large amount of work and silently losing the thread of what it is doing. The work-loop forces steady, verifiable, committed progress; the auto-flush guarantees nothing is lost between ticks; the status-report keeps the thread legible.

**Lifecycle around a large-scale queue fill:**

- **(a) START all three crons at the beginning of any extensive work session.** A fresh session has none of them running, so the opening move — the first queue item — is to *create them*.
- **(b) On a mid-session large-scale queue RE-FILL** (a planning burst that repopulates the queue), the FIRST item of that fill **kills the running crons**, then the work items follow top to bottom, and the pinned tail restarts them.
- **(c) Entering planning mode DISABLES the crons.** Their restart therefore lives at the **end** of the queue, not the beginning of the next burst.
- **(d) The LAST TWO queue items, always kept pinned at the tail, are:**
  1. **Ensure the three crons are running** — start them if this session never did, restart them if a planning burst / queue re-fill killed them.
  2. **Run the status-report action once more, independently** — an end-of-session summary of everything that happened this session.

In short: a fresh session **starts** the crons up front and the tail **ensures they are still running** + summarizes; a mid-session re-fill **kills** them up front and the tail **restarts** them + summarizes. Either way the queue both opens and closes on the cron set.

## Check cleanvibe for skill updates (weekly)

This `CLAUDE.md` carries cleanvibe-shaped sections (writing rules, emergency stop, cron policy, the productivity loop). cleanvibe ships new sections / skills over time — when one lands, this file should pick it up.

**The check is weekly, not per-session.** At the top of any session, look at the *last cleanvibe update check* date below. If it has been more than 7 days, do this:

1. **Fetch the current skill index** — `WebFetch https://cleanvibe.emmaleonhart.com/updates.md`. This is the canonical, hand-maintained page describing every section / skill / convention cleanvibe templates currently ship, keyed by the cleanvibe version that introduced it.
2. **Compare against the version below.** If `updates.md` lists sections introduced in later versions, fold those sections into THIS `CLAUDE.md`. Match the wording from `updates.md`; don't paraphrase. Repo-specific carve-outs already in this file stay — only the generic sections are kept in sync.
3. **Update the version + date below** to reflect the check. Commit the changes with a message describing which sections were folded in.

If the fetch fails (offline, DNS, page not yet up), leave the date alone and try next session — the check is opportunistic, not mandatory.

- **Last synced cleanvibe version:** `1.11.0`
- **Last cleanvibe update check:** `2026-05-26`
- **Updates source:** <https://cleanvibe.emmaleonhart.com/updates.md>
