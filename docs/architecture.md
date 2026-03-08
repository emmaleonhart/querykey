# Architecture Plan

## System Components

### 1. Flutter Frontend

Single codebase targeting mobile (iOS/Android) and desktop/web.

**Mobile-specific features:**
- Microphone access and audio recording
- Background audio streaming to server
- Push notifications for contradiction alerts and new task assignments

**Shared features (mobile + desktop):**
- Knowledge graph visualization (interactive node-edge view)
- Task board view (Jira-style kanban: unassigned, assigned, in progress, done)
- Contradiction/conflict view with side-by-side comparison of conflicting instructions
- Feed management (connect/disconnect sources, view raw feed history)
- Person directory (who is tracked, their assignments, workload)

### 2. OpenClaw Backend

The server handles all heavy processing. The Flutter app is a thin client that streams data and renders the graph.

#### 2.1 Ingest Service

Responsible for connecting to external communication sources and normalizing incoming messages into a common format.

**Common message format:**
```
{
  "source": "discord" | "slack" | "audio" | "paste" | "social",
  "channel": string,        // channel/thread/meeting identifier
  "author": string,          // who said it
  "timestamp": ISO 8601,
  "content": string,         // raw text (or transcription for audio)
  "raw_metadata": object     // source-specific metadata
}
```

**Source adapters:**

| Adapter | Connection Method | Notes |
|---|---|---|
| Discord | Discord bot via gateway websocket | Monitors configured servers/channels |
| Slack | Slack app via Events API | Monitors configured workspaces/channels |
| Audio | WebSocket stream from phone app | Raw audio → server-side transcription |
| Paste | REST endpoint | User pastes text, server processes it |
| Social | Polling or API integration | Platform-dependent |

#### 2.2 AI Pipeline

Processes normalized messages through a series of LLM-based analysis steps:

1. **Entity extraction** — Identify people, projects, deadlines, and other named entities
2. **Task detection** — Determine if a message contains an implicit or explicit task assignment
3. **Attribution** — Who assigned the task, who is it assigned to
4. **Contradiction detection** — Compare new tasks/instructions against existing graph state to flag conflicts
5. **Ambiguity scoring** — Rate how vague or unclear an instruction is

This pipeline runs on every incoming message (or batch of messages for paste/social). It updates the knowledge graph with new nodes/edges or flags conflicts.

#### 2.3 Knowledge Graph Store

The core data structure. Stores:

- **People** — team members, their roles, communication handles across platforms
- **Tasks** — extracted assignments with status, assignee, source message, deadline
- **Instructions** — raw directives that may or may not be tasks
- **Relationships** — who assigned what to whom, which messages relate to which tasks
- **Conflicts** — pairs of contradictory instructions with explanation of the contradiction
- **Timeline** — full history of graph mutations for audit trail

**Storage options to evaluate:**
- Neo4j (native graph DB, good for traversal queries)
- PostgreSQL with JSONB + recursive CTEs (simpler ops, good enough for most graph queries)
- A hybrid approach (relational for tasks/people, graph DB for relationship traversal)

#### 2.4 Real-time Sync

WebSocket connections between server and all connected Flutter clients. When the graph updates:

1. Server processes new message through AI pipeline
2. Graph store is updated
3. Diff is broadcast to all connected clients via WebSocket
4. Clients update their local graph view in real time

### 3. Audio Pipeline (Detail)

The phone app's audio recording feature is one of the most complex subsystems.

```
Phone mic → local audio buffer → WebSocket stream → Server
                                                      │
                                          ┌───────────┴───────────┐
                                          │ Transcription Service │
                                          │ (Whisper / equivalent)│
                                          └───────────┬───────────┘
                                                      │
                                              Normalized text
                                                      │
                                              AI Pipeline
                                                      │
                                              Graph Update
```

**Considerations:**
- Audio should be streamed in chunks, not recorded and uploaded as a whole file
- Server-side VAD (voice activity detection) to segment speakers
- Speaker diarization to attribute statements to specific people
- Transcription must be near real-time for the live graph update experience
- Privacy: need clear user consent model; audio can be discarded after transcription or retained per policy

## Deployment

- Backend runs on OpenClaw infrastructure
- Flutter app distributed via app stores (mobile) and as a web/desktop app
- Graph data lives server-side; clients cache a local subset for offline viewing

## Open Questions

- Exact graph database choice
- Speaker diarization model/service selection
- How to handle multi-language conversations
- Privacy and data retention policy details
- Rate limiting / cost management for LLM calls on high-volume feeds
- How to handle encrypted or private channels (permissions model)
