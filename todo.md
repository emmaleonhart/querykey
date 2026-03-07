# Sakuya Assistant - TODO

## Completed
- [x] Project architecture and directory structure
- [x] Planning documentation (planning/architecture.md)
- [x] Python backend core modules (file_organizer, excel_checker, data_processor)
- [x] Integration modules (salesforce, google_suite, databases, api_discovery)
- [x] Pipeline builder
- [x] OpenClaw bridge with Gateway HTTP API (SSE streaming)
- [x] FastAPI server with 35 REST + 1 WebSocket endpoint
- [x] Electron desktop app with Izayoi Sakuya theme (TypeScript)
- [x] TypeScript migration (6 renderer modules + shared types)
- [x] Full test suite (9 backend + 6 frontend files)
- [x] GitHub Actions CI workflow (lint → test → build)
- [x] electron-builder config for .exe installer + PyInstaller backend
- [x] Competitor analysis module with web scraping + Blue Ocean Strategy
- [x] OpenClaw gateway auto-start with retry logic (5 retries, health polling)
- [x] Dynamic OpenClaw detection (fresh detect on each message)
- [x] Backend port reuse detection (reuse existing on port 8000)
- [x] OpenClaw browser relay config (managed Chromium profile)
- [x] System tray integration with restart options
- [x] Installer architecture documentation (planning/installer.md)

## Security Notes
- [ ] **Brave Search API key**: May not be needed — browser relay handles web research. If bundled later, the key would be exposed in the binary. Use a proxy server or user-provided keys for production.

## Installer - Full Zero-to-Working Setup
See [planning/installer.md](planning/installer.md) for detailed architecture.

The .exe installer must handle everything for non-technical users (~2.5 GB total):
- [ ] **First-run dependency checker** in Electron (detect what's missing)
- [ ] **Install WSL** if not present (`wsl --install`, requires admin + reboot)
- [ ] **Install Ubuntu distro** in WSL (auto with WSL, but first boot takes 1-2 min)
- [ ] **Install Node.js** in WSL (nvm or apt)
- [ ] **Install OpenClaw** in WSL (`npm install -g openclaw`)
- [ ] **Install Chromium** in WSL (`apt install chromium-browser`)
- [ ] **Configure OpenClaw** (gateway, browser relay, managed browser profile)
- [ ] **OpenClaw onboarding UI** (LLM provider picker — embed in Electron or open terminal)
- [ ] **Setup state persistence** (resume after reboot, track completed steps)
- [ ] **Progress UI** during first-time setup (component-by-component progress)
- [ ] **Admin elevation handling** (UAC prompt for WSL install only)

## Next Steps
- [ ] Polish the Electron UI (error states, loading indicators)
- [ ] Set up Google OAuth2 credential flow in Electron
- [ ] Add Salesforce credential management UI
- [ ] Implement pipeline visual builder in the frontend
- [ ] Add data preview/visualization in the chat
- [ ] Package and test the .exe installer end-to-end
- [ ] Run full test suite and fix any failures
