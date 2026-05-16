# Markdown On-Disk Schema (canonical source of truth)

> **Status: spec / decided-direction (Round 2, 2026-05-15).** This is
> the load-bearing decision in QueryKey: everything downstream (the
> ingestion pipeline, the agent acting on files, the derived graph)
> depends on it. It is a *spec*; the on-disk model is **not implemented
> yet**. Authoritative alongside `CLAUDE.md` / `queue.md`.

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
confidence: 0.95
source: [message:2026-05-14-15:05-discord]
---

Climbing session, the usual gym.
```

### Note — `notes/<slug>.md`

Frontmatter optional beyond `id`/`type`. The body is the point;
`[[wikilinks]]` to entity slugs are how a note attaches to the graph.

## Derived graph contract

- The graph is generated **from** these files; it is never the source
  of truth and must be reconstructible by a full re-scan.
- Edges come from frontmatter references (`person:`, `people:`,
  `[[links]]`) — not hand-authored separately.
- The time dimension is first-class (git history + `created`/`updated`
  + event times) because relationship history matters.

## Open sub-questions (decide before implementing)

- Exact `status` / `ambiguity` enumerations and their transitions.
- How freeform-body `[[wikilinks]]` resolve vs. explicit frontmatter
  refs (precedence, dangling links).
- Whether `confidence`/`source` live inline per-field for fine-grained
  provenance, or once per file (leaning once-per-file for editability).
- Conflict/Instruction/OpenQuestion/FollowUp/VoiceProfile file shapes
  (entities exist in `docs/data-model.md`; on-disk forms TBD).

These do not block writing the spec; they block *implementation*.
