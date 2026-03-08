# Architecture Plan

## Design Philosophy

**The secretarybird is here to serve you. You don't serve it.**

Small businesses communicate in messy, informal ways — and that's fine. Secretarybird accepts any form of input and makes sense of it. A pasted screenshot of a Discord conversation is just as valid as a bot monitoring that channel in real time. A voice note saying "hey, John told me to redo the invoices" is just as valid as a recorded meeting transcript.

The system never asks users to change how they communicate. It meets them where they are.

### Anti-Performative Work

Jira, Git, Azure DevOps — these tools often become performative work. People spend more time updating tickets and writing status reports than doing the actual work. Secretarybird actively defies this pattern. Nobody should have to manually file what their boss told them to do. The AI handles it.

### "AI Writes the Code" — Not No-Code

This is not a no-code platform with drag-and-drop workflow builders. The intelligence lives in the AI. OpenClaw does the hard work: parsing unstructured input, resolving entities across platforms, detecting contradictions, generating structured output. The system is smart, not simple.

### External Tool Integration

Secretarybird connects to existing project management tools rather than replacing them:

- **Git** — link extracted tasks to repos, branches, commits
- **Jira** — sync tasks into Jira projects automatically
- **Azure DevOps** — sync tasks into boards and work items
- **Other PM tools** — extensible integration layer

Teams that need formal tracking in these systems get it for free — populated by AI from real conversations, not by humans doing data entry.

## Three Deployable Components

### 1. Secretarybird Server

The backend. Runs locally on a user's machine or deployed to the cloud. Handles:

- Ingesting and normalizing all input (text, audio, images)
- Storing the knowledge graph
- Real-time WebSocket sync with all connected clients
- Coordinating with OpenClaw for AI analysis
- Outbound messaging (dispatching follow-ups to people via Discord DM, app push, etc.)
- Batch processing on a schedule (hourly by default)

The server is where the knowledge graph lives. Clients are thin — they send data in and render the graph out.

**Local mode**: Server runs on the user's machine. Good for privacy-sensitive environments, solo use, or small teams that want to keep everything on-premise.

**Cloud mode**: Server runs on cloud infrastructure. Good for distributed teams, always-on monitoring (Discord/Slack bots), and mobile access from anywhere.

### 2. Discord Bot (Top Priority)

The Discord bot is the primary interaction surface for many users. It is the first integration to build.

**What it does:**
- Sits on a server and reads every message in monitored channels
- DMs people directly with follow-up questions, task confirmations, contradiction alerts
- Can DM multiple people simultaneously about different things
- Receives replies to its DMs and records them as follow-up responses
- People can also message *it* — ask it questions, give it instructions, report status

**Processing model**: Not everything needs real-time processing. The bot collects messages continuously but processes them in **hourly batches** by default. This is sufficient for most teams and keeps OpenClaw costs manageable. Critical events (explicit contradictions, direct messages to the bot) can be processed immediately.

**Why Discord first**: A large number of small teams and communities already live on Discord. The bot meets them where they are. No app install required for basic interaction — the bot DMs you, you reply. That's it.

### 3. Secretarybird Mobile (Flutter App)

Single Flutter codebase targeting:
- **iOS** and **Android** (phone/tablet)
- **Desktop** (macOS, Windows, Linux)
- **Web**

All platforms share the same core UI. Mobile gets additional capabilities:
- Microphone recording (meetings, conversations)
- Voice note capture
- Push notifications
- Background audio streaming

The Flutter app provides richer features than Discord alone (task boards, knowledge graph visualization, audio recording, calendars), but the Discord bot is sufficient for basic interaction.

## Unstructured Input Pipeline

The core differentiator. The ingest service accepts anything and normalizes it.

### Input Types

| Input | Processing | Notes |
|---|---|---|
| **Discord bot** | Reads all channel messages, DMs people, receives replies | Top priority. Hourly batch processing, immediate for DMs to the bot |
| **Slack bot** | Same as Discord bot but for Slack workspaces | Second priority after Discord |
| **Pasted chatlog** | AI parses conversation format, extracts speakers + messages | Handles Discord, Slack, iMessage, WhatsApp, etc. formats |
| **Screenshot** | OCR extracts text → AI parses as chatlog | Works for any messaging app screenshot |
| **Voice note** | Transcription → AI processes as text | User records a note about something they were told |
| **Recorded conversation** | Streaming transcription + speaker diarization | Phone app streams audio chunks to server |
| **Pasted freeform text** | AI extracts whatever structure exists | Email forwards, meeting notes, anything |

### Normalization

All inputs get normalized into a common internal format before hitting the AI pipeline:

