# QueryKey — Master TODO

## Current status — START HERE (2026-05-15)

**Working today (Rounds 1–5, all on `main`):**
- **Server:** Rust (`server/`, crate `querykey-server`) — the *only*
  server; the Go implementation was fully ported then deleted
  (recoverable from git history). Builds clean & warning-free in all
  three configs: `cargo build`, `--features loca`, `--features
  discord`.
- **Canonical store:** the **markdown vault** (`server/src/vault/`,
  `$VAULT_DIR`, default `./vault`) — Person/Task/Event as YAML
  frontmatter + body, lossless round-trip (unit-tested), survives
  restarts. This is the store of record.
- **Derived graph:** Loca/SutraDB, rebuilt from the vault on startup;
  SPARQL query bridge + typed read-backs. **Fuseki is gone.**
- **Agent:** model-agnostic — the agent is *whoever operates
  QueryKey* (**Claude/Claude Code is first-class now**; Gemma is the
  not-yet-built GUI default; Hermes/GPT optional). MCP endpoint
  (`/mcp`); OpenClaw WSL gateway is one optional backend. **Honest
  about availability (R13):** `detect()` verifies the real chat API
  (not just `/health`); ingest surfaces `agent_error`, never a silent
  empty. Ingest: relaxed parse → vault → graph + typed GraphDiff over
  the WebSocket hub.
- **Specs/context:** `docs/markdown-schema.md` (as-built),
  `docs/card-format.md`, vision corpus in `chat/public/`.

**Next big pieces:**
1. Calendar structure — DONE (Round 11).
2. Agent-drafted card↔graph — DONE (Round 12): PRM digest + agents.md
   envelope → `POST /api/card/draft`, heuristic fallback, approve via
   `PUT /api/card`.
3. **P2P card layer** — format + local layer DONE (Round 7).
   **Open/parked: the P2P transport** (what moves a card between
   peers) + discovery — the gating *design* question; do not barrel
   on a guess (explicit user steering). This is now the main
   remaining social unknown; PRM side is broadly built out.

(Done: Conflict/OpenQuestion/FollowUp forms — R6. Semantic
`[[wikilink]]`/`[[property:target]]` — R8. Status-workflow
enforcement — R9. Instruction/VoiceProfile forms — R10: the full
canonical entity set is on disk. Calendar — R11. Agent-drafted
card — R12. Agent-honesty fix [detect capability + no silent-empty
ingest] — R13.)

**Deprioritized (back of the list, per user 2026-05-16):** the audio
pipeline (transcription/diarization). Would later fill VoiceProfile
embeddings, but it is design-heavy (model selection unresolved) and
explicitly *not* near-term. See the audio items further down.

**How to work here:** [`queue.md`](queue.md) is the barrel-through
queue — do work from there. This `todo.md` is the long roadmap
(reference, not a worklist). **Discord is deprioritized** → Phase Z.
**Flutter** is the frontend (firm).

---

> **Status of this file.** QueryKey is a **rationalist social network**
> that doubles as a local-first PRM / lightweight CRM / JIRA-style task
> tracker, for one person, run from your own desktop. The phase
> skeleton below is preserved from an earlier prototype's roadmap, but
> the framing is now **personal-first**: a single owner and the people
> in *their* life. Multi-person / shared ("team") behavior is **not the
> spine** of the product — it is an explicitly optional, deprioritized
> later phase (Phase 8). Anything that reads as "the team" means "the
> people in your contact graph."
>
> Authoritative near-term plan: [`queue.md`](queue.md). Settled
> decisions (do not relitigate): **Flutter** UI; **Rust** is the
> server (Go fully removed in Round 4); **model-agnostic local agent,
> Gemma default** via MCP (OpenClaw is today's bridge, an
> implementation detail).

## Core Behavior

QueryKey watches the unstructured streams of how you actually
communicate — chat logs, pasted conversations, screenshots, voice
notes — and uses a local AI agent to build a private model of the
people and commitments in your life. It surfaces ambiguities,
contradictions, and unconfirmed commitments — then asks *you* (or,
where appropriate, the relevant person) to clarify. It doesn't need to
be perfectly correct. Everything it extracts shows a
**confidence/certainty indicator** so you know what the agent is sure
about vs. guessing at.

