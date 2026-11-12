
(() => {
  const input = document.getElementById('docs-search');
  const panel = document.getElementById('search-panel');
  if (!input || !panel) return;
  let docs = [];
  let loading = null;
  const load = async () => {
    if (docs.length > 0) return;
    if (!loading) {
      loading = fetch('search.json')
        .then((response) => response.json())
        .catch(() => [])
        .finally(() => { loading = null; });
    }
    docs = await loading;
  };
  const escapeHtml = (value) => String(value).replace(/[&<>"']/g, (char) => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
  const search = () => {
    const query = input.value.trim().toLowerCase();
    if (!query) { panel.hidden = true; panel.innerHTML = ''; return; }
    const results = docs.filter((doc) => doc.text.toLowerCase().includes(query)).slice(0, 8);
    panel.hidden = false;
    if (!results.length) {
      panel.innerHTML = '<div class="search-empty">No page matches that search.</div>';
      return;
    }
    panel.innerHTML = results.map((doc) => {
      const index = doc.text.toLowerCase().indexOf(query);
      let start = Math.max(0, index - 55);
      while (start > 0 && !/\s/.test(doc.text[start - 1])) start -= 1;
      let end = Math.min(doc.text.length, index + query.length + 110);
      while (end < doc.text.length && !/\s/.test(doc.text[end])) end += 1;
      const snippet = (start > 0 ? '... ' : '') + doc.text.slice(start, end).trim() + (end < doc.text.length ? ' ...' : '');
      return `<a class="search-result" href="${doc.slug}.html"><strong>${escapeHtml(doc.title)}</strong><span>${escapeHtml(doc.group)} · ${escapeHtml(snippet)}</span></a>`;
    }).join('');
  };
  input.addEventListener('focus', load);
  input.addEventListener('input', async () => { await load(); search(); });
  input.addEventListener('keydown', (event) => { if (event.key === 'Escape') { input.value = ''; search(); input.blur(); } });
  document.addEventListener('keydown', (event) => {
    if (event.key === '/' && document.activeElement !== input) { event.preventDefault(); input.focus(); }
  });
})();
