use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use rand::distributions::Alphanumeric;
use rand::Rng;

/// Set by the UI theme toggle; drives the taskbar/window icon colour.
pub static LIGHT_THEME: AtomicBool = AtomicBool::new(false);

/// ports this instance actually listens on (used by the setup page status)
pub static BOUND_PORTS: std::sync::Mutex<Vec<u16>> = std::sync::Mutex::new(Vec::new());

/* ---------------------------------------------------------------- time */

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
pub fn now_secs() -> i64 { now_ms() / 1000 }

pub fn local_midnight_ms() -> i64 {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    unsafe {
        let mut st = std::mem::zeroed();
        GetLocalTime(&mut st);
        let secs = (st.wHour as i64) * 3600 + (st.wMinute as i64) * 60 + (st.wSecond as i64);
        now_ms() - secs * 1000
    }
}

/* ---------------------------------------------------------------- paths */

#[derive(Clone)]
pub struct Paths { pub base: PathBuf }
impl Paths {
    pub fn new() -> Self {
        let exe_dir = std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        // user data lives in %APPDATA%\LocalMonoDonate so updating/replacing the app keeps settings
        let base = std::env::var_os("LMD_DATA").map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("LocalMonoDonate")))
            .unwrap_or_else(|| exe_dir.clone());
        if base != exe_dir {
            let _ = fs::create_dir_all(&base);
            // one-time migration from an older portable install that kept data next to the exe
            for f in ["config.json", "data.json"] {
                let (src, dst) = (exe_dir.join(f), base.join(f));
                if src.exists() && !dst.exists() { let _ = fs::copy(&src, &dst); }
            }
            let (sm, dm) = (exe_dir.join("media"), base.join("media"));
            if sm.is_dir() && !dm.exists() { copy_dir_all(&sm, &dm); }
        }
        Paths { base }
    }
    pub fn config(&self) -> PathBuf { self.base.join("config.json") }
    pub fn data(&self) -> PathBuf { self.base.join("data.json") }
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) {
    let _ = fs::create_dir_all(dst);
    if let Ok(rd) = fs::read_dir(src) {
        for e in rd.flatten() {
            let p = e.path();
            let d = dst.join(e.file_name());
            if p.is_dir() { copy_dir_all(&p, &d); } else { let _ = fs::copy(&p, &d); }
        }
    }
}

/* ---------------------------------------------------------------- config */

fn d_host() -> String { "127.0.0.1".into() }
fn d_bits_mode() -> String { "single".into() }
fn d_port() -> u16 { 27183 }
fn d_aport() -> u16 { 27184 }
fn d_public_port() -> u16 { 27185 }   // public (tunnel-facing) server; 0 = off
fn d_min() -> f64 { 10.0 }
fn d_max() -> f64 { 29999.0 }
fn d_prefix() -> String { "tf".into() }
fn d_true() -> bool { true }
fn d_idle() -> u64 { 600 }
fn d_poll() -> u64 { 20 }
fn d_ttl() -> u64 { 24 }
fn d_s() -> String { String::new() }
fn d_grid() -> u32 { 3 }
fn uah_code() -> i64 { 980 }
/// ISO-4217 numeric currency code → display symbol (Monobank jar/statement currency).
pub fn currency_symbol(code: i64) -> String {
    match code {
        980 => "\u{20B4}", 840 => "$", 978 => "\u{20AC}", 985 => "z\u{142}", 826 => "\u{A3}",
        756 => "CHF", 392 => "\u{A5}", 203 => "K\u{10D}", 348 => "Ft", 946 => "lei",
        124 => "C$", 36 => "A$", 752 => "kr", 208 => "kr", 578 => "kr",
        _ => "",
    }.to_string()
}

/// ISO-4217 numeric code -> the 3-letter code StreamElements tips expect.
pub fn currency_iso(code: i64) -> String {
    match code {
        980 => "UAH", 840 => "USD", 978 => "EUR", 985 => "PLN", 826 => "GBP", 756 => "CHF",
        392 => "JPY", 203 => "CZK", 348 => "HUF", 946 => "RON", 124 => "CAD", 36 => "AUD",
        752 => "SEK", 208 => "DKK", 578 => "NOK", 949 => "TRY", 356 => "INR", 76 => "BRL",
        484 => "MXN", 608 => "PHP", 702 => "SGD", 764 => "THB", 344 => "HKD", 376 => "ILS",
        458 => "MYR", 554 => "NZD", 643 => "RUB", 901 => "TWD",
        _ => "UAH",
    }.to_string()
}

pub const MAX_JARS: usize = 5;
pub const MAX_QR_WIDGETS: usize = 5;

/// true when we can bind the public listener on this port right now
pub fn port_free(port: u16) -> bool {
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::time::Duration;
    // a live listener answers a connect even if it never accepts; this catches services
    // that allow a non-exclusive second bind (which made the old check unreliable)
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    if TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok() { return false; }
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(l) => { drop(l); true }
        Err(_) => false,
    }
}

/// first free port at/after `preferred`, skipping the app's own ports.
/// Falls back to an OS-assigned port if the whole range is taken.
pub fn pick_free_port(preferred: u16, skip: &[u16]) -> u16 {
    let start = preferred.max(1024);
    let end = start.saturating_add(200);
    for p in start..end {
        if skip.contains(&p) { continue; }
        if port_free(p) { return p; }
    }
    std::net::TcpListener::bind(("0.0.0.0", 0))
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|a| a.port())
        .unwrap_or(preferred)
}



#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    #[serde(default = "d_name_max")] pub name_max: usize,
    #[serde(default = "d_msg_max")] pub message_max: usize,
}
fn d_name_max() -> usize { 40 }
fn d_msg_max() -> usize { 500 }
impl Default for Limits { fn default() -> Self { Limits { name_max: 40, message_max: 500 } } }

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OverlayCfg {
    #[serde(default = "d_dur")] pub duration_ms: u64,
    #[serde(default = "d_vmax")] pub video_max_ms: u64,
}
fn d_dur() -> u64 { 8000 }
fn d_vmax() -> u64 { 180000 }
impl Default for OverlayCfg { fn default() -> Self { OverlayCfg { duration_ms: 8000, video_max_ms: 180000 } } }

/* marathon timer (donations + Twitch subs top up a countdown) */
fn t_per_amount() -> f64 { 10.0 }
fn t_per_seconds() -> f64 { 60.0 }
fn t_min() -> f64 { 10.0 }
fn t_t1() -> u64 { 300 }
fn t_t2() -> u64 { 600 }
fn t_t3() -> u64 { 1500 }
fn t_poll() -> u64 { 60 }
fn t_color() -> String { "#ffffff".into() }
fn t_accent() -> String { "#7dffb0".into() }
fn t_outline() -> String { "#000000".into() }
fn t_size() -> f64 { 22.0 }
fn t_font() -> String { String::new() }

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerCfg {
    /// "every `per_amount` of currency adds `per_seconds` seconds" (proportional)
    #[serde(default = "t_per_amount")] pub per_amount: f64,
    #[serde(default = "t_per_seconds")] pub per_seconds: f64,
    #[serde(default = "t_min")] pub min_donation: f64,
    #[serde(default = "t_t1")] pub twitch_tier1: u64,
    #[serde(default = "t_t2")] pub twitch_tier2: u64,
    #[serde(default = "t_t3")] pub twitch_tier3: u64,
    /// how often the Monobank statement is polled while the timer is enabled
    #[serde(default = "t_poll")] pub poll_seconds: u64,
    // overlay look
    #[serde(default = "t_color")] pub color: String,
    #[serde(default = "t_accent")] pub accent: String,
    #[serde(default = "t_outline")] pub outline: String,
    #[serde(default = "t_size")] pub size: f64,
    #[serde(default = "t_font")] pub font: String,
}
impl Default for TimerCfg {
    fn default() -> Self {
        TimerCfg { per_amount: t_per_amount(), per_seconds: t_per_seconds(), min_donation: t_min(),
            twitch_tier1: t_t1(), twitch_tier2: t_t2(), twitch_tier3: t_t3(), poll_seconds: t_poll(),
            color: t_color(), accent: t_accent(), outline: t_outline(), size: t_size(), font: t_font() }
    }
}

/// one top-up event in the marathon log
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimerEvent {
    #[serde(default)] pub source: String,  // "monobank" | "twitch" | "manual"
    #[serde(default)] pub label: String,   // donor name / tier label
    #[serde(default)] pub amount: f64,     // donation amount (0 for twitch/manual)
    #[serde(default)] pub seconds: i64,    // seconds added (can be negative for manual -)
    #[serde(default)] pub at: i64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimerState {
    #[serde(default)] pub running: bool,
    #[serde(default)] pub ends_at: i64,      // ms, valid while running
    #[serde(default)] pub remaining_ms: i64, // ms, valid while paused
    #[serde(default)] pub subs: u64,         // lifetime: subscriptions that topped up the timer
    #[serde(default)] pub money: f64,        // lifetime: donations that topped up the timer
    #[serde(default)] pub log: Vec<TimerEvent>,
}
impl TimerState {
    pub fn remaining(&self) -> i64 {
        if self.running { (self.ends_at - now_ms()).max(0) } else { self.remaining_ms.max(0) }
    }
    pub fn add(&mut self, ms: i64) {
        if self.running { self.ends_at += ms; } else { self.remaining_ms = (self.remaining_ms + ms).max(0); }
    }
    pub fn set(&mut self, ms: i64) {
        let ms = ms.max(0);
        if self.running { self.ends_at = now_ms() + ms; } else { self.remaining_ms = ms; }
    }
    pub fn pause(&mut self) { self.remaining_ms = self.remaining(); self.running = false; self.ends_at = 0; }
    /// stop the countdown keeping the last snapshot (used when the app closes/reboots)
    pub fn freeze(&mut self) { self.running = false; self.ends_at = 0; }
    pub fn resume(&mut self) { self.ends_at = now_ms() + self.remaining_ms.max(0); self.running = true; }
    pub fn push_log(&mut self, ev: TimerEvent) { self.log.insert(0, ev); self.log.truncate(200); }
}

