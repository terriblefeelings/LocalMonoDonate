/* LocalMonoDonate — shared alert renderer. Used by BOTH the overlay and the editor preview,
   so what you see in settings is exactly what plays. */
(function () {
  const DEFAULTS = { top_text:'', money_text:'%donate% задонатив %money%',
    blue_font:'Segoe UI', blue_size:20, blue_weight:600, blue_color:'#ffffff', blue_outline:'#000000', blue_outline_op:0, blue_maxw:720, blue_tracking:0, blue_line:1.2,
    green_font:'Segoe UI', green_size:26, green_weight:800, green_color:'#ffffff', green_outline:'#000000', green_outline_op:0, green_maxw:720, green_tracking:0, green_line:1.2,
    msg_font:'Segoe UI', msg_size:18, msg_weight:500, msg_color:'#ffffff', msg_outline:'#000000', msg_outline_op:0, msg_maxw:720, msg_maxh:96,
    win_bg:'#131313', win_bg_opacity:0.9, win_radius:16, win_maxh:0, win_border_color:'#000000', win_border_width:0, win_shadow:0, win_padding:10,
    blue_bg:'#000000', blue_bg_opacity:0, blue_radius:12,
    green_bg:'#000000', green_bg_opacity:0, green_radius:12,
    red_bg:'#000000', red_bg_opacity:0, red_radius:12,
    duration_ms:8000, wait_media:false, anim_in:'slide_up', anim_out:'fade', anim_ms:400,
    image_url:'/assets/default.jpg',
    video_url:'', sound_url:'', media_pos:'top', media_w:720, media_h:320, media_zoom:18, media_radius:12, media_pan_x:-12, media_pan_y:32, max_width:720 };

  const fmt = (n, c) => Math.round(Number(n) || 0).toLocaleString('uk-UA') + ' ' + (c || '\u20B4');
  const hexA = (hex, a) => { const h=(hex||'#000').replace('#',''); const n=parseInt(h.length===3?h.split('').map(c=>c+c).join(''):h,16); return `rgba(${(n>>16)&255},${(n>>8)&255},${n&255},${a})`; };
  const sub = (tpl, name, money) => String(tpl||'').replace(/%donate%/g, name).replace(/%money%/g, money);
  const isVidUrl = u => /\.(mp4|webm|mov|mkv|avi|m4v)(\?|$)/i.test(u || '');
  function outline(c, op) { if (!op) return 'none'; const col = hexA(c, op); const p = [];
    for (const [dx,dy] of [[-1,-1],[1,-1],[-1,1],[1,1],[0,-1],[0,1],[-1,0],[1,0]]) p.push(`${dx*2}px ${dy*2}px 0 ${col}`);
    return p.join(', '); }
  function animFrom(k){ return ({slide_up:'translateY(30px) scale(.96)',fade:'scale(1)',zoom:'scale(.85)',slide_left:'translateX(-40px)',slide_down:'translateY(30px) scale(.96)',slide_right:'translateX(40px)',rise:'translateY(-40px) scale(.96)',flip_x:'rotateY(90deg) scale(.96)'})[k]||'scale(1)'; }
  function lineStyle(el, font, size, weight, color, out, outop, maxw, bg, bgop, rad) {
    el.style.fontFamily = `"${font}", system-ui, sans-serif`; el.style.fontSize = size + 'px'; el.style.fontWeight = weight;
    el.style.color = color; el.style.textShadow = outline(out, outop); el.style.maxWidth = maxw + 'px';
    if (bgop > 0) { el.style.background = hexA(bg, bgop); el.style.borderRadius = rad + 'px'; }
  }
  function mediaBox(N, opts) {
    const src = N.video_url || N.image_url; if (!src) return null;
    const box = document.createElement('div'); box.className = 'nmbox';
    box.style.width = (N.media_w||320) + 'px'; box.style.height = (N.media_h||320) + 'px'; box.style.borderRadius = (N.media_radius||0) + 'px';
    const el = document.createElement(N.video_url ? 'video' : 'img'); el.src = src;
    el.style.transform = `translate(${N.media_pan_x||0}px, ${N.media_pan_y||0}px) scale(${(N.media_zoom||100)/100})`;
    el.style.transformOrigin = 'center center';
    if (el.tagName === 'VIDEO') { el.playsInline = true;
      if (opts && opts.preview) { el.muted = true; el.autoplay = false; el.preload = 'metadata'; }
      else { el.muted = false; el.autoplay = true; el.loop = false; } }
    box.append(el);
    return box;
  }
  function build(data, N, opts) {
    N = Object.assign({}, DEFAULTS, N || {});
    const name = data.name || '', money = fmt(data.amount, data.currency);
    const topTxt = sub(N.top_text, name, money), moneyTxt = sub(N.money_text, name, money);
    const notif = document.createElement('div'); notif.className = 'notif';
    const stack = document.createElement('div'); stack.className = 'stack'; stack.style.width = N.max_width + 'px';
    if (topTxt.trim()) { const blue = document.createElement('div'); blue.className = 'nline'; blue.style.width = '100%'; blue.textContent = topTxt;
      lineStyle(blue, N.blue_font, N.blue_size, N.blue_weight, N.blue_color, N.blue_outline, N.blue_outline_op, N.blue_maxw, N.blue_bg, N.blue_bg_opacity, N.blue_radius);
      blue.style.letterSpacing = (N.blue_tracking || 0) + 'px'; blue.style.lineHeight = N.blue_line || 1.2; stack.append(blue); }
    const win = document.createElement('div'); win.className = 'nwin'; win.style.width = '100%'; win.style.boxSizing = 'border-box';
    win.style.background = hexA(N.win_bg, N.win_bg_opacity); win.style.borderRadius = N.win_radius + 'px';
    win.style.padding = Math.max(0, Number(N.win_padding) || 0) + 'px';
    win.style.border = Number(N.win_border_width) > 0 ? `${N.win_border_width}px solid ${N.win_border_color || '#000000'}` : 'none';
    win.style.boxShadow = Number(N.win_shadow) > 0 ? `0 ${Math.round(N.win_shadow / 3)}px ${N.win_shadow}px rgba(0,0,0,.55)` : 'none';
    if (N.win_maxh > 0) { win.style.maxHeight = N.win_maxh + 'px'; win.style.overflow = 'hidden'; }
    if (moneyTxt.trim()) { const green = document.createElement('div'); green.className = 'nline'; green.style.width = '100%'; green.textContent = moneyTxt;
      lineStyle(green, N.green_font, N.green_size, N.green_weight, N.green_color, N.green_outline, N.green_outline_op, N.green_maxw, N.green_bg, N.green_bg_opacity, N.green_radius);
      green.style.letterSpacing = (N.green_tracking || 0) + 'px'; green.style.lineHeight = N.green_line || 1.2; win.append(green); }
    const red = document.createElement('div'); red.className = 'nmsg'; red.style.width = '100%'; red.textContent = data.message || '';
    lineStyle(red, N.msg_font, N.msg_size, N.msg_weight, N.msg_color, N.msg_outline, N.msg_outline_op, N.msg_maxw, N.red_bg, N.red_bg_opacity, N.red_radius); win.append(red);
    stack.append(win);
    const mbox = mediaBox(N, opts);
    if (mbox) {
      if (N.media_pos === 'left') { notif.style.flexDirection='row'; notif.append(mbox, stack); }
      else if (N.media_pos === 'right') { notif.style.flexDirection='row'; notif.append(stack, mbox); }
      else if (N.media_pos === 'bottom') { notif.append(stack, mbox); }
      else notif.append(mbox, stack);
    } else notif.append(stack);
    return { notif, red, msgBase: N.msg_size, msgH: N.msg_maxh };
  }
  function fitMessage(el, maxh, base) {
    el.style.minHeight = Math.round(base * 1.32) + 'px'; el.style.maxHeight = maxh + 'px'; el.style.overflow = 'hidden';
    let size = base; el.style.fontSize = size + 'px'; let g = 80;
    while (el.scrollHeight > maxh + 1 && size > 8 && g-- > 0) { size -= 1; el.style.fontSize = size + 'px'; }
  }
  /* pool rules only compete for amounts inside their own range, so a 500-800 pool
     never plays for 400; the default rule is the global pool because it spans 10-29999 */
  function rangeHit(r, amount) { const mn = r.min || 0, mx = r.max || 0; return amount + 1e-9 >= mn && (mx <= 0 || amount <= mx + 1e-9); }
  function pickPooled(rules, amount) {
    const pool = rules.filter(r => r.id !== 'test' && (r.id === 'default' || r.pool) && rangeHit(r, amount));
    if (!pool.length) return (rules.find(r => r.id === 'default') || {}).style || {};
    return pool[Math.floor(Math.random() * pool.length)].style || {};
  }
  window.Alert = { DEFAULTS, fmt, hexA, sub, isVidUrl, outline, animFrom, lineStyle, mediaBox, build, fitMessage, rangeHit, pickPooled };
})();
