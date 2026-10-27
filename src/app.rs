use crate::model::*;
use crate::mono;
use serde_json::json;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// Bits bought on the sound-alerts page carry their nickname in the payment comment
/// ("... <code> | name: <nick>"). Every bit is counted as "<nick> SA", or "Анонім SA"
/// when no nickname was given.
/// Provider name for the media part of a StreamElements message.
fn media_label(url: &str, ytid: &str) -> &'static str {
    if !ytid.is_empty() { return "YouTube"; }
    let u = url.to_ascii_lowercase();
    if u.contains("tiktok") { "TikTok" }
    else if u.contains("instagram") { "Instagram" }
    else if u.contains("twitch") { "Twitch" }
    else if u.contains("reddit") { "Reddit" }
    else if u.contains("x.com") || u.contains("twitter") { "X" }
    else if u.contains("youtu") { "YouTube" }
    else { "Media" }
}

/// StreamElements message: "Message | SoundAlert: NAME | YouTube: LINK".
pub fn se_message(message: &str, media: &str, ytid: &str, sound: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !sound.is_empty() { parts.push(format!("SoundAlert: {}", sound)); }
    let link = if !ytid.is_empty() { format!("https://youtu.be/{}", ytid) } else { media.to_string() };
    if !link.is_empty() { parts.push(format!("{}: {}", media_label(&link, ytid), link)); }
    if parts.is_empty() { message.to_string() } else { format!("{} | {}", message, parts.join(" | ")) }
}
fn bit_display_name(comment: &str) -> String {
    if let Some(pos) = comment.find("name:") {
        let nick = comment[pos + 5..].split('|').next().unwrap_or("").trim();
        if !nick.is_empty() { return format!("{} SA", nick); }
    }
    "Анонім SA".to_string()
}

/// Public-deck order: `s:0,2,1 | mode:adhd | name: Nick`.
/// Returns the picked bits in the listed order and the playback mode.
fn parse_slot_order(comment: &str, bits: &[Bit]) -> Option<(Vec<Bit>, String)> {
    let pos = comment.find("bits:")?;
    let rest = &comment[pos + 5..];
    let list = rest.split(|c: char| c.is_whitespace() || c == '|').next().unwrap_or("");
    let mut picked: Vec<Bit> = Vec::new();
    for n in list.split(',') {
        if let Ok(slot) = n.trim().parse::<u32>() {
            if let Some(b) = bits.iter().find(|b| b.slot == slot) {
                if !picked.iter().any(|x| x.code == b.code) { picked.push(b.clone()); }
            }
        }
    }
    if picked.is_empty() { return None; }
    let lc = comment.to_ascii_lowercase();
    let mode = if lc.contains("mode:adhd") { "adhd" }
        else if lc.contains("mode:order") { "order" }
        else if picked.len() > 1 { "order" }
        else { "single" };
    Some((picked, mode.to_string()))
}

#[cfg(test)]
mod bit_name_tests {
    use super::*;

    #[test]
    fn bits_use_the_nickname_from_the_comment_with_sa_suffix() {
        assert_eq!(bit_display_name("донат сповіщення abc123 | name: goodfeelings"), "goodfeelings SA");
        assert_eq!(bit_display_name("донат сповіщення abc123"), "Анонім SA");
        assert_eq!(bit_display_name("| name:   "), "Анонім SA");
    }

    #[test]
    fn se_message_carries_the_real_link_and_sound_name() {
        assert_eq!(se_message("hi", "", "abc", ""), "hi | YouTube: https://youtu.be/abc");
        assert_eq!(se_message("hi", "https://www.tiktok.com/@x/video/1", "", "Airhorn"),
                   "hi | SoundAlert: Airhorn | TikTok: https://www.tiktok.com/@x/video/1");
        assert_eq!(se_message("hi", "", "", ""), "hi");
    }

