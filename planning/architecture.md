# Sakuya Assistant - Architecture

## Overview
Sakuya Assistant is a business data assistant themed after Izayoi Sakuya from Touhou Project,
built for the Girls in Tech Hackathon 2026 at UBC Okanagan.

It provides an Electron desktop GUI that wraps OpenClaw (LLM-agnostic CLI) with
specialized business capabilities: file organization, Salesforce integration,
Excel/Google Sheets processing & error checking, database connectivity,
dynamic API discovery for building data pipelines, and competitor analysis
with Blue Ocean Strategy (web scraping + strategic framework analysis).

## System Architecture

```
+-------------------------------+
|    Electron Desktop App       |
|   (Izayoi Sakuya themed UI)   |
|   TypeScript + esbuild        |
|   - Chat interface            |
|   - Sidebar navigation        |
|   - System tray integration   |
+----------+--------------------+
           | IPC / WebSocket / HTTP
+----------v--------------------+
|   Python Backend (FastAPI)    |
|   - 35 REST endpoints         |
|   - WebSocket streaming chat   |
|   - OpenClaw bridge (HTTP)     |
+----------+---------------------+
           |
    +------+------+------+------+------+
    |      |      |      |      |      |
  Core  Integrations  Pipeline  OpenClaw  WSL
    |      |            |        Bridge   (Gateway)
    |      +-- Salesforce          |
    |      +-- Google Suite        +-- OpenAI-compatible HTTP API
    |      +-- Databases           +-- Browser Relay (Chromium/Chrome)
    |      +-- API Discovery       +-- Auth token (auto-read from config)
    |      +-- Competitor Analysis
    |
    +-- File Organizer
    +-- Excel/Sheets Error Checker
    +-- Data Processor
```

## OpenClaw Integration

OpenClaw runs inside WSL and provides the LLM backend. Sakuya connects via HTTP:

```
Electron (Windows)
  |
  +-- spawns: wsl -d Ubuntu -e bash -lc 'openclaw gateway'
  |     (auto-start with retry logic, health polling)
  |
  +-- spawns: python -m backend.server
        |
        +-- OpenClawBridge.detect()   → GET http://127.0.0.1:18789/
        +-- OpenClawBridge.chat_stream() → POST /v1/chat/completions (SSE)
        |     (OpenAI-compatible, streams to frontend via WebSocket)
        |
        +-- Gateway auth token auto-read from ~/.openclaw/openclaw.json via WSL
```

**Browser Relay**: OpenClaw can control a browser for web research via:
- **Managed browser** (isolated Chromium in WSL) — preferred for bundled installs
- **Chrome extension relay** — uses existing Chrome with extension attached

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop App | Electron 28+ |
| Frontend | TypeScript, esbuild (modular: 6 renderer modules) |
| Backend | Python 3.13 + FastAPI |
| IPC | HTTP REST + WebSocket (streaming) |
| AI Engine | OpenClaw (LLM-agnostic) via WSL Gateway HTTP API |
| Browser Automation | OpenClaw Browser Relay (CDP protocol) |
| Testing | pytest (backend, 9 files) + vitest (frontend, 6 files) |
| CI/CD | GitHub Actions (lint + test + build installer) |
| Installer | electron-builder (NSIS .exe) + PyInstaller (backend) |

## Directory Structure

```
tojo-assistant/
├── electron/
│   ├── src/                  # TypeScript source
│   │   ├── main.ts           # Main process (lifecycle, IPC, tray)
│   │   ├── preload.ts        # Context bridge (safe API for renderer)
│   │   ├── shared/
│   │   │   └── types.ts      # IPC channels, API interfaces
│   │   └── renderer/         # Frontend modules
│   │       ├── app.ts        # Entry point, wires everything
│   │       ├── chat.ts       # Chat UI, message rendering, streaming
│   │       ├── websocket.ts  # WebSocket with auto-reconnect
│   │       ├── ui.ts         # Connection status, modals
│   │       ├── actions.ts    # Sidebar/quick action routing
│   │       └── markdown.ts   # Lightweight Markdown renderer
│   ├── compiled/             # tsc output (main + preload, gitignored)
│   └── renderer/             # Frontend assets
│       ├── index.html        # Main UI (Izayoi Sakuya theme)
│       ├── styles.css        # Dark theme + gold accents
│       └── app.js            # esbuild bundle (IIFE, gitignored)
├── backend/
│   ├── server.py             # FastAPI entry point (REST + WebSocket)
│   ├── core/
│   │   ├── file_organizer.py # Scan, categorize, deduplicate
│   │   ├── excel_checker.py  # Formula errors, type mismatches
│   │   └── data_processor.py # Load, transform, profile, export
│   ├── integrations/
│   │   ├── salesforce.py     # SOQL, CRUD, metadata
│   │   ├── google_suite.py   # Sheets/Drive (OAuth2-ready)
│   │   ├── databases.py      # PostgreSQL, MySQL, SQLite, MongoDB
│   │   ├── api_discovery.py  # OpenAPI/Swagger parsing
│   │   └── competitor_analysis.py  # Web scraping + Blue Ocean Strategy
│   ├── pipeline/
│   │   └── builder.py        # Source → Transform → Destination chains
│   └── openclaw/
│       └── bridge.py         # Gateway HTTP client + WSL utilities
├── tests/
│   ├── backend/              # 9 pytest files
│   └── frontend/             # 6 vitest files
├── scripts/
│   └── build.js              # tsc + esbuild build orchestrator
├── planning/                 # This file
├── .github/workflows/
│   └── ci.yml                # Lint → Test → Build installer
├── assets/                   # tojo-avatar.ico, tojo-avatar.png
├── build_backend.py          # PyInstaller wrapper for CI
├── !testrun.bat              # Quick-start script for development
├── package.json              # npm config + electron-builder
├── requirements.txt          # Python dependencies
└── tsconfig.json             # TypeScript config (main + preload)
```

