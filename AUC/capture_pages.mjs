import { spawn } from 'node:child_process';
import { mkdirSync, rmSync, copyFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { chromium } from 'playwright-core';

const ROOT = 'J:/MONOBANK';
const TMP = 'C:/Users/TERRIB~1/AppData/Local/Temp/opencode/exe-shots';
const OUT = 'J:/MONOBANK/shots';
const PORT = 8899;
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const BASE = `http://127.0.0.1:${PORT}`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

rmSync(TMP, { recursive: true, force: true }); mkdirSync(TMP, { recursive: true });
rmSync(OUT, { recursive: true, force: true }); mkdirSync(OUT, { recursive: true });
copyFileSync(join(ROOT, 'LocalMonoDonate.exe'), join(TMP, 'LocalMonoDonate.exe'));
writeFileSync(join(TMP, 'config.json'), JSON.stringify({ host: '127.0.0.1', port: PORT, auctionPort: PORT + 1, nickname: 'test', overlayname: 'ovl', statsToken: 'tok', jars: ['TESTJAR'], token: '', features: { donate: true, stats: true, overlay: true, bits: true, bitsOverlay: true, youtube: true, timer: true, auction: true } }));
const srv = spawn(join(TMP, 'LocalMonoDonate.exe'), ['--headless'], { cwd: TMP, stdio: 'ignore' });
async function waitUp() { for (let i = 0; i < 100; i++) { try { if ((await fetch(BASE + '/setup')).ok) return true; } catch {} await sleep(200); } return false; }

const PAGES = [
  ['setup', '/setup'], ['notify', '/notify'], ['bitsedit', '/bitsedit'], ['donateedit', '/donateedit'],
  ['timeradmin', '/timeradmin'], ['stats', '/check'], ['bitsform', '/bitsform'], ['form', '/form'],
  ['bits', '/bits'], ['overlay', '/overlay'], ['timer', '/timer'], ['aucsettings', '/auctionsettings'],
  ['youtube', '/youtube'],
];

(async () => {
  if (!(await waitUp())) { console.log('no server'); process.exit(1); }
  const browser = await chromium.launch({ executablePath: CHROME, headless: true });
  const index = [];
  for (const [label, path] of PAGES) {
    for (const theme of ['dark', 'light']) {
      for (const lang of ['en', 'ua']) {
        const ctx = await browser.newContext({ viewport: { width: 1500, height: 950 } });
        const page = await ctx.newPage();
        await page.addInitScript(([t, l]) => { try { localStorage.setItem('lm_theme', t); localStorage.setItem('lm_lang', l); } catch {} }, [theme, lang]);
        await page.goto(BASE + path, { waitUntil: 'domcontentloaded' }).catch(() => {});
        await sleep(650);
        const file = `${label}.${theme}.${lang}.png`;
        await page.screenshot({ path: join(OUT, file), fullPage: true }).catch(() => {});
        // visible text snapshot (labels/notes/buttons) for review
        const txt = await page.evaluate(() => [...document.querySelectorAll('.sec,.field>label,.note,.seg button,button,h1,h2,h3,.pill,.k,.u')].map((e) => e.textContent.trim()).filter(Boolean).slice(0, 80)).catch(() => []);
        index.push({ page: label, theme, lang, file, text: txt });
        await ctx.close();
      }
    }
  }
  // AUC host page (dark only)
  {
    const ctx = await browser.newContext({ viewport: { width: 1500, height: 950 } });
    const page = await ctx.newPage();
    await page.goto(`http://test.auction.localhost:${PORT}/`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await sleep(900);
    await page.screenshot({ path: join(OUT, 'auction-host.dark.en.png'), fullPage: true }).catch(() => {});
    await ctx.close();
  }
  writeFileSync(join(OUT, 'index.json'), JSON.stringify(index, null, 1));
  await browser.close();
  try { srv.kill(); } catch {}
  console.log(`captured ${index.length + 1} screenshots + index.json into ${OUT}`);
  process.exit(0);
})();
