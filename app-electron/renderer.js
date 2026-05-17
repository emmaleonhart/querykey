'use strict';
// QueryKey desktop renderer.
//
// Talks straight to the local Rust server via fetch() (server CORS is
// allow_origin(Any), verified) — no IPC for data. IPC is only for the
// main-process-managed server status chip.
//
// R20-3: API client. R20-4: Profile view. R20-5: Wiki view.

const content = document.getElementById('content');
const statusEl = document.getElementById('server-status');

// ---------- API client (R20-3) ----------
const API = 'http://127.0.0.1:8000';

async function jget(pathStr) {
  const r = await fetch(API + pathStr);
  if (!r.ok) throw new Error(`GET ${pathStr} -> ${r.status}`);
  return r.json();
}
async function jsend(pathStr, method, body) {
  const r = await fetch(API + pathStr, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body || {}),
  });
  if (!r.ok) throw new Error(`${method} ${pathStr} -> ${r.status} ${await r.text()}`);
  return r.json();
}

const api = {
  // Card (key/query signal). PUT body = CardInput
  // {handle,name,website,bio,offering[],looking_for[],visibility}.
  getCard: () => jget('/api/card'), // -> {card, propagation}
  putCard: (c) => jsend('/api/card', 'PUT', c),
  draftCard: () => jsend('/api/card/draft', 'POST', {}),
  revertCard: () => jsend('/api/card/revert', 'POST', {}),
  // Wiki page-type lists -> [{id,title}] (persons: {id,display_name})
  listPersons: () => jget('/api/persons').then((d) => d.persons || []),
  listProjects: () => jget('/api/projects').then((d) => d.projects || []),
  listNotes: () => jget('/api/notes').then((d) => d.notes || []),
  listEvents: () => jget('/api/events').then((d) => d.events || []),
  // Entity detail -> {kind,id,title,body,frontmatter}
  getEntity: (k, id) =>
    jget(`/api/entities/${encodeURIComponent(k)}/${encodeURIComponent(id)}`),
  listLinks: () => jget('/api/links').then((d) => d.links || []),
  entityLinks: (k, id) =>
    jget(`/api/entities/${encodeURIComponent(k)}/${encodeURIComponent(id)}/links`),
};

// ---------- server status chip (R20-2) ----------
function renderStatus(s) {
  if (!statusEl || !s) return;
  const label =
    {
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

// ---------- views (real content in R20-4 / R20-5) ----------
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
