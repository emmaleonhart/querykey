# QueryKey — Queue

> **This file is concrete in-flight steps only.** Delete an item when it
> is done — **no `[x]` checkmarks, no "COMPLETE" blocks, no per-Round
> logs.** Finished work lives in `git log` (each change is its own commit
> whose message is the record). Long-horizon / abstract goals live in
> `todo.md` and get decomposed into items here when ready. **Plan-first:**
> a plan is written here *before* execution so an interrupted session
> resumes from the queue, not from chat. This discipline was adopted in
> the Round 14 audit below (matching the life-planning / cleanvibe
> convention); before that this file had grown to 882 lines of completed
> Round history — that is exactly what `git log` is for.

---

## RECOVERY QUEUE 2

See the new bigger chat I just imported, please extract it and move forward since it has more vision, once the timing is complete we can move onto the other stuff. Remembering here that the queue might not be ptoperly cleared with the size of it, but not sure

Please keep this section when rebasing with the remote

> **Status (extracted 2026-05-15).** The bigger chat was extracted via
> Sutra `extract_chat.py` and now lives at
> `chat/public/vision - rationalist p2p social network (Claude).md`
> (39 KB, 19 exchanges) — it **replaces** the prior 27 KB extraction.
> **Future/other sessions: read THIS file as the authoritative vision.**
> New material vs. the prior version (folded lightly into the docs;
> deeper reorg left for the next session):
> - The **card is literally a Q/K pair** — `query` = what you're
>   looking for, `key` = what you offer. **V is not stored**: it's the
>   real-world outcome of people actually connecting (epistemically
>   honest, anti-gamification).
> - **Card ≠ profile.** The card is a lean *signal* (query + key + a
>   short bio or a link out to your personal site). Your personal
>   website is the substance/source of truth for "who you are"; the
>   card just points to it. Keeps the P2P payload small and the format
>   stable.
> - **Agent-drafted cards.** People (esp. younger) are bad at
>   articulating their own value; the local agent drafts `key`/`query`
>   from the PRM it already built by observing you — you curate/approve.
> - **`agents.md`** (name/shape flexible — could be an `agents/` dir):
>   the local agent's behavior/heuristics/prompts as an editable,
>   version-controlled markdown file in your repo. Transparent, not a
>   black box; the MCP server executes within that envelope; the
>   rationalist community will share/compare configs.
> - **Strategy:** stop over-engineering the social layer; the PRM is
>   "pretty complete" — next step is real daily use (eat own cooking),
>   rationalist/LessWrong as first cohort.

---

## ACTIVE

*Nothing mid-flight. Round 20 (user-directed Flutter→Electron rewrite
of the desktop UI: `app-electron/` scaffold → Electron-managed Rust
server lifecycle → renderer API client → Profile view → Wiki view →
`!run-UI.bat` launcher → Flutter `app/` retired) shipped on top of
R18/R19. The Electron app talks to the same unchanged Rust server and
manages it itself (no `.bat`). Record is in `git log` + README Status;
**UI = Electron is settled (CLAUDE.md/todo.md), do not revert**. Pull
the next item from `todo.md` when starting fresh.*

### Open follow-ups (small, not mid-flight)

- **Visual GUI confirmation of the UI.** R18 verified the data path
  end-to-end (live: `/api/card` + 135 contacts served from
  `life-planning/prm`) and the screen code by review, but no one has
  run the actual desktop window to eyeball Profile/Wiki rendering.
  Next time the app is run via `!run-UI.bat`, confirm the card
  renders and Wiki→Contacts lists people; file anything off as a
  new round.
- **Loka Studio graph-viz reuse (deferred, user-flagged).** The user
  wants knowledge-graph visualization à la `SutraDB/loka-studio`
  (force-directed `graph_canvas.dart`). Deferred: only the Flutter
  loka-studio exists (no Electron one found) and its RDF/SPARQL
  backend ≠ QueryKey's markdown vault, so reuse = lifting the canvas +
  a vault→graph adapter, not a drop-in. The 2026-05-16 framework
  review concluded Flutter is fine for this (loka-studio's canvas is
  itself Flutter). Revisit as its own round if/when the user reopens.

