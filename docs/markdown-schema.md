# Markdown On-Disk Schema (canonical source of truth)

> **Status: IMPLEMENTED (Round 5, 2026-05-15).** This was the
> load-bearing decision; it is now built. `server/src/vault/` is the
> canonical store: the API and ingest pipeline write these markdown
> files first, then project a derived index into Loca; the server
> rebuilds that index from the vault on startup; `update_task` mutates
> the markdown. Round-trips are lossless (unit-tested). `VAULT_DIR`
> selects the root (default `./vault`). Authoritative alongside
> `CLAUDE.md` / `queue.md`. The spec below matches the implementation;
> deviations are noted under "Implementation notes".

## Principles

1. **Markdown files are the canonical store of truth.** Not a database,
   not the graph. The graph (Loca/SutraDB) is *derived* from these
   files and is always rebuildable from them.
2. **Git tracks history.** A relationship changes over time and you
   want that provenance. The repo *is* the database's WAL.
3. **YAML frontmatter + freeform body.** Structured, queryable fields
   go in frontmatter; everything human goes in the body. This is the
   Obsidian convention on purpose — the files are immediately useful in
   any markdown editor with **zero QueryKey installed**.
4. **Human-readable IDs.** Every entity has a readable slug
   (`person:john-smith`), never a bare UUID, as `id` in frontmatter.
   The filename should match the slug.
5. **Epistemic humility is in the schema.** Extracted fields carry
   `confidence` and `source` so nothing pretends to be certain.
6. **The agent edits these files.** Round-trips must be lossless and
   diff-friendly (stable key order, no reflowing the human body).

## Directory layout

```
vault/                      # the user's QueryKey git repo
  people/<slug>.md          # one file per person
  tasks/<slug>.md           # one file per task
  events/<slug>.md          # one file per event
  notes/<slug>.md           # freeform notes (may reference entities)
  card.md                   # YOUR broadcast card (git-tracked; see
                             #   docs/card-format.md)
  .querykey/                # derived/cache (graph snapshots) — gitignored
  peers/<handle>/card.md    # other people's cards — GITIGNORED
```

Rationale for `peers/` being gitignored: the **asymmetry** from the
vision — your own card has history (for undo); other people's cards are
usable in the moment but never archived into your history. See
`docs/card-format.md`.

## Frontmatter conventions

Common fields on every entity:

| Key | Type | Notes |
|---|---|---|
| `id` | string | `type:slug`, matches filename. Required. |
| `type` | enum | `person` \| `task` \| `event` \| `note`. Required. |
| `created` / `updated` | ISO-8601 | Maintained by QueryKey. |
| `confidence` | 0.0–1.0 | Agent's certainty in the structured fields. |
| `source` | list | Provenance: ingest item ids / message refs. |
| `tags` | list | Freeform. |

### Person — `people/<slug>.md`

```markdown
---
id: person:john-smith
type: person
handles:
  discord: john_dev#1234
  github: jsmith
  email: john@example.com
relationship: friend
confidence: 0.9
source: [ingest:2026-05-14-discord-batch]
tags: [climbing, rust]
---

# John Smith

Met through the climbing gym. Strong Rust opinions; owes me a book.
```

`handles` is the cross-platform identity map (one human, many
platforms). The graph edge "same person" is derived from this.

### Task — `tasks/<slug>.md`

```markdown
---
id: task:return-johns-book
type: task
status: extracted        # extracted → confirmed → in_progress → done | disputed
person: person:john-smith   # who it relates to (not "assigned to")
deadline: 2026-05-20         # optional — tasks are time-flexible
confidence: 0.7
ambiguity: low
source: [message:2026-05-14-15:02-discord]
---

Return the Rust book John lent me. He mentioned it casually, so the
deadline is a guess.
```

### Event — `events/<slug>.md`

Time-**fixed** (has `start` + `end`); a task is time-flexible. Rule of
thumb: if you can move it to tomorrow without asking anyone, it's a
task, not an event.

```markdown
---
id: event:climbing-with-john
type: event
start: 2026-05-18T18:00:00
end:   2026-05-18T20:00:00
people: [person:john-smith]
recurrence: FREQ=WEEKLY;INTERVAL=1;COUNT=10   # optional; omit = one-off
confidence: 0.95
source: [message:2026-05-14-15:05-discord]
---

Climbing session, the usual gym.
```

