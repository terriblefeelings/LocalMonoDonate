/* LocalMonoDonate — shared header, identical on every page:
   [Dark|Light] [EN|UA] [Setup · Alerts · Bits · Donate · Timer · Auction] [Save]
   - window.savePage()      : handled by Save
   - window.applyLang()     : page hook on EN/UA change (page i18n)
   - window.extraTools(el)  : extra page buttons
   - window.HEAD_THEMES     : theme options (default dark/light)
   - window.HEAD_BODY       : 'bg' (default) or 'bitsPageBg'
   - window.mountBgButtons(el, opts) / window.mountGlobalBg(el) */
(function () {
  const RAT_SVG = "<svg viewBox=\"0 0 64 64\" fill=\"currentColor\" xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M14,54c6.409,0,11.645-5.054,11.969-11.383c1.384,0.424,2.797,0.749,4.226,0.979C30.471,44.02,34,49.526,34,54v2.095  l-6.114,0.611C25.67,56.928,24,58.774,24,61c0,0.552,0.448,1,1,1h26c3.459,0,10-1.463,10-7V43c0-5.262-1.374-10.178-4.454-15.94  c-1.475-2.76-3.147-5.888-3.581-9.148c-0.317-2.381,0.135-4.744,1.239-6.481c0.602-0.947,1.501-1.55,2.406-1.615  c0.74-0.046,1.438,0.277,2.018,0.953c0.717,0.837,0.863,2.015,0.37,3l-0.392,0.783l1.789,0.895l0.392-0.783  c0.854-1.708,0.602-3.748-0.64-5.197c-0.994-1.16-2.302-1.74-3.678-1.646c-1.534,0.109-3.011,1.058-3.951,2.537  c-1.373,2.16-1.918,4.936-1.534,7.818c0.481,3.62,2.245,6.917,3.8,9.827C57.699,33.459,59,38.085,59,43v1.603  c-2.332-5.197-6.409-8.344-10.652-11.595c-1.547-1.186-3.147-2.412-4.689-3.761c-0.474-0.415-0.912-0.815-1.316-1.201  c-1.211-1.157-1.496-2.961-0.675-4.289c0.216-0.349,0.404-0.714,0.579-1.085C45.566,21.7,48,18.631,48,15c0-4.411-3.589-8-8-8  c-0.52,0-1.03,0.068-1.533,0.167C37.307,4.083,34.369,2,31,2c-4.411,0-8,3.589-8,8c0,0.596,0.095,1.204,0.241,1.812  c-0.183,0.267-0.373,0.528-0.534,0.813c-0.087,0.165-2.212,4.063-8.167,5.659c-1.545,0.414-2.527,1.148-2.916,2.181  c-0.353,0.936-0.16,2.033,0.544,3.089c2.422,3.634,6.231,5.038,14.557,5.365c1.729,0.068,3.38,0.854,4.527,2.156  c0.794,0.901,1.524,1.773,2.062,2.428c0.493,0.6,0.479,1.481-0.08,2.056c-0.286,0.284-0.665,0.438-1.094,0.43  c-2.415-0.061-4.544-0.371-7.347-1.072l-1.239-0.31c-0.047-0.012-0.094-0.014-0.141-0.024C21.146,31.714,17.666,30,14,30  C7.383,30,2,35.383,2,42S7.383,54,14,54z M31,4c2.497,0,4.672,1.531,5.562,3.801c-0.031,0.015-0.062,0.03-0.093,0.045  C35.133,7.295,33.735,7,32.298,7C29.574,7,27,8.036,25.01,9.807C25.113,6.588,27.756,4,31,4z M32.09,37.989  c0.938,0.033,1.881-0.345,2.546-1.004c0.006-0.006,0.075-0.075,0.081-0.081c1.229-1.264,1.289-3.271,0.142-4.669  c-0.549-0.668-1.296-1.56-2.106-2.48c-1.507-1.71-3.676-2.743-5.949-2.832c-0.953-0.038-1.823-0.093-2.637-0.16  c0.125-0.085,0.257-0.16,0.378-0.251c2.609-1.947,3.218-2.849,3.359-3.146l-1.778-0.913c-0.018,0.029-0.47,0.733-2.778,2.456  c-0.841,0.627-1.834,1.077-2.944,1.356c-3.736-0.742-5.384-2.037-6.573-3.82c-0.157-0.235-0.504-0.832-0.337-1.274  c0.145-0.385,0.7-0.724,1.563-0.955c6.849-1.835,9.321-6.47,9.406-6.634C26.061,10.755,29.063,9,32.298,9  c0.722,0,1.432,0.095,2.127,0.267c-0.49,0.474-0.923,1.013-1.282,1.609l1.713,1.032C35.954,10.087,37.876,9,40,9  c3.309,0,6,2.691,6,6s-2.691,6-6,6c-0.286,0-0.575-0.021-0.859-0.061l-0.281,1.98c0.32,0.045,0.644,0.061,0.967,0.068  c-1.118,2.093-0.676,4.777,1.135,6.506c0.424,0.404,0.883,0.824,1.38,1.259c1.591,1.393,3.218,2.638,4.791,3.843  C53.5,39.475,59,43.689,59,55c0,4.926-7.921,5-8,5H26.225c0.338-0.709,1.027-1.22,1.86-1.303l7.015-0.702  C35.611,57.944,36,57.514,36,57v-3.982c0-2.294,1.792-4.229,4.08-4.406l7.997-0.615l-0.153-1.994l-7.997,0.615  c-1.975,0.152-3.69,1.211-4.762,2.741c-0.713-2.201-1.728-4.188-2.455-5.464C33.509,43.957,34.308,44,35.109,44H37v-2h-1.891  c-4.496,0-8.999-1.063-13.02-3.074c-0.317-0.158-0.549-0.432-0.654-0.771c-0.105-0.34-0.065-0.706,0.114-1.029  c0.271-0.488,0.909-0.733,1.521-0.579l1.239,0.31C27.224,37.587,29.551,37.925,32.09,37.989z M14,32c2.653,0,5.188,1.081,7.049,2.93  c-0.516,0.284-0.956,0.697-1.251,1.227c-0.445,0.805-0.543,1.725-0.275,2.59c0.264,0.852,0.873,1.569,1.67,1.968  c0.912,0.456,1.85,0.856,2.803,1.219C23.998,41.957,24,41.978,24,42c0,5.514-4.486,10-10,10S4,47.514,4,42S8.486,32,14,32z\"/><path d=\"M11,46h6c0.552,0,1-0.448,1-1v-6c0-0.552-0.448-1-1-1h-6c-0.552,0-1,0.448-1,1v6C10,45.552,10.448,46,11,46z M12,40h4v4h-4  V40z\"/><rect height=\"2\" width=\"2\" x=\"28\" y=\"16\"/><rect height=\"2\" width=\"2\" x=\"4\" y=\"22\"/><rect height=\"2\" width=\"2\" x=\"2\" y=\"24\"/><rect height=\"2\" width=\"2\" x=\"6\" y=\"24\"/><rect height=\"2\" width=\"2\" x=\"4\" y=\"26\"/><rect height=\"2\" width=\"2\" x=\"12\" y=\"56\"/><rect height=\"2\" width=\"2\" x=\"10\" y=\"58\"/><rect height=\"2\" width=\"2\" x=\"14\" y=\"58\"/><rect height=\"2\" width=\"2\" x=\"12\" y=\"60\"/><rect height=\"2\" width=\"2\" x=\"11\" y=\"2\"/><rect height=\"2\" width=\"2\" x=\"9\" y=\"4\"/><rect height=\"2\" width=\"2\" x=\"13\" y=\"4\"/><rect height=\"2\" width=\"2\" x=\"11\" y=\"6\"/></svg>";
  const root = document.documentElement;
  const $ = id => document.getElementById(id);
  document.body.dataset.page = location.pathname.replace(/\//g, '') || 'root';
  const HEAD = { bg: '', bitsBg: '', bitsPageBg: '' };
  const HEAD_I18N = {
    en: { navSetup: 'Setup', navAlerts: 'Alerts', navBits: 'Sound alerts', navDonate: 'Donation page', navQr: 'QR widgets', navTimer: 'Timer', navAuction: 'Auction', save: 'Save', th_dark: 'Dark', th_light: 'Light' },
    ua: { navSetup: 'Налаштування', navAlerts: 'Сповіщення', navBits: 'Звукові сповіщення', navDonate: 'Сторінка донатів', navQr: 'QR-віджети', navTimer: 'Таймер', navAuction: 'Аукціон', save: 'Зберегти', th_dark: 'Темна', th_light: 'Світла' }
  };
  const PAGE_LABELS = {
    '/setup': { en: '://Credentials configuration', ua: '://Налаштування доступів' },
    '/notify': { en: '://Alerts configuration', ua: '://Налаштування алертів' },
    '/bitsedit': { en: '://Sound Alerts configuration', ua: '://Налаштування звукових сповіщень' },
    '/donateedit': { en: '://Donate page configuration', ua: '://Налаштування сторінки донатів' },
    '/qredit': { en: '://QR widgets configuration', ua: '://Налаштування QR-віджетів' },
    '/timeradmin': { en: '://Marathon timer configuration', ua: '://Налаштування таймера марафону' },
    '/auctionsettings': { en: '://Auction configuration', ua: '://Налаштування аукціону' }
  };
  const PAGE_TITLES = {
    '/setup': { en: 'Credentials configuration — LocalMonoDonate', ua: 'Налаштування доступів — LocalMonoDonate' },
    '/notify': { en: 'Alerts configuration — LocalMonoDonate', ua: 'Налаштування алертів — LocalMonoDonate' },
    '/bitsedit': { en: 'Sound Alerts configuration — LocalMonoDonate', ua: 'Налаштування звукових сповіщень — LocalMonoDonate' },
    '/donateedit': { en: 'Donate page configuration — LocalMonoDonate', ua: 'Налаштування сторінки донатів — LocalMonoDonate' },
    '/qredit': { en: 'QR widgets configuration — LocalMonoDonate', ua: 'Налаштування QR-віджетів — LocalMonoDonate' },
    '/timeradmin': { en: 'Marathon timer configuration — LocalMonoDonate', ua: 'Налаштування таймера марафону — LocalMonoDonate' },
    '/auctionsettings': { en: 'Auction configuration — LocalMonoDonate', ua: 'Налаштування аукціону — LocalMonoDonate' }
  };
  const ADMIN_PAGES = {
    '/setup': { code: 'S/01', kicker: { en: 'System', ua: 'Система' }, title: { en: 'Credentials configuration', ua: 'Налаштування доступів' }, intro: { en: 'Configure the Monobank jar, public link, integrations, and service endpoints.', ua: 'Налаштуйте банку Monobank, публічне посилання, інтеграції та сервісні адреси.' } },
    '/bitsedit': { code: 'B/01', kicker: { en: 'Sound alerts', ua: 'Звукові сповіщення' }, title: { en: 'Sound Alerts configuration', ua: 'Налаштування звукових сповіщень' }, intro: { en: 'Lay out the sound-alert deck and assign clips to each slot.', ua: 'Налаштуйте дек звукових сповіщень і призначте кліпи для кожного слота.' } },
    '/donateedit': { code: 'D/01', kicker: { en: 'Donation page', ua: 'Сторінка донатів' }, title: { en: 'Donate page configuration', ua: 'Налаштування сторінки донатів' }, intro: { en: 'Choose the donation-page background and YouTube playback rules.', ua: 'Виберіть фон сторінки донатів і правила відтворення YouTube.' } },
    '/qredit': { code: 'Q/01', kicker: { en: 'Widgets', ua: 'Віджети' }, title: { en: 'QR widgets configuration', ua: 'Налаштування QR-віджетів' }, intro: { en: 'Design QR overlays, goal cards, alignment, and scale.', ua: 'Налаштуйте QR-оверлеї, картки цілі, вирівнювання та масштаб.' } },
    '/timeradmin': { code: 'T/01', kicker: { en: 'Marathon', ua: 'Марафон' }, title: { en: 'Marathon timer configuration', ua: 'Налаштування таймера марафону' }, intro: { en: 'Control the stream timer, rates, and visual treatment.', ua: 'Керуйте таймером стріму, ставками часу та виглядом.' } },
    '/auctionsettings': { code: 'A/01', kicker: { en: 'Auction', ua: 'Аукціон' }, title: { en: 'Auction configuration', ua: 'Налаштування аукціону' }, intro: { en: 'Tune timer extensions, bid modes, sources, and Twitch rewards.', ua: 'Налаштуйте продовження таймера, режими ставок, джерела та нагороди Twitch.' } }
  };
  function syncAdminHero() {
    const wrap = document.querySelector('.wrap');
    const page = ADMIN_PAGES[location.pathname];
    if (!wrap || !page) return;
    let hero = document.querySelector('.admin-hero');
    if (!hero) {
      wrap.insertAdjacentHTML('afterbegin',
        '<header class="admin-hero">' +
          '<div class="admin-hero__meta"><span class="admin-hero__code" data-hero-code></span><span>LocalMonoDonate</span><span class="admin-hero__slash">/</span><span data-hero-kicker></span></div>' +
          '<div class="admin-hero__row"><div><h1 data-hero-title></h1><p data-hero-intro></p></div><div class="admin-hero__status"><span class="admin-hero__lamp" aria-hidden="true"></span><span data-hero-status></span></div></div>' +
        '</header><div class="admin-rail" aria-hidden="true"></div>');
      hero = document.querySelector('.admin-hero');
    }
    let l = 'en';
    try { l = localStorage.getItem('lm_lang') || 'en'; } catch {}
    const lang = l === 'ua' ? 'ua' : 'en';
    const set = (sel, value) => { const el = hero.querySelector(sel); if (el) el.textContent = value; };
    set('[data-hero-code]', page.code);
    set('[data-hero-kicker]', page.kicker[lang]);
    const title = page.title[lang];
    const esc = v => String(v).replace(/[&<>]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;'}[c]));
    const titleEl = hero.querySelector('[data-hero-title]');
    if (titleEl) {
      if (lang === 'ua') {
        const cut = title.indexOf(' ');
        const lead = cut > 0 ? title.slice(0, cut) : '';
        const name = cut > 0 ? title.slice(cut + 1) : title;
        titleEl.innerHTML = (lead ? esc(lead) + ' ' : '') + '<span class="admin-title-accent">' + esc(name) + '</span>';
      } else {
        const cut = title.lastIndexOf(' ');
        const name = cut > 0 ? title.slice(0, cut) : title;
        const tail = cut > 0 ? ' ' + esc(title.slice(cut + 1)) : '';
        titleEl.innerHTML = '<span class="admin-title-accent">' + esc(name) + '</span>' + tail;
      }
    }
    set('[data-hero-intro]', page.intro[lang]);
    set('[data-hero-status]', lang === 'ua' ? 'Локальне керування' : 'Local admin');
  }
  const HEAD_ICON = {
    moon: '<path d="M20.6 14.2A8.5 8.5 0 0 1 9.8 3.4 8.5 8.5 0 1 0 20.6 14.2Z"/>',
    sun: '<circle cx="12" cy="12" r="3.5"/><path d="M12 2v2.2M12 19.8V22M2 12h2.2M19.8 12H22M4.9 4.9l1.6 1.6M17.5 17.5l1.6 1.6M19.1 4.9l-1.6 1.6M6.5 17.5l-1.6 1.6"/>'
  };
  const headIcon = name => '<svg class="head-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' + HEAD_ICON[name] + '</svg>';
  const headFlag = code => code === 'ua'
    ? '<svg class="head-flag" viewBox="0 0 60 36" aria-hidden="true"><rect width="60" height="18" fill="#0057B7"/><rect y="18" width="60" height="18" fill="#FFD700"/></svg>'
    : '<svg class="head-flag" viewBox="0 0 60 36" aria-hidden="true"><defs><clipPath id="head-uk-flag"><rect width="60" height="36" rx="2"/></clipPath></defs><g clip-path="url(#head-uk-flag)"><rect width="60" height="36" fill="#012169"/><path d="M0 0 60 36M60 0 0 36" stroke="#fff" stroke-width="9"/><path d="M0 0 60 36M60 0 0 36" stroke="#C8102E" stroke-width="4"/><path d="M30 0v36M0 18h60" stroke="#fff" stroke-width="14"/><path d="M30 0v36M0 18h60" stroke="#C8102E" stroke-width="7"/></g></svg>';
  function syncHeadIcons() {
    const tr = window.t || (k => k);
    document.querySelectorAll('#themeseg button').forEach(button => {
      if (!button.querySelector('.head-icon')) {
        button.innerHTML = headIcon(button.dataset.th === 'light' ? 'sun' : 'moon');
        button.removeAttribute('data-i18n');
      }
      const label = tr(button.dataset.th === 'light' ? 'th_light' : 'th_dark');
      button.title = label; button.setAttribute('aria-label', label);
    });
    document.querySelectorAll('#langseg button').forEach(button => {
      if (!button.querySelector('.head-flag')) button.innerHTML = headFlag(button.dataset.lang === 'ua' ? 'ua' : 'uk');
      const label = button.dataset.lang === 'ua' ? 'Українська' : 'English';
      button.title = label; button.setAttribute('aria-label', label);
    });
  }
  const TAB_PAGES = new Set(['/setup','/donateedit','/qredit','/timeradmin']);
  function syncAdminLayout() {
    const wrap = document.querySelector('.wrap');
    const rail = document.querySelector('.admin-rail');
    if (!wrap || !rail) return;
    if (!TAB_PAGES.has(location.pathname)) return;
    let work = document.querySelector('.admin-workbench');
    if (!work) {
      const preview = (location.pathname === '/timeradmin' || location.pathname === '/qredit' || location.pathname === '/donateedit')
        ? Array.from(wrap.children).find(el => el.classList.contains('panel') && el.querySelector('.preview, .qrPreview, .donatePreview'))
        : null;
      work = document.createElement('div'); work.className = 'admin-workbench';
      const index = document.createElement('aside'); index.className = 'admin-index';
      const main = document.createElement('div'); main.className = 'admin-main';
      while (rail.nextSibling) main.appendChild(rail.nextSibling);
      work.append(index, main);
      rail.after(work);
      if (preview) rail.after(preview);
    }
    const main = work.querySelector('.admin-main');
    const index = work.querySelector('.admin-index');
    if (!main || !index) return;
    const status = main.querySelector(':scope > .status');
    if (status) rail.after(status);
    const targets = Array.from(main.querySelectorAll('.panel, .setpanel'));
    if (!targets.length) return;
    const previous = main.dataset.activeTarget;
    let uiLang = 'en'; try { uiLang = localStorage.getItem('lm_lang') || 'en'; } catch {}
    index.innerHTML = '<div class="admin-index__head"><span>' + (uiLang === 'ua' ? 'Налаштування' : 'Settings') + '</span><span>' + (uiLang === 'ua' ? 'Локально' : 'Local') + '</span></div>';
    const nav = document.createElement('nav');
    const select = (target) => {
      main.dataset.activeTarget = target.id;
      targets.forEach(t => { t.hidden = t !== target; });
      nav.querySelectorAll('button').forEach(b => b.classList.toggle('on', b.dataset.target === target.id));
    };
    targets.forEach((target, i) => {
      if (!target.id) target.id = 'admin-sec-' + (i + 1);
      const titleEl = target.querySelector('.sec .h, h3');
      const name = titleEl ? titleEl.textContent.trim() : 'Section ' + (i + 1);
      const b = document.createElement('button');
      b.type = 'button'; b.dataset.target = target.id;
      b.innerHTML = '<span class="admin-index__n">' + String(i + 1).padStart(2, '0') + '</span><span>' + name.replace(/[&<>]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;'}[c])) + '</span>';
      b.onclick = () => select(target);
      nav.append(b);
    });
    index.append(nav);
    const active = targets.find(t => t.id === previous) || targets[0];
    select(active);
  }
  function syncPageLabel() {
    const brand = document.querySelector('.head .brand .t');
    const el = document.querySelector('.head .brand .pg');
    const labels = PAGE_LABELS[location.pathname];
    const titles = PAGE_TITLES[location.pathname];
    let l = 'en';
    try { l = localStorage.getItem('lm_lang') || 'en'; } catch {}
    const lang = l === 'ua' ? 'ua' : 'en';
    if (brand && brand.firstChild && brand.firstChild.nodeType === 3) brand.firstChild.nodeValue = 'LMD';
    if (el && labels) el.textContent = labels[lang];
    if (titles) document.title = titles[lang];
  }
  document.addEventListener('lm:lang', () => { syncPageLabel(); syncHeadIcons(); syncAdminHero(); syncAdminLayout(); });

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
      window.dispatchEvent(new CustomEvent('lm:bg'));
    } catch {}
  }
  async function clearBg(which) {
    await fetch('/api/bg', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ url: '', which }) });
    if (which === 'bitsdefault') HEAD.bitsBg = ''; else if (which === 'bitspagedefault') HEAD.bitsPageBg = ''; else HEAD.bg = '';
    applyBgs();
    window.dispatchEvent(new CustomEvent('lm:bg'));
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

  function initAdminZoom() {
    if (document.body.classList.contains('notify-page')) return;
    const wrap = document.querySelector('.wrap');
    if (!wrap) return;
    let zoom = 1;
    const hud = document.createElement('button');
    hud.className = 'admin-zoom-hud'; hud.type = 'button'; hud.title = 'Reset page zoom'; hud.setAttribute('aria-label', 'Reset page zoom');
    const apply = next => {
      zoom = Math.min(2, Math.max(0, next));
      wrap.style.zoom = String(zoom);
      hud.textContent = Math.round(zoom * 100) + '%';
    };
    hud.onclick = () => apply(1);
    document.body.append(hud);
    window.addEventListener('wheel', e => {
      if (!e.ctrlKey || (e.target instanceof Element && e.target.closest('.head'))) return;
      e.preventDefault();
      apply(zoom + (e.deltaY < 0 ? 0.01 : -0.01));
    }, { passive: false });
    document.addEventListener('keydown', e => { if (e.ctrlKey && e.key === '0') { e.preventDefault(); apply(1); } });
    apply(1);
  }
  function mount() {
    const head = document.querySelector('.head');
    if (!head) return;
    head.innerHTML = '<div class="brand"><button class="logo nt-rat-logo" id="logo" type="button" title="LocalMonoDonate">' + RAT_SVG + '</button><div class="t">LMD<span class="pg"></span></div></div><div class="tools"></div>';
    const tools = head.querySelector('.tools');
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
      qr: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><path d="M14 14h3v3h-3zM20 14v3M14 20h3M20 20h1"/>',
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
      '<a class="ib btn" href="/qredit" data-i18n-title="navQr" title="QR widgets">' + ico(ICON.qr) + '</a>' +
      '<a class="ib btn" href="/timeradmin" data-i18n-title="navTimer" title="Timer">' + ico(ICON.timer) + '</a>' +
      '<a class="ib btn" href="/auctionsettings" data-i18n-title="navAuction" title="Auction settings">' + ico(ICON.auction) + '</a>' +
      '<button class="primary save-icon" id="save" data-i18n-title="save" title="Save" aria-label="Save"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2Z"/><path d="M17 21v-8H7v8M7 3v5h8"/></svg></button>' +
      '<a id="updateBtn" class="update-btn" target="_blank" rel="noopener"></a>';
    document.querySelectorAll('#themeseg button').forEach(b => b.onclick = () => applyTheme(b.dataset.th));
    document.querySelectorAll('#langseg button').forEach(b => b.onclick = () => { window.setLang(b.dataset.lang); document.querySelectorAll('#langseg button').forEach(x => x.classList.toggle('on', x.dataset.lang === b.dataset.lang)); syncPageLabel(); syncHeadIcons(); syncAdminHero(); syncAdminLayout(); });
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
    syncPageLabel();
    syncHeadIcons();
    syncAdminHero();
    syncAdminLayout();
    initAdminZoom();
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