**Why "QueryKey":** Q / K / V of a transformer attention matrix — your
life is the *values*, the agent computes *queries* against *keys* built
from your notes and logs. See `README.md`.

### We Serve You — The Mission

QueryKey exists to serve you and conform to your workflow. The tool
adapts to how you already work; you never reformat your life to fit a
form. The purpose is to help you keep up with the people in your life —
care and coordination, not surveillance. The privacy that matters is
not just yours: it's the privacy of the people you talk about too.

### Your Data Is Markdown On Your Disk

Markdown files on your machine, tracked in a **git repository**, are
the **canonical source of truth** — you can read and edit them by
hand. The knowledge graph is **derived from them**, never the other
way round. Working format: **YAML frontmatter** for structured fields
(person, date, tags, status) + freeform markdown body — the Obsidian
convention, so files are useful even without QueryKey installed.
**Implemented (Round 5):** `server/src/vault/` for Person/Task/Event;
the graph is derived and rebuilt from the vault on startup. See
`docs/markdown-schema.md` (as-built) and `queue.md` Round 5.

### Open Questions System

You have a queue of **open questions** the system is trying to get
resolved — contradictions, ambiguities, unconfirmed commitments.

- Questions have **urgency levels**: "needs to be known by 8 AM", "by
  end of day", "ASAP", "whenever you get to it"
- Open the app and see all pending questions; resolve them at your pace
- QueryKey attempts to get answers via DM (in-app, then Discord, etc.)
  on a schedule based on urgency
- Questions resolved anywhere, on any platform, disappear from the queue

## Platform Targets

Primary framework: **Flutter** (single codebase, locked in).
Current focus: **Windows Desktop**.

| Platform | Priority | Status |
|---|---|---|
| Windows Desktop | **NOW** | Not started |
| Browser (Web) | Later | Not started |
| macOS Desktop | Later | Not started |
| Linux Desktop | Later | Not started |
| iPhone (iOS) | Later | Not started |
| Android | Later | Not started |

---

## Phase 0 — Project Scaffolding

- [ ] Initialize Flutter project (`flutter create`)
- [ ] Set up project structure (feature-based folder layout)
- [ ] Configure Windows desktop target
- [ ] Set up testing infrastructure (unit, widget, integration)
- [ ] Set up CI (GitHub Actions — lint, test, build)
- [ ] Choose and configure state management (Riverpod, Bloc, etc.)
- [ ] Choose local store (see Phase 1 — graph store is an open decision)

## Phase 1 — Account & Core Data Model

Implements the entities from `docs/data-model.md`. Node IDs should have
**human-readable aliases** — not just opaque UUIDs.

> **Graph store is DECIDED: Loca (formerly SutraDB).** The author's
> own embedded **Rust** graph-vector-time DB (separate project). The
> graph is **derived from the markdown** (RDF/graph generated out of
> the files), not the store of record — it must be rebuildable from
> the files. The time dimension matters (relationship history).
> **Fuseki is NOT used**; the Go `server/internal/graph/` Fuseki stub
> is stale pre-pivot scaffolding slated for removal — do not build on
> it.

### Account

Personal mode has **one account: yours.** It is a real account, not an
admin row. (Multi-account / others-have-logins is Phase 8, optional.)

- [ ] **Single-owner account** — your login on the app
  - Convenience bootstrap via **Discord OAuth** (Discord is the first
    ingest surface)
  - Low friction: if the bot reaches you somewhere, you can sign in there
- [ ] **Person profile** — tracks one person's accounts/usernames across
  platforms (Discord, Slack, WhatsApp, Instagram, phone, email, …);
  the unified view of one human
- [ ] **Cross-platform identity resolution** — same person across
  Discord, Slack, phone, voice. Manual handle linking first; AI-assisted
  matching later