/* auction / wheel (viewers bid with channel points or donations, then a winner is picked) */
fn a_timer() -> u64 { 300 }
fn a_costs() -> Vec<u64> { vec![1000] }
fn a_reward() -> String { "Bid".into() }
fn a_title() -> String { "Auction".into() }

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuctionCfg {
    // timer
    #[serde(default = "a_timer")] pub timer_seconds: u64, // countdown to auto-end (0 = off)
    #[serde(default)] pub auto_end: bool,                 // pick a winner automatically when it ends
    // bids
    #[serde(default)] pub min_bid: f64,                   // ignore bids below this
    #[serde(default = "a_costs")] pub reward_costs: Vec<u64>, // channel-point reward costs
    #[serde(default = "a_reward")] pub reward_title: String,
    #[serde(default = "a_reward")] pub reward_prompt: String,
    // appearance
    #[serde(default = "a_title")] pub title: String,
    #[serde(default = "t_color")] pub color: String,
    #[serde(default = "t_accent")] pub accent: String,
    #[serde(default = "t_outline")] pub outline: String,
    #[serde(default = "t_font")] pub font: String,
    // merged AUC settings (exe-owned; the web page has no settings)
    #[serde(default = "a_mode")] pub mode: String,       // manual | auto
    #[serde(default = "a_source")] pub source: String,   // points | donations | both
    #[serde(default = "a_cur")] pub currency: String,    // UAH | USD
    #[serde(default)] pub show_total_time: bool,
    #[serde(default)] pub leader_change_points: f64,
    #[serde(default)] pub leader_change_donations: f64,
    #[serde(default)] pub new_position_points: f64,
    #[serde(default)] pub new_position_donations: f64,
    #[serde(default)] pub add_existing_points: f64,
    #[serde(default)] pub add_existing_donations: f64,
    #[serde(default)] pub time_more_than_min: f64,
}
fn a_mode() -> String { "manual".into() }
fn a_source() -> String { "points".into() }
fn a_cur() -> String { "UAH".into() }
impl Default for AuctionCfg {
    fn default() -> Self {
        AuctionCfg { timer_seconds: a_timer(), auto_end: false, min_bid: 0.0, reward_costs: a_costs(),
            reward_title: a_reward(), reward_prompt: a_reward(), title: a_title(),
            color: t_color(), accent: t_accent(), outline: t_outline(), font: t_font(),
            mode: a_mode(), source: a_source(), currency: a_cur(), show_total_time: false,
            leader_change_points: 0.0, leader_change_donations: 0.0, new_position_points: 0.0,
            new_position_donations: 0.0, add_existing_points: 0.0, add_existing_donations: 0.0, time_more_than_min: 0.0 }
    }
}

/// one bidder's aggregated bid
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Bid {
    #[serde(default)] pub name: String,
    #[serde(default)] pub amount: f64,
    #[serde(default)] pub source: String, // "twitch" | "monobank" | "manual"
    #[serde(default)] pub at: i64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuctionState {
    #[serde(default)] pub active: bool,
    #[serde(default)] pub title: String,
    #[serde(default)] pub bids: Vec<Bid>,      // one entry per bidder, amount = total
    #[serde(default)] pub winner: Option<Bid>,
    #[serde(default)] pub ends_at: i64,        // ms, valid while active
    #[serde(default)] pub spin: i64,           // increments to trigger a wheel spin on the overlay
}
impl AuctionState {
    /// aggregate a new bid into the bidder's total (or add the bidder)
    pub fn add_bid(&mut self, name: &str, amount: f64, source: &str) {
        if amount <= 0.0 { return; }
        let key = donor_key(name);
        if let Some(b) = self.bids.iter_mut().find(|b| donor_key(&b.name) == key) {
            b.amount += amount; b.at = now_ms();
            if !source.is_empty() { b.source = source.into(); }
        } else {
            self.bids.push(Bid { name: name.to_string(), amount, source: source.into(), at: now_ms() });
        }
    }
    pub fn total(&self) -> f64 { self.bids.iter().map(|b| b.amount).sum() }
}

/// weighted-random pick over bids (chance proportional to amount)
pub fn weighted_pick(bids: &[Bid]) -> Option<Bid> {
    if bids.is_empty() { return None; }
    let total: f64 = bids.iter().map(|b| b.amount).sum();
    if total <= 0.0 { return bids.first().cloned(); }
    let mut r = rand::thread_rng().gen::<f64>() * total;
    for b in bids { r -= b.amount; if r <= 0.0 { return Some(b.clone()); } }
    bids.last().cloned()
}

/// (auction UI, auction keys settings) URLs
pub fn auction_links(cfg: &Config) -> (String, String) {
    let p = cfg.port; let n = sanitize_name(&cfg.nickname);
    (format!("http://{}.auction.localhost:{}/", n, p),
     format!("http://localhost:{}/auctionsettings", p))
}

/* Twitch: subscriptions top up the marathon timer via EventSub WebSocket.
   Credentials (client id / secret / tokens) live in the per-user secrets file, never in the project. */

/// which optional features are enabled (toggled on the Configuration page)
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Features {
    #[serde(default = "d_true")] pub donate: bool,
    #[serde(default = "d_true")] pub stats: bool,
    #[serde(default = "d_true")] pub overlay: bool,
    #[serde(default = "d_true")] pub bits: bool,
    #[serde(default = "d_true")] pub bits_overlay: bool,
    #[serde(default = "d_true")] pub youtube: bool,
    #[serde(default = "d_true")] pub timer: bool,
    #[serde(default = "d_true")] pub auction: bool,
    #[serde(default = "d_true")] pub qr: bool,
}
impl Default for Features {
    fn default() -> Self { Features { donate: true, stats: true, overlay: true, bits: true, bits_overlay: true, youtube: true, timer: true, auction: true, qr: true } }
}

/* notification appearance (own clean-room implementation) */
fn n_font() -> String { "Segoe UI".into() }
fn n_dur() -> u64 { 8000 }
fn n_ytmin() -> f64 { 0.0 }
fn n_ytviews() -> u64 { 500 }
fn n_ytmaxmin() -> u64 { 20 }
fn n_othermin() -> f64 { 0.0 }
fn n_outc() -> String { "#000000".into() }
fn n_anin() -> String { "slide_up".into() }
fn n_anout() -> String { "fade".into() }
fn n_anms() -> u64 { 400 }
fn n_maxw() -> u32 { 720 }
fn m_pos() -> String { "top".into() }
fn n_toptext() -> String { String::new() }
fn n_moneytext() -> String { "%donate% задонатив %money%".into() }
fn n_bsize() -> u32 { 20 }
fn n_bweight() -> u32 { 600 }
fn n_bcolor() -> String { "#ffffff".into() }
fn n_gsize() -> u32 { 26 }
fn n_gweight() -> u32 { 800 }
fn n_gcolor() -> String { "#ffffff".into() }
fn n_msize() -> u32 { 18 }
fn n_mweight() -> u32 { 500 }
fn n_mcolor() -> String { "#ffffff".into() }
fn n_msgh() -> u32 { 96 }
fn n_winbg() -> String { "#131313".into() }
fn n_oop() -> f64 { 0.0 }
fn n_winop() -> f64 { 0.90 }
fn n_winrad() -> u32 { 16 }
fn n_winmaxh() -> u32 { 0 }
fn n_rad12() -> u32 { 12 }
fn n_mw() -> u32 { 720 }
fn n_mh() -> u32 { 320 }
fn n_zoom() -> f64 { 18.0 }
fn m_rad() -> u32 { 12 }
fn n_panx() -> f64 { -12.0 }
fn n_pany() -> f64 { 32.0 }
fn n_img() -> String { "/assets/default.jpg".into() }

