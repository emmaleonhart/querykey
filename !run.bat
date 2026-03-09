@echo off
title Secretarybird Server
echo ========================================
echo   Secretarybird - Go Server
echo   AI secretary for team coordination
echo ========================================
echo.

cd /d "%~dp0"

REM --- Check for Go ---
where go >NUL 2>NUL
if errorlevel 1 (
    REM Try the default install path
    if exist "C:\Program Files\Go\bin\go.exe" (
        set "PATH=%PATH%;C:\Program Files\Go\bin"
    ) else (
        echo [ERROR] Go is not installed or not in PATH.
        echo Install it with: winget install GoLang.Go
        pause
        exit /b 1
    )
)
echo [OK] Go found:
go version
echo.

REM --- Build the server ---
echo [1/2] Building server...
cd server
go build -o secretarybird.exe ./cmd/secretarybird/
if errorlevel 1 (
    echo [ERROR] Build failed.
    pause
    exit /b 1
)
echo [OK] Built server\secretarybird.exe
echo.

REM --- Show configuration ---
echo [2/2] Starting server...
echo.
echo   Server:   http://127.0.0.1:8000
echo   Health:   http://127.0.0.1:8000/health
echo   Status:   http://127.0.0.1:8000/api/status
echo   WebSocket: ws://127.0.0.1:8000/ws/chat
echo.
echo   OpenClaw gateway expected at http://127.0.0.1:18789
echo   Start it in WSL with: openclaw gateway
echo.
echo   Set DISCORD_TOKEN to enable the Discord bot.
echo   Set FUSEKI_URL to connect to Apache Jena Fuseki.
echo.
echo   Press Ctrl+C to stop.
echo ========================================
echo.

REM --- Run ---
secretarybird.exe

echo.
echo Server stopped.
pause
