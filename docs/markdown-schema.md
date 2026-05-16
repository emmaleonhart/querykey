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

## Implementation notes (Round 5 — as built)

- **Entities implemented:** Person (`people/<id>.md`), Task
  (`tasks/<uuid>.md`), Event (`events/<uuid>.md`). `notes/` exists.
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

- `status` / `ambiguity` transition rules (enums exist; the *workflow*
  isn't enforced yet).
- Freeform-body `[[wikilinks]]` resolution vs. explicit frontmatter
  refs (precedence, dangling links).
- Conflict / Instruction / OpenQuestion / FollowUp / VoiceProfile
  on-disk forms — **TBD**; these currently stay graph-only (conflicts)
  or unimplemented. They do not block the Person/Task/Event canonical
  path that is now live.
