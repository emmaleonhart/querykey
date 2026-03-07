const { app, BrowserWindow, ipcMain, dialog, Tray, Menu, nativeImage } = require('electron');
const path = require('path');
const { spawn } = require('child_process');

// ── Paths ──────────────────────────────────────────────────────────────────────
const ROOT_DIR = path.join(__dirname, '..');
const ASSETS_DIR = path.join(ROOT_DIR, 'assets');
const ICON_PATH = path.join(ASSETS_DIR, 'tojo-avatar.png');
const PRELOAD_PATH = path.join(__dirname, 'preload.js');
const RENDERER_PATH = path.join(__dirname, 'renderer', 'index.html');
const BACKEND_DIR = path.join(ROOT_DIR, 'backend');

// ── Packaged mode detection ─────────────────────────────────────────────────
// When built with electron-builder, app.isPackaged is true and the PyInstaller
// backend exe lives in resources/backend/tojo-backend/tojo-backend.exe
const IS_PACKAGED = app.isPackaged;
const PACKAGED_BACKEND_EXE = IS_PACKAGED
  ? path.join(process.resourcesPath, 'backend', 'tojo-backend', 'tojo-backend.exe')
  : null;

// ── State ──────────────────────────────────────────────────────────────────────
let mainWindow = null;
let tray = null;
let backendProcess = null;
let backendReady = false;

// ── Backend lifecycle ──────────────────────────────────────────────────────────

function startBackend() {
  let cmd, args, cwd;

  if (IS_PACKAGED && PACKAGED_BACKEND_EXE) {
    // Packaged mode: run the PyInstaller-built executable
    const fs = require('fs');
    if (fs.existsSync(PACKAGED_BACKEND_EXE)) {
      cmd = PACKAGED_BACKEND_EXE;
      args = [];
      cwd = path.dirname(PACKAGED_BACKEND_EXE);
      console.log('[backend] Using packaged exe:', cmd);
    } else {
      console.warn('[backend] Packaged exe not found, falling back to Python');
      cmd = process.platform === 'win32' ? 'python' : 'python3';
      args = [path.join(BACKEND_DIR, 'server.py')];
      cwd = BACKEND_DIR;
    }
  } else {
    // Dev mode: run via Python interpreter
    cmd = process.platform === 'win32' ? 'python' : 'python3';
    args = [path.join(BACKEND_DIR, 'server.py')];
    cwd = BACKEND_DIR;
  }

  try {
    backendProcess = spawn(cmd, args, {
      cwd: cwd,
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, PYTHONUNBUFFERED: '1' },
    });

    backendProcess.stdout.on('data', (data) => {
      const output = data.toString().trim();
      console.log(`[backend] ${output}`);
      if (output.includes('Application startup complete') || output.includes('Uvicorn running')) {
        backendReady = true;
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send('backend-status', { connected: true });
        }
      }
    });

    backendProcess.stderr.on('data', (data) => {
      const output = data.toString().trim();
      // Uvicorn prints normal startup info to stderr
      console.log(`[backend:err] ${output}`);
      if (output.includes('Application startup complete') || output.includes('Uvicorn running')) {
        backendReady = true;
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send('backend-status', { connected: true });
        }
      }
    });

    backendProcess.on('error', (err) => {
      console.error('[backend] Failed to start:', err.message);
      backendReady = false;
      if (mainWindow && !mainWindow.isDestroyed()) {
        mainWindow.webContents.send('backend-status', {
          connected: false,
          error: err.message,
        });
      }
    });

    backendProcess.on('exit', (code) => {
      console.log(`[backend] Exited with code ${code}`);
      backendReady = false;
      backendProcess = null;
      if (mainWindow && !mainWindow.isDestroyed()) {
        mainWindow.webContents.send('backend-status', { connected: false });
      }
    });
  } catch (err) {
    console.error('[backend] Spawn error:', err);
  }
}

function stopBackend() {
  if (backendProcess) {
    console.log('[backend] Stopping...');
    backendProcess.kill('SIGTERM');
    // Force-kill after 3 seconds if it hasn't exited
    setTimeout(() => {
      if (backendProcess) {
        try {
          backendProcess.kill('SIGKILL');
        } catch (_) {
          // already dead
        }
      }
    }, 3000);
  }
}

// ── Window creation ────────────────────────────────────────────────────────────