`recurrence` (Round 11) is an optional RFC-5545 **subset**:
`FREQ=DAILY|WEEKLY|MONTHLY|YEARLY`, `INTERVAL`, `COUNT`, `UNTIL`.
Unsupported parts (`BYDAY`, …) are ignored, not errors. Occurrences
are computed from the `start` anchor (a clamped short month — Jan 31
→ Feb 28 — does not drag later months off the day-of-month).
`GET /api/calendar?from&to` returns a **merged agenda**: expanded
event occurrences (`movable:false`) plus tasks whose `deadline` is in
the window (`movable:true`) — the Task-vs-Event distinction made
queryable. Computed live from the canonical vault.

### Note — `notes/<slug>.md`

Frontmatter optional beyond `id`/`type` (a note may have *no*
frontmatter at all — the body is the point). `[[wikilinks]]` in the
body are how a note attaches to the graph.

### Semantic wikilinks (IMPLEMENTED, Round 8)

Any entity body (person/task/event/note) may contain wikilinks; each
becomes a derived graph edge from that entity:

- `[[Target]]` — an **untyped reference**. Predicate: `references`.
- `[[property:Target]]` — a **semantic triple**: the token before the
  **single** `:` is the predicate. `[[employer:Acme Corp]]` in
  `people/jane.md` ⇒ `(person:jane) —employer→ (Acme Corp)`.
  Deliberately a single colon — *not* Semantic-MediaWiki's `::` (an
  accidental `::` is parsed forgivingly).
- `[[Target|Alias]]` — Obsidian display alias; the alias does not
  affect the edge (left side is the link).
- A predicate token is `[a-z][a-z0-9_-]*` and not a URI scheme, so
  `[[https://x]]` is an untyped link, not an `https:` predicate.

**Resolution precedence** (first match wins — this is the answer to
the formerly-open "wikilink vs frontmatter ref" question):

1. an explicit `kind:id` (`[[knows:person:jane]]`) — symmetry with
   frontmatter refs;
