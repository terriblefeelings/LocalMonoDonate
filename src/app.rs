use crate::model::*;
use crate::mono;
use serde_json::json;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// Bits bought on the sound-alerts page carry their nickname in the payment comment
/// ("... <code> | name: <nick>"). Every bit is counted as "<nick> SA", or "Anonymous SA"
/// when no nickname was given.
fn bit_display_name(comment: &str) -> String {
    if let Some(pos) = comment.find("name:") {
        let nick = comment[pos + 5..].split('|').next().unwrap_or("").trim();
        if !nick.is_empty() { return format!("{} SA", nick); }
    }
    "Anonymous SA".to_string()
}

#[cfg(test)]
mod bit_name_tests {
    use super::*;

    #[test]
    fn bits_use_the_nickname_from_the_comment_with_sa_suffix() {
        assert_eq!(bit_display_name("донат сповіщення abc123 | name: pendra"), "pendra SA");
        assert_eq!(bit_display_name("донат сповіщення abc123"), "Anonymous SA");
        assert_eq!(bit_display_name("| name:   "), "Anonymous SA");
    }
}

pub struct Ctx {
    pub cfg: Mutex<Config>,
    pub state: Mutex<State>,
    pub events: Mutex<(u64, Vec<(u64, String)>)>, // (next id, recent events)
    pub watch_until: Mutex<i64>,
    pub watch_start: Mutex<i64>,
    pub cv: Condvar,
    pub paths: Paths,
}

impl Ctx {
    pub fn new(cfg: Config, paths: Paths) -> Arc<Ctx> {
        let mut state = load_state(&paths, &cfg.jars);
        // freeze a running countdown on startup — it only resumes on a manual Start
        if state.timer.running { state.timer.freeze(); write_state(&paths, &state); }
        Arc::new(Ctx {
            cfg: Mutex::new(cfg),
            state: Mutex::new(state),
            events: Mutex::new((1, Vec::new())),
            watch_until: Mutex::new(0),
            watch_start: Mutex::new(0),
            cv: Condvar::new(),
            paths,
        })
    }

    pub fn cfg(&self) -> Config { self.cfg.lock().unwrap().clone() }

    pub fn save(&self) {
        let mut st = self.state.lock().unwrap();
        // keep a fresh snapshot of the countdown so a close/reboot freezes at the right time
        if st.timer.running { let rem = st.timer.remaining(); st.timer.remaining_ms = rem; }
        write_state(&self.paths, &st);
    }

    pub fn broadcast(&self, event: serde_json::Value) {
        let frame = event.to_string();
        let mut ev = self.events.lock().unwrap();
        let id = ev.0;
        ev.0 += 1;
        ev.1.push((id, frame));
        if ev.1.len() > 100 { let n = ev.1.len() - 100; ev.1.drain(0..n); }
    }

    pub fn events_since(&self, since: u64) -> (Vec<String>, u64) {
        let ev = self.events.lock().unwrap();
        let items = ev.1.iter().filter(|(id, _)| *id >= since).map(|(_, s)| s.clone()).collect();
        (items, ev.0)
    }

    pub fn watch(&self, ms: i64) {
        let now = now_ms();
        *self.watch_start.lock().unwrap() = now;
        *self.watch_until.lock().unwrap() = now + ms;
        self.cv.notify_all();
    }
    pub fn stop_watch(&self) {
        *self.watch_until.lock().unwrap() = 0;
        self.cv.notify_all();
    }

    /// reflect config.jars in state (add/remove) after the user edits the setup page
    pub fn sync_jars(&self) {
        let codes = self.cfg().jars;
        let mut st = self.state.lock().unwrap();
        let mut out: Vec<JarState> = Vec::new();
        for c in &codes {
            if let Some(pos) = st.jars.iter().position(|j| j.code == *c) { out.push(st.jars.remove(pos)); }
            else { out.push(JarState { code: c.clone(), ..Default::default() }); }
        }
        st.jars = out;
        drop(st);
        self.save();
    }

    /// resolve title/balance/goal + internal account id for every configured jar
    pub fn refresh_jars(&self) -> Result<(), String> {
        let cfg = self.cfg();
        if cfg.token.is_empty() { return Ok(()); }
        let info = mono::client_info(&cfg.token)?;
        let mut st = self.state.lock().unwrap();
        for (i, j) in st.jars.iter_mut().enumerate() {
            // 3rd-party jar: never query the API for it (no account id => never polled)
            if !cfg.jar_mine.get(i).copied().unwrap_or(true) { continue; }
            if let Some(ji) = info.jars.iter().find(|x| {
                x.id == j.code
                    || x.send_id.as_deref().map(mono::short_send_id) == Some(j.code.clone())
            }) {
                j.title = ji.title.clone().unwrap_or_default();
                j.balance = ji.balance.unwrap_or(0);
                j.goal = ji.goal.unwrap_or(0);
                j.statement_account = ji.id.clone();
                j.currency = ji.currency_code.unwrap_or(980);
                j.updated_at = now_ms();
            }
        }
        drop(st);
        self.save();
        Ok(())
    }

