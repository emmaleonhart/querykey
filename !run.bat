@echo off
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
echo.

REM --- Build the Go server ---
echo [1/3] Building Go server...
cd /d "%~dp0server"
go build -o secretarybird.exe ./cmd/secretarybird/
if errorlevel 1 (
    echo [ERROR] Server build failed.
    pause
    exit /b 1
)
echo [OK] Built server\secretarybird.exe
echo.

REM --- Get Flutter dependencies ---
echo [2/3] Getting Flutter dependencies...
cd /d "%~dp0app"
call flutter pub get >NUL 2>NUL
echo [OK] Flutter dependencies ready
echo.

REM --- Start the Go server in background ---
echo [3/3] Starting...
echo.
echo   Server:    http://127.0.0.1:8000
echo   Health:    http://127.0.0.1:8000/health
echo   WebSocket: ws://127.0.0.1:8000/ws/chat
echo.
echo   OpenClaw:  start in WSL with "openclaw gateway"
echo   Discord:   set DISCORD_TOKEN env var to enable
echo ========================================
echo.

cd /d "%~dp0server"
start "Secretarybird Server" /min secretarybird.exe

REM --- Wait a moment for the server to start ---
timeout /t 2 /nobreak >NUL

REM --- Launch Flutter app ---
echo Launching Flutter app (Windows)...
echo Close the app window to stop everything.
echo.
cd /d "%~dp0app"
call flutter run -d windows

REM --- When Flutter exits, kill the server ---
echo.
echo Stopping server...
taskkill /fi "WINDOWTITLE eq Secretarybird Server" >NUL 2>NUL
taskkill /im secretarybird.exe /f >NUL 2>NUL
echo Done.
pause
