# Secretarybird Pivot — Master TODO

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

## Phase 1 — Core Data Model

Implements the entities from `docs/data-model.md`.

- [ ] **Person** entity — display name, handles (Discord/Slack/phone), role, preferred channel
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

## Phase 4 — Discord Bot (Top Priority Integration)

The Discord bot is the primary interaction surface for many users. First integration to build.

- [ ] **Bot setup** — Discord bot application, permissions, OAuth
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

## Phase 5 — Calendar & Scheduling

- [ ] **Calendar view** in Flutter app
- [ ] **Task vs Event distinction** in the UI
  - Tasks: shown as items with optional deadlines, can be reordered/moved freely
  - Events: shown as time blocks on the calendar, fixed in time
  - Visual distinction between the two
- [ ] **Followable calendars** — subscribable calendars generated from extracted deadlines and events
  - Team members can subscribe and see AI-extracted schedule update in real time
- [ ] **Proactive reminders** — notifications before deadlines hit
- [ ] **Scheduled check-ins** — periodic "how's X going?" messages
- [ ] **Deadline extraction** — AI pulls dates/times from unstructured input and creates events or task deadlines
- [ ] **Context-aware scheduling** — AI infers what project/context a deadline belongs to

## Phase 6 — Conversational Agent (The Secretary)

- [ ] **Follow-up engine** — detect contradictions, ambiguities, missed deadlines → generate follow-ups
- [ ] **Outbound messaging** — send follow-ups via Discord DM, app push, Slack DM, SMS
- [ ] **Response handling** — record answers, update knowledge graph, resolve conflicts
- [ ] **Epistemic humility** — confidence scores on everything, ask when unsure
  - ~75% correct from data alone
  - ~20% catches own mistakes via follow-up
  - ~5% genuinely wrong
- [ ] **Transparency** — all AI work is visible and auditable, nothing hidden
- [ ] **Contextual answers** — "What is John working on?" answered from knowledge graph, or ask John if unsure

## Phase 7 — Team Member Tracking

- [ ] **Person management UI** — add/edit team members
- [ ] **Cross-platform identity linking** — link Discord username, Slack ID, phone number, email to one Person
  - Same person may appear as "john_dev#1234" on Discord, "U12345" on Slack, and a voice in a meeting
  - Manual linking first, AI-assisted matching later
- [ ] **Preferred contact channel** — track where each person is most responsive
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

- [ ] Graph database choice (Neo4j vs PostgreSQL vs hybrid)
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
