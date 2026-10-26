//! Twitch integration: local OAuth (authorization code) + EventSub over a WebSocket.
//! Subscriptions top up the marathon timer. No public URL or Cloudflare worker needed.
use crate::app::Ctx;
use crate::model::*;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const ID_BASE: &str = "https://id.twitch.tv";
const API_BASE: &str = "https://api.twitch.tv/helix";
const WS_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const SCOPES: &str = "channel:read:subscriptions channel:read:redemptions channel:manage:redemptions";

/// Twitch matches the redirect URI exactly against the app registration, so the
/// callback keeps this fixed port even when the main listener had to move.
pub const OAUTH_PORT: u16 = 8787;

pub fn redirect_uri() -> String { format!("http://localhost:{}/twitch/callback", OAUTH_PORT) }

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b { b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)) }
    }
    out
}

static OAUTH_STATE: OnceLock<Mutex<String>> = OnceLock::new();
fn state_cell() -> &'static Mutex<String> { OAUTH_STATE.get_or_init(|| Mutex::new(String::new())) }

/// ports whose EventSub socket is currently subscribed
static CONNECTED: OnceLock<Mutex<Vec<u16>>> = OnceLock::new();
fn connected_cell() -> &'static Mutex<Vec<u16>> { CONNECTED.get_or_init(|| Mutex::new(Vec::new())) }
fn set_connected(port: u16, on: bool) {
    let mut c = connected_cell().lock().unwrap();
    if on { if !c.contains(&port) { c.push(port); } } else { c.retain(|p| *p != port); }
}
pub fn is_connected(port: u16) -> bool { connected_cell().lock().unwrap().contains(&port) }

pub fn auth_url(port: u16) -> String {
    let s = load_twitch_at(port);
    let state: String = rand::thread_rng().sample_iter(&Alphanumeric).take(24).map(|c| (c as char).to_ascii_lowercase()).collect();
    *state_cell().lock().unwrap() = state.clone();
    format!("{}/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
        ID_BASE, s.client_id, urlencode(&redirect_uri()), urlencode(SCOPES), state)
}
pub fn check_state(s: &str) -> bool { let cur = state_cell().lock().unwrap().clone(); !cur.is_empty() && cur == s }

fn post_form(url: &str, body: &str) -> Result<Value, String> {
    let r = ureq::post(url).set("Content-Type", "application/x-www-form-urlencoded")
        .timeout(Duration::from_secs(15)).send_string(body);
    match r {
        Ok(resp) => { let t = resp.into_string().map_err(|e| e.to_string())?; serde_json::from_str(&t).map_err(|e| e.to_string()) }
        Err(ureq::Error::Status(code, resp)) => { let t = resp.into_string().unwrap_or_default();
            let t: String = t.chars().take(200).collect(); Err(format!("HTTP {} {}", code, t)) }
        Err(e) => Err(e.to_string()),
    }
}

fn helix_users(client_id: &str, token: &str) -> Result<(String, String), String> {
    match ureq::get(&format!("{}/users", API_BASE))
        .set("Client-Id", client_id).set("Authorization", &format!("Bearer {}", token))
        .timeout(Duration::from_secs(15)).call() {
        Ok(r) => {
            let t = r.into_string().map_err(|e| e.to_string())?;
            let v: Value = serde_json::from_str(&t).map_err(|e| e.to_string())?;
            let d = &v["data"][0];
            Ok((d["id"].as_str().unwrap_or("").to_string(), d["login"].as_str().unwrap_or("").to_string()))
        }
        Err(ureq::Error::Status(401, _)) => Err("401".into()),
        Err(e) => Err(e.to_string()),
    }
}