#[derive(Serialize, Deserialize, Clone)]
pub struct NotifyCfg {
    // two templated lines: %donate% = donor name, %money% = amount (both work in either line)
    #[serde(default = "n_toptext")] pub top_text: String,
    #[serde(default = "n_moneytext")] pub money_text: String,
    // top (light-blue) line style
    #[serde(default = "n_font")] pub blue_font: String,
    #[serde(default = "n_bsize")] pub blue_size: u32,
    #[serde(default = "n_bweight")] pub blue_weight: u32,
    #[serde(default = "n_bcolor")] pub blue_color: String,
    #[serde(default = "n_outc")] pub blue_outline: String,
    #[serde(default = "n_oop")] pub blue_outline_op: f64,
    #[serde(default = "n_maxw")] pub blue_maxw: u32,
    // money (green) line style
    #[serde(default = "n_font")] pub green_font: String,
    #[serde(default = "n_gsize")] pub green_size: u32,
    #[serde(default = "n_gweight")] pub green_weight: u32,
    #[serde(default = "n_gcolor")] pub green_color: String,
    #[serde(default = "n_outc")] pub green_outline: String,
    #[serde(default = "n_oop")] pub green_outline_op: f64,
    #[serde(default = "n_maxw")] pub green_maxw: u32,
    // message (red) style
    #[serde(default = "n_font")] pub msg_font: String,
    #[serde(default = "n_msize")] pub msg_size: u32,
    #[serde(default = "n_mweight")] pub msg_weight: u32,
    #[serde(default = "n_mcolor")] pub msg_color: String,
    #[serde(default = "n_outc")] pub msg_outline: String,
    #[serde(default = "n_oop")] pub msg_outline_op: f64,
    #[serde(default = "n_maxw")] pub msg_maxw: u32,
    #[serde(default = "n_msgh")] pub msg_maxh: u32,
    // backgrounds
    #[serde(default = "n_winbg")] pub win_bg: String,
    #[serde(default = "n_winop")] pub win_bg_opacity: f64,
    #[serde(default = "n_winrad")] pub win_radius: u32,
    #[serde(default = "n_winmaxh")] pub win_maxh: u32,
    #[serde(default = "n_outc")] pub blue_bg: String,
    #[serde(default)] pub blue_bg_opacity: f64,
    #[serde(default = "n_rad12")] pub blue_radius: u32,
    #[serde(default = "n_outc")] pub green_bg: String,
    #[serde(default)] pub green_bg_opacity: f64,
    #[serde(default = "n_rad12")] pub green_radius: u32,
    #[serde(default = "n_outc")] pub red_bg: String,
    #[serde(default)] pub red_bg_opacity: f64,
    #[serde(default = "n_rad12")] pub red_radius: u32,
    // duration
    #[serde(default = "n_dur")] pub duration_ms: u64,
    #[serde(default)] pub wait_media: bool,
    #[serde(default = "n_ytmin")] pub yt_min: f64,
    #[serde(default = "n_ytviews")] pub yt_views: u64,
    #[serde(default = "n_ytmaxmin")] pub yt_max_min: u64,
    #[serde(default)] pub yt_playlists: bool,
    /// minimum amount for a non-YouTube media link
    #[serde(default = "n_othermin")] pub other_media_min: f64,
    // enabled media link types (default style is the global switch)
    #[serde(default = "d_true")] pub media_youtube: bool,
    #[serde(default = "d_true")] pub media_instagram: bool,
    #[serde(default = "d_true")] pub media_tiktok: bool,
    #[serde(default = "d_true")] pub media_twitch: bool,
    #[serde(default = "d_true")] pub media_reddit: bool,
    #[serde(default = "d_true")] pub media_x: bool,
    // animation
    #[serde(default = "n_anin")] pub anim_in: String,
    #[serde(default = "n_anout")] pub anim_out: String,
    #[serde(default = "n_anms")] pub anim_ms: u64,
    // media
    #[serde(default = "d_s")] pub image_url: String,
    #[serde(default = "d_s")] pub video_url: String,
    #[serde(default = "d_s")] pub sound_url: String,
    #[serde(default = "m_pos")] pub media_pos: String,
    #[serde(default = "n_mw")] pub media_w: u32,
    #[serde(default = "n_mh")] pub media_h: u32,
    #[serde(default = "n_zoom")] pub media_zoom: f64,
    #[serde(default = "m_rad")] pub media_radius: u32,
    #[serde(default = "n_panx")] pub media_pan_x: f64,
    #[serde(default = "n_pany")] pub media_pan_y: f64,
    #[serde(default = "n_maxw")] pub max_width: u32,
}
impl Default for NotifyCfg {
    fn default() -> Self {
        NotifyCfg {
            top_text: n_toptext(), money_text: n_moneytext(),
            blue_font: n_font(), blue_size: n_bsize(), blue_weight: n_bweight(), blue_color: n_bcolor(), blue_outline: n_outc(), blue_outline_op: n_oop(), blue_maxw: n_maxw(),
            green_font: n_font(), green_size: n_gsize(), green_weight: n_gweight(), green_color: n_gcolor(), green_outline: n_outc(), green_outline_op: n_oop(), green_maxw: n_maxw(),
            msg_font: n_font(), msg_size: n_msize(), msg_weight: n_mweight(), msg_color: n_mcolor(), msg_outline: n_outc(), msg_outline_op: n_oop(), msg_maxw: n_maxw(), msg_maxh: n_msgh(),
            win_bg: n_winbg(), win_bg_opacity: n_winop(), win_radius: n_winrad(), win_maxh: n_winmaxh(),
            blue_bg: n_outc(), blue_bg_opacity: 0.0, blue_radius: n_rad12(),
            green_bg: n_outc(), green_bg_opacity: 0.0, green_radius: n_rad12(),
            red_bg: n_outc(), red_bg_opacity: 0.0, red_radius: n_rad12(),
            duration_ms: n_dur(), wait_media: false, yt_min: n_ytmin(), yt_views: n_ytviews(), yt_max_min: n_ytmaxmin(), yt_playlists: false, other_media_min: n_othermin(),
            media_youtube: true, media_instagram: true, media_tiktok: true, media_twitch: true, media_reddit: true, media_x: true,
            anim_in: n_anin(), anim_out: n_anout(), anim_ms: n_anms(),
            image_url: n_img(), video_url: String::new(), sound_url: String::new(),
            media_pos: m_pos(), media_w: n_mw(), media_h: n_mh(), media_zoom: n_zoom(), media_radius: m_rad(),
            media_pan_x: n_panx(), media_pan_y: n_pany(), max_width: n_maxw(),
        }
    }
}

impl NotifyCfg {
    pub fn media_enabled(&self, provider: &str) -> bool {
        match provider {
            "youtube" => self.media_youtube,
            "instagram" => self.media_instagram,
            "tiktok" => self.media_tiktok,
            "twitch" => self.media_twitch,
            "reddit" => self.media_reddit,
            "x" => self.media_x,
            _ => true,
        }
    }
}

/// one rule: an amount range [min, max] (max<=0 = no upper bound) and its style
#[derive(Serialize, Deserialize, Clone)]
pub struct NotifyRule {
    #[serde(default = "d_s")] pub id: String,
    #[serde(default = "d_s")] pub label: String,
    #[serde(default)] pub min: f64,
    #[serde(default)] pub max: f64,
    #[serde(default)] pub pool: bool,
    #[serde(default)] pub style: NotifyCfg,
    #[serde(default)] pub test_media_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NotifyRules {
    #[serde(default)] pub rules: Vec<NotifyRule>,
}

pub fn default_rules() -> NotifyRules {
    NotifyRules { rules: vec![NotifyRule { id: "default".into(), label: "Default".into(), min: 10.0, max: 29999.0, pool: false, style: NotifyCfg::default(), test_media_url: String::new() }] }
}

/// keep exactly one "default" catch-all rule, always last
pub fn normalize_rules(mut r: NotifyRules) -> NotifyRules {
    let mut def = r.rules.iter().position(|x| x.id == "default").map(|i| r.rules.remove(i));
    let test = r.rules.iter().position(|x| x.id == "test").map(|i| r.rules.remove(i));
    r.rules.retain(|x| !x.id.is_empty());
    let mut d = def.take().unwrap_or(NotifyRule { id: "default".into(), label: "Default".into(), min: 10.0, max: 29999.0, pool: false, style: NotifyCfg::default(), test_media_url: String::new() });
    d.label = if d.label.is_empty() { "Default".into() } else { d.label };
    r.rules.push(d);
    if let Some(t) = test { r.rules.push(t); }
    r
}

/// first non-pool rule (in order) whose range contains amount; otherwise the default (pool selection happens in the overlay)
pub fn pick_style(rules: &[NotifyRule], amount: f64) -> NotifyCfg {
    for r in rules {
        if r.id == "default" || r.id == "test" || r.pool { continue; }
        if amount + 1e-9 >= r.min && (r.max <= 0.0 || amount <= r.max + 1e-9) {
            return r.style.clone();
        }
    }
    rules.iter().find(|r| r.id == "default").map(|r| r.style.clone()).unwrap_or_default()
}

/// all installed font families (Windows GDI enumeration)
pub fn list_fonts() -> Vec<String> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::{EnumFontFamiliesExW, GetDC, ReleaseDC, LOGFONTW, TEXTMETRICW};
    unsafe extern "system" fn cb(lf: *const LOGFONTW, _tm: *const TEXTMETRICW, _ft: u32, lp: isize) -> i32 {
        let names = &mut *(lp as *mut Vec<String>);
        let lf = &*lf;
        let len = lf.lfFaceName.iter().position(|&c| c == 0).unwrap_or(lf.lfFaceName.len());
        let s = String::from_utf16_lossy(&lf.lfFaceName[..len]);
        let s = s.trim_start_matches('@').trim().to_string();
        if !s.is_empty() && !names.contains(&s) { names.push(s); }
        1
    }
    unsafe {
        let hdc = GetDC(null_mut());
        if hdc.is_null() { return vec![]; }
        let mut lf: LOGFONTW = std::mem::zeroed();
        lf.lfCharSet = 1; // DEFAULT_CHARSET
        let mut names: Vec<String> = Vec::new();
        EnumFontFamiliesExW(hdc, &lf, Some(cb), &mut names as *mut _ as isize, 0);
        ReleaseDC(null_mut(), hdc);
        names.sort_by_key(|a| a.to_lowercase());
        names
    }
}

