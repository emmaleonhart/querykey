'use strict';
// QueryKey desktop — Electron main process.
//
// Owns the Rust server lifecycle so there is NO fragile .bat launcher
// (the thing that kept breaking). On launch: find the sibling
// querykey/server, build it if the binary is missing, spawn it with
// VAULT_DIR resolved, health-poll /health, surface status to the
// renderer, and tear the child down on quit. Kept small on purpose.

const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const http = require('http');
const { spawn, spawnSync } = require('child_process');

let mainWindow = null;
let serverProc = null;
let serverState = { state: 'starting', detail: 'initialising' };

function setStatus(state, detail = '') {
  serverState = { state, detail };
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send('server-status', serverState);
  }
}

// app-electron/ -> querykey/server
function resolveServerDir() {
  const d = path.join(__dirname, '..', 'server');
  return fs.existsSync(path.join(d, 'Cargo.toml')) ? d : null;
}

// Priority: explicit env (set by the life-planning !run-UI.bat) ->
// the canonical dev vault (sibling life-planning/prm) -> let the
// server use its own querykey.toml walk-up / ./vault fallback.
function resolveVaultDir() {
  if (process.env.QK_VAULT_DIR) return process.env.QK_VAULT_DIR;
  if (process.env.VAULT_DIR) return process.env.VAULT_DIR;
  const lp = path.join(__dirname, '..', '..', 'life-planning', 'prm');
  if (fs.existsSync(path.join(lp, 'querykey.toml'))) return lp;
  return null;
}

function healthPoll(timeoutMs, cb) {
  const start = Date.now();
  const tick = () => {
    const req = http.get('http://127.0.0.1:8000/health', (res) => {
      res.resume();
      if (res.statusCode === 200) return cb(true);
      schedule();
    });
    req.on('error', schedule);
    req.setTimeout(2000, () => req.destroy());
  };
  const schedule = () => {
    if (Date.now() - start > timeoutMs) return cb(false);
    setTimeout(tick, 1000);
  };
  tick();
}

function startServer() {
  const serverDir = resolveServerDir();
  if (!serverDir) {
    setStatus('error', 'querykey/server not found (expected sibling of app-electron/)');
    return;
  }
  const exe = path.join(
    serverDir,
    'target',
    'debug',
    process.platform === 'win32' ? 'querykey-server.exe' : 'querykey-server',
  );
  const env = { ...process.env };
  const vault = resolveVaultDir();
  if (vault) env.VAULT_DIR = vault;

  const launch = () => {
    setStatus('starting', vault ? `launching · vault ${path.basename(vault)}` : 'launching');
    serverProc = spawn(exe, [], { cwd: serverDir, env, windowsHide: true });
    serverProc.on('error', (e) => setStatus('error', `spawn failed: ${e.message}`));
    serverProc.on('exit', (code) => {
      const wasStopping = serverState.state === 'stopping';
      serverProc = null;
      if (!wasStopping) setStatus('error', `server exited (code ${code})`);
    });
    healthPoll(180000, (ok) => {
      if (ok) setStatus('ok', vault ? `connected · ${path.basename(vault)}` : 'connected');
      else setStatus('error', 'no /health response — check the server console');
    });
  };

  if (!fs.existsSync(exe)) {
    setStatus('starting', 'building server (first run can take a few minutes)…');
    const build = spawn('cargo', ['build'], {
      cwd: serverDir,
      env,
      shell: true,
      windowsHide: true,
    });
    build.stderr && build.stderr.on('data', () => {});
    build.on('error', (e) => setStatus('error', `cargo not found: ${e.message}`));
    build.on('exit', (code) => {
      if (code === 0 && fs.existsSync(exe)) launch();
      else setStatus('error', `cargo build failed (code ${code})`);
    });
  } else {
    launch();
  }
}

function stopServer() {
  if (!serverProc) return;
  setStatus('stopping', 'shutting down server');
  const pid = serverProc.pid;
  try {
    serverProc.kill();
  } catch (_) {}
  if (process.platform === 'win32' && pid) {
    try {
      spawnSync('taskkill', ['/F', '/T', '/PID', String(pid)], { windowsHide: true });
    } catch (_) {}
  }
  serverProc = null;
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1180,
    height: 780,
    backgroundColor: '#263238',
    title: 'QueryKey',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  mainWindow.removeMenu();
  mainWindow.loadFile('index.html');
  // Re-emit current status once the renderer is ready to receive it.
  mainWindow.webContents.on('did-finish-load', () => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send('server-status', serverState);
    }
  });
  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

ipcMain.handle('server-status', () => serverState);

app.whenReady().then(() => {
  startServer();
  createWindow();
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('before-quit', stopServer);
app.on('window-all-closed', () => {
  stopServer();
  if (process.platform !== 'darwin') app.quit();
});