    /// returns true when this was the live donation a form submission was waiting for
    fn handle_incoming(&self, i: usize, tx: &mono::StatementItem, quiet: bool) -> bool {
        let cfg = self.cfg();
        let uah = tx.amount as f64 / 100.0;
        let comment = [tx.comment.clone().unwrap_or_default(), tx.description.clone().unwrap_or_default()]
            .into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
        let low = comment.to_lowercase();

        let mut st = self.state.lock().unwrap();
        let jar = &mut st.jars[i];
        let jar_code = jar.code.clone();
        let cur = crate::model::currency_symbol(jar.currency);
        let iso = crate::model::currency_iso(jar.currency);

        // bits have their own codes: a donation whose comment contains a bit code plays on the bits overlay
        if let Some(bit) = cfg.bits.iter().find(|b| !b.code.is_empty() && low.contains(&b.code.to_lowercase())).cloned() {
            let name = bit_display_name(&comment);
            let msg: String = comment.chars().take(cfg.limits.message_max).collect();
            jar.history.insert(0, HistoryItem {
                id: tx.id.clone(), name: name.clone(), amount: uah, message: msg.clone(),
                youtube_id: String::new(), matched: "bit".into(), code: Some(bit.code.clone()), at: now_ms(),
            });
            jar.stats.count += 1; jar.stats.total += uah;
            if uah > jar.stats.biggest { jar.stats.biggest = uah; }
            drop(st);
            if !quiet {
                self.broadcast(json!({ "type": "bit", "data": {
                    "id": tx.id, "name": name, "amount": uah, "message": msg, "currency": cur, "bit": bit.clone() } }));
                log(&format!("[{}] bit '{}' ({}) {:.0} from {}", jar_code, bit.label, bit.kind, uah, name));
                crate::streamelements::spawn_tip(cfg.port, cfg.clone(), name.clone(), uah, iso.clone(), msg.clone());
            }
            self.timer_add_donation(uah, &name, quiet);
            if !quiet { self.auction_bid(&name, uah, "monobank"); }
            self.save();
            return false;
        }

        // match a pending donation by code first, else by amount + recency
        let mut idx = jar.pending.iter().position(|p| low.contains(&p.code));
        let mut matched = "code";
        if idx.is_none() {
            let now = now_secs();
            idx = jar.pending.iter().position(|p| {
                (p.amount - uah).abs() <= 1.0f64.max(p.amount * 0.02) && (now - p.created_at / 1000) < 3600
            });
            if idx.is_some() { matched = "amount"; }
        }

        // resolve the alert payload (if any) plus the local history record
        let (name, message, youtube_id, kind, code, alert, voice) = if let Some(k) = idx {
            let p = jar.pending.remove(k);
            // the fallback-link twin is the same intent, never a second alert
            if !p.pair.is_empty() { jar.pending.retain(|x| x.code != p.pair); }
            if let Some(bit) = p.bit.clone() {
                // bit picked on the donation form: play it on the bits overlay (then YouTube)
                let n = if p.name.trim().is_empty() { "Anonymous SA".to_string() } else { format!("{} SA", p.name.trim()) };
                let m = if p.message.trim().is_empty() { bit.label.clone() } else { p.message.clone() };
                let yt_min = cfg.notify.rules.iter().find(|r| r.id == "default").map(|r| r.style.yt_min).unwrap_or(0.0);
                let yt = if !p.youtube_id.is_empty() && uah >= yt_min { p.youtube_id.clone() } else { String::new() };
                jar.history.insert(0, HistoryItem {
                    id: tx.id.clone(), name: n.clone(), amount: uah, message: m.clone(),
                    youtube_id: yt.clone(), matched: "bit".into(), code: Some(bit.code.clone()), at: now_ms(),
                });
                jar.stats.count += 1; jar.stats.total += uah;
                if uah > jar.stats.biggest { jar.stats.biggest = uah; }
                drop(st);
                if !quiet {
                    self.broadcast(json!({ "type": "bit", "data": {
                        "id": tx.id, "name": n.clone(), "amount": uah, "message": m.clone(),
                        "currency": cur, "bit": bit, "youtubeId": yt } }));
                    log(&format!("[{}] form bit {:.0} from {}", jar_code, uah, n));
                    crate::streamelements::spawn_tip(cfg.port, cfg.clone(), n.clone(), uah, iso.clone(), m.clone());
                }
                self.timer_add_donation(uah, &n, quiet);
                if !quiet { self.auction_bid(&n, uah, "monobank"); }
                self.save();
                return false;
            }
            let pname = if p.name.trim().is_empty() { "Anonymous".to_string() } else { p.name.trim().to_string() };
            (pname, p.message, p.youtube_id, matched, Some(p.code), true, p.voice)
        } else {
            let name = "Anonymous".to_string();
            let msg: String = comment.chars().take(cfg.limits.message_max).collect();
            let v = String::new();
            if cfg.notify_unmatched { (name, msg, String::new(), "unmatched", None, true, v) }
            else { (name, msg, String::new(), "none", None, false, v) }
        };

        // every positive transaction is kept in the local history + stats, whether or not it alerts
        jar.history.insert(0, HistoryItem {
            id: tx.id.clone(), name: name.clone(), amount: uah, message: message.clone(),
            youtube_id: youtube_id.clone(), matched: kind.into(), code: code.clone(), at: now_ms(),
        });
        jar.stats.count += 1;
        jar.stats.total += uah;
        if uah > jar.stats.biggest { jar.stats.biggest = uah; }
        let no_pending = jar.pending.is_empty();
        drop(st);

        self.timer_add_donation(uah, &name, quiet);
        if !quiet { self.auction_bid(&name, uah, "monobank"); }
        if alert && !quiet {
            // synthesize the voice FIRST so the alert and the audio appear together
            let mut tts_url = String::new();
            if cfg.tts.enabled() {
                let money = format!("{:.0} {}", uah, cur);
                let money_text = pick_style(&cfg.notify.rules, uah).money_text;
                let spoken = crate::tts::spoken_text(&cfg.tts.read_mode, &name, &money, &message, &money_text);
                let allowed = cfg.tts.allowed_in(&cfg.tts.dir);
                let eff_voice = if cfg.tts.donor_picks() && !voice.is_empty() && allowed.contains(&voice) {
                    voice.clone()
                } else {
                    let d = cfg.tts.default_voice();
                    if d.is_empty() { allowed.first().cloned().unwrap_or_default() } else { d }
                };
                if !spoken.trim().is_empty() && !eff_voice.is_empty() {
                    let dir = self.paths.base.join("tts");
                    let _ = std::fs::create_dir_all(&dir);
                    let fname = format!("{}.wav", tx.id.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect::<String>());
                    if crate::tts::synth(&cfg.tts.dir, &eff_voice, &spoken, &dir.join(&fname), &cfg.tts.azure_key, &cfg.tts.azure_region).is_ok() {
                        tts_url = format!("/tts/{}", fname);
                    } else {
                        log("tts synth failed");
                    }
                }
            }
            self.broadcast(json!({ "type": "donation", "data": {
                "id": tx.id, "name": name, "amount": uah, "message": message,
                "youtubeId": youtube_id, "matched": kind, "jar": jar_code, "currency": cur,
                "ttsUrl": tts_url } }));
            log(&format!("[{}] notification ({}) {:.0} UAH from {}", jar_code, kind, uah, name));
            // StreamElements: mirror the live donation as a tip so the SE alert box plays
            crate::streamelements::spawn_tip(cfg.port, cfg.clone(), name.clone(), uah, iso.clone(), message.clone());
        }
        self.save();
        alert && !quiet && no_pending && (kind == "code" || kind == "amount")
    }

