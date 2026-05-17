'use strict';
// QueryKey desktop renderer.
//
// Talks straight to the local Rust server via fetch() (server CORS is
// allow_origin(Any), verified) — no IPC for data. IPC only for the
// main-process-managed server-status chip.
//
// R20-3 API client · R20-4 Profile view · R20-5 Wiki view.

const content = document.getElementById('content');
const statusEl = document.getElementById('server-status');

// ---------- API client (R20-3) ----------
const API = 'http://127.0.0.1:8000';

async function jget(p) {
  const r = await fetch(API + p);
  if (!r.ok) throw new Error(`GET ${p} -> ${r.status}`);
  return r.json();
}
async function jsend(p, method, body) {
  const r = await fetch(API + p, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body || {}),
  });
  if (!r.ok) throw new Error(`${method} ${p} -> ${r.status} ${await r.text()}`);
  return r.json();
}

const api = {
  getCard: () => jget('/api/card'),
  putCard: (c) => jsend('/api/card', 'PUT', c),
  draftCard: () => jsend('/api/card/draft', 'POST', {}),
  revertCard: () => jsend('/api/card/revert', 'POST', {}),
  listPersons: () => jget('/api/persons').then((d) => d.persons || []),
  listProjects: () => jget('/api/projects').then((d) => d.projects || []),
  listNotes: () => jget('/api/notes').then((d) => d.notes || []),
  listEvents: () => jget('/api/events').then((d) => d.events || []),
  getEntity: (k, id) =>
    jget(`/api/entities/${encodeURIComponent(k)}/${encodeURIComponent(id)}`),
  listLinks: () => jget('/api/links').then((d) => d.links || []),
  entityLinks: (k, id) =>
    jget(`/api/entities/${encodeURIComponent(k)}/${encodeURIComponent(id)}/links`),
};

