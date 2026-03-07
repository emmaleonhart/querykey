import { describe, it, expect, beforeEach } from 'vitest';
import {
  init,
  getHistory,
  appendUserMessage,
  appendAssistantMessage,
  showTypingIndicator,
  hideTypingIndicator,
  appendStreamChunk,
  finalizeStream,
  clearChat,
  formatTime,
} from '../../electron/src/renderer/chat';

describe('chat', () => {
  let chatArea: HTMLDivElement;
  let typingIndicator: HTMLDivElement;

  beforeEach(() => {
    chatArea = document.createElement('div');
    typingIndicator = document.createElement('div');
    typingIndicator.classList.add('hidden');
    document.body.innerHTML = '';
    document.body.appendChild(chatArea);
    document.body.appendChild(typingIndicator);
    init(chatArea, typingIndicator);
    // Reset internal state
    clearChat(() => {});
  });

  describe('formatTime', () => {
    it('formats a date as HH:MM', () => {
      const result = formatTime(new Date(2026, 0, 1, 14, 30));
      expect(result).toMatch(/\d{1,2}:\d{2}/);
    });
  });

  describe('getHistory', () => {
    it('starts with empty history', () => {
      expect(getHistory()).toEqual([]);
    });
  });

  describe('appendUserMessage', () => {
    it('adds a user message bubble to the chat area', () => {
      appendUserMessage('Hello Tojo');
      const bubble = chatArea.querySelector('.message.user .message-bubble');
      expect(bubble).not.toBeNull();
      expect(bubble!.textContent).toContain('Hello Tojo');
    });

    it('adds to history', () => {
      appendUserMessage('Test message');
      const history = getHistory();
      expect(history).toHaveLength(1);
      expect(history[0].role).toBe('user');
      expect(history[0].content).toBe('Test message');
    });
  });

  describe('appendAssistantMessage', () => {
    it('adds an assistant message bubble with avatar', () => {
      appendAssistantMessage('I shall handle it.');
      const wrapper = chatArea.querySelector('.message.assistant');
      expect(wrapper).not.toBeNull();
      const avatar = wrapper!.querySelector('.message-avatar');
      expect(avatar).not.toBeNull();
      const bubble = wrapper!.querySelector('.message-bubble');
      expect(bubble).not.toBeNull();
    });

    it('adds to history', () => {
      appendAssistantMessage('Response text');
      const history = getHistory();
      expect(history).toHaveLength(1);
      expect(history[0].role).toBe('assistant');
    });
  });

  describe('typing indicator', () => {
    it('shows typing indicator', () => {
      showTypingIndicator();
      expect(typingIndicator.classList.contains('hidden')).toBe(false);
    });

    it('hides typing indicator', () => {
      showTypingIndicator();
      hideTypingIndicator();
      expect(typingIndicator.classList.contains('hidden')).toBe(true);
    });
  });

  describe('streaming', () => {
    it('accumulates stream chunks', () => {
      appendStreamChunk('Hello ');
      appendStreamChunk('world');
      const bubbles = chatArea.querySelectorAll('.message.assistant');
      expect(bubbles).toHaveLength(1);
    });

    it('finalizes stream into history', () => {
      appendStreamChunk('Streamed content');
      finalizeStream();
      const history = getHistory();
      expect(history).toHaveLength(1);
      expect(history[0].content).toBe('Streamed content');
    });
  });

  describe('clearChat', () => {
    it('clears messages and resets history', () => {
      appendUserMessage('Test');
      appendAssistantMessage('Reply');
      let callbackCalled = false;
      clearChat(() => { callbackCalled = true; });
      expect(getHistory()).toEqual([]);
      expect(chatArea.querySelector('.welcome-card')).not.toBeNull();
      expect(callbackCalled).toBe(true);
    });
  });
});