fn d_qr_color() -> String { "#101217".into() }
fn d_qr_style() -> String { "both".into() }
fn d_qr_align() -> String { "bottom".into() }
fn d_qr_size() -> u32 { 100 }
fn d_qr_radius() -> u32 { 20 }
fn d_qr_accent() -> String { "#8b5cf6".into() }

/// one QR overlay: which jar it shows, goal-window background and layout style
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QrWidgetCfg {
    #[serde(default = "d_s")] pub jar: String,
    #[serde(default = "d_qr_color")] pub color: String,
    #[serde(default = "d_qr_style")] pub style: String, // "both" | "qr" | "goal"
    #[serde(default = "d_qr_radius")] pub radius: u32,
    #[serde(default = "d_qr_accent")] pub accent: String,
}
impl Default for QrWidgetCfg {
    fn default() -> Self { QrWidgetCfg { jar: String::new(), color: d_qr_color(), style: d_qr_style(), radius: d_qr_radius(), accent: d_qr_accent() } }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "d_host")] pub host: String,
    #[serde(default = "d_port")] pub port: u16,
    #[serde(default = "d_aport")] pub auction_port: u16,
    #[serde(default = "d_public_port")] pub public_port: u16,
    #[serde(default = "d_s")] pub public_base: String,
    #[serde(default = "d_s")] pub nickname: String,
    #[serde(default = "d_s")] pub overlayname: String,
    #[serde(default = "d_s")] pub stats_token: String,
    #[serde(default)] pub jars: Vec<String>,
    /// optional user-defined jar names (shown on the donation page; API title is the fallback)
    #[serde(default)] pub jar_names: Vec<String>,
    /// per-jar "my jar" flags; false = 3rd-party jar (no API, no alerts, no codes)
    #[serde(default)] pub jar_mine: Vec<bool>,
    #[serde(default = "d_s")] pub token: String,
    #[serde(default = "d_s")] pub bg: String,
    #[serde(default = "d_s")] pub bits_bg: String,
    #[serde(default = "d_s")] pub bits_page_bg: String,
    #[serde(default = "d_min")] pub min_amount: f64,
    #[serde(default = "d_max")] pub max_amount: f64,
    #[serde(default = "d_prefix")] pub code_prefix: String,
    #[serde(default = "d_true")] pub notify_unmatched: bool,
    #[serde(default = "d_poll")] pub poll_seconds: u64,
    #[serde(default = "d_idle")] pub idle_poll_seconds: u64,
    #[serde(default = "d_ttl")] pub pending_ttl_hours: u64,
    #[serde(default)] pub limits: Limits,
    #[serde(default)] pub overlay: OverlayCfg,
    #[serde(default = "default_rules")] pub notify: NotifyRules,
    #[serde(default)] pub bits: Vec<Bit>,
    #[serde(default = "d_bits_mode")] pub bits_mode: String,
    /// enabled selectors on the donation page; absent means the legacy single bits_mode
    #[serde(default)] pub bits_modes: Option<Vec<String>>,
    #[serde(default = "d_grid")] pub bits_grid: u32,
    #[serde(default)] pub timer: TimerCfg,
    #[serde(default)] pub features: Features,
    #[serde(default)] pub auction: AuctionCfg,
    #[serde(default)] pub tts: TtsCfg,
    #[serde(default)] pub streamelements: StreamElementsCfg,
    /// legacy first QR widget jar (also kept in sync with qr_widgets[0].jar)
    #[serde(default = "d_s")] pub widget_jar: String,
    /// /qr, /qr2 … /qr5 overlays (up to MAX_QR_WIDGETS)
    #[serde(default)] pub qr_widgets: Vec<QrWidgetCfg>,
    #[serde(default = "d_qr_align")] pub qr_align: String, // top | mid | bottom
    #[serde(default = "d_qr_size")] pub qr_size: u32,      // 0..=200 %

}

impl Config {
    /// selectors the donation page may offer, in a stable order
    pub fn enabled_bits_modes(&self) -> Vec<String> {
        let raw = self.bits_modes.clone().unwrap_or_else(|| vec![self.bits_mode.clone()]);
        let mut out = Vec::new();
        for m in raw {
            if ["single", "order", "adhd"].contains(&m.as_str()) && !out.contains(&m) { out.push(m); }
        }
        out
    }
}
impl Default for Config {
    fn default() -> Self {
        Config {
            host: d_host(), port: d_port(), auction_port: d_aport(), public_port: d_public_port(), public_base: d_s(), nickname: d_s(), overlayname: d_s(),
            stats_token: d_s(), jars: Vec::new(), jar_names: Vec::new(), jar_mine: Vec::new(), token: d_s(), bg: d_s(), bits_bg: d_s(), bits_page_bg: d_s(),
            min_amount: d_min(), max_amount: d_max(), code_prefix: d_prefix(),
            notify_unmatched: true, poll_seconds: d_poll(), idle_poll_seconds: d_idle(),
            pending_ttl_hours: d_ttl(), limits: Limits::default(), overlay: OverlayCfg::default(),
            notify: default_rules(), bits: Vec::new(), bits_mode: d_bits_mode(), bits_modes: None, bits_grid: d_grid(),
            timer: TimerCfg::default(),
            features: Features::default(),
            auction: AuctionCfg::default(),
            tts: TtsCfg::default(),
            streamelements: StreamElementsCfg::default(),
            widget_jar: String::new(),
            qr_widgets: Vec::new(),
            qr_align: d_qr_align(),
            qr_size: d_qr_size(),
        }
    }
}

/// text-to-speech for donation alerts (voices served by the local tts-test daemon)
fn t_mode() -> String { "google:uk".into() }   // "default" | "google:uk" | "google:us" | "custom" | "off"
fn t_voice() -> String { "google:uk".into() }   // voice id, or "" = ALL (let the donor pick within the set)
fn t_read() -> String { "both".into() }   // "message" | "announcement" | "both"
fn t_dir() -> String { "tts-engine".into() }   // relative to the exe; absolute paths still work
fn t_speed() -> f64 { 1.2 }                   // speech rate; 0.5-2.0
fn t_s() -> String { String::new() }

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TtsCfg {
    #[serde(default = "t_mode")] pub mode: String,
    #[serde(default = "t_voice")] pub voice: String,
    #[serde(default = "t_read")] pub read_mode: String,
    #[serde(default = "t_speed")] pub speed: f64,
    #[serde(default = "t_dir")] pub dir: String,
    #[serde(default = "t_s")] pub azure_key: String,
    #[serde(default = "t_s")] pub azure_region: String,
}
impl TtsCfg {
    pub fn enabled(&self) -> bool { self.mode != "off" }
    pub fn has_azure(&self) -> bool { !self.azure_key.is_empty() && !self.azure_region.is_empty() }
    /// voice ids the donor may pick for this mode (empty = no picker shown).
    /// Google UA/EN always; Azure only with keys; StyleTTS2 only when its model exists.
    pub fn allowed(&self) -> Vec<String> {
        self.allowed_in(&self.dir)
    }
    /// This build intentionally uses one voice only: Google Ukrainian.
    pub fn allowed_in(&self, _dir: &str) -> Vec<String> {
        if self.mode == "off" { vec![] } else { vec!["google:uk".into()] }
    }
    pub fn donor_picks(&self) -> bool { false }
    pub fn default_voice(&self) -> String {
        if self.mode == "off" { String::new() } else { "google:uk".into() }
    }
}
impl Default for TtsCfg {
    fn default() -> Self {
        TtsCfg { mode: t_mode(), voice: t_voice(), read_mode: t_read(), speed: t_speed(), dir: t_dir(), azure_key: t_s(), azure_region: t_s() }
    }
}

/// clean a jar code the user typed (accepts a full link too)
pub fn clean_jar_code(s: &str) -> String {
    let s = s.trim();
    let s = s.rsplit('/').next().unwrap_or(s);      // take the last path segment
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

pub fn normalize_jars(jars: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for j in jars {
        let c = clean_jar_code(j);
        if !c.is_empty() && !out.contains(&c) { out.push(c); }
        if out.len() >= MAX_JARS { break; }
    }
    out
}

/// Secrets are kept per-instance, keyed by the port that owns them, so one process can serve
/// two ports (e.g. 8787 donations + 8788 auction) each with its own token + Twitch app.
static SECRETS_PORT: AtomicU16 = AtomicU16::new(0);
pub fn set_secrets_port(p: u16) { SECRETS_PORT.store(p, Ordering::Relaxed); }
fn secrets_dir_at(port: u16) -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("LocalMonoDonate").join(if port == 0 { "default".to_string() } else { port.to_string() })
}
pub fn token_file_at(port: u16) -> PathBuf { secrets_dir_at(port).join("token") }
pub fn load_token_at(port: u16) -> String { fs::read_to_string(token_file_at(port)).map(|s| s.trim().to_string()).unwrap_or_default() }
/// one-time move of the old un-keyed secrets into the port-keyed folder
pub fn migrate_secrets(port: u16) {
    let base = secrets_dir_at(0).parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
    let new = base.join(port.to_string());
    if new.exists() { return; }
    let (ot, ow) = (base.join("token"), base.join("twitch.json"));
    if !ot.exists() && !ow.exists() { return; }
    let _ = fs::create_dir_all(&new);
    if ot.exists() { let _ = fs::rename(&ot, new.join("token")); }
    if ow.exists() { let _ = fs::rename(&ow, new.join("twitch.json")); }
}
/// copy the per-port secret files forward after the listening port changed
pub fn migrate_port_secrets(from: u16, to: u16) {
    if from == to { return; }
    let base = secrets_dir_at(0).parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
    let src = base.join(from.to_string());
    let dst = base.join(to.to_string());
    if !src.exists() { return; }
    for name in ["token", "twitch.json", "streamelements.json"] {
        let s = src.join(name);
        let d = dst.join(name);
        if s.exists() && !d.exists() {
            let _ = fs::create_dir_all(&dst);
            let _ = fs::copy(&s, &d);
            log(&format!("secrets: {} copied from port {}", name, from));
        }
    }
}

