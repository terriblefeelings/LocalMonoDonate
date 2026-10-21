// Quick regression smoke — recent changes (bits image+sound) + core features (bids, donations, timer).
// Run: node _smoke.mjs
import { spawn } from 'node:child_process';
import { mkdirSync, rmSync, copyFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = 'J:/MONOBANK', TMP = 'C:/Users/TERRIB~1/AppData/Local/Temp/opencode/exe-smoke', PORT = 8897;
const BASE = `http://127.0.0.1:${PORT}`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let pass = 0, fail = 0;
const ok = (n, c, e = '') => { if (c) pass++; else { fail++; console.log('  FAIL ' + n + (e ? '  ' + e : '')); } };
const api = async (p, b) => { const r = await fetch(BASE + p, b ? { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(b) } : undefined); const t = await r.text(); try { return JSON.parse(t); } catch { throw new Error(p + ' -> ' + r.status); } };

rmSync(TMP, { recursive: true, force: true }); mkdirSync(TMP, { recursive: true });
copyFileSync(join(ROOT, 'LocalMonoDonate.exe'), join(TMP, 'LocalMonoDonate.exe'));
writeFileSync(join(TMP, 'config.json'), JSON.stringify({ host: '127.0.0.1', port: PORT, auctionPort: PORT + 1, nickname: 'test', overlayname: 'ovl', statsToken: 'tok', jars: ['TESTJAR'], token: '', features: { donate: true, stats: true, overlay: true, bits: true, bitsOverlay: true, youtube: true, timer: true, auction: true } }));
const srv = spawn(join(TMP, 'LocalMonoDonate.exe'), ['--headless'], { cwd: TMP, stdio: 'ignore' });
async function waitUp() { for (let i = 0; i < 100; i++) { try { if ((await fetch(BASE + '/setup')).ok) return true; } catch {} await sleep(200); } return false; }

(async () => {
  if (!(await waitUp())) { console.log('no server'); process.exit(1); }

  // bits: image kind + sound (NEW fields)
  await api('/api/bits', { bits: [{ slot: 0, label: 'pic', kind: 'image', url: '/assets/default.jpg', sound: '/media/x.mp3', img: '', price: 100, code: 'IMG1' }], grid: 5 });
  const bb = (await api('/api/bits')).bits[0];
  ok('bits: kind=image preserved', bb.kind === 'image', 'kind=' + bb.kind);
  ok('bits: sound preserved', bb.sound === '/media/x.mp3', 'sound=' + bb.sound);

  // bids (AUC)
  await api('/api/auc/settings', { mode: 'auto', source: 'points' });
  const s0 = await api('/api/auc/state'); for (const l of (s0.lots || [])) await api('/api/auc/lots/delete', { lotId: l.id });
  await api('/api/auc/lots', { title: 'LotA' });
  await api('/api/auc/bid', { username: 'viewer1', amount: 250, userInput: 'LotA', source: 'manual' });
  const la = (await api('/api/auc/state')).lots.find((l) => l.title === 'LotA');
  ok('bid applied (total 250, 1 participant)', la && la.total === 250 && la.participants === 1, JSON.stringify(la));

  // donation form
  const st = await api('/api/state'); const amt = Math.max(st.minAmount, Math.min(st.maxAmount, 100));
  const sub = await api('/api/submit', { amount: amt, name: 'D', message: 'hi' });
  ok('donation submit -> code', sub.ok === true && !!sub.code, JSON.stringify(sub).slice(0, 60));

  // marathon timer
  await api('/api/timer', { action: 'set', seconds: 600 });
  ok('timer set 600s', Math.abs((await api('/api/timer')).remainingMs - 600000) < 2500);

  console.log(`\n==== ${pass} passed, ${fail} failed ====`);
  try { srv.kill(); } catch {}
  process.exit(fail ? 1 : 0);
})();