// ---------- helpers ----------
function esc(s) {
  return String(s == null ? '' : s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
function lines(arr) {
  return (arr || []).map((x) => `<div class="bullet">• ${esc(x)}</div>`).join('') || '<div class="muted">—</div>';
}

// ---------- server status chip (R20-2) ----------
function renderStatus(s) {
  if (!statusEl || !s) return;
  const label =
    { starting: '● starting…', ok: '● connected', error: '● server error', stopping: '● stopping…', unknown: '● unknown' }[
      s.state
    ] || '● …';
  statusEl.textContent = label;
  statusEl.classList.toggle('ok', s.state === 'ok');
  statusEl.classList.toggle('err', s.state === 'error');
  statusEl.title = s.detail || s.state || '';
}
if (window.qk) {
  window.qk.serverStatus().then(renderStatus).catch(() => {});
  window.qk.onServerStatus(renderStatus);
}

// ---------- Profile view (R20-4) ----------
// Parity with the retired Flutter profile_screen.dart: view / edit /
// empty states, draft-with-agent, revert, and the 24h propagation
// valve SURFACED (never bypassed).

let _card = null;
let _prop = null;

function propLine(p) {
  if (!p) return '';
  let text;
  let cls = 'muted';
  if (p.pending) {
    if (p.eligible_at) {
      const ms = new Date(p.eligible_at).getTime() - Date.now();
      if (ms > 0) {
        const h = Math.floor(ms / 3600000);
        const m = Math.floor((ms % 3600000) / 60000);
        text = `Pending — eligible in ${h}h ${m}m (24h safety valve)`;
      } else {
        text = 'Pending — eligible now (24h safety valve elapsed)';
      }
    } else {
      text = 'Pending — 24h propagation delay active';
    }
  } else if (p.published) {
    text = 'Published';
  } else {
    text = 'Not yet published';
  }
  return `<div class="prop ${cls}">${esc(text)}</div>`;
}

function profileError(msg) {
  content.innerHTML =
    `<h1>Profile</h1><div class="err-banner">Could not load card.</div>` +
    `<div class="muted">${esc(msg)}</div>` +
    `<button class="action" id="p-retry">Retry</button>`;
  document.getElementById('p-retry').onclick = loadProfile;
}

function renderProfileView() {
  const c = _card;
  if (!c) {
    content.innerHTML =
      `<h1>Profile</h1><div class="empty">No card yet — your key/query signal for the P2P layer.</div>` +
      `<div><button class="action primary" id="p-create">Create card</button>` +
      `<button class="action" id="p-draft">Draft with agent</button></div>`;
    document.getElementById('p-create').onclick = () => renderProfileEdit({});
    document.getElementById('p-draft').onclick = doDraft;
    return;
  }
  content.innerHTML =
    `<div class="card-box">` +
    `<h1>${esc(c.name || c.handle)}</h1>` +
    `<div class="muted">${esc(c.handle)}${c.visibility ? `<span class="tag">${esc(c.visibility)}</span>` : ''}</div>` +
    (c.website ? `<div style="margin-top:6px"><a id="p-web">${esc(c.website)}</a></div>` : '') +
    (c.bio ? `<p style="margin-top:12px">${esc(c.bio)}</p>` : '') +
    `<h2>Offering (key)</h2>${lines(c.offering)}` +
    `<h2>Looking for (query)</h2>${lines(c.looking_for)}` +
    propLine(_prop) +
    (c.updated ? `<div class="muted">updated ${esc(c.updated)}</div>` : '') +
    `<div style="margin-top:18px">` +
    `<button class="action primary" id="p-edit">Edit card</button>` +
    `<button class="action" id="p-draft">Draft with agent</button>` +
    (_prop && _prop.pending ? `<button class="action" id="p-revert">Revert</button>` : '') +
    `</div></div>`;
  document.getElementById('p-edit').onclick = () => renderProfileEdit(c);
  document.getElementById('p-draft').onclick = doDraft;
  const rv = document.getElementById('p-revert');
  if (rv) rv.onclick = doRevert;
}

function renderProfileEdit(c) {
  content.innerHTML =
    `<div class="card-box"><h1>Edit card</h1>` +
    `<label>Handle (required)</label><input id="f-handle" value="${esc(c.handle || '')}" placeholder="github:yourusername" />` +
    `<label>Display name</label><input id="f-name" value="${esc(c.name || '')}" />` +
    `<label>Website</label><input id="f-web" value="${esc(c.website || '')}" placeholder="https://…" />` +
    `<label>Bio (one line)</label><input id="f-bio" value="${esc(c.bio || '')}" />` +
    `<label>Offering (key) — one per line</label><textarea id="f-off">${esc((c.offering || []).join('\n'))}</textarea>` +
    `<label>Looking for (query) — one per line</label><textarea id="f-look">${esc((c.looking_for || []).join('\n'))}</textarea>` +
    `<div id="f-err" class="err-banner"></div>` +
    `<div style="margin-top:12px">` +
    `<button class="action primary" id="f-save">Save card</button>` +
    `<button class="action" id="f-cancel">Cancel</button>` +
    `<button class="action" id="f-draft">Draft with agent</button>` +
    `</div>` +
    `<div class="muted" style="margin-top:10px">Saving stages the edit behind the 24h propagation valve — it does not broadcast immediately.</div>` +
    `</div>`;
  document.getElementById('f-cancel').onclick = renderProfileView;
  document.getElementById('f-draft').onclick = doDraft;
  document.getElementById('f-save').onclick = async () => {
    const handle = document.getElementById('f-handle').value.trim();
    if (!handle) {
      document.getElementById('f-err').textContent = 'Handle is required.';
      return;
    }
    const body = {
      handle,
      name: document.getElementById('f-name').value.trim(),
      website: document.getElementById('f-web').value.trim(),
      bio: document.getElementById('f-bio').value.trim(),
      offering: document
        .getElementById('f-off')
        .value.split('\n')
        .map((x) => x.trim())
        .filter(Boolean),
      looking_for: document
        .getElementById('f-look')
        .value.split('\n')
        .map((x) => x.trim())
        .filter(Boolean),
      visibility: (c && c.visibility) || 'public',
    };
    try {
      await api.putCard(body);
      await loadProfile();
    } catch (e) {
      document.getElementById('f-err').textContent = 'Save failed: ' + e.message;
    }
  };
}

async function doDraft() {
  try {
    const d = await api.draftCard();
    const draft = d.draft || {};
    renderProfileEdit({
      handle: draft.handle || (_card && _card.handle) || '',
      name: draft.name || (_card && _card.name) || '',
      website: draft.website || (_card && _card.website) || '',
      bio: draft.bio || '',
      offering: draft.offering || [],
      looking_for: draft.looking_for || [],
      visibility: (_card && _card.visibility) || 'public',
    });
    const err = document.getElementById('f-err');
    if (err) err.textContent = `Draft generated (source: ${esc(d.source || 'agent')}). Review, then Save.`;
  } catch (e) {
    alert('Draft failed: ' + e.message);
  }
}

async function doRevert() {
  if (!confirm('Revert? Discards the pending staged edit and restores the last published version.')) return;
  try {
    await api.revertCard();
    await loadProfile();
  } catch (e) {
    alert('Revert failed: ' + e.message);
  }
}

async function loadProfile() {
  content.innerHTML = '<h1>Profile</h1><div class="placeholder">Loading…</div>';
  try {
    const d = await api.getCard();
    _card = d.card || null;
    _prop = d.propagation || null;
    renderProfileView();
  } catch (e) {
    profileError(e.message);
  }
}

// ---------- Wiki view (real content in R20-5) ----------
const views = {
  profile: loadProfile,
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