```
IngestItem {
  id: UUID
  input_type: "bot_feed" | "chatlog_paste" | "screenshot" | "voice_note" | "recorded_audio" | "freeform_text"
  raw_content: bytes | string       // the original input
  extracted_messages: [              // AI-extracted after processing
    {
      speaker: string               // best-effort speaker identification
      content: string               // what was said
      timestamp: timestamp | null   // if determinable
    }
  ]
  submitted_by: User.id             // who submitted this input
  submitted_at: timestamp
  source_context: string            // user-provided context ("this is from our Monday standup")
}
```

The key insight: bot feeds produce `extracted_messages` directly (structured input). Everything else goes through AI extraction first (unstructured → structured). But once normalized, the downstream pipeline treats them identically.

## OpenClaw Integration

OpenClaw is the AI analysis engine. It is separate from the Secretarybird Server. The server sends normalized data to OpenClaw and receives structured analysis back.

**What OpenClaw does:**
- Entity extraction (people, projects, deadlines)
- Task detection (implicit and explicit assignments)
- Attribution (who assigned what to whom)
- Contradiction detection (compare against existing graph state)
- Ambiguity scoring (how vague is this instruction)
- Chatlog parsing (turn raw pasted text into structured messages)
- OCR text interpretation (make sense of screenshot extractions)
- **Generating follow-up questions** — deciding what to ask whom based on detected ambiguities and contradictions
- **Composing outbound messages** — short, clear, secretary-style messages to team members

