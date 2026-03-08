import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setMessageHandler, setStatusHandler, send, isConnected } from '../../electron/src/renderer/websocket';

describe('websocket', () => {
  describe('isConnected', () => {
    it('returns false when not connected', () => {
      expect(isConnected()).toBe(false);
    });
  });

  describe('send', () => {
    it('returns false when not connected', () => {
      const result = send({ type: 'message', content: 'hello' });
      expect(result).toBe(false);
    });
  });

  describe('setMessageHandler', () => {
    it('accepts a handler function without error', () => {
      const handler = vi.fn();
      expect(() => setMessageHandler(handler)).not.toThrow();
    });
  });

  describe('setStatusHandler', () => {
    it('accepts a handler function without error', () => {
      const handler = vi.fn();
      expect(() => setStatusHandler(handler)).not.toThrow();
    });
  });
});
