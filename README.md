# Sakuya Assistant

A business data assistant themed after Izayoi Sakuya from Touhou Project, built for the Girls in Tech Hackathon 2026 at UBC Okanagan.

Sakuya Assistant provides an Electron desktop GUI wrapping OpenClaw with specialized business capabilities, and an easier installation for non-technical customers.

## Features

- **AI Chat with OpenClaw** - LLM-agnostic chat powered by OpenClaw Gateway (works with any LLM — cloud APIs, local models). Streams responses in real-time via WebSocket
- **Browser Automation** - OpenClaw Browser Relay lets the AI do web research, scrape data, and interact with websites via Chrome extension or managed Chromium
- **File Organization** - Scan, categorize, and reorganize messy directories with duplicate detection
- **Excel/Spreadsheet Error Checking** - Detect formula errors (#REF!, #VALUE!, etc.), circular references, inconsistent formulas, mixed data types, and ambiguous date formats
- **Data Processing** - Load, transform, profile, and export data in CSV, Excel, JSON, and Parquet formats
- **Salesforce Integration** - SOQL queries, record CRUD, object metadata
- **Google Suite Integration** - Google Sheets read/write/error-check, Google Drive file management
- **Database Connectivity** - PostgreSQL, MySQL, SQLite, and MongoDB support with schema introspection
- **API Discovery** - Parse OpenAPI/Swagger specs, store API configs, test endpoints
- **Competitor Analysis & Blue Ocean Strategy** - Web-scrape competitor websites, build a Strategy Canvas, apply the Four Actions Framework (Eliminate / Reduce / Raise / Create), and surface uncontested market opportunities
- **Data Pipeline Builder** - Chain source, transform, and destination steps into reusable pipelines

## Architecture

```
Electron Desktop App (Izayoi Sakuya themed UI, TypeScript)
        |  WebSocket / IPC
Python Backend (FastAPI, 35 REST endpoints + streaming chat)
        |
   Core          Integrations          Pipeline    OpenClaw Gateway
   - File Org    - Salesforce          - Builder   - HTTP API (SSE streaming)
   - Excel       - Google Suite                    - Browser Relay (CDP)
   - Data Proc   - Databases                       - Auth token management
                 - API Discovery
                 - Competitor Analysis
```

### OpenClaw Integration

OpenClaw runs inside WSL. The Electron app auto-starts the gateway process and the Python backend communicates with it via the OpenAI-compatible HTTP API:

- **Gateway** (`http://127.0.0.1:18789`) — auto-started on app launch with retry logic and health polling
- **Chat** (`POST /v1/chat/completions`) — SSE streaming, proxied through backend WebSocket to frontend
- **Browser Relay** — managed Chromium (isolated) or Chrome extension (uses existing tabs)
- **Auth** — bearer token auto-read from `~/.openclaw/openclaw.json` in WSL

## Getting Started

### Prerequisites

- Python 3.13+
- Node.js 18+
- WSL with Ubuntu (for OpenClaw)
- OpenClaw installed in WSL (`npm install -g openclaw`)
- OpenClaw gateway configured and onboarded (`openclaw onboard`)

### Quick Start

```bash
# Clone and install
git clone https://github.com/Emma-Leonhart/tojo-assistant.git
cd tojo-assistant
npm install
pip install -r requirements.txt

# Run (compiles TypeScript, starts backend + Electron)
npm start
```

Or use the batch file:
```bash
!testrun.bat
```

The Electron app automatically:
1. Starts the OpenClaw gateway in WSL (if not already running)
2. Starts the Python backend on port 8000
3. Connects via WebSocket for streaming chat

### Development

```bash
# Compile TypeScript only
npm run compile

# Run with DevTools open
npm run dev

# Run backend standalone
python -m backend.server
```

### Running Tests

```bash
# Backend tests (pytest)
npm run test:backend

# Frontend tests (vitest)
npm run test

# Both
npm run test:backend && npm run test
```

### Building the Installer

```bash
npm run build
```

This compiles TypeScript, packages the Python backend with PyInstaller, and produces a `.exe` installer via electron-builder + NSIS.

See [planning/installer.md](planning/installer.md) for the full installer architecture including WSL bootstrapping for non-technical users.

## Project Structure

```
tojo-assistant/
├── electron/
│   ├── src/                  # TypeScript source
│   │   ├── main.ts           # Main process (lifecycle, IPC, tray)
│   │   ├── preload.ts        # Context bridge
│   │   ├── shared/types.ts   # IPC channels, API interfaces
│   │   └── renderer/         # Frontend modules (6 files)
│   ├── compiled/             # tsc output (gitignored)
│   └── renderer/             # HTML/CSS + esbuild bundle
├── backend/
│   ├── server.py             # FastAPI entry (35 REST + 1 WebSocket)
│   ├── core/                 # File org, Excel checker, data processor
│   ├── integrations/         # Salesforce, Google Suite, databases, API, competitors
│   ├── pipeline/             # Data pipeline builder
│   └── openclaw/             # Gateway bridge + WSL utilities
├── tests/
│   ├── backend/              # 9 pytest files
│   └── frontend/             # 6 vitest files
├── scripts/build.js          # tsc + esbuild build orchestrator
├── planning/                 # Architecture + installer docs
├── .github/workflows/        # CI/CD (lint → test → build)
└── assets/                   # Sakuya avatar (PNG + ICO)
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop App | Electron 28+ |
| Frontend | TypeScript + esbuild (6 modular renderer files) |
| Backend | Python 3.13 + FastAPI (35 REST endpoints + WebSocket) |
| AI Engine | OpenClaw via WSL Gateway HTTP API (LLM-agnostic) |
| Browser Automation | OpenClaw Browser Relay (CDP protocol) |
| Testing | pytest (9 files) + vitest (6 files) + GitHub Actions |
| Installer | electron-builder (NSIS) + PyInstaller (backend) |

## Hackathon Strategy

### Scoring Rubric (Hack-Attack 2026 — /65 total)

| Category | Points | Our approach |
|---|---|---|
| **Technical Complexity & Implementation** | **/15** | **This is the big one.** 9 working skills (including web-scraping competitor analysis), LLM-agnostic architecture via OpenClaw with browser automation, Electron desktop app with TypeScript frontend, full test suite (15 test files), CI/CD pipeline, 35 REST endpoints. We're not demoing a mockup — this is a working product. |
| Innovation & Creativity | /10 | Blue Ocean Strategy analysis — web-scrape competitors, build a Strategy Canvas, auto-generate the Four Actions Framework. No other hackathon project replaces a business consultant. |
| Entrepreneurial Value & Business Impact | /10 | The competitor analysis feature alone justifies a subscription. Businesses pay consultants thousands for the kind of strategic output Sakuya generates from a URL list. |
| Design & User Experience | /10 | Izayoi Sakuya themed dark UI with silver-blue accents. Chat-based interface with real-time streaming. System tray integration. One-click installer for non-technical users. |
| Presentation | /5 |  |
| Q&A Session | /5 |  |
| Exec Check-in | /5 | GitHub with full commit history, this README, todo.md with project plan, architecture docs. |

### Key talking points for the pitch
- **Technical depth** is our strongest card — working prototype, not slides
- The AI doesn't just chat — it does **web research via browser automation** and **generates real analysis**
- **LLM-agnostic** means the customer saves money — use what you already pay for (OpenAI, Anthropic, local models, anything)
- **Competitor Analysis** is the killer feature — it doesn't just replace a database person, it replaces a strategy consultant. Give it your competitors' URLs and it delivers a full Blue Ocean Strategy analysis
- Blue Ocean Strategy is a proven framework used by Fortune 500 companies — Sakuya automates it for SMBs who can't afford McKinsey
- **Zero-to-working installer** — one `.exe` sets up everything including WSL, OpenClaw, and browser automation

## License

MIT
