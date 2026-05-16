# QueryKey — Queue

> **Purpose.** This is the working queue for the QueryKey pivot. It captures
> the current vision, open decisions, and the ordered set of things to do
> next. The very first section is a **recovery dump** so any future session
> (or a different machine) can pick up exactly where this one left off.

---

## RECOVERY QUEUE 2

See the new bigger chat I just imported, please extract it and move forward since it has more vision, once the timing is complete we can move onto the other stuff. Remembering here that the queue might not be ptoperly cleared with the size of it, but not sure

Please keep this section when rebasing with the remote

> **Status (extracted 2026-05-15).** The bigger chat was extracted via
> Sutra `extract_chat.py` and now lives at
> `chat/public/vision - rationalist p2p social network (Claude).md`
> (39 KB, 19 exchanges) — it **replaces** the prior 27 KB extraction.
> **Future/other sessions: read THIS file as the authoritative vision.**
> New material vs. the prior version (folded lightly into the docs;
> deeper reorg left for the next session):
> - The **card is literally a Q/K pair** — `query` = what you're
>   looking for, `key` = what you offer. **V is not stored**: it's the
>   real-world outcome of people actually connecting (epistemically
>   honest, anti-gamification).
> - **Card ≠ profile.** The card is a lean *signal* (query + key + a
>   short bio or a link out to your personal site). Your personal
>   website is the substance/source of truth for "who you are"; the
>   card just points to it. Keeps the P2P payload small and the format
>   stable.
> - **Agent-drafted cards.** People (esp. younger) are bad at
>   articulating their own value; the local agent drafts `key`/`query`
>   from the PRM it already built by observing you — you curate/approve.
> - **`agents.md`** (name/shape flexible — could be an `agents/` dir):
>   the local agent's behavior/heuristics/prompts as an editable,
>   version-controlled markdown file in your repo. Transparent, not a
>   black box; the MCP server executes within that envelope; the
>   rationalist community will share/compare configs.
> - **Strategy:** stop over-engineering the social layer; the PRM is
>   "pretty complete" — next step is real daily use (eat own cooking),
>   rationalist/LessWrong as first cohort.

## RECOVERY DUMP — written 2026-05-15

The user's laptop battery was about to die in the middle of a planning
session, so this dump is intentionally redundant with the rest of the file.
If you are a fresh session reading this for the first time, **start here**.

### Where the session left off

- The user dictated a long voice-to-text message describing a large pivot of
  this repo: rebrand QueryKey as a **rationalist social network** that is
  also a PRM / lightweight CRM / JIRA-style task tracker, **local-first**,
  with people tracking tasks as **markdown files on their own computer** and
  a **local AI agent** (default: Gemma; model should be switchable) acting
  on those files.
- The name "QueryKey" comes from the **Q / K / V (query / key / value)** of
  an attention matrix. That etymology was not yet documented anywhere in
  the repo and needs to be.
- Three Explore agents ran a full inventory of the repo (see findings
  summarized below).
- Three decisions were resolved with the user via a clarifying question:
  1. **Delete `secretarybird-old/`** after writing a comparison doc that
     captures anything worth salvaging.
  2. **Commit to Rust** for the server. The current Go server (`server/`)
     was accidentally Go (the user said "go" in a prompt and the tool ran
     with it). Mark Go server as deprecated.
  3. **Reframe the AI engine as a model-agnostic local agent**, with
     **Gemma as the default**. Push the OpenClaw / Hermes naming out of
     the high-level vision docs entirely. OpenClaw stays in the tree only
     as an implementation detail under `server/internal/openclaw/` until
     the Rust rewrite supersedes it.
- Flutter is **locked in** for the UI — the user is very confident about
  this. Do not relitigate.
- The session was ended (or paused) before any of the actual writes
  below were performed.

### What needs to happen next (ordered)

These are the action items the session was about to execute. They are
deliberately small so any one of them can be done independently and
committed.

1. **Create `chat/`** at the repo root with a `README.md` explaining
   that this directory is where the user dumps chat-log exports (Discord,
   ChatGPT, Claude, voice transcripts, etc.) that contain context about
   the project's vision. Note that contents may be large and informal;
   future agents should treat the dir as a corpus to read selectively,
   not a spec to follow literally.
2. **Create this `queue.md`** (this file). ← *done as of the recovery
   dump itself.*
3. **Delete `cleanvibe_examples/`** and **`README_cleanvibe.md`** —
   verified to be empty scaffolding (3 stub files, no actual code or
   content). They were created by the `cleanvibe` tool and add nothing.
