/**
 * Action routing for sidebar nav buttons and quick actions.
 */

import type { ActionId, WSPayload } from '../shared/types';
import * as chat from './chat';

type SendFn = (text: string) => void;
type BackendSendFn = (payload: WSPayload) => void;

let sendUserMessage: SendFn;
let sendToBackend: BackendSendFn;

export function init(sendMsg: SendFn, sendBackend: BackendSendFn): void {
  sendUserMessage = sendMsg;
  sendToBackend = sendBackend;
}

/** Static messages for simple actions */
const ACTION_MESSAGES: Partial<Record<ActionId, string>> = {
  'organize-files': 'Please organize my files. I will select a folder.',
  'check-spreadsheet': 'Please check a spreadsheet for errors. I will select a file.',
  'analyze-data': 'I would like to analyze some data. What format is it in?',
  'export-report': 'Please export a report from the current data.',
  // Integrations (disabled - not yet implemented)
  // 'connect-salesforce': 'I would like to connect to Salesforce. Please guide me through the setup.',
  // 'connect-gsheets': 'I would like to connect to Google Sheets.',
  // 'connect-database': 'I would like to connect to a database. What connection details are needed?',
  // Pipeline (disabled - not yet implemented)
  // 'build-pipeline': 'I would like to build a data pipeline. Please help me configure the steps.',
  // 'run-pipeline': 'Please run the configured data pipeline.',
  // 'view-logs': 'Show me the recent pipeline execution logs.',
};

export async function handleAction(action: string): Promise<void> {
  const api = window.tojoAPI;

  // Competitor analysis actions (Strategy - top priority)
  if (action === 'analyze-competitors') {
    sendUserMessage(
      'I would like to run a competitor analysis with Blue Ocean Strategy. ' +
      'Please help me identify my competitors and analyze the competitive landscape.',
    );
    sendToBackend({
      type: 'message',
      content: 'Start competitor analysis',
      context: { handler: 'competitor_analysis' },
      history: chat.getHistory().slice(-20),
    });
    return;
  }

  if (action === 'blue-ocean') {
    sendUserMessage(
      'I want to create a Blue Ocean Strategy Canvas for my business. ' +
      'Help me identify uncontested market space and strategic opportunities.',
    );
    sendToBackend({
      type: 'message',
      content: 'Blue Ocean Strategy analysis',
      context: { handler: 'competitor_analysis' },
      history: chat.getHistory().slice(-20),
    });
    return;
  }

  if (action === 'scrape-competitor') {
    sendUserMessage(
      'I want to scout a specific competitor. ' +
      'Please tell me their website URL and I will gather intelligence on them.',
    );
    sendToBackend({
      type: 'message',
      content: 'Scout a competitor',
      context: { handler: 'competitor_analysis' },
      history: chat.getHistory().slice(-20),
    });
    return;
  }

  // File/folder pickers
  if (action === 'select-file') {
    if (api) {
      const filePath = await api.selectFile();
      if (filePath) sendUserMessage(`I selected this file: ${filePath}`);
    }
    return;
  }

  if (action === 'select-folder') {
    if (api) {
      const folderPath = await api.selectFolder();
      if (folderPath) sendUserMessage(`I selected this folder: ${folderPath}`);
    }
    return;
  }

  // Actions that open a picker first
  if (action === 'organize-files' && api) {
    const folderPath = await api.selectFolder();
    if (folderPath) {
      sendUserMessage(`Please organize the files in this folder: ${folderPath}`);
      return;
    }
  }

  if (action === 'check-spreadsheet' && api) {
    const filePath = await api.selectFile({
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
  const msg = ACTION_MESSAGES[action as ActionId];
  if (msg) {
    sendUserMessage(msg);
  }
}