---

## Vision

QueryKey is a **rationalist social network** that doubles as a personal
PRM / lightweight CRM / JIRA-style task tracker. It runs on your own
machine. It is built around the working theory that the most useful
software for thinking carefully about people, commitments, and the
state of your own projects is software that:

- you fully own and run locally,
- holds its data as plain markdown files on your disk that you can read
  and edit by hand,
- is operated mostly by a **local AI agent** that you can swap models
  for (default: **Gemma**), and
- treats epistemic humility — confidence scores, "I'm not sure, want me
  to ask?" — as the central UX, not a footnote.

### Why "QueryKey"

The name is a reference to the **Q / K / V** (query, key, value)
projections inside a transformer attention matrix. The product
metaphor: your day, your relationships, and your tasks are a body of
*values* that the local agent attends over by computing *queries* from
your current intent against *keys* it has built from your markdown
notes, chat logs, and prior conversations.

### Product surfaces

- **Local markdown files** on disk — the source of truth for tasks,
  events, and notes. Users can edit them in any editor.
- **Flutter app** — desktop-first (Windows now; macOS/Linux/iOS/Android/
  Web later). The interactive surface for chat with the agent, task
  boards, calendars, etc.
- **Local agent** — runs on the user's machine. Default model Gemma;
  switchable to other local models or, optionally, hosted ones.
- **Server** — **Rust** (`server/`, crate `querykey-server`); the only
  server. Go was fully ported then removed in Round 4 (recoverable from
  git history). Wires the agent, the markdown vault, the derived graph,
  and the app together.

### What it is *not*

- Not a SaaS. Not a team coordination tool you're forced to adopt. Not
  a surveillance/productivity scoreboard.
- Not opinionated about *how* you work — it conforms to your workflow.
- **Not privacy-focused.** By design it collects and processes personal
  information about people (that is what a PRM *is*). The privacy stance
  is *soft*: (1) no *careless* spread, (2) **no centralized store**,
  (3) anything beyond a local user moves **peer-to-peer**, never via a
  central server. A user's vault is their own tracked git repo —
  committing personal data there is the design, not a leak. See
  `docs/card-format.md` and `README.md`.

---

## Open decisions — RESOLVED (settled architecture; do not relitigate)

| Decision | Choice | Notes |
|---|---|---|
| Disposition of `secretarybird-old/` | **Deleted** after comparison doc | Hackathon refs gone. Code preserved in git history. |
| Server language | **Rust** | Go fully removed (Round 4). |
| AI engine framing | **Model-agnostic via MCP, Gemma default** | MCP day one; any agent attaches. The agent is *whoever operates QueryKey* (Claude now; Gemma is the not-yet-built GUI default). |
| UI framework | **Flutter** | Locked. Not up for debate. |
| Graph store | **Loca (formerly SutraDB)** | Author's embedded Rust graph-vector-time DB. **Fuseki is NOT used.** |
| Canonical store | **Markdown + git; graph is derived** | Markdown is source of truth; graph rebuildable from it. |
| On-disk format | **YAML frontmatter + freeform body** | Obsidian-style; usable without QueryKey. |
| Social model | **Pure P2P card exchange** | Own card git-tracked; others' cards git-ignored; 24h propagation delay; no central server. |
| Identity / discovery | **GitHub (swappable)** | Usernames as handles, follow-on-GitHub discovery, behind a thin handle abstraction. |
| Sequencing | **PRM → P2P; MCP day one** | Private PRM first (builds the graph the cards window into); card layer second. |

## Open decisions — STILL OPEN / parked

- **P2P transport + discovery** — what actually moves a card between
  peers (shared GitHub org repo? Nostr relay? true P2P) + how you
  discover whose cards to pull. *The* gating social design question;
  **parked by explicit user steering — do not barrel on a guess.**
