# Versions comparison: earlier prototype vs. current pivot

This document compares the **earlier prototype** that lived in
`secretarybird-old/` against the **current pivot** (`app/` + `server/`),
so that the prototype directory can be deleted without losing the few
lessons worth keeping. It is a one-time salvage pass, not living
documentation.

> The earlier prototype was a short-lived effort under a different
> framing. This doc records only what is technically relevant to
> QueryKey going forward; the old product framing is deliberately not
> reproduced.

## At a glance

| Concern | Earlier prototype (`secretarybird-old/`) | Current pivot (`app/` + `server/`) |
|---|---|---|
| UI | Electron 28 + hand-rolled TypeScript/HTML/CSS renderer | Flutter (one codebase: Windows/macOS/Linux/iOS/Android/Web) |
| Backend | Python 3.13 + FastAPI, ~43 REST endpoints + 1 WS | Go single binary (`server/`), `internal/*` packages |
| Runtime chain | TS (Electron) ↔ Python (FastAPI) ↔ WSL (OpenClaw) — three runtimes, three languages | Flutter ↔ Go ↔ WSL (OpenClaw) — OpenClaw bridge isolated in `internal/openclaw/` |
| AI engine | OpenClaw via WSL gateway HTTP API, surfaced directly in the product framing | Model-agnostic local agent (Gemma default, switchable); OpenClaw demoted to an implementation detail; **Rust is the server target**, Go marked deprecated |
| Install burden | Python + Node + WSL + OpenClaw + PyInstaller/NSIS installer | Flutter build + a single Go (later Rust) binary; WSL still needed only for the local agent today |
| Scope | Broad "business data assistant" — file org, Excel/CSV linting, Salesforce/Google/DB connectors, competitor scraping, social-feed monitoring, pipeline builder | Narrow and personal — local-first PRM / task graph; ingest → entities → tasks/events → follow-ups |
| Data model | Implicit, spread across Python integration modules | Explicit and documented (`docs/data-model.md`): Person, Handle, Task, Event, Message, etc. |
| Tests | pytest (backend) + vitest (frontend) + CI | Flutter widget test scaffold; server tests thin (gap to close) |

## Verdict: is the pivot superior?

For QueryKey's actual goal — a **local-first personal relationship /
task system you fully own** — yes, the pivot is the right base, and the
prototype is not worth resurrecting:

- **One UI codebase instead of a bespoke Electron renderer.** The old
  frontend was six hand-maintained renderer modules with no framework.
  Flutter replaces that with one codebase across every target platform
  and is the locked UI decision.
- **The three-runtime chain was the prototype's biggest liability.**
  TS→Python→WSL meant three failure domains, three dependency trees,
  and an installer that had to bootstrap WSL + OpenClaw + a PyInstaller
  bundle for non-technical users. The pivot collapses the backend to a
  single shippable binary; the only remaining external runtime is the
  local agent itself, and that is being made swappable.
- **Scope.** The prototype's value proposition was a broad B2B data
  assistant (connectors, competitor scraping, market reports). None of
  that serves the QueryKey vision — a private, local model of the
  people and commitments in *your* life. Carrying it forward would be
  dead weight.
- **The data model is now explicit.** The prototype encoded its model
  implicitly inside integration code; the pivot has a written
  `docs/data-model.md`. That is a prerequisite for the local-first
  markdown task model QueryKey is moving toward.

So: confirmed superior **for this product**. The prototype was a
different product that happened to share an AI bridge.

## Worth keeping (the actual point of this doc)

Most of `secretarybird-old/` is correctly discarded. A few things are
worth remembering — none require porting code today, only not
re-learning the lesson:

1. **The OpenClaw/WSL bridge was solved once already.** The prototype's
   `backend/openclaw/` had working gateway auto-start, health polling
   with retry, and bearer-token auto-read from
   `~/.openclaw/openclaw.json`. The Go server re-solved this in
   `internal/openclaw/{bridge,wsl}.go`. The forthcoming **Rust** server
   will have to solve it a third time — treat the Go implementation as
   the reference, and keep the agent interface model-agnostic so a
   future non-OpenClaw local agent (Gemma) drops in without touching
   callers.
2. **Messy-input handling is real and was exercised.** The prototype
   carried a `test-data/` corpus of deliberately messy real-world
   exports (partial CSVs, mixed date formats, raw JSON dumps, etc.).
   QueryKey's whole premise is ingesting unstructured streams, so that
   class of fixture is valuable. The corpus itself is generic enough to
   regenerate; the lesson is: **build ingestion against intentionally
   broken inputs from day one.**
3. **Confidence / epistemic-humility UX.** The "ask when unsure rather
   than guess" principle predates the pivot and is now a core QueryKey
   design decision. Keep it central, not a footnote.
4. **A single-binary installer is a feature, not a nicety.** The
   prototype's hardest UX problem was bootstrapping WSL + OpenClaw for
   non-technical users. Local-first only works if install is trivial —
   factor this into the Rust server's distribution story early.

## Explicitly not carried forward

External-tool connectors (Salesforce/Google/DB), competitor scraping,
social-feed monitoring, the data-pipeline builder, the Electron shell,
the Python/FastAPI backend, and the prototype's product framing. These
are out of scope per `queue.md` and `todo.md`.

---

*After this doc is committed, `secretarybird-old/` is deleted
(`queue.md` item 5). Its full contents remain recoverable from git
history if ever needed.*
