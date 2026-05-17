'use strict';
// QueryKey desktop — Electron main process.
//
// R20-1: window only. R20-2 adds Rust server lifecycle (spawn the
// sibling querykey-server, health-poll, tear down on quit) so there is
// no fragile .bat launcher. Kept intentionally small.

const { app, BrowserWindow } = require('electron');
const path = require('path');

let mainWindow = null;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1180,
    height: 780,
    backgroundColor: '#263238', // blue-grey 900 (matches old theme seed)
    title: 'QueryKey',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  mainWindow.removeMenu();
  mainWindow.loadFile('index.html');
  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

app.whenReady().then(() => {
  createWindow();
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  // R20-2 will also kill the spawned server here before quitting.
  if (process.platform !== 'darwin') app.quit();
});
