# QueryKey — Master TODO

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
> decisions (do not relitigate): **Flutter** UI; **Rust** server
> target (current Go server deprecated, no rewrite this round);
> **model-agnostic local agent, Gemma default** (OpenClaw is today's
> bridge, an implementation detail).

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

Tasks, events, and notes are intended to live as **plain markdown
files** on your machine that you can read and edit by hand. The agent
operates *on* those files; it does not lock your life inside an app
database. **The on-disk format is an open design question** (see
`queue.md` open decisions): frontmatter vs. org-style vs. plain bullets.
Document the model before writing ingestion code; do not implement the
on-disk model this round.

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

> **Graph store is an OPEN decision.** Apache Jena Fuseki was the prior
> pick (and a Fuseki client stub exists), but given the local-first /
> single-user reorientation, an embedded triple/RDF store or just
> SQLite + application-level relations may be a better fit. Decide
> before building real persistence. (queue.md open decisions.)

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
**model-agnostic** — default **Gemma**, switchable to other local (or
optionally hosted) models. *Today's bridge is OpenClaw via a WSL
gateway under `server/internal/openclaw/`; treat it as an
implementation detail behind a model-agnostic interface, to be
superseded by the Rust rewrite. Callers must never name a model.*

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
- [ ] **Markdown reconciliation** — extracted structure round-trips to
  the on-disk markdown task model (pending that format decision)

## Phase 4 — In-App Messaging & Discord

### App-Internal DM System (primary channel)

- [ ] **In-app conversation view** — your chat thread with the agent
- [ ] **Unified inbox** — all conversations with the agent across
  platforms (app, Discord, WhatsApp, …) in one threaded view
- [ ] **Multi-channel delivery** — agent tries the app first, then
  Discord, then other platforms
- [ ] **Response aggregation** — your reply from any platform shows up
  in the same conversation

### Discord (first external ingest surface)

- [ ] **Bot setup** — Discord application, permissions, OAuth
- [ ] **Account bootstrap via Discord** — interacting with the bot can
  create/sign in your account
- [ ] **Channel monitoring** — read messages in monitored channels/DMs
- [ ] **DM interaction** — bot DMs people with follow-ups, confirmations,
  contradiction alerts; replies recorded as follow-up answers
- [ ] **Message logging** — log messages from monitored surfaces
- [ ] **Hourly batch processing** — collect continuously, process hourly
  by default; critical events (explicit contradictions, direct DMs)
  processed immediately; keeps agent cost manageable
- [ ] **Surface → knowledge graph** — parsed messages feed the graph

### Future Messaging Platforms

- [ ] **WhatsApp** (Business API), **Instagram** (DM API), **Slack**
- [ ] Any platform the bot reaches you on → you can sign in from there

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

## Phase 8 — Shared / Multi-Person Mode (OPTIONAL, deprioritized)

> Not the spine of the product. QueryKey is **not** a team-coordination
> tool you are forced to adopt. This phase exists only if the
> rationalist-social-network angle is later exposed to other people.
> Whether/how that happens (federated? local-only? selective node
> sharing?) is an unresolved **product** question — decide before any
> networking code (queue.md open decisions).

- [ ] Others can have real accounts
- [ ] Multi-person DMs (bot DMs several people about different things)
- [ ] Shared open-questions resolution across people
- [ ] Cloud / hybrid run modes for always-on multi-person use

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

> **Target language: Rust.** The current `server/` is Go and is
> **deprecated** (kept compilable until the Rust rewrite supersedes it;
> no rewrite this round). The Go OpenClaw/WSL bridge is the reference
> implementation for re-solving the local-agent bridge in Rust.

- [ ] **QueryKey Server (Rust, target)** — ingestion, knowledge graph,
  real-time sync, local-agent coordination
- [ ] **Local mode** — server on your machine (privacy-first; the default)
- [ ] **Cloud / hybrid modes** — only relevant alongside Phase 8
- [ ] **WebSocket sync** — real-time graph diffs to connected clients
- [ ] **GraphDiff format** — added/updated nodes, added/removed edges,
  new/resolved conflicts
- [ ] **Batch processing scheduler** — hourly default, configurable
- [ ] **Single-binary distribution** — trivial install is a feature,
  not a nicety (see `docs/versions-comparison.md`)

---

## Open Decisions

See `queue.md` for the canonical list. Highlights:

- [ ] **Graph store** — Fuseki (prior pick) vs. embedded store vs.
  SQLite + app relations, given local-first/single-user
- [ ] **On-disk markdown task format** — frontmatter / org-style / bullets
- [ ] **Whether/how the rationalist-social-network angle is exposed to
  other users** — federated? local-only? selective sharing? (product)
- [ ] Server language is **Rust** (resolved); Go server deprecated
- [ ] AI is a **model-agnostic local agent, Gemma default** (resolved)
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

## Notes

- **Testing**: tests for every feature; run before commit.
- **Commits**: early and often, with clear "why" messages per `CLAUDE.md`.
- **Agent tone**: secretary, not consultant. Short, direct, never wordy.
- **Task vs Event**: tasks are time-flexible (optional deadline); events
  are time-fixed (start + end). If you can move it without asking
  permission, it's a task.
- **Settled, do not relitigate**: Flutter UI; Rust server target;
  model-agnostic local agent with Gemma default.