fn helix_get(url: &str, client_id: &str, token: &str) -> Result<Value, String> {
    match ureq::get(url).set("Client-Id", client_id).set("Authorization", &format!("Bearer {}", token))
        .timeout(Duration::from_secs(15)).call() {
        Ok(r) => { let t = r.into_string().map_err(|e| e.to_string())?; serde_json::from_str(&t).map_err(|e| e.to_string()) }
        Err(ureq::Error::Status(code, r)) => { let t = r.into_string().unwrap_or_default(); let t: String = t.chars().take(200).collect(); Err(format!("HTTP {} {}", code, t)) }
        Err(e) => Err(e.to_string()),
    }
}

/// find (or create) the channel-point rewards viewers redeem to bid; returns their ids
fn ensure_rewards(cfg: &Config, s: &TwitchSecrets) -> Result<Vec<String>, String> {
    let base = if cfg.auction.reward_title.trim().is_empty() { "Bid".to_string() } else { cfg.auction.reward_title.trim().to_string() };
    let costs: Vec<u64> = if cfg.auction.reward_costs.is_empty() { vec![1000] } else { cfg.auction.reward_costs.clone() };
    let url = format!("{}/channel_points/custom_rewards?broadcaster_id={}", API_BASE, s.broadcaster_id);
    let existing: Vec<(String, String)> = helix_get(&url, &s.client_id, &s.access_token).ok()
        .and_then(|v| v["data"].as_array().cloned())
        .map(|arr| arr.iter().filter_map(|r| Some((r["id"].as_str()?.to_string(), r["title"].as_str()?.to_string()))).collect())
        .unwrap_or_default();
    let mut ids = Vec::new();
    for cost in &costs {
        let title = if costs.len() == 1 { base.clone() } else { format!("{} {}k", base, cost / 1000) };
        if let Some((id, _)) = existing.iter().find(|(_, t)| *t == title) { ids.push(id.clone()); continue; }
        let body = json!({ "title": title, "cost": (*cost).max(1), "prompt": cfg.auction.reward_prompt, "is_enabled": true }).to_string();
        match ureq::post(&url).set("Client-Id", &s.client_id).set("Authorization", &format!("Bearer {}", s.access_token))
            .set("Content-Type", "application/json").timeout(Duration::from_secs(15)).send_string(&body) {
            Ok(r) => { let t = r.into_string().map_err(|e| e.to_string())?; let v: Value = serde_json::from_str(&t).map_err(|e| e.to_string())?;
                if let Some(id) = v["data"][0]["id"].as_str() { ids.push(id.to_string()); } }
            Err(ureq::Error::Status(code, r)) => { let t = r.into_string().unwrap_or_default(); let t: String = t.chars().take(200).collect(); return Err(format!("HTTP {} {}", code, t)); }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(ids)
}

/// exchange the OAuth `code` for tokens and store them (per-user, outside the project)
pub fn exchange_code_at(port: u16, code: &str) -> Result<TwitchSecrets, String> {
    let mut s = load_twitch_at(port);
    if s.client_secret.is_empty() { return Err("client secret not set".into()); }
    if s.client_id.is_empty() { return Err("client id not set".into()); }
    let body = format!("client_id={}&client_secret={}&code={}&grant_type=authorization_code&redirect_uri={}",
        urlencode(&s.client_id), urlencode(&s.client_secret), urlencode(code), urlencode(&redirect_uri()));
    let v = post_form(&format!("{}/oauth2/token", ID_BASE), &body)?;
    s.access_token = v["access_token"].as_str().unwrap_or("").to_string();
    s.refresh_token = v["refresh_token"].as_str().unwrap_or("").to_string();
    if s.access_token.is_empty() { return Err(format!("token exchange failed: {}", v)); }
    let (id, login) = helix_users(&s.client_id, &s.access_token)?;
    s.broadcaster_id = id; s.login = login;
    save_twitch_at(port, &s);
    Ok(s)
}

fn refresh(port: u16, s: &mut TwitchSecrets) -> Result<(), String> {
    if s.client_secret.is_empty() { return Err("no client secret".into()); }
    if s.client_id.is_empty() { return Err("no client id".into()); }
    let body = format!("grant_type=refresh_token&refresh_token={}&client_id={}&client_secret={}",
        urlencode(&s.refresh_token), urlencode(&s.client_id), urlencode(&s.client_secret));
    let v = post_form(&format!("{}/oauth2/token", ID_BASE), &body)?;
    if let Some(t) = v["access_token"].as_str() { s.access_token = t.to_string(); }
    if let Some(t) = v["refresh_token"].as_str() { s.refresh_token = t.to_string(); }
    if s.access_token.is_empty() { return Err("refresh failed".into()); }
    save_twitch_at(port, s);
    Ok(())
}

fn ensure_token(port: u16, s: &mut TwitchSecrets) -> Result<(), String> {
    if s.access_token.is_empty() { return refresh(port, s); }
    match helix_users(&s.client_id, &s.access_token) {
        Ok((id, login)) => { if s.broadcaster_id != id || s.login != login { s.broadcaster_id = id; s.login = login; save_twitch_at(port, s); } Ok(()) }
        Err(e) if e == "401" => refresh(port, s),
        Err(e) => Err(e),
    }
}

fn tier_secs(cfg: &Config, tier: &str) -> u64 {
    match tier { "2000" => cfg.timer.twitch_tier2, "3000" => cfg.timer.twitch_tier3, _ => cfg.timer.twitch_tier1 }
}
fn tier_label(tier: &str) -> &'static str {
    match tier { "2000" => "Tier 2", "3000" => "Tier 3", _ => "Tier 1" }
}

fn add_time(ctx: &Ctx, seconds: u64, label: String, subs: u64) {
    if seconds == 0 && subs == 0 { return; }
    let mut st = ctx.state.lock().unwrap();
    st.timer.add((seconds as i64) * 1000);
    st.timer.subs += subs;
    st.timer.push_log(TimerEvent { source: "twitch".into(), label: label.clone(), amount: 0.0, seconds: seconds as i64, at: now_ms() });
    drop(st);
    ctx.save();
    log(&format!("twitch: +{}s ({}), subs+{}", seconds, label, subs));
}

fn on_notification(ctx: &Ctx, sub_type: &str, event: &Value) {
    let cfg = ctx.cfg();
    let who = event["user_name"].as_str().filter(|s| !s.is_empty()).unwrap_or("someone");
    match sub_type {
        "channel.channel_points_custom_reward_redemption.add" => {
            let cost = event["reward"]["cost"].as_u64().unwrap_or_else(|| cfg.auction.reward_costs.first().copied().unwrap_or(1000)) as f64;
            let input = event["user_input"].as_str().unwrap_or("");
            let rid = event["id"].as_str().unwrap_or("");
            ctx.auction_bid_full(who, cost, "twitch", input, rid);
        }
        "channel.subscribe" if cfg.features.timer => {
            let tier = event["tier"].as_str().unwrap_or("1000");
            add_time(ctx, tier_secs(&cfg, tier), format!("{} · {}", who, tier_label(tier)), 1);
        }
        "channel.subscription.gift" if cfg.features.timer => {
            let tier = event["tier"].as_str().unwrap_or("1000");
            let total = event["total"].as_u64().unwrap_or(1).max(1);
            add_time(ctx, tier_secs(&cfg, tier) * total, format!("{} ×{} · {}", who, total, tier_label(tier)), total);
        }
        "channel.subscription.message" if cfg.features.timer => {
            let tier = event["tier"].as_str().unwrap_or("1000");
            add_time(ctx, tier_secs(&cfg, tier), format!("{} · resub {}", who, tier_label(tier)), 1);
        }
        _ => {}
    }
}

fn subscribe(s: &TwitchSecrets, sid: &str, ty: &str, reward: Option<&str>) -> Result<(), String> {
    let mut cond = json!({ "broadcaster_user_id": s.broadcaster_id });
    if let Some(r) = reward { cond["reward_id"] = json!(r); }
    let body = json!({ "type": ty, "version": "1",
        "condition": cond,
        "transport": { "method": "websocket", "session_id": sid } }).to_string();
    match ureq::post(&format!("{}/eventsub/subscriptions", API_BASE))
        .set("Client-Id", &s.client_id).set("Authorization", &format!("Bearer {}", s.access_token))
        .set("Content-Type", "application/json").timeout(Duration::from_secs(15)).send_string(&body) {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(code, r)) => { let t = r.into_string().unwrap_or_default(); let t: String = t.chars().take(200).collect(); Err(format!("HTTP {} {}", code, t)) }
        Err(e) => Err(e.to_string()),
    }
}