pub fn save_token_at(port: u16, t: &str) {
    let p = token_file_at(port);
    if let Some(d) = p.parent() { let _ = fs::create_dir_all(d); }
    let _ = fs::write(&p, t.trim());
}

/// Twitch secrets (client id/secret + OAuth tokens) are also kept per-port, outside the project.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TwitchSecrets {
    #[serde(default)] pub client_id: String,
    #[serde(default)] pub client_secret: String,
    #[serde(default)] pub access_token: String,
    #[serde(default)] pub refresh_token: String,
    #[serde(default)] pub broadcaster_id: String,
    #[serde(default)] pub login: String,
}
pub fn twitch_file_at(port: u16) -> PathBuf { secrets_dir_at(port).join("twitch.json") }
pub fn load_twitch_at(port: u16) -> TwitchSecrets {
    fs::read(twitch_file_at(port)).ok()
        .and_then(|b| serde_json::from_slice::<TwitchSecrets>(&b).ok())
        .unwrap_or_default()
}
pub fn save_twitch_at(port: u16, s: &TwitchSecrets) {
    let p = twitch_file_at(port);
    if let Some(d) = p.parent() { let _ = fs::create_dir_all(d); }
    if let Ok(j) = serde_json::to_string_pretty(s) { let _ = fs::write(&p, j); }
}

/// StreamElements: public settings in config.json, the JWT in the per-port secrets file.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamElementsCfg {
    #[serde(default)] pub enabled: bool,
    /// StreamElements account (channel) id, e.g. 5f2de5dd9a474a2c2aaaaaaa
    #[serde(default)] pub account_id: String,
}
impl Default for StreamElementsCfg {
    fn default() -> Self { StreamElementsCfg { enabled: false, account_id: String::new() } }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct StreamElementsSecrets {
    #[serde(default)] pub token: String,
}
pub fn streamelements_file_at(port: u16) -> PathBuf { secrets_dir_at(port).join("streamelements.json") }
pub fn load_streamelements_at(port: u16) -> StreamElementsSecrets {
    fs::read(streamelements_file_at(port)).ok()
        .and_then(|b| serde_json::from_slice::<StreamElementsSecrets>(&b).ok())
        .unwrap_or_default()
}
pub fn save_streamelements_at(port: u16, s: &StreamElementsSecrets) {
    let p = streamelements_file_at(port);
    if let Some(d) = p.parent() { let _ = fs::create_dir_all(d); }
    if let Ok(j) = serde_json::to_string_pretty(s) { let _ = fs::write(&p, j); }
}

// global-port wrappers (the main port) used by the config/token flow
pub fn load_token() -> String { load_token_at(SECRETS_PORT.load(Ordering::Relaxed)) }
pub fn save_token(t: &str) { save_token_at(SECRETS_PORT.load(Ordering::Relaxed), t); }

pub fn load_config(paths: &Paths) -> Config {
    let p = paths.config();
    let mut cfg = if p.exists() {
        let raw = fs::read(&p).unwrap_or_default();
        let raw = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) { &raw[3..] } else { &raw[..] };
        match serde_json::from_slice::<serde_json::Value>(raw) {
            Ok(mut v) => {
                // migrate old single jarId -> jars[]
                if v.get("jars").is_none() {
                    if let Some(j) = v.get("jarId").and_then(|x| x.as_str()) {
                        v["jars"] = serde_json::json!([j]);
                    }
                }
                // migrate old flat notify -> single default rule
                if let Some(n) = v.get("notify") {
                    if n.is_object() && n.get("rules").is_none() {
                        let style = n.clone();
                        v["notify"] = serde_json::json!({ "rules": [ { "id": "default", "label": "Default", "min": 0.0, "max": 0.0, "style": style } ] });
                    }
                }
                serde_json::from_value::<Config>(v).unwrap_or_default()
            }
            Err(e) => {
                log(&format!("config.json NOT parsed ({}); refusing to overwrite it. Fix the file and restart.", e));
                return Config::default();
            }
        }
    } else {
        Config::default()
    };
    if cfg.nickname.trim().is_empty() { cfg.nickname = "mystream".into(); }
    // stats code is created only when the user asks for it (Setup -> Stats -> ↻)
    cfg.jars = normalize_jars(&cfg.jars);
    cfg.notify = normalize_rules(cfg.notify);
    set_secrets_port(cfg.port);
    migrate_secrets(cfg.port);
    // the token lives in a per-instance file outside the project; migrate any value still in config.json
    let ext = load_token();
    if !ext.is_empty() { cfg.token = ext; }
    else if !cfg.token.is_empty() { save_token(&cfg.token); }
    save_config(paths, &cfg);
    cfg
}

pub fn save_config(paths: &Paths, cfg: &Config) {
    // never write the Monobank token into the project's config.json
    let mut c = cfg.clone();
    c.token = String::new();
    if let Ok(s) = serde_json::to_string_pretty(&c) { let _ = fs::write(paths.config(), s); }
}

/* ---------------------------------------------------------------- state */

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub code: String,
    pub amount: f64,
    pub name: String,
    pub message: String,
    #[serde(default)] pub youtube_id: String,
    #[serde(default)] pub media_url: String,
    #[serde(default)] pub voice: String,
    /// bit chosen on the donation form (plays on the bits overlay instead of a normal alert)
    #[serde(default)] pub bit: Option<Bit>,
    /// several sound alerts picked on the donation form (order/adhd modes)
    #[serde(default)] pub bits: Vec<Bit>,
    #[serde(default)] pub bits_mode: String,
    /// fallback-link twin code; both are consumed by the first matching payment
    #[serde(default)] pub pair: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub name: String,
    pub amount: f64,
    pub message: String,
    #[serde(default)] pub youtube_id: String,
    #[serde(default)] pub media_url: String,
    pub matched: String,
    #[serde(default)] pub bit_label: String,
    #[serde(default)] pub bit_code: String,
    #[serde(default)] pub code: Option<String>,
    pub at: i64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Stats {
    #[serde(default)] pub total: f64,
    #[serde(default)] pub count: u64,
    #[serde(default)] pub biggest: f64,
}

/// a "bit" — a purchasable button on the bits deck (sound, video, or an existing alert)
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Bit {
    #[serde(default)] pub slot: u32,
    #[serde(default = "d_s")] pub label: String,
    #[serde(default = "d_s")] pub kind: String,      // "sound" | "video" | "image" | "alert"
    #[serde(default = "d_s")] pub url: String,        // media url for sound/video/image
    #[serde(default = "d_s")] pub sound: String,      // optional sound played alongside an image bit
    #[serde(default = "d_s")] pub img: String,        // button image for the deck
    #[serde(default = "d_s")] pub rule_id: String,    // referenced alert rule id
    #[serde(default)] pub price: f64,
    #[serde(default = "d_s")] pub code: String,
}

/// all per-jar data
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct JarState {
    pub code: String,
    #[serde(default)] pub title: String,
    #[serde(default = "uah_code")] pub currency: i64,
    #[serde(default)] pub balance: i64,
    #[serde(default)] pub goal: i64,
    #[serde(default)] pub statement_account: String,
    #[serde(default)] pub last_ts: i64,
    #[serde(default)] pub seen: Vec<String>,
    #[serde(default)] pub backfilled: bool,
    #[serde(default)] pub pending: Vec<Pending>,
    #[serde(default)] pub history: Vec<HistoryItem>,
    #[serde(default)] pub stats: Stats,
    #[serde(default)] pub issued: Vec<String>,
    #[serde(default)] pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct State {
    #[serde(default)] pub jars: Vec<JarState>,
    #[serde(default)] pub timer: TimerState,
    #[serde(default)] pub auction: AuctionState,
    #[serde(default)] pub auc: crate::auction::AuctionState,
}

