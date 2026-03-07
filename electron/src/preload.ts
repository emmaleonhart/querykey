import { contextBridge, ipcRenderer } from 'electron';
import * as os from 'os';
import { IPC_CHANNELS, TojoAPI } from './shared/types';

const tojoAPI: TojoAPI = {
  // --- Message / Backend Communication ---
  sendMessage: (message: string) => ipcRenderer.invoke(IPC_CHANNELS.SEND_MESSAGE, message),
  onBackendMessage: (callback) => {
    ipcRenderer.on(IPC_CHANNELS.BACKEND_MESSAGE, (_event, data) => callback(data));
  },
  onBackendStatus: (callback) => {
    ipcRenderer.on(IPC_CHANNELS.BACKEND_STATUS, (_event, status) => callback(status));
  },

  // --- File / Folder Dialogs ---
  selectFile: (options?) => ipcRenderer.invoke(IPC_CHANNELS.DIALOG_OPEN_FILE, options),
  selectFolder: () => ipcRenderer.invoke(IPC_CHANNELS.DIALOG_OPEN_FOLDER),
  selectSaveFile: (options?) => ipcRenderer.invoke(IPC_CHANNELS.DIALOG_SAVE_FILE, options),

  // --- System Info ---
  getSystemInfo: () => ({
    platform: process.platform,
    arch: process.arch,
    hostname: os.hostname(),
    username: os.userInfo().username,
    homeDir: os.homedir(),
    cpus: os.cpus().length,
    totalMemory: Math.round(os.totalmem() / (1024 * 1024 * 1024)) + ' GB',
    nodeVersion: process.versions.node,
    electronVersion: process.versions.electron,
    chromeVersion: process.versions.chrome,
  }),

  // --- App Control ---
  minimizeWindow: () => ipcRenderer.send(IPC_CHANNELS.WINDOW_MINIMIZE),
  maximizeWindow: () => ipcRenderer.send(IPC_CHANNELS.WINDOW_MAXIMIZE),
  closeWindow: () => ipcRenderer.send(IPC_CHANNELS.WINDOW_CLOSE),

  // --- OpenClaw Status ---
  getOpenClawStatus: () => ipcRenderer.invoke(IPC_CHANNELS.OPENCLAW_STATUS),
};

contextBridge.exposeInMainWorld('tojoAPI', tojoAPI);