function createMainWindow() {
  const icon = nativeImage.createFromPath(ICON_PATH);

  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    icon: icon,
    title: 'Tojo Assistant',
    backgroundColor: '#2d2d2d',
    show: false, // show after ready-to-show to avoid flash
    webPreferences: {
      preload: PRELOAD_PATH,
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
    },
  });

  mainWindow.loadFile(RENDERER_PATH);

  // Graceful show
  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
    // Notify renderer of current backend status
    mainWindow.webContents.send('backend-status', { connected: backendReady });
  });

  mainWindow.on('close', (event) => {
    // Minimize to tray instead of closing (optional behavior)
    if (tray && !app.isQuitting) {
      event.preventDefault();
      mainWindow.hide();
    }
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

// ── System tray ────────────────────────────────────────────────────────────────

function createTray() {
  const icon = nativeImage.createFromPath(ICON_PATH).resize({ width: 16, height: 16 });
  tray = new Tray(icon);
  tray.setToolTip('Tojo Assistant - I shall handle everything.');

  const contextMenu = Menu.buildFromTemplate([
    {
      label: 'Show Tojo Assistant',
      click: () => {
        if (mainWindow) {
          mainWindow.show();
          mainWindow.focus();
        }
      },
    },
    { type: 'separator' },
    {
      label: 'Restart Backend',
      click: () => {
        stopBackend();
        setTimeout(startBackend, 1000);
      },
    },
    { type: 'separator' },
    {
      label: 'Quit',
      click: () => {
        app.isQuitting = true;
        app.quit();
      },
    },
  ]);

  tray.setContextMenu(contextMenu);

  tray.on('double-click', () => {
    if (mainWindow) {
      mainWindow.show();
      mainWindow.focus();
    }
  });
}

// ── IPC handlers ───────────────────────────────────────────────────────────────

function registerIPC() {
  // File picker
  ipcMain.handle('dialog-open-file', async (_event, options = {}) => {
    const result = await dialog.showOpenDialog(mainWindow, {
      title: options.title || 'Select File',
      filters: options.filters || [
        { name: 'All Files', extensions: ['*'] },
        { name: 'Spreadsheets', extensions: ['xlsx', 'xls', 'csv'] },
        { name: 'Documents', extensions: ['pdf', 'docx', 'doc', 'txt'] },
      ],
      properties: ['openFile'],
    });
    return result.canceled ? null : result.filePaths[0];
  });

  // Folder picker
  ipcMain.handle('dialog-open-folder', async () => {
    const result = await dialog.showOpenDialog(mainWindow, {
      title: 'Select Folder',
      properties: ['openDirectory'],
    });
    return result.canceled ? null : result.filePaths[0];
  });

  // Save-as dialog
  ipcMain.handle('dialog-save-file', async (_event, options = {}) => {
    const result = await dialog.showSaveDialog(mainWindow, {
      title: options.title || 'Save File',
      defaultPath: options.defaultPath || '',
      filters: options.filters || [{ name: 'All Files', extensions: ['*'] }],
    });
    return result.canceled ? null : result.filePath;
  });

  // Forward messages to backend (placeholder -- real impl uses WebSocket from renderer)
  ipcMain.handle('send-message', async (_event, message) => {
    // This is a fallback path; the renderer normally talks over WebSocket directly.
    return { ok: true, note: 'Use WebSocket for real-time communication.' };
  });

  // OpenClaw status check — queries the Python backend
  ipcMain.handle('openclaw-status', async () => {
    if (!backendReady) {
      return { available: false, note: 'Backend not running yet.' };
    }
    try {
      const http = require('http');
      return await new Promise((resolve) => {
        const req = http.get('http://127.0.0.1:8000/api/openclaw/status', (res) => {
          let body = '';
          res.on('data', (chunk) => (body += chunk));
          res.on('end', () => {
            try {
              resolve(JSON.parse(body));
            } catch {
              resolve({ available: false, note: 'Invalid response from backend.' });
            }
          });
        });
        req.on('error', () => {
          resolve({ available: false, note: 'Could not reach backend.' });
        });
        req.setTimeout(3000, () => {
          req.destroy();
          resolve({ available: false, note: 'Backend request timed out.' });
        });
      });
    } catch {
      return { available: false, note: 'Error checking OpenClaw status.' };
    }
  });

  // Window controls
  ipcMain.on('window-minimize', () => mainWindow?.minimize());
  ipcMain.on('window-maximize', () => {
    if (mainWindow?.isMaximized()) {
      mainWindow.unmaximize();
    } else {
      mainWindow?.maximize();
    }
  });
  ipcMain.on('window-close', () => mainWindow?.close());
}

// ── App lifecycle ──────────────────────────────────────────────────────────────

app.whenReady().then(() => {
  registerIPC();
  startBackend();
  createMainWindow();
  createTray();

  app.on('activate', () => {
    // macOS dock click re-creates window
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    } else if (mainWindow) {
      mainWindow.show();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.isQuitting = true;
    app.quit();
  }
});

app.on('before-quit', () => {
  app.isQuitting = true;
  stopBackend();
});