pub fn load_state(paths: &Paths, codes: &[String]) -> State {
    let raw = fs::read(paths.data()).ok();
    if let Some(raw) = raw {
        let raw = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) { raw[3..].to_vec() } else { raw.clone() };
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&raw) {
            if v.get("jars").is_some() {
                if let Ok(s) = serde_json::from_value::<State>(v) { return ensure_jars(s, codes); }
            } else if v.get("history").is_some() || v.get("pending").is_some() {
                // migrate the old flat state into jar #1
                let code = codes.first().cloned().unwrap_or_default();
                let legacy: LegacyState = serde_json::from_value(v).unwrap_or_default();
                let (lt, lb, lg) = legacy.jar.map(|j| (j.title, j.balance, j.goal)).unwrap_or_default();
                let mut s = State::default();
                s.jars.push(JarState {
                    code, title: lt, balance: lb, goal: lg, currency: 980,
                    statement_account: legacy.statement_account,
                    last_ts: legacy.last_ts,
                    seen: legacy.seen, pending: legacy.pending,
                    history: legacy.history, stats: legacy.stats, issued: Vec::new(), updated_at: now_ms(),
                    backfilled: false,
                });
                return ensure_jars(s, codes);
            }
        }
    }
    ensure_jars(State::default(), codes)
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LegacyState {
    #[serde(default)] pending: Vec<Pending>,
    #[serde(default)] history: Vec<HistoryItem>,
    #[serde(default)] seen: Vec<String>,
    #[serde(default)] last_ts: i64,
    #[serde(default)] jar: Option<LegacyJar>,
    #[serde(default)] statement_account: String,
    #[serde(default)] stats: Stats,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LegacyJar { #[serde(default)] title: String, #[serde(default)] balance: i64, #[serde(default)] goal: i64 }

/// make sure State has exactly one JarState per configured code (preserving data)
fn ensure_jars(mut s: State, codes: &[String]) -> State {
    let mut out: Vec<JarState> = Vec::new();
    for c in codes {
        if let Some(pos) = s.jars.iter().position(|j| j.code == *c) {
            out.push(s.jars.remove(pos));
        } else {
            out.push(JarState { code: c.clone(), ..Default::default() });
        }
    }
    s.jars = out;
    s
}

pub fn write_state(paths: &Paths, st: &State) {
    if let Ok(s) = serde_json::to_string_pretty(st) {
        let tmp = paths.data().with_extension("json.tmp");
        if fs::write(&tmp, s).is_ok() { let _ = fs::rename(&tmp, paths.data()); }
    }
}

/* ---------------------------------------------------------------- codes */

pub fn gen_stats_token() -> String {
    let rng = rand::thread_rng();
    rng.sample_iter(&Alphanumeric).take(12).map(|c| c as char).collect()
}

pub fn gen_bit_code() -> String {
    let rng = rand::thread_rng();
    rng.sample_iter(&Alphanumeric).take(12).map(|c| c as char).collect()
}

pub fn gen_code(_prefix: &str, issued: &[String], pending: &[Pending]) -> String {
    loop {
        let rng = rand::thread_rng();
        let code: String = rng.sample_iter(&Alphanumeric).take(12).map(|c| c as char).collect();
        if !issued.contains(&code) && !pending.iter().any(|p| p.code == code) { return code; }
    }
}

/// prefix that precedes the code in the Monobank payment comment
pub const DONATE_PREFIX: &str = "донат сповіщення ";
pub fn donate_comment(flags: &str, code: &str) -> String {
    if flags.is_empty() { format!("{}{}", DONATE_PREFIX, code) }
    else { format!("{}{} {}", DONATE_PREFIX, flags, code) }
}

/* ---------------------------------------------------------------- misc */

/// Accepts every common YouTube URL shape (with or without scheme, any subdomain)
/// and a bare 11-character id. Returns Ok("") for an empty input (the field is optional).
pub fn youtube_id(input: &str) -> Result<String, ()> {
    let raw = input.trim();
    if raw.is_empty() { return Ok(String::new()); }
    if is_youtube_id(raw) { return Ok(raw.to_string()); }

    let s = raw.strip_prefix("https://").or_else(|| raw.strip_prefix("http://")).unwrap_or(raw);
    let s = s.strip_prefix("//").unwrap_or(s);
    let (host_raw, rest) = match s.find('/') { Some(i) => (&s[..i], &s[i..]), None => (s, "") };
    let host = host_raw.rsplit('@').next().unwrap_or(host_raw);
    let host = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    let host = host.trim_start_matches("www.").trim_start_matches("m.");

    let rest = rest.split('#').next().unwrap_or("");
    let (path, query) = match rest.split_once('?') { Some((p, q)) => (p, q), None => (rest, "") };
    let segs: Vec<&str> = path.trim_start_matches('/').split('/').filter(|x| !x.is_empty()).collect();

    // youtu.be/ID, yt.be/ID, www.youtu.be/ID
    if host == "youtu.be" || host == "yt.be" {
        if let Some(id) = segs.first() { if is_youtube_id(id) { return Ok((*id).to_string()); } }
    }

    // ?v=ID wins anywhere: /watch, /watch_popup, music., tv., international TLDs
    for pair in query.split('&') {
        let (k, v) = match pair.split_once('=') { Some(kv) => kv, None => (pair, "") };
        if k.eq_ignore_ascii_case("v") && is_youtube_id(v) { return Ok(v.to_string()); }
    }

    let youtube_host = host.split('.').any(|l| l == "youtube" || l == "youtube-nocookie");
    let ytimg_host = host.split('.').any(|l| l == "ytimg");
    if !youtube_host && !ytimg_host { return Err(()); }

    // /embed/, /shorts/, /live/, /v/, /e/
    if let Some(first) = segs.first() {
        let key = first.to_ascii_lowercase();
        if ["embed", "shorts", "live", "v", "e"].contains(&key.as_str()) {
            if let Some(id) = segs.get(1) { if is_youtube_id(id) { return Ok((*id).to_string()); } }
        }
        // thumbnails: i.ytimg.com/vi/<id>/..., /vi_webp/, /vi_live/
        if ytimg_host && ["vi", "vi_webp", "vi_live"].contains(&key.as_str()) {
            if let Some(id) = segs.get(1) { if is_youtube_id(id) { return Ok((*id).to_string()); } }
        }
    }
    Err(())
}
/// Accept a public video/page URL from a known non-YouTube provider.
pub fn other_media_url(input: &str) -> Option<String> {
    media_provider(input).map(|_| input.trim().to_string())
}

/// Classify an allowed non-YouTube media URL by provider.
pub fn media_provider(input: &str) -> Option<&'static str> {
    let raw = input.trim();
    if raw.is_empty() { return None; }
    let s = raw.strip_prefix("https://").or_else(|| raw.strip_prefix("http://")).unwrap_or(raw);
    let s = s.strip_prefix("//").unwrap_or(s);
    let host = s.split('/').next().unwrap_or("").rsplit('@').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    let host = host.trim_start_matches("www.").trim_start_matches("m.");
    if host == "tiktok.com" || host.ends_with(".tiktok.com") { return Some("tiktok"); }
    if host == "instagram.com" || host.ends_with(".instagram.com") || host == "instagr.am" { return Some("instagram"); }
    if host == "twitch.tv" || host.ends_with(".twitch.tv") { return Some("twitch"); }
    if host == "v.redd.it" || host == "packaged-media.redd.it" { return Some("reddit"); }
    if host == "x.com" || host.ends_with(".x.com") || host == "twitter.com" || host.ends_with(".twitter.com") { return Some("x"); }
    None
}

pub fn is_youtube_id(s: &str) -> bool {
    s.len() == 11 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod qr_path_tests {
    use super::*;

    #[test]
    fn only_qr_and_qr2_to_qr5_are_widget_routes() {
        assert_eq!(qr_path_index("/qr"), Some(0));
        assert_eq!(qr_path_index("/qr2"), Some(1));
        assert_eq!(qr_path_index("/qr5"), Some(4));
        assert_eq!(qr_path_index("/qr6"), None);
        assert_eq!(qr_path_index("/qrcode.js"), None);
        assert_eq!(qr_path_index("/qr/"), None);
    }
}

pub fn sanitize_name(s: &str) -> String { s.trim().to_ascii_lowercase() }

/// Grouping key: the nickname itself, trimmed and case-insensitive - nothing else.
/// "goodfeelings" and "goodfeelings2" are two donors even with the same card/email; every donation
/// under "grafdralkuma" is one total no matter who or what paid it.
pub fn donor_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// top donors all-time, grouped by donor_key: (display name, total, count), highest first
pub fn top_donators(history: &[HistoryItem]) -> Vec<(String, f64, u64)> {
    use std::collections::HashMap;
    // (display name, total, count, first donation time)
    let mut map: HashMap<String, (String, f64, u64, i64)> = HashMap::new();
    for h in history {
        let k = donor_key(&h.name);
        if k.is_empty() { continue; }
        let e = map.entry(k).or_insert((h.name.clone(), 0.0, 0, h.at));
        e.1 += h.amount;
        e.2 += 1;
        if h.at < e.3 { e.3 = h.at; }
        if h.name.chars().count() < e.0.chars().count() { e.0 = h.name.clone(); }
    }
    let mut v: Vec<(String, f64, u64, i64)> = map.into_values().collect();
    // equal totals: the donor who reached the amount first (oldest donation) stays on top.
    // Without this tie-break HashMap order made tied rows swap on every 4 s state poll.
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.3.cmp(&b.3)));
    v.truncate(20);
    v.into_iter().map(|(n, t, c, _)| (n, t, c)).collect()
}

#[cfg(test)]
mod top_donators_tests {
    use super::*;

    fn h(name: &str, amount: f64, at: i64) -> HistoryItem {
        HistoryItem {
            id: String::new(), name: name.into(), amount, message: String::new(),
            youtube_id: String::new(), media_url: String::new(), bit_label: String::new(), bit_code: String::new(), matched: "code".into(), code: None, at,
        }
    }

    #[test]
    fn equal_totals_keep_the_older_donor_first() {
        let history = vec![h("new", 100.0, 2_000), h("old", 100.0, 1_000)];
        for _ in 0..20 { // HashMap order must not matter
            let top = top_donators(&history);
            assert_eq!(top[0].0, "old");
            assert_eq!(top[1].0, "new");
        }
    }

