/* LocalMonoDonate — shared i18n. A page sets window.I18N = { en:{...}, ua:{...} } and marks
   text with data-i18n / data-i18n-ph. window.mountLangSeg(el) renders the EN/UA control. */
(function () {
  const get = () => { try { return localStorage.getItem('lm_lang') || window.I18N_DEFAULT || 'en'; } catch { return window.I18N_DEFAULT || 'en'; } };
  window.t = function (k) { const lang = get(); const d = (window.I18N && window.I18N[lang]) || {}; return d[k] || (window.I18N && window.I18N.en && window.I18N.en[k]) || k; };
  window.getLang = get;
  window.applyI18n = function () {
    document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = window.t(el.dataset.i18n); });
    document.querySelectorAll('[data-i18n-ph]').forEach(el => { el.placeholder = window.t(el.dataset.i18nPh); });
    document.querySelectorAll('[data-i18n-title]').forEach(el => { el.title = window.t(el.dataset.i18nTitle); el.setAttribute('aria-label', el.title); });
    document.documentElement.lang = get() === 'ua' ? 'uk' : 'en';
  };
  window.setLang = function (l) { try { localStorage.setItem('lm_lang', l); } catch {} window.applyI18n(); if (typeof window.applyLang === 'function') { try { window.applyLang(); } catch {} } document.dispatchEvent(new CustomEvent('lm:lang', { detail: l })); };
  const flag = code => code === 'ua'
    ? '<svg class="head-flag" viewBox="0 0 60 36" aria-hidden="true"><rect width="60" height="18" fill="#0057B7"/><rect y="18" width="60" height="18" fill="#FFD700"/></svg>'
    : '<svg class="head-flag" viewBox="0 0 60 36" aria-hidden="true"><rect width="60" height="36" fill="#012169"/><path d="M0 0 60 36M60 0 0 36" stroke="#fff" stroke-width="9"/><path d="M0 0 60 36M60 0 0 36" stroke="#C8102E" stroke-width="4"/><path d="M30 0v36M0 18h60" stroke="#fff" stroke-width="14"/><path d="M30 0v36M0 18h60" stroke="#C8102E" stroke-width="7"/></svg>';
  window.mountLangSeg = function (el) {
    el.innerHTML = '<button data-lang="en" title="English" aria-label="English">' + flag('en') + '</button><button data-lang="ua" title="Українська" aria-label="Українська">' + flag('ua') + '</button>';
    const cur = get();
    el.querySelectorAll('button').forEach(b => { b.classList.toggle('on', b.dataset.lang === cur); b.onclick = () => { window.setLang(b.dataset.lang); el.querySelectorAll('button').forEach(x => x.classList.toggle('on', x.dataset.lang === b.dataset.lang)); }; });
  };
})();
