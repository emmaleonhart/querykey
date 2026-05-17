'use strict';
// Minimal, safe bridge. Data API calls go straight from the renderer
// via fetch() (no IPC for data). IPC is used only for the one thing
// the renderer cannot do itself: observing the main-process-managed
// server lifecycle. `marked` is exposed here (preload has node) so the
// renderer needs no bundler / script-src juggling.

const { contextBridge, ipcRenderer } = require('electron');
const { marked } = require('marked');

contextBridge.exposeInMainWorld('qk', {
  serverStatus: () => ipcRenderer.invoke('server-status'),
  onServerStatus: (cb) => {
    const h = (_e, s) => cb(s);
    ipcRenderer.on('server-status', h);
    return () => ipcRenderer.removeListener('server-status', h);
  },
});

contextBridge.exposeInMainWorld('md', {
  // Render markdown to HTML. Bodies are the user's own local vault
  // files (trusted, local-first); marked's default output is fine.
  parse: (s) => marked.parse(String(s || ''), { breaks: true }),
});
