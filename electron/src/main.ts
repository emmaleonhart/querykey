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
import { spawn, execSync, ChildProcess } from 'child_process';
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
let openclawProcess: ChildProcess | null = null;
let backendReady = false;
let backendHealthTimer: ReturnType<typeof setInterval> | null = null;
let openclawRetries = 0;
let openclawHealthTimer: ReturnType<typeof setInterval> | null = null;
const OPENCLAW_MAX_RETRIES = 5;
const OPENCLAW_RETRY_DELAY = 3000;

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

async function startBackend(): Promise<void> {
  // Check if backend is already running (stale process or external)
  const alreadyRunning = await new Promise<boolean>((resolve) => {
    const req = http.get('http://127.0.0.1:8000/health', (res) => {
      resolve(res.statusCode === 200);
    });
    req.on('error', () => resolve(false));
    req.setTimeout(2000, () => { req.destroy(); resolve(false); });
  });

  if (alreadyRunning) {
    console.log('[backend] Backend already running on port 8000, reusing existing instance');
    backendReady = true;
    notifyBackendStatus({ connected: true });
    return;
  }

  let cmd: string;
  let args: string[];
  let cwd: string;

  // Startup diagnostics
  console.log('[startup] __dirname:', __dirname);
  console.log('[startup] ROOT_DIR:', ROOT_DIR);
  console.log('[startup] RENDERER_PATH:', RENDERER_PATH, 'exists:', fs.existsSync(RENDERER_PATH));
  console.log('[startup] PRELOAD_PATH:', PRELOAD_PATH, 'exists:', fs.existsSync(PRELOAD_PATH));

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
    console.log('[backend] CWD:', cwd);
    console.log('[backend] backend/ exists:', fs.existsSync(path.join(cwd, 'backend')));
  }

  try {
    backendProcess = spawn(cmd, args, {
      cwd,
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, PYTHONUNBUFFERED: '1' },
    });

    console.log('[backend] Spawned PID:', backendProcess.pid);

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

    // Poll backend health as a safety net
    startBackendHealthCheck();
  } catch (err) {
    console.error('[backend] Spawn error:', err);
  }
}

function startBackendHealthCheck(): void {
  if (backendHealthTimer) return;
  let attempts = 0;
  const MAX_ATTEMPTS = 30; // 30 × 2s = 60s

  backendHealthTimer = setInterval(() => {
    if (backendReady) {
      if (backendHealthTimer) clearInterval(backendHealthTimer);
      backendHealthTimer = null;
      return;
    }

    attempts++;
    if (attempts > MAX_ATTEMPTS) {
      console.error('[backend] Health check gave up after', MAX_ATTEMPTS, 'attempts');
      if (backendHealthTimer) clearInterval(backendHealthTimer);
      backendHealthTimer = null;
      return;
    }

    const req = http.get('http://127.0.0.1:8000/health', (res) => {
      if (res.statusCode === 200 && !backendReady) {
        console.log('[backend] Health check detected running backend');
        backendReady = true;
        notifyBackendStatus({ connected: true });
      }
    });
    req.on('error', () => {
      // Backend not ready yet, keep polling
    });
    req.setTimeout(2000, () => req.destroy());
  }, 2000);
}

