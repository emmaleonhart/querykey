@echo off
setlocal enabledelayedexpansion
title QueryKey
echo ========================================
echo   QueryKey
echo   Local-first PRM / rationalist social net
echo ========================================
echo.

cd /d "%~dp0"

REM --- Check for Rust/Cargo ---
where cargo >NUL 2>NUL
if errorlevel 1 (
    echo [ERROR] Rust/Cargo is not installed or not in PATH.
    echo Install it from: https://rustup.rs
    pause
    exit /b 1
)
echo [OK] Cargo found:
cargo --version

REM --- Check for Node/npm (Electron desktop UI) ---
where npm >NUL 2>NUL
if errorlevel 1 (
    echo [ERROR] Node.js / npm is not installed or not in PATH.
    echo Install it from: https://nodejs.org
    pause
    exit /b 1
)
echo [OK] npm found:
npm --version

REM --- Check for WSL + Ubuntu (local AI agent gateway) ---
wsl -d Ubuntu -- echo "WSL OK" >NUL 2>NUL
if errorlevel 1 (
    echo [WARN] WSL Ubuntu not found - the local agent gateway will not be available.
) else (
    echo [OK] WSL Ubuntu found
)
echo.

REM --- Build the Rust server (with Loca/SutraDB graph store) ---
REM Pre-build so the Electron app's first-run is instant; the app
REM itself also builds it if missing (server lifecycle lives in
REM app-electron/main.js now).
echo [1/4] Building Rust server (--features loca)...
cd /d "%~dp0server"
cargo build --features loca
if errorlevel 1 (
    echo [WARN] Loca build failed (is ..\..\SutraDB present?). Falling back
    echo        to the in-memory graph build...
    cargo build
    if errorlevel 1 (
        echo [ERROR] Server build failed.
        pause
        exit /b 1
    )
)
echo [OK] Built server\target\debug\querykey-server.exe

REM --- Electron app dependencies ---
echo [2/4] Installing Electron app dependencies...
cd /d "%~dp0app-electron"
if not exist "node_modules" (
    call npm install --no-audit --no-fund
    if errorlevel 1 (
        echo [ERROR] npm install failed.
        pause
        exit /b 1
    )
)
echo [OK] app-electron dependencies ready
echo.

REM --- Start the local agent gateway in WSL (OpenClaw bridge today) ---
echo [3/4] Starting local agent gateway...

REM Clean stale lock files (the fix for the old stuck-gateway problem)
wsl -d Ubuntu -- bash -c "pkill -f openclaw-gateway 2>/dev/null; pkill -f 'openclaw gateway' 2>/dev/null; rm -f /tmp/openclaw-*/gateway.*.lock; true" >NUL 2>NUL

REM Check if already running
curl -s -o NUL -w "%%{http_code}" http://127.0.0.1:18789/ --connect-timeout 2 2>NUL | findstr "200" >NUL 2>NUL
if not errorlevel 1 (
    echo [OK] Agent gateway already running on port 18789
    goto :gateway_done
)

start "Agent Gateway" /min wsl -d Ubuntu -- bash -lc "openclaw gateway"

echo Waiting for gateway to start...
set TRIES=0
:gateway_wait
if !TRIES! GEQ 15 goto :gateway_timeout
timeout /t 1 /nobreak >NUL
curl -s -o NUL -w "%%{http_code}" http://127.0.0.1:18789/ --connect-timeout 1 2>NUL | findstr "200" >NUL 2>NUL
if not errorlevel 1 (
    echo [OK] Agent gateway started on port 18789
    goto :gateway_done
)
set /a TRIES+=1
goto :gateway_wait

:gateway_timeout
echo [WARN] Agent gateway did not start within 15 seconds.
echo        Server will work without it, but AI chat needs the gateway.
echo        Try manually in WSL: openclaw gateway

:gateway_done
echo.

REM --- Launch the Electron app (it spawns + health-polls + tears
REM down the Rust server itself; do NOT start the server here too,
REM or two instances would fight over port 8000). ---
echo [4/4] Launching QueryKey desktop (Electron manages the server)...
echo.
echo   Server:    http://127.0.0.1:8000
echo   Health:    http://127.0.0.1:8000/health
echo   Agent:     http://127.0.0.1:18789
echo   WebSocket: ws://127.0.0.1:8000/ws/chat
echo ========================================
echo.
echo Close the app window to stop everything.
echo.
cd /d "%~dp0app-electron"
call npm start

REM --- Cleanup: kill everything on exit ---
echo.
echo Stopping all processes...
taskkill /im querykey-server.exe /f >NUL 2>NUL
wsl -d Ubuntu -- bash -c "pkill -f openclaw-gateway 2>/dev/null; pkill -f 'openclaw gateway' 2>/dev/null; rm -f /tmp/openclaw-*/gateway.*.lock; true" >NUL 2>NUL
taskkill /fi "WINDOWTITLE eq Agent Gateway" >NUL 2>NUL
echo Done.
pause
