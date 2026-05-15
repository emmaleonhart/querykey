# `chat/` — vision corpus (mostly gitignored)

This directory is where chat-log exports that carry context about
QueryKey's vision get dumped: Discord servers and DMs, Claude / Grok /
ChatGPT conversations, voice transcripts, pasted text.

**This file is the only thing in here that is committed.** Everything
else is gitignored (see the repo root `.gitignore`) because these
exports contain private and personal information — about the author and
about other real people who did not consent to being in a public repo.
Privacy here is not just the author's; it is everyone mentioned.

## How future agents should treat this directory

- It is a **corpus to read selectively**, not a spec to follow
  literally. The messages are long, informal, stream-of-consciousness,
  and often contradict each other as the idea evolved.
- Listen for **underlying intent**, not surface wording. The author
  dictates via voice-to-text; transcription noise is common.
- Use it as background when reframing docs (`README.md`, `CLAUDE.md`,
  `todo.md`, `docs/`), naming things, or resolving "what did they
  actually mean" questions. Do not quote personal content into
  committed files.
- The authoritative, committed plan is `queue.md` at the repo root —
  not anything in here.

## What's currently dumped here

Conversations referencing **Secretarybird**, **KQV / QKV** (the
transformer query/key/value attention origin of the "QueryKey" name),
and the QueryKey framing — moved out of a separate life-planning
archive so this project is self-contained. Sources: Grok, Claude, and
Discord (a "Secretarybird" server channel plus several DMs).
