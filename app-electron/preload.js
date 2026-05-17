'use strict';
// Minimal, safe bridge. Data API calls go straight from the renderer
// via fetch() to the local server (no IPC for data — fewest moving
// parts). This bridge is reserved for things the renderer genuinely
// cannot do itself; R20-2 will expose server status here.

const { contextBridge } = require('electron');

contextBridge.exposeInMainWorld('qk', {
  // Placeholder until R20-2. Renderer treats absence as "unknown".
  serverStatus: async () => ({ state: 'unknown' }),
});
