import { useEffect, useRef, useState, type CSSProperties } from 'react';

/* --- icons --- */
const P: Record<string, string> = {
  logo: 'M6 3h12l3 6-9 12L3 9z M3 9h18 M9 3l3 6 3-6 M12 9v12',
  wheel: 'M12 3a9 9 0 1 0 9 9h-9V3z M12 12l7 5',
  chart: 'M4 20V10 M10 20V4 M16 20v-8 M22 20H2',
  gear: 'M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-2.9 1.2 2 2 0 1 1-4 0 1.7 1.7 0 0 0-2.9-1.2l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1A1.7 1.7 0 0 0 3 15a2 2 0 1 1 0-4 1.7 1.7 0 0 0 1.5-2.6l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1A1.7 1.7 0 0 0 10 4a2 2 0 1 1 4 0 1.7 1.7 0 0 0 2.7 1.5l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1A1.7 1.7 0 0 0 21 11a2 2 0 1 1 0 4z',
  play: 'M7 4l12 8-12 8z', pause: 'M8 5v14 M16 5v14', reset: 'M4 12a8 8 0 1 0 3-6 M4 20v-6h6', up: 'M6 14l6-6 6 6', down: 'M6 10l6 6 6-6', arrowUp: 'M12 20V6 M6 12l6-6 6 6',
  heart: 'M12 20s-7-4.3-9-8a5 5 0 0 1 9-3 5 5 0 0 1 9 3c-2 3.7-9 8-9 8z',
  trash: 'M4 7h16 M9 7V4h6v3 M6 7l1 13h10l1-13', dots: 'M5 12h.01 M12 12h.01 M19 12h.01', plus: 'M12 5v14 M5 12h14',
  search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14z M20 20l-4-4', percent: 'M19 5L5 19 M7 5a2 2 0 1 0 0 4 2 2 0 0 0 0-4z M17 15a2 2 0 1 0 0 4 2 2 0 0 0 0-4z',
  sort: 'M4 6h16 M6 12h12 M9 18h6', check: 'M5 12l4 4 10-10', close: 'M6 6l12 12 M18 6L6 18', lang: 'M4 5h10 M9 3v2 M6 5c0 5-2 8-4 9 M8 12c1 2 3 4 5 5 M13 21l4-9 4 9 M14.5 18h5',
  dice: 'M4 4h16v16H4z M8 8h.01 M16 8h.01 M12 12h.01 M8 16h.01 M16 16h.01', music: 'M9 18V5l10-2v13 M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0z M19 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0z',
};
const Icon = ({ n, size = 20 }: { n: string; size?: number }) => <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round"><path d={P[n] || P.dots} /></svg>;

/* --- i18n (EN / UA only) --- */
const T: Record<string, Record<string, string>> = {
  en: { auction: 'Auction', wheel: 'Wheel', settings: 'Settings', statistic: 'Statistic', newLot: 'New Lot Name', addLot: 'Add Lot', search: 'Search among lots...', pending: 'Pending bids', history: 'Bid history', manual: 'Manual', auto: 'Automatic', approve: 'Approve', decline: 'Decline', both: 'Points + Donations', points: 'Points only', donations: 'Donations only', showChances: 'Show winning chances', compact: 'Compact view', save: 'Save auction', upload: 'Upload auction', newAuction: 'New Auction', noLots: 'No lots yet - add one above, or they appear automatically from bids.', noPending: 'no pending bids', noHistory: 'no bids yet', timer: 'Timer', appearance: 'Appearance', bids: 'Bids general', start: 'Start', reset: 'Reset', addTime: 'Add Time', reduceTime: 'Reduce Time', addTimeX2: 'Add Time X5', addCost: 'Add Cost', delSlot: 'Delete Slot', extra: 'Extra', participant: 'Participant', sum: 'Sum', lot: 'Lot', source: 'Source', bid: 'Bid', amount: 'Amount', winner: 'Winner' },
  ua: { auction: 'Аукціон', wheel: 'Колесо', settings: 'Налаштування', statistic: 'Статистика', newLot: 'Назва лоту', addLot: 'Додати лот', search: 'Пошук серед лотів...', pending: 'Очікують', history: 'Історія ставок', manual: 'Вручну', auto: 'Автоматично', approve: 'Прийняти', decline: 'Відхилити', both: 'Бали + донати', points: 'Лише бали', donations: 'Лише донати', showChances: 'Шанси на перемогу', compact: 'Компактний вигляд', save: 'Зберегти аукціон', upload: 'Завантажити аукціон', newAuction: 'Новий аукціон', noLots: 'Ще немає лотів - додайте вище, або вони зʼявляться зі ставок.', noPending: 'немає заявок', noHistory: 'ще немає ставок', timer: 'Таймер', appearance: 'Вигляд', bids: 'Ставки', start: 'Старт', reset: 'Скинути', addTime: 'Додати час', reduceTime: 'Зменшити час', addTimeX2: 'Додати час X5', addCost: 'Додати суму', delSlot: 'Видалити', extra: 'Ще', participant: 'Учасник', sum: 'Сума', lot: 'Лот', source: 'Джерело', bid: 'Ставка', amount: 'Сума', winner: 'Переможець' },
};
const getLang = () => (localStorage.getItem('pa_lang') || 'en') as 'en' | 'ua';

interface Lot { id: string; number: number; title: string; aliases: string[]; lockedPercent: number; adjust: number; pinned: boolean; firstBidder: string; total: number; participants: number; }
interface Bid { id: string; lotId: string; lotTitle?: string; username: string; source: string; amount: number; userInput?: string; createdAt: number; }
interface TimerCfg { showTotalTime: boolean; leaderChangePoints: number; leaderChangeDonations: number; newPositionPoints: number; newPositionDonations: number; addExistingPoints: number; addExistingDonations: number; timeMoreThanMin: number; }
interface Appearance { background: string; geometry: string; primaryColor: string; showTotalAmount: boolean; showWinningChances: boolean; showViewerNames: boolean; autoParseLinks: boolean; hideValues: boolean; }
interface St { status: string; running: boolean; endsAt: number | null; remainingMs: number; totalMs: number; title: string; lots: Lot[]; pending: Bid[]; log: Bid[]; winner: any; integrations?: any; settings: { mode: string; source: string; currency: string; timerSeconds: number; timer: TimerCfg; appearance: Appearance }; }
const EMPTY: St = { status: 'idle', running: false, endsAt: null, remainingMs: 600000, totalMs: 600000, title: 'Auction', lots: [], pending: [], log: [], winner: null, settings: { mode: 'auto', source: 'both', currency: 'UAH', timerSeconds: 600, timer: { showTotalTime: false, leaderChangePoints: 0, leaderChangeDonations: 0, newPositionPoints: 0, newPositionDonations: 0, addExistingPoints: 0, addExistingDonations: 0, timeMoreThanMin: 0 }, appearance: { background: 'default', geometry: 'default', primaryColor: '#006bb9', showTotalAmount: false, showWinningChances: false, showViewerNames: true, autoParseLinks: false, hideValues: false } } };
const EXE = !['3000', '3001'].includes(location.port);
const apiUrl = (p: string) => (EXE ? p.replace(/^\/api/, '/api/auc') : p);
const api = (p: string, b?: any) => fetch(apiUrl(p), b ? { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(b) } : undefined).then((r) => r.json());

