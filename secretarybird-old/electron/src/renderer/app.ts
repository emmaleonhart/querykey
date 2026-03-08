/**
 * Secretary Bird Assistant - Frontend entry point.
 * Wires together all modules and sets up event listeners.
 */

import type { WSMessage } from '../shared/types';
import * as ws from './websocket';
import * as chat from './chat';
import * as actions from './actions';
import * as ui from './ui';

// ── DOM references ─────────────────────────────────────────────────────────────
const chatArea = document.getElementById('chat-area')!;
const messageInput = document.getElementById('message-input') as HTMLTextAreaElement;
const btnSend = document.getElementById('btn-send') as HTMLButtonElement;
const btnToggleSidebar = document.getElementById('btn-toggle-sidebar')!;
const btnClearChat = document.getElementById('btn-clear-chat')!;
const btnSystemInfo = document.getElementById('btn-system-info')!;
const typingIndicator = document.getElementById('typing-indicator')!;
const statusBackend = document.getElementById('status-backend')!;
const statusOpenClaw = document.getElementById('status-openclaw')!;
const sidebar = document.getElementById('sidebar')!;
const btnKillOpenClaw = document.getElementById('btn-kill-openclaw') as HTMLButtonElement;
const btnRestartOpenClaw = document.getElementById('btn-restart-openclaw') as HTMLButtonElement;

// ── Initialize modules ─────────────────────────────────────────────────────────
chat.init(chatArea, typingIndicator);
ui.init(statusBackend, statusOpenClaw);
actions.init(sendUserMessage, (payload) => {
  if (!ws.send(payload)) {
    chat.appendAssistantMessage(
      'I am unable to reach the backend server at the moment. Please ensure it is running.',
    );
  }
});

// ── WebSocket message handler ─────────────────────────────────────────────────
function handleServerMessage(raw: string): void {
  let data: WSMessage;
  try {
    data = JSON.parse(raw) as WSMessage;
  } catch {
    data = { type: 'response', content: raw };
  }

  chat.hideTypingIndicator();

  switch (data.type) {
    case 'response':
      chat.appendAssistantMessage(data.content || data.message || '');
      break;
    case 'stream_start':
      chat.showTypingIndicator();
      break;
    case 'stream_chunk':
      chat.appendStreamChunk(data.content || '');
      break;
    case 'stream_end':
      chat.hideTypingIndicator();
      chat.finalizeStream();
      break;
    case 'error':
      chat.appendAssistantMessage(`An error occurred: ${data.message || 'Unknown error'}`);
      break;
    case 'status':
      if (data.openclaw !== undefined) {
        ui.updateConnectionStatus('openclaw', data.openclaw);
      }
      break;
    default:
      if (data.content || data.message) {
        chat.appendAssistantMessage(data.content || data.message || '');
      }
  }
}

ws.setMessageHandler(handleServerMessage);
ws.setStatusHandler((connected) => ui.updateConnectionStatus('backend', connected));

// ── Send message ──────────────────────────────────────────────────────────────

function sendUserMessage(text: string): void {
  if (!text || !text.trim()) return;
  const trimmed = text.trim();

  chat.appendUserMessage(trimmed);
  chat.showTypingIndicator();

  const sent = ws.send({
    type: 'message',
    content: trimmed,
    history: chat.getHistory().slice(-20),
  });

  if (!sent) {
    chat.hideTypingIndicator();
    chat.appendAssistantMessage(
      'I am unable to reach the backend server at the moment. Please ensure it is running.',
    );
  }

  messageInput.value = '';
  messageInput.style.height = 'auto';
  updateSendButton();
}

function updateSendButton(): void {
  btnSend.disabled = !messageInput.value.trim();
}

// ── Event listeners ───────────────────────────────────────────────────────────

btnSend.addEventListener('click', () => {
  sendUserMessage(messageInput.value);
});

messageInput.addEventListener('keydown', (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    sendUserMessage(messageInput.value);
  }
});

messageInput.addEventListener('input', () => {
  updateSendButton();
  messageInput.style.height = 'auto';
  messageInput.style.height = Math.min(messageInput.scrollHeight, 120) + 'px';
});

btnToggleSidebar.addEventListener('click', () => {
  sidebar.classList.toggle('collapsed');
});

btnClearChat.addEventListener('click', () => {
  chat.clearChat(bindQuickActions);
});

btnSystemInfo.addEventListener('click', ui.showSystemInfoModal);

// OpenClaw kill / restart buttons
btnKillOpenClaw.addEventListener('click', async () => {
  if (!window.tojoAPI) return;
  btnKillOpenClaw.disabled = true;
  btnKillOpenClaw.textContent = '...';
  const result = await window.tojoAPI.killOpenClaw();
  chat.appendAssistantMessage(`OpenClaw emergency stop: ${result.message}`);
  ui.updateConnectionStatus('openclaw', false);
  btnKillOpenClaw.disabled = false;
  btnKillOpenClaw.textContent = 'STOP';
});

btnRestartOpenClaw.addEventListener('click', async () => {
  if (!window.tojoAPI) return;
  btnRestartOpenClaw.disabled = true;
  btnRestartOpenClaw.textContent = 'Restarting...';
  chat.appendAssistantMessage('Restarting OpenClaw gateway...');
  const result = await window.tojoAPI.restartOpenClaw();
  chat.appendAssistantMessage(`OpenClaw restart: ${result.message}`);
  ui.updateConnectionStatus('openclaw', result.ok);
  btnRestartOpenClaw.disabled = false;
  btnRestartOpenClaw.textContent = 'Restart';
});

// Sidebar nav buttons
document.querySelectorAll<HTMLButtonElement>('.nav-btn').forEach((btn) => {
  btn.addEventListener('click', () => {
    const action = btn.dataset.action;
    if (action) actions.handleAction(action);
  });
});

// Quick action buttons
function bindQuickActions(): void {
  document.querySelectorAll<HTMLButtonElement>('.quick-action-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const action = btn.dataset.action;
      if (action) actions.handleAction(action);
    });
  });
}
bindQuickActions();

// Event delegation for dynamically created quick actions
chatArea.addEventListener('click', (e: MouseEvent) => {
  const qab = (e.target as HTMLElement).closest<HTMLButtonElement>('.quick-action-btn');
  if (qab?.dataset.action) {
    actions.handleAction(qab.dataset.action);
  }
});

// ── Backend status from Electron IPC ──────────────────────────────────────────
if (window.tojoAPI) {
  window.tojoAPI.onBackendStatus((status) => {
    ui.updateConnectionStatus('backend', status.connected);
    if (status.connected && !ws.isConnected()) {
      ws.connect();
    }
  });
}

// ── Initialization ────────────────────────────────────────────────────────────
messageInput.focus();
ws.connect();

if (window.tojoAPI) {
  window.tojoAPI.getOpenClawStatus().then((status) => {
    ui.updateConnectionStatus('openclaw', status.available);
  });
}

console.log('[tojo] Frontend initialized.');
