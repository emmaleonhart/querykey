/**
 * WebSocket connection manager with auto-reconnect.
 */

import type { WSPayload } from '../shared/types';

const WS_URL = 'ws://127.0.0.1:8000/ws/chat';
const RECONNECT_INTERVAL = 3000;
const MAX_RECONNECT_TRIES = 20;

let ws: WebSocket | null = null;
let wsConnected = false;
let reconnectAttempts = 0;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

type MessageHandler = (raw: string) => void;
type StatusHandler = (connected: boolean) => void;

let onMessage: MessageHandler = () => {};
let onStatusChange: StatusHandler = () => {};

export function setMessageHandler(handler: MessageHandler): void {
  onMessage = handler;
}

export function setStatusHandler(handler: StatusHandler): void {
  onStatusChange = handler;
}

export function isConnected(): boolean {
  return wsConnected;
}

export function connect(): void {
  if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
    return;
  }

  try {
    ws = new WebSocket(WS_URL);
  } catch (err) {
    console.error('[ws] Failed to create WebSocket:', err);
    scheduleReconnect();
    return;
  }

  ws.addEventListener('open', () => {
    console.log('[ws] Connected');
    wsConnected = true;
    reconnectAttempts = 0;
    onStatusChange(true);
  });

  ws.addEventListener('message', (event: MessageEvent) => {
    onMessage(event.data as string);
  });

  ws.addEventListener('close', () => {
    console.log('[ws] Disconnected');
    wsConnected = false;
    onStatusChange(false);
    scheduleReconnect();
  });

  ws.addEventListener('error', (err) => {
    console.error('[ws] Error:', err);
    wsConnected = false;
    onStatusChange(false);
  });
}

function scheduleReconnect(): void {
  if (reconnectTimer) return;
  if (reconnectAttempts >= MAX_RECONNECT_TRIES) {
    console.warn('[ws] Max reconnect attempts reached.');
    return;
  }
  reconnectAttempts++;
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connect();
  }, RECONNECT_INTERVAL);
}

export function send(payload: WSPayload): boolean {
  if (!ws || ws.readyState !== WebSocket.OPEN) {
    return false;
  }
  ws.send(JSON.stringify(payload));
  return true;
}
