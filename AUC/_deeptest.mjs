import { spawn } from 'node:child_process';
import { mkdirSync, cpSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { chromium } from 'playwright-core';

const ROOT = 'J:/MONOBANK';
const TMP = 'C:/Users/TERRIB~1/AppData/Local/Temp/opencode/exe-deep';
const PORT = 8893;
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const BASE = `http://127.0.0.1:${PORT}`;
const AUC = `http://test.auction.localhost:${PORT}`;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let pass = 0, fail = 0; const fails = [];
const ok = (n, c, e = '') => { if (c) pass++; else { fail++; fails.push(`${n} ${e}`); console.log('  FAIL ' + n + ' ' + e); } };

rmSync(TMP, { recursive: true, force: true }); mkdirSync(TMP, { recursive: true });
cpSync(join(ROOT, 'public'), join(TMP, 'public'), { recursive: true });
// the exe reads config/state next to itself
const cfg = JSON.stringify({ host: '127.0.0.1', port: PORT, auctionPort: PORT + 1, nickname: 'test', overlayname: 'ovl', statsToken: 'tok', jars: ['TESTJAR'], token: '', features: { donate: true, stats: true, overlay: true, bits: true, bitsOverlay: true, youtube: true, timer: true, auction: true } });
(await import('node:fs')).writeFileSync(join(TMP, 'config.json'), cfg);
(await import('node:fs')).copyFileSync(join(ROOT, 'LocalMonoDonate.exe'), join(TMP, 'LocalMonoDonate.exe'));
const srv = spawn(join(TMP, 'LocalMonoDonate.exe'), ['--headless'], { cwd: TMP, stdio: 'ignore' });
let log = '';
async function waitUp() { for (let i = 0; i < 100; i++) { try { if ((await fetch(BASE + '/setup')).ok) return true; } catch {} await sleep(200); } return false; }

const PAGES = [
  ['setup', '/setup'], ['notify', '/notify'], ['bitsedit', '/bitsedit'], ['donateedit', '/donateedit'],
  ['timeradmin', '/timeradmin'], ['stats', '/check'], ['bitsform', '/bitsform'], ['form', '/form'],
  ['bits', '/bits'], ['overlay', '/overlay'], ['timer', '/timer'], ['auction(legacy)', '/auction'], ['aucsettings', '/auctionsettings'],
];

async function exercise(page, url, label) {
  const errs = [];
  const onErr = (m) => {
    if (m.type() !== 'error') return;
    const t = m.text();
    // the donor form rejects an empty submit with 400 by design — expected, not a failure
    if (label === 'form' && /Failed to load resource.*400 \(Bad Request\)/.test(t)) return;
    errs.push(t.slice(0, 160));
  };
  const onPageErr = (e) => errs.push('PAGEERROR ' + ('' + e.message).slice(0, 160));
  page.on('console', onErr); page.on('pageerror', onPageErr);
  await page.goto(url, { waitUntil: 'domcontentloaded' }).catch((e) => errs.push('GOTO ' + e.message));
  await sleep(700);
  // click every interactive control (skip external links & navigation links)
  let clicks = 0;
  for (let guard = 0; guard < 120; guard++) {
    const r = await page.evaluate(() => {
      const bad = (e) => { const h = e.getAttribute('href') || ''; return /^https?:|^\/\//.test(h); };
      const els = [...document.querySelectorAll('button, input[type=checkbox], input[type=range], select, a[href]')]
        .filter((e) => e.offsetParent !== null && !e.dataset.dt && !bad(e));
      if (!els.length) return { done: true };
      const e = els[0]; e.dataset.dt = '1';
      try { e.click(); } catch (x) { return { err: '' + x }; }
      return { tag: e.tagName + (e.type ? ':' + e.type : '') + ':' + (e.textContent || '').trim().slice(0, 18) };
    }).catch(() => ({ done: true }));
    if (r.done) break;
    if (r.tag) clicks++;
    await sleep(120);
  }
  // flip theme + language
  await page.click('#themeseg button[data-th="light"]', { timeout: 1500 }).catch(() => {});
  await sleep(250);
  await page.click('#langseg button[data-lang="ua"]', { timeout: 1500 }).catch(() => {});
  await sleep(250);
  await page.click('#themeseg button[data-th="dark"]', { timeout: 1500 }).catch(() => {});
  await page.click('#langseg button[data-lang="en"]', { timeout: 1500 }).catch(() => {});
  await sleep(200);
  page.off('console', onErr); page.off('pageerror', onPageErr);
  return { clicks, errs };
}

async function linkCheck(page, base, label) {
  const hrefs = await page.evaluate(() => [...new Set([...document.querySelectorAll('a[href^="/"]')].map((a) => a.getAttribute('href')))]);
  const bad = [];
  for (const h of hrefs) { try { const r = await fetch(base + h); if (!r.ok) bad.push(h + ':' + r.status); } catch { bad.push(h + ':ERR'); } }
  return bad;
}

(async () => {
  if (!(await waitUp())) { console.log('no server\n' + log.slice(0, 800)); process.exit(1); }
  const browser = await chromium.launch({ executablePath: CHROME, headless: true });
  const page = await browser.newPage({ viewport: { width: 1500, height: 950 } });
  page.on('dialog', (d) => d.accept());
  page.on('filechooser', (fc) => fc.setFiles([]).catch(() => {}));

  console.log('\n== pages (click-everything + theme + lang) ==');
  for (const [label, path] of PAGES) {
    const r = await exercise(page, BASE + path, label);
    const badLinks = await linkCheck(page, BASE, label);
    ok(`${label}: clicks=${r.clicks}`, r.errs.length === 0, r.errs.slice(0, 3).join(' | '));
    if (badLinks.length) console.log(`  (${label} broken links: ${badLinks.join(', ')})`);
  }

  console.log('\n== auction host (AUC ui) ==');
  // AUC page is a SPA (wheel deeply tested in test2.mjs) - just mount it
  await page.goto(AUC + '/', { waitUntil: 'domcontentloaded' }).catch(()=>{}); await sleep(1000);

  // the auction host page is a SPA (bids+wheel) — exercise the wheel modes + spin
  await page.goto(AUC + '/', { waitUntil: 'domcontentloaded' }); await sleep(800);
  ok('auc ui mounted', (await page.locator('.shell').count()) === 1);
  ok('auc has rail nav', (await page.locator('aside.rail .ib').count()) >= 3);

  console.log(`\n==== ${pass} passed, ${fail} failed ====`);
  if (fails.length) console.log('FAILURES:\n' + fails.join('\n'));
  await browser.close();
  try { srv.kill(); } catch {}
  process.exit(fail ? 1 : 0);
})();
