/**
 * UI helpers: connection status, system info modal, and utility functions.
 */

import type { SystemInfo } from '../shared/types';

let statusBackend: HTMLElement;
let statusOpenClaw: HTMLElement;

export function init(backendEl: HTMLElement, openclawEl: HTMLElement): void {
  statusBackend = backendEl;
  statusOpenClaw = openclawEl;
}

export function updateConnectionStatus(service: 'backend' | 'openclaw', connected: boolean): void {
  const el = service === 'backend' ? statusBackend : statusOpenClaw;
  if (connected) {
    el.classList.add('connected');
  } else {
    el.classList.remove('connected');
  }
}

export function formatLabel(camelCase: string): string {
  return camelCase
    .replace(/([A-Z])/g, ' $1')
    .replace(/^./, (s) => s.toUpperCase())
    .trim();
}

export function showSystemInfoModal(): void {
  const existing = document.querySelector('.modal-overlay');
  if (existing) existing.remove();

  let info: SystemInfo | { note: string };
  if (window.tojoAPI) {
    info = window.tojoAPI.getSystemInfo();
  } else {
    info = { note: 'Running outside Electron - limited info available.' };
  }

  const overlay = document.createElement('div');
  overlay.classList.add('modal-overlay');

  const rows = Object.entries(info)
    .map(
      ([key, val]) => `
      <div class="modal-info-row">
        <span class="modal-info-label">${formatLabel(key)}</span>
        <span class="modal-info-value">${String(val)}</span>
      </div>
    `,
    )
    .join('');

  overlay.innerHTML = `
    <div class="modal">
      <h3>System Information</h3>
      ${rows}
      <button class="modal-close">Close</button>
    </div>
  `;

  document.body.appendChild(overlay);

  overlay.querySelector('.modal-close')!.addEventListener('click', () => overlay.remove());
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) overlay.remove();
  });
}