function stopBackend(): void {
  if (backendHealthTimer) {
    clearInterval(backendHealthTimer);
    backendHealthTimer = null;
  }
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

// ── OpenClaw gateway lifecycle ────────────────────────────────────────────────

function findWSLDistro(): string | null {
  try {
    const output = execSync('wsl --list --quiet', { encoding: 'utf-8', timeout: 5000 });
    // Prefer Ubuntu, fall back to first non-docker distro
    const distros = output
      .split(/\r?\n/)
      .map((line) => line.replace(/\0/g, '').trim())
      .filter((line) => line.length > 0);

    const ubuntu = distros.find((d) => d.toLowerCase().includes('ubuntu'));
    if (ubuntu) return ubuntu;

    const nonDocker = distros.find((d) => !d.toLowerCase().includes('docker'));
    return nonDocker || null;
  } catch {
    return null;
  }
}

function isOpenClawGatewayRunning(): Promise<boolean> {
  return new Promise((resolve) => {
    const req = http.get('http://127.0.0.1:18789/', (res) => {
      resolve(res.statusCode === 200);
    });
    req.on('error', () => resolve(false));
    req.setTimeout(3000, () => {
      req.destroy();
      resolve(false);
    });
  });
}

async function startOpenClawGateway(): Promise<void> {
  const running = await isOpenClawGatewayRunning();
  if (running) {
    console.log('[openclaw] Gateway already running on port 18789');
    openclawRetries = 0;
    return;
  }

  const distro = findWSLDistro();
  if (!distro) {
    console.warn('[openclaw] No suitable WSL distro found (need Ubuntu)');
    return;
  }

  console.log('[openclaw] Starting gateway via WSL distro:', distro, `(attempt ${openclawRetries + 1}/${OPENCLAW_MAX_RETRIES})`);

  try {
    openclawProcess = spawn('wsl', ['-d', distro, '-e', 'bash', '-lc', 'openclaw gateway'], {
      stdio: ['pipe', 'pipe', 'pipe'],
      detached: false,
    });

    console.log('[openclaw] Spawned gateway PID:', openclawProcess.pid);

    openclawProcess.stdout?.on('data', (data: Buffer) => {
      const output = data.toString().trim();
      if (output) console.log(`[openclaw] ${output}`);
    });

    openclawProcess.stderr?.on('data', (data: Buffer) => {
      const output = data.toString().trim();
      if (output) console.log(`[openclaw:err] ${output}`);
    });

    openclawProcess.on('error', (err: Error) => {
      console.error('[openclaw] Failed to start gateway:', err.message);
      openclawProcess = null;
      retryOpenClawGateway();
    });

    openclawProcess.on('exit', (code) => {
      console.log(`[openclaw] Gateway exited with code ${code}`);
      openclawProcess = null;
      // Only retry on unexpected exits (not when we deliberately stopped it)
      if (!(app as unknown as { isQuitting: boolean }).isQuitting) {
        retryOpenClawGateway();
      }
    });

    // Start health polling for the gateway
    startOpenClawHealthCheck();
  } catch (err) {
    console.error('[openclaw] Spawn error:', err);
    retryOpenClawGateway();
  }
}

function retryOpenClawGateway(): void {
  if ((app as unknown as { isQuitting: boolean }).isQuitting) return;
  openclawRetries++;
  if (openclawRetries >= OPENCLAW_MAX_RETRIES) {
    console.error(`[openclaw] Gave up after ${OPENCLAW_MAX_RETRIES} attempts`);
    return;
  }
  console.log(`[openclaw] Retrying in ${OPENCLAW_RETRY_DELAY / 1000}s...`);
  setTimeout(() => {
    if (!(app as unknown as { isQuitting: boolean }).isQuitting) {
      startOpenClawGateway();
    }
  }, OPENCLAW_RETRY_DELAY);
}

function startOpenClawHealthCheck(): void {
  if (openclawHealthTimer) return;

  // Wait a bit for the gateway to initialize before first check
  openclawHealthTimer = setInterval(async () => {
    const running = await isOpenClawGatewayRunning();
    if (running) {
      // Gateway is healthy, reset retry counter
      openclawRetries = 0;
    }
  }, 10000); // Check every 10 seconds
}

function stopOpenClawGateway(): void {
  if (openclawHealthTimer) {
    clearInterval(openclawHealthTimer);
    openclawHealthTimer = null;
  }
  if (openclawProcess) {
    console.log('[openclaw] Stopping gateway...');
    openclawProcess.kill('SIGTERM');
    const proc = openclawProcess;
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
    title: 'Sakuya Assistant',
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
  tray.setToolTip('Sakuya Assistant - I shall handle everything.');

  const contextMenu = Menu.buildFromTemplate([
    {
      label: 'Show Sakuya Assistant',
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
    {
      label: 'Restart OpenClaw Gateway',
      click: () => {
        stopOpenClawGateway();
        setTimeout(startOpenClawGateway, 1000);
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
  startOpenClawGateway().catch((err) => {
    console.error('[openclaw] Auto-start failed:', err);
    retryOpenClawGateway();
  });
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
  stopOpenClawGateway();
});
