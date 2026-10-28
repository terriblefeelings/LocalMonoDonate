/* Presentation adapter for /notify: preview fitting, media pan, and range readouts. */
(() => {
  const $ = id => document.getElementById(id);
  const stage = $('stage');
  if (!stage) return;

  const tabs = Array.from(document.querySelectorAll('[role="tab"][data-tab]'));
  const panels = tabs.map(tab => document.getElementById(tab.getAttribute('aria-controls'))).filter(Boolean);
  function activateTab(tab, focus) {
    if (!tab) return;
    tabs.forEach(item => {
      const active = item === tab;
      item.classList.toggle('is-active', active);
      item.setAttribute('aria-selected', active ? 'true' : 'false');
      item.tabIndex = active ? 0 : -1;
    });
    panels.forEach(panel => { panel.hidden = panel.id !== tab.getAttribute('aria-controls'); });
    if (focus) tab.focus();
  }
  tabs.forEach((tab, index) => {
    tab.addEventListener('click', () => activateTab(tab, false));
    tab.addEventListener('keydown', event => {
      let next = null;
      if (event.key === 'ArrowDown' || event.key === 'ArrowRight') next = tabs[(index + 1) % tabs.length];
      else if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') next = tabs[(index - 1 + tabs.length) % tabs.length];
      else if (event.key === 'Home') next = tabs[0];
      else if (event.key === 'End') next = tabs[tabs.length - 1];
      if (!next) return;
      event.preventDefault();
      activateTab(next, true);
    });
  });
  const initialTab = document.querySelector('[role="tab"][aria-selected="true"]') || tabs[0];
  if (initialTab) activateTab(initialTab, false);

  const panelHost = document.querySelector('.nt-tabpanels');
  function syncInspectorHeight() {
    if (panelHost) panelHost.style.minHeight = '';
  }
  window.addEventListener('resize', syncInspectorHeight, { passive: true });
  document.addEventListener('notify:style', syncInspectorHeight);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(syncInspectorHeight).catch(() => {});
  setTimeout(syncInspectorHeight, 80);
  setTimeout(syncInspectorHeight, 400);

  let fitFrame = 0;
  function fitPreview() {
    cancelAnimationFrame(fitFrame);
    fitFrame = requestAnimationFrame(() => {
      const el = stage.querySelector('.notif');
      if (!el) return;
      el.style.transform = 'none';
      const width = el.offsetWidth;
      const height = el.offsetHeight;
      if (!width || !height) return;
      const availableWidth = Math.max(1, stage.clientWidth - 40);
      const availableHeight = Math.max(1, stage.clientHeight - 40);
      const scale = Math.min(1, availableWidth / width, availableHeight / height);
      el.style.transformOrigin = 'center center';
      el.style.transform = `scale(${scale})`;
      stage.style.setProperty('--nt-preview-scale', String(scale));
    });
  }

  new MutationObserver(fitPreview).observe(stage, { childList: true, subtree: true });
  if ('ResizeObserver' in window) new ResizeObserver(fitPreview).observe(stage);
  window.addEventListener('resize', fitPreview, { passive: true });
  window.addEventListener('load', fitPreview);
  fitPreview();

  const page = $('page-zoom');
  const zoomValue = $('zoomValue');
  let pageZoom = 1;
  function applyPageZoom(next) {
    pageZoom = Math.min(2, Math.max(0, next));
    if (page) page.style.zoom = String(pageZoom);
    if (zoomValue) zoomValue.textContent = Math.round(pageZoom * 100) + '%';
    requestAnimationFrame(fitPreview);
  }
  window.addEventListener('wheel', event => {
    if (!event.ctrlKey) return;
    event.preventDefault();
    applyPageZoom(pageZoom + (event.deltaY < 0 ? 0.01 : -0.01));
  }, { passive: false });
  document.addEventListener('keydown', event => {
    if (event.ctrlKey && event.key === '0') {
      event.preventDefault();
      applyPageZoom(1);
    }
  });
  $('zoomReset')?.addEventListener('click', () => applyPageZoom(1));
  applyPageZoom(1);

  function syncRangeOutputs() {
    document.querySelectorAll('output[data-range-value]').forEach(output => {
      const input = document.getElementById(output.getAttribute('for'));
      if (!input) return;
      const value = Number(input.value);
      output.textContent = Number.isFinite(value) ? value.toFixed(2) : '—';
    });
  }
  document.addEventListener('notify:style', syncRangeOutputs);
  document.addEventListener('input', event => {
    if (event.target && event.target.matches('input[type="range"]')) syncRangeOutputs();
  });
  syncRangeOutputs();

  const NOTE_KEYS = {
    rule_label:'ruleNameNote', rule_min:'ruleMinNote', rule_max:'ruleMaxNote', addrule:'addRuleNote',
    top_text:'topTextNote', money_text:'moneyTextNote', blue_size:'sizeNote', green_size:'sizeNote', msg_size:'sizeNote',
    blue_weight:'weightNote', green_weight:'weightNote', msg_weight:'weightNote',
    blue_color:'colorNote', green_color:'colorNote', msg_color:'colorNote',
    blue_outline_op:'opacityNote', green_outline_op:'opacityNote', msg_outline_op:'opacityNote',
    blue_maxw:'sizeNote', green_maxw:'sizeNote', msg_maxw:'sizeNote', msg_maxh:'sizeNote',
    win_bg:'winBgNote', win_bg_opacity:'opacityNote', win_radius:'radiusNote', win_maxh:'sizeNote',
    win_border_width:'borderNote', win_border_color:'borderNote', win_shadow:'shadowNote', win_padding:'paddingNote',
    duration_ms:'durationNote', anim_in:'animInNote', anim_out:'animOutNote', anim_ms:'speedNote',
    image_url:'mediaUrlNote', video_url:'mediaUrlNote', sound_url:'mediaUrlNote',
    media_pos:'mediaPosNote', media_w:'mediaSizeNote', media_h:'mediaSizeNote', media_zoom:'mediaZoomNote', media_radius:'radiusNote',
    centerMedia:'centerNote', export:'exportNote', import:'importNote', reset:'resetNote'
  };
  function applyNotes() {
    const tr = window.t || (key => key);
    for (const [id, key] of Object.entries(NOTE_KEYS)) {
      const el = $(id);
      if (el) el.title = tr(key);
    }
    document.querySelectorAll('[data-up]').forEach(button => { button.title = tr('uploadNote'); });
    if ($('stage')) $('stage').title = tr('previewNote');
    document.querySelectorAll('.nt-field input, .nt-field select').forEach(el => {
      if (NOTE_KEYS[el.id]) return;
      const label = el.closest('.nt-field')?.querySelector('.nt-field__label');
      if (label) el.title = label.textContent.trim();
    });
  }
  applyNotes();
  document.addEventListener('notify:lang', applyNotes);

  $('centerMedia')?.addEventListener('click', () => {
    if (typeof window.setMediaPan === 'function') window.setMediaPan(0, 0);
  });

  stage.addEventListener('dblclick', () => {
    if (typeof window.setMediaPan === 'function') window.setMediaPan(0, 0);
  });

  document.addEventListener('notify:lang', () => { syncInspectorHeight(); });

  stage.addEventListener('wheel', event => {
    if (!event.ctrlKey) return;
    event.preventDefault();
    const zoom = $('media_zoom');
    if (!zoom) return;
    const current = Number(zoom.value) || 0;
    const next = Math.min(200, Math.max(0, current + (event.deltaY < 0 ? 1 : -1)));
    if (next === current) return;
    zoom.value = next;
    zoom.dispatchEvent(new Event('input', { bubbles: true }));
    if (typeof window.refreshPreview === 'function') window.refreshPreview();
  }, { passive: false });

  let drag = null;
  stage.addEventListener('pointerdown', event => {
    if (!(event.target instanceof Element)) return;
    const box = event.target.closest('.nmbox');
    if (!box) return;
    const media = box.querySelector('img, video');
    if (!media) return;
    event.preventDefault();
    media.draggable = false;
    const origin = typeof window.getMediaPan === 'function' ? window.getMediaPan() : { x: 0, y: 0 };
    const scale = Number.parseFloat(getComputedStyle(stage).getPropertyValue('--nt-preview-scale')) || 1;
    drag = {
      box,
      media,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      originX: origin.x,
      originY: origin.y,
      x: origin.x,
      y: origin.y,
      scale: scale || 1
    };
    try { box.setPointerCapture(event.pointerId); } catch {}
    stage.classList.add('is-panning');
  });

  stage.addEventListener('pointermove', event => {
    if (!drag || event.pointerId !== drag.pointerId) return;
    event.preventDefault();
    drag.x = Math.round(drag.originX + (event.clientX - drag.startX) / drag.scale);
    drag.y = Math.round(drag.originY + (event.clientY - drag.startY) / drag.scale);
    const zoom = Number($('media_zoom')?.value);
    const mediaScale = Number.isFinite(zoom) ? zoom / 100 : 1;
    drag.media.style.transform = `translate(${drag.x}px, ${drag.y}px) scale(${mediaScale})`;
  });

  function finishPan(event) {
    if (!drag || event.pointerId !== drag.pointerId) return;
    const x = drag.x;
    const y = drag.y;
    const moved = x !== drag.originX || y !== drag.originY;
    drag = null;
    stage.classList.remove('is-panning');
    if (moved && typeof window.setMediaPan === 'function') window.setMediaPan(x, y);
  }
  stage.addEventListener('dragstart', event => event.preventDefault());
  stage.addEventListener('pointerup', finishPan);
  stage.addEventListener('pointercancel', finishPan);
})();
