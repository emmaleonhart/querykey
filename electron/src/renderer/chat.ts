/**
 * Chat UI: message rendering, history, and streaming support.
 */

import type { ChatEntry } from '../shared/types';
import { renderMarkdown } from './markdown';

const AVATAR_SRC = '../../assets/tojo-avatar.png';

let chatArea: HTMLElement;
let typingIndicator: HTMLElement;
let chatHistory: ChatEntry[] = [];
let welcomeVisible = true;

// Streaming state
let streamBuffer = '';
let streamBubble: HTMLElement | null = null;

export function init(chatEl: HTMLElement, typingEl: HTMLElement): void {
  chatArea = chatEl;
  typingIndicator = typingEl;
}

export function getHistory(): ChatEntry[] {
  return chatHistory;
}

export function formatTime(date: Date): string {
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function scrollToBottom(): void {
  requestAnimationFrame(() => {
    chatArea.scrollTop = chatArea.scrollHeight;
  });
}

function removeWelcomeCard(): void {
  if (!welcomeVisible) return;
  const card = chatArea.querySelector('.welcome-card');
  if (card) card.remove();
  welcomeVisible = false;
}

export function appendUserMessage(text: string): void {
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

export function appendAssistantMessage(text: string): void {
  removeWelcomeCard();
  chatHistory.push({ role: 'assistant', content: text, time: new Date() });
  createAssistantBubble(text);
  scrollToBottom();
}

export function createAssistantBubble(text: string): HTMLElement {
  const wrapper = document.createElement('div');
  wrapper.classList.add('message', 'assistant');

  const avatar = document.createElement('img');
  avatar.classList.add('message-avatar');
  avatar.src = AVATAR_SRC;
  avatar.alt = 'Sakuya';

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

export function showTypingIndicator(): void {
  typingIndicator.classList.remove('hidden');
  scrollToBottom();
}

export function hideTypingIndicator(): void {
  typingIndicator.classList.add('hidden');
}

// ── Streaming ─────────────────────────────────────────────────────────────────

export function appendStreamChunk(text: string): void {
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

export function finalizeStream(): void {
  if (streamBuffer) {
    chatHistory.push({ role: 'assistant', content: streamBuffer, time: new Date() });
  }
  streamBuffer = '';
  streamBubble = null;
}

// ── Clear ─────────────────────────────────────────────────────────────────────

export function clearChat(onQuickActionsBound: () => void): void {
  chatArea.innerHTML = '';
  chatHistory = [];
  welcomeVisible = true;
  streamBuffer = '';
  streamBubble = null;

  chatArea.innerHTML = `
    <div class="welcome-card">
      <img src="${AVATAR_SRC}" alt="Izayoi Sakuya" class="welcome-avatar" />
      <h2>Welcome. I am Sakuya Assistant.</h2>
      <p>I shall handle everything. Please tell me what you need, and I will attend to it promptly and precisely.</p>
      <div class="quick-actions">
        <button class="quick-action-btn" data-action="organize-files">Organize my files</button>
        <button class="quick-action-btn" data-action="check-spreadsheet">Check a spreadsheet</button>
        <button class="quick-action-btn" data-action="analyze-data">Analyze data</button>
        <button class="quick-action-btn" data-action="analyze-competitors">Analyze competitors</button>
        <button class="quick-action-btn" data-action="build-pipeline">Build a pipeline</button>
      </div>
    </div>
  `;
  onQuickActionsBound();
}
