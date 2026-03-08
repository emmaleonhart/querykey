# Secretarybird Pivot — Master TODO

## Core Behavior

The bot DMs every person on Discord individually and asks them what they think they're doing that day / over that time period with the project. It surfaces ambiguities, contradictions, and unconfirmed tasks — then asks the relevant people directly. It doesn't need to be perfectly correct. Everything it extracts shows a **confidence/certainty indicator** so people know what the AI is sure about vs guessing at.

### Anti-Policing Principle

**This tool is NOT for policing whether people know what they're supposed to be doing.** The purpose is to get everyone on the same page and have it so that everyone can look at and verify what they need. It is a coordination tool, not a surveillance tool. If someone uses it to catch people out or micromanage, they are misusing it. The design should make this kind of misuse difficult and obvious.

### Open Questions System

Each team member has a queue of **open questions** the bot is trying to get resolved. These are things the system needs clarity on — contradictions, ambiguities, unconfirmed assignments, etc.

- Questions have **urgency levels**: "needs to be known by 8 AM", "needs to be known by end of day", "needs to be known ASAP", "whenever you get to it"
- People can open the app and see all their pending questions and resolve them at their own pace
- The bot will attempt to get answers via DM on a schedule based on urgency
- Questions that get resolved (by anyone, on any platform) disappear from the queue

## Platform Targets

Primary framework: **Flutter** (single codebase).
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
- [ ] Choose local database (Hive, Isar, Drift/SQLite)

## Phase 1 — Accounts & Core Data Model

Implements the entities from `docs/data-model.md`. Uses **Apache Jena Fuseki** as the graph/triple store (already known well). Node IDs should have **human-readable aliases** — not just opaque UUIDs.

### User Accounts

People have **real accounts** on Secretarybird. This is not just a manager tool — everyone on the team uses it.

- [ ] **Account system** — users have logins on the app
  - Initial account creation via **Discord OAuth** (since Discord bot is first)
  - Future: create account from any platform the bot contacts you on (WhatsApp, Instagram, etc.)
  - Goal: low friction. If the bot DMs you somewhere, you can sign up from there.
  - People shouldn't have to go out of their way to sign up
- [ ] **Person profile** — tracks all of a person's accounts and usernames across platforms
  - Discord username, Slack ID, WhatsApp number, Instagram handle, phone, email, etc.
  - Profile is the unified view of one human across all platforms
- [ ] **Cross-platform identity resolution** — same person across Discord, Slack, phone, voice
  - Manual handle linking first
  - AI-assisted matching later
- [ ] **Task** entity — title, description, status flow (extracted → confirmed → in_progress → done / disputed), assigned_to, assigned_by, deadline, confidence, ambiguity_score, source audit trail
- [ ] **Event** entity — distinct from Task
  - Events have a **duration** (start time + end time), are time-fixed
  - Tasks have an optional **deadline**, are time-flexible
  - Events: "1 PM sync meeting" — can't move without asking someone
  - Tasks: "Refactor the query engine" — do it whenever, just finish by Friday
  - Grey area: milestones (zero-duration time markers)
  - Rule of thumb: if you can move it to tomorrow without asking permission, it's a task
- [ ] **IngestItem** entity — raw input container (bot_feed, chatlog_paste, screenshot, voice_note, recorded_audio, freeform_text)
- [ ] **Message** entity — normalized record of something said, with confidence score
- [ ] **Conflict** entity — contradictory_tasks, reassignment, deadline_change, scope_change
- [ ] **Instruction** entity — broader than Task, any directive
- [ ] **FollowUp** entity — outbound AI questions to team members
- [ ] **OpenQuestion** entity — per-person queue of things the system needs resolved
  - Urgency levels: "asap", "by [specific time]", "end of day", "whenever"
  - Visible in-app as a list the person can work through
  - Resolved by anyone on any platform → disappears from queue
- [ ] **VoiceProfile** entity — speaker embeddings for voice recognition
- [ ] **ExternalSync** entity — tracking tasks pushed to Jira/Azure DevOps/GitHub/etc.
- [ ] **Context** entity — inferred or user-provided context labels ("Monday standup", "client call", "sprint planning")
  - AI should guess contexts from content when not provided
  - Contexts help group related tasks/events/messages
- [ ] Graph relationships between all entities (see `docs/data-model.md`)

## Phase 2 — OpenClaw Integration

OpenClaw is the AI analysis engine. It does the hard work so the user doesn't have to.

- [ ] **OpenClaw API client** — connect to OpenClaw service
- [ ] **Entity extraction** — people, projects, deadlines from unstructured text
- [ ] **Task detection** — implicit and explicit assignments
  - "Get the video done by tonight" → Task(assigned_to=you, deadline=tonight)
  - "We should probably look into that" → Instruction(is_task=false, ambiguity=high)