export default function App() {
  const [tab, setTab] = useState(location.hash.slice(1) || 'auction');
  const [ptab, setPtab] = useState<'pending' | 'history'>('pending');
  const [lang, setLang] = useState(getLang());
  const [st, setSt] = useState<St>(EMPTY);
  const [now, setNow] = useState(Date.now());
  const t = (k: string) => T[lang][k] || T.en[k] || k;
  const nav = (k: string) => { location.hash = k; };
  useEffect(() => {
    const on = () => setTab(location.hash.slice(1) || 'auction');
    window.addEventListener('hashchange', on);
    let close: () => void;
    if (EXE) { const p = setInterval(() => api('/api/state').then(setSt).catch(() => {}), 1000); close = () => clearInterval(p); }
    else { const ws = new WebSocket(`ws://${location.host}/ws`); ws.onmessage = (e) => { const m = JSON.parse(e.data); if (m.type === 'state') setSt(m.state); }; close = () => ws.close(); }
    api('/api/state').then(setSt);
    const i = setInterval(() => setNow(Date.now()), 250);
    return () => { window.removeEventListener('hashchange', on); close(); clearInterval(i); };
  }, []);
  const setLang2 = (l: 'en' | 'ua') => { localStorage.setItem('pa_lang', l); setLang(l); };
  useEffect(() => {
    const ap = st.settings.appearance; if (!ap) return;
    document.documentElement.style.setProperty('--accent', ap.primaryColor || '#006bb9');
    document.documentElement.setAttribute('data-geo', ap.geometry || 'default');
    document.documentElement.setAttribute('data-bg', ap.background || 'default');
  }, [st.settings.appearance]);
  const secs = Math.max(0, Math.floor((st.running && st.endsAt ? st.endsAt - now : st.remainingMs) / 1000));
  const hh = String(Math.floor(secs / 3600)).padStart(2, '0'), mm = String(Math.floor(secs % 3600 / 60)).padStart(2, '0'), ss = String(secs % 60).padStart(2, '0');
  const tsec = Math.max(0, Math.floor((st.totalMs || 0) / 1000));
  const tt = String(Math.floor(tsec / 3600)).padStart(2, '0') + ':' + String(Math.floor(tsec % 3600 / 60)).padStart(2, '0') + ':' + String(tsec % 60).padStart(2, '0');

  return (
    <div className={'shell' + (tab === 'wheel' ? ' no-side' : '')}>
      <aside className="rail">
        <button className={'ib' + (tab === 'auction' ? ' on' : '')} onClick={() => nav('auction')} title={t('auction')}><Icon n="logo" size={26} /></button>
        <button className={'ib' + (tab === 'wheel' ? ' on' : '')} onClick={() => nav('wheel')} title={t('wheel')}><Icon n="wheel" /></button>
        <div className="sp" />
        <button className="ib lang" onClick={() => setLang2(lang === 'en' ? 'ua' : 'en')} title="Language"><Icon n="lang" size={18} /><span>{lang.toUpperCase()}</span></button>
      </aside>

      <section className="main">
        <div className="titlerow"><b>{tab === 'wheel' ? t('wheel') : t('auction')}</b></div>
        {tab === 'wheel' ? <Wheel st={st} t={t} /> : <Auction st={st} t={t} />}
      </section>

      {tab !== 'wheel' && <aside className="side">
        <div className="timer">{hh}:{mm}:{ss}</div>
        {st.settings.timer.showTotalTime && <div className="totalTime">{tt}</div>}
        <div className="ctrls">
          <button className="ib" title={t('start')} onClick={() => api('/api/start', {})}><Icon n="play" /></button>
          <button className="ib" title="Pause" onClick={() => api('/api/pause', {})}><Icon n="pause" /></button>
          <button className="ib" title={t('reset')} onClick={() => api('/api/timer/reset', {})}><Icon n="reset" /></button>
          <button className="ib" title="+1 min" onClick={() => api('/api/timer', { seconds: 60 })}><Icon n="up" /></button>
          <button className="ib" title="-1 min" onClick={() => api('/api/timer', { seconds: -60 })}><Icon n="down" /></button>
          <button className="ib" title="+5 min" onClick={() => api('/api/timer', { seconds: 300 })}><Icon n="arrowUp" /></button>
        </div>
        <div className="tabs">
          <button className={ptab === 'pending' ? 'on' : ''} onClick={() => setPtab('pending')}>{t('pending')} {st.pending.length ? `(${st.pending.length})` : ''}</button>
          <button className={ptab === 'history' ? 'on' : ''} onClick={() => setPtab('history')}>{t('history')}</button>
        </div>
        {ptab === 'history' ? <>
          <div className="histats">
            <span>{t('bid')}: <b>{st.log.length}</b></span>
            <span>points: <b>{Math.round(st.log.filter((b) => b.source === 'twitch').reduce((a, b) => a + b.amount, 0))}</b></span>
            <span>donate: <b>{Math.round(st.log.filter((b) => b.source !== 'twitch').reduce((a, b) => a + b.amount, 0))}</b></span>
          </div>
          <div className="list">
            {st.log.length === 0 ? t('noHistory') : st.log.map((b) => (
              <div className="pitem" key={b.id}>
                <div className="prow2"><b>{b.username}</b><span>{b.source === 'twitch' ? 'points' : 'donate'} {Math.round(b.amount)}</span></div>
                <div className="muted">{b.lotTitle || b.userInput || '-'}</div>
              </div>
            ))}
          </div>
        </> : <div className="list">
          {st.pending.length === 0 ? t('noPending') : st.pending.map((b) => (
            <div className="pitem" key={b.id}>
              <div className="prow2"><b>{b.username}</b><span>{b.source === 'twitch' ? 'points' : 'donate'} {Math.round(b.amount)}</span></div>
              <div className="muted">{b.userInput || '-'}{b.lotTitle ? ` -> ${b.lotTitle}` : ''}</div>
              {st.settings.mode === 'manual' && <div className="row"><button className="mini ok" onClick={() => api('/api/pending/approve', { id: b.id })}><Icon n="check" size={14} /> {t('approve')}</button><button className="mini bad" onClick={() => api('/api/pending/decline', { id: b.id })}><Icon n="close" size={14} /> {t('decline')}</button></div>}
            </div>
          ))}
        </div>}
      </aside>}
    </div>
  );
}