- [ ] **Task** entity — title, description, status flow (extracted →
  confirmed → in_progress → done / disputed), related_person,
  origin, deadline, confidence, ambiguity_score, source audit trail
- [ ] **Event** entity — distinct from Task
  - Events have a **duration** (start + end), are time-fixed
  - Tasks have an optional **deadline**, are time-flexible
  - Rule of thumb: if you can move it to tomorrow without asking
    anyone's permission, it's a task
  - Grey area: milestones (zero-duration time markers)
- [ ] **IngestItem** entity — raw input container (bot_feed,
  chatlog_paste, screenshot, voice_note, recorded_audio, freeform_text)
- [ ] **Message** entity — normalized record of something said, with
  confidence score
- [ ] **Conflict** entity — contradictory_tasks, reassignment,
  deadline_change, scope_change
- [ ] **Instruction** entity — broader than Task, any directive
- [ ] **FollowUp** entity — outbound agent questions
- [ ] **OpenQuestion** entity — your queue of things to resolve
  - Urgency: "asap", "by [time]", "end of day", "whenever"
  - Resolved anywhere → disappears from the queue
- [ ] **VoiceProfile** entity — speaker embeddings for voice recognition
- [ ] **ExternalSync** entity — tasks pushed to Jira/Azure DevOps/GitHub
- [ ] **Context** entity — inferred or user-provided context labels
  ("client call", "sprint planning"); agent guesses contexts from content
- [ ] Graph relationships between all entities (see `docs/data-model.md`)

## Phase 2 — Local AI Agent Integration

The local AI agent does the hard work so you don't have to. It is
**model-agnostic** — default **Gemma** (cheap, private, local),
switchable to Claude/GPT for power users. *Today's bridge is OpenClaw
via a WSL gateway under `server/internal/openclaw/`; treat it as an
implementation detail behind a model-agnostic interface, to be
superseded by the Rust rewrite. Callers must never name a model.*

- [ ] **MCP server — present from day one.** QueryKey exposes itself
  as an MCP server so *any* agent can attend over the graph and act on
  the markdown files. This is infrastructure, not a late feature; it
  is what makes the model-agnostic story real and frames QueryKey as a
  platform, not a single-model app.
- [ ] **Agent client** — model-agnostic interface to the local agent
- [ ] **Entity extraction** — people, projects, deadlines from text
- [ ] **Task detection** — implicit and explicit
  - "Get the video done by tonight" → Task(deadline=tonight)
  - "We should probably look into that" → Instruction(is_task=false,
    ambiguity=high)
- [ ] **Event detection** — fixed-time things ("Meeting at 3 PM tomorrow")
- [ ] **Context inference** — guess the context/project for a message
- [ ] **Attribution** — who said / asked what
- [ ] **Contradiction detection** — compare new input vs. existing graph
- [ ] **Ambiguity scoring** — how vague is this instruction?
- [ ] **Follow-up question generation** — what to ask, of whom
- [ ] **Message composition** — short, secretary-style outbound messages
  - **CRITICAL: the agent must not be wordy.** Secretary, not consultant.
  - Good: "Are you doing the video for 10 PM or 8 AM?"
  - Bad: "Based on my analysis of your recent communications, I've
    identified a scheduling discrepancy…"
- [ ] **Chatlog parsing** — raw pasted text → structured messages
- [ ] **OCR text interpretation** — make sense of screenshot extractions

## Phase 3 — Unstructured Input Pipeline

The core differentiator. Accept anything, normalize it.

- [ ] **Freeform text input** — paste any text, agent extracts structure
  (email forwards, meeting notes, anything; optional context label)
- [ ] **Chatlog paste** — Discord/Slack/iMessage/WhatsApp conversation;
  agent identifies speakers, messages, timestamps; handles varied formats
- [ ] **Screenshot input** — OCR extracts text, agent parses as chatlog
- [ ] **Voice note** — record a memo, transcribe, extract
- [ ] **Recorded conversation** — streaming transcription + diarization
- [ ] **Normalization** — all inputs → common IngestItem format
- [x] **Markdown reconciliation** — extracted structure is written to
  the canonical vault then projected to the graph (Round 5,
  `server/src/vault/`); round-trip is lossless (unit-tested).