    #[test]
    fn bigger_total_wins_over_age() {
        let history = vec![h("old", 50.0, 1_000), h("big", 500.0, 5_000)];
        let top = top_donators(&history);
        assert_eq!(top[0].0, "big");
    }
}

/// (donate, stats, overlay) URLs; *.localhost resolves to loopback in browsers.
pub fn links(cfg: &Config) -> (String, String, String) {
    let p = cfg.port;
    let n = sanitize_name(&cfg.nickname);
    (
        format!("http://{}.donate.localhost:{}/banka", n, p),
        if cfg.stats_token.is_empty() { String::new() } else { format!("http://{}.{}.localhost:{}", n, cfg.stats_token, p) },
        format!("http://{}.{}.localhost:{}/main", n, sanitize_name(&cfg.overlayname), p),
    )
}

/// QR + jar widget overlay (OBS browser source); index 0 = /qr, 1 = /qr2, …
pub fn qr_link_at(cfg: &Config, index: usize) -> String {
    let suffix = if index == 0 { String::new() } else { (index + 1).to_string() };
    format!("http://{}.{}.localhost:{}/qr{}",
        sanitize_name(&cfg.nickname), sanitize_name(&cfg.overlayname), cfg.port, suffix)
}

pub fn qr_link(cfg: &Config) -> String { qr_link_at(cfg, 0) }

/// `/qr` -> 0, `/qr2` -> 1 … `/qr5` -> 4
pub fn qr_path_index(path: &str) -> Option<usize> {
    let rest = path.strip_prefix("/qr")?;
    if rest.is_empty() { return Some(0); }
    let n: usize = rest.parse().ok()?;
    if (2..=MAX_QR_WIDGETS).contains(&n) { Some(n - 1) } else { None }
}

/// (bits donor page, bits overlay)
pub fn bits_links(cfg: &Config) -> (String, String) {
    let p = cfg.port;
    let n = sanitize_name(&cfg.nickname);
    let o = sanitize_name(&cfg.overlayname);
    (
        format!("http://{}.donate.localhost:{}/bits", n, p),
        format!("http://{}.{}.localhost:{}/bits", n, o, p),
    )
}

/// Same-origin TTS URL: HTTPS when the overlay page is served through Caddy.
pub fn tts_url(_cfg: &Config, fname: &str) -> String { format!("/tts/{}", fname) }

/// youtube overlay (plays donated videos in OBS)
pub fn youtube_link(cfg: &Config) -> String {
    let p = cfg.port;
    let n = sanitize_name(&cfg.nickname);
    let o = sanitize_name(&cfg.overlayname);
    format!("http://{}.{}.localhost:{}/youtube", n, o, p)
}