function Auction({ st, t }: { st: St; t: (k: string) => string }) {
  const [name, setName] = useState(''); const [q, setQ] = useState('');
  const [adj, setAdj] = useState<Record<string, string>>({});
  const [compact, setCompact] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);
  const ap = st.settings.appearance;
  const showChances = ap.showWinningChances;
  const cur = st.settings.currency === 'USD' ? '$' : '\u20B4';
  const grand = st.lots.reduce((a, l) => a + l.total, 0);
  const setAp = (patch: Partial<Appearance>) => api('/api/settings', { appearance: patch });
  const add = () => { if (!name.trim()) return; api('/api/lots', { title: name, aliases: [] }); setName(''); };
  const adjust = (lotId: string, sign: number) => { const v = Number(adj[lotId]) || 0; if (!v) return; api('/api/lots/adjust', { lotId, delta: sign * v }); setAdj({ ...adj, [lotId]: '' }); };
  const save = async () => {
    const state = await api('/api/state'); const bids = await api('/api/bids');
    const data = { savedAt: Date.now(), settings: state.settings, lots: state.lots, bids, remainingMs: state.running && state.endsAt ? Math.max(0, state.endsAt - Date.now()) : state.remainingMs };
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = 'auction-' + new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-') + '.json'; a.click();
  };
  const upload = async (f: File) => { try { const data = JSON.parse(await f.text()); await api('/api/load', data); } catch {} };
  const lots = st.lots.filter((l) => !q || l.title.toLowerCase().includes(q.toLowerCase()));
  return (
    <>
      <div className="newrow">
        <input className="name" placeholder={t('newLot')} value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && add()} />
        <button className="btn-addlot" onClick={add}><Icon n="plus" size={18} /> {t('addLot')}</button>
        <div className="divider" />
        <div className="search"><Icon n="search" size={18} /><input placeholder={t('search')} value={q} onChange={(e) => setQ(e.target.value)} /></div>
      </div>
      <div className={'lots' + (compact ? ' compact' : '')}>
        {lots.length === 0 ? <div className="empty">{t('noLots')}</div> : lots.map((l) => (
          <div className={'lot' + (l.pinned ? ' pinned' : '')} key={l.id}>
            <div className="tag">#{l.number}</div>
            {showChances && <div className="chance">{grand ? ((l.total / grand) * 100).toFixed(1) : '0.0'}%</div>}
            <div className="nmcell">{ap.autoParseLinks && /^https?:\/\//i.test(l.title) ? <a className="nm" href={l.title} target="_blank" rel="noreferrer">{l.title}</a> : <input className="nm" value={l.title} readOnly />}</div>
            <div className="first">{ap.showViewerNames && l.firstBidder ? 'first: ' + l.firstBidder : ''}</div>
            <div className="amt">{ap.hideValues ? '' : `${Math.round(l.total)} ${cur}`}</div>
            <input className="adj" type="number" placeholder="0" value={adj[l.id] || ''} onChange={(e) => setAdj({ ...adj, [l.id]: e.target.value })} />
            <button className="ib" title={t('addCost')} onClick={() => adjust(l.id, 1)}><Icon n="plus" size={16} /></button>
            <button className="ib" title="Remove" onClick={() => adjust(l.id, -1)}><Icon n="down" size={16} /></button>
            <button className="ib" title={t('delSlot')} onClick={() => confirm('Delete lot?') && api('/api/lots/delete', { lotId: l.id })}><Icon n="trash" size={16} /></button>
            <button className={'ib heart' + (l.pinned ? ' on' : '')} title="Pin to top" onClick={() => api('/api/lots/pin', { lotId: l.id })}><Icon n="heart" size={16} /></button>
          </div>
        ))}
      </div>
      <input ref={fileRef} type="file" accept="application/json" style={{ display: 'none' }} onChange={(e) => { const f = e.target.files && e.target.files[0]; if (f) upload(f); e.target.value = ''; }} />
      <div className="bottom">
        {ap.showTotalAmount && <div className="grandtotal">{Math.round(grand)} {cur}</div>}
        <button className={'btn-outline' + (showChances ? ' on' : '')} onClick={() => setAp({ showWinningChances: !showChances })}><Icon n="percent" size={16} /> {t('showChances')}</button>
        <button className={'btn-outline' + (compact ? ' on' : '')} onClick={() => setCompact(!compact)}><Icon n="sort" size={16} /> {t('compact')}</button>
        <button className="btn-outline" onClick={save}><Icon n="layers" size={16} /> {t('save')}</button>
        <button className="btn-outline" onClick={() => fileRef.current && fileRef.current.click()}><Icon n="arrowUp" size={16} /> {t('upload')}</button>
        <button className="btn-new" onClick={() => { if (confirm('New auction?')) { api('/api/reset', {}); api('/api/timer/reset', {}); } }}><Icon n="plus" size={16} /> {t('newAuction')}</button>
      </div>
    </>
  );
}