4. **Write `docs/versions-comparison.md`** comparing `secretarybird-old/`
   (Electron + Python + WSL socket chain, hackathon era) against the
   current `app/` + `server/` (Flutter + Go). The user's stated belief
   is that the pivot version is superior in every way; the doc should
   either confirm that or surface specific things worth porting.
5. **Delete `secretarybird-old/`** entirely after #4. This also removes
   100% of the hackathon references, which all live inside that
   directory (the hackathon went badly and should not be visible).
6. **Reframe `README.md`** around the new vision: rationalist social
   network + PRM/CRM/JIRA framing, Q/K/V naming origin, local-first
   markdown task model, Rust as the server target (with a note that the
   current Go server is being deprecated), Flutter for UI, model-agnostic
   local agent (Gemma default, switchable). Keep the existing
   "what's actually in the tree" / "status" honesty — do not oversell.
7. **Reframe `CLAUDE.md`** — drop the "Secretarybird Pivot" heading and
   any Secretarybird/Discord-bot-first framing. Rewrite around QueryKey
   as the rationalist-social-network + personal-task-graph product, with
   Rust + Flutter + local agent as the stack. Keep the workflow rules
   block (commit early, no planning-only modes, keep README updated).
   Update the "Project Description" and "Architecture and Conventions"
   sections to match the new framing.
8. **Reframe `todo.md`** — currently 100% in old Secretarybird framing
   (team-coordination secretary bot). Rewrite around the QueryKey vision
   while preserving phase structure where it still applies (data model,
   ingest pipeline, calendar, audio). Move the "team coordination" /
   "Discord bot DMs every team member" framing out of the headline and
   into a later phase or drop it.
9. **Verify and (likely) update `site/index.html`** — the existing copy
   already says "A social network you run locally from your own
   desktop", which is on-message. Add the rationalist-network framing and
   the PRM/CRM/JIRA angle if the user wants it surfaced publicly; check
   first.
