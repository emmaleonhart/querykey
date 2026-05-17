'use strict';
// QueryKey desktop renderer.
//
// R20-1 shell + nav. R20-2 wires the server-status chip from the
// main-process-managed Rust server. R20-3 adds the API client; R20-4
// the Profile view; R20-5 the Wiki view. Data goes straight to the
// local server via fetch() — no IPC for data.

const content = document.getElementById('content');
const statusEl = document.getElementById('server-status');

// ---- server status chip (R20-2) ----
function renderStatus(s) {
  if (!statusEl || !s) return;
  const label = {
    starting: '● starting…',
    ok: '● connected',
    error: '● server error',
    stopping: '● stopping…',
    unknown: '● unknown',
  }[s.state] || '● …';
  statusEl.textContent = label;
  statusEl.classList.toggle('ok', s.state === 'ok');
  statusEl.classList.toggle('err', s.state === 'error');
  statusEl.title = s.detail || s.state || '';
}

if (window.qk) {
  window.qk.serverStatus().then(renderStatus).catch(() => {});
  window.qk.onServerStatus(renderStatus);
}

// ---- views (real content in R20-4 / R20-5) ----
const views = {
  profile() {
    content.innerHTML =
      '<h1>Profile</h1><div class="placeholder">Card view lands in R20-4.</div>';
  },
  wiki() {
    content.innerHTML =
      '<h1>Wiki</h1><div class="placeholder">Vault browser lands in R20-5.</div>';
  },
};

function show(view) {
  document.querySelectorAll('.navbtn').forEach((b) => {
    b.classList.toggle('active', b.dataset.view === view);
  });
  (views[view] || views.profile)();
}

document.querySelectorAll('.navbtn').forEach((b) => {
  b.addEventListener('click', () => show(b.dataset.view));
});

show('profile');
