import { chromium } from 'playwright-core';
import { writeFileSync, mkdirSync } from 'node:fs';

const CHROME = process.env.CHROME || 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
const ROUTES = (process.argv[2] || '/').split(',');
mkdirSync('capture', { recursive: true });

const browser = await chromium.launch({ executablePath: CHROME, headless: true });
const ctx = await browser.newContext({ viewport: { width: 1440, height: 810 } });
const page = await ctx.newPage();

const logs = [];
const net = [];
page.on('console', (m) => logs.push(`${m.type()}: ${m.text()}`));
page.on('pageerror', (e) => logs.push(`pageerror: ${e.message}`));
page.on('request', (r) => { const u = r.url(); if (!u.startsWith('data:') && !u.includes('.woff') && !u.endsWith('.png') && !u.endsWith('.jpg') && !u.endsWith('.svg')) net.push(`${r.method()} ${u}${r.method() === 'POST' ? '  BODY=' + (r.postData() || '').slice(0, 200) : ''}`); });
page.on('response', (r) => { const u = r.url(); if (/\/api\/|socket|hub|events|graphql/i.test(u)) net.push(`  -> ${r.status()} ${u}`); });

for (const route of ROUTES) {
  const url = 'https://pointauc.com' + (route.startsWith('/') ? route : '/' + route);
  logs.push(`\n===== NAV ${url} =====`);
  try { await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 60000 }); } catch (e) { logs.push('goto error: ' + e.message); }
  await page.waitForTimeout(4500);
  const name = route === '/' ? 'home' : route.replace(/[^a-z0-9]+/gi, '_');
  writeFileSync(`capture/dom-${name}.html`, await page.content());
  await page.screenshot({ path: `capture/${name}.png`, fullPage: true }).catch(() => {});
  // dump computed styles of key elements
  const styles = await page.evaluate(() => {
    const pick = (sel) => { const el = document.querySelector(sel); if (!el) return null; const c = getComputedStyle(el); return { sel, bg: c.backgroundColor, color: c.color, font: c.fontFamily, size: c.fontSize, pad: c.padding, radius: c.borderRadius, h: Math.round(el.getBoundingClientRect().height), w: Math.round(el.getBoundingClientRect().width) }; };
    return {
      body: pick('body'), navbar: pick('.mantine-AppShell-navbar'), main: pick('.mantine-AppShell-main'),
      title: pick('.mantine-Title-root'), paper: pick('.mantine-Paper-root'), btn: pick('.mantine-Button-root'),
      input: pick('input'), text: document.body.innerText.slice(0, 1200),
    };
  });
  writeFileSync(`capture/styles-${name}.json`, JSON.stringify(styles, null, 2));
}

writeFileSync('capture/console.txt', logs.join('\n'));
writeFileSync('capture/network.txt', [...new Set(net)].join('\n'));
console.log('done. logs=' + logs.length + ' net=' + net.length);
await browser.close();