fn session(ctx: &Ctx, s: &TwitchSecrets, port: u16, auction: bool) -> Result<(), String> {
    let cfg = ctx.cfg();
    let (mut ws, _) = tungstenite::connect(WS_URL).map_err(|e| e.to_string())?;
    loop {
        let msg = ws.read().map_err(|e| e.to_string())?;
        if !ctx.cfg().features.timer && !ctx.cfg().features.auction { return Ok(()); } // both off: drop the socket
        let txt = match msg { tungstenite::Message::Text(t) => t, tungstenite::Message::Close(_) => return Ok(()), _ => continue };
        let v: Value = match serde_json::from_str(&txt) { Ok(v) => v, Err(_) => continue };
        match v["metadata"]["message_type"].as_str().unwrap_or("") {
            "session_welcome" => {
                let sid = v["payload"]["session"]["id"].as_str().unwrap_or("").to_string();
                if sid.is_empty() { return Err("no session id".into()); }
                if auction {
                    if cfg.features.auction {
                        match ensure_rewards(&cfg, s) {
                            Ok(rids) => { for rid in rids { if let Err(e) = subscribe(s, &sid, "channel.channel_points_custom_reward_redemption.add", Some(&rid)) {
                                log(&format!("twitch: redemption subscription: {}", e)); } } }
                            Err(e) => log(&format!("twitch: reward: {}", e)),
                        }
                    }
                } else if cfg.features.timer {
                    subscribe(s, &sid, "channel.subscribe", None)?;
                    subscribe(s, &sid, "channel.subscription.gift", None)?;
                    subscribe(s, &sid, "channel.subscription.message", None)?;
                }
                set_connected(port, true);
                log(if auction { "twitch: subscribed (channel-point redemptions)" } else { "twitch: subscribed (subs)" });
            }
            "session_keepalive" => {}
            "session_reconnect" => return Ok(()), // outer loop reconnects
            "notification" => {
                let ty = v["payload"]["subscription"]["type"].as_str().unwrap_or("");
                on_notification(ctx, ty, &v["payload"]["event"]);
            }
            "revocation" => log("twitch: a subscription was revoked"),
            _ => {}
        }
    }
}

/// background loop: keeps the EventSub WebSocket alive while Twitch is enabled
pub fn run(ctx: Arc<Ctx>, port: u16, auction: bool) {
    loop {
        let on = if auction { ctx.cfg().features.auction } else { ctx.cfg().features.timer };
        if !on { set_connected(port, false); sleep(10); continue; }
        let mut s = load_twitch_at(port);
        if s.client_id.is_empty() { set_connected(port, false); sleep(10); continue; }
        if s.access_token.is_empty() && s.refresh_token.is_empty() { set_connected(port, false); sleep(10); continue; }
        if let Err(e) = ensure_token(port, &mut s) { log(&format!("twitch[{}]: token: {}", port, e)); set_connected(port, false); sleep(20); continue; }
        if s.broadcaster_id.is_empty() { log(&format!("twitch[{}]: no broadcaster id", port)); set_connected(port, false); sleep(20); continue; }
        if let Err(e) = session(&ctx, &s, port, auction) { log(&format!("twitch[{}]: session ended: {}", port, e)); }
        set_connected(port, false);
        sleep(5);
    }
}
fn sleep(secs: u64) { std::thread::sleep(Duration::from_secs(secs)); }