    /// add marathon time for a jar donation (proportional: amount / per_amount * per_seconds)
    fn timer_add_donation(&self, amount: f64, name: &str, quiet: bool) {
        if quiet { return; } // historical backfill must not top up the live timer
        let cfg = self.cfg();
        if !cfg.features.timer || cfg.timer.per_amount <= 0.0 { return; }
        if amount < cfg.timer.min_donation { return; }
        let secs = (amount / cfg.timer.per_amount * cfg.timer.per_seconds).round() as i64;
        if secs <= 0 { return; }
        {
            let mut st = self.state.lock().unwrap();
            st.timer.add(secs * 1000);
            st.timer.money += amount;
            st.timer.push_log(TimerEvent { source: "monobank".into(), label: name.to_string(), amount, seconds: secs, at: now_ms() });
        }
        log(&format!("timer: +{}s from {} ({:.0})", secs, name, amount));
    }

    /// add a bid to the active auction (channel-point redemption or donation)
    pub fn auction_bid(&self, name: &str, amount: f64, source: &str) { self.auction_bid_full(name, amount, source, "", ""); }
    /// same, with the viewer input (lot) + redemption id for the merged AUC
    pub fn auction_bid_full(&self, name: &str, amount: f64, source: &str, user_input: &str, redemption_id: &str) {
        let cfg = self.cfg();
        let active = { let st = self.state.lock().unwrap(); st.auction.active };
        if amount < cfg.auction.min_bid { return; }
        {
            let mut st = self.state.lock().unwrap();
            if active { st.auction.add_bid(name, amount, source); } // legacy model (kept until removed)
            let _ = st.auc.intake(name, amount, user_input, source, redemption_id); // merged AUC
        }
        log(&format!("auction: bid {:.0} from {} ({})", amount, name, source));
        self.save();
    }