## Backend API Endpoints (35 REST + 1 WebSocket)

| Group | Method | Path | Purpose |
|-------|--------|------|---------|
| System | GET | `/health` | Health check + uptime |
| System | GET | `/api/openclaw/status` | OpenClaw gateway availability |
| Chat | WS | `/ws/chat` | Streaming chat (routes to OpenClaw or handlers) |
| Files | POST | `/api/files/scan` | Directory categorization |
| Files | POST | `/api/files/organize` | Move files to folders |
| Files | POST | `/api/files/duplicates` | Duplicate detection |
| Excel | POST | `/api/excel/check` | Check file at path |
| Excel | POST | `/api/excel/upload-check` | Upload and check |
| Data | POST | `/api/data/load` | Load + preview |
| Data | POST | `/api/data/profile` | Statistics + distributions |
| Data | POST | `/api/data/transform` | Apply transformations |
| Salesforce | POST | `/api/salesforce/query` | SOQL queries |
| Salesforce | POST | `/api/salesforce/describe` | Object metadata |
| Salesforce | POST | `/api/salesforce/record` | Create/update/delete |
| Database | POST | `/api/database/query` | SQL execution |
| Database | POST | `/api/database/schema` | Table introspection |
| Database | POST | `/api/database/test-connection` | Connectivity test |
| API | POST | `/api/discover` | Parse OpenAPI specs |
| API | POST | `/api/discover/test` | Test endpoint |
| Competitors | POST | `/api/competitors/analyze` | Full Blue Ocean analysis |
| Competitors | POST | `/api/competitors/scrape` | Single competitor scrape |
| Competitors | GET | `/api/competitors/industries` | Supported industries |
| Competitors | GET | `/api/competitors/reports` | Saved reports |
| Competitors | POST | `/api/competitors/save` | Save report |
| Pipeline | POST | `/api/pipeline/run` | Execute pipeline |
| Pipeline | POST | `/api/pipeline/validate` | Validate definition |
| Pipeline | GET | `/api/pipeline/list` | List saved pipelines |
| Pipeline | POST | `/api/pipeline/save` | Save for reuse |

## Key Design Decisions

1. **Electron + Python**: Electron for accessible GUI, Python for data processing
   power and library ecosystem (pandas, openpyxl, simple-salesforce, etc.)

2. **FastAPI backend**: Async-capable, auto-generates OpenAPI docs, WebSocket
   support for streaming OpenClaw output to the UI.

3. **OpenClaw Gateway HTTP API**: Instead of spawning OpenClaw as a subprocess
   (which doesn't work — it's a TUI), we connect to the Gateway's
   OpenAI-compatible HTTP endpoint (`POST /v1/chat/completions`). The gateway
   handles LLM communication, session management, and tool execution.

4. **WSL bridge**: OpenClaw runs in WSL. Electron auto-starts the gateway
   process via `wsl -d Ubuntu -e bash -lc 'openclaw gateway'` with retry
   logic and health polling. The Python backend reads auth tokens from
   the WSL-side OpenClaw config.

5. **Browser Relay**: OpenClaw can do web research via browser automation
   (CDP protocol). The managed browser mode runs an isolated Chromium
   instance; the Chrome extension relay uses existing Chrome tabs.

6. **Plugin-style integrations**: Each integration (Salesforce, G-Suite, DB)
   is a self-contained module with a common interface for the pipeline builder.

7. **Dynamic OpenClaw detection**: The backend does a fresh `bridge.detect()`
   on each chat message, so the gateway can start after the backend without
   requiring a restart.

## Data Flow

1. User sends message via Electron chat UI
2. Frontend sends JSON over WebSocket to `ws://127.0.0.1:8000/ws/chat`
3. Backend routes to appropriate handler based on context:
   - Default → OpenClaw `chat_stream()` (SSE streaming via gateway)
   - File/Excel/Data/Pipeline/Competitor → dedicated handler modules
4. Responses stream back chunk-by-chunk via WebSocket
5. Frontend renders chunks in real-time with typing indicator
6. OpenClaw can use browser relay for web research during chat

## Build Pipeline

```
Development:
  npm run compile → tsc (main.ts, preload.ts) + esbuild (renderer modules)
  npm start       → compile + electron .

Production:
  build_backend.py → PyInstaller → dist/backend/tojo-backend/tojo-backend.exe
  npm run build    → compile:prod + electron-builder → Sakuya-Assistant-Setup.exe
```