- [ ] **Event detection** — things with fixed times
  - "Meeting at 3 PM tomorrow" → Event(start=3PM tomorrow)
  - "Demo on Friday at 2" → Event(start=Friday 2PM)
- [ ] **Context inference** — guess what context/project a message belongs to
- [ ] **Attribution** — who assigned what to whom
- [ ] **Contradiction detection** — compare new input against existing graph
- [ ] **Ambiguity scoring** — how vague is this instruction?
- [ ] **Follow-up question generation** — what to ask whom
- [ ] **Message composition** — short, secretary-style outbound messages
  - **CRITICAL: OpenClaw must not be wordy.** It's a secretary, not a consultant.
  - Good: "Are you doing the video for 10 PM or 8 AM?"
  - Bad: "Based on my analysis of your recent communications, I've identified a scheduling discrepancy..."
  - Messages are short, clear, direct. Ask a question and listen.
- [ ] **Chatlog parsing** — turn raw pasted text into structured messages
- [ ] **OCR text interpretation** — make sense of screenshot extractions

## Phase 3 — Unstructured Input Pipeline

The core differentiator. Accept anything, normalize it.

- [ ] **Freeform text input** — paste any text, AI extracts structure
  - Email forwards, meeting notes, random text, anything
  - User can optionally provide a context label ("this is from our Monday standup")
- [ ] **Chatlog paste** — paste a Discord/Slack/iMessage/WhatsApp conversation
  - AI identifies speakers, messages, timestamps
  - Handles various chatlog formats
- [ ] **Screenshot input** — OCR extracts text, AI parses as chatlog
- [ ] **Voice note** — record a memo, transcribe, extract
- [ ] **Recorded conversation** — streaming transcription + speaker diarization
- [ ] **Normalization** — all inputs → common IngestItem format before AI pipeline

## Phase 4 — In-App Messaging & Discord Bot

### App-Internal DM System

The app has its own built-in messaging system between the bot and each user. This is the **primary** channel.

- [ ] **In-app conversation view** — chat thread between you and the bot within the app
- [ ] **Unified inbox** — see all your conversations with the bot regardless of which platform the message came from (app, Discord, WhatsApp, etc.) in one threaded view
- [ ] **Multi-channel delivery** — when the bot needs to reach you, it tries the app first, then falls back to Discord, then other platforms. Tries multiple ways to contact you.
- [ ] **Response aggregation** — your reply from any platform shows up in the same conversation in the app

### Discord Bot (Top Priority External Integration)

The Discord bot is the primary external interaction surface. First integration to build.

- [ ] **Bot setup** — Discord bot application, permissions, OAuth
- [ ] **Account creation via Discord** — replying to the bot or interacting with it can create your Secretarybird account
- [ ] **Channel monitoring** — read every message in monitored channels
- [ ] **DM interaction** — bot DMs people with follow-up questions, task confirmations, contradiction alerts
  - People reply to DMs → responses recorded as follow-up answers
  - People can message the bot directly — ask questions, give instructions, report status
- [ ] **Message logging** — log ALL messages on the server
- [ ] **Hourly batch processing** — collect messages continuously, process in hourly batches by default
  - Critical events (explicit contradictions, direct DMs to bot) processed immediately
  - Keeps OpenClaw costs manageable
- [ ] **Multi-person DMs** — can DM multiple people simultaneously about different things
- [ ] **Server → knowledge graph** — parsed messages feed into the knowledge graph

### Future Messaging Platforms

- [ ] **WhatsApp** — bot DMs people via WhatsApp Business API
- [ ] **Instagram** — bot DMs people via Instagram DM API
- [ ] **Slack** — same model as Discord
- [ ] Any platform the bot messages you on → you can create an account from there

## Phase 5 — Calendar & Scheduling

- [ ] **Calendar view** in Flutter app — shows all tasks and events in the UI
- [ ] **Task vs Event distinction** in the UI
  - Tasks: shown as items with optional deadlines, can be reordered/moved freely
  - Events: shown as time blocks on the calendar, fixed in time
  - Visual distinction between the two
  - **Confidence indicators** — every extracted task/event shows how certain the AI is
- [ ] **Subscribable iCal feed** — standard .ics calendar that any calendar app (Google Calendar, Outlook, Apple Calendar) can subscribe to
  - Updates in real time as the AI extracts new events and deadlines
  - Per-person calendar feeds
- [ ] **Proactive reminders** — notifications before deadlines hit
- [ ] **Scheduled check-ins** — periodic "how's X going?" messages
- [ ] **Deadline extraction** — AI pulls dates/times from unstructured input and creates events or task deadlines
- [ ] **Context-aware scheduling** — AI infers what project/context a deadline belongs to

## Phase 6 — Conversational Agent (The Secretary)