/// marathon timer overlay (OBS browser source)
pub fn timer_link(cfg: &Config) -> String {
    let p = cfg.port;
    let n = sanitize_name(&cfg.nickname);
    let o = sanitize_name(&cfg.overlayname);
    format!("http://{}.{}.localhost:{}/timer", n, o, p)
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Page { Donor, Stats, Overlay, Qr, QrEdit, Setup, Notify, BitsOverlay, BitsDonor, BitsEdit, DonateEdit, YouTube, TimerAdmin, TimerOverlay, AuctionUi, AuctionSettings }

pub fn route_by_host(cfg: &Config, host: &str, path: &str) -> Option<Page> {
    if path == "/setup" { return Some(Page::Setup); }
    if path == "/qredit" { return Some(Page::QrEdit); }
    if path == "/auctionsettings" { return Some(Page::AuctionSettings); }
    if path == "/notify" { return Some(Page::Notify); }
    if path == "/bitsedit" { return Some(Page::BitsEdit); }
    if path == "/donateedit" { return Some(Page::DonateEdit); }
    if path == "/timeradmin" { return Some(Page::TimerAdmin); }
    let f = &cfg.features;
    let on = |b: bool, p: Page| if b { Some(p) } else { None };
    let h = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    if h.is_empty() || h == "127.0.0.1" || h == "localhost" || h == "0.0.0.0" {
        if qr_path_index(path).is_some() { return on(f.qr, Page::Qr); }
        return match path {
            "/banka" | "/donatiks" | "/form" => on(f.donate, Page::Donor),
            "/stats" | "/check" | "/" => on(f.stats, Page::Stats),
            "/main" | "/overlay" => on(f.overlay, Page::Overlay),
            "/bits" => on(f.bits_overlay, Page::BitsOverlay),
            "/bitsform" => on(f.bits, Page::BitsDonor),
            "/youtube" | "/yt" => on(f.youtube, Page::YouTube),
            "/timer" => on(f.timer, Page::TimerOverlay),
            "/auction" => on(f.auction, Page::AuctionUi),
            _ => None,
        };
    }
    let labels: Vec<&str> = h.split('.').collect();
    // every host-scoped page must start with the configured nickname - otherwise an old
    // link (e.g. "mystream.donate.localhost") keeps working after the nick changes
    let nick = sanitize_name(&cfg.nickname);
    if nick.is_empty() || labels.first().copied().unwrap_or("") != nick { return None; }
    let page = labels.get(1).copied().unwrap_or("");
    if page == "auction" { return on(f.auction, Page::AuctionUi); }
    if page == "donate" { if path == "/bits" { return on(f.bits, Page::BitsDonor); } return on(f.donate, Page::Donor); }
    if page == sanitize_name(&cfg.overlayname) && !cfg.overlayname.trim().is_empty() {
        if qr_path_index(path).is_some() { return on(f.qr, Page::Qr); }
        if path == "/bits" { return on(f.bits_overlay, Page::BitsOverlay); }
        if path == "/youtube" || path == "/yt" { return on(f.youtube, Page::YouTube); }
        if path == "/timer" { return on(f.timer, Page::TimerOverlay); }
        if path == "/auction" { return on(f.auction, Page::AuctionUi); }
        return on(f.overlay, Page::Overlay);
    }
    if !cfg.stats_token.is_empty() && (page == cfg.stats_token.to_ascii_lowercase() || page == "codegenerate") { return on(f.stats, Page::Stats); }
    None
}

/// public (tunnel-facing) routes: path-based only — no admin, no overlays, no stats
pub fn route_public(cfg: &Config, path: &str) -> Option<Page> {
    route_public_exact(cfg, path).or_else(|| {
        // allow a leading /<nickname> segment: /<nick>/banka -> /banka
        let rest = path.strip_prefix('/')?;
        let (_seg, tail) = rest.split_once('/')?;
        route_public_exact(cfg, &format!("/{}", tail))
    })
}

fn route_public_exact(cfg: &Config, path: &str) -> Option<Page> {
    let f = &cfg.features;
    let on = |b: bool, p: Page| if b { Some(p) } else { None };
    match path {
        "/donation" => on(f.donate, Page::Donor),
        "/soundalerts" => on(f.bits, Page::BitsDonor),
        _ => None,
    }
}

static LOGFILE: std::sync::OnceLock<std::sync::Mutex<std::fs::File>> = std::sync::OnceLock::new();

pub fn init_logging(base: &Paths) {
    if let Ok(f) = std::fs::OpenOptions::new().create(true).append(true).open(base.base.join("LocalMonoDonate.log")) {
        let _ = LOGFILE.set(std::sync::Mutex::new(f));
    }
}

pub fn log(msg: &str) {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let (h, m, s) = unsafe {
        let mut st = std::mem::zeroed();
        GetLocalTime(&mut st);
        (st.wHour, st.wMinute, st.wSecond)
    };
    let line = format!("[{:02}:{:02}:{:02}] {}", h, m, s, msg);
    { use std::io::Write; let _ = writeln!(std::io::stdout(), "{}", line); }
    if let Some(f) = LOGFILE.get() {
        if let Ok(mut f) = f.lock() { use std::io::Write; let _ = writeln!(f, "{}", line); }
    }
}



#[cfg(test)]
mod youtube_id_tests {
    use super::*;

    const ID: &str = "dQw4w9WgXcQ";

    #[test]
    fn accepts_every_common_youtube_shape() {
        let cases = [
            ID,
            "https://youtube.com/watch?v=dQw4w9WgXcQ",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=10s",
            "http://m.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=RD",
            "https://youtu.be/dQw4w9WgXcQ?t=1",
            "https://www.youtu.be/dQw4w9WgXcQ",
            "https://yt.be/dQw4w9WgXcQ",
            "https://youtube.com/shorts/dQw4w9WgXcQ",
            "https://www.youtube.com/live/dQw4w9WgXcQ?si=x",
            "https://www.youtube.com/embed/dQw4w9WgXcQ",
            "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ",
            "https://youtube.com/v/dQw4w9WgXcQ",
            "https://youtube.com/e/dQw4w9WgXcQ",
            "https://youtube.com/watch_popup?v=dQw4w9WgXcQ",
            "www.youtube.com/watch?v=dQw4w9WgXcQ",
            "youtube.com/shorts/dQw4w9WgXcQ",
            "https://youtube.googleapis.com/v/dQw4w9WgXcQ",
            "https://www.youtubekids.com/watch?v=dQw4w9WgXcQ",
            "https://tv.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://youtube.com/watch?v=dQw4w9WgXcQ&t=120s",
            "https://youtu.be/dQw4w9WgXcQ?t=120",
            "https://youtube.com/embed/dQw4w9WgXcQ?start=120",
            "https://youtu.be/dQw4w9WgXcQ?si=SHARE_TOKEN",
            "https://youtube.com/watch?v=dQw4w9WgXcQ&feature=shared",
            "https://youtube.com/shorts/dQw4w9WgXcQ?si=SHARE_TOKEN",
            "https://youtube.com/watch?v=dQw4w9WgXcQ&list=PL123",
            "https://youtube.co.uk/watch?v=dQw4w9WgXcQ",
            "https://www.youtube.co.jp/watch?v=dQw4w9WgXcQ",
            "https://youtube.co.kr/watch?v=dQw4w9WgXcQ",
            "https://youtube.co.in/watch?v=dQw4w9WgXcQ",
            "https://youtube.co.nz/watch?v=dQw4w9WgXcQ",
            "https://youtube.co.za/watch?v=dQw4w9WgXcQ",
            "https://youtube.co.id/watch?v=dQw4w9WgXcQ",
            "https://youtube.co.th/watch?v=dQw4w9WgXcQ",
            "https://youtube.co.il/watch?v=dQw4w9WgXcQ",
            "https://youtube.co.ae/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.br/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.au/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.mx/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.ar/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.tr/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.hk/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.sg/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.tw/watch?v=dQw4w9WgXcQ",
            "https://youtube.de/watch?v=dQw4w9WgXcQ",
            "https://youtube.fr/watch?v=dQw4w9WgXcQ",
            "https://youtube.it/watch?v=dQw4w9WgXcQ",
            "https://youtube.es/watch?v=dQw4w9WgXcQ",
            "https://youtube.ca/watch?v=dQw4w9WgXcQ",
            "https://youtube.be/watch?v=dQw4w9WgXcQ",
            "https://youtube.nl/watch?v=dQw4w9WgXcQ",
            "https://youtube.pl/watch?v=dQw4w9WgXcQ",
            "https://youtube.ru/watch?v=dQw4w9WgXcQ",
            "https://youtube.ua/watch?v=dQw4w9WgXcQ",
            "https://youtube.ua/shorts/dQw4w9WgXcQ",
            "https://youtube.ua/embed/dQw4w9WgXcQ",
            "https://youtube.ua/live/dQw4w9WgXcQ",
            "https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg",
            "https://ytimg.com/vi_webp/dQw4w9WgXcQ/maxres.webp",
        ];
        for input in cases {
            assert_eq!(youtube_id(input), Ok(ID.to_string()), "input: {input}");
        }
    }

    #[test]
    fn accepts_the_shared_youtube_variants() {
        for (input, id) in [
            ("https://www.youtube.com/watch?v=XqZsoesa55w", "XqZsoesa55w"),
            ("https://youtube.com/watch?v=XqZsoesa55w", "XqZsoesa55w"),
            ("https://m.youtube.com/watch?v=XqZsoesa55w", "XqZsoesa55w"),
            ("https://youtu.be/XqZsoesa55w", "XqZsoesa55w"),
            ("https://www.youtube.com/shorts/XqZsoesa55w", "XqZsoesa55w"),
            ("https://www.youtube.com/embed/XqZsoesa55w", "XqZsoesa55w"),
            ("https://www.youtube-nocookie.com/embed/XqZsoesa55w", "XqZsoesa55w"),
            ("https://www.youtube.com/watch?v=XqZsoesa55w&t=30s", "XqZsoesa55w"),
            ("https://www.youtube.com/watch?v=XqZsoesa55w&feature=share", "XqZsoesa55w"),
            ("https://youtu.be/XqZsoesa55w?t=30", "XqZsoesa55w"),
            ("https://www.youtube.com/watch?app=desktop&v=XqZsoesa55w", "XqZsoesa55w"),
            ("https://www.youtube.com/v/XqZsoesa55w", "XqZsoesa55w"),
            ("https://www.youtube.com/shorts/iEHkJRECAjk", "iEHkJRECAjk"),
            ("https://youtube.com/shorts/iEHkJRECAjk", "iEHkJRECAjk"),
            ("https://m.youtube.com/shorts/iEHkJRECAjk", "iEHkJRECAjk"),
            ("https://www.youtube.com/watch?v=iEHkJRECAjk", "iEHkJRECAjk"),
            ("https://m.youtube.com/watch?v=iEHkJRECAjk", "iEHkJRECAjk"),
            ("https://youtu.be/iEHkJRECAjk", "iEHkJRECAjk"),
            ("https://www.youtube.com/embed/iEHkJRECAjk", "iEHkJRECAjk"),
            ("https://www.youtube-nocookie.com/embed/iEHkJRECAjk", "iEHkJRECAjk"),
            ("https://www.youtube.com/shorts/iEHkJRECAjk?feature=share", "iEHkJRECAjk"),
            ("https://youtube.com/shorts/iEHkJRECAjk?si=example", "iEHkJRECAjk"),
            ("https://youtu.be/iEHkJRECAjk?feature=shared", "iEHkJRECAjk"),
        ] {
            assert_eq!(youtube_id(input), Ok(id.to_string()), "input: {input}");
        }
    }

    #[test]
    fn accepts_the_listed_non_youtube_hosts() {
        for input in [
            "https://www.reddit.com/r/Unexpected/comments/jpnav3/",
            "https://redd.it/8v3dw1",
            "https://reddit.com/r/videos/comments/8v3dw1/",
        ] { assert!(other_media_url(input).is_none(), "reddit posts must be rejected: {input}"); }
        for input in [
            "https://www.instagram.com/reel/C_bnaTAO2Ll/",
            "https://instagram.com/reel/C_bnaTAO2Ll/",
            "https://www.instagram.com/reel/C_bnaTAO2Ll/?utm_source=ig_web_copy_link",
            "https://www.instagram.com/reel/C_bnaTAO2Ll/?hl=en",
            "https://www.instagram.com/reel/C_bnaTAO2Ll/embed/",
            "https://instagr.am/reel/C_bnaTAO2Ll/",
            "https://www.instagram.com/p/CZVAueAP-U3/",
            "https://www.instagram.com/p/CZVAueAP-U3/?igsh=dHEwZ3VjajRvcm5k",
            "https://www.instagram.com/tv/CWHDfowBrvV/",
            "https://www.tiktok.com/@zachking/video/6768504823336815877",
            "https://m.tiktok.com/v/6768504823336815877.html",
            "https://www.tiktok.com/embed/v2/6768504823336815877",
            "https://clips.twitch.tv/RepleteEnthusiasticMageCeilingCat",
            "https://www.twitch.tv/s1mple/clip/RepleteEnthusiasticMageCeilingCat",
            "https://m.twitch.tv/s1mple/clip/RepleteEnthusiasticMageCeilingCat",
            "https://clips.twitch.tv/embed?clip=RepleteEnthusiasticMageCeilingCat&parent=example.com",
            "https://v.redd.it/yfdajqud3sx51",
            "https://v.redd.it/yfdajqud3sx51/DASH_720.mp4?source=fallback",
            "https://packaged-media.redd.it/abc123/pb/mp4/720/video.mp4?source=fallback",
            "https://x.com/NASA/status/1446120853117177872",
            "https://mobile.twitter.com/NASA/status/1446120853117177872",
            "https://x.com/i/status/1446120853117177872",
            "https://twitter.com/NASA/status/1446120853117177872/video/1",
            "https://x.com/NASA/status/1446120853117177872?s=20",
        ] {
            assert!(other_media_url(input).is_some(), "input: {input}");
        }
    }

    #[test]
    fn classifies_each_allowed_media_provider() {
        assert_eq!(media_provider("https://www.instagram.com/reel/C_bnaTAO2Ll/"), Some("instagram"));
        assert_eq!(media_provider("https://www.tiktok.com/@x/video/1"), Some("tiktok"));
        assert_eq!(media_provider("https://clips.twitch.tv/abc"), Some("twitch"));
        assert_eq!(media_provider("https://v.redd.it/abc"), Some("reddit"));
        assert_eq!(media_provider("https://x.com/NASA/status/1"), Some("x"));
        assert_eq!(media_provider("https://www.reddit.com/r/test/comments/abc/"), None);
    }

    #[test]
    fn media_toggles_default_on_and_can_be_disabled() {
        let mut style = NotifyCfg::default();
        assert!(style.media_enabled("reddit"));
        style.media_reddit = false;
        assert!(!style.media_enabled("reddit"));
        assert!(style.media_enabled("x"));
    }

    #[test]
    fn rejects_links_without_a_single_video() {
        for input in [
            "https://www.youtube.com/playlist?list=PL123",
            "https://www.youtube.com/clip/Ugkx123",
            "https://www.youtube.com/@SomeChannel",
            "https://www.youtube.com/@SomeChannel/live",
            "https://www.youtube.com/@SomeChannel/shorts",
            "https://www.youtube.com/channel/UC123/live",
            "https://www.youtube.com/c/SomeChannel",
            "https://www.youtube.com/user/SomeUser",
            "https://www.youtube.com/results?search_query=cats",
            "https://youtubei.googleapis.com/v1/search",
            "https://redirector.googlevideo.com/videoplayback?id=abc",
            "https://lh3.ggpht.com/avatar.png",
            "just some text",
            "https://example.com/watch?v=short",
        ] {
            assert!(youtube_id(input).is_err(), "input: {input}");
        }
    }

    #[test]
    fn empty_is_allowed() {
        assert_eq!(youtube_id("   "), Ok(String::new()));
    }
}
