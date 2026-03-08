# Data Model & Knowledge Graph

## Design Principle

The data model must handle the fact that most input is unstructured. A pasted screenshot and a bot feed produce the same downstream entities, but with different confidence levels. The system tracks provenance — every task and instruction can be traced back to the raw input that produced it.

## Core Entities

### Person
Represents a team member tracked by the system.

```
Person {
  id: UUID
  display_name: string
  handles: [
    { platform: "discord", identifier: "user#1234" },
    { platform: "slack", identifier: "U12345" },
    { platform: "phone", identifier: "+1..." },
  ]
  role: string (optional)
  created_at: timestamp
}
```

The system must resolve the same person across platforms. A single person may appear as a Discord username, a Slack user, and a voice in a meeting. Handle linking is done initially by manual mapping and later augmented by AI-assisted matching.

### Task
An actionable work item extracted from conversation.

```
Task {
  id: UUID
  title: string                    // AI-generated summary
  description: string              // fuller context
  status: "extracted" | "confirmed" | "in_progress" | "done" | "disputed"
  assigned_to: Person.id (nullable)
  assigned_by: Person.id (nullable)
  deadline: timestamp (nullable)
  confidence: float                // AI confidence that this is a real task
  ambiguity_score: float           // how vague the instruction was
  source_messages: [Message.id]    // the messages this task was derived from
  created_at: timestamp
  updated_at: timestamp
}
```

**Status flow:**
```
extracted → confirmed → in_progress → done
    │                       │
    └── disputed ◄──────────┘
```

- `extracted`: AI detected a task, not yet reviewed by a human
- `confirmed`: A user confirmed this is a real task
- `disputed`: Marked as contradictory or contested
- Tasks can also be manually dismissed/deleted

### IngestItem
A raw input submitted to the system — could be anything from a bot feed message to a pasted screenshot.

```
IngestItem {
  id: UUID
  input_type: "bot_feed" | "chatlog_paste" | "screenshot" | "voice_note" | "recorded_audio" | "freeform_text"
  raw_content: bytes | string
  submitted_by: User.id
  submitted_at: timestamp
  source_context: string (optional)   // user-provided label like "Monday standup"
}
```

### Message
A normalized record of something someone said. Extracted from IngestItems — a single IngestItem (like a pasted chatlog) may produce many Messages.

```
Message {
  id: UUID
  source_ingest: IngestItem.id        // which input this came from
  author: Person.id
  content: string
  timestamp: timestamp (nullable)     // not always determinable from unstructured input
  confidence: float                   // how confident the extraction is
  raw_metadata: object
}
```

### Conflict
When two instructions contradict each other.

```
Conflict {
  id: UUID
  type: "contradictory_tasks" | "reassignment" | "deadline_change" | "scope_change"
  message_a: Message.id
  message_b: Message.id
  task: Task.id (nullable)         // the task affected, if applicable
  explanation: string              // AI-generated explanation of the contradiction
  resolution: "unresolved" | "a_wins" | "b_wins" | "merged" | "dismissed"
  resolved_by: Person.id (nullable)
  created_at: timestamp
  resolved_at: timestamp (nullable)
}
```

### Instruction
A broader category than Task — any directive or statement of intent, even if not clearly actionable.

```
Instruction {
  id: UUID
  content: string
  speaker: Person.id
  audience: [Person.id]            // who it was directed at
  is_task: bool                    // if true, a linked Task entity exists
  task_id: Task.id (nullable)
  source_message: Message.id
  created_at: timestamp
}
```

### ExternalSync
Tracks tasks that have been pushed to external project management tools.

```
ExternalSync {
  id: UUID
  task_id: Task.id
  platform: "jira" | "azure_devops" | "github" | "gitlab" | "trello" | "asana"
  external_id: string               // the ID in the external system (e.g., PROJ-123)
  external_url: string              // link to the item in the external tool
  sync_direction: "push" | "bidirectional"
  last_synced_at: timestamp
  status_in_external: string        // the status as reported by the external tool
}
```

## Graph Relationships

The knowledge graph connects these entities:

```
IngestItem --[PRODUCED]--> Message (one-to-many: a pasted chatlog produces many messages)
Person --[ASSIGNED_TO]--> Task
Person --[ASSIGNED_BY]--> Task
Person --[SAID]--> Message
Message --[EXTRACTED]--> Task
Message --[EXTRACTED]--> Instruction
Conflict --[BETWEEN]--> Message (x2)
Conflict --[AFFECTS]--> Task
Instruction --[CONFLICTS_WITH]--> Instruction
Task --[DEPENDS_ON]--> Task (optional, if detected)
Task --[SUPERSEDED_BY]--> Task
Task --[SYNCED_TO]--> ExternalSync (pushed to Jira, Azure DevOps, etc.)
```

## Contradiction Detection Logic

The AI pipeline compares each new instruction/task against the existing graph to find:

1. **Direct contradictions**: "Do X" followed by "Don't do X" or "Do Y instead"
2. **Reassignments**: Task was assigned to A, now being assigned to B without acknowledging the change
3. **Deadline conflicts**: Different deadlines stated for the same deliverable
4. **Scope changes**: The definition of a task silently shifts between conversations
5. **Authority conflicts**: Two people with different levels of authority give conflicting instructions

Each detected conflict gets an AI-generated explanation and surfaces in the UI for human resolution.

## Real-time Graph Updates

When a new message arrives:

1. Message is stored
2. AI pipeline extracts entities, instructions, tasks
3. New entities are linked to existing graph nodes (or new nodes created)
4. Contradiction detection runs against relevant subgraph
5. Graph diff is computed and broadcast to clients
6. Clients apply the diff to their local graph state

The diff format:

```
GraphDiff {
  added_nodes: [Node]
  updated_nodes: [Node]
  added_edges: [Edge]
  removed_edges: [Edge]
  new_conflicts: [Conflict]
  resolved_conflicts: [Conflict.id]
}
```

## Queries the System Must Support

- "What tasks are assigned to person X?"
- "What contradictions are currently unresolved?"
- "Show me everything said about topic Y across all channels"
- "Who assigned this task and when?"
- "What changed about this task over time?" (audit trail)
- "What is person X's current workload?"
- "Show all instructions from the last meeting that haven't been turned into tasks"