function Wheel({ st, t }: { st: St; t: (k: string) => string }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const imgRef = useRef<HTMLImageElement | null>(null);
  const [img, setImg] = useState(() => localStorage.getItem('pa_wheel_img') || '');
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [emotes, setEmotes] = useState<{ name: string; url: string }[]>([]);
  const [mode, setMode] = useState('Normal');
  const [duration, setDuration] = useState(5);
  const [method, setMethod] = useState('normal');
  const [dividing, setDividing] = useState(1);
  const [grouping, setGrouping] = useState(2);
  const [music, setMusic] = useState('');
  const [musicName, setMusicName] = useState('');
  const [volume, setVolume] = useState(100);
  const [elim, setElim] = useState<Record<string, boolean>>({});
  const [bt, setBt] = useState<{ pts: Record<string, number>; rounds: { ids: string[]; winner: string | null }[][]; ri: number; mi: number; champ: string | null } | null>(null);
  const [brFull, setBrFull] = useState(false);
  const [brView, setBrView] = useState({ x: 8, y: 8, s: 0.7 });
  const [roll, setRoll] = useState<{ id: string; title: string; key: string } | null>(null);
  const [spinning, setSpinning] = useState(false);
  const spinLock = useRef(false);
  const [confirmed, setConfirmed] = useState<{ title: string; number: number; first: string; participants: number; chance: number } | null>(null);
  const rotRef = useRef(-Math.PI / 2);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const playMusic = () => { if (!music) return; try { if (!audioRef.current) audioRef.current = new Audio(); audioRef.current.volume = volume / 100; audioRef.current.loop = true; if (mode === 'Dropout' && !audioRef.current.paused) return; audioRef.current.src = music; audioRef.current.currentTime = 0; audioRef.current.play().catch(() => {}); } catch {} };
  const stopMusic = () => { try { if (audioRef.current) audioRef.current.pause(); } catch {} };
  useEffect(() => { if (audioRef.current) audioRef.current.volume = volume / 100; }, [volume]);
  const lots = st.lots.filter((l) => l.total > 0);
  const gStarts = (n: number, g: number) => { const m = Math.max(1, Math.ceil(n / g)); const base = Math.floor(n / m), rem = n % m; const st: number[] = []; let s = 0; for (let j = 0; j < m; j++) { st.push(s); s += base + (j < rem ? 1 : 0); } st.push(n); return st; };
  const gIndexOf = (i: number, n: number, g: number) => { const st = gStarts(n, g); for (let j = 0; j < st.length - 1; j++) if (i >= st[j] && i < st[j + 1]) return j; return 0; };
  const nextOpen = (rounds: { ids: string[]; winner: string | null }[][], r0: number, i0: number) => {
    let r = r0, i = i0;
    while (r < rounds.length) { if (i >= rounds[r].length) { r += 1; i = 0; continue; } const m = rounds[r][i]; if (m && !m.winner && m.ids.length >= 2) return { r, i }; i += 1; }
    return null;
  };
  type BtM = { ids: string[]; winner: string | null };
  const resolveByes = (rounds: BtM[][], g: number) => {
    let changed = true;
    while (changed) { changed = false;
      for (let r = 0; r < rounds.length - 1; r++) for (let i = 0; i < rounds[r].length; i++) {
        const m = rounds[r][i];
        if (m.winner || m.ids.length !== 1) continue;
        if (r > 0) { const st = gStarts(rounds[r - 1].length, g); const j = gIndexOf(i, rounds[r].length, g); const feeders = rounds[r - 1].slice(st[j], st[j + 1]); if (feeders.some((f) => !f.winner)) continue; }
        m.winner = m.ids[0]; const n = gIndexOf(i, rounds[r].length, g);
        if (rounds[r + 1][n] && !rounds[r + 1][n].ids.includes(m.ids[0])) rounds[r + 1][n].ids.push(m.ids[0]);
        changed = true;
      } }
  };
  const initBt = (g = grouping) => {
    const ranked = [...lots].sort((a, b) => b.total - a.total).map((l) => l.id);
    const pts: Record<string, number> = {}; for (const l of lots) pts[l.id] = l.total;
    const N = ranked.length;
    if (N < 2) { setBt(null); return; }
    const m0 = Math.max(1, Math.ceil(N / g));
    const groups: string[][] = Array.from({ length: m0 }, () => []);
    const sums = new Array(m0).fill(0);
    for (const id of ranked) { let bi = 0; for (let j = 1; j < m0; j++) if (groups[j].length < groups[bi].length || (groups[j].length === groups[bi].length && sums[j] < sums[bi])) bi = j; groups[bi].push(id); sums[bi] += pts[id] || 0; }
    const rounds: BtM[][] = [];
    rounds.push(groups.map((ids) => ({ ids, winner: null })));
    while (rounds[rounds.length - 1].length > 1) { const prev = rounds[rounds.length - 1]; const next: BtM[] = gStarts(prev.length, g).slice(0, -1).map(() => ({ ids: [], winner: null })); rounds.push(next); }
    resolveByes(rounds, g);
    const first = nextOpen(rounds, 0, 0);
    setBt({ pts, rounds, ri: first ? first.r : rounds.length - 1, mi: first ? first.i : 0, champ: first ? null : (rounds[rounds.length - 1][0]?.winner || null) });
  };
  const withPts = (id: string): Lot | null => { const l = st.lots.find((x) => x.id === id); return l ? { ...l, total: bt ? (bt.pts[id] ?? l.total) : l.total } : null; };
  const brOn = mode === 'Battle Royale' && !!bt;
  const lostIds = new Set<string>();
  if (bt) for (const rd of bt.rounds) for (const m of rd) if (m.winner) for (const id of m.ids) if (id !== m.winner) lostIds.add(id);
  const aliveIds = brOn ? bt!.rounds[0].flatMap((m) => m.ids).filter((id) => !lostIds.has(id)) : [];
  const listLots: Lot[] = brOn ? aliveIds.map(withPts).filter((x): x is Lot => !!x) : lots;
  const listGrand = listLots.reduce((a, l) => a + l.total, 0) || 1;
  const curMatch: string[] = brOn ? (bt!.rounds[bt!.ri][bt!.mi]?.ids || []) : [];
  const wlots: Lot[] = brOn ? curMatch.map(withPts).filter((x): x is Lot => !!x) : lots;
  const grand = wlots.reduce((a, l) => a + l.total, 0) || 1;
  const maxT = Math.max(1, ...wlots.map((l) => l.total));
  const size = maxT * dividing;
  const COLS = ['#e5484d', '#f76b15', '#ffb224', '#a3e635', '#22c55e', '#12b5a5', '#00b8d9', '#3b82f6', '#6366f1', '#a855f7', '#ec4899', '#b45309'];
  const lotColor = (id: string) => COLS[Math.max(0, lots.findIndex((l) => l.id === id)) % COLS.length];
  const all: { lot: Lot; key: string }[] = [];
  for (const l of wlots) { const n = Math.max(1, Math.round(l.total / size)); for (let i = 0; i < n; i++) all.push({ lot: l, key: l.id + ':' + i }); }
  const kept = all.filter((s) => !elim[s.key]);
  const lotFrac = (id: string) => { const t = all.filter((s) => s.lot.id === id).length || 1; const r = kept.filter((s) => s.lot.id === id).length; return r / t; };
  // Dropout chance = reverse of the Normal share: 1/total, normalised.
  const invSum = wlots.reduce((a, l) => a + 1 / Math.max(1, l.total), 0) || 1;
  const invPct = (l: Lot) => (1 / Math.max(1, l.total) / invSum) * 100;
  // Normal: slice ∝ bid (bigger bid = bigger slice). Dropout: slice ∝ elimination chance (smaller bid = bigger slice).
  const wOf = (l: Lot) => mode === 'Dropout' ? 1 / Math.max(1, l.total) : l.total;
  const remGrand = wlots.reduce((a, l) => a + wOf(l) * lotFrac(l.id), 0) || 1;
  const live: { lot: Lot; key: string; a0: number; ang: number }[] = [];
  { let a = -Math.PI / 2; for (const l of wlots) { const ks = kept.filter((s) => s.lot.id === l.id); if (!ks.length) continue; const arc = (wOf(l) * lotFrac(l.id) / remGrand) * Math.PI * 2; for (const k of ks) { live.push({ lot: l, key: k.key, a0: a, ang: arc / ks.length }); a += arc / ks.length; } } }
  { const sumA = live.reduce((a, s) => a + s.ang, 0) || 1; let a = -Math.PI / 2; for (const s of live) { s.ang = (s.ang / sumA) * Math.PI * 2; s.a0 = a; a += s.ang; } }
  const liveRef = useRef(live); liveRef.current = live;
  const cols = ['#17c4b8', '#74b6ef', '#006bb9', '#4da0e9', '#35d07f', '#ffb03a'];
  const draw = () => {
    const cv = ref.current; if (!cv) return; const ctx = cv.getContext('2d')!;
    const W = cv.width, H = cv.height, cx = W / 2, cy = H / 2, R = Math.min(W, H) / 2 - 26;
    ctx.clearRect(0, 0, W, H);
    ctx.save(); ctx.translate(cx, cy); ctx.rotate(rotRef.current);
    if (!live.length) { ctx.beginPath(); ctx.arc(0, 0, R, 0, Math.PI * 2); ctx.fillStyle = '#1b1b1b'; ctx.fill(); }
    for (const s of live) { ctx.beginPath(); ctx.moveTo(0, 0); ctx.arc(0, 0, R, s.a0, s.a0 + s.ang); ctx.closePath();
      ctx.fillStyle = lotColor(s.lot.id); ctx.fill(); ctx.strokeStyle = '#131313'; ctx.lineWidth = 1; ctx.stroke(); }
    for (const l of wlots) { const ss = live.filter((s) => s.lot.id === l.id); if (!ss.length) continue; const s0 = ss[0], sN = ss[ss.length - 1]; const mid = (s0.a0 + sN.a0 + sN.ang) / 2;
      ctx.save(); ctx.rotate(mid); ctx.textAlign = 'right'; ctx.fillStyle = '#08161a'; ctx.font = 'bold 16px Inter, sans-serif'; ctx.fillText(l.title.slice(0, 20), R - 18, 5); ctx.restore(); }
    ctx.restore();
    const cr = R * 0.16;
    ctx.beginPath(); ctx.arc(cx, cy, cr, 0, Math.PI * 2); ctx.strokeStyle = 'rgba(255,255,255,.4)'; ctx.lineWidth = 2; ctx.stroke();
    const im = imgRef.current;
    if (im) { ctx.save(); ctx.beginPath(); ctx.arc(cx, cy, cr, 0, Math.PI * 2); ctx.clip(); const s = Math.max((2 * cr) / im.width, (2 * cr) / im.height); const w = im.width * s, h = im.height * s; ctx.drawImage(im, cx - w / 2, cy - h / 2, w, h); ctx.restore(); }
    else { ctx.beginPath(); ctx.arc(cx, cy, cr, 0, Math.PI * 2); ctx.fillStyle = '#ffd23f'; ctx.fill(); }
    ctx.beginPath(); ctx.arc(cx, cy, cr, 0, Math.PI * 2); ctx.strokeStyle = 'rgba(0,0,0,.5)'; ctx.lineWidth = 3; ctx.stroke();
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, Math.PI * 2); ctx.strokeStyle = '#131313'; ctx.lineWidth = 5; ctx.stroke();
    ctx.beginPath(); ctx.moveTo(cx, cy - R + 6); ctx.lineTo(cx - 16, cy - R - 22); ctx.lineTo(cx + 16, cy - R - 22); ctx.closePath(); ctx.fillStyle = '#ff5c6c'; ctx.fill();
  };
  const animate = (start: number, target: number, ms: number) => {
    spinLock.current = true; setSpinning(true); const t0 = performance.now();
    const step = (now: number) => { const k = Math.min(1, (now - t0) / ms); const e = 1 - Math.pow(1 - k, 3);
      rotRef.current = start + (target - start) * e; draw();
      if (k < 1) requestAnimationFrame(step); else { rotRef.current = target; spinLock.current = false; setSpinning(false); } };
    requestAnimationFrame(step);
  };
  const spin = async () => {
    const lv = liveRef.current;
    if (spinLock.current || !lv.length) return;
    setRoll(null); setConfirmed(null);
    playMusic();
    const r = method === 'enhanced' ? crypto.getRandomValues(new Uint32Array(1))[0] / 4294967296 : Math.random();
    let seg = lv[lv.length - 1];
    const tot = lv.reduce((a, s) => a + s.ang, 0) || 1; let rr = r * tot;
    for (const s of lv) { rr -= s.ang; if (rr <= 0) { seg = s; break; } }
    const mid = seg.a0 + seg.ang / 2;
    const ms = Math.max(5000, Math.min(60000, duration * 1000));
    const start = rotRef.current;
    let target = -Math.PI / 2 - mid;
    while (target <= start + Math.PI * 2 * 3) target += Math.PI * 2;
    animate(start, target, ms);
    await api('/api/winner', { mode: 'wheel', method, lotId: seg.lot.id });
    setTimeout(() => {
      const rest = lv.filter((s) => s.key !== seg.key);
      if (mode === 'Dropout') {
        setElim((e) => ({ ...e, [seg.key]: true }));
        if (rest.length <= 1) { const w = rest[0] ? rest[0].lot : seg.lot; setRoll({ id: w.id, title: w.title, key: rest[0] ? rest[0].key : seg.key }); }
      } else setRoll({ id: seg.lot.id, title: seg.lot.title, key: seg.key });
    }, ms);
  };
  const advance = () => {
    if (!bt || mode !== 'Battle Royale') return;
    const match = bt.rounds[bt.ri][bt.mi];
    if (!match || match.ids.length < 2) return;
    const winId = roll ? roll.id : match.ids[0];
    const newPts = { ...bt.pts }; let gain = 0;
    for (const id of match.ids) if (id !== winId) { gain += bt.pts[id] || 0; newPts[id] = 0; }
    newPts[winId] = (newPts[winId] || 0) + gain;
    const rounds = bt.rounds.map((rd) => rd.map((m) => ({ ...m, ids: m.ids.slice() })));
    rounds[bt.ri][bt.mi].winner = winId;
    const nri = bt.ri + 1;
    if (nri < rounds.length) { const n = gIndexOf(bt.mi, bt.rounds[bt.ri].length, grouping); if (rounds[nri][n] && !rounds[nri][n].ids.includes(winId)) rounds[nri][n].ids.push(winId); }
    resolveByes(rounds, grouping);
    setRoll(null); stopMusic();
    const nx = nextOpen(rounds, bt.ri, bt.mi + 1);
    if (!nx) { const champ = winId; setBt({ pts: newPts, rounds, ri: bt.ri, mi: bt.mi, champ }); const l = st.lots.find((x) => x.id === champ); if (l) setConfirmed({ title: l.title, number: l.number, first: l.firstBidder || '-', participants: l.participants, chance: 100 }); return; }
    setBt({ pts: newPts, rounds, ri: nx.r, mi: nx.i, champ: null });
  };
  const pickMode = (m: string) => { setMode(m); setRoll(null); setConfirmed(null); setElim({}); if (m === 'Battle Royale') initBt(); else setBt(null); };
  useEffect(() => { if (!img) { imgRef.current = null; draw(); return; } const im = new Image(); im.onload = () => { imgRef.current = im; draw(); }; im.src = img; }, [img]);
  useEffect(() => { draw(); });
  const onCanvasClick = (e: React.MouseEvent) => { const cv = ref.current; if (!cv) return; const r = cv.getBoundingClientRect();
    const x = (e.clientX - r.left) * (cv.width / r.width), y = (e.clientY - r.top) * (cv.height / r.height);
    const cx = cv.width / 2, cy = cv.height / 2, R = Math.min(cv.width, cv.height) / 2 - 26;
    if (Math.hypot(x - cx, y - cy) < R * 0.16) setMenu({ x: e.clientX, y: e.clientY }); };
  const setImage = (url: string) => { setImg(url); try { localStorage.setItem('pa_wheel_img', url); } catch {} setMenu(null); };
  useEffect(() => { api('/api/emotes').then((d) => setEmotes(d.emotes || [])).catch(() => {}); }, []);
  const brDrag = useRef<{ x: number; y: number; vx: number; vy: number } | null>(null);
  const brDown = (e: React.MouseEvent) => { brDrag.current = { x: e.clientX, y: e.clientY, vx: brView.x, vy: brView.y }; };
  const brMove = (e: React.MouseEvent) => { if (!brDrag.current) return; const d = brDrag.current; setBrView((v) => ({ ...v, x: d.vx + (e.clientX - d.x), y: d.vy + (e.clientY - d.y) })); };
  const brUp = () => { brDrag.current = null; };
  const brWheel = (e: React.WheelEvent) => { e.preventDefault(); setBrView((v) => ({ ...v, s: Math.max(0.3, Math.min(2.5, v.s * (e.deltaY < 0 ? 1.12 : 0.89))) })); };
  const BOXW = 132, ROWH = 17, BPAD = 4, HGAP = 26, VGAP = 14;
  const brMaxG = bt ? Math.max(2, ...bt.rounds.flat().map((m) => m.ids.length)) : 2;
  const BRH = brMaxG * ROWH + BPAD * 2;
  const brR = bt ? bt.rounds.length : 0;
  const brCols = brR ? 2 * (brR - 1) + 1 : 0;
  const halfAt = (r: number) => Math.ceil((bt!.rounds[r].length) / 2);
  const cL: number[][] = [], cR: number[][] = [];
  if (bt) {
    const h0 = halfAt(0), n0 = bt.rounds[0].length;
    cL.push(Array.from({ length: h0 }, (_, i) => i * (BRH + VGAP) + BRH / 2));
    cR.push(Array.from({ length: n0 - h0 }, (_, i) => i * (BRH + VGAP) + BRH / 2));
    for (let r = 1; r <= brR - 2; r++) {
      const pL = cL[r - 1]; const h = halfAt(r); const n = bt.rounds[r].length;
      cL.push(Array.from({ length: h }, (_, j) => { const a = pL[2 * j] ?? pL[pL.length - 1] ?? 0; const b = pL[2 * j + 1] ?? a; return (a + b) / 2; }));
      const pR = cR[r - 1]; cR.push(Array.from({ length: n - h }, (_, j) => { const a = pR[2 * j] ?? pR[pR.length - 1] ?? 0; const b = pR[2 * j + 1] ?? a; return (a + b) / 2; }));
    }
  }
  const yFin = cL[brR - 2]?.[0] ?? BRH / 2;
  const brTW = brCols * BOXW + Math.max(0, brCols - 1) * HGAP + 20;
  const brTOP = bt?.champ ? BRH + VGAP : 0;
  const brTH = Math.max(BRH, (cL[0]?.length || 1) * (BRH + VGAP)) + 20 + brTOP;
  const cX = (c: number) => 10 + c * (BOXW + HGAP);
  const box = (key: string, x: number, y: number, m: { ids: string[]; winner: string | null }, now: boolean) => (
    <g key={key}>
      <rect x={x} y={y} width={BOXW} height={BRH} rx={4} fill="#2b2c30" stroke={now ? '#ff1717' : '#3a3c41'} strokeWidth={now ? 2 : 1} />
      {Array.from({ length: Math.max(1, brMaxG) }).map((_, k) => { const id = m.ids[k]; const l = id ? st.lots.find((z) => z.id === id) : null; const won = !!id && m.winner === id; const lost = !!id && !!m.winner && m.winner !== id; const ry = y + BPAD + k * ROWH;
        return <g key={k}>
          <rect x={x + 1} y={ry} width={BOXW - 2} height={ROWH} fill={won ? '#1f3d2a' : '#4a4b50'} />
          <rect x={x + BOXW - 32} y={ry} width={31} height={ROWH} fill="#787a80" />
          <text x={x + 6} y={ry + 12} fontSize={10} fill={id ? '#fff' : '#7a7c82'} style={{ textDecoration: lost ? 'line-through' : 'none' }}>{l?.title || (id ? '?' : '')}</text>
          {id && <text x={x + BOXW - 16} y={ry + 12} fontSize={10} fill="#23252d" textAnchor="middle">{Math.round(bt!.pts[id] || 0)}</text>}
        </g>; })}
    </g>
  );
  const bracketSvg = !bt ? null : (
    <svg width={brTW} height={brTH} style={{ display: 'block' }}>
      <g transform={`translate(0, ${brTOP})`}>
      {bt.champ && (() => { const l = st.lots.find((z) => z.id === bt.champ); const ty = yFin - BRH / 2 - BRH - 8; return <g>
        <rect x={cX(brR - 1)} y={ty} width={BOXW} height={ROWH + BPAD * 2} rx={4} fill="#1f3d2a" stroke="#22c55e" strokeWidth={2} />
        <text x={cX(brR - 1) + BOXW / 2} y={ty + ROWH} fontSize={11} fill="#7ee2a8" textAnchor="middle" fontWeight={700}>{`WINNER: ${l?.title || ''}`}</text>
      </g>; })()}
      {(() => { const els: any[] = [];
        for (let r = 0; r < brR - 1; r++) {
          const h = halfAt(r); const n = bt.rounds[r].length;
          for (let i = 0; i < h; i++) { const j = i >> 1; const x1 = cX(r) + BOXW, y1 = cL[r][i]; const x2 = cX(r + 1), y2 = cL[r + 1]?.[j] ?? yFin; const mx = (x1 + x2) / 2; els.push(<path key={`cl${r}-${i}`} d={`M${x1} ${y1} H${mx} V${y2} H${x2}`} fill="none" stroke="#44464b" strokeWidth={2} />); }
          for (let i = h; i < n; i++) { const j = (i - h) >> 1; const x1 = cX(brCols - 1 - r), y1 = cR[r][i - h]; const x2 = cX(brCols - 2 - r) + BOXW, y2 = cR[r + 1]?.[j] ?? yFin; const mx = (x1 + x2) / 2; els.push(<path key={`cr${r}-${i}`} d={`M${x1} ${y1} H${mx} V${y2} H${x2}`} fill="none" stroke="#44464b" strokeWidth={2} />); }
        }
        return els; })()}
      {Array.from({ length: Math.max(0, brR - 1) }).map((_, r) => Array.from({ length: halfAt(r) }).map((_, i) => box(`L${r}-${i}`, cX(r), cL[r][i] - BRH / 2, bt.rounds[r][i], r === bt.ri && i === bt.mi && !bt.champ)))}
      {Array.from({ length: Math.max(0, brR - 1) }).map((_, r) => Array.from({ length: bt.rounds[r].length - halfAt(r) }).map((_, k) => { const i = halfAt(r) + k; return box(`R${r}-${i}`, cX(brCols - 1 - r), cR[r][k] - BRH / 2, bt.rounds[r][i], r === bt.ri && i === bt.mi && !bt.champ); }))}
      {brR > 0 && box('F', cX(brR - 1), yFin - BRH / 2, bt.rounds[brR - 1][0], bt.ri === brR - 1 && !bt.champ)}
      </g>
    </svg>
  );
  return <div className="wheelgrid">
    <div className="wheelcol">
      <div className="setpanel"><div className="in">
        {listLots.length === 0 ? <div className="muted">{t('noLots')}</div> : listLots.map((l) => (
          <div className="wrow" key={l.id} style={{ background: `linear-gradient(90deg, ${lotColor(l.id)}26 ${(lotFrac(l.id) * 100).toFixed(0)}%, #3a3a3a44 ${(lotFrac(l.id) * 100).toFixed(0)}%)` }}>
            <span className="dot" style={{ background: lotColor(l.id) }} />
            <div className="pnm">{l.title}{l.firstBidder ? <div className="first" title={l.firstBidder} style={{ fontSize: l.firstBidder.length > 20 ? 9 : l.firstBidder.length > 14 ? 10 : 11 }}>{l.firstBidder}</div> : null}</div>
            <div className="ptot">{Math.round(l.total)}</div>
            <div className="pch">{(mode === 'Dropout' ? invPct(l) : (l.total / listGrand) * 100).toFixed(1)}%</div>
          </div>
        ))}
      </div></div>
    </div>
    <div className="wheelstage">
      {mode === 'Battle Royale' && <div style={{ marginBottom: 6, color: 'var(--muted)', fontSize: 13, fontWeight: 600 }}>{bt?.champ ? `Champion: ${st.lots.find((x) => x.id === bt.champ)?.title || ''}` : bt ? `Round ${bt.ri + 1} · Match ${bt.mi + 1}/${bt.rounds[bt.ri].length}` : 'Add 2+ lots to start'}</div>}
      <canvas ref={ref} width={760} height={760} onClick={onCanvasClick} style={{ maxWidth: '100%', maxHeight: '88vh', cursor: 'pointer' }} />
      {menu && <div className="wmenu" style={{ left: menu.x, top: menu.y }}>
        <button className="btn-outline" onClick={() => { setMenu(null); fileRef.current && fileRef.current.click(); }}><Icon n="image" size={16} /> Choose image from PC</button>
        <div className="wgrid">{emotes.length ? emotes.map((em) => <img key={em.name} src={em.url} title={em.name} onClick={() => setImage(em.url)} />) : <span className="muted">no emotes connected</span>}</div>
      </div>}
      <input ref={fileRef} type="file" accept="image/*" style={{ display: 'none' }} onChange={(e) => { const f = e.target.files && e.target.files[0]; if (!f) return; const rd = new FileReader(); rd.onload = () => setImage(String(rd.result)); rd.readAsDataURL(f); e.target.value = ''; }} />
      <div className="row" style={{ justifyContent: 'center' }}><button className="btn-spin" onClick={spin} disabled={spinning}>Spin</button></div>
      {roll && <div className="wfloat">
        <button className="wx" onClick={() => { stopMusic(); setRoll(null); }}><Icon n="close" size={14} /></button>
        <div className="winnerbox">Winner: <b>{roll.title}</b></div>
        <button className="btn-spin" onClick={() => { if (mode === 'Battle Royale') { advance(); } else { stopMusic(); const l = st.lots.find((x) => x.id === roll.id); if (l) setConfirmed({ title: l.title, number: l.number, first: l.firstBidder || '-', participants: l.participants, chance: mode === 'Dropout' ? invPct(l) : (l.total / grand) * 100 }); setRoll(null); } }}>Confirm winner</button>
        <button className="btn-outline" onClick={() => { if (mode === 'Dropout') { setElim({}); setRoll(null); stopMusic(); } else { setRoll(null); setTimeout(spin, 80); } }}>Reroll</button>
        <button className="btn-outline" onClick={() => { api('/api/lots/delete', { lotId: roll.id }); setRoll(null); }}>Delete lot</button>
      </div>}
      {confirmed && <div className="wfloat">
        <div className="winnerbox">WINNER: <b>{confirmed.title}</b></div>
        <div className="wmeta">Bid #{confirmed.number} &middot; winning chance {confirmed.chance.toFixed(1)}%</div>
        <div className="wmeta">first bid by <b>{confirmed.first}</b> &middot; {confirmed.participants} bidders</div>
        <button className="btn-spin" onClick={() => setConfirmed(null)}>Close</button>
      </div>}
    </div>
    <div className="setbody wset">
      <Panel title={t('wheel')}>
        <div className="field"><label>Spin time (sec)</label><input type="number" min={5} max={60} value={duration} onChange={(e) => setDuration(Math.max(5, Math.min(60, Number(e.target.value) || 5)))} /></div>
        <div className="stack"><label>Roll sound</label>
          <input placeholder="audio / youtube link" value={musicName} onChange={(e) => { setMusicName(e.target.value); setMusic(e.target.value); }} />
          <button className="btn-outline" onClick={() => { const f = document.createElement('input'); f.type = 'file'; f.accept = 'audio/*,video/*'; f.onchange = () => { const fl = f.files && f.files[0]; if (fl) { setMusic(URL.createObjectURL(fl)); setMusicName('Local Audio'); } }; f.click(); }}>Upload from PC</button>
          {music && <div className="row"><Icon n="music" size={15} /><input type="range" min={0} max={100} value={volume} onChange={(e) => setVolume(Number(e.target.value))} style={{ flex: 1 }} /><span className="muted">{volume}</span></div>}
        </div>
        <div className="field"><label>Mode</label><div className="seg">{['Normal', 'Dropout', 'Battle Royale'].map((m) => <button key={m} className={mode === m ? 'on' : ''} onClick={() => pickMode(m)}>{m}</button>)}</div></div>
        {mode === 'Battle Royale' && <div className="field"><label>Nesting ({grouping})</label><input type="range" value={grouping} min={2} max={8} step={1} onChange={(e) => { const n = Number(e.target.value); setGrouping(n); initBt(n); }} /></div>}
        <div className="field"><label>Dividing ({dividing})</label><input type="range" value={dividing} min={0.1} max={1} step={0.1} onChange={(e) => setDividing(Number(e.target.value))} /></div>
        <div className="field"><label>Random method</label><div className="seg">{['normal', 'enhanced'].map((m) => <button key={m} className={method === m ? 'on' : ''} onClick={() => setMethod(m)}>{m === 'normal' ? 'Normal' : 'Enhanced'}</button>)}</div></div>
      </Panel>
      {mode === 'Battle Royale' && bt && <div className={'bracket-wrapper' + (brFull ? ' full' : '')}>
        <div className="bracket-controls">
          <button className="btn-outline" onClick={() => setBrFull(!brFull)}>{brFull ? 'Exit full screen' : 'Full screen'}</button>
          <button className="btn-outline" onClick={() => setBrView({ x: 8, y: 8, s: 0.7 })}>Current game</button>
          <span className="muted">drag / scroll</span>
        </div>
        <div className="bracket-view" onMouseDown={brDown} onMouseMove={brMove} onMouseUp={brUp} onMouseLeave={brUp} onWheel={brWheel}>
          <div style={{ transform: `translate(${brView.x}px, ${brView.y}px) scale(${brView.s})`, transformOrigin: '0 0' }}>{bracketSvg}</div>
        </div>
      </div>}
    </div>
  </div>;
}

