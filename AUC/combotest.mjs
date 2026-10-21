import { spawn } from 'node:child_process';
import { mkdirSync, rmSync, copyFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = 'J:/MONOBANK';
const TMP = 'C:/Users/TERRIB~1/AppData/Local/Temp/opencode/exe-combo';
const PORT = 8894;
const BASE = `http://127.0.0.1:${PORT}`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let pass = 0, fail = 0; const fails = [];
const ok = (n, c, e = '') => { if (c) pass++; else { fail++; fails.push(`${n} ${e}`); console.log('  FAIL ' + n + ' ' + e); } };
const api = async (p, b) => { const r = await fetch(BASE + p, b ? { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(b) } : undefined); const t = await r.text(); try { return JSON.parse(t); } catch { throw new Error(`${p} -> ${r.status} ${t.slice(0, 50)}`); } };

rmSync(TMP, { recursive: true, force: true }); mkdirSync(TMP, { recursive: true });
copyFileSync(join(ROOT, 'LocalMonoDonate.exe'), join(TMP, 'LocalMonoDonate.exe'));
writeFileSync(join(TMP, 'config.json'), JSON.stringify({ host: '127.0.0.1', port: PORT, auctionPort: PORT + 1, nickname: 'test', jars: ['TESTJAR'], token: '', features: { auction: true, timer: true } }));
const srv = spawn(join(TMP, 'LocalMonoDonate.exe'), ['--headless'], { cwd: TMP, stdio: 'ignore' });
async function waitUp() { for (let i = 0; i < 100; i++) { try { if ((await fetch(BASE + '/api/auc/state')).ok) return true; } catch {} await sleep(200); } return false; }

async function clearLots() { const s = await api('/api/auc/state'); for (const l of (s.lots || [])) await api('/api/auc/lots/delete', { lotId: l.id }); }

async function combo(mode, source, currency) {
  const tag = `${mode}/${source}/${currency}`;
  await api('/api/auc/settings', { mode, source, currency });
  const s0 = await api('/api/auc/settings');
  ok(`${tag}: settings saved`, s0.mode === mode && s0.source === source && s0.currency === currency, JSON.stringify(s0).slice(0, 80));
  await clearLots();
  const before = (await api('/api/auc/state')).pending.length;
  const lot = (await api('/api/auc/lots', { title: 'Alpha' })).lot;
  // one bid from each source
  await api('/api/auc/bid', { username: 'tw', amount: 10, userInput: 'Alpha', source: 'twitch', redemptionId: 'R-' + tag });
  await api('/api/auc/bid', { username: 'mb', amount: 10, userInput: 'Alpha', source: 'monobank', redemptionId: 'M-' + tag });
  await api('/api/auc/bid', { username: 'mn', amount: 10, userInput: 'Alpha', source: 'manual' });
  const st = await api('/api/auc/state');
  const A = (st.lots || []).find((l) => l.title === 'Alpha') || { total: 0, participants: 0 };
  const dpend = (st.pending || []).length - before;
  // accepted = the matching-source bid + the always-allowed manual bid = 2 of 3
  if (mode === 'manual') {
    ok(`${tag}: manual -> 2 pending (3rd source-filtered)`, dpend === 2 && A.total === 0, `dpend=${dpend} total=${A.total}`);
  } else {
    ok(`${tag}: auto -> 20 applied (source-filtered)`, A.total === 20 && dpend === 0, `total=${A.total} dpend=${dpend}`);
  }
  // winner (top) over the pool
  await api('/api/auc/winner', { mode: 'top' });
  const w = await api('/api/auc/state');
  ok(`${tag}: winner picked`, !!w.winner || (w.lots || []).length === 0, JSON.stringify(w.winner || {}).slice(0, 60));
  void lot;
}

async function rewardCostsMatrix() {
  const sets = [[1000], [1000, 5000], [10000, 50000, 100000], [5000], [1000, 5000, 10000, 50000, 100000], []];
  for (const set of sets) {
    await api('/api/auc/settings', { rewardCosts: set });
    const got = (await api('/api/auc/settings')).rewardCosts;
    ok(`rewardCosts ${JSON.stringify(set)}`, JSON.stringify(got) === JSON.stringify(set), JSON.stringify(got));
  }
}

async function timerExt() {
  const rem = async () => { const s = await api('/api/auc/state'); return s.running && s.endsAt ? s.endsAt - Date.now() : s.remainingMs; };
  await api('/api/auc/settings', { mode: 'auto', source: 'points', leaderChangePoints: 60, addExistingPoints: 30, newPositionPoints: 0, timeMoreThanMin: 0 });
  await clearLots();
  await api('/api/auc/lots', { title: 'Alpha' }); await api('/api/auc/lots', { title: 'Beta' });
  await api('/api/auc/start', {});
  const r0 = await rem();
  await api('/api/auc/bid', { username: 'a', amount: 10, userInput: 'Alpha', source: 'twitch' }); const r1 = await rem();
  ok('leader change extends ~60s', Math.abs((r1 - r0) - 60000) < 2500, String(Math.round(r1 - r0)));
  await api('/api/auc/bid', { username: 'd', amount: 5, userInput: 'Alpha', source: 'twitch' }); const r2 = await rem();
  ok('add-to-existing extends ~30s', Math.abs((r2 - r1) - 30000) < 2500, String(Math.round(r2 - r1)));
  await api('/api/auc/settings', { timeMoreThanMin: 1 });
  await api('/api/auc/bid', { username: 'e', amount: 500, userInput: 'Beta', source: 'twitch' }); const r3 = await rem();
  ok('time-more-than exception suppresses', Math.abs(r3 - r2) < 2500, String(Math.round(r3 - r2)));
}

(async () => {
  if (!(await waitUp())) { console.log('no server'); process.exit(1); }
  console.log('== mode x source x currency matrix ==');
  for (const mode of ['auto', 'manual']) for (const source of ['points', 'donations']) for (const currency of ['UAH', 'USD']) await combo(mode, source, currency);
  console.log('\n== reward-cost subsets ==');
  await rewardCostsMatrix();
  console.log('\n== timer extensions ==');
  await timerExt();
  console.log(`\n==== ${pass} passed, ${fail} failed ====`);
  if (fails.length) console.log('FAILURES:\n' + fails.join('\n'));
  try { srv.kill(); } catch {}
  process.exit(fail ? 1 : 0);
})();
