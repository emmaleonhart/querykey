'use strict';
// QueryKey desktop renderer.
//
// Talks straight to the local Rust server via fetch() (server CORS is
// allow_origin(Any), verified) — no IPC for data. IPC only for the
// server-status chip; markdown via window.md (preload-exposed marked).
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
function bullets(arr) {
  return (
    (arr || []).map((x) => `<div class="bullet">• ${esc(x)}</div>`).join('') ||
    '<div class="muted">—</div>'
  );
}
// Mirrors server vault::slug + the old Flutter _slug: lowercase,
// non-alphanumeric runs -> single '-', trailing '-' trimmed.
function slug(s) {
  let out = '';
  let dash = false;
  for (const ch of String(s == null ? '' : s).toLowerCase().trim()) {
    if (/[a-z0-9]/.test(ch)) {
      out += ch;
      dash = false;
    } else if (out && !dash) {
      out += '-';
      dash = true;
    }
  }
  return out.replace(/-+$/, '');
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
let _card = null;
let _prop = null;

function propLine(p) {
  if (!p) return '';
  let text;
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
  return `<div class="prop">${esc(text)}</div>`;
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
    (c.website ? `<div style="margin-top:6px">${esc(c.website)}</div>` : '') +
    (c.bio ? `<p style="margin-top:12px">${esc(c.bio)}</p>` : '') +
    `<h2>Offering (key)</h2>${bullets(c.offering)}` +
    `<h2>Looking for (query)</h2>${bullets(c.looking_for)}` +
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
      offering: document.getElementById('f-off').value.split('\n').map((x) => x.trim()).filter(Boolean),
      looking_for: document.getElementById('f-look').value.split('\n').map((x) => x.trim()).filter(Boolean),
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

// ---------- Wiki view (R20-5) ----------
// Parity with the retired Flutter wiki_screen.dart: type picker ->
// list -> entity detail with markdown + [[wikilink]] click-through
// (resolved via /api/links) + backlinks.

const WIKI_TYPES = [
  { kind: 'person', label: 'Contacts' },
  { kind: 'project', label: 'Projects' },
  { kind: 'note', label: 'Notes' },
  { kind: 'event', label: 'Events' },
];
const KIND_LABEL = { person: 'Contacts', project: 'Projects', note: 'Notes', event: 'Events' };

let _allLinks = []; // cached resolved-edge graph for wikilink resolution

function wikiPicker() {
  content.innerHTML =
    `<h1>Wiki</h1>` +
    WIKI_TYPES.map(
      (t) => `<div class="list-item" data-kind="${t.kind}"><span>${t.label}</span><span class="muted">›</span></div>`,
    ).join('');
  content.querySelectorAll('.list-item').forEach((el) => {
    el.onclick = () => wikiList(el.dataset.kind);
  });
}

async function listForKind(kind) {
  if (kind === 'person') {
    const ps = await api.listPersons();
    return ps.map((p) => ({ id: p.id, title: p.display_name || p.id }));
  }
  if (kind === 'project') return api.listProjects();
  if (kind === 'note') return api.listNotes();
  if (kind === 'event') return api.listEvents();
  return [];
}

async function wikiList(kind) {
  const label = KIND_LABEL[kind] || kind;
  content.innerHTML =
    `<div class="crumbs"><a id="b-wiki">Wiki</a> › ${esc(label)}</div>` +
    `<div class="placeholder">Loading…</div>`;
  document.getElementById('b-wiki').onclick = wikiPicker;
  try {
    const items = await listForKind(kind);
    const crumb = `<div class="crumbs"><a id="b-wiki">Wiki</a> › ${esc(label)}</div>`;
    if (!items.length) {
      content.innerHTML = crumb + `<div class="empty">No ${esc(label)} yet.</div>`;
      document.getElementById('b-wiki').onclick = wikiPicker;
      return;
    }
    content.innerHTML =
      crumb +
      items
        .map(
          (it) =>
            `<div class="list-item" data-id="${esc(it.id)}"><span>${esc(it.title || it.id)}</span>` +
            `<span class="sub">${esc(it.id)}</span></div>`,
        )
        .join('');
    document.getElementById('b-wiki').onclick = wikiPicker;
    content.querySelectorAll('.list-item').forEach((el) => {
      el.onclick = () => wikiDetail(kind, el.dataset.id, el.querySelector('span').textContent);
    });
  } catch (e) {
    content.innerHTML =
      `<div class="crumbs"><a id="b-wiki">Wiki</a> › ${esc(label)}</div>` +
      `<div class="err-banner">Could not load ${esc(label)}.</div><div class="muted">${esc(e.message)}</div>`;
    document.getElementById('b-wiki').onclick = wikiPicker;
  }
}

// [[Target]] / [[pred:Target]] -> resolved markdown link or dimmed.
function resolveWikilink(raw) {
  const target = raw.includes(':') ? raw.split(':').slice(1).join(':') : raw;
  const tslug = slug(target.trim());
  for (const lk of _allLinks) {
    if (!lk || lk.resolved !== true) continue;
    if (slug(lk.to_label) === tslug || slug(lk.to_id) === tslug) {
      return { kind: lk.to_kind, id: lk.to_id };
    }
  }
  return null;
}
function preprocessWikilinks(body) {
  return String(body || '').replace(/\[\[([^\]]+)\]\]/g, (_m, raw) => {
    const r = resolveWikilink(raw);
    if (r) return `[${raw}](qkwiki://${r.kind}/${encodeURIComponent(r.id)})`;
    return `*${raw}*`; // dangling -> emphasis, non-tappable
  });
}

async function wikiDetail(kind, id, title) {
  const label = KIND_LABEL[kind] || kind;
  content.innerHTML =
    `<div class="crumbs"><a id="b-wiki">Wiki</a> › <a id="b-list">${esc(label)}</a> › ${esc(title || id)}</div>` +
    `<div class="placeholder">Loading…</div>`;
  document.getElementById('b-wiki').onclick = wikiPicker;
  document.getElementById('b-list').onclick = () => wikiList(kind);
  try {
    const [page, links, allLinks] = await Promise.all([
      api.getEntity(kind, id),
      api.entityLinks(kind, id).catch(() => ({ to: [] })),
      api.listLinks().catch(() => []),
    ]);
    _allLinks = allLinks || [];
    const bodyHtml = window.md
      ? window.md.parse(preprocessWikilinks(page.body || ''))
      : esc(page.body || '');
    const backlinks = (links && links.to) || [];
    const crumb =
      `<div class="crumbs"><a id="b-wiki">Wiki</a> › <a id="b-list">${esc(label)}</a> › ${esc(page.title || title || id)}</div>`;
    content.innerHTML =
      crumb +
      `<div class="detail"><h1>${esc(page.title || title || id)}</h1>` +
      `<div class="markdown">${bodyHtml}</div>` +
      (backlinks.length
        ? `<h2>Backlinks</h2>` +
          backlinks
            .map((b) => {
              const fk = b.from_kind || '';
              const fi = b.from_id || '';
              const pr = b.predicate || 'references';
              return `<div class="list-item" data-k="${esc(fk)}" data-i="${esc(fi)}"><span>${esc(fk)}:${esc(fi)}</span><span class="sub">${esc(pr)}</span></div>`;
            })
            .join('')
        : '') +
      `</div>`;
    document.getElementById('b-wiki').onclick = wikiPicker;
    document.getElementById('b-list').onclick = () => wikiList(kind);
    // in-app wikilink navigation
    content.querySelectorAll('a[href^="qkwiki://"]').forEach((a) => {
      a.addEventListener('click', (ev) => {
        ev.preventDefault();
        const rest = a.getAttribute('href').replace('qkwiki://', '');
        const slash = rest.indexOf('/');
        if (slash < 0) return;
        const k = rest.slice(0, slash);
        const i = decodeURIComponent(rest.slice(slash + 1));
        wikiDetail(k, i, i);
      });
    });
    content.querySelectorAll('.list-item[data-k]').forEach((el) => {
      el.onclick = () => wikiDetail(el.dataset.k, el.dataset.i, el.dataset.i);
    });
  } catch (e) {
    content.innerHTML =
      `<div class="crumbs"><a id="b-wiki">Wiki</a> › <a id="b-list">${esc(label)}</a></div>` +
      `<div class="err-banner">Could not load page.</div><div class="muted">${esc(e.message)}</div>`;
    document.getElementById('b-wiki').onclick = wikiPicker;
    document.getElementById('b-list').onclick = () => wikiList(kind);
  }
}

// ---------- nav ----------
const views = { profile: loadProfile, wiki: wikiPicker };

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
