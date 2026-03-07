# tojo-assistant

## Workflow Rules
- **Commit early and often.** Every meaningful change gets a commit with a clear message explaining *why*, not just what.
- **Do not enter planning-only modes.** All thinking must produce files and commits. If scope is unclear, create a `planning/` directory and write `.md` files there instead of using an internal planning mode.
- **Keep this file up to date.** As the project takes shape, record architectural decisions, conventions, and anything needed to work effectively in this repo.
- **Update README.md regularly.** It should always reflect the current state of the project for human readers.

## Project Description
Sakuya Assistant is an Izayoi Sakuya-themed business data assistant for the Girls in Tech Hackathon 2026 at UBC Okanagan. It wraps OpenClaw (an LLM-agnostic CLI that works with any LLM including local models) in an Electron desktop GUI with business-specific capabilities: file organization, Excel/Sheets error checking, data processing, Salesforce/Google Suite/database integrations, API discovery, data pipeline building, and competitor analysis with Blue Ocean Strategy.

## Architecture and Conventions
- **Backend**: Python 3.13 + FastAPI (`backend/` directory)
  - `backend/server.py` - FastAPI entry point (REST + WebSocket)
  - `backend/core/` - File organizer, Excel checker, Data processor
  - `backend/integrations/` - Salesforce, Google Suite, Databases, API Discovery, Competitor Analysis
  - `backend/pipeline/` - Data pipeline builder
  - `backend/openclaw/` - OpenClaw bridge + WSL manager
- **Frontend**: Electron 28+ with vanilla HTML/CSS/JS (`electron/` directory)
  - Dark theme with silver-blue accents (Izayoi Sakuya color palette)
  - Chat-based interface with sidebar navigation
- **Tests**: pytest (`tests/backend/`)
- **CI**: GitHub Actions (`.github/workflows/ci.yml`)
- **Installer**: electron-builder produces `.exe` via NSIS
- Use `python` (not `python3`) on this Windows system
