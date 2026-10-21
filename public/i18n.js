/* LocalMonoDonate — shared i18n. A page sets window.I18N = { en:{...}, ua:{...} } and marks
   text with data-i18n / data-i18n-ph. window.mountLangSeg(el) renders the EN/UA control. */
(function () {
  const get = () => { try { return localStorage.getItem('lm_lang') || window.I18N_DEFAULT || 'en'; } catch { return window.I18N_DEFAULT || 'en'; } };
  window.t = function (k) { const lang = get(); const d = (window.I18N && window.I18N[lang]) || {}; return d[k] || (window.I18N && window.I18N.en && window.I18N.en[k]) || k; };
  window.getLang = get;
  window.applyI18n = function () {
    document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = window.t(el.dataset.i18n); });
    document.querySelectorAll('[data-i18n-ph]').forEach(el => { el.placeholder = window.t(el.dataset.i18nPh); });
    document.querySelectorAll('[data-i18n-title]').forEach(el => { el.title = window.t(el.dataset.i18nTitle); });
    document.documentElement.lang = get() === 'ua' ? 'uk' : 'en';
  };
  window.setLang = function (l) { try { localStorage.setItem('lm_lang', l); } catch {} window.applyI18n(); if (typeof window.applyLang === 'function') { try { window.applyLang(); } catch {} } };
  window.mountLangSeg = function (el) {
    el.innerHTML = '<button data-lang="en">EN</button><button data-lang="ua">UA</button>';
    const cur = get();
    el.querySelectorAll('button').forEach(b => { b.classList.toggle('on', b.dataset.lang === cur); b.onclick = () => { window.setLang(b.dataset.lang); el.querySelectorAll('button').forEach(x => x.classList.toggle('on', x.dataset.lang === b.dataset.lang)); }; });
  };
})();
