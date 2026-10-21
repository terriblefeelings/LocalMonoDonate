/* LocalMonoDonate — shared header, identical on every page:
   [Dark|Light] [EN|UA] [Setup · Alerts · Bits · Donate · Timer · Auction] [Save]
   - window.savePage()      : handled by Save
   - window.applyLang()     : page hook on EN/UA change (page i18n)
   - window.extraTools(el)  : extra page buttons
   - window.HEAD_THEMES     : theme options (default dark/light)
   - window.HEAD_BODY       : 'bg' (default) or 'bitsPageBg'
   - window.mountBgButtons(el, opts) / window.mountGlobalBg(el) */
(function () {
  const root = document.documentElement;
  const $ = id => document.getElementById(id);
  const HEAD = { bg: '', bitsBg: '', bitsPageBg: '' };
  const HEAD_I18N = {
    en: { navSetup: 'Setup', navAlerts: 'Alerts', navBits: 'Sound alerts', navDonate: 'Donation page', navTimer: 'Timer', navAuction: 'Auction', save: 'Save', th_dark: 'Dark', th_light: 'Light' },
    ua: { navSetup: 'Налаштування', navAlerts: 'Сповіщення', navBits: 'Звукові сповіщення', navDonate: 'Сторінка донатів', navTimer: 'Таймер', navAuction: 'Аукціон', save: 'Зберегти', th_dark: 'Темна', th_light: 'Світла' }
  };

  function applyTheme(th) {
    root.dataset.theme = th; try { localStorage.setItem('lm_theme', th); } catch {}
    fetch('/api/theme', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ light: th === 'light' }) }).catch(() => {});
    document.querySelectorAll('#themeseg button').forEach(b => b.classList.toggle('on', b.dataset.th === th));
    applyBgs();
  }
  function applyBgs() {
    const pageBg = (window.HEAD_BODY === 'bitsPageBg') ? HEAD.bitsPageBg : (window.HEAD_PAGE_BG ? HEAD.bg : '');
    document.body.style.backgroundImage = pageBg ? `url("${pageBg}")` : '';
    document.body.classList.toggle('hasbg', !!pageBg);
    const win = document.getElementById('win');
    if (win) {
      win.style.backgroundImage = HEAD.bitsBg ? `url("${HEAD.bitsBg}")` : '';
      const alpha = HEAD.bitsBg ? 1 : (HEAD.bitsPageBg ? 0.95 : 1);
      win.style.setProperty('--win-alpha', alpha);
      win.style.setProperty('--deck-alpha', HEAD.bitsBg ? 1 : (HEAD.bitsPageBg ? 0.95 : 1));
    }
  }
  async function uploadBg(f, which) {
    const safe = f.name.replace(/[^A-Za-z0-9._-]/g, '_');
    try {
      const r = await fetch('/api/upload?name=' + encodeURIComponent(safe), { method: 'POST', body: f });
      const d = await r.json();
      if (!d.ok) return;
      await fetch('/api/bg', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ url: d.url, which }) });
      if (which === 'bits') HEAD.bitsBg = d.url; else if (which === 'bitspage') HEAD.bitsPageBg = d.url; else HEAD.bg = d.url;
      applyBgs();
    } catch {}
  }
  async function clearBg(which) {
    await fetch('/api/bg', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ url: '', which }) });
    if (which === 'bitsdefault') HEAD.bitsBg = ''; else if (which === 'bitspagedefault') HEAD.bitsPageBg = ''; else HEAD.bg = '';
    applyBgs();
  }
  async function loadCfg() { try { const c = await (await fetch('/api/config')).json(); HEAD.bg = c.bg || ''; HEAD.bitsBg = c.bitsBg || ''; HEAD.bitsPageBg = c.bitsPageBg || ''; } catch {} applyBgs(); }
  function mkFile() { const i = document.createElement('input'); i.type = 'file'; i.accept = 'image/*'; i.style.display = 'none'; document.body.append(i); return i; }

  function mountBgButtons(el, opts) {
    opts = opts || {};
    const pageWhich = opts.pageWhich || 'bitspage', deckWhich = opts.deckWhich || 'bits';
    const wrap = document.createElement('span'); wrap.style.display = 'inline-flex'; wrap.style.gap = '8px'; wrap.style.verticalAlign = 'middle';
    wrap.innerHTML =
      '<button class="ib" data-i18n-title="bgPage" title="Bits page background">\u25A4</button>' +
      '<button class="ib" data-i18n-title="bgDeck" title="Streamdeck background">\u25A6</button>' +
      '<button class="ib" data-i18n-title="bgPageDef" title="Bits page background — default">\u21BA</button>' +
      '<button class="ib" data-i18n-title="bgDeckDef" title="Streamdeck background — default">\u21BB</button>';
    const btns = wrap.querySelectorAll('button');
    const pf = mkFile(), df = mkFile();
    btns[0].onclick = () => pf.click(); pf.onchange = e => { const f = e.target.files && e.target.files[0]; if (f) uploadBg(f, pageWhich); };
    btns[1].onclick = () => df.click(); df.onchange = e => { const f = e.target.files && e.target.files[0]; if (f) uploadBg(f, deckWhich); };
    btns[2].onclick = () => clearBg('bitspagedefault');
    btns[3].onclick = () => clearBg('bitsdefault');
    el.append(wrap);
  }
  function mountGlobalBg(el) {
    const wrap = document.createElement('span'); wrap.style.display = 'inline-flex'; wrap.style.gap = '8px'; wrap.style.verticalAlign = 'middle';
    wrap.innerHTML = '<button class="ib" data-i18n-title="bgCustom" title="Custom background">\u25A4</button><button class="ib" data-i18n-title="bgDefault" title="Default dark background">\u21BA</button>';
    const [up, def] = wrap.querySelectorAll('button');
    const f = mkFile();
    up.onclick = () => f.click(); f.onchange = e => { const file = e.target.files && e.target.files[0]; if (file) uploadBg(file, 'page'); };
    def.onclick = () => clearBg('page');
    el.append(wrap);
  }

  function mount() {
    const tools = document.querySelector('.head .tools');
    if (!tools) return;
    window.I18N = window.I18N || { en: {}, ua: {} };
    Object.assign(window.I18N.en, HEAD_I18N.en); Object.assign(window.I18N.ua, HEAD_I18N.ua);
    const themes = window.HEAD_THEMES || ['dark', 'light'];
    const labels = { dark: 'Dark', light: 'Light', custom: 'Custom' };
    const ico = p => '<svg class="navi" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">' + p + '</svg>';
    const ICON = {
      setup: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>',
      alerts: '<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>',
      bits: '<rect x="3" y="8" width="18" height="4" rx="1"/><path d="M12 8v13"/><path d="M19 12v7a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-7"/><path d="M7.5 8a2.5 2.5 0 0 1 0-5C11 3 12 8 12 8s1-5 4.5-5a2.5 2.5 0 0 1 0 5"/>',
      donate: '<path d="M19 14c1.5-1.5 3-3.3 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.8 0-3 .5-4.5 2-1.5-1.5-2.7-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.2 1.5 4 3 5.5l7 7Z"/>',
      timer: '<path d="M10 2h4"/><path d="M12 14v-4"/><circle cx="12" cy="14" r="8"/>',
      auction: '<path d="m14.5 12.5-8 8a2.119 2.119 0 1 1-3-3l8-8"/><path d="m16 16 6-6"/><path d="m8 8 6-6"/><path d="m9 7 8 8"/><path d="m21 11-8-8"/>'
    };
    tools.innerHTML =
      '<div class="seg" id="themeseg">' + themes.map(t => '<button data-th="' + t + '" data-i18n="th_' + t + '">' + labels[t] + '</button>').join('') + '</div>' +
      '<div class="seg" id="langseg"><button data-lang="en">EN</button><button data-lang="ua">UA</button></div>' +
      '<a class="ib btn" href="/setup" data-i18n-title="navSetup" title="Setup">' + ico(ICON.setup) + '</a>' +
      '<a class="ib btn" href="/notify" data-i18n-title="navAlerts" title="Alerts">' + ico(ICON.alerts) + '</a>' +
      '<a class="ib btn" href="/bitsedit" data-i18n-title="navBits" title="Bits">' + ico(ICON.bits) + '</a>' +
      '<a class="ib btn" href="/donateedit" data-i18n-title="navDonate" title="Donate page">' + ico(ICON.donate) + '</a>' +
      '<a class="ib btn" href="/timeradmin" data-i18n-title="navTimer" title="Timer">' + ico(ICON.timer) + '</a>' +
      '<a class="ib btn" href="/auctionsettings" data-i18n-title="navAuction" title="Auction settings">' + ico(ICON.auction) + '</a>' +
      '<button class="primary" id="save" data-i18n="save">Save</button>';
    document.querySelectorAll('#themeseg button').forEach(b => b.onclick = () => applyTheme(b.dataset.th));
    document.querySelectorAll('#langseg button').forEach(b => b.onclick = () => { window.setLang(b.dataset.lang); document.querySelectorAll('#langseg button').forEach(x => x.classList.toggle('on', x.dataset.lang === b.dataset.lang)); });
    $('save').onclick = () => { if (typeof window.savePage === 'function') window.savePage(); };
    // rat easter egg on every page (setup.html has its own copy)
    if (!document.getElementById('ratbox')) {
      const box = document.createElement('div'); box.id = 'ratbox';
      box.innerHTML = '<button class="x" id="ratx">\u2715</button><video id="ratvid" playsinline controls></video>';
      document.body.append(box);
      const play = () => { const v = $('ratvid'); if (!v.getAttribute('src')) v.setAttribute('src', '/rat.mp4'); box.classList.add('show'); try { v.currentTime = 0; } catch {} v.play().catch(() => {}); };
      const close = () => { const v = $('ratvid'); box.classList.remove('show'); try { v.pause(); } catch {} };
      $('ratx').onclick = close;
      box.onclick = (e) => { if (e.target.id === 'ratbox') close(); };
      document.addEventListener('keydown', (e) => { if (e.key === 'Escape') close(); });
      window.playRat = play;
      const logo = document.querySelector('.head .brand .logo'); if (logo) logo.onclick = play;
    }
    if (typeof window.extraTools === 'function') { try { window.extraTools(tools); } catch {} }
    let saved = 'dark'; try { saved = localStorage.getItem('lm_theme') || 'dark'; } catch {}
    if (themes.indexOf(saved) < 0) saved = themes[0];
    applyTheme(saved);
    let l = 'en'; try { l = localStorage.getItem('lm_lang') || 'en'; } catch {}
    document.querySelectorAll('#langseg button').forEach(b => b.classList.toggle('on', b.dataset.lang === l));
    if (typeof window.applyI18n === 'function') { try { window.applyI18n(); } catch {} }
    loadCfg();
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', mount); else mount();

  window.HEAD = HEAD;
  window.applyTheme = applyTheme;
  window.applyBgs = applyBgs;
  window.loadThemeCfg = loadCfg;
  window.mountBgButtons = mountBgButtons;
  window.mountGlobalBg = mountGlobalBg;
})();