10. **Commit each of the above as its own commit** with a message
    explaining *why* (per `CLAUDE.md`'s workflow rules). Push to
    `origin/main` so a different machine can resume.

### What is *not* in scope for this round

- ~~The actual Go → Rust rewrite of the server.~~ **Superseded:** done
  in Round 3 (see below) at the user's explicit request. The server is
  now Rust (`server/`); Go archived in `server-go-old/`.
- Replacing OpenClaw with Hermes (or anything else) in code. The
  implementation lives untouched under `server/internal/openclaw/` for
  now.
- Building the markdown-file local task model. Document the model in
  `todo.md` / `docs/`, do not implement it yet.
- The "data lake for additional planning stuff" the user mentioned in
  passing. Defer.

### Files that already exist and matter

- `README.md` — already partially pivoted to QueryKey framing; needs
  expansion (rationalist network, Q/K/V, markdown model, Rust, Gemma).
- `CLAUDE.md` — still says "Secretarybird Pivot" at the top; needs full
  reframe.
- `todo.md` — entirely in old framing; needs reframe.
- `docs/architecture.md`, `docs/data-model.md`, `docs/why-go.md` — still
  from the previous era. `why-go.md` will become misleading once the
  Rust pivot lands; either retitle to `why-not-electron-python.md` (its
  real subject) or fold its lessons into a new `docs/why-rust.md`.
- `site/index.html` + `site/CNAME` — public landing page, already
  on-message.
- `app/` (Flutter) — keep, expand.
- `server/` (Go) — keep compilable, mark deprecated.
- `secretarybird-old/` — to be deleted after comparison doc.
- `cleanvibe_examples/`, `README_cleanvibe.md` — to be deleted.
- `dev_scheduling/receipts/discord/` — committed-empty data dir for CI;
  decide later whether the Rust pivot keeps this shape.

### Where the in-flight plan file lives (local only)

`C:\Users\ambie\.claude\plans\okay-so-first-things-sharded-candle.md`

That file is on the user's machine only — it does not survive a switch
to another computer. This `queue.md` is the durable, committed copy.

---

## Vision

QueryKey is a **rationalist social network** that doubles as a personal
PRM / lightweight CRM / JIRA-style task tracker. It runs on your own
machine. It is built around the working theory that the most useful
software for thinking carefully about people, commitments, and the
state of your own projects is software that:

- you fully own and run locally,
- holds its data as plain markdown files on your disk that you can read
  and edit by hand,
- is operated mostly by a **local AI agent** that you can swap models
  for (default: **Gemma**), and
- treats epistemic humility — confidence scores, "I'm not sure, want me
  to ask?" — as the central UX, not a footnote.

### Why "QueryKey"

The name is a reference to the **Q / K / V** (query, key, value)
projections inside a transformer attention matrix. The product
metaphor: your day, your relationships, and your tasks are a body of
*values* that the local agent attends over by computing *queries* from
your current intent against *keys* it has built from your markdown
notes, chat logs, and prior conversations.

### Product surfaces

- **Local markdown files** on disk — the source of truth for tasks,
  events, and notes. Users can edit them in any editor.
- **Flutter app** — desktop-first (Windows now; macOS/Linux/iOS/Android/
  Web later). The interactive surface for chat with the agent, task
  boards, calendars, etc.
- **Local agent** — runs on the user's machine. Default model Gemma;
  switchable to other local models or, optionally, hosted ones.
- **Server (Rust, target)** — the local backend that wires the agent,
  the markdown files, the graph, and the app together. Currently a Go
  implementation lives under `server/`; this is being deprecated.

### What it is *not*

- Not a SaaS. Not a team coordination tool you're forced to adopt. Not
  a surveillance/productivity scoreboard.
- Not opinionated about *how* you work — it conforms to your workflow.

---

## Open decisions (resolved this round)

| Decision | Choice | Notes |
|---|---|---|
| Disposition of `secretarybird-old/` | **Delete** after comparison doc | Hackathon refs vanish in one move. Code preserved in git history. |
| Server language | **Rust** (target) | Current Go server (`server/`) marked deprecated. No rewrite this round. |
| AI engine framing | **Model-agnostic via MCP, Gemma default** | MCP server present day one; any agent can attach. OpenClaw is an implementation detail until the Rust rewrite. |
| UI framework | **Flutter** | Locked. Not up for debate. |

## Open decisions (resolved in Round 2 — from the vision/strategy chat)

> Source: `chat/public/vision - rationalist p2p social network
> (Claude).md`. These were previously "still open"; the user clarified
> them directly.

| Decision | Choice | Notes |
|---|---|---|
| Graph store | **Loca (formerly SutraDB)** | Author's own embedded Rust graph-vector-time DB. **Fuseki is NOT used** — its presence in docs/stub was an error; stub slated for removal. |
| Canonical store | **Markdown + git; graph is derived** | Markdown files are the source of truth; RDF/graph generated *from* them, rebuildable. |
| On-disk format | **YAML frontmatter + freeform body** | Obsidian-style; usable without QueryKey. Spec to be finalized before ingestion code. |
| Social model | **Pure P2P card exchange** | Offer/looking-for cards; own card git-tracked, others' git-ignored; 24h propagation delay; no central server. |
| Identity / discovery | **GitHub (swappable)** | Usernames as handles, follow-on-GitHub discovery, behind a thin handle abstraction. |
| Sequencing | **PRM → P2P; MCP day one** | Private PRM built first (builds the graph the cards window into); card layer second. |

## Open decisions (still open)

- **Card format spec** — the highest-leverage remaining design
  question; it ossifies fast once cards are exchanged. Spec before any
  P2P code.
- Private vs. public card (deferred — more complex; after single-card).
- Audio pipeline ownership in the Rust world.
- Voice-profile / speaker-diarization model selection.
- External tool sync (Jira / Azure DevOps / GitHub) — still desired? In
  what tier?

## Action queue

(Same as the recovery dump's "What needs to happen next", repeated here
for ergonomic editing as items are completed.)

- [x] 1. Create `chat/` with explanatory README — done; corpus moved
      in from life-planning, bodies gitignored (commit 99c9dcb).
- [x] 2. Create `queue.md` (← this file; committed in prior session).
- [x] 3. Delete `cleanvibe_examples/` and `README_cleanvibe.md` — done.
- [x] 4. Write `docs/versions-comparison.md` — done (commit ca3394c).
- [x] 5. Delete `secretarybird-old/` — done; recoverable from git
      history. No hackathon refs remain in the tracked tree.
- [x] 6. Reframe `README.md` — done (commit 2fbe9a9): Q/K/V origin,
      rationalist/PRM/CRM/JIRA framing, markdown model, Rust target,
      Gemma agent, stale refs fixed.
- [x] 7. Reframe `CLAUDE.md` — done (commit a6a5bd2): Secretarybird
      lineage removed, Rust/Gemma framing, workflow rules kept.
- [x] 8. Reframe `todo.md` — done: personal-first rewrite, phase
      skeleton kept, team mode demoted to optional Phase 8, Rust/Gemma/
      markdown folded in. Also corrected `docs/why-go.md` framing note.
- [x] 9. Verify / update `site/index.html` — verified on-message and
      free of stale refs; per an explicit user decision (asked because
      it is public-facing), applied the **full** public reframe:
      rationalist social network + PRM/CRM/JIRA + Q/K/V naming.
- [x] 10. Commit and push each step — each queue item was its own
      commit, pulled `--ff-only` and pushed to `origin/main`.

---

**Action queue COMPLETE (2026-05-15).** All 10 items done and pushed.
This was executed live in the current session (the user asked to start
now rather than wait for the scheduled fallback run). Remaining work is
the deferred, out-of-scope items below (Go→Rust rewrite, markdown
on-disk model, graph-store decision) and the still-open product/design
questions — none of which were in scope for this round.

---

## Round 2 — vision clarified (2026-05-15, evening)

A strategy conversation (now committed at `chat/public/vision -
rationalist p2p social network (Claude).md`) clarified and **corrected**
the architecture. The four canonical docs (`README.md`, `CLAUDE.md`,
`todo.md`, this file) were updated to match: Fuseki removed, graph is
**derived from canonical markdown** and stored in **Loca/SutraDB**, an
**MCP server** is day-one infra, identity bootstraps via **GitHub**,
and the social layer is a **pure-P2P card** model (own card tracked,
others' ignored, 24h delay) built *after* the solo PRM.

### Round 2 action queue (next — barrel through these)

These are documentation/spec + cleanup. Still **not** in scope: the
Go→Rust rewrite, and *implementing* the on-disk model or P2P code
(spec them first). Each item = its own commit, pull `--ff-only`, push.

- [x] R2-1. Reorganize the 4 canonical docs from the vision chat —
      done (commit 6ae8508).
- [x] R2-2. Purge stale pre-pivot framing from `docs/architecture.md`,
      `docs/data-model.md` (and `why-go.md`) — done (commit after
      6ae8508): misleading banners replaced with accurate "superseded
      by" notes; inline Fuseki-as-store assertion fixed.
- [x] R2-3. Write `docs/markdown-schema.md` — done. Canonical on-disk
      spec (YAML frontmatter + body, git, derived graph).
- [x] R2-4. Write `docs/card-format.md` — done. P2P card spec
      (key/query, asymmetric git-tracking, 24h delay, GitHub identity).
- [x] R2-5. ~~Remove the dead Fuseki stub~~ → **adjusted.** On
      inspection the Fuseki client is NOT a dead isolated stub: it is
      wired through `main.go`, `handlers.go`, `router.go`, `bot.go`,
      `config.go`, `pipeline.go` and is load-bearing for the
      *deprecated* Go server's compilation. Excising it = refactoring
      the Go server, which is explicitly out of scope ("no rewrite this
      round; keep it compilable"). Instead added a prominent DEPRECATED
      banner to `fuseki.go` stating Fuseki is not the plan; it will be
      deleted wholesale with the Go→Rust rewrite. The user's actual
      concern (Fuseki *documented as the plan*) is fully resolved by
      R2-1/R2-2.
- [x] R2-6. Commit + push each of the above (per-item commits,
      `--ff-only` pull, push to origin/main).

**Round 2 COMPLETE (2026-05-15).** Docs are organized and consistent.
The remaining real design question is the **card format** (specced but
will evolve); the next *building* work (implementing the markdown model
/ MCP server / Loca integration) belongs to the Rust effort.

---

## Round 3 — Go → Rust port (2026-05-15)

The "Go→Rust rewrite is out of scope" guardrail above was **lifted by
explicit user request** ("just copy the go to Rust with Loka being
used … the go would go into a subdirectory just like the old
secretarybird").

- [x] R3-1. Archived the Go server to `server-go-old/` (deprecated
      reference; `secretarybird-old/` pattern). README maps Go→Rust
      file-for-file.
- [x] R3-2. New Rust crate `querykey-server` in `server/` mirroring the
      Go layout. `cargo build` (in-memory) and `cargo build --features
      loca` (real SutraDB workspace) both compile clean.
- [x] R3-3. **Loca/SutraDB** wired as the graph store via `loka-core`
      (`PersistentStore`), behind `--features loca`; in-memory default
      so the crate always builds. Fuseki fully gone from `server/`.
- [x] R3-4. Smoke-tested: boots, detects the live OpenClaw gateway,
      opens a Loca `.sdb` (`graph_ok:true`), `/health` + SPARQL
      passthrough respond.
- [x] R3-5. `!run.bat` + README + CLAUDE + todo updated: Rust is **the**
      server, Go archived.

**Round 3 building steps — DONE (2026-05-15, scheduled session).**
The structural port has been fleshed out (each its own commit, all
pushed, all three build configs green):
- ✅ Incremental agent streaming — real SSE delta parsing (`1ef5af1`).
- ✅ Persistent SPARQL query bridge — PersistentStore snapshot →
  loka_sparql, smoke-verified (`ee434eb`).
- ✅ Typed graph read-back (persons) via the POS index (`1d6ee13`).
- ✅ MCP endpoint — minimal JSON-RPC `/mcp` (`7bf0f46`).
- ✅ Discord bot port — feature-gated serenity client (`f59cd4b`).

**Remaining `TODO(port)` (deeper, lower priority):** MCP stdio/SSE
transports + `agents.md`-governed write tools; Discord per-channel
filters + hourly-batch-into-pipeline + DM follow-ups; markdown-canonical
read path for tasks/conflicts (the derived graph is intentionally lossy
for those); perf — cache/incrementally maintain the SPARQL snapshot.
None block the server; tracked in `todo.md` Phase 11.

---

## Round 4 — Rust server to parity (goal: NO MORE GO)

**User directive (2026-05-15):** `queue.md` is the barrel-through file
(`todo.md` is the roadmap, not barreled). Build out *all the Rust
server stuff* until `server-go-old/` is no longer needed and can be
deleted. **Flutter stays the frontend (firm).** Discord deep logic is
**deprioritized** to `todo.md` Phase Z — the feature-gated serenity
skeleton stays as-is; do NOT barrel Discord here.

Rules: each item its own commit with a *why*; `cargo build`,
`--features loca`, `--features discord` must all stay green before
committing; `git pull --rebase` + push after each; only stop for a
real conflict/blocker.

- [x] R4-1. **WSL / gateway lifecycle parity** — done. wsl.rs:
      findDistro (null-byte strip, Ubuntu pref), CleanStaleLockFiles,
      ForceKillOpenClaw, StartGateway (returns a tokio Child).
      bridge.rs: detect-first ensure_gateway (Arc<Self>), supervised
      retry loop (start→wait→backoff, max_retries), 10s health check
      that resets retries, graceful stop_gateway/force_kill, plus the
      `x-openclaw-agent-id` header and system-prompt buildMessages.
      All 3 build configs green; boots clean, gateway detected.
- [x] R4-2. **Graph store completeness** — store_task/message/conflict
      enriched to the full fuseki.go field sets (smoke-verified via
      SPARQL). `insert_triples` implemented via
      `loka_core::ntriples::parse_ntriples_line`. `update` validates
      syntax but is a documented read-only limitation (no public SPARQL
      UPDATE writer in loka; writes go via store_*/insert_triples).
      Event/Instruction/FollowUp persistence needs new GraphStore
      trait methods → folded into R4-4 (pipeline parity).
- [x] R4-3. **API handler parity** — `get_all_tasks` added to the
      GraphStore trait (loca: faithful POS-index reconstruction now
      that R4-2 projects all fields; memory: vec). `list_tasks` and
      `/persons/:id/tasks` now return real data (smoke-verified,
      timestamps round-trip). Mutation endpoints (update_task,
      resolve_conflict/question, create_followup) made **honest
      not_implemented** (canonical-markdown write path not built; no
      faking success). All builds clean, zero warnings.
- [x] R4-4. **Ingest pipeline parity** — fixed a real bug: parse now
      uses a *relaxed* schema (the agent returns loose fields, no
      ids/timestamps; the old strict `from_str::<AnalysisResult>` would
      always fail) then constructs full typed models with new uuids +
      timestamps + `source_messages=[ingest_id]`, matching pipeline.go.
      storeResults persists tasks+conflicts (Go parity; events
      broadcast-only). Smoke-verified: POST /api/ingest returns a valid
      result, no crash on non-JSON agent output (Go's fallback).
- [x] R4-5. **WS hub parity** — broadcast_results now emits a typed
      `models::GraphDiff` (added_nodes = tasks+events, new_conflicts);
      `ws.rs` wraps it as `WsMessage{type:"graph_diff", data}` exactly
      like hub.go BroadcastGraphDiff. Fan-out already worked.
- [x] R4-6. **dump-messages** — triaged: **wholly Discord-coupled**
      (discordgo session, guild/channel walk; no non-Discord part).
      Stub kept, re-pointed at `todo.md` Phase Z + the serenity
      feature. Explicitly does NOT block Go removal.
- [x] R4-7. **Parity review + DELETE `server-go-old/`** — done. Parity
      audit: all remaining `TODO(port)` markers are non-blocking
      (architectural markdown-canonical deferrals, deprioritized Discord
      Phase Z, future OCR/audio, MCP transport enhancements) — none
      needed the Go tree. CI is only `pages.yml` (no Go build).
      `server-go-old/` deleted (history preserved in git, like
      `secretarybird-old/`). CLAUDE.md / README.md / todo.md / this
      file updated: **no more Go.**

**Round 4 COMPLETE (2026-05-15).** The Rust server is the sole server;
Go is gone. All R4 items done, each its own commit, all pushed, all
three build configs green. Honest non-blocking TODOs remain (canonical
markdown write path — the load-bearing next piece; P2P card layer;
GitHub identity; MCP stdio/SSE; Phase-Z Discord). In-code comments
that reference `server-go-old/` are left as historical breadcrumbs
(the Go reference is in git history) — not chased, per scope.

---

## Round 5 — Canonical markdown source of truth (load-bearing)

**User directive (2026-05-15):** barrel through implementing the
markdown source-of-truth model (`docs/markdown-schema.md`). This is
*the* load-bearing piece: markdown files are canonical; the Loca graph
becomes a derived, rebuildable index; task/conflict mutation and full
hydration route through markdown (fixes the lossy-graph reads and the
honest `not_implemented` mutations).

- [x] R5-1. **`vault` module** — done. `src/vault/mod.rs`: YAML
      frontmatter + body (de)serialize, idempotent/lossless (stable
      key order; per-entity Fm structs; title in frontmatter so
      nothing is body-derived). `Vault` upsert/get/list for Person /
      Task / Event under `<root>/{people,tasks,events,notes}/`.
      Round-trip unit test passes (caught + fixed a trailing-newline
      bug). serde_yaml dep added. All 3 builds green, zero warnings.
      (`VAULT_DIR` config wired in R5-2.)
- [x] R5-2. **Canonical-first wiring** — done. `VAULT_DIR` config;
      `AppState.vault` + `Pipeline.vault`. API create/list/get and
      `update_task` (now **real** — read vault, patch, write, project)
      go vault-first; reads are full-fidelity from the vault.
      `main` opens the vault and **rebuilds the graph from it on
      startup**. Pipeline `store_results` writes tasks/events to the
      vault then projects (folded in here to avoid a dead field).
      Smoke-verified end-to-end incl. **persistence across restart**
      (a status mutation survived a full reboot via the markdown) and
      the startup `projected vault → graph` rebuild. All 3 builds
      green, zero warnings.
- [x] R5-3. **Docs** — done. `docs/markdown-schema.md` flipped to
      IMPLEMENTED with as-built notes + still-open list;
      CLAUDE/README/todo updated (canonical vault is live, lossy-graph
      problem gone); runtime vault gitignored.

**Round 5 COMPLETE (2026-05-15).** The canonical markdown source of
truth is built and wired: the vault is the store of record, the Loca
graph is a derived index rebuilt from it on startup, `update_task`
mutates markdown, reads are full-fidelity, round-trip is unit-tested
and survives restarts. All three build configs green, zero warnings,
each item its own commit, all pushed. The Round-5 close-out's
Conflict/OpenQuestion/FollowUp gap is now closed in Round 6 below;
`[[wikilink]]` resolution + status-workflow enforcement remain
non-blocking opens; the P2P card layer and GitHub identity remain the
next big pieces.

---

## Round 6 — finish the canonical-vault entity set (2026-05-16)

**Context.** A divergent PR (#1 "Other machine") turned up: stale
parallel work that branched *before* Rounds 3–5. Diagnosed and
resolved — it was strictly behind `main` (merging it would have
resurrected the deleted Go tree and deleted the canonical vault); the
only net-new artifact, `.gitattributes` (LF normalization, stops the
Windows CRLF rewrite-storm `main` still suffered), was cherry-picked
(`-x`, provenance preserved). PR to be closed once the salvage is on
`origin/main`; the `other-machine` branch deleted after.

Then barrelled the natural Round-5 follow-on: the last three entities
with no on-disk form (conflicts were graph-only; questions/followups
were `not_implemented`).

- [x] R6-0. Cherry-pick `.gitattributes` from PR #1; close PR + delete
      branch. **Push blocked by the auto-mode classifier** (direct
      push to `main` from a vague instruction) — PR close/branch
      delete are *gated on the push landing* so the salvage isn't
      lost. Awaiting `! git push origin main` or a push permission.
- [x] R6-1. **Vault on-disk forms** — `conflicts/<uuid>.md`,
      `questions/<slug>.md`, `followups/<slug>.md`, mirroring the
      Person/Task/Event pattern (per-entity Fm struct, stable key
      order, body = the one human field). Lossless round-trip unit
      test (enums, optional timestamps, nested delivery_attempts).
      All 3 builds green, zero warnings. Commit `acd5b99`.
- [x] R6-2. **Vault-first wiring** — ingest writes conflicts
      vault-first then projects; `GET /api/conflicts|questions|
      followups` read the vault at full fidelity; `resolve_conflict`
      / `resolve_question` / `create_followup` are real markdown
      mutations (was `not_implemented`); conflict resolve broadcasts
      `GraphDiff{resolved_conflicts}`. Commit `73c3845`.
- [x] R6-3. **Docs** — `docs/markdown-schema.md` (schemas + as-built
      notes; the three moved from "Still open" → DONE),
      CLAUDE/README/todo updated. (This commit.)

**Round 6 status.** COMPLETE. Pushed (`b3193fe..3056886`); R6-0 also
done — `.gitattributes` on `origin/main`, PR #1 closed with the full
diagnosis, `other-machine` deleted + pruned.

---

## Round 7 — P2P card layer + GitHub identity (2026-05-16)

**User directive:** delete the orphaned
`claude/team-communication-bot-C2WBA` branch (a pre-pivot stub whose
markdown-vault idea is now built+surpassed on `main`), then barrel
the P2P card layer + GitHub-bootstrapped identity. The PRM/vault is
built (R5–R6), so per the adoption sequencing the card layer is the
right next piece. Build the **format** (it ossifies once cards are
exchanged) + identity + the *local* propagation/asymmetry — **not**
the transport (the doc's top open question; the format must not
assume it; no exchange/relay code).

- [~] R7-0. Delete `claude/team-communication-bot-C2WBA` — **blocked
      by the auto-mode classifier** (read "delete it" as ambiguous
      despite the branch being named and the user replying to it).
      Handed to the user as `! git push origin --delete …` (recoverable
      from SHA `ad728a5`). Non-blocking side cleanup.
- [x] R7-1. **Card format module** — `src/card`: Card model +
      render/parse, the stable `## Offering`/`## Looking for` heading
      contract, no `value` field (V is the real-world outcome).
      `Vault::{card_path,get_card,upsert_card}`. **Privacy `.gitignore`
      asymmetry** written on `Vault::open` (ignores `/peers/` +
      `/.querykey/`, NOT `card.md`; idempotent, non-clobbering).
      2 round-trip tests. Commit `f4a9abd`.
- [x] R7-2. **Identity abstraction** — `src/identity`:
      `CanonicalHandle` + `IdentityProvider` trait; `GitHubIdentity`
      normalizes every input form; `default_provider()` the only site
      naming GitHub (swappable). Discovery deliberately absent
      (transport). 2 tests. Commit `792968c`.
- [x] R7-3. **API + propagation + peers** — `card.md` working/tracked,
      `.querykey/card.pending.md`+`eligible_at` staged, `card.published
      .md` the snapshot a transport *would* ship; 24h lazy promotion;
      revert-before-propagation restores from published. Read-only
      `peers/` (FS-safe slug; `:` never hits disk).
      `/api/card|card/published|card/revert|identity|peers`. Full
      cycle + asymmetry unit-tested. Commit `d9763b7`.
- [x] R7-4. **Docs** — `docs/card-format.md` flipped to
      format+local IMPLEMENTED with as-built notes + the narrowed
      open-questions (transport/discovery/private-card/agent-draft);
      CLAUDE/README/todo updated. (This commit.)

**Round 7 status.** Code + docs complete; all 3 build configs green,
zero warnings (the loca dead_code note is the external loka-core
crate); 7 lib tests pass; each item its own commit. The card
*format* and *local* layer are done and unit-tested. **The next
gating question is the P2P transport** (what actually moves a card
between peers) + discovery + the agent-drafted card↔graph projection
(the PRM it would draw from now exists). Only loose end: R7-0's
branch delete, handed to the user.

---

## Direction (2026-05-16) — social layer parked, PRM is the priority

**User decision:** put the P2P / social layer **aside**. The card
*format* + *local* layer (Round 7) is a good stopping point; the
remaining social work is the **P2P transport**, which is an
unresolved *design* question, not just unbuilt code — do **not**
barrel a transport on a guess. The **PRM structure is the better
thing to work on** and is where effort should go next: the social
card is only ever a selective window into a graph the PRM builds, so
deepening the PRM compounds; the social layer can wait for a real
transport decision. Future sessions: prioritize PRM/vault/graph
structure over anything P2P until the user reopens the social track.

---

## Round 8 — semantic wikilinks (2026-05-16, PRM-priority)

First round under the PRM-first direction. **User ask (voice):**
per-person markdown is central; want `[[wikilink]]` *and* semantic
`[[property:wikilink]]` — single colon (NOT SMW's ugly `::`), the
property defining "the triple type." This also resolves the
long-open `docs/markdown-schema.md` question (wikilink vs frontmatter
ref: precedence + dangling).

- [x] R8-1. **Parser** `src/wikilink` — `[[X]]` untyped, `[[p:X]]`
      typed, `[[X|Alias]]` Obsidian alias off the edge; predicate =
      lowercase non-URI-scheme token; forgives `::`. 7 tests.
      Commit `81da6ad`.
- [x] R8-2. **Vault resolver** — `collect_links()` scans every
      person/task/event/note body, resolves targets with explicit
      precedence (kind:id → person → task → event → note → dangling
      `thing`, never dropped), slug-insensitive; `links_from`/
      `links_to` (backlinks). Backend-independent + unit-tested.
      Commit `d2aa501`.
- [x] R8-3. **Projection + API** — `graph::NS` deduped (loca uses it);
      `graph::link_ntriples` (resolved edges + dangling label,
      escaping-tested); startup vault→graph projects links;
      `GET /api/links` + `GET /api/entities/:kind/:id/links` served
      LIVE from the vault (every build config, never stale).
      Commit `1563294`.
- [x] R8-4. **Docs** — `docs/markdown-schema.md` gets a "Semantic
      wikilinks" section (syntax + precedence + dangling) and the
      open item flipped to DONE; CLAUDE/README/todo updated. (This
      commit.)

**Round 8 status.** COMPLETE. All 3 build configs green; 16 lib
tests pass (parser 7, projector 1, vault incl. resolver 8). Each
item its own commit. Markdown stays canonical; the link graph is
derived and rebuildable. Next PRM threads (queued, not started):
status-workflow enforcement; Instruction/VoiceProfile on-disk forms;
agent-drafted card↔graph (PRM exists, heuristic doesn't);
calendar/audio. Code-fence/nested-bracket wikilink edge cases are
documented future refinements.

---

## Round 9 — status-workflow enforcement (2026-05-16, PRM-priority)

Second PRM-priority round. Make the PRM's state trustworthy: the
*server* must not make illegal status jumps behind the user's back.

- [x] R9-1. `src/workflow` — pure transition predicates for
      Task/Conflict/Question, unit-tested (3). Idempotent allowed.
      Task: extracted→confirmed→in_progress→done + disputed
      side-state + explicit reopen; never rewind done→extracted.
      Conflict: never return to unresolved (the user's rule).
      Question: open→resolved|expired, expired re-askable, resolved
      terminal. Commit `71189bc`.
- [x] R9-2. Enforced in `update_task` / `resolve_conflict` /
      `resolve_question` against the *current persisted* state;
      illegal jump → `{error,from,to}`, vault NOT mutated. Vault
      `upsert_*` deliberately NOT gated (hand-edited markdown /
      Obsidian imports stay legal — "the tool serves you"). All 3
      configs green; 19 lib tests. Commit `a9052e9`.
- [x] R9-3. Docs — `docs/markdown-schema.md` open item flipped to
      DONE; CLAUDE/README/todo updated. (This commit.)

**Round 9 status.** COMPLETE. Each item its own commit. Next PRM
thread: Instruction/VoiceProfile on-disk forms (the last two
unimplemented canonical entities), then calendar/audio; the
agent-drafted card↔graph projection remains design-heavy/social-
leaning (parked-social steering applies — flag before barrelling).

## Notes for future sessions

- The user dictates long stream-of-consciousness messages via voice. Do
  not interpret them literally — listen for the underlying intent.
- The user has explicitly asked to **avoid planning-only modes** in
  `CLAUDE.md`. Plan mode was used this once because the scope of the
  pivot warranted it; default to executing.
- Do not reintroduce the "Secretarybird" name into new docs.
- Do not reintroduce hackathon references anywhere.
- Flutter is settled (firm — frontend). Rust **is** the server (Go
  fully removed in Round 4; only git history has it). Local-agent /
  Gemma is the AI framing. These are not open questions.
- **Fuseki is NOT used.** If you see Fuseki anywhere it is stale; the
  graph store is **Loca/SutraDB**, derived from canonical markdown.
- Markdown + git is the source of truth; the graph is rebuildable from
  it. MCP server is day-one infra. Identity bootstraps via GitHub
  (swappable). Social layer is pure-P2P cards, built after the PRM.
- Don't relitigate the Round 2 resolved decisions above. The card
  *format* is now built (Round 7) — the one real open design question
  is now the **P2P transport** (what actually moves a card between
  peers) + discovery; the format deliberately does not assume it.
- **Priority (2026-05-16): PRM structure > social layer.** The P2P
  track is parked pending a transport *design* decision; barrel PRM/
  vault/graph structure until the user reopens social. See "Direction
  (2026-05-16)" above.
