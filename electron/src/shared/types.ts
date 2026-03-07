/** IPC channel names used between main, preload, and renderer */
export const IPC_CHANNELS = {
  DIALOG_OPEN_FILE: 'dialog-open-file',
  DIALOG_OPEN_FOLDER: 'dialog-open-folder',
  DIALOG_SAVE_FILE: 'dialog-save-file',
  SEND_MESSAGE: 'send-message',
  OPENCLAW_STATUS: 'openclaw-status',
  BACKEND_MESSAGE: 'backend-message',
  BACKEND_STATUS: 'backend-status',
  WINDOW_MINIMIZE: 'window-minimize',
  WINDOW_MAXIMIZE: 'window-maximize',
  WINDOW_CLOSE: 'window-close',
} as const;

/** Backend connection status sent over IPC */
export interface BackendStatus {
  connected: boolean;
  error?: string;
}

/** OpenClaw status response */
export interface OpenClawStatus {
  available: boolean;
  note?: string;
  gateway_url?: string;
  agent_id?: string;
}

/** File dialog options */
export interface FileDialogOptions {
  title?: string;
  defaultPath?: string;
  filters?: Array<{ name: string; extensions: string[] }>;
}

/** System information exposed by preload */
export interface SystemInfo {
  platform: string;
  arch: string;
  hostname: string;
  username: string;
  homeDir: string;
  cpus: number;
  totalMemory: string;
  nodeVersion: string;
  electronVersion: string;
  chromeVersion: string;
}

/** The API surface exposed to the renderer via contextBridge */
export interface TojoAPI {
  sendMessage: (message: string) => Promise<{ ok: boolean; note: string }>;
  onBackendMessage: (callback: (data: unknown) => void) => void;
  onBackendStatus: (callback: (status: BackendStatus) => void) => void;
  selectFile: (options?: FileDialogOptions) => Promise<string | null>;
  selectFolder: () => Promise<string | null>;
  selectSaveFile: (options?: FileDialogOptions) => Promise<string | null>;
  getSystemInfo: () => SystemInfo;
  minimizeWindow: () => void;
  maximizeWindow: () => void;
  closeWindow: () => void;
  getOpenClawStatus: () => Promise<OpenClawStatus>;
}

/** WebSocket message types from the backend */
export type WSMessageType =
  | 'response'
  | 'stream_start'
  | 'stream_chunk'
  | 'stream_end'
  | 'error'
  | 'status';

/** WebSocket message from the backend */
export interface WSMessage {
  type: WSMessageType;
  content?: string;
  message?: string;
  openclaw?: boolean;
}

/** Payload sent to the backend over WebSocket */
export interface WSPayload {
  type: string;
  content: string;
  context?: Record<string, string>;
  history?: ChatEntry[];
}

/** A single chat history entry */
export interface ChatEntry {
  role: 'user' | 'assistant';
  content: string;
  time: Date;
}

/** Action identifiers for sidebar and quick action buttons */
export type ActionId =
  | 'organize-files'
  | 'select-file'
  | 'select-folder'
  | 'check-spreadsheet'
  | 'analyze-data'
  | 'export-report'
  | 'connect-salesforce'
  | 'connect-gsheets'
  | 'connect-database'
  | 'analyze-competitors'
  | 'blue-ocean'
  | 'scrape-competitor'
  | 'build-pipeline'
  | 'run-pipeline'
  | 'view-logs';

/** Augment the global Window interface so TS knows about tojoAPI */
declare global {
  interface Window {
    tojoAPI?: TojoAPI;
  }
}