    /// returns true when every jar poll succeeded (or was legitimately skipped)
    pub fn poll_once(&self) -> bool {        let cfg = self.cfg();
        if cfg.token.is_empty() { return true; }
        let to = now_secs();

        let jobs: Vec<(usize, String, i64, bool)> = {
            let st = self.state.lock().unwrap();
            st.jars.iter().enumerate().filter_map(|(i, j)| {
                if j.statement_account.is_empty() { None }
                else {
                    // first poll for a jar pulls the last 30 days so older donations reach the local history;
                    // those are recorded silently (no overlay) — only live donations alert
                    let quiet = !j.backfilled;
                    let from = if quiet { to - 30 * 86_400 }
                               else if j.last_ts > 0 { j.last_ts - 5 }
                               else { to - 600 };
                    Some((i, j.statement_account.clone(), from, quiet))
                }
            }).collect()
        };

        let mut ok = true;
        for (i, acct, from, quiet) in jobs {
            if to <= from { continue; }
            match mono::statement(&acct, &cfg.token, from, to) {
                Ok(items) => {
                    let mut fresh: Vec<mono::StatementItem> = items.into_iter()
                        .filter(|t| !self.state.lock().unwrap().jars[i].seen.contains(&t.id))
                        .collect();
                    fresh.sort_by_key(|t| t.time);
                    let mut fetched = false;
                    for tx in fresh {
                        self.state.lock().unwrap().jars[i].seen.push(tx.id.clone());
                        if tx.amount > 0 && self.handle_incoming(i, &tx, quiet) { fetched = true; }
                    }
                    // the QR/stat widgets need the post-donation jar balance: refresh now and
                    // only then stop the 10-minute watch instead of waiting for the idle interval
                    if fetched && self.refresh_jars().is_ok() { self.stop_watch(); }
                    let ttl = now_ms() - (cfg.pending_ttl_hours as i64) * 3_600_000;
                    {
                        let mut st = self.state.lock().unwrap();
                        let j = &mut st.jars[i];
                        j.last_ts = to;
                        j.backfilled = true;
                        while j.seen.len() > 2000 { j.seen.remove(0); }
                        j.pending.retain(|p| p.created_at >= ttl);
                    }
                    self.save();
                }
                Err(e) => {
                    let code = self.state.lock().unwrap().jars[i].code.clone();
                    log(&format!("statement poll failed [{}]: {}", code, e));
                    ok = false;
                }
            }
        }
        ok
    }
}

pub fn run_poller(ctx: Arc<Ctx>) {
    let mut last_jar = 0i64;
    loop {
        let token = !ctx.cfg().token.is_empty();
        let unresolved = ctx.state.lock().unwrap().jars.iter()
            .any(|j| j.statement_account.is_empty());
        let watching = now_ms() < *ctx.watch_until.lock().unwrap();
        // after a form submit, refresh jar title/balance/goal every 60 s until the
        // donation is fetched (stop_watch) or the 10-minute watch expires
        let jar_interval: i64 = if token && (unresolved || watching) { 60_000 }
            else { (ctx.cfg().idle_poll_seconds.max(60) * 1000) as i64 };
        if now_ms() - last_jar > jar_interval {
            if let Err(e) = ctx.refresh_jars() { log(&format!("client-info failed: {}", e)); }
            last_jar = now_ms();
        }

        let ok = ctx.poll_once();
        let cfg = ctx.cfg();

        // 20 s for the first 5 min after a donation is expected, then 60 s for 10 min, then quiet.
        let wait = if !ok {
            60_000
        } else {
            let now = now_ms();
            let wu = *ctx.watch_until.lock().unwrap();
            let ws = *ctx.watch_start.lock().unwrap();
            let secs = if now < wu {
                if now - ws < 5 * 60_000 { cfg.poll_seconds.clamp(15, 600) } else { 60 }
            } else if cfg.features.timer {
                // marathon mode: poll at the user-set interval (never the fast form-poll rate)
                cfg.timer.poll_seconds.clamp(15, 86_400)
            } else {
                cfg.idle_poll_seconds.clamp(60, 86_400)
            };
            secs * 1000
        };

        let guard = ctx.watch_until.lock().unwrap();
        let _ = ctx.cv.wait_timeout(guard, Duration::from_millis(wait)).unwrap();
    }
}
