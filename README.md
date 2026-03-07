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
- **Data Pipeline Builder** - Chain source, transform, and destination steps into reusable pipelines
- **OpenClaw Bridge** - WSL-aware bridge to OpenClaw (LLM-agnostic CLI, works with any LLM including local models)

## Architecture

```
Electron Desktop App (Kirumi Tojo themed UI)
        |  IPC / HTTP
Python Backend (FastAPI)
        |
   Core          Integrations       Pipeline    OpenClaw
   - File Org    - Salesforce        - Builder   - WSL Bridge
   - Excel       - Google Suite
   - Data Proc   - Databases
                 - API Discovery
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

# Run the backend server
python -m backend.server

# Run the Electron app
npm start
```

### Running Tests

```bash
pytest tests/ -v
```

### Building the Installer

```bash
npm run build
```

This produces a `.exe` installer via electron-builder.

## Project Structure

```
tojo-assistant/
├── electron/              # Electron desktop app
│   ├── main.js            # Main process
│   ├── preload.js         # Context bridge
│   └── renderer/          # Frontend (HTML/CSS/JS)
├── backend/               # Python backend
│   ├── server.py          # FastAPI entry point
│   ├── core/              # File org, Excel checker, data processor
│   ├── integrations/      # Salesforce, Google Suite, databases, API discovery
│   ├── pipeline/          # Data pipeline builder
│   └── openclaw/          # OpenClaw bridge + WSL manager
├── tests/                 # pytest test suite
├── .github/workflows/     # CI/CD
├── assets/                # Shared assets (Kirumi Tojo avatar)
└── planning/              # Architecture docs
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop App | Electron 28+ |
| Frontend | HTML/CSS/JS (vanilla) |
| Backend | Python 3.13 + FastAPI |
| AI Engine | OpenClaw (LLM-agnostic CLI) via WSL |
| Testing | pytest + GitHub Actions |
| Installer | electron-builder (NSIS) |

## License

MIT