    #[test]
    fn slot_orders_keep_click_order_and_mode() {
        let bits = vec![
            Bit { slot: 0, label: "A".into(), code: "a".into(), price: 10.0, ..Default::default() },
            Bit { slot: 1, label: "B".into(), code: "b".into(), price: 20.0, ..Default::default() },
            Bit { slot: 2, label: "C".into(), code: "c".into(), price: 30.0, ..Default::default() },
        ];
        let (picked, mode) = parse_slot_order("донат сповіщення bits:2,0 | mode:adhd | name: Nick", &bits).unwrap();
        assert_eq!(picked.iter().map(|b| b.code.as_str()).collect::<Vec<_>>(), vec!["c", "a"]);
        assert_eq!(mode, "adhd");
        let (picked, mode) = parse_slot_order("донат сповіщення bits:1 | name: Nick", &bits).unwrap();
        assert_eq!(picked[0].code, "b");
        assert_eq!(mode, "single");
        assert!(parse_slot_order("донат сповіщення bits:9", &bits).is_none());
    }
}

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::OnceLock;

/// Donation events held back while the sound-alerts overlay plays the chosen clips.
struct PreAlert { data: serde_json::Value, at: i64 }
static PRE_ALERTS: OnceLock<Mutex<HashMap<String, PreAlert>>> = OnceLock::new();
static PRE_SEQ: AtomicU64 = AtomicU64::new(1);
static BITS_OVERLAY: AtomicI64 = AtomicI64::new(0);
fn pre_map() -> &'static Mutex<HashMap<String, PreAlert>> { PRE_ALERTS.get_or_init(|| Mutex::new(HashMap::new())) }

/// The sound-alerts overlay polls with `&o=bits`; that is how we know it is open.
pub fn mark_bits_overlay() { BITS_OVERLAY.store(now_ms(), Ordering::Relaxed); }
fn bits_overlay_alive() -> bool {
    let t = BITS_OVERLAY.load(Ordering::Relaxed);
    t > 0 && now_ms() - t < 5000
}

/// A donation with sound alerts plays its clips on the bits overlay first; the
/// donation alert is broadcast only after that overlay reports the clips finished.
/// With no bits overlay open the alert goes out immediately (and silently).
pub fn send_donation(ctx: &Ctx, data: serde_json::Value, sound_urls: Vec<String>, sound_mode: &str) {
    if sound_urls.is_empty() || !bits_overlay_alive() {
        if !sound_urls.is_empty() { log("bits overlay is not open, sending the alert without sounds"); }
        ctx.broadcast(json!({ "type": "donation", "data": data }));
        return;
    }
    let token = format!("snd-{}-{}", now_ms(), PRE_SEQ.fetch_add(1, Ordering::Relaxed));
    pre_map().lock().unwrap().insert(token.clone(), PreAlert { data, at: now_ms() });
    log(&format!("sounds pre-roll: {} clip(s) mode={}", sound_urls.len(), sound_mode));
    ctx.broadcast(json!({ "type": "bits_pre", "data": {
        "token": token, "sounds": sound_urls, "mode": sound_mode } }));
}

pub fn pre_alert_done(ctx: &Ctx, token: &str) -> bool {
    let item = pre_map().lock().unwrap().remove(token);
    match item {
        Some(p) => { log("sounds pre-roll done, sending the donation"); ctx.broadcast(json!({ "type": "donation", "data": p.data })); true }
        None => { log("sounds pre-roll done for an unknown/expired token"); false }
    }
}

