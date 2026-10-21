// Deep end-to-end test of the merged LocalMonoDonate exe.
// Covers: every page route + alias, every asset, the auction subdomain host,
// config/features/state, donations (form + test), marathon timer, bits, notify,
// legacy auction, the merged AUC (lots/bids/matching/pending/winner/timer),
// twitch settings, upload/media, theme.
// Run: node _alltest.mjs
import { spawn } from 'node:child_process';
import http from 'node:http';
import { mkdirSync, rmSync, copyFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = 'J:/MONOBANK';
const TMP = 'C:/Users/TERRIB~1/AppData/Local/Temp/opencode/exe-alltest';
const PORT = 8896;
const BASE = `http://127.0.0.1:${PORT}`;
const AUCHOST = `test.auction.localhost:${PORT}`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let pass = 0, fail = 0; const fails = [];
const ok = (n, c, e = '') => { if (c) pass++; else { fail++; fails.push(n + (e ? ' — ' + e : '')); console.log('  FAIL ' + n + (e ? '  ' + e : '')); } };
const sec = (t) => console.log('\n== ' + t + ' ==');

const api = async (p, b) => {
  const r = await fetch(BASE + p, b !== undefined ? { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(b) } : undefined);
  const t = await r.text();
  try { return JSON.parse(t); } catch { throw new Error(`${p} -> ${r.status} ${t.slice(0, 60)}`); }
};
const text = async (p, base = BASE) => { const r = await fetch(base + p); return { status: r.status, body: await r.text() }; };

rmSync(TMP, { recursive: true, force: true }); mkdirSync(TMP, { recursive: true });
copyFileSync(join(ROOT, 'LocalMonoDonate.exe'), join(TMP, 'LocalMonoDonate.exe'));
writeFileSync(join(TMP, 'config.json'), JSON.stringify({
  host: '127.0.0.1', port: PORT, auctionPort: PORT + 1, nickname: 'test', overlayname: 'ovl', statsToken: 'tok',
  jars: ['TESTJAR'], token: '',
  features: { donate: true, stats: true, overlay: true, bits: true, bitsOverlay: true, youtube: true, timer: true, auction: true },
}));
const srv = spawn(join(TMP, 'LocalMonoDonate.exe'), ['--headless'], { cwd: TMP, stdio: 'ignore' });
async function waitUp() { for (let i = 0; i < 120; i++) { try { if ((await fetch(BASE + '/setup')).ok) return true; } catch {} await sleep(200); } return false; }

/* ---------------------------------------------------------------- pages */
async function testPages() {
  sec('pages — every route + alias');
  // kind: 2 = admin (head.js + i18n.js), 1 = public (i18n.js + lang switch), 0 = overlay (no text)
  const PAGES = [
    ['setup', '/setup', 2], ['auctionsettings', '/auctionsettings', 2], ['notify', '/notify', 2],
    ['bitsedit', '/bitsedit', 2], ['donateedit', '/donateedit', 2], ['timeradmin', '/timeradmin', 2],
    ['donor /form', '/form', 1], ['donor /banka', '/banka', 1], ['donor /donatiks', '/donatiks', 1],
    ['stats /check', '/check', 1], ['stats /stats', '/stats', 1], ['stats /', '/', 1],
    ['overlay /overlay', '/overlay', 0], ['overlay /main', '/main', 0],
    ['bits overlay /bits', '/bits', 0], ['bits donor /bitsform', '/bitsform', 1],
    ['youtube /youtube', '/youtube', 0], ['youtube /yt', '/yt', 0],
    ['qr /qr', '/qr', 0],
    ['timer overlay /timer', '/timer', 0], ['auction /auction', '/auction', 0],
  ];
  for (const [name, path, kind] of PAGES) {
    const { status, body } = await text(path);
    ok(`page ${name} -> 200`, status === 200, 'status ' + status);
    ok(`page ${name} html`, body.length > 200 && /<html|<!doctype/i.test(body), 'len ' + body.length);
    ok(`page ${name} clean`, !body.includes('???') && !body.includes('\uFFFD'));
    if (kind > 0) {
      ok(`page ${name} i18n dict en+ua+data-i18n`, body.includes('I18N') && /\bua\b\s*[:=]/.test(body) && body.includes('data-i18n'));
      if (kind === 2) ok(`page ${name} admin: head.js + i18n.js`, body.includes('/head.js') && body.includes('/i18n.js'));
      else ok(`page ${name} public: i18n.js + lang switch`, body.includes('/i18n.js') && /langseg|lm_lang|data-lang/.test(body));
    }
  }
  sec('assets');
  for (const [n, p] of [['app.css', '/app.css'], ['head.js', '/head.js'], ['i18n.js', '/i18n.js'], ['auc.js', '/auc.js'], ['auc.css', '/auc.css'], ['alert.js', '/alert.js'], ['qrcode.js', '/qrcode.js'], ['default.jpg', '/assets/default.jpg'], ['rat.mp4', '/rat.mp4']]) {
    ok(`asset ${n} 200`, (await fetch(BASE + p)).status === 200);
  }
  ok('favicon 204', (await fetch(BASE + '/favicon.ico')).status === 204);
  sec('subdomain host routing');
  const hostGet = (host, path) => new Promise((res) => {
    const req = http.request({ host: '127.0.0.1', port: PORT, path, headers: { Host: host } }, (r) => {
      let b = ''; r.on('data', (d) => (b += d)); r.on('end', () => res({ status: r.statusCode, body: b }));
    });
    req.on('error', (e) => res({ status: 0, body: e.message })); req.end();
  });
  const root = await hostGet(AUCHOST, '/');
  ok('auction host / -> AUC shell', root.status === 200 && /PointAUC|auc\.js/i.test(root.body), 'status ' + root.status);
  ok('auction host /auction', (await hostGet(AUCHOST, '/auction')).status === 200);
  ok('auction host /bits -> bits overlay', (await hostGet(AUCHOST, '/bits')).status === 200);
  ok('auction host /youtube', (await hostGet(AUCHOST, '/youtube')).status === 200);
  ok('auction host /timer', (await hostGet(AUCHOST, '/timer')).status === 200);
  ok('donate subdomain /bits -> bits donor', (await hostGet('test.donate.localhost:' + PORT, '/bits')).status === 200);
  ok('overlay subdomain / -> overlay', (await hostGet('test.ovl.localhost:' + PORT, '/')).status === 200);
  ok('overlay subdomain /qr -> qr widget', (await hostGet('test.ovl.localhost:' + PORT, '/qr')).status === 200);
}

/* ---------------------------------------------------------------- core */
async function testCore() {
  sec('config / features / state');
  const cfg = await api('/api/config');
  ok('config: nickname+jars+features+links', cfg.nickname === 'test' && Array.isArray(cfg.jars) && !!cfg.features && !!cfg.links);
  ok('config: every link kind', !!(cfg.links.donate && cfg.links.stats && cfg.links.overlay && cfg.links.bitsPage && cfg.links.bitsOverlay && cfg.links.youtube && cfg.links.qr), JSON.stringify(cfg.links || {}).slice(0, 120));
  await api('/api/config', { nickname: 'test', overlayname: 'ovl', jars: ['TESTJAR'], token: '', widgetJar: 'TESTJAR' });
  const cfg2 = await api('/api/config');
  ok('config POST round-trip', cfg2.nickname === 'test');
  ok('config: widget jar persisted', cfg2.widgetJar === 'TESTJAR', JSON.stringify(cfg2.widgetJar));
  const f = await api('/api/features');
  ok('features: all true', !!(f.donate && f.stats && f.overlay && f.bits && f.bitsOverlay && f.youtube && f.timer && f.auction && f.qr), JSON.stringify(f));
  const up = await api('/api/update');
  ok('update: {current, available}', typeof up.current === 'string' && typeof up.available === 'boolean', JSON.stringify(up));
  const st = await api('/api/state');
  ok('state: jars + links + minAmount', Array.isArray(st.jars) && !!st.links && typeof st.minAmount === 'number');
  const h = await api('/api/history?jar=0'); ok('history: {jar,items}', h.jar === 0 && Array.isArray(h.items));
  ok('newstats: returns token', !!(await api('/api/newstats', {})).statsToken);
  ok('fonts: array', Array.isArray((await api('/api/fonts')).fonts));
  ok('theme POST ok', (await api('/api/theme', { light: true })).ok === true);
  await api('/api/theme', { light: false });
  ok('poll 200', (await fetch(BASE + '/api/poll?since=0')).status === 200);
}

/* ---------------------------------------------------------------- donations */
async function testDonations() {
  sec('donations — form + test');
  ok('test donation ok', (await api('/api/test', { amount: 100, name: 'Tester', message: 'hi' })).ok === true);
  const { minAmount: mn, maxAmount: mx } = await api('/api/state');
  const amt = Math.max(mn, Math.min(mx, 100));
  const sub = await api('/api/submit', { amount: amt, name: 'Donor', message: 'hello' });
  ok('submit: code + monobank redirect', sub.ok === true && !!sub.code && /send\.monobank\.ua\/jar\//.test(sub.redirect || ''), JSON.stringify(sub).slice(0, 90));
  ok('submit: empty name rejected', (await api('/api/submit', { amount: amt, name: '', message: 'x' })).ok === false);
  ok('submit: empty message rejected', (await api('/api/submit', { amount: amt, name: 'x', message: '' })).ok === false);
  ok('submit: below-min rejected', (await api('/api/submit', { amount: Math.max(0, mn - 1), name: 'x', message: 'y' })).ok === false);
  ok('submit: non-integer rejected', (await api('/api/submit', { amount: amt + 0.5, name: 'x', message: 'y' })).ok === false);
}

/* ---------------------------------------------------------------- timer */
async function testTimer() {
  sec('marathon timer');
  const t0 = await api('/api/timer');
  ok('timer shape', typeof t0.running === 'boolean' && !!t0.cfg && Array.isArray(t0.log));
  await api('/api/timer', { action: 'set', seconds: 600 });
  ok('timer set 600s', Math.abs((await api('/api/timer')).remainingMs - 600000) < 2500);
  await api('/api/timer', { action: 'add', seconds: 60 });
  ok('timer add 60s', Math.abs((await api('/api/timer')).remainingMs - 660000) < 3000);
  await api('/api/timer', { action: 'sub', seconds: 60 });
  ok('timer sub 60s', Math.abs((await api('/api/timer')).remainingMs - 600000) < 3000);
  await api('/api/timer', { action: 'start' });
  ok('timer running', (await api('/api/timer')).running === true);
  await api('/api/timer', { action: 'pause' });
  ok('timer paused', (await api('/api/timer')).running === false);
  await api('/api/timer', { action: 'reset' });
  ok('timer reset -> 0', (await api('/api/timer')).remainingMs === 0);
  const ts = await api('/api/timer/settings', { perAmount: 10, perSeconds: 60, minDonation: 1, pollSeconds: 15, twitchTier1: 60, twitchTier2: 120, twitchTier3: 300, color: '#ffffff', accent: '#00ff00', outline: '#000000', size: 22 });
  ok('timer settings ok', ts.ok === true);
  const tc = (await api('/api/timer')).cfg;
  ok('timer settings applied', tc.perAmount === 10 && tc.twitchTier3 === 300, JSON.stringify(tc).slice(0, 100));
}

/* ---------------------------------------------------------------- bits */
async function testBits() {
  sec('bits');
  const b0 = await api('/api/bits');
  ok('bits GET shape', Array.isArray(b0.bits) && typeof b0.grid === 'number');
  const save = await api('/api/bits', { bits: [
    { slot: 0, label: 'Beep', kind: 'sound', url: '/media/beep.mp3', img: '', ruleId: '', price: 100, code: '' },
    { slot: 1, label: 'Bad kind', kind: 'weird', url: '', img: '', ruleId: '', price: 50, code: 'ABC' },
    { slot: 30, label: 'Too high', kind: 'alert', url: '', img: '', ruleId: '', price: 10, code: '' },
  ], grid: 4 });
  ok('bits POST ok + grid', save.ok === true && save.grid === 4);
  ok('bits: slot>=25 dropped, bad kind->alert, code generated', save.bits.length === 2 && save.bits[1].kind === 'alert' && save.bits[0].code.length > 0, JSON.stringify(save.bits));
  ok('bits grid clamp high', (await api('/api/bits', { grid: 99 })).grid === 7);
  ok('bits grid clamp low', (await api('/api/bits', { grid: 1 })).grid === 2);
}

/* ---------------------------------------------------------------- notify */
async function testNotify() {
  sec('notify rules');
  const n0 = await api('/api/notify');
  ok('notify GET shape', !!n0 && Array.isArray(n0.rules) && n0.rules.some((r) => r.id === 'default'));
  ok('notify round-trip ok', (await api('/api/notify', n0)).ok === true);
}

/* ---------------------------------------------------------------- legacy auction */
async function testAuction() {
  sec('legacy auction');
  await api('/api/auction', { action: 'reset' });
  ok('auction shape', typeof (await api('/api/auction')).active === 'boolean');
  await api('/api/auction', { action: 'bid', name: 'Bob', amount: 50 });
  await api('/api/auction', { action: 'bid', name: 'Ann', amount: 80 });
  ok('auction: 2 bids, total 130', (await api('/api/auction')).total === 130, 'total ' + (await api('/api/auction')).total);
  await api('/api/auction', { action: 'top' });
  ok('auction: winner top = 80', (await api('/api/auction')).winner?.amount === 80);
  await api('/api/auction', { action: 'spin' });
  ok('auction: spin increments', (await api('/api/auction')).spin >= 1);
  await api('/api/auction', { action: 'remove', name: 'Bob' });
  const a = await api('/api/auction');
  ok('auction: remove Bob -> 1 bid', a.bids.length === 1, JSON.stringify(a.bids.map((b) => b.name)));
  await api('/api/auction', { action: 'clear' });
  ok('auction: clear -> 0 bids', (await api('/api/auction')).bids.length === 0);
  ok('auction settings ok', (await api('/api/auction/settings', { timerSeconds: 600, autoEnd: false, minBid: 1, rewardCosts: [1000], rewardTitle: 'RW', title: 'Auc', color: '#ffffff', accent: '#00ff00', outline: '#000000' })).ok === true);
}

/* ---------------------------------------------------------------- AUC */
const clearLots = async () => { const s = await api('/api/auc/state'); for (const l of (s.lots || [])) await api('/api/auc/lots/delete', { lotId: l.id }); };
const aucSet = (o) => api('/api/auc/settings', o);

async function testAuc() {
  sec('AUC — settings');
  await aucSet({ mode: 'auto', source: 'points', currency: 'UAH', timerSeconds: 600, showTotalTime: false, leaderChangePoints: 0, leaderChangeDonations: 0, newPositionPoints: 0, newPositionDonations: 0, addExistingPoints: 0, addExistingDonations: 0, timeMoreThanMin: 0, rewardTitle: '', rewardCosts: [] });
  const s0 = await api('/api/auc/settings');
  ok('auc settings round-trip', s0.mode === 'auto' && s0.source === 'points' && s0.currency === 'UAH' && s0.timerSeconds === 600, JSON.stringify(s0).slice(0, 120));
  const stt = await api('/api/auc/state');
  ok('auc state has settings.timer + appearance', !!stt.settings?.timer && !!stt.settings?.appearance, JSON.stringify(Object.keys(stt.settings || {})));
  ok('auc emotes GET', Array.isArray((await api('/api/auc/emotes')).emotes));
  ok('auc bids GET array', Array.isArray(await api('/api/auc/bids')));
  ok('auc history GET array', Array.isArray(await api('/api/auc/history')));

  sec('AUC — lots + matching');
  await clearLots();
  await api('/api/auc/lots', { title: 'Alpha' });
  await api('/api/auc/lots', { title: 'Beta' });
  let s = await api('/api/auc/state');
  const A = s.lots.find((l) => l.title === 'Alpha'), B = s.lots.find((l) => l.title === 'Beta');
  ok('lots created + numbered 1,2', A?.number === 1 && B?.number === 2, JSON.stringify([A?.number, B?.number]));
  await api('/api/auc/bid', { username: 'u1', amount: 10, userInput: 'Alpha', source: 'manual' });
  ok('exact name -> Alpha total 10', (await api('/api/auc/state')).lots.find((l) => l.id === A.id).total === 10);
  await api('/api/auc/bid', { username: 'u2', amount: 20, userInput: '#2', source: 'manual' });
  ok('#2 -> Beta total 20', (await api('/api/auc/state')).lots.find((l) => l.id === B.id).total === 20);
  await api('/api/auc/bid', { username: 'u3', amount: 5, userInput: 'Alphha', source: 'manual' });
  s = await api('/api/auc/state');
  ok('fuzzy "Alphha" -> Alpha (total 15)', s.lots.find((l) => l.id === A.id).total === 15);
  ok('participants Alpha = 2', s.lots.find((l) => l.id === A.id).participants === 2);
  const p0 = (await api('/api/auc/state')).pending.length;
  await api('/api/auc/bid', { username: 'u4', amount: 7, userInput: 'zzz', source: 'manual' });
  const pend = (await api('/api/auc/state')).pending;
  ok('no-match -> pending', pend.length === p0 + 1);
  await api('/api/auc/pending/approve', { id: pend[0].id, lotId: A.id });
  ok('approve pending -> Alpha total 22', (await api('/api/auc/state')).lots.find((l) => l.id === A.id).total === 22);
  await api('/api/auc/bid', { username: 'u5', amount: 9, userInput: 'qqq', source: 'manual' });
  const p2 = (await api('/api/auc/state')).pending;
  await api('/api/auc/pending/decline', { id: p2[0].id });
  ok('decline pending removes it', (await api('/api/auc/state')).pending.length === p2.length - 1);

  sec('AUC — pin / adjust / delete');
  await api('/api/auc/lots/pin', { lotId: A.id });
  ok('pin toggles true', (await api('/api/auc/state')).lots.find((l) => l.id === A.id).pinned === true);
  await api('/api/auc/lots/adjust', { lotId: A.id, delta: 100 });
  ok('adjust +100 -> 122', (await api('/api/auc/state')).lots.find((l) => l.id === A.id).total === 122);
  await api('/api/auc/lots/delete', { lotId: B.id });
  ok('delete removes Beta', !(await api('/api/auc/state')).lots.find((l) => l.id === B.id));

  sec('AUC — source filter + dedup');
  await clearLots();
  await api('/api/auc/lots', { title: 'X' });
  const X = (await api('/api/auc/state')).lots[0];
  await aucSet({ mode: 'auto', source: 'points' });
  await api('/api/auc/bid', { username: 'mb', amount: 10, userInput: 'X', source: 'monobank' });
  ok('source=points blocks monobank', (await api('/api/auc/state')).lots[0].total === 0);
  await api('/api/auc/bid', { username: 'tw', amount: 10, userInput: 'X', source: 'twitch', redemptionId: 'R1' });
  await api('/api/auc/bid', { username: 'tw', amount: 10, userInput: 'X', source: 'twitch', redemptionId: 'R1' });
  ok('dedup redemptionId (1 applied)', (await api('/api/auc/state')).lots[0].total === 10, 'total ' + (await api('/api/auc/state')).lots[0].total);
  await aucSet({ source: 'donations' });
  await api('/api/auc/bid', { username: 'tw2', amount: 10, userInput: 'X', source: 'twitch', redemptionId: 'R2' });
  ok('source=donations blocks twitch', (await api('/api/auc/state')).lots[0].total === 10);
  await api('/api/auc/bid', { username: 'mb2', amount: 30, userInput: 'X', source: 'monobank', redemptionId: 'M2' });
  ok('source=donations allows monobank', (await api('/api/auc/state')).lots[0].total === 40);
  void X;

  sec('AUC — manual mode queues');
  await aucSet({ mode: 'manual', source: 'points' });
  const before = (await api('/api/auc/state')).pending.length;
  await api('/api/auc/bid', { username: 'm1', amount: 5, userInput: 'X', source: 'manual' });
  ok('manual mode -> pending', (await api('/api/auc/state')).pending.length === before + 1);

  sec('AUC — winner');
  await aucSet({ mode: 'auto', source: 'points' });
  await clearLots();
  await api('/api/auc/lots', { title: 'Lo' }); await api('/api/auc/lots', { title: 'Hi' });
  await api('/api/auc/bid', { username: 'a', amount: 10, userInput: 'Lo', source: 'manual' });
  await api('/api/auc/bid', { username: 'b', amount: 90, userInput: 'Hi', source: 'manual' });
  await api('/api/auc/winner', { mode: 'top' });
  ok('winner top -> Hi', (await api('/api/auc/state')).winner?.lotTitle === 'Hi', JSON.stringify((await api('/api/auc/state')).winner || {}).slice(0, 60));
  const Lo = (await api('/api/auc/state')).lots.find((l) => l.title === 'Lo');
  await api('/api/auc/winner', { mode: 'top', lotId: Lo.id });
  ok('winner by lotId -> Lo', (await api('/api/auc/state')).winner?.lotTitle === 'Lo');
  await api('/api/auc/winner', { mode: 'wheel' });
  ok('winner wheel -> a lot', !!(await api('/api/auc/state')).winner);

  sec('AUC — timer start/pause/add/reset');
  await api('/api/auc/start', {});
  ok('start -> running', (await api('/api/auc/state')).running === true);
  const rem0 = await (async () => { const s = await api('/api/auc/state'); return s.endsAt - Date.now(); })();
  await api('/api/auc/timer', { seconds: 60 });
  const rem1 = await (async () => { const s = await api('/api/auc/state'); return s.endsAt - Date.now(); })();
  ok('timer +60s while running', Math.abs((rem1 - rem0) - 60000) < 3000, String(Math.round(rem1 - rem0)));
  await api('/api/auc/pause', {});
  ok('pause -> not running', (await api('/api/auc/state')).running === false);
  await api('/api/auc/timer/reset', {});
  ok('timer/reset -> 600s', Math.abs((await api('/api/auc/state')).remainingMs - 600000) < 2000);

  sec('AUC — timer extensions (points vs donations)');
  await aucSet({ mode: 'auto', source: 'points', leaderChangePoints: 60, leaderChangeDonations: 120, addExistingPoints: 30, newPositionPoints: 0, timeMoreThanMin: 0 });
  await clearLots();
  await api('/api/auc/lots', { title: 'P1' }); await api('/api/auc/lots', { title: 'P2' });
  await api('/api/auc/start', {});
  const r = async () => { const s = await api('/api/auc/state'); return s.running && s.endsAt ? s.endsAt - Date.now() : s.remainingMs; };
  const e0 = await r();
  await api('/api/auc/bid', { username: 'a', amount: 10, userInput: 'P1', source: 'twitch', redemptionId: 'E1' });
  ok('points leader-change extends ~60s', Math.abs(((await r()) - e0) - 60000) < 3000, String(Math.round((await r()) - e0)));
  const e1 = await r();
  await api('/api/auc/bid', { username: 'b', amount: 5, userInput: 'P1', source: 'twitch', redemptionId: 'E2' });
  ok('points add-existing extends ~30s', Math.abs(((await r()) - e1) - 30000) < 3000, String(Math.round((await r()) - e1)));
  const e2 = await r();
  await api('/api/auc/bid', { username: 'c', amount: 999, userInput: 'P2', source: 'twitch', redemptionId: 'E3' });
  ok('points leader-change on new top extends ~60s', Math.abs(((await r()) - e2) - 60000) < 3500, String(Math.round((await r()) - e2)));
  const e3 = await r();
  await aucSet({ timeMoreThanMin: 1 });
  await api('/api/auc/bid', { username: 'd', amount: 1000, userInput: 'P1', source: 'twitch', redemptionId: 'E4' });
  ok('timeMoreThanMin exception suppresses', Math.abs((await r()) - e3) < 3000, String(Math.round((await r()) - e3)));
}

/* ---------------------------------------------------------------- twitch */
async function testTwitch() {
  sec('twitch settings');
  const tw = await api('/api/twitch');
  ok('twitch shape', typeof tw.authorized === 'boolean' && typeof tw.hasSecret === 'boolean');
  ok('twitch auth-url', /^https:\/\/id\.twitch\.tv\/oauth2\/authorize/.test((await api('/api/twitch/auth-url')).url || ''));
  await api('/api/twitch', { clientId: 'abc123', secret: 'sec' });
  const tw2 = await api('/api/twitch');
  ok('twitch save round-trip', tw2.clientId === 'abc123' && tw2.hasSecret === true, JSON.stringify(tw2).slice(0, 80));
  await api('/api/twitch', { clientId: '', secret: '' });
}

/* ---------------------------------------------------------------- media */
async function testMedia() {
  sec('upload / media');
  const up = await (await fetch(BASE + '/api/upload?name=test.bin', { method: 'POST', body: 'hello' })).json();
  ok('upload ok + url', up.ok === true && up.url === '/media/test.bin', JSON.stringify(up));
  const md = await fetch(BASE + '/media/test.bin');
  ok('media served back', md.ok && (await md.text()) === 'hello');
  ok('media traversal blocked', (await fetch(BASE + '/media/..%2Fconfig.json')).status !== 200);
}

(async () => {
  if (!(await waitUp())) { console.log('server did not start'); process.exit(1); }
  await testPages();
  await testCore();
  await testDonations();
  await testTimer();
  await testBits();
  await testNotify();
  await testAuction();
  await testAuc();
  await testTwitch();
  await testMedia();
  console.log(`\n==== ${pass} passed, ${fail} failed ====`);
  if (fails.length) console.log('FAILURES:\n' + fails.map((f) => ' - ' + f).join('\n'));
  try { srv.kill(); } catch {}
  process.exit(fail ? 1 : 0);
})();