**What OpenClaw does NOT do:**
- Store the knowledge graph (that's the Secretarybird Server)
- Handle real-time sync (that's WebSocket on the server)
- Manage users or permissions (that's the server)
- Deliver messages (that's the server's outbound messaging service)

## Conversational Agent (The Secretary)

This is the core differentiator. Secretarybird is not a passive dashboard — it actively talks to people.

### Why

The best way to know what someone thinks they're supposed to do is to ask them. No amount of passive monitoring replaces a direct question. A good secretary doesn't just file papers — they follow up, clarify, and make sure everyone is on the same page.

### Epistemic Humility

The AI is designed to be epistemically humble. It does not pretend to know things it doesn't. When extraction confidence is low, it asks rather than guesses.

Expected accuracy from data feeds alone: **~75%**. The remaining ~25% breaks down as:
- **~20%**: Wrong, but the system detects its own uncertainty and asks a clarifying question → gets to the right answer
- **~5%**: Genuinely wrong, and doesn't catch it

This is viable *because* the system can ask. A passive tool at 75% accuracy is unreliable. An active agent at 75% accuracy that catches most of its own mistakes through follow-up questions is a useful secretary.

The key design constraint: the system must always surface its work transparently. Every extraction, every task, every conflict resolution is visible and auditable. Followable calendars, conversation logs, task boards — nothing is hidden. When the AI is wrong, people can see it and correct it.

### How It Works

When the AI detects something that needs human input — a contradiction, an ambiguity, a missed deadline, a scope change — it generates a follow-up action:

```
FollowUp {
  trigger: Conflict | Task | Instruction    // what triggered this
  target: Person.id                          // who to ask
  question: string                           // short, clear question
  context: string                            // brief background for the recipient
  channel: "app_notification" | "discord" | "slack" | "sms"
  status: "pending" | "sent" | "answered" | "expired"
}
```

Example flow:
1. Boss says in a meeting: "Get the video done by 10 PM tonight"
2. Later, a Slack message from another manager says: "Video needs to be ready by 8 AM tomorrow"
3. System detects the contradiction
4. System messages the assignee: "You were asked to finish the video by 10 PM tonight (from the standup meeting) but also by 8 AM tomorrow (from Sarah on Slack). Which one are you going for?"
5. Person replies: "8 AM tomorrow, Sarah updated the deadline"
6. System records the resolution, updates the task, logs everything

### Message Style

The AI sends **short, simple messages**. It is not trying to explain or impress. It asks a question and listens.

Good: "Are you doing the video for 10 PM or 8 AM?"
Bad: "Based on my analysis of your recent communications, I've identified a scheduling discrepancy regarding the video deliverable timeline..."

The AI acts like a secretary, not a consultant.

### What Triggers Follow-ups

- **Contradictions**: Two conflicting instructions about the same task
- **Ambiguity**: A task was assigned but the scope is unclear
- **Scope changes**: The definition of a task shifted between conversations
- **Missed deadlines**: A deadline passed with no status update
- **Unconfirmed tasks**: AI extracted a task but nobody has acknowledged it
- **Reassignments**: A task silently moved from one person to another

### Contextual Understanding

The system can message different people on the team with different context. It maintains clear logs of every conversation. If person A asks "what's John working on?", the system can answer based on what it knows — and if it's not sure, it can ask John directly and relay the answer.

## Notification & Scheduling

Secretarybird sends proactive notifications based on:

- **Extracted deadlines** — reminders before a deadline hits
- **Scheduled check-ins** — periodic "how's X going?" messages
- **Contradiction alerts** — immediate notification when conflicting instructions are detected
- **Task assignments** — notify someone when they've been assigned something (with the source)
- **Resolution requests** — ask for clarification on ambiguous instructions

Notifications go through the Flutter app (push notifications), or through integrated channels (Discord DM, Slack DM) depending on where the person is most reachable.

### Followable Calendars

The system generates calendars from extracted deadlines, assignments, and scheduling information. These are followable — team members can subscribe and see their AI-extracted schedule update in real time. This is another surface where the AI's work is transparent and auditable.

## Voice Learning

The system learns team members' voices over time. This replaces manual speaker tagging for recorded conversations.

**Enrollment**: Initial voice samples collected during onboarding or from early recordings with manual tagging.

**Recognition**: As the system processes more audio, it builds speaker embeddings and improves identification accuracy. Eventually, recording a meeting produces fully attributed transcripts automatically.

**Privacy**: Voice embeddings are stored on the server (local or cloud, depending on deployment). Raw audio can be discarded after transcription if desired.

## External Tool Sync

Secretarybird can push extracted tasks into existing project management systems. This is a one-way or two-way sync depending on the tool:

| Tool | Sync Direction | What Syncs |
|---|---|---|
| **Jira** | Bi-directional | Tasks → Jira issues; Jira status changes → task updates |
| **Azure DevOps** | Bi-directional | Tasks → work items; status changes sync back |
| **Git** (GitHub, GitLab, etc.) | Read + link | Link tasks to repos/branches/PRs; read commit activity |
| **Trello, Asana, etc.** | Push | Tasks → cards/tasks in those systems |

The key: nobody manually creates tickets. Secretarybird extracts them from conversation and pushes them into whatever system the team already uses. If the team doesn't use any formal tool, the built-in task board is sufficient.

## Real-time Sync

WebSocket connections between Secretarybird Server and all connected Flutter clients.

Flow:
1. New input arrives (bot message, pasted text, audio chunk, etc.)
2. Server normalizes the input
3. Server sends to OpenClaw for analysis
4. OpenClaw returns extracted entities, tasks, contradictions
5. Server updates the knowledge graph
6. Server computes a graph diff
7. Diff is broadcast to all connected clients via WebSocket
8. Clients apply the diff to their local graph view

For audio streaming, the flow is continuous — audio chunks stream in, transcription happens incrementally, and the graph updates as new content is recognized.

## Audio Pipeline

```
Phone mic
    │
    ▼
Local audio buffer (Flutter app)
    │
    ▼ WebSocket stream (audio chunks)
    │
Secretarybird Server
    │
    ├─→ Transcription service (Whisper or equivalent)
    │       │
    │       ▼
    │   Raw transcript with speaker segments
    │       │
    │       ▼
    ├─→ OpenClaw analysis
    │       │
    │       ▼
    │   Extracted tasks, entities, contradictions
    │       │
    │       ▼
    └─→ Knowledge graph update → WebSocket broadcast
```

Voice notes follow a simpler path: record → upload → transcribe → analyze. No streaming needed.

**Speaker diarization**: For recorded conversations, the system identifies who is speaking using learned voice profiles (see Voice Learning above). Initial recordings may need manual tagging, but the system improves over time as it builds speaker embeddings for each team member.

## Deployment Options

| Mode | Server Location | Bot Monitoring | Best For |
|---|---|---|---|
| **Local** | User's machine | Only while running | Solo users, privacy-first, small teams |
| **Cloud** | Cloud VM/container | Always-on | Distributed teams, 24/7 Discord/Slack monitoring |
| **Hybrid** | Cloud server, local processing | Always-on | Teams that want always-on but sensitive data stays local |

## Open Questions

- Exact graph database choice (Neo4j vs PostgreSQL vs hybrid)
- Speaker diarization / voice embedding model selection
- How to handle multi-language conversations
- Privacy and data retention policies (especially for audio and voice embeddings)
- Rate limiting / cost management for OpenClaw calls on high-volume feeds
- How to handle encrypted or private channels (permissions model)
- Offline mode behavior for the Flutter app
- How much history to port from the original secretarybird repo (socket layer is priority)
- Outbound message rate limiting (don't spam people with too many follow-ups)
- How much autonomy should the AI have before asking a human admin? (e.g., auto-resolving obvious contradictions vs. always asking)
- Voice enrollment UX — how to collect initial voice samples without friction
- Which channels to use for outbound messages per person (app, Discord, Slack, SMS)
