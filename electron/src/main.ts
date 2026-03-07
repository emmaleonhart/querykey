import {
  app,
  BrowserWindow,
  ipcMain,
  dialog,
  Tray,
  Menu,
  nativeImage,
  session,
} from 'electron';
import * as path from 'path';
import * as fs from 'fs';
import * as http from 'http';
import { spawn, ChildProcess } from 'child_process';
import { IPC_CHANNELS, BackendStatus, FileDialogOptions, OpenClawStatus } from './shared/types';

// ── Paths ──────────────────────────────────────────────────────────────────────
const ROOT_DIR = path.join(__dirname, '..', '..');
const ASSETS_DIR = path.join(ROOT_DIR, 'assets');
const ICON_PATH = path.join(ASSETS_DIR, 'tojo-avatar.png');
const PRELOAD_PATH = path.join(__dirname, 'preload.js');
const RENDERER_PATH = path.join(__dirname, '..', 'renderer', 'index.html');

// ── Packaged mode detection ─────────────────────────────────────────────────
const IS_PACKAGED = app.isPackaged;
const PACKAGED_BACKEND_EXE = IS_PACKAGED
  ? path.join(process.resourcesPath, 'backend', 'tojo-backend', 'tojo-backend.exe')
  : null;

// ── State ──────────────────────────────────────────────────────────────────────
let mainWindow: BrowserWindow | null = null;
let tray: Tray | null = null;
let backendProcess: ChildProcess | null = null;
let backendReady = false;

// ── Backend lifecycle ──────────────────────────────────────────────────────────

function findPython(): string {
  const candidates = [
    path.join(process.env.LOCALAPPDATA || '', 'Programs', 'Python', 'Python313', 'python.exe'),
    process.platform === 'win32' ? 'python' : 'python3',
  ];
  for (const candidate of candidates) {
    try {
      if (candidate.includes(path.sep) && fs.existsSync(candidate)) {
        return candidate;
      }
      if (!candidate.includes(path.sep)) {
        return candidate;
      }
    } catch {
      continue;
    }
  }
  return process.platform === 'win32' ? 'python' : 'python3';
}

function notifyBackendStatus(status: BackendStatus): void {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send(IPC_CHANNELS.BACKEND_STATUS, status);
  }
}

function handleBackendOutput(data: Buffer): void {
  const output = data.toString().trim();
  console.log(`[backend] ${output}`);
  if (output.includes('Application startup complete') || output.includes('Uvicorn running')) {
    backendReady = true;
    notifyBackendStatus({ connected: true });
  }
}

function startBackend(): void {
  let cmd: string;
  let args: string[];
  let cwd: string;

  if (IS_PACKAGED && PACKAGED_BACKEND_EXE) {
    if (fs.existsSync(PACKAGED_BACKEND_EXE)) {
      cmd = PACKAGED_BACKEND_EXE;
      args = [];
      cwd = path.dirname(PACKAGED_BACKEND_EXE);
      console.log('[backend] Using packaged exe:', cmd);
    } else {
      console.warn('[backend] Packaged exe not found, falling back to Python');
      cmd = findPython();
      args = ['-m', 'backend.server'];
      cwd = ROOT_DIR;
    }
  } else {
    cmd = findPython();
    args = ['-m', 'backend.server'];
    cwd = ROOT_DIR;
    console.log('[backend] Using Python:', cmd);
  }

  try {
    backendProcess = spawn(cmd, args, {
      cwd,
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, PYTHONUNBUFFERED: '1' },
    });

    backendProcess.stdout?.on('data', handleBackendOutput);
    backendProcess.stderr?.on('data', handleBackendOutput);

    backendProcess.on('error', (err: Error) => {
      console.error('[backend] Failed to start:', err.message);
      backendReady = false;
      notifyBackendStatus({ connected: false, error: err.message });
    });

    backendProcess.on('exit', (code) => {
      console.log(`[backend] Exited with code ${code}`);
      backendReady = false;
      backendProcess = null;
      notifyBackendStatus({ connected: false });
    });
  } catch (err) {
    console.error('[backend] Spawn error:', err);
  }
}

function stopBackend(): void {
  if (backendProcess) {
    console.log('[backend] Stopping...');
    backendProcess.kill('SIGTERM');
    const proc = backendProcess;
    setTimeout(() => {
      try {
        proc.kill('SIGKILL');
      } catch {
        // already dead
      }
    }, 3000);
  }
}