2. **person** by id/slug, then by display name;
3. **task** by uuid, then by title;
4. **event** by uuid, then by title;
5. **note** by slug;
6. **dangling** → kind `thing`, id `slug(target)`, `resolved:false` —
   the edge is **never dropped**; dangling links stay queryable and
   the raw target is kept as a `label`. (A note for a not-yet-created
   person still connects; the node materializes when they're added.)

Matching is slug-insensitive (`John  Smith` ≡ `john-smith`).
Frontmatter refs (`person:`, `people:`) and wikilink edges are
**additive**, not competing — frontmatter carries structural fields,
wikilinks carry freeform relations.

Served live from the canonical vault at `GET /api/links` and
`GET /api/entities/:kind/:id/links` (`{from: outgoing, to:
backlinks}`); also projected into the derived triple store on the
startup rebuild (a SPARQL convenience — markdown stays canon).
Code-fence exclusion and nested brackets are documented future
refinements (rare in practice).

### Conflict — `conflicts/<uuid>.md`

Two pieces of information that contradict each other (a reassignment, a
deadline change, …). The body is the human `explanation`; resolution
state lives in frontmatter so it survives a restart and is hand-editable.

```markdown
---
id: conflict:8f3a…-uuid
type: conflict
conflict_type: deadline_change
message_a: msg-a-ref
message_b: msg-b-ref
task: task:return-johns-book
resolution: a_wins        # unresolved → a_wins | b_wins | merged | dismissed
resolved_by: immanuelle
created: 2026-05-15T09:00:00+00:00
resolved: 2026-05-15T10:00:00+00:00
---

Alice said Friday, Bob said Monday. Needs a human call.
```

### Open question — `questions/<slug>.md`

The queue of things the system needs answered. Body is the
human-facing `question`; `urgency` drives surfacing.

```markdown
---
id: question:deadline-for-johns-book
type: question
target: person:john-smith
context: Casual mention; deadline is a guess.
urgency: by_time          # asap | by_time | end_of_day | whenever
urgency_deadline: 2026-05-20T00:00:00+00:00
trigger_type: ambiguity
trigger_id: task:return-johns-book
status: open               # open → resolved | expired
created: 2026-05-15T09:00:00+00:00
---

When does John actually need the book back?
```

### Follow-up — `followups/<slug>.md`

A nudge the agent sends (and tracks delivery of) on the user's behalf.
Body is the `question`; `delivery_attempts` is a nested frontmatter list.

```markdown
---
id: followup:ping-john-about-book
type: followup
trigger_type: unconfirmed_task
trigger_id: task:return-johns-book
target: person:john-smith
context: No reply to the first nudge.
delivery_attempts:
  - channel: discord
    status: delivered
    sent_at: 2026-05-15T11:00:00+00:00
status: sent               # pending → sent → answered | expired
created: 2026-05-15T10:30:00+00:00
---

Still good to drop the book off Saturday?
```

### Instruction — `instructions/<uuid>.md`

Who said what to whom. Body is the human `content` (the actual
utterance); `is_task`/`task` link it to a Task when it produced one.

```markdown
---
id: instruction:7c2a…-uuid
type: instruction
speaker: alice
audience: [bob, carol]
is_task: true
task: task:ship-the-report
source_message: ingest:2026-05-16-batch
created: 2026-05-16T09:00:00+00:00
---

Ship the report by Friday. No extensions.
```

### Voice profile — `voiceprofiles/<uuid>.md`

Speaker identity for diarization — the most "machine" entity. Body is
a cosmetic heading; everything is frontmatter. `embedding` is a YAML
int sequence, **omitted entirely until the audio pipeline fills it**
(the common case today); not meant for hand-editing.

```markdown
---
id: voiceprofile:9f1b…-uuid
type: voice_profile
person: person:ada-lovelace
sample_count: 12
confidence: 0.83
last_updated: 2026-05-16T10:00:00+00:00
created: 2026-05-16T08:00:00+00:00
---

# Voice profile — person:ada-lovelace
```

## Derived graph contract

- The graph is generated **from** these files; it is never the source
  of truth and must be reconstructible by a full re-scan.
- Edges come from frontmatter references (`person:`, `people:`,
  `[[links]]`) — not hand-authored separately.
- The time dimension is first-class (git history + `created`/`updated`
  + event times) because relationship history matters.

## Implementation notes (Round 5 — as built)

- **Entities implemented:** Person (`people/<id>.md`), Task
  (`tasks/<uuid>.md`), Event (`events/<uuid>.md`), Conflict
  (`conflicts/<uuid>.md`), OpenQuestion (`questions/<slug>.md`),
  FollowUp (`followups/<slug>.md`) — the last three added in Round 6.
  `notes/` exists.
- **Round 6 wiring:** ingest writes conflicts vault-first then
  projects to the derived graph; `GET /api/conflicts|questions|
  followups` read the vault at full fidelity; `resolve_conflict`,
  `resolve_question`, and `create_followup` are real markdown
  mutations (read → patch → write → project/broadcast), replacing the
  R4 `not_implemented` stubs. Conflict's body is its `explanation`;
  OpenQuestion/FollowUp bodies are the `question`; the `conflict_type`
  is in frontmatter (`type` is reserved for the entity kind).
- **`title` lives in frontmatter** for Task/Event (the model carries a
  separate `title` and `description`); the **body is the description**.
  This keeps round-trips lossless rather than deriving a title from the
  body. Person's body is a `# Display Name` heading.
- **`confidence`/`source` are once-per-file** (frontmatter), per the
  editability lean — not inline per-field.
- **Slugs:** Person uses its human id; Task/Event use the UUID. (Title
  → human-slug generation is a future nicety.)
- **Idempotent:** `compose`/`split` trim the body consistently so
  write→read→write is stable; a unit test asserts lossless Person/Task
  round-trip (timestamps, handles, deadline, multi-line body).
- **Derived graph:** the server projects vault → Loca on every write
  and rebuilds the whole graph from the vault on startup.

## Still open (not blocking)

- `status` transition rules — **DONE (Round 9).** Task/Conflict/
  Question state machines (`src/workflow`) enforced at the API
  mutation boundary (not the vault — hand-edited markdown stays
  legal): a resolved conflict can't be un-resolved, `done` can't
  rewind to `extracted`, etc. (`ambiguity` is a score, not a
  lifecycle — no transition rules apply.)
- Freeform-body `[[wikilinks]]` resolution + semantic
  `[[property:target]]` triples — **DONE (Round 8).** Precedence +
  dangling specified above; parser/resolver/projection unit-tested;
  `/api/links` + backlinks live from the vault.
- Conflict / OpenQuestion / FollowUp — **DONE (Round 6).** Canonical
  on-disk forms + vault-first wiring; lossless round-trip unit-tested.
- Instruction / VoiceProfile on-disk forms — **DONE (Round 10).**
  `instructions/<uuid>.md` (body = the utterance `content`) and
  `voiceprofiles/<uuid>.md` (machine entity: all frontmatter,
  embedding as a YAML int sequence omitted until audio fills it).
  Instruction is written by ingest; both have read/upsert API.
  Lossless round-trip unit-tested. **The full canonical entity set
  is now on disk** — no entity is graph-only or unimplemented.
  (Round 9 also covers the new entities — the resolved-conflict rule
  is enforced; see the `status` transition entry above.)
