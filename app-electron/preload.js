'use strict';
// Minimal, safe bridge. Data API calls go straight from the renderer
// via fetch() to the local server (no IPC for data — fewest moving
// parts). IPC is used only for the one thing the renderer cannot do
// itself: observe the main-process-managed server lifecycle.

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('qk', {
  // Current {state,detail}. state: starting|ok|error|stopping.
  serverStatus: () => ipcRenderer.invoke('server-status'),
  // Subscribe to status changes. Returns an unsubscribe fn.
  onServerStatus: (cb) => {
    const h = (_e, s) => cb(s);
    ipcRenderer.on('server-status', h);
    return () => ipcRenderer.removeListener('server-status', h);
  },
});