const Panel = ({ title, children }: any) => <div className="setpanel"><h3>{title}</h3><div className="in">{children}</div></div>;
const Field = ({ label, type = 'text', def = '', opts }: any) => (<div className="field">{label ? <label>{label}</label> : <span />}{opts ? <select defaultValue={def}>{opts.map((o: string) => <option key={o}>{o}</option>)}</select> : <input type={type} defaultValue={def} />}</div>);
const Check = ({ label }: any) => <label className="check"><input type="checkbox" /> {label}</label>;
const Seg = ({ value, set, opts, t }: any) => <div className="seg">{opts.map((o: [string, string]) => <button key={o[0]} className={value === o[0] ? 'on' : ''} onClick={() => set(o[0])}>{t(o[1])}</button>)}</div>;

function Settings({ st, t }: { st: St; t: (k: string) => string }) {
  const s = st.settings;
  const tc = s.timer;
  const setS = (patch: any) => api('/api/settings', patch);
  const setT = (patch: Partial<TimerCfg>) => api('/api/settings', { timer: patch });
  const ig = st.integrations || { monobank: { jarId: '', account: '', pollSeconds: 15, connected: false, hasToken: false }, twitch: { clientId: '', rewardTitle: 'Bid', rewardCost: 100, login: '', connected: false, hasSecret: false, authorized: false } };
  const [igTok, setIgTok] = useState({ mono: '', tw: '' });
  const setIg = (patch: any) => api('/api/integrations', patch);
  const Num = ({ label, k }: { label: string; k: keyof TimerCfg }) => <div className="field"><label>{label}</label><input type="number" min={0} value={Number(tc[k]) || 0} onChange={(e) => setT({ [k]: Number(e.target.value) || 0 } as Partial<TimerCfg>)} /></div>;
  return <>
    <div className="setbody">
      <Panel title="Twitch">
        <div className="field"><label>Client ID</label><input value={ig.twitch.clientId} onChange={(e) => setIg({ twitch: { clientId: e.target.value } })} /></div>
        <div className="field"><label>Client Secret</label><input type="password" value={igTok.tw} placeholder={ig.twitch.hasSecret ? '(saved)' : 'paste secret'} onChange={(e) => setIgTok({ ...igTok, tw: e.target.value })} /></div>
        <div className="field"><label>Reward title</label><input value={ig.twitch.rewardTitle} onChange={(e) => setIg({ twitch: { rewardTitle: e.target.value } })} /></div>
        <div className="field"><label>Reward cost</label><input type="number" min={1} value={ig.twitch.rewardCost} onChange={(e) => setIg({ twitch: { rewardCost: Number(e.target.value) || 1 } })} /></div>
        <div className="field"><label>Status</label><div className="muted">{ig.twitch.connected ? 'connected' : ig.twitch.authorized ? 'authorized' : 'not connected'}{ig.twitch.login ? ` · ${ig.twitch.login}` : ''}</div></div>
        <div className="row"><button className="btn-outline" onClick={() => setIg({ twitch: { clientSecret: igTok.tw } })}>Save secret</button><a className="btn-outline" href="/twitch/login" target="_blank" rel="noreferrer">Connect Twitch</a></div>
      </Panel>
      <Panel title="Monobank">
        <div className="field"><label>Jar ID or link</label><input value={ig.monobank.jarId} placeholder="https://send.monobank.ua/jar/xxxx" onChange={(e) => setIg({ monobank: { jarId: e.target.value } })} /></div>
        <div className="field"><label>Token (X-Token)</label><input type="password" value={igTok.mono} placeholder={ig.monobank.hasToken ? '(saved)' : 'paste token'} onChange={(e) => setIgTok({ ...igTok, mono: e.target.value })} /></div>
        <div className="field"><label>Poll seconds</label><input type="number" min={5} value={ig.monobank.pollSeconds} onChange={(e) => setIg({ monobank: { pollSeconds: Number(e.target.value) || 15 } })} /></div>
        <div className="field"><label>Status</label><div className="muted">{ig.monobank.connected ? 'connected' : ig.monobank.account ? 'idle' : 'not configured'}{ig.monobank.account ? ` · ${ig.monobank.account}` : ''}</div></div>
        <div className="row"><button className="btn-outline" onClick={() => setIg({ monobank: { token: igTok.mono } })}>Save token</button></div>
      </Panel>
      <Panel title={t('timer')}>
        <div className="field"><label>Initial Time (min)</label><input type="number" min={0} value={Math.round((s.timerSeconds || 0) / 60)} onChange={(e) => setS({ timerSeconds: (Number(e.target.value) || 0) * 60 })} /></div>
        <label className="switch"><input type="checkbox" checked={!!tc.showTotalTime} onChange={(e) => setT({ showTotalTime: e.target.checked })} /><i /><span>Show total auction time</span></label>
        <h4>Add time on conditions</h4>
        <div className="subhead">Leader Change</div>
        <Num label="Points (sec)" k="leaderChangePoints" />
        <Num label="Donations (sec)" k="leaderChangeDonations" />
        <div className="subhead">New Position</div>
        <Num label="Points (sec)" k="newPositionPoints" />
        <Num label="Donations (sec)" k="newPositionDonations" />
        <Num label="Add points to already existing bid (sec)" k="addExistingPoints" />
        <Num label="Add donation to already existing bid (sec)" k="addExistingDonations" />
        <h4>Exceptions</h4>
        <Num label="Time is more than (min)" k="timeMoreThanMin" />
      </Panel>
      <Panel title={t('bids')}>
        <div className="field"><label>Mode</label><Seg value={s.mode} set={(v: string) => setS({ mode: v })} opts={[['manual', 'manual'], ['auto', 'auto']]} t={t} /></div>
        <div className="field"><label>Source</label><Seg value={s.source} set={(v: string) => setS({ source: v })} opts={[['points', 'points'], ['donations', 'donations']]} t={t} /></div>
        <div className="field"><label>Currency</label><Seg value={s.currency} set={(v: string) => setS({ currency: v })} opts={[['UAH', 'UAH'], ['USD', 'USD']]} t={(x: string) => x} /></div>
      </Panel>
    </div>
  </>;
}