- [ ] **Proactive daily check-ins** — DMs every person individually on Discord asking what they think they're doing that day with the project
- [ ] **Open questions queue** — each person has a visible list of things the system needs answered
  - Urgency levels determine when/how aggressively the bot asks
  - "ASAP" → bot DMs you right now
  - "By 8 AM" → bot makes sure to ask before 8 AM
  - "End of day" → included in daily check-in
  - "Whenever" → sits in your queue, bot mentions it occasionally
  - Person can open the app and resolve questions proactively without waiting for a DM
- [ ] **Follow-up engine** — detect contradictions, ambiguities, missed deadlines → generate open questions
- [ ] **Outbound messaging** — send follow-ups via in-app DM first, then Discord, WhatsApp, Instagram, Slack, SMS
- [ ] **Response handling** — record answers, update knowledge graph, resolve open questions
- [ ] **Epistemic humility** — confidence scores on everything, ask when unsure
  - ~75% correct from data alone
  - ~20% catches own mistakes via follow-up
  - ~5% genuinely wrong
- [ ] **Transparency** — all AI work is visible and auditable, nothing hidden
- [ ] **Contextual answers** — "What is John working on?" answered from knowledge graph, or ask John if unsure
- [ ] **Anti-policing guardrails** — system is for coordination, not surveillance. Design should make micromanagement misuse difficult and obvious.

## Phase 7 — Team Member Tracking

- [ ] **Person profile page** — view a person's profile showing all their linked accounts and usernames across platforms
- [ ] **Cross-platform identity linking** — link Discord username, Slack ID, WhatsApp number, Instagram handle, phone number, email to one Person
  - Same person may appear as "john_dev#1234" on Discord, "U12345" on Slack, "@john" on Instagram, and a voice in a meeting
  - Manual linking first, AI-assisted matching later
- [ ] **Preferred contact channel** — track where each person is most responsive
- [ ] **Contact cascade** — when reaching out, try app → Discord → WhatsApp → Instagram → etc.
- [ ] **Workload view** — what's assigned to each person, deadlines, status
- [ ] **Voice profile enrollment** — collect voice samples for speaker identification
  - Initial samples from onboarding or manually tagged recordings
  - Improves over time as more audio is processed

## Phase 8 — External Tool Sync

- [ ] **Jira** — bi-directional sync (tasks → Jira issues, status changes sync back)
- [ ] **Azure DevOps** — bi-directional sync (tasks → work items)
- [ ] **Git (GitHub, GitLab)** — read + link (link tasks to repos/branches/PRs)
- [ ] **Trello, Asana, etc.** — push (tasks → cards)
- [ ] Nobody manually creates tickets. AI extracts from conversation and pushes.

## Phase 9 — Audio Pipeline

- [ ] **On-device recording** (Flutter app mic)
- [ ] **Audio streaming** — WebSocket stream audio chunks to server
- [ ] **Server-side transcription** (Whisper or equivalent)
- [ ] **Speaker diarization** — identify who is speaking using VoiceProfiles
- [ ] **Voice note capture** — simpler path: record → upload → transcribe → analyze

## Phase 10 — Server & Real-time

- [ ] **Secretarybird Server** — ingestion, knowledge graph storage, real-time sync, OpenClaw coordination
- [ ] **Local mode** — server on user's machine (privacy-first)
- [ ] **Cloud mode** — always-on for distributed teams
- [ ] **Hybrid mode** — always-on with local processing option
- [ ] **WebSocket sync** — real-time graph diffs broadcast to all connected clients
- [ ] **GraphDiff format** — added_nodes, updated_nodes, added_edges, removed_edges, new_conflicts, resolved_conflicts
- [ ] **Batch processing scheduler** — hourly by default, configurable

---

## Open Questions

From `docs/architecture.md` — decisions still needed:

- [x] Graph store: **Apache Jena Fuseki** (decided — already well-known)
- [ ] Speaker diarization / voice embedding model selection
- [ ] Multi-language conversation handling
- [ ] Privacy and data retention policies (especially audio + voice embeddings)
- [ ] Rate limiting / cost management for OpenClaw on high-volume feeds
- [ ] Encrypted/private channel handling (permissions model)
- [ ] Offline mode behavior for Flutter app
- [ ] How much to port from original secretarybird repo (socket layer is priority)
- [ ] Outbound message rate limiting (don't spam people with too many follow-ups)
- [ ] AI autonomy level (auto-resolve obvious contradictions vs always ask human?)
- [ ] Voice enrollment UX (how to collect samples without friction)
- [ ] Per-person outbound channel selection (app, Discord, Slack, SMS)

---

## Notes

- **Testing**: Follow cleanvibe practices — tests for every feature, run before commit.
- **Commits**: Early and often, with clear "why" messages per CLAUDE.md.
- **OpenClaw tone**: Secretary, not consultant. Short, direct, never wordy.
- **Task vs Event**: Tasks are flexible in time (optional deadline). Events are fixed in time (start + end). If you can move it without asking permission, it's a task.
