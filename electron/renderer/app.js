/* ════════════════════════════════════════════════════════════════════════════
   TOJO ASSISTANT - Frontend Application Logic
   ════════════════════════════════════════════════════════════════════════════ */

(() => {
  'use strict';

  // ── Constants ──────────────────────────────────────────────────────────────
  const WS_URL = 'ws://127.0.0.1:8000/ws/chat';
  const RECONNECT_INTERVAL = 3000;   // ms between reconnect attempts
  const MAX_RECONNECT_TRIES = 20;
  const AVATAR_SRC = '../../assets/tojo-avatar.png';

  // ── DOM references ─────────────────────────────────────────────────────────
  const chatArea          = document.getElementById('chat-area');
  const messageInput      = document.getElementById('message-input');
  const btnSend           = document.getElementById('btn-send');
  const btnToggleSidebar  = document.getElementById('btn-toggle-sidebar');
  const btnClearChat      = document.getElementById('btn-clear-chat');
  const btnSystemInfo     = document.getElementById('btn-system-info');
  const typingIndicator   = document.getElementById('typing-indicator');
  const statusBackend     = document.getElementById('status-backend');
  const statusOpenClaw    = document.getElementById('status-openclaw');
  const sidebar           = document.getElementById('sidebar');

  // ── State ──────────────────────────────────────────────────────────────────
  let ws = null;
  let wsConnected = false;
  let reconnectAttempts = 0;
  let reconnectTimer = null;
  let chatHistory = [];     // { role: 'user'|'assistant', content: string, time: Date }
  let welcomeVisible = true;

  // ══════════════════════════════════════════════════════════════════════════
  //  WEBSOCKET
  // ══════════════════════════════════════════════════════════════════════════

  function connectWebSocket() {
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
      updateConnectionStatus('backend', true);
    });

    ws.addEventListener('message', (event) => {
      handleServerMessage(event.data);
    });

    ws.addEventListener('close', () => {
      console.log('[ws] Disconnected');
      wsConnected = false;
      updateConnectionStatus('backend', false);
      scheduleReconnect();
    });

    ws.addEventListener('error', (err) => {
      console.error('[ws] Error:', err);
      wsConnected = false;
      updateConnectionStatus('backend', false);
    });
  }

  function scheduleReconnect() {
    if (reconnectTimer) return;
    if (reconnectAttempts >= MAX_RECONNECT_TRIES) {
      console.warn('[ws] Max reconnect attempts reached.');
      return;
    }
    reconnectAttempts++;
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      connectWebSocket();
    }, RECONNECT_INTERVAL);
  }

  function sendToBackend(payload) {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      appendAssistantMessage('I am unable to reach the backend server at the moment. Please ensure it is running.');
      return;
    }
    ws.send(JSON.stringify(payload));
  }

  // ══════════════════════════════════════════════════════════════════════════
  //  MESSAGE HANDLING
  // ══════════════════════════════════════════════════════════════════════════

  function handleServerMessage(raw) {
    let data;
    try {
      data = JSON.parse(raw);
    } catch {
      // Treat as plain text
      data = { type: 'response', content: raw };
    }

    hideTypingIndicator();

    switch (data.type) {
      case 'response':
        appendAssistantMessage(data.content || data.message || '');
        break;
      case 'stream_start':
        showTypingIndicator();
        break;
      case 'stream_chunk':
        // For streaming, append to the last assistant message or create new
        appendStreamChunk(data.content || '');
        break;
      case 'stream_end':
        hideTypingIndicator();
        finalizeStream();
        break;
      case 'error':
        appendAssistantMessage(`An error occurred: ${data.message || 'Unknown error'}`);
        break;
      case 'status':
        if (data.openclaw !== undefined) {
          updateConnectionStatus('openclaw', data.openclaw);
        }
        break;
      default:
        if (data.content || data.message) {
          appendAssistantMessage(data.content || data.message);
        }
    }
  }

  // ── Streaming support ──────────────────────────────────────────────────────
  let streamBuffer = '';
  let streamBubble = null;

  function appendStreamChunk(text) {
    if (!streamBubble) {
      streamBubble = createAssistantBubble('');
    }
    streamBuffer += text;
    const bubbleContent = streamBubble.querySelector('.message-bubble');
    if (bubbleContent) {
      bubbleContent.innerHTML = renderMarkdown(streamBuffer);
    }
    scrollToBottom();
  }

  function finalizeStream() {
    if (streamBuffer) {
      chatHistory.push({ role: 'assistant', content: streamBuffer, time: new Date() });
    }
    streamBuffer = '';
    streamBubble = null;
  }

  // ══════════════════════════════════════════════════════════════════════════
  //  CHAT UI
  // ══════════════════════════════════════════════════════════════════════════

  function removeWelcomeCard() {
    if (!welcomeVisible) return;
    const card = chatArea.querySelector('.welcome-card');
    if (card) card.remove();
    welcomeVisible = false;
  }

  function appendUserMessage(text) {
    removeWelcomeCard();
    chatHistory.push({ role: 'user', content: text, time: new Date() });

    const wrapper = document.createElement('div');
    wrapper.classList.add('message', 'user');

    const bubble = document.createElement('div');
    bubble.classList.add('message-bubble');
    bubble.textContent = text;

    const time = document.createElement('span');
    time.classList.add('message-time');
    time.textContent = formatTime(new Date());
    bubble.appendChild(time);

    wrapper.appendChild(bubble);
    chatArea.appendChild(wrapper);
    scrollToBottom();
  }

  function appendAssistantMessage(text) {
    removeWelcomeCard();
    chatHistory.push({ role: 'assistant', content: text, time: new Date() });
    createAssistantBubble(text);
    scrollToBottom();
  }

  function createAssistantBubble(text) {
    const wrapper = document.createElement('div');
    wrapper.classList.add('message', 'assistant');

    const avatar = document.createElement('img');
    avatar.classList.add('message-avatar');
    avatar.src = AVATAR_SRC;
    avatar.alt = 'Tojo';

    const bubble = document.createElement('div');
    bubble.classList.add('message-bubble');
    bubble.innerHTML = renderMarkdown(text);

    const time = document.createElement('span');
    time.classList.add('message-time');
    time.textContent = formatTime(new Date());
    bubble.appendChild(time);

    wrapper.appendChild(avatar);
    wrapper.appendChild(bubble);
    chatArea.appendChild(wrapper);
    return wrapper;
  }

  function showTypingIndicator() {
    typingIndicator.classList.remove('hidden');
    scrollToBottom();
  }

  function hideTypingIndicator() {
    typingIndicator.classList.add('hidden');
  }

  function clearChat() {
    chatArea.innerHTML = '';
    chatHistory = [];
    welcomeVisible = true;
    streamBuffer = '';
    streamBubble = null;

    // Re-create welcome card
    chatArea.innerHTML = `
      <div class="welcome-card">
        <img src="${AVATAR_SRC}" alt="Kirumi Tojo" class="welcome-avatar" />
        <h2>Welcome. I am Tojo Assistant.</h2>
        <p>I shall handle everything. Please tell me what you need, and I will attend to it promptly and precisely.</p>
        <div class="quick-actions">
          <button class="quick-action-btn" data-action="organize-files">Organize my files</button>
          <button class="quick-action-btn" data-action="check-spreadsheet">Check a spreadsheet</button>
          <button class="quick-action-btn" data-action="analyze-data">Analyze data</button>
          <button class="quick-action-btn" data-action="build-pipeline">Build a pipeline</button>
        </div>
      </div>
    `;
    bindQuickActions();
  }

  function scrollToBottom() {
    requestAnimationFrame(() => {
      chatArea.scrollTop = chatArea.scrollHeight;
    });
  }

  // ══════════════════════════════════════════════════════════════════════════
  //  MARKDOWN RENDERER (lightweight)
  // ══════════════════════════════════════════════════════════════════════════

  function renderMarkdown(text) {
    if (!text) return '';

    let html = escapeHtml(text);

    // Code blocks: ```...```
    html = html.replace(/```(\w*)\n([\s\S]*?)```/g, (_, lang, code) => {
      return `<pre><code class="lang-${lang}">${code.trim()}</code></pre>`;
    });

    // Inline code: `...`
    html = html.replace(/`([^`]+)`/g, '<code>$1</code>');

    // Bold: **...**
    html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');

    // Italic: *...*
    html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');

    // Headings: ### ...
    html = html.replace(/^### (.+)$/gm, '<strong style="font-size:14px;">$1</strong>');
    html = html.replace(/^## (.+)$/gm, '<strong style="font-size:15px;">$1</strong>');

    // Unordered lists: - item
    html = html.replace(/^- (.+)$/gm, '&bull; $1');

    // Line breaks
    html = html.replace(/\n/g, '<br/>');

    return html;
  }

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  // ══════════════════════════════════════════════════════════════════════════
  //  ACTION HANDLERS
  // ══════════════════════════════════════════════════════════════════════════

  const ACTION_MESSAGES = {
    'organize-files':      'Please organize my files. I will select a folder.',
    'select-file':         null,  // special handling
    'select-folder':       null,  // special handling
    'check-spreadsheet':   'Please check a spreadsheet for errors. I will select a file.',
    'analyze-data':        'I would like to analyze some data. What format is it in?',
    'export-report':       'Please export a report from the current data.',
    'connect-salesforce':  'I would like to connect to Salesforce. Please guide me through the setup.',
    'connect-gsheets':     'I would like to connect to Google Sheets.',
    'connect-database':    'I would like to connect to a database. What connection details are needed?',
    'build-pipeline':      'I would like to build a data pipeline. Please help me configure the steps.',
    'run-pipeline':        'Please run the configured data pipeline.',
    'view-logs':           'Show me the recent pipeline execution logs.',
  };

  async function handleAction(action) {
    // Special: file/folder pickers
    if (action === 'select-file') {
      if (window.tojoAPI) {
        const filePath = await window.tojoAPI.selectFile();
        if (filePath) {
          sendUserMessage(`I selected this file: ${filePath}`);
        }
      }
      return;
    }

    if (action === 'select-folder') {
      if (window.tojoAPI) {
        const folderPath = await window.tojoAPI.selectFolder();
        if (folderPath) {
          sendUserMessage(`I selected this folder: ${folderPath}`);
        }
      }
      return;
    }

    // For actions that open a file picker as the next step
    if (action === 'organize-files' && window.tojoAPI) {
      const folderPath = await window.tojoAPI.selectFolder();
      if (folderPath) {
        sendUserMessage(`Please organize the files in this folder: ${folderPath}`);
        return;
      }
    }

    if (action === 'check-spreadsheet' && window.tojoAPI) {
      const filePath = await window.tojoAPI.selectFile({
        title: 'Select Spreadsheet',
        filters: [
          { name: 'Spreadsheets', extensions: ['xlsx', 'xls', 'csv'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });
      if (filePath) {
        sendUserMessage(`Please check this spreadsheet for errors: ${filePath}`);
        return;
      }
    }

    // Default: send the mapped message
    const msg = ACTION_MESSAGES[action];
    if (msg) {
      sendUserMessage(msg);
    }
  }

  // ══════════════════════════════════════════════════════════════════════════
  //  SEND MESSAGE
  // ══════════════════════════════════════════════════════════════════════════

  function sendUserMessage(text) {
    if (!text || !text.trim()) return;
    const trimmed = text.trim();

    appendUserMessage(trimmed);
    showTypingIndicator();

    sendToBackend({
      type: 'message',
      content: trimmed,
      history: chatHistory.slice(-20),  // send recent context
    });

    messageInput.value = '';
    messageInput.style.height = 'auto';
    updateSendButton();
  }

  // ══════════════════════════════════════════════════════════════════════════
  //  CONNECTION STATUS
  // ══════════════════════════════════════════════════════════════════════════

  function updateConnectionStatus(service, connected) {
    const el = service === 'backend' ? statusBackend : statusOpenClaw;
    if (connected) {
      el.classList.add('connected');
    } else {
      el.classList.remove('connected');
    }
  }

  // Listen for backend status from Electron main process
  if (window.tojoAPI) {
    window.tojoAPI.onBackendStatus((status) => {
      updateConnectionStatus('backend', status.connected);
      if (status.connected && !wsConnected) {
        // Backend just started, try connecting WebSocket
        connectWebSocket();
      }
    });
  }

  // ══════════════════════════════════════════════════════════════════════════
  //  SYSTEM INFO MODAL
  // ══════════════════════════════════════════════════════════════════════════

  function showSystemInfoModal() {
    // Remove existing modal
    const existing = document.querySelector('.modal-overlay');
    if (existing) existing.remove();

    let info = {};
    if (window.tojoAPI) {
      info = window.tojoAPI.getSystemInfo();
    } else {
      info = { note: 'Running outside Electron - limited info available.' };
    }

    const overlay = document.createElement('div');
    overlay.classList.add('modal-overlay');

    const rows = Object.entries(info)
      .map(([key, val]) => `
        <div class="modal-info-row">
          <span class="modal-info-label">${formatLabel(key)}</span>
          <span class="modal-info-value">${val}</span>
        </div>
      `)
      .join('');

    overlay.innerHTML = `
      <div class="modal">
        <h3>System Information</h3>
        ${rows}
        <button class="modal-close">Close</button>
      </div>
    `;

    document.body.appendChild(overlay);

    overlay.querySelector('.modal-close').addEventListener('click', () => overlay.remove());
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) overlay.remove();
    });
  }

  function formatLabel(camelCase) {
    return camelCase
      .replace(/([A-Z])/g, ' $1')
      .replace(/^./, (s) => s.toUpperCase())
      .trim();
  }

  // ══════════════════════════════════════════════════════════════════════════
  //  UTILITIES
  // ══════════════════════════════════════════════════════════════════════════

  function formatTime(date) {
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function updateSendButton() {
    btnSend.disabled = !messageInput.value.trim();
  }

  // ══════════════════════════════════════════════════════════════════════════
  //  EVENT LISTENERS
  // ══════════════════════════════════════════════════════════════════════════

  // Send button
  btnSend.addEventListener('click', () => {
    sendUserMessage(messageInput.value);
  });

  // Textarea: Enter to send, Shift+Enter for newline, auto-resize
  messageInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendUserMessage(messageInput.value);
    }
  });

  messageInput.addEventListener('input', () => {
    updateSendButton();
    // Auto-resize textarea
    messageInput.style.height = 'auto';
    messageInput.style.height = Math.min(messageInput.scrollHeight, 120) + 'px';
  });

  // Toggle sidebar
  btnToggleSidebar.addEventListener('click', () => {
    sidebar.classList.toggle('collapsed');
  });

  // Clear chat
  btnClearChat.addEventListener('click', clearChat);

  // System info
  btnSystemInfo.addEventListener('click', showSystemInfoModal);

  // Sidebar nav buttons
  document.querySelectorAll('.nav-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const action = btn.dataset.action;
      if (action) handleAction(action);
    });
  });

  // Quick action buttons (delegated for dynamic content)
  function bindQuickActions() {
    document.querySelectorAll('.quick-action-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        const action = btn.dataset.action;
        if (action) handleAction(action);
      });
    });
  }
  bindQuickActions();

  // Also use event delegation on chat area for dynamically created quick actions
  chatArea.addEventListener('click', (e) => {
    const qab = e.target.closest('.quick-action-btn');
    if (qab && qab.dataset.action) {
      handleAction(qab.dataset.action);
    }
  });

  // ══════════════════════════════════════════════════════════════════════════
  //  INITIALIZATION
  // ══════════════════════════════════════════════════════════════════════════

  // Focus input
  messageInput.focus();

  // Start WebSocket connection
  connectWebSocket();

  // Check OpenClaw status
  if (window.tojoAPI) {
    window.tojoAPI.getOpenClawStatus().then((status) => {
      updateConnectionStatus('openclaw', status.available);
    });
  }

  console.log('[tojo] Frontend initialized.');
})();
