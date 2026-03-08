@echo off
title Secretary Bird Assistant - Test Run
echo ========================================
echo   Secretary Bird Assistant - Test Run
echo   Your AI-powered business assistant.
echo ========================================
echo.

REM --- Check for Node.js ---
where node >NUL 2>NUL
if errorlevel 1 (
    echo [ERROR] Node.js is not installed or not in PATH.
    echo Please install Node.js from https://nodejs.org/
    pause
    exit /b 1
)

REM --- Find the correct Python (Immanuelle's installation with packages) ---
set "PYTHON="
if exist "%LOCALAPPDATA%\Programs\Python\Python313\python.exe" (
    set "PYTHON=%LOCALAPPDATA%\Programs\Python\Python313\python.exe"
) else (
    where python >NUL 2>NUL
    if errorlevel 1 (
        echo [ERROR] Python is not installed or not in PATH.
        echo Please install Python from https://python.org/
        pause
        exit /b 1
    )
    set "PYTHON=python"
)
echo Using Python: %PYTHON%

REM --- Install Node.js dependencies if needed ---
if not exist "node_modules" (
    echo [1/3] Installing Node.js dependencies...
    call npm install
    if errorlevel 1 (
        echo [ERROR] npm install failed.
        pause
        exit /b 1
    )
) else (
    echo [1/3] Node.js dependencies already installed.
)

REM --- Install Python dependencies if needed ---
%PYTHON% -c "import fastapi" >NUL 2>NUL
if errorlevel 1 (
    echo [2/3] Installing Python dependencies...
    %PYTHON% -m pip install -r requirements.txt
    if errorlevel 1 (
        echo [ERROR] pip install failed.
        pause
        exit /b 1
    )
) else (
    echo [2/3] Python dependencies already installed.
)

REM --- Launch the Electron app (which starts the Python backend automatically) ---
echo [3/3] Starting Secretary Bird Assistant...
echo.
echo   The Electron app will start the Python backend automatically.
echo   Close the Secretary Bird Assistant window to stop everything.
echo.
npx electron .

echo.
echo Secretary Bird Assistant closed.
pause