## Phase 4 — In-App Messaging

### App-Internal DM System (primary channel)

- [ ] **In-app conversation view** — your chat thread with the agent
- [ ] **Unified inbox** — all conversations with the agent across
  platforms in one threaded view
- [ ] **Multi-channel delivery** — agent tries the app first, then
  other platforms
- [ ] **Response aggregation** — your reply from any platform shows up
  in the same conversation

> **Discord ingest moved to the back** — see *Phase Z* below. It is
> genuinely useful but explicitly deprioritized (the user is unsure how
> much they'll use it). It is **not** the "first external ingest
> surface" anymore. Other external messaging platforms (WhatsApp,
> Instagram, Slack) ride behind Discord.

## Phase 5 — Calendar & Scheduling

- [ ] **Calendar view** in the Flutter app — all tasks and events
- [ ] **Task vs Event distinction** in the UI
  - Tasks: items with optional deadlines, reorderable
  - Events: fixed time blocks
  - **Confidence indicators** on every extracted task/event
- [ ] **Subscribable iCal feed** — standard `.ics` any calendar app can
  subscribe to; updates in real time as the agent extracts events
- [ ] **Proactive reminders** — notifications before deadlines
- [ ] **Scheduled check-ins** — periodic "how's X going?" prompts
- [ ] **Deadline extraction** — dates/times from unstructured input
- [ ] **Context-aware scheduling** — infer the project/context

## Phase 6 — Conversational Agent (The Secretary)

- [ ] **Proactive daily check-in** — a single daily prompt to *you*
  about what you're working on / who you owe a reply (not blasting a
  team; that's Phase 8)
- [ ] **Open questions queue** — your visible list; urgency drives how
  aggressively the agent asks
  - "ASAP" → asks now; "By 8 AM" → before 8 AM; "End of day" → in the
    daily check-in; "Whenever" → sits in the queue
- [ ] **Follow-up engine** — contradictions, ambiguities, missed
  deadlines → open questions
- [ ] **Outbound messaging** — in-app DM first, then Discord, WhatsApp,
  Instagram, Slack, SMS
- [ ] **Response handling** — record answers, update graph, resolve
  open questions
- [ ] **Epistemic humility** — confidence scores on everything; ask
  when unsure (~75% from data, ~20% caught via follow-up, ~5% wrong)
- [ ] **Transparency** — all agent work visible and auditable
- [ ] **Contextual answers** — "What is X working on?" from the graph,
  or ask X if unsure
- [ ] **Anti-surveillance guardrails** — this is for care and
  coordination, not policing; make misuse hard and obvious

## Phase 7 — People Tracking

- [ ] **Person profile page** — all of a person's linked accounts
  across platforms
- [ ] **Cross-platform identity linking** — Discord/Slack/WhatsApp/
  Instagram/phone/email → one Person (manual first, AI-assisted later)
- [ ] **Preferred contact channel** — where each person is responsive
- [ ] **Contact cascade** — app → Discord → WhatsApp → Instagram → …
- [ ] **Commitment view** — what you owe each person / they owe you,
  deadlines, status
- [ ] **Voice profile enrollment** — voice samples for speaker ID;
  improves as more audio is processed

## Phase 8 — Peer-to-Peer Card Layer (after the solo PRM)

> **Built second, on purpose.** The private PRM (Phases 1–7) is useful
> with zero other users and *builds the graph the cards are a window
> into*. The card layer is what makes using a relationship tracker
> socially legitimate — it changes the meaning of the tool, it is not
> just a feature. Not a team-coordination tool you are forced into.

The **card**: each user broadcasts one markdown file — what they're
**offering** (their *key*) and **looking for** (their *query*). It is a
*selective window into the private graph you already built*, not a
separate publishing chore.

- [ ] **Card format spec first** — human-readable markdown,
  machine-parseable, expressive enough for offer/looking-for. Spec it
  in the docs before any exchange code; it ossifies fast once used.
