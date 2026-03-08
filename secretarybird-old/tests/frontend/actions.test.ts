import { describe, it, expect, vi, beforeEach } from 'vitest';
import { init, handleAction } from '../../electron/src/renderer/actions';
import * as chat from '../../electron/src/renderer/chat';

// Mock chat module
vi.mock('../../electron/src/renderer/chat', () => ({
  getHistory: vi.fn(() => []),
}));

describe('actions', () => {
  let sendMsg: ReturnType<typeof vi.fn>;
  let sendBackend: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    sendMsg = vi.fn();
    sendBackend = vi.fn();
    init(sendMsg, sendBackend);
    // Clear window.tojoAPI
    (window as any).tojoAPI = undefined;
  });

  it('sends a message for organize-files when no tojoAPI', async () => {
    await handleAction('organize-files');
    expect(sendMsg).toHaveBeenCalledWith(
      expect.stringContaining('organize'),
    );
  });

  it('sends a message for check-spreadsheet when no tojoAPI', async () => {
    await handleAction('check-spreadsheet');
    expect(sendMsg).toHaveBeenCalledWith(
      expect.stringContaining('spreadsheet'),
    );
  });

  it('sends a message for analyze-data', async () => {
    await handleAction('analyze-data');
    expect(sendMsg).toHaveBeenCalledWith(
      expect.stringContaining('analyze'),
    );
  });

  it('handles analyze-competitors action', async () => {
    await handleAction('analyze-competitors');
    expect(sendMsg).toHaveBeenCalled();
    expect(sendBackend).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'message',
        context: { handler: 'competitor_analysis' },
      }),
    );
  });

  it('handles blue-ocean action', async () => {
    await handleAction('blue-ocean');
    expect(sendMsg).toHaveBeenCalled();
    expect(sendBackend).toHaveBeenCalledWith(
      expect.objectContaining({
        context: { handler: 'competitor_analysis' },
      }),
    );
  });

  it('handles scrape-competitor action', async () => {
    await handleAction('scrape-competitor');
    expect(sendMsg).toHaveBeenCalled();
    expect(sendBackend).toHaveBeenCalled();
  });

  it('does nothing for unknown action', async () => {
    await handleAction('unknown-action');
    expect(sendMsg).not.toHaveBeenCalled();
    expect(sendBackend).not.toHaveBeenCalled();
  });

  it('handles select-file when no tojoAPI', async () => {
    await handleAction('select-file');
    // Without tojoAPI, should do nothing
    expect(sendMsg).not.toHaveBeenCalled();
  });

  it('handles select-folder when no tojoAPI', async () => {
    await handleAction('select-folder');
    expect(sendMsg).not.toHaveBeenCalled();
  });
});
