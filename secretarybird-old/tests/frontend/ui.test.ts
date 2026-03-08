import { describe, it, expect, beforeEach } from 'vitest';
import { init, updateConnectionStatus, formatLabel } from '../../electron/src/renderer/ui';

describe('ui', () => {
  let statusBackend: HTMLSpanElement;
  let statusOpenClaw: HTMLSpanElement;

  beforeEach(() => {
    statusBackend = document.createElement('span');
    statusOpenClaw = document.createElement('span');
    init(statusBackend, statusOpenClaw);
  });

  describe('updateConnectionStatus', () => {
    it('adds connected class when connected', () => {
      updateConnectionStatus('backend', true);
      expect(statusBackend.classList.contains('connected')).toBe(true);
    });

    it('removes connected class when disconnected', () => {
      statusBackend.classList.add('connected');
      updateConnectionStatus('backend', false);
      expect(statusBackend.classList.contains('connected')).toBe(false);
    });

    it('works for openclaw status', () => {
      updateConnectionStatus('openclaw', true);
      expect(statusOpenClaw.classList.contains('connected')).toBe(true);
    });
  });

  describe('formatLabel', () => {
    it('converts camelCase to title case', () => {
      expect(formatLabel('backendStatus')).toBe('Backend Status');
    });

    it('handles single word', () => {
      expect(formatLabel('platform')).toBe('Platform');
    });

    it('handles multiple capitals', () => {
      expect(formatLabel('totalMemoryGB')).toBe('Total Memory G B');
    });
  });
});
