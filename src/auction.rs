//! Auction core (-style) — ported from the JS AUC.
//! Lots, bids, name/number/alias/fuzzy matching, pending queue, winner pick.
//! Lives in the exe so the web page only renders; all settings are exe config.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use crate::model::now_ms;

/* ------------------------------------------------------------------ model */

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Lot {
    pub id: String,
    pub number: i64,
    pub title: String,
    #[serde(default)] pub aliases: Vec<String>,
    #[serde(default)] pub pinned: bool,
    #[serde(default)] pub adjust: f64,
    #[serde(default)] pub first_bidder: String,
    #[serde(default)] pub total: f64,
    #[serde(default)] pub participants: usize,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Bid {
    pub id: String,
    #[serde(default)] pub lot_id: String,
    #[serde(default)] pub lot_title: String,
    pub username: String,
    #[serde(default)] pub source: String, // "twitch" | "monobank" | "manual"
    pub amount: f64,
    #[serde(default)] pub redemption_id: String,
    #[serde(default)] pub user_input: String,
    #[serde(default)] pub at: i64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuctionSettings {
    #[serde(default = "d_manual")] pub mode: String,      // "manual" | "auto"
    #[serde(default = "d_points")] pub source: String,    // "points" | "donations"
    #[serde(default = "d_uah")] pub currency: String,     // "UAH" | "USD"
    #[serde(default = "d_timer")] pub timer_seconds: u64,
    #[serde(default)] pub show_total_time: bool,
    #[serde(default)] pub leader_change_points: f64,
    #[serde(default)] pub leader_change_donations: f64,
    #[serde(default)] pub new_position_points: f64,
    #[serde(default)] pub new_position_donations: f64,
    #[serde(default)] pub add_existing_points: f64,
    #[serde(default)] pub add_existing_donations: f64,
    #[serde(default)] pub time_more_than_min: f64,
}
fn d_manual() -> String { "manual".into() }
fn d_points() -> String { "points".into() }
fn d_uah() -> String { "UAH".into() }
fn d_timer() -> u64 { 600 }
impl Default for AuctionSettings {
    fn default() -> Self { AuctionSettings { mode: d_manual(), source: d_points(), currency: d_uah(), timer_seconds: d_timer(), show_total_time: false, leader_change_points: 0.0, leader_change_donations: 0.0, new_position_points: 0.0, new_position_donations: 0.0, add_existing_points: 0.0, add_existing_donations: 0.0, time_more_than_min: 0.0 } }
}

/// full auction state (persisted with the rest of the exe state)
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuctionState {
    #[serde(default = "d_idle")] pub status: String, // idle | open | paused | closed
    #[serde(default)] pub running: bool,
    #[serde(default)] pub ends_at: i64,      // ms while running
    #[serde(default = "d_timer_ms")] pub remaining_ms: i64,
    #[serde(default = "d_timer_ms")] pub total_ms: i64, // initial + all extensions
    #[serde(default)] pub lots: Vec<Lot>,
    #[serde(default)] pub pending: Vec<Bid>,
    #[serde(default)] pub log: Vec<Bid>,     // recent bids (newest first)
    #[serde(default)] pub winner: Option<Bid>,
    #[serde(default)] pub settings: AuctionSettings,
    #[serde(default)] pub seen: Vec<String>, // redemption ids (dedup)
}
fn d_idle() -> String { "idle".into() }
fn d_timer_ms() -> i64 { 600_000 }
impl Default for AuctionState {
    fn default() -> Self {
        AuctionState { status: d_idle(), running: false, ends_at: 0, remaining_ms: d_timer_ms(), total_ms: d_timer_ms(),
            lots: Vec::new(), pending: Vec::new(), log: Vec::new(), winner: None, settings: AuctionSettings::default(), seen: Vec::new() }
    }
}

fn uid() -> String { format!("{:016x}", rand::random::<u64>()) }

/* ------------------------------------------------------------------ matching */

pub fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric() || ('\u{0400}'..='\u{04ff}').contains(c)).collect()
}
pub fn lev(a: &str, b: &str) -> usize {
    let (m, n) = (a.chars().count(), b.chars().count());
    if m == 0 { return n; }
    if n == 0 { return m; }
    let av: Vec<char> = a.chars().collect(); let bv: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut cur = vec![0usize; n + 1];
    for i in 1..=m { cur[0] = i; for j in 1..=n { cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + if av[i - 1] == bv[j - 1] { 0 } else { 1 }); } std::mem::swap(&mut prev, &mut cur); }
    prev[n]
}
pub fn ratio(a: &str, b: &str) -> f64 { let l = a.chars().count().max(b.chars().count()); if l == 0 { 1.0 } else { 1.0 - lev(a, b) as f64 / l as f64 } }

