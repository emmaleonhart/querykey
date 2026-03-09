@echo off
setlocal enabledelayedexpansion
title Secretarybird
echo ========================================
echo   Secretarybird
echo   AI secretary for team coordination
echo ========================================
echo.

cd /d "%~dp0"

REM --- Check for Go ---
where go >NUL 2>NUL
if errorlevel 1 (
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

REM --- Check for Flutter ---
where flutter >NUL 2>NUL
if errorlevel 1 (
    echo [ERROR] Flutter is not installed or not in PATH.
    pause
    exit /b 1
)
echo [OK] Flutter found:
flutter --version 2>NUL | findstr /C:"Flutter"

REM --- Check for WSL + Ubuntu ---
wsl -d Ubuntu -- echo "WSL OK" >NUL 2>NUL
if errorlevel 1 (
    echo [WARN] WSL Ubuntu not found - OpenClaw will not be available.
) else (
    echo [OK] WSL Ubuntu found
)
echo.

REM --- Build the Go server ---
echo [1/4] Building Go server...
cd /d "%~dp0server"
go build -o secretarybird.exe ./cmd/secretarybird/
if errorlevel 1 (
    echo [ERROR] Server build failed.
    pause
    exit /b 1
)
echo [OK] Built server\secretarybird.exe

REM --- Get Flutter dependencies ---
echo [2/4] Getting Flutter dependencies...
cd /d "%~dp0app"
call flutter pub get >NUL 2>NUL
echo [OK] Flutter dependencies ready
echo.

REM --- Start OpenClaw gateway in WSL ---
echo [3/4] Starting OpenClaw gateway...

REM Clean stale lock files (the fix for the old stuck-gateway problem)
wsl -d Ubuntu -- bash -c "pkill -f openclaw-gateway 2>/dev/null; pkill -f 'openclaw gateway' 2>/dev/null; rm -f /tmp/openclaw-*/gateway.*.lock; true" >NUL 2>NUL

REM Check if already running
curl -s -o NUL -w "%%{http_code}" http://127.0.0.1:18789/ --connect-timeout 2 2>NUL | findstr "200" >NUL 2>NUL
if not errorlevel 1 (
    echo [OK] OpenClaw gateway already running on port 18789
    goto :openclaw_done
)

REM Start the gateway in a minimized window
start "OpenClaw Gateway" /min wsl -d Ubuntu -- bash -lc "openclaw gateway"

REM Wait up to 15 seconds for it to come up
echo Waiting for gateway to start...
set TRIES=0
:openclaw_wait
if !TRIES! GEQ 15 goto :openclaw_timeout
timeout /t 1 /nobreak >NUL
curl -s -o NUL -w "%%{http_code}" http://127.0.0.1:18789/ --connect-timeout 1 2>NUL | findstr "200" >NUL 2>NUL
if not errorlevel 1 (
    echo [OK] OpenClaw gateway started on port 18789
    goto :openclaw_done
)
set /a TRIES+=1
goto :openclaw_wait

:openclaw_timeout
echo [WARN] OpenClaw gateway did not start within 15 seconds.
echo        Server will work without it, but AI chat needs the gateway.
echo        Try manually in WSL: openclaw gateway

:openclaw_done
echo.

REM --- Start the Go server ---
echo [4/4] Starting Secretarybird server...
echo.
echo   Server:    http://127.0.0.1:8000
echo   Health:    http://127.0.0.1:8000/health
echo   OpenClaw:  http://127.0.0.1:18789
echo   WebSocket: ws://127.0.0.1:8000/ws/chat
echo ========================================
echo.

cd /d "%~dp0server"
start "Secretarybird Server" /min secretarybird.exe
timeout /t 2 /nobreak >NUL

REM --- Launch Flutter app ---
echo Launching Flutter app...
echo Close the app window to stop everything.
echo.
cd /d "%~dp0app"
call flutter run -d windows

REM --- Cleanup: kill everything on exit ---
echo.
echo Stopping all processes...
taskkill /fi "WINDOWTITLE eq Secretarybird Server" >NUL 2>NUL
taskkill /im secretarybird.exe /f >NUL 2>NUL
wsl -d Ubuntu -- bash -c "pkill -f openclaw-gateway 2>/dev/null; pkill -f 'openclaw gateway' 2>/dev/null; rm -f /tmp/openclaw-*/gateway.*.lock; true" >NUL 2>NUL
taskkill /fi "WINDOWTITLE eq OpenClaw Gateway" >NUL 2>NUL
echo Done.
pause