- **`wiki/information/` and `wiki/projects/` semantics** — the
  canonical buckets are pre-created but their meaning + whether
  non-contact entities (tasks / events / …) are themselves
  graph-bearing is **user-defined-open**. Flagged in `todo.md`. Do
  not guess.
- **Card format spec** — built (the `## Offering`/`## Looking for`
  contract) but will still evolve; it ossifies once cards are exchanged.
- Private vs. public card (deferred — more complex; after single-card).
- Audio pipeline ownership in the Rust world (→ back of `todo.md`).
- Voice-profile / speaker-diarization model selection (waits on audio).
- External tool sync (Jira / Azure DevOps / GitHub) — still desired? In
  what tier?

## Direction (2026-05-16) — social layer parked, PRM is the priority

**User decision:** put the P2P / social layer **aside**. The card
*format* + *local* layer is a good stopping point; the remaining social
work is the **P2P transport**, an unresolved *design* question, not
just unbuilt code — do **not** barrel a transport on a guess. The
**PRM structure is the better thing to work on**: the social card is
only ever a selective window into a graph the PRM builds, so deepening
the PRM compounds. Future sessions: prioritize PRM/vault/graph
structure over anything P2P until the user reopens the social track.

**UPDATE (2026-05-16, later):** the user **reopened one sub-piece** —
the **agent-drafted card↔graph** projection (the local agent reads the
PRM and drafts a key/query for approval). The PRM *output* side is in
scope. **The P2P transport + discovery remain parked.** Audio pipeline
moved to the back of `todo.md`.

---

## Rounds 1–13, 15 — COMPLETE (history in `git log`)

The full Round-by-Round detail (the pivot bootstrap; Go→Rust port;
canonical markdown vault; Conflict/Question/FollowUp +
Instruction/VoiceProfile on-disk forms; semantic wikilinks;
status-workflow enforcement; calendar; agent-drafted card;
P2P card *format*+local layer + GitHub identity; the R13 agent-honesty
fix; the R15 `querykey.toml` vault-root marker + `wiki/` layout +
people→contacts rename) is **in `git log`** — every round was its
own commit whose message is the record. Per the discipline at the
top of this file, completed rounds are **not** retained here. Net
state is reflected in `README.md` (Status), `CLAUDE.md`, `todo.md`,
and `docs/`. (Round 14 was an audit, not a feature.)

---

## Notes for future sessions

- The user dictates long stream-of-consciousness messages via voice. Do
  not interpret them literally — listen for the underlying intent.
- The user has explicitly asked to **avoid planning-only modes** in
  `CLAUDE.md`. Default to executing; write the plan into this queue
  first, then do it.
- Do not reintroduce the "Secretarybird" name or hackathon references.
- Flutter is settled (firm — frontend). Rust **is** the server (Go
  fully removed; only git history has it). The agent is model-agnostic
  (Claude now; Gemma the not-yet-built GUI default). Not open questions.
- **Fuseki is NOT used.** Graph store is **Loca/SutraDB**, derived from
  canonical markdown; markdown + git is the source of truth.
- A user's vault repo (e.g. the `life-planning` prototype) tracks
  people's personal data **by design** — never gitignore/reset/scrub
  it. Only the public *software* repo (this one) must stay PII-free.
- **Priority: PRM structure > social layer.** PRM/vault/graph is
  broadly built out. **P2P transport + discovery remain parked**
  pending a *design* decision — the single biggest open question; do
  **not** barrel it on a guess.

## Pointers

- Long-horizon roadmap: `todo.md`
- Narrative history of completed work: `git log`
- Authoritative vision corpus: `chat/public/vision - rationalist p2p social network (Claude).md`
- Specs: `docs/markdown-schema.md`, `docs/card-format.md`, `docs/data-model.md`, `docs/architecture.md`