/// find the best lot for a raw input: exact number, exact name, alias, then fuzzy (>= 0.82)
pub fn match_lot<'a>(lots: &'a [Lot], input: &str) -> Option<&'a Lot> {
    let raw = input.trim(); let n = norm(raw); if n.is_empty() { return None; }
    let num: i64 = raw.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(-1);
    let pure_num = { let t = raw.trim_start_matches('#').trim(); !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) };
    if pure_num { for l in lots { if l.number == num { return Some(l); } } }
    for l in lots { if norm(&l.title) == n { return Some(l); } }
    for l in lots { if l.aliases.iter().any(|a| norm(a) == n) { return Some(l); } }
    let mut best: Option<(&Lot, f64)> = None;
    for l in lots {
        let mut s = ratio(&norm(&l.title), &n);
        for a in &l.aliases { s = s.max(ratio(&norm(a), &n)); }
        if best.is_none_or(|(_, bs)| s > bs) { best = Some((l, s)); }
    }
    best.filter(|(_, s)| *s >= 0.82).map(|(l, _)| l)
}

/* ------------------------------------------------------------------ state ops */

impl AuctionState {
    pub fn remaining(&self) -> i64 { if self.running && self.ends_at > 0 { (self.ends_at - now_ms()).max(0) } else { self.remaining_ms.max(0) } }

    pub fn next_number(&self) -> i64 { self.lots.iter().map(|l| l.number).max().unwrap_or(0) + 1 }

    pub fn make_lot(&mut self, title: &str, by_admin: bool) -> &Lot {
        let lot = Lot { id: uid(), number: self.next_number(), title: title.to_string(), aliases: Vec::new(),
            pinned: false, adjust: 0.0, first_bidder: if by_admin { "admin".into() } else { String::new() }, total: 0.0, participants: 0 };
        self.lots.push(lot); self.lots.last().unwrap()
    }

    /// recompute lot totals + participants from the bid log
    pub fn recompute(&mut self) {
        let mut acc: Vec<(String, f64, std::collections::HashSet<String>)> = self.lots.iter().map(|l| (l.id.clone(), 0.0, std::collections::HashSet::new())).collect();
        for b in &self.log {
            if let Some(e) = acc.iter_mut().find(|(id, _, _)| *id == b.lot_id) { e.1 += b.amount; e.2.insert(b.username.to_lowercase()); }
        }
        for (i, l) in self.lots.iter_mut().enumerate() { l.total = acc[i].1 + l.adjust; l.participants = acc[i].2.len(); }
    }

    pub fn add_seconds(&mut self, sec: f64) {
        if sec <= 0.0 { return; }
        let d = (sec * 1000.0) as i64;
        if self.running && self.ends_at > 0 { self.ends_at = (self.ends_at + d).max(now_ms()); } else { self.remaining_ms = (self.remaining_ms + d).max(0); }
        self.total_ms = (self.total_ms + d).max(0);
    }

    /// add a bid. `source` = twitch|monobank|manual. Returns the lot title on a match, or None if queued.
    pub fn intake(&mut self, username: &str, amount: f64, user_input: &str, source: &str, redemption_id: &str) -> Result<Option<String>, String> {
        if username.is_empty() || amount <= 0.0 { return Err("bad bid".into()); }
        if self.settings.source == "points" && source == "monobank" { return Err("donations disabled".into()); }
        if self.settings.source == "donations" && source == "twitch" { return Err("points disabled".into()); }
        if !redemption_id.is_empty() && self.seen.iter().any(|r| r == redemption_id) { return Err("duplicate".into()); }
        if !redemption_id.is_empty() { self.seen.push(redemption_id.to_string()); }

        let matched = match_lot(&self.lots, user_input).map(|l| (l.id.clone(), l.title.clone()));
        let bid = Bid { id: uid(), lot_id: matched.as_ref().map(|m| m.0.clone()).unwrap_or_default(),
            lot_title: matched.as_ref().map(|m| m.1.clone()).unwrap_or_default(), username: username.to_string(),
            source: source.to_string(), amount, redemption_id: redemption_id.to_string(), user_input: user_input.to_string(), at: now_ms() };

        if self.settings.mode == "manual" || matched.is_none() { self.pending.insert(0, bid); return Ok(None); }
        let (lot_id, lot_title) = matched.unwrap();
        self.apply_bid(bid, &lot_id, &lot_title);
        Ok(Some(lot_title))
    }

    /// apply a bid directly (used by intake + pending approval)
    pub fn apply_bid(&mut self, mut b: Bid, lot_id: &str, lot_title: &str) {
        b.lot_id = lot_id.to_string(); b.lot_title = lot_title.to_string();
        let had = self.log.iter().any(|x| x.lot_id == lot_id);
        let before_top = self.top_lot().map(|l| l.id.clone());
        let before_rank = self.rank_of(lot_id);
        if let Some(l) = self.lots.iter_mut().find(|l| l.id == lot_id) { if l.first_bidder.is_empty() { l.first_bidder = b.username.clone(); } }
        self.log.insert(0, b.clone());
        self.log.truncate(200);
        self.recompute();
        let after_top = self.top_lot().map(|l| l.id.clone());
        let leader_changed = after_top.is_some() && after_top != before_top;
        let new_position = self.rank_of(lot_id).is_some_and(|r| before_rank.is_none_or(|br| r < br));
        self.extend_timer(&b.source, leader_changed, new_position, had);
    }

