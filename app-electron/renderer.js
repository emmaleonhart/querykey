'use strict';
// QueryKey desktop renderer.
//
// R20-1: shell + nav only. R20-3 adds the API client; R20-4 the
// Profile view; R20-5 the Wiki view. Renderer talks straight to the
// local server via fetch() (set up in R20-3) — no IPC for data.

const content = document.getElementById('content');

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
