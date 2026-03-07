# Tojo Assistant

A business data assistant themed after Kirumi Tojo from Danganronpa, built for the Girls in Tech Hackathon 2026 at UBC Okanagan.

Tojo Assistant provides an Electron desktop GUI wrapping OpenClaw with specialized business capabilities, and an easier installaton for non-technical customers.

## Features

- **File Organization** - Scan, categorize, and reorganize messy directories with duplicate detection
- **Excel/Spreadsheet Error Checking** - Detect formula errors (#REF!, #VALUE!, etc.), circular references, inconsistent formulas, mixed data types, and ambiguous date formats
- **Data Processing** - Load, transform, profile, and export data in CSV, Excel, JSON, and Parquet formats
- **Salesforce Integration** - SOQL queries, record CRUD, object metadata
- **Google Suite Integration** - Google Sheets read/write/error-check, Google Drive file management
- **Database Connectivity** - PostgreSQL, MySQL, SQLite, and MongoDB support with schema introspection
- **API Discovery** - Parse OpenAPI/Swagger specs, store API configs, test endpoints
- **Competitor Analysis & Blue Ocean Strategy** - Web-scrape competitor websites, build a Strategy Canvas, apply the Four Actions Framework (Eliminate / Reduce / Raise / Create), and surface uncontested market opportunities. Turns Tojo from a data tool into a strategic business consultant
- **Data Pipeline Builder** - Chain source, transform, and destination steps into reusable pipelines
- **OpenClaw Bridge** - WSL-aware bridge to OpenClaw (LLM-agnostic CLI, works with any LLM including local models)

## Architecture

```
Electron Desktop App (Kirumi Tojo themed UI)
        |  IPC / HTTP
Python Backend (FastAPI)
        |
   Core          Integrations          Pipeline    OpenClaw
   - File Org    - Salesforce          - Builder   - WSL Bridge
   - Excel       - Google Suite
   - Data Proc   - Databases
                 - API Discovery
                 - Competitor Analysis
```

## Getting Started

### Prerequisites

- Python 3.13+
- Node.js 18+
- WSL (for OpenClaw bridge on Windows)

### Setup

```bash
# Install Python dependencies
pip install -r requirements.txt

# Install Node.js dependencies
npm install

# Compile TypeScript (required before first run)
npm run compile

# Run the backend server
python -m backend.server

# Run the Electron app (auto-compiles TypeScript)
npm start
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

This compiles TypeScript, bundles the renderer, and produces a `.exe` installer via electron-builder.

## Project Structure

```
tojo-assistant/
├── electron/
│   ├── src/               # TypeScript source
│   │   ├── main.ts        # Main process
│   │   ├── preload.ts     # Context bridge
│   │   ├── shared/        # Shared types (IPC channels, API interfaces)
│   │   └── renderer/      # Renderer modules (app, chat, websocket, etc.)
│   ├── compiled/          # tsc output (main + preload, gitignored)
│   └── renderer/          # Frontend (HTML/CSS + esbuild bundle)
├── backend/               # Python backend
│   ├── server.py          # FastAPI entry point
│   ├── core/              # File org, Excel checker, data processor
│   ├── integrations/      # Salesforce, Google Suite, databases, API discovery
│   ├── pipeline/          # Data pipeline builder
│   └── openclaw/          # OpenClaw bridge + WSL manager
├── tests/
│   ├── backend/           # pytest test suite
│   └── frontend/          # vitest test suite
├── scripts/               # Build scripts (esbuild + tsc)
├── .github/workflows/     # CI/CD
├── assets/                # Shared assets (Kirumi Tojo avatar)
└── planning/              # Architecture docs
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop App | Electron 28+ |
| Frontend | TypeScript + esbuild (modular architecture) |
| Backend | Python 3.13 + FastAPI |
| AI Engine | OpenClaw (LLM-agnostic CLI) via WSL |
| Testing | pytest (backend) + vitest (frontend) + GitHub Actions |
| Installer | electron-builder (NSIS) |


## Hackathon Strategy

### Scoring Rubric (Hack-Attack 2026 — /65 total)

| Category | Points | Our approach |
|---|---|---|
| **Technical Complexity & Implementation** | **/15** | **This is the big one.** 9 working skills (including web-scraping competitor analysis), LLM-agnostic architecture via OpenClaw, Electron desktop app, full test suite (60+ tests), CI/CD pipeline, script generation. We're not demoing a mockup — this is a working product. |
| Innovation & Creativity | /10 | Blue Ocean Strategy analysis — web-scrape competitors, build a Strategy Canvas, auto-generate the Four Actions Framework. No other hackathon project replaces a business consultant. |
| Entrepreneurial Value & Business Impact | /10 | The competitor analysis feature alone justifies a subscription. Businesses pay consultants thousands for the kind of strategic output Tojo generates from a URL list. |
| Design & User Experience | /10 |  |
| Presentation | /5 |  |
| Q&A Session | /5 |  |
| Exec Check-in | /5 | GitHub with full commit history, this README, todo.md with project plan. |

### Key talking points for the pitch
- **Technical depth** is our strongest card — working prototype, not slides
- The AI doesn't just chat — it **generates real code** (scripts, automations)
- **LLM-agnostic** means the customer saves money — use what you already pay for
- **Competitor Analysis** is the killer feature — it doesn't just replace a database person, it replaces a strategy consultant. Give it your competitors' URLs and it delivers a full Blue Ocean Strategy analysis: Strategy Canvas, Four Actions Framework, and actionable recommendations for uncontested market space
- Blue Ocean Strategy is a proven framework used by Fortune 500 companies — Tojo automates it for SMBs who can't afford McKinsey



## License

MIT