    fn top_lot(&self) -> Option<&Lot> { self.lots.iter().filter(|l| l.total > 0.0).max_by(|a, b| a.total.partial_cmp(&b.total).unwrap_or(std::cmp::Ordering::Equal)) }
    fn rank_of(&self, lot_id: &str) -> Option<usize> {
        let mut v: Vec<&Lot> = self.lots.iter().filter(|l| l.total > 0.0).collect();
        v.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap_or(std::cmp::Ordering::Equal));
        v.iter().position(|l| l.id == lot_id)
    }

    fn extend_timer(&mut self, source: &str, leader_changed: bool, new_position: bool, had: bool) {
        let s = self.settings.clone();
        if s.time_more_than_min > 0.0 && self.remaining() as f64 > s.time_more_than_min * 60_000.0 { return; }
        let pts = source == "twitch";
        let mut add = 0.0;
        if leader_changed { add += if pts { s.leader_change_points } else { s.leader_change_donations }; }
        if new_position { add += if pts { s.new_position_points } else { s.new_position_donations }; }
        if had { add += if pts { s.add_existing_points } else { s.add_existing_donations }; }
        self.add_seconds(add);
    }

    /// pick a winner: mode = "top" (highest lot) or "wheel" (weighted random)
    pub fn pick_winner(&mut self, mode: &str, lot_id: Option<&str>) {
        let pool: Vec<Lot> = self.lots.iter().filter(|l| l.total > 0.0).cloned().collect();
        if pool.is_empty() { return; }
        let chosen = if let Some(id) = lot_id { pool.iter().find(|l| l.id == id).cloned() }
            else if mode == "top" { pool.iter().max_by(|a, b| a.total.partial_cmp(&b.total).unwrap()).cloned() }
            else { weighted_pick(&pool) };
        if let Some(lot) = chosen {
            let top = self.log.iter().filter(|b| b.lot_id == lot.id).map(|b| (b.username.clone(), b.amount)).fold(std::collections::HashMap::<String, f64>::new(), |mut m, (u, a)| { *m.entry(u).or_insert(0.0) += a; m })
                .into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let (tname, tamt) = top.map(|t| (t.0, t.1)).unwrap_or((String::new(), lot.total));
            self.winner = Some(Bid { id: uid(), lot_id: lot.id.clone(), lot_title: lot.title.clone(),
                username: tname, source: String::new(), amount: tamt,
                redemption_id: String::new(), user_input: String::new(), at: now_ms() });
            self.status = "closed".into();
        }
    }
}

/// weighted-random pick over lots (chance proportional to total)
pub fn weighted_pick(lots: &[Lot]) -> Option<Lot> {
    let total: f64 = lots.iter().map(|l| l.total).sum();
    if total <= 0.0 { return lots.first().cloned(); }
    let mut r = rand::random::<f64>() * total;
    for l in lots { r -= l.total; if r <= 0.0 { return Some(l.clone()); } }
    lots.last().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn st() -> AuctionState { let mut s = AuctionState::default(); s.settings.mode = "auto".into(); s.make_lot("Alpha", true); s.make_lot("Beta", true); s }
    #[test] fn exact_and_number_and_fuzzy() {
        let mut s = st();
        assert_eq!(s.intake("u1", 10.0, "Alpha", "manual", "").unwrap().as_deref(), Some("Alpha"));
        assert_eq!(s.intake("u2", 5.0, "#2", "manual", "").unwrap().as_deref(), Some("Beta"));
        assert_eq!(s.intake("u3", 5.0, "Alphha", "manual", "").unwrap().as_deref(), Some("Alpha"));
        assert_eq!(s.intake("u4", 5.0, "zzz", "manual", "").unwrap(), None); // pending
    }
    #[test] fn filters_dedup_totals() {
        let mut s = st();
        s.settings.source = "points".into();
        assert!(s.intake("m", 5.0, "Alpha", "monobank", "").is_err());
        assert!(s.intake("t", 5.0, "Alpha", "twitch", "R1").is_ok());
        assert!(s.intake("t", 5.0, "Alpha", "twitch", "R1").is_err()); // dedup
        s.intake("t", 5.0, "Alpha", "twitch", "R2").unwrap();
        s.intake("u", 20.0, "Alpha", "manual", "").unwrap();
        let a = s.lots.iter().find(|l| l.title == "Alpha").unwrap();
        assert_eq!(a.total, 30.0);
        assert_eq!(a.participants, 2);
        assert_eq!(a.first_bidder, "admin");
    }
    #[test] fn winner_top() { let mut s = st(); s.intake("a", 10.0, "Alpha", "manual", "").unwrap(); s.intake("b", 40.0, "Beta", "manual", "").unwrap(); s.pick_winner("top", None); assert_eq!(s.winner.unwrap().lot_title, "Beta"); }
}