// ── Window creation ────────────────────────────────────────────────────────────

function createMainWindow(): void {
  const icon = nativeImage.createFromPath(ICON_PATH);

  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    icon,
    title: 'Tojo Assistant',
    backgroundColor: '#2d2d2d',
    show: false,
    webPreferences: {
      preload: PRELOAD_PATH,
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
    },
  });

  mainWindow.loadFile(RENDERER_PATH);

  mainWindow.webContents.on('console-message', (_event, level, message) => {
    const prefix = ['LOG', 'WARN', 'ERR'][level] || 'LOG';
    console.log(`[renderer:${prefix}] ${message}`);
  });

  mainWindow.once('ready-to-show', () => {
    mainWindow!.show();
    if (!IS_PACKAGED) {
      mainWindow!.webContents.openDevTools({ mode: 'detach' });
    }
    notifyBackendStatus({ connected: backendReady });
  });

  mainWindow.on('close', (event) => {
    if (tray && !(app as unknown as { isQuitting: boolean }).isQuitting) {
      event.preventDefault();
      mainWindow!.hide();
    }
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

// ── System tray ────────────────────────────────────────────────────────────────

function createTray(): void {
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
        (app as unknown as { isQuitting: boolean }).isQuitting = true;
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

function registerIPC(): void {
  ipcMain.handle(IPC_CHANNELS.DIALOG_OPEN_FILE, async (_event, options: FileDialogOptions = {}) => {
    const result = await dialog.showOpenDialog(mainWindow!, {
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

  ipcMain.handle(IPC_CHANNELS.DIALOG_OPEN_FOLDER, async () => {
    const result = await dialog.showOpenDialog(mainWindow!, {
      title: 'Select Folder',
      properties: ['openDirectory'],
    });
    return result.canceled ? null : result.filePaths[0];
  });

  ipcMain.handle(IPC_CHANNELS.DIALOG_SAVE_FILE, async (_event, options: FileDialogOptions = {}) => {
    const result = await dialog.showSaveDialog(mainWindow!, {
      title: options.title || 'Save File',
      defaultPath: options.defaultPath || '',
      filters: options.filters || [{ name: 'All Files', extensions: ['*'] }],
    });
    return result.canceled ? null : result.filePath;
  });

  ipcMain.handle(IPC_CHANNELS.SEND_MESSAGE, async () => {
    return { ok: true, note: 'Use WebSocket for real-time communication.' };
  });

  ipcMain.handle(IPC_CHANNELS.OPENCLAW_STATUS, async (): Promise<OpenClawStatus> => {
    if (!backendReady) {
      return { available: false, note: 'Backend not running yet.' };
    }
    try {
      return await new Promise<OpenClawStatus>((resolve) => {
        const req = http.get('http://127.0.0.1:8000/api/openclaw/status', (res) => {
          let body = '';
          res.on('data', (chunk: Buffer) => (body += chunk));
          res.on('end', () => {
            try {
              resolve(JSON.parse(body) as OpenClawStatus);
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

  ipcMain.on(IPC_CHANNELS.WINDOW_MINIMIZE, () => mainWindow?.minimize());
  ipcMain.on(IPC_CHANNELS.WINDOW_MAXIMIZE, () => {
    if (mainWindow?.isMaximized()) {
      mainWindow.unmaximize();
    } else {
      mainWindow?.maximize();
    }
  });
  ipcMain.on(IPC_CHANNELS.WINDOW_CLOSE, () => mainWindow?.close());
}

// ── App lifecycle ──────────────────────────────────────────────────────────────

app.whenReady().then(() => {
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          "default-src 'self';" +
          " style-src 'self' 'unsafe-inline';" +
          " script-src 'self';" +
          " img-src 'self' data:;" +
          " connect-src 'self' ws://127.0.0.1:8000 ws://localhost:8000 http://127.0.0.1:8000 http://localhost:8000 http://127.0.0.1:18789;",
        ],
      },
    });
  });

  registerIPC();
  startBackend();
  createMainWindow();
  createTray();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    } else if (mainWindow) {
      mainWindow.show();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    (app as unknown as { isQuitting: boolean }).isQuitting = true;
    app.quit();
  }
});

app.on('before-quit', () => {
  (app as unknown as { isQuitting: boolean }).isQuitting = true;
  stopBackend();
});
