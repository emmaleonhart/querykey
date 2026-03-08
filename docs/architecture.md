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

## Two Deployable Components

### 1. Secretarybird Server

The backend. Runs locally on a user's machine or deployed to the cloud. Handles:

- Ingesting and normalizing all input (text, audio, images)
- Storing the knowledge graph
- Real-time WebSocket sync with all connected clients
- Coordinating with OpenClaw for AI analysis

The server is where the knowledge graph lives. Clients are thin — they send data in and render the graph out.

**Local mode**: Server runs on the user's machine. Good for privacy-sensitive environments, solo use, or small teams that want to keep everything on-premise.

**Cloud mode**: Server runs on cloud infrastructure. Good for distributed teams, always-on monitoring (Discord/Slack bots), and mobile access from anywhere.

### 2. Secretarybird Mobile (Flutter App)

Single Flutter codebase targeting:
- **iOS** and **Android** (phone/tablet)
- **Desktop** (macOS, Windows, Linux)
- **Web**

All platforms share the same core UI. Mobile gets additional capabilities:
- Microphone recording (meetings, conversations)
- Voice note capture
- Push notifications
- Background audio streaming

## Unstructured Input Pipeline

The core differentiator. The ingest service accepts anything and normalizes it.

### Input Types

| Input | Processing | Notes |
|---|---|---|
| **Live bot feed** (Discord/Slack) | Direct text ingestion | Real-time, structured, easiest to process |
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

**What OpenClaw does NOT do:**
- Store the knowledge graph (that's the Secretarybird Server)
- Handle real-time sync (that's WebSocket on the server)
- Manage users or permissions (that's the server)

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

**Speaker diarization**: For recorded conversations, the system needs to identify who is speaking. This is hard but important for attribution. Initial approach can be manual tagging ("that was John") with AI-assisted suggestions over time.

## Deployment Options

| Mode | Server Location | Bot Monitoring | Best For |
|---|---|---|---|
| **Local** | User's machine | Only while running | Solo users, privacy-first, small teams |
| **Cloud** | Cloud VM/container | Always-on | Distributed teams, 24/7 Discord/Slack monitoring |
| **Hybrid** | Cloud server, local processing | Always-on | Teams that want always-on but sensitive data stays local |

## Open Questions

- Exact graph database choice (Neo4j vs PostgreSQL vs hybrid)
- Speaker diarization model/service selection
- How to handle multi-language conversations
- Privacy and data retention policies (especially for audio)
- Rate limiting / cost management for OpenClaw calls on high-volume feeds
- How to handle encrypted or private channels (permissions model)
- Offline mode behavior for the Flutter app
- How much history to port from the original secretarybird repo (socket layer is priority)