- [ ] **Own card is git-tracked** — full local history enables
  revert/undo.
- [ ] **Others' cards are git-ignored on your machine** — usable in
  the moment, never archived. Intentional asymmetry; no surveillance.
- [ ] **24-hour propagation delay** — privacy safety valve (late-night
  mistake fixable by morning, unseen). Revert before propagation is
  immediate, no delay.
- [ ] **Pure peer-to-peer** — no central server, no global source of
  truth. Soft, non-cryptographic guarantee; community-appropriate.
- [ ] **GitHub bootstraps identity + discovery** — usernames as
  handles, follow-on-GitHub as the find mechanism, behind a swappable
  handle abstraction ("a user is a canonical handle currently resolved
  via GitHub"; later: DIDs/Nostr).
- [ ] **Private vs. public card** — planned, explicitly *not now*
  (more complex; revisit after the single-card model works).
- [ ] Others can have real accounts; multi-person open-questions;
  cloud/hybrid run modes — only as the network warrants.

## Phase 9 — External Tool Sync

- [ ] **Jira** — bi-directional (tasks → issues, status syncs back)
- [ ] **Azure DevOps** — bi-directional (tasks → work items)
- [ ] **Git (GitHub, GitLab)** — read + link tasks to repos/branches/PRs
- [ ] **Trello, Asana, etc.** — push (tasks → cards)
- [ ] You don't manually create tickets — the agent extracts and pushes
- [ ] Is external sync still desired, and in what tier? (open question)

## Phase 10 — Audio Pipeline

- [ ] **On-device recording** (Flutter app mic)
- [ ] **Audio streaming** — WebSocket audio chunks to the server
- [ ] **Server-side transcription** (Whisper or equivalent)
- [ ] **Speaker diarization** — who is speaking, via VoiceProfiles
- [ ] **Voice note capture** — record → upload → transcribe → analyze
- [ ] Voice-profile / diarization model selection (open decision)

## Phase 11 — Server & Real-time

> **Server is Rust — no more Go.** `server/` is the `querykey-server`
> Rust crate (compiles + runs). The Go implementation was fully ported
> (Round 4) then **deleted**; recoverable from git history if needed.
> Builds: `cargo build` (in-memory), `--features loca` (Loca graph
> store), `--features discord` (serenity bot). All three clean.

- [x] **QueryKey Server (Rust)** — crate scaffolded, compiles, runs;
  axum HTTP API + WebSocket + agent bridge, graceful shutdown.
- [x] **Derived graph on Loca/SutraDB** — `loka-core` wired behind
  `--features loca`; person/task/message/conflict persist as triples;
  in-memory fallback default.
- [x] **Incremental agent streaming** — real SSE delta parsing in
  `src/openclaw/bridge.rs` (port of bridge.go ChatStream); `ws.rs`
  broadcasts each delta live as `stream_chunk`.
- [x] **Persistent SPARQL query bridge** — `query()` snapshots the
  PersistentStore (`iter()` + `load_terms_into`, id-consistent) into a
  TripleStore+TermDictionary and runs `loka_sparql::execute`.
  Smoke-verified: stored person → SPARQL returned its triples.
  (`TODO(perf)`: cache/incrementally maintain the snapshot.)
- [x] **Typed graph read-back (persons)** — `get_all_persons` rebuilt
  from the POS index (find_by_predicate_object + find_by_subject);
  smoke-verified. Tasks/conflicts intentionally left as markdown-read
  TODOs (the derived graph is lossy; markdown is the store of record).
- [x] **Discord bot port** — feature-gated `serenity` client
  (`--features discord`, default OFF so the default build never pulls
  it): connects, logs on ready, receives + logs human messages.
  Compiles clean. `TODO(port)`: per-channel filters, hourly batch into
  the ingest pipeline, outbound DM follow-ups (bot.go).
- [x] **MCP endpoint** — minimal JSON-RPC-over-HTTP at `POST /mcp`
  (`initialize` / `tools/list` / `tools/call`), tools: query_graph,
  list_persons, server_health. Dependency-free; smoke-verified.
  `TODO`: stdio+SSE transports, agents.md-governed write tools.
- [ ] **Local mode** — server on your machine (privacy-first; default)
- [ ] **Cloud / hybrid modes** — only relevant alongside Phase 8
- [ ] **WebSocket sync** — typed graph-diff broadcast (fan-out works)
- [ ] **GraphDiff format** — added/updated nodes, added/removed edges,
  new/resolved conflicts
- [ ] **Batch processing scheduler** — hourly default, configurable
- [ ] **Single-binary distribution** — trivial install is a feature,
  not a nicety (see `docs/versions-comparison.md`)

---

## Open Decisions

See `queue.md` for the canonical list. Highlights:

- [x] **Graph store** — RESOLVED: **Loca/SutraDB** (author's Rust
  graph-vector-time DB), graph **derived from markdown**. Fuseki not
  used; stub slated for removal.
- [x] **On-disk markdown format** — RESOLVED & **IMPLEMENTED**
  (Round 5): YAML frontmatter + freeform body, Obsidian-compatible,
  lossless round-trip. `server/src/vault/`; `docs/markdown-schema.md`
  has the as-built notes.
- [x] **How the social angle is exposed** — RESOLVED: pure **P2P card**
  exchange, asymmetric git-tracking, 24h delay, GitHub-bootstrapped
  identity. Built *after* the solo PRM. (Private-vs-public card still
  deferred.)
- [x] Server language is **Rust** (resolved); Go server deprecated
- [x] AI is **model-agnostic via MCP, Gemma default** (resolved)
- [ ] Card format spec (the high-leverage remaining design question)
- [ ] Speaker diarization / voice embedding model selection
- [ ] Multi-language conversation handling
- [ ] Privacy and data retention (especially audio + voice embeddings)
- [ ] Rate/cost management for the agent on high-volume feeds
- [ ] Encrypted/private channel handling (permissions model)
- [ ] Offline mode behavior for the Flutter app
- [ ] Outbound message rate limiting (don't over-DM people)
- [ ] Agent autonomy level (auto-resolve obvious contradictions vs ask?)
- [ ] Voice enrollment UX (collect samples without friction)
- [ ] Per-person outbound channel selection

---

## Phase Z — Discord ingest (DEPRIORITIZED — back of the roadmap)

> **User decision (2026-05-15):** Discord is very useful but the user
> is **not sure how much they'll use it**, so it sits at the back of
> the roadmap. The feature-gated `serenity` skeleton already exists
> (`server/src/discord.rs`, `--features discord`, default OFF) and is
> intentionally left as a connect-and-log skeleton. **Do not barrel
> this.** Pull from here only after the core Rust server work in
> `queue.md` is done. Other external messaging platforms come after.

- [ ] **Bot setup** — Discord application, permissions, OAuth
- [ ] **Account bootstrap via Discord** — interacting with the bot can
  create/sign in your account
- [ ] **Channel monitoring** — read messages in monitored channels/DMs
- [ ] **DM interaction** — bot DMs people with follow-ups, confirmations,
  contradiction alerts; replies recorded as follow-up answers
- [ ] **Message logging** — log messages from monitored surfaces
- [ ] **Hourly batch processing** — collect continuously, process hourly
  by default; critical events processed immediately
- [ ] **Surface → knowledge graph** — parsed messages feed the graph
- [ ] **Future messaging platforms** — WhatsApp (Business API),
  Instagram (DM API), Slack; any platform the bot reaches you on →
  you can sign in from there

## Notes

- **Testing**: tests for every feature; run before commit.
- **Commits**: early and often, with clear "why" messages per `CLAUDE.md`.
- **Agent tone**: secretary, not consultant. Short, direct, never wordy.
- **Task vs Event**: tasks are time-flexible (optional deadline); events
  are time-fixed (start + end). If you can move it without asking
  permission, it's a task.
- **Settled, do not relitigate**: Flutter UI; Rust server target;
  model-agnostic local agent with Gemma default.