/// Safety net: if the bits overlay dies mid-play, do not lose the alert.
pub fn expire_pre_alerts(ctx: &Ctx) {
    let now = now_ms();
    let expired: Vec<PreAlert> = {
        let mut m = pre_map().lock().unwrap();
        let keys: Vec<String> = m.iter().filter(|(_, v)| now - v.at > 30000).map(|(k, _)| k.clone()).collect();
        keys.into_iter().filter_map(|k| m.remove(&k)).collect()
    };
    for p in expired { log("sounds pre-roll timed out, sending the donation"); ctx.broadcast(json!({ "type": "donation", "data": p.data })); }
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

        // public deck order: slots + mode, e.g. "bits:0,1,2 | mode:adhd | name: Nick"
        let slot_order = parse_slot_order(&comment, &cfg.bits);
        if let Some((picked, mode)) = slot_order {
            let name = bit_display_name(&comment);
            let msg = "Нічого не написав".to_string();
            let labels = picked.iter().map(|b| b.label.clone()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ");
            let codes = picked.iter().map(|b| b.code.clone()).collect::<Vec<_>>().join(",");
            jar.history.insert(0, HistoryItem {
                id: tx.id.clone(), name: name.clone(), amount: uah, message: msg.clone(),
                youtube_id: String::new(), media_url: String::new(), bit_label: labels.clone(), bit_code: codes,
                matched: "bit".into(), code: None, at: now_ms(),
            });
            jar.stats.count += 1; jar.stats.total += uah;
            if uah > jar.stats.biggest { jar.stats.biggest = uah; }
            drop(st);
            if picked.len() == 1 {
                if !quiet {
                    self.broadcast(json!({ "type": "bit", "data": {
                        "id": tx.id, "name": name, "amount": uah, "message": msg, "currency": cur, "bit": picked[0].clone() } }));
                }
            } else {
                let sounds: Vec<String> = picked.iter().filter_map(|b| {
                    let s = if b.kind == "sound" { b.url.clone() } else { b.sound.clone() };
                    if s.is_empty() { None } else { Some(s) }
                }).collect();
                if !quiet && !sounds.is_empty() {
                    self.broadcast(json!({ "type": "bits_play", "data": {
                        "id": tx.id, "name": name, "amount": uah, "sounds": sounds, "mode": mode.clone() } }));
                }
            }
            if !quiet {
                log(&format!("[{}] bit order {} ({}) {:.0} from {}", jar_code, labels, mode, uah, name));
                crate::streamelements::spawn_tip(cfg.port, cfg.clone(), name.clone(), uah, iso.clone(), se_message(&msg, "", "", &labels));
            }
            self.timer_add_donation(uah, &name, quiet);
            if !quiet { self.auction_bid(&name, uah, "monobank"); }
            self.save();
            return false;
        }

        // bits have their own codes: a donation whose comment contains a bit code plays on the bits overlay
        if let Some(bit) = cfg.bits.iter().find(|b| !b.code.is_empty() && low.contains(&b.code.to_lowercase())).cloned() {
            let name = bit_display_name(&comment);
            let msg = "Нічого не написав".to_string();
            jar.history.insert(0, HistoryItem {
                id: tx.id.clone(), name: name.clone(), amount: uah, message: msg.clone(),
                youtube_id: String::new(), media_url: String::new(), bit_label: bit.label.clone(), bit_code: bit.code.clone(), matched: "bit".into(), code: Some(bit.code.clone()), at: now_ms(),
            });
            jar.stats.count += 1; jar.stats.total += uah;
            if uah > jar.stats.biggest { jar.stats.biggest = uah; }
            drop(st);
            if !quiet {
                self.broadcast(json!({ "type": "bit", "data": {
                    "id": tx.id, "name": name, "amount": uah, "message": msg, "currency": cur, "bit": bit.clone() } }));
                log(&format!("[{}] bit '{}' ({}) {:.0} from {}", jar_code, bit.label, bit.kind, uah, name));
                crate::streamelements::spawn_tip(cfg.port, cfg.clone(), name.clone(), uah, iso.clone(), se_message(&msg, "", "", &bit.label));
            }
            self.timer_add_donation(uah, &name, quiet);
            if !quiet { self.auction_bid(&name, uah, "monobank"); }
            self.save();
            return false;
        }

        // match a pending donation by code first, else by amount + recency
        let mut idx = jar.pending.iter().position(|p| low.contains(&p.code.to_lowercase()));
        let mut matched = "code";
        if idx.is_none() {
            let now = now_secs();
            idx = jar.pending.iter().position(|p| {
                (p.amount - uah).abs() <= 1.0f64.max(p.amount * 0.02) && (now - p.created_at / 1000) < 3600
            });
            if idx.is_some() { matched = "amount"; }
        }

        // resolve the alert payload (if any) plus the local history record
        let (name, message, youtube_id, media_url, kind, code, alert, voice, sound_urls, bit_label, bit_code, sound_mode) = if let Some(k) = idx {
            let p = jar.pending.remove(k);
            // the fallback-link twin is the same intent, never a second alert
            if !p.pair.is_empty() { jar.pending.retain(|x| x.code != p.pair); }
            let bits_all: Vec<crate::model::Bit> = if !p.bits.is_empty() { p.bits.clone() } else { p.bit.clone().into_iter().collect() };
            let sound_urls: Vec<String> = bits_all.iter().filter_map(|b| {
                let s = if b.kind == "sound" { b.url.clone() } else { b.sound.clone() };
                if s.is_empty() { None } else { Some(s) }
            }).collect();
            let sound_mode = if p.bits_mode.is_empty() { cfg.bits_mode.clone() } else { p.bits_mode.clone() };
            let bit_label = bits_all.iter().map(|b| b.label.clone()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ");
            let bit_code = bits_all.iter().map(|b| b.code.clone()).collect::<Vec<_>>().join(",");
            let pname = if p.name.trim().is_empty() {
                if p.bit.is_some() || !p.bits.is_empty() { "Анонім SA".to_string() } else { "Анонім".to_string() }
            } else { p.name.trim().to_string() };
            let pmsg = if p.message.trim().is_empty() { "Нічого не написав".to_string() } else { p.message.clone() };
            (pname, pmsg, p.youtube_id, p.media_url, matched, Some(p.code), true, p.voice, sound_urls, bit_label, bit_code, sound_mode)
        } else {
            let name = "Анонім".to_string();
            let raw: String = comment.chars().take(cfg.limits.message_max).collect();
            let msg = if raw.trim().is_empty() { format!("{} нічого не написав", name) } else { raw };
            let v = String::new();
            if cfg.notify_unmatched { (name, msg, String::new(), String::new(), "unmatched", None, true, v, Vec::new(), String::new(), String::new(), String::new()) }
            else { (name, msg, String::new(), String::new(), "none", None, false, v, Vec::new(), String::new(), String::new(), String::new()) }
        };

        // every positive transaction is kept in the local history + stats, whether or not it alerts
        jar.history.insert(0, HistoryItem {
            id: tx.id.clone(), name: name.clone(), amount: uah, message: message.clone(),
            youtube_id: youtube_id.clone(), media_url: media_url.clone(), bit_label: bit_label.clone(), bit_code: bit_code.clone(), matched: kind.into(), code: code.clone(), at: now_ms(),
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
                    if crate::tts::synth(&cfg.tts.dir, &eff_voice, &spoken, &dir.join(&fname), &cfg.tts.azure_key, &cfg.tts.azure_region, cfg.tts.speed).is_ok() {
                        tts_url = crate::model::tts_url(&cfg, &fname);
                    } else {
                        log("tts synth failed");
                    }
                }
            }
            crate::app::send_donation(self, json!({
                "id": tx.id, "name": name, "amount": uah, "message": message,
                "youtubeId": youtube_id, "mediaUrl": media_url, "matched": kind, "jar": jar_code, "currency": cur,
                "ttsUrl": tts_url }), sound_urls, &sound_mode);
            log(&format!("[{}] notification ({}) {:.0} UAH from {}", jar_code, kind, uah, name));
            // StreamElements: mirror the live donation as a tip so the SE alert box plays
            crate::streamelements::spawn_tip(cfg.port, cfg.clone(), name.clone(), uah, iso.clone(), se_message(&message, &media_url, &youtube_id, &bit_label));
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
