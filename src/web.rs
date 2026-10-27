use crate::app::Ctx;
use crate::model::*;
use crate::mono;
use serde_json::json;
use std::io::Read;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tiny_http::{Header, Method, Response, StatusCode};

type Resp = Response<std::io::Cursor<Vec<u8>>>;

static UPDATE_INFO: std::sync::OnceLock<std::sync::Mutex<serde_json::Value>> = std::sync::OnceLock::new();

fn update_info() -> &'static std::sync::Mutex<serde_json::Value> {
    UPDATE_INFO.get_or_init(|| std::sync::Mutex::new(json!({
        "checking": true, "available": false, "current": env!("CARGO_PKG_VERSION")
    })))
}

fn version_gt(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim().trim_start_matches(['v', 'V'])
            .split('.')
            .map(|x| x.split(|c: char| !c.is_ascii_digit()).next().unwrap_or("0").parse().unwrap_or(0))
            .collect()
    };
    let a = parse(latest);
    let b = parse(current);
    for i in 0..a.len().max(b.len()) {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        if x != y { return x > y; }
    }
    false
}

/// Background GitHub release check; /api/update serves the cached result.
pub fn start_update_check() {
    std::thread::spawn(|| {
        let current = env!("CARGO_PKG_VERSION");
        let val = match ureq::get("https://api.github.com/repos/terriblefeelings/LocalMonoDonate/releases/latest")
            .set("User-Agent", "LocalMonoDonate")
            .set("Accept", "application/vnd.github+json")
            .timeout(Duration::from_secs(10))
            .call()
        {
            Ok(r) => match r.into_string().ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
                Some(j) => {
                    let tag = j.get("tag_name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let url = j.get("html_url").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let name = j.get("name").and_then(|x| x.as_str()).unwrap_or(&tag).to_string();
                    json!({ "checking": false, "current": current, "latest": tag,
                            "available": version_gt(&tag, current), "url": url, "name": name })
                }
                None => json!({ "checking": false, "available": false, "current": current, "error": "invalid GitHub response" }),
            },
            Err(e) => json!({ "checking": false, "available": false, "current": current, "error": e.to_string() }),
        };
        *update_info().lock().unwrap() = val;
    });
}

fn hdr(k: &str, v: &str) -> Header { Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap() }

fn body_resp(code: u16, ctype: &str, body: Vec<u8>) -> Resp {
    Response::from_data(body)
        .with_status_code(StatusCode(code))
        .with_header(hdr("Content-Type", ctype))
        .with_header(hdr("Cache-Control", "no-store"))
}

/// Serve a file with HTTP Range support (needed for reliable <video> playback/seeking).
fn file_resp(request: tiny_http::Request, bytes: Vec<u8>, ctype: &str) {
    let total = bytes.len();
    let range = request.headers().iter()
        .find(|h| h.field.equiv("Range"))
        .map(|h| h.value.as_str().to_string());
    if let Some(r) = range.as_deref().and_then(|r| r.strip_prefix("bytes=")) {
        let part = r.split(',').next().unwrap_or("").trim();
        let (s, e) = match part.split_once('-') {
            Some((a, b)) => {
                let st = a.trim().parse::<usize>().ok();
                let en = b.trim().parse::<usize>().ok();
                let last = total.saturating_sub(1);
                match (st, en) {
                    (Some(s), Some(e)) => (s, e.min(last)),
                    (Some(s), None) => (s, last),
                    (None, Some(n)) => (total.saturating_sub(n), last),
                    (None, None) => (0, last),
                }
            }
            None => (0, total.saturating_sub(1)),
        };
        if total > 0 && s <= e && s < total {
            let slice = bytes[s..=e].to_vec();
            let len = slice.len();
            let resp = Response::from_data(slice)
                .with_status_code(StatusCode(206))
                .with_header(hdr("Content-Type", ctype))
                .with_header(hdr("Accept-Ranges", "bytes"))
                .with_header(hdr("Content-Range", &format!("bytes {}-{}/{}", s, e, total)))
                .with_header(hdr("Content-Length", &len.to_string()))
                .with_header(hdr("Cache-Control", "no-store"));
            let _ = request.respond(resp);
            return;
        }
    }
    let resp = Response::from_data(bytes)
        .with_status_code(StatusCode(200))
        .with_header(hdr("Content-Type", ctype))
        .with_header(hdr("Accept-Ranges", "bytes"))
        .with_header(hdr("Cache-Control", "no-store"));
    let _ = request.respond(resp);
}

fn json_resp(code: u16, v: serde_json::Value) -> Resp {
    body_resp(code, "application/json; charset=utf-8", v.to_string().into_bytes())
}

/// value of `key` in a `path?a=b&c=d` request line
fn query(full: &str, key: &str) -> Option<String> {
    full.split('?').nth(1)?.split('&')
        .find_map(|kv| { let (k, v) = kv.split_once('=')?; (k == key).then(|| v.to_string()) })
}

/* ---------------------------------------------------------------- events */

fn poll_events(ctx: &Ctx, request: tiny_http::Request, full: &str) {
    let since: u64 = full.split('?').nth(1).unwrap_or("")
        .split('&').find_map(|kv| kv.strip_prefix("since="))
        .and_then(|v| v.parse().ok()).unwrap_or(0);
    let (items, last) = ctx.events_since(since);
    let _ = request.respond(json_resp(200, json!({ "events": items, "last": last })));
}

/* ---------------------------------------------------------------- state */

fn state_json(ctx: &Ctx) -> serde_json::Value {
    let cfg = ctx.cfg();
    let st = ctx.state.lock().unwrap();
    let today_start = local_midnight_ms();

    let jars: Vec<serde_json::Value> = st.jars.iter().enumerate().map(|(i, j)| {
        let custom = cfg.jar_names.get(i).cloned().unwrap_or_default();
        let biggest_today = j.history.iter()
            .filter(|h| h.at >= today_start)
            .map(|h| h.amount)
            .fold(0.0f64, f64::max);
        let top: Vec<serde_json::Value> = top_donators(&j.history).into_iter()
            .map(|(name, total, count)| json!({ "name": name, "total": total, "count": count }))
            .collect();
        let pending: Vec<serde_json::Value> = j.pending.iter()
            .map(|p| json!({ "code": p.code, "amount": p.amount, "name": p.name, "message": p.message, "createdAt": p.created_at }))
            .collect();
        let history: Vec<serde_json::Value> = j.history.iter().take(200)
            .map(|h| json!({ "id": h.id, "name": h.name, "amount": h.amount, "message": h.message,
                             "youtubeId": h.youtube_id, "mediaUrl": h.media_url, "bitLabel": h.bit_label, "bitCode": h.bit_code, "matched": h.matched, "at": h.at }))
            .collect();
        json!({
            "code": j.code, "title": if !custom.trim().is_empty() { custom } else { j.title.clone() },
            "mine": cfg.jar_mine.get(i).copied().unwrap_or(true),
            "balance": j.balance, "goal": j.goal, "currency": currency_symbol(j.currency),
            "accountResolved": !j.statement_account.is_empty(),
            "stats": { "total": j.stats.total, "count": j.stats.count,
                       "biggest": j.stats.biggest, "biggestToday": biggest_today },
            "pending": pending, "history": history, "top": top
        })
    }).collect();

    let (donate, stats_url, overlay_url) = links(&cfg);
    json!({
        "nickname": cfg.nickname,
        "hasToken": !cfg.token.is_empty(),
        "minAmount": cfg.min_amount,
        "maxAmount": cfg.max_amount,
        "tts": { "mode": cfg.tts.mode, "voice": cfg.tts.default_voice(), "readMode": cfg.tts.read_mode,
                 "allowed": cfg.tts.allowed_in(&cfg.tts.dir), "picks": cfg.tts.donor_picks() },
        "voices": [{"id": "google:uk", "label": "Google (українська)", "lang": "uk"}],
        "limits": { "nameMax": cfg.limits.name_max, "messageMax": cfg.limits.message_max },
        "overlay": { "durationMs": cfg.overlay.duration_ms, "videoMaxMs": cfg.overlay.video_max_ms },
        "jars": jars,
        "links": { "donate": donate, "stats": stats_url, "overlay": overlay_url },
        "notify": serde_json::to_value(&cfg.notify).unwrap_or(serde_json::Value::Null)
    })
}

/* ------------------------------------------------------------- handlers */

fn read_body(request: &mut tiny_http::Request) -> serde_json::Value {
    let mut s = String::new();
    let _ = request.as_reader().read_to_string(&mut s);
    serde_json::from_str(&s).unwrap_or(serde_json::Value::Null)
}

fn media_type_enabled(cfg: &Config, provider: &str) -> bool {
    cfg.notify.rules.iter()
        .find(|r| r.id == "default")
        .map(|r| r.style.media_enabled(provider))
        .unwrap_or(true)
}

fn submit(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let cfg = ctx.cfg();
    let amount_in = v.get("amount").and_then(|x| x.as_f64()).unwrap_or(0.0);
    let mut name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let mut message = v.get("message").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let yt = v.get("youtube").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let voice = v.get("voice").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    // optional bit picked on the donation form (must exist in the configured deck)
    // only the code is trusted from the client; price/label/url come from the local deck
    let bit_sel: Option<Bit> = v.get("bit").and_then(|b| b.get("code")).and_then(|c| c.as_str())
        .and_then(|c| cfg.bits.iter().find(|x| !x.code.is_empty() && x.code == c).cloned());
    let bits_sel: Vec<Bit> = {
        let mut out: Vec<Bit> = Vec::new();
        if let Some(arr) = v.get("bits").and_then(|b| b.as_array()) {
            for item in arr {
                let Some(c) = item.get("code").and_then(|c| c.as_str()) else { continue };
                if let Some(b) = cfg.bits.iter().find(|x| !x.code.is_empty() && x.code == c) {
                    if !out.iter().any(|o| o.code == b.code) { out.push(b.clone()); }
                }
            }
        }
        if out.is_empty() { if let Some(b) = bit_sel.clone() { out.push(b); } }
        out
    };
    let mode = cfg.bits_mode.clone();
    let amount = if amount_in > 0.0 { amount_in } else if bits_sel.is_empty() {
        0.0
    } else {
        bits_sel.iter().map(|b| b.price).sum()
    };

    let fail = |req: tiny_http::Request, m: &str| { let _ = req.respond(json_resp(400, json!({"ok": false, "error": m}))); };
    if !cfg.features.donate { return fail(request, "Донати вимкнено"); }
    if cfg.jars.is_empty() { return fail(request, "Не налаштовано жодної банки"); }
    if amount.fract() != 0.0 || amount < cfg.min_amount || amount > cfg.max_amount {
        return fail(request, &format!("Сума має бути цілим числом від {:.0} до {:.0} ₴", cfg.min_amount, cfg.max_amount));
    }
    if name.chars().count() > cfg.limits.name_max { return fail(request, "Занадто довге ім'я"); }
    if message.chars().count() > cfg.limits.message_max { return fail(request, "Занадто довге повідомлення"); }
    if bits_sel.is_empty() {
        if name.is_empty() { name = "Анонім".into(); }
        if message.is_empty() { message = format!("{} нічого не написав", name); }
    }
    let parsed_yt = youtube_id(&yt);
    let ytid = parsed_yt.clone().unwrap_or_default();
    let youtube_link = !ytid.is_empty() || {
        let lower = yt.to_ascii_lowercase();
        lower.contains("youtube.com") || lower.contains("youtu.be")
    };
    if youtube_link && !media_type_enabled(&cfg, "youtube") {
        return fail(request, "YouTube links are disabled");
    }
    if let Some(provider) = media_provider(&yt) {
        if !media_type_enabled(&cfg, provider) {
            return fail(request, "This media type is disabled");
        }
    }
    let other_media = if !yt.is_empty() && ytid.is_empty() && !yt.contains("list=") {
        other_media_url(&yt)
    } else { None };
    if !yt.is_empty() && ytid.is_empty() && !yt.contains("list=") && other_media.is_none() {
        return fail(request, "Invalid media link");
    }
    // media checks use the variation matched for this amount
    let ystyle = pick_style(&cfg.notify.rules, amount);
    // a watch link with &list= still points at one video and is allowed; a pure playlist has no id
    if ytid.is_empty() && yt.contains("list=") && !ystyle.yt_playlists { return fail(request, "Playlist is not allowed"); }
    if !ytid.is_empty() {
        if ystyle.yt_min > 0.0 && amount < ystyle.yt_min { return fail(request, &format!("YouTube requires at least {:.0} UAH", ystyle.yt_min)); }
        if let Some((views, secs)) = mono::youtube_meta(&ytid) {
            if ystyle.yt_views > 0 && views < ystyle.yt_views { return fail(request, &format!("Video needs at least {} views", ystyle.yt_views)); }
            if ystyle.yt_max_min > 0 && secs > ystyle.yt_max_min * 60 { return fail(request, &format!("Video must be at most {} min", ystyle.yt_max_min)); }
        }
    }
    if let Some(_url) = &other_media {
        if ystyle.other_media_min > 0.0 && amount < ystyle.other_media_min {
            return fail(request, &format!("Other media requires at least {:.0} UAH", ystyle.other_media_min));
        }
    }
    let media_url = other_media.map(|u| resolve_short_media(&u)).unwrap_or_default();

    let jar_req = v.get("jar").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let jar_code = if cfg.jars.iter().any(|j| j == &jar_req) { jar_req } else { cfg.jars[0].clone() };
    let jar_idx = cfg.jars.iter().position(|j| j == &jar_code).unwrap_or(0);
    if !cfg.jar_mine.get(jar_idx).copied().unwrap_or(true) {
        // 3rd-party jar: no code, no pending, no tracking — just the payment link
        let t = name.replace(' ', "%20");
        let redirect = format!("https://send.monobank.ua/jar/{}?a={:.0}&t={}", jar_code, amount, t);
        let _ = request.respond(json_resp(200, json!({ "ok": true, "thirdParty": true, "redirect": redirect, "fallback": redirect })));
        return;
    }
    // two codes for one intent: the auto-opened payment page and the fallback link.
    // The first matching payment consumes both so a viewer cannot trigger the alert twice
    // when both tabs are open during the redirect.
    let (code, code2) = {
        let st = ctx.state.lock().unwrap();
        let j = st.jars.iter().find(|j| j.code == jar_code);
        let mut issued = j.map(|j| j.issued.clone()).unwrap_or_default();
        let pending = j.map(|j| j.pending.clone()).unwrap_or_default();
        let code = gen_code(&cfg.code_prefix, &issued, &pending);
        issued.push(code.clone());
        let code2 = gen_code(&cfg.code_prefix, &issued, &pending);
        (code, code2)
    };
    {
        let mut st = ctx.state.lock().unwrap();
        if let Some(j) = st.jars.iter_mut().find(|j| j.code == jar_code) {
            j.pending.push(Pending { code: code.clone(), amount, name: name.clone(), message: message.clone(), youtube_id: ytid.clone(), media_url: media_url.clone(), voice: voice.clone(), bit: bit_sel.clone(), bits: bits_sel.clone(), bits_mode: mode.clone(), pair: code2.clone(), created_at: now_ms() });
            j.pending.push(Pending { code: code2.clone(), amount, name: name.clone(), message: message.clone(), youtube_id: ytid.clone(), media_url: media_url.clone(), voice: voice.clone(), bit: bit_sel.clone(), bits: bits_sel.clone(), bits_mode: mode.clone(), pair: code.clone(), created_at: now_ms() });
            j.issued.push(code.clone());
            j.issued.push(code2.clone());
            if j.issued.len() > 5000 { let n = j.issued.len() - 5000; j.issued.drain(0..n); }
        }
    }
    ctx.save();
    ctx.watch(10 * 60 * 1000);

    // variation flags in the bank comment: A - anonymous, M - media, SA - sound alert
    let mut flags = String::new();
    if name.trim().is_empty() || name == "Анонім" { flags.push('A'); }
    if !ytid.is_empty() || !media_url.is_empty() { flags.push('M'); }
    if !bits_sel.is_empty() { flags.push_str("SA"); }
    let comment = donate_comment(&flags, &code).replace(' ', "%20");
    let comment2 = donate_comment(&flags, &code2).replace(' ', "%20");
    let redirect = format!("https://send.monobank.ua/jar/{}?a={:.0}&t={}", jar_code, amount, comment);
    let fallback = format!("https://send.monobank.ua/jar/{}?a={:.0}&t={}", jar_code, amount, comment2);
    log(&format!("form: {:.0} UAH from {} -> {} / {} ({})", amount, name, code, code2, jar_code));
    let _ = request.respond(json_resp(200, json!({ "ok": true, "code": code, "code2": code2, "redirect": redirect, "fallback": fallback })));
}

fn pct_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' { out.push(b as char); }
        else { out.push_str(&format!("%{:02X}", b)); }
    }
    out
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(b[i]); i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn twitch_clip_slug(url: &str) -> Option<String> {
    let s = url.trim().strip_prefix("https://").or_else(|| url.trim().strip_prefix("http://")).unwrap_or(url.trim());
    let s = s.strip_prefix("//").unwrap_or(s);
    let (host, rest) = s.split_once('/').unwrap_or((s, ""));
    let host = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    if !(host == "twitch.tv" || host.ends_with(".twitch.tv")) { return None; }
    let path = format!("/{}", rest.split(['?', '#']).next().unwrap_or(""));
    let mut slug = if let Some(i) = path.find("/clip/") {
        path[i + 6..].split(['/', '?', '#']).next().unwrap_or("").to_string()
    } else {
        path.split('/').find(|x| !x.is_empty()).unwrap_or("").to_string()
    };
    if slug == "embed" || slug.is_empty() {
        if let Some(q) = rest.split_once("clip=").map(|(_, q)| q) {
            slug = q.split(['&', '#']).next().unwrap_or("").to_string();
        }
    }
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') { return None; }
    Some(slug)
}

fn twitch_clip_mp4(url: &str) -> Option<String> {
    let slug = twitch_clip_slug(url)?;
    let body = json!([{
        "operationName": "VideoAccessToken_Clip",
        "variables": { "slug": slug },
        "extensions": { "persistedQuery": { "version": 1,
            "sha256Hash": "36b89d2507fce29e5ca551df756d27c1cfe079e2609642b4390aa4c35796eb11" } }
    }]).to_string();
    let resp = ureq::post("https://gql.twitch.tv/gql")
        .set("Client-ID", "kimne78kx3ncx6brgo4mv6wki5h1ko")
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(5))
        .send_string(&body).ok()?;
    let text = resp.into_string().ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let clip = v.get(0)?.get("data")?.get("clip")?;
    let qualities = clip.get("videoQualities")?.as_array()?;
    let best = qualities.iter().find(|q| q.get("quality").and_then(|x| x.as_str()) == Some("720"))
        .or_else(|| qualities.first())?;
    let src = best.get("sourceURL")?.as_str()?;
    let tok = clip.get("playbackAccessToken")?;
    let sig = tok.get("signature")?.as_str()?;
    let value = tok.get("value")?.as_str()?;
    let join = if src.contains('?') { '&' } else { '?' };
    Some(format!("{}{}sig={}&token={}", src, join, sig, pct_encode(value)))
}

/// TikTok share links need their redirect resolved at submit time; keep the original
/// Twitch URL in history and resolve its short-lived MP4 only when it is about to play.
fn resolve_short_media(url: &str) -> String {
    let short = url.contains("vm.tiktok.com") || url.contains("vt.tiktok.com") || url.contains("tiktok.com/t/");
    if !short { return url.to_string(); }
    match ureq::get(url).timeout(Duration::from_secs(4)).call() {
        Ok(r) => r.get_url().to_string(),
        Err(_) => url.to_string(),
    }
}

/// X links often need the /video/1 route before the player exposes the MP4.
fn x_with_video(url: &str) -> String {
    let raw = url.trim();
    let (base, tail) = match raw.split_once('?') { Some((b, q)) => (b, format!("?{}", q)), None => (raw, String::new()) };
    let lower = base.to_ascii_lowercase();
    if !(lower.contains("x.com/") || lower.contains("twitter.com/")) { return raw.to_string(); }
    if base.contains("/video/") { return raw.to_string(); }
    if base.contains("/status/") {
        format!("{}/video/1{}", base.trim_end_matches('/'), tail)
    } else { raw.to_string() }
}

fn x_status_id(url: &str) -> Option<String> {
    let s = url.trim().strip_prefix("https://").or_else(|| url.trim().strip_prefix("http://")).unwrap_or(url.trim());
    let s = s.strip_prefix("//").unwrap_or(s);
    let (host, rest) = s.split_once('/').unwrap_or((s, ""));
    let host = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    if !(host == "x.com" || host.ends_with(".x.com") || host == "twitter.com" || host.ends_with(".twitter.com")) {
        return None;
    }
    let segs: Vec<&str> = rest.split(['?', '#']).next().unwrap_or("").split('/').filter(|x| !x.is_empty()).collect();
    for i in 0..segs.len().saturating_sub(1) {
        if (segs[i] == "status" || segs[i] == "statuses") && segs[i + 1].chars().all(|c| c.is_ascii_digit()) {
            return Some(segs[i + 1].to_string());
        }
    }
    None
}

/// X/Twitter videos are resolved to their direct MP4 so the overlay shows the
/// video itself instead of the whole tweet card.
fn best_mp4(variants: &[serde_json::Value], url_key: &str, type_key: &str) -> Option<String> {
    let mut best: Option<(u32, String)> = None;
    let mut fallback: Option<String> = None;
    for q in variants {
        if q.get(type_key).and_then(|x| x.as_str()) != Some("video/mp4") { continue; }
        let src = q.get(url_key).and_then(|x| x.as_str()).unwrap_or("");
        if src.is_empty() { continue; }
        fallback = Some(src.to_string());
        // pick the best rung that still fits the 640x360 PiP (720p max)
        let h = src.split('/').filter_map(|p| {
            let (w, h) = p.split_once('x')?;
            if !w.is_empty() && !h.is_empty() && w.chars().all(|c| c.is_ascii_digit()) && h.chars().all(|c| c.is_ascii_digit()) {
                h.parse::<u32>().ok()
            } else { None }
        }).max().unwrap_or(0);
        if h <= 720 && best.as_ref().map(|(bh, _)| h > *bh).unwrap_or(true) { best = Some((h, src.to_string())); }
    }
    best.map(|(_, s)| s).or(fallback)
}

fn x_video_syndication(id: &str) -> Option<String> {
    let endpoint = format!("https://cdn.syndication.twimg.com/tweet-result?id={}&lang=en&token=a", id);
    let resp = ureq::get(&endpoint).set("User-Agent", "Mozilla/5.0")
        .timeout(Duration::from_secs(4)).call().ok()?;
    let text = resp.into_string().ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let variants = v.get("video").and_then(|x| x.get("variants")).and_then(|x| x.as_array())?;
    best_mp4(variants, "src", "type")
}

fn x_video_fx(id: &str) -> Option<String> {
    let endpoint = format!("https://api.fxtwitter.com/status/{}", id);
    let resp = ureq::get(&endpoint).set("User-Agent", "Mozilla/5.0")
        .timeout(Duration::from_secs(4)).call().ok()?;
    let text = resp.into_string().ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let variants = v.get("tweet")?.get("media")?.get("videos")?.get(0)?.get("variants")?.as_array()?;
    best_mp4(variants, "url", "content_type")
}

fn x_video_html(url: &str) -> Option<String> {
    let resp = ureq::get(url).set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .timeout(Duration::from_secs(6)).call().ok()?;
    let text = resp.into_string().ok()?;
    let mut best: Option<(u32, String)> = None;
    for piece in text.split('"') {
        let src = piece.replace("\\/", "/");
        if !src.starts_with("https://video.twimg.com/") || !src.contains(".mp4") { continue; }
        let h = src.split('/').filter_map(|p| {
            let (w, h) = p.split_once('x')?;
            if !w.is_empty() && !h.is_empty() && w.chars().all(|c| c.is_ascii_digit()) && h.chars().all(|c| c.is_ascii_digit()) {
                h.parse::<u32>().ok()
            } else { None }
        }).max().unwrap_or(0);
        if h <= 720 && best.as_ref().map(|(bh, _)| h > *bh).unwrap_or(true) { best = Some((h, src)); }
    }
    best.map(|(_, s)| s)
}

fn x_video_mp4(url: &str) -> Option<String> {
    let url = x_with_video(url);
    let id = x_status_id(&url)?;
    x_video_syndication(&id)
        .or_else(|| x_video_fx(&id))
        .or_else(|| x_video_html(&url))
}

/// Cache the 720p X MP4 locally once, so OBS plays a normal same-origin media
/// file instead of depending on the CDN/proxy for every range request.
fn cache_x_video(ctx: &Ctx, url: &str) -> Option<String> {
    let id = x_status_id(url)?;
    let name = format!("x-{}.mp4", id);
    let path = ctx.paths.base.join("media").join(&name);
    let valid = std::fs::metadata(&path).map(|m| m.len() > 100_000).unwrap_or(false);
    if !valid {
        let src = x_video_mp4(url)?;
        let resp = ureq::get(&src)
            .set("Referer", "https://x.com/")
            .set("User-Agent", "Mozilla/5.0")
            .set("Accept", "video/mp4,*/*")
            .timeout(Duration::from_secs(60))
            .call().ok()?;
        let mut bytes = Vec::new();
        resp.into_reader().read_to_end(&mut bytes).ok()?;
        if bytes.is_empty() { return None; }
        let _ = std::fs::create_dir_all(ctx.paths.base.join("media"));
        std::fs::write(&path, bytes).ok()?;
    }
    Some(format!("/media/{}", name))
}

fn cache_remote_video(ctx: &Ctx, src: &str, name: &str, referer: &str) -> Option<String> {
    if !src.starts_with("https://") { return None; }
    let path = ctx.paths.base.join("media").join(name);
    if std::fs::metadata(&path).map(|m| m.len() > 100_000).unwrap_or(false) {
        return Some(format!("/media/{}", name));
    }
    let resp = ureq::get(src)
        .set("Referer", referer)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36")
        .set("Accept", "video/mp4,*/*")
        .timeout(Duration::from_secs(60))
        .call().ok()?;
    let mut bytes = Vec::new();
    resp.into_reader().read_to_end(&mut bytes).ok()?;
    if bytes.is_empty() { return None; }
    let _ = std::fs::create_dir_all(ctx.paths.base.join("media"));
    std::fs::write(&path, bytes).ok()?;
    Some(format!("/media/{}", name))
}

fn tiktok_id(url: &str) -> Option<String> {
    let s = url.split(['?', '#']).next().unwrap_or(url);
    let parts: Vec<&str> = s.split('/').filter(|x| !x.is_empty()).collect();
    for i in 0..parts.len().saturating_sub(1) {
        if parts[i] == "video" && parts[i + 1].chars().all(|c| c.is_ascii_digit()) {
            return Some(parts[i + 1].to_string());
        }
    }
    for p in parts.iter().rev() {
        let n = p.trim_end_matches(".html");
        if n.chars().all(|c| c.is_ascii_digit()) { return Some(n.to_string()); }
    }
    None
}

fn resolve_tiktok(ctx: &Ctx, url: &str) -> Option<String> {
    let id = tiktok_id(url)?;
    let name = format!("tiktok-{}.mp4", id);
    if std::fs::metadata(ctx.paths.base.join("media").join(&name)).map(|m| m.len() > 100_000).unwrap_or(false) {
        return Some(format!("/media/{}", name));
    }
    let api = format!("https://www.tikwm.com/api/?url={}", pct_encode(url));
    let resp = ureq::get(&api).set("User-Agent", "Mozilla/5.0")
        .timeout(Duration::from_secs(8)).call().ok()?;
    let text = resp.into_string().ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let src = v.get("data")?.get("play")?.as_str()
        .or_else(|| v.get("data")?.get("hdplay")?.as_str())?;
    cache_remote_video(ctx, src, &name, "https://www.tiktok.com/")
}

fn reddit_post_id(url: &str) -> Option<String> {
    let s = url.split(['?', '#']).next().unwrap_or(url);
    let s2 = s.trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("//");
    let host = s2.split('/').next().unwrap_or("").to_ascii_lowercase();
    if host == "redd.it" {
        return s2.split('/').filter(|x| !x.is_empty()).nth(1)
            .filter(|x| x.chars().all(|c| c.is_ascii_alphanumeric())).map(|x| x.to_string());
    }
    let parts: Vec<&str> = s.split('/').filter(|x| !x.is_empty()).collect();
    for i in 0..parts.len().saturating_sub(1) {
        if parts[i] == "comments" {
            let id = parts[i + 1];
            if id.chars().all(|c| c.is_ascii_alphanumeric()) { return Some(id.to_string()); }
        }
    }
    None
}

fn resolve_reddit(ctx: &Ctx, url: &str) -> Option<String> {
    let id = reddit_post_id(url)?;
    let name = format!("reddit-{}.mp4", id);
    if std::fs::metadata(ctx.paths.base.join("media").join(&name)).map(|m| m.len() > 100_000).unwrap_or(false) {
        return Some(format!("/media/{}", name));
    }
    let api = format!("https://api.pullpush.io/reddit/search/submission/?ids={}", id);
    let resp = ureq::get(&api).set("User-Agent", "Mozilla/5.0")
        .timeout(Duration::from_secs(8)).call().ok()?;
    let text = resp.into_string().ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let post = v.get("data")?.get(0)?;
    let raw = post.get("secure_media").and_then(|m| m.get("reddit_video")).and_then(|r| r.get("fallback_url"))
        .or_else(|| post.get("media").and_then(|m| m.get("reddit_video")).and_then(|r| r.get("fallback_url")))
        .and_then(|x| x.as_str()).map(|s| s.replace("&amp;", "&"))
        .or_else(|| post.get("url").and_then(|x| x.as_str()).filter(|u| u.contains("v.redd.it/")).map(|u| u.to_string()));
    // PullPush returns old archived signed URLs; DASH_720 is public without the token.
    let src = raw.and_then(|u| {
        let hash = u.split("v.redd.it/").nth(1)?.split(['/', '?']).next()?;
        Some(format!("https://v.redd.it/{}/DASH_720.mp4", hash))
    })?;
    cache_remote_video(ctx, &src, &name, "https://www.reddit.com/")
}

fn instagram_code(url: &str) -> Option<String> {
    let s = url.split(['?', '#']).next().unwrap_or(url);
    let parts: Vec<&str> = s.split('/').filter(|x| !x.is_empty()).collect();
    for i in 0..parts.len().saturating_sub(1) {
        if ["reel", "reels", "p", "tv"].contains(&parts[i]) {
            let code = parts[i + 1];
            if code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return Some(code.to_string());
            }
        }
    }
    None
}

fn media_python(ctx: &Ctx) -> std::path::PathBuf {
    let mut base = std::path::PathBuf::from(ctx.cfg().tts.dir);
    if !base.is_absolute() {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() { base = parent.join(base); }
        }
    }
    let py = base.join("venv2").join("Scripts").join("python.exe");
    if py.exists() { py } else { std::path::PathBuf::from("python") }
}

fn resolve_instagram(ctx: &Ctx, url: &str) -> Option<String> {
    let code = instagram_code(url)?;
    let name = format!("instagram-{}.mp4", code);
    if std::fs::metadata(ctx.paths.base.join("media").join(&name)).map(|m| m.len() > 100_000).unwrap_or(false) {
        return Some(format!("/media/{}", name));
    }
    let out = std::process::Command::new(media_python(ctx))
        .args(["-m", "yt_dlp", "-g", "--no-warnings", "--socket-timeout", "10", "-f", "best[ext=mp4]/best", url])
        .output().ok()?;
    if !out.status.success() { return None; }
    let text = String::from_utf8_lossy(&out.stdout);
    let src = text.lines().map(str::trim).find(|l| l.starts_with("https://"))?;
    cache_remote_video(ctx, src, &name, "https://www.instagram.com/")
}

/// Fetch an X/Twitter MP4 with the referer its CDN expects; direct playback from
/// the overlay gets 403 because browsers send the overlay's own referer.
fn media_x(request: tiny_http::Request, encoded: &str) {
    let target = pct_decode(encoded);
    if !target.starts_with("https://video.twimg.com/") {
        let _ = request.respond(body_resp(400, "text/plain; charset=utf-8", b"bad media".to_vec()));
        return;
    }
    match ureq::get(&target)
        .set("Referer", "https://x.com/")
        .set("User-Agent", "Mozilla/5.0")
        .set("Accept", "video/mp4,*/*")
        .timeout(Duration::from_secs(60))
        .call() {
        Ok(resp) => {
            let mut bytes = Vec::new();
            if resp.into_reader().read_to_end(&mut bytes).is_ok() {
                file_resp(request, bytes, "video/mp4");
            } else {
                let _ = request.respond(body_resp(502, "text/plain; charset=utf-8", b"media read failed".to_vec()));
            }
        }
        Err(_) => { let _ = request.respond(body_resp(502, "text/plain; charset=utf-8", b"media fetch failed".to_vec())); }
    }
}

/// Twitch clips are turned into their signed direct MP4 at playback time so the
/// overlay does not depend on Twitch's iframe CSP.
fn resolve_other_media(ctx: &Ctx, url: &str) -> String {
    let url = resolve_short_media(url);
    if let Some(mp4) = twitch_clip_mp4(&url) { return mp4; }
    if x_status_id(&url).is_some() {
        if let Some(mp4) = cache_x_video(ctx, &url) { return mp4; }
        return x_with_video(&url);
    }
    if tiktok_id(&url).is_some() {
        if let Some(local) = resolve_tiktok(ctx, &url) { return local; }
    }
    if reddit_post_id(&url).is_some() {
        if let Some(local) = resolve_reddit(ctx, &url) { return local; }
    }
    if instagram_code(&url).is_some() {
        if let Some(local) = resolve_instagram(ctx, &url) { return local; }
    }
    url
}

fn test(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    if v.get("seVariants").and_then(|x| x.as_bool()).unwrap_or(false) {
        let cfg = ctx.cfg();
        let variants: [(&str, &str, &str, &str, &str); 5] = [
            ("SE Test 1", "Звичайне повідомлення", "", "", ""),
            ("SE Test 2", "Повідомлення з YouTube", "", "dQw4w9WgXcQ", ""),
            ("SE Test 3", "Повідомлення зі звуком", "", "", "Airhorn"),
            ("SE Test 4", "Медіа та звук разом", "https://www.tiktok.com/@tiktok/video/7106594312292453675", "", "Bruh"),
            ("SE Test 5", "Дуже довге повідомлення, яке перевіряє, як StreamElements переносить текст у сповіщенні та стрічці активності", "", "", ""),
        ];
        let mut errors: Vec<String> = Vec::new();
        for (name, msg, media, ytid, sound) in variants {
            let text = crate::app::se_message(msg, media, ytid, sound);
            if let Err(e) = crate::streamelements::send_tip(cfg.port, &cfg, name, 5.0, "UAH", &text) {
                errors.push(format!("{}: {}", name, e));
            }
        }
        log(&format!("streamelements: 5 test tips, {} failed", errors.len()));
        let _ = request.respond(json_resp(200, json!({ "ok": errors.is_empty(), "errors": errors })));
        return;
    }
    let amount = v.get("amount").and_then(|x| x.as_f64()).unwrap_or(100.0);
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("Test User").to_string();
    let test_yt = v.get("youtube").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let test_ytid = youtube_id(&test_yt).unwrap_or_default();
    let test_rule = v.get("ruleId").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let test_media = v.get("mediaUrl").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    if !test_ytid.is_empty() && !media_type_enabled(&ctx.cfg(), "youtube") {
        let _ = request.respond(json_resp(400, json!({ "ok": false, "error": "YouTube links are disabled" })));
        return;
    }
    if let Some(provider) = media_provider(&test_media) {
        if !media_type_enabled(&ctx.cfg(), provider) {
            let _ = request.respond(json_resp(400, json!({ "ok": false, "error": "This media type is disabled" })));
            return;
        }
    }
    let test_media = other_media_url(&test_media).map(|u| resolve_short_media(&u)).unwrap_or_default();
    let test_style = v.get("style").cloned().unwrap_or(serde_json::Value::Null);
    let message = v.get("message").and_then(|x| x.as_str()).filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string()).unwrap_or_else(|| "Шановна Служба Безпеки України та голова СБ України! Я не маю відношення до цього каналу та чату і був доданий сюди без моєї згоди.".to_string());
    if let Some(b) = v.get("bit") {
        if let Ok(bit) = serde_json::from_value::<Bit>(b.clone()) {
            ctx.broadcast(json!({ "type": "bit", "data": {
                "id": format!("bittest-{}", now_ms()), "name": name, "amount": amount,
                "message": message, "currency": "", "bit": bit } }));
            let _ = request.respond(json_resp(200, json!({ "ok": true })));
            return;
        }
    }
    // speak the test donation (synth first, so the alert and audio appear together)
    let cfg = ctx.cfg();
    let mut tts_url = String::new();
    if cfg.tts.enabled() {
        let allowed = cfg.tts.allowed_in(&cfg.tts.dir);
        let voice = v.get("voice").and_then(|x| x.as_str()).filter(|s| allowed.contains(&s.to_string()))
            .map(|s| s.to_string())
            .unwrap_or_else(|| { let d = cfg.tts.default_voice(); if d.is_empty() { allowed.first().cloned().unwrap_or_default() } else { d } });
        let money = format!("{:.0} {}", amount, "");
        let money_text = pick_style(&cfg.notify.rules, amount).money_text;
        let spoken = crate::tts::spoken_text(&cfg.tts.read_mode, &name, &money, &message, &money_text);
        if !spoken.trim().is_empty() && !voice.is_empty() {
            let dir = ctx.paths.base.join("tts");
            let _ = std::fs::create_dir_all(&dir);
            let fname = format!("test-{}.wav", now_ms());
            if crate::tts::synth(&cfg.tts.dir, &voice, &spoken, &dir.join(&fname), &cfg.tts.azure_key, &cfg.tts.azure_region, cfg.tts.speed).is_ok() {
                tts_url = crate::model::tts_url(&cfg, &fname);
            }
        }
    }
    ctx.broadcast(json!({ "type": "donation", "data": {
        "id": format!("test-{}", now_ms()), "name": name, "amount": amount,
        "message": message, "youtubeId": test_ytid, "mediaUrl": test_media, "ruleId": test_rule, "style": test_style, "matched": "test", "jar": "", "currency": "",
        "ttsUrl": tts_url }}));
    let _ = request.respond(json_resp(200, json!({ "ok": true })));
}

/// Called by the donation overlay after the alert (and TTS) finished; the YouTube
/// overlay listens for the resulting event, so the video never competes with the alert audio.
fn media_allowed_with_style(style: &NotifyCfg, amount: f64, yt: &str, has_other: bool) -> Result<(), String> {
    if !yt.is_empty() {
        if style.yt_min > 0.0 && amount < style.yt_min {
            return Err(format!("YouTube requires at least {:.0} UAH", style.yt_min));
        }
        if let Some((views, secs)) = mono::youtube_meta(yt) {
            if style.yt_views > 0 && views < style.yt_views { return Err(format!("video needs at least {} views", style.yt_views)); }
            if style.yt_max_min > 0 && secs > style.yt_max_min * 60 { return Err(format!("video must be at most {} min", style.yt_max_min)); }
        }
        Ok(())
    } else if has_other {
        if style.other_media_min > 0.0 && amount < style.other_media_min {
            return Err(format!("other media requires at least {:.0} UAH", style.other_media_min));
        }
        Ok(())
    } else {
        Err("no media".into())
    }
}

fn media_allowed(cfg: &Config, amount: f64, rule: &str, yt: &str, has_other: bool) -> Result<(), String> {
    let style = if !rule.is_empty() {
        cfg.notify.rules.iter().find(|r| r.id == rule).map(|r| r.style.clone())
    } else { None }.unwrap_or_else(|| pick_style(&cfg.notify.rules, amount));
    media_allowed_with_style(&style, amount, yt, has_other)
}

static YT_LAST_PLAY: OnceLock<Mutex<(String, i64)>> = OnceLock::new();
fn yt_play_gate(key: &str) -> bool {
    let gate = YT_LAST_PLAY.get_or_init(|| Mutex::new((String::new(), 0)));
    let mut g = gate.lock().unwrap();
    let now = now_ms();
    if g.0 == key && now - g.1 < 2000 { return false; }
    *g = (key.to_string(), now);
    true
}
fn yt_recent_play() -> bool {
    let gate = YT_LAST_PLAY.get_or_init(|| Mutex::new((String::new(), 0)));
    let g = gate.lock().unwrap();
    !g.0.is_empty() && now_ms() - g.1 < 2000
}

fn youtube_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    if v.get("stop").and_then(|x| x.as_bool()).unwrap_or(false) {
        if !yt_recent_play() {
            ctx.broadcast(json!({ "type": "youtube", "data": { "stop": true } }));
        }
        let _ = request.respond(json_resp(200, json!({ "ok": true })));
        return;
    }
    let id = v.get("youtubeId").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let media = v.get("mediaUrl").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let amount = v.get("amount").and_then(|x| x.as_f64()).unwrap_or(0.0);
    let force = v.get("force").and_then(|x| x.as_bool()).unwrap_or(false);
    let rule = v.get("ruleId").and_then(|x| x.as_str()).unwrap_or("");
    if !id.is_empty() && !media_type_enabled(&ctx.cfg(), "youtube") {
        let _ = request.respond(json_resp(200, json!({ "ok": false, "error": "YouTube links are disabled" })));
        return;
    }
    if let Some(provider) = media_provider(&media) {
        if !media_type_enabled(&ctx.cfg(), provider) {
            let _ = request.respond(json_resp(200, json!({ "ok": false, "error": "This media type is disabled" })));
            return;
        }
    }
    let media = if id.is_empty() {
        crate::model::other_media_url(&media).map(|u| {
            if x_status_id(&u).is_some() {
                cache_x_video(ctx, &u).unwrap_or_else(|| resolve_other_media(ctx, &u))
            } else { resolve_other_media(ctx, &u) }
        })
    } else { None };
    if !crate::model::is_youtube_id(&id) && media.is_none() {
        let _ = request.respond(json_resp(400, json!({ "ok": false, "error": "invalid media" })));
        return;
    }
    if !ctx.cfg().features.youtube {
        let _ = request.respond(json_resp(200, json!({ "ok": false, "error": "youtube overlay disabled" })));
        return;
    }
    if !force {
        if let Err(e) = media_allowed(&ctx.cfg(), amount, rule, &id, media.is_some()) {
            log(&format!("youtube: blocked {}: {}", if id.is_empty() { "other media" } else { &id }, e));
            let _ = request.respond(json_resp(200, json!({ "ok": false, "error": e })));
            return;
        }
    }
    let key = format!("{}|{}|{}", id, media.as_deref().unwrap_or(""), rule);
    if !yt_play_gate(&key) {
        let _ = request.respond(json_resp(200, json!({ "ok": true, "duplicate": true })));
        return;
    }
    ctx.broadcast(json!({ "type": "youtube", "data": {
        "youtubeId": id, "mediaUrl": media.unwrap_or_default(), "amount": amount, "ruleId": rule } }));
    log(&format!("youtube: play {} after alert", if id.is_empty() { "other media" } else { &id }));
    let _ = request.respond(json_resp(200, json!({ "ok": true })));
}

fn replay(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
    let mode = v.get("mode").and_then(|x| x.as_str()).unwrap_or("donation");
    let found = {
        let st = ctx.state.lock().unwrap();
        let mut hit = None;
        for j in st.jars.iter() {
            if let Some(h) = j.history.iter().find(|h| h.id == id) {
                hit = Some((h.clone(), j.currency));
                break;
            }
        }
        hit
    };
    match found {
        Some((h, currency)) => {
            let cfg = ctx.cfg();
            let mut bits: Vec<crate::model::Bit> = h.bit_code.split(',')
                .filter(|c| !c.is_empty())
                .filter_map(|c| cfg.bits.iter().find(|b| b.code == c).cloned())
                .collect();
            if bits.is_empty() && !h.bit_label.is_empty() {
                for label in h.bit_label.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    if let Some(b) = cfg.bits.iter().find(|b| !b.label.is_empty() && b.label == label) {
                        bits.push(b.clone());
                    }
                }
            }
            let bit = bits.first().cloned();
            let sound_urls: Vec<String> = bits.iter()
                .map(|b| if b.kind == "sound" { b.url.clone() } else { b.sound.clone() })
                .filter(|s| !s.is_empty())
                .collect();
            let replay_id = format!("replay-{}", now_ms());

            let direct_bit = h.matched == "bit";
            if mode == "sound" || (mode == "full" && direct_bit) {
                if let Some(b) = bit.clone() {
                    ctx.broadcast(json!({ "type": "bit", "data": {
                        "id": replay_id, "name": h.name, "amount": h.amount,
                        "message": h.message, "currency": "", "bit": b } }));
                    log(&format!("replay sound: {} for {}", h.bit_label, h.name));
                }
                let _ = request.respond(json_resp(200, json!({ "ok": true })));
                return;
            }

            // voice for the replay, same rules as a live donation
            let mut tts_url = String::new();
            if cfg.tts.enabled() {
                let cur = crate::model::currency_symbol(currency);
                let money = format!("{:.0} {}", h.amount, cur);
                let spoken = crate::tts::spoken_text(&cfg.tts.read_mode, &h.name, &money, &h.message, "");
                let allowed = cfg.tts.allowed_in(&cfg.tts.dir);
                let voice = {
                    let d = cfg.tts.default_voice();
                    if d.is_empty() { allowed.first().cloned().unwrap_or_default() } else { d }
                };
                if !spoken.trim().is_empty() && !voice.is_empty() {
                    let dir = ctx.paths.base.join("tts");
                    let _ = std::fs::create_dir_all(&dir);
                    let fname = format!("replay-{}.wav", now_ms());
                    if crate::tts::synth(&cfg.tts.dir, &voice, &spoken, &dir.join(&fname),
                        &cfg.tts.azure_key, &cfg.tts.azure_region, cfg.tts.speed).is_ok() {
                        tts_url = crate::model::tts_url(&cfg, &fname);
                    }
                }
            }

            let (yt, media, sounds) = if mode == "full" {
                (h.youtube_id.clone(), h.media_url.clone(), sound_urls.clone())
            } else {
                (String::new(), String::new(), Vec::new())
            };
            ctx.broadcast(json!({ "type": "donation", "data": {
                "id": replay_id, "name": h.name, "amount": h.amount, "message": h.message,
                "youtubeId": yt, "mediaUrl": media, "soundUrls": sounds, "soundMode": "order",
                "soundUrl": "", "ttsUrl": tts_url,
                "matched": "replay", "jar": "", "currency": "" } }));
            log(&format!("replay {}: {:.0} from {}", mode, h.amount, h.name));
            let _ = request.respond(json_resp(200, json!({ "ok": true })));
        }
        None => { let _ = request.respond(json_resp(404, json!({ "ok": false, "error": "not found" }))); }
    }
}

fn refresh(ctx: &Ctx, request: tiny_http::Request) {
    if let Err(e) = ctx.refresh_jars() { log(&format!("refresh: {}", e)); }
    ctx.poll_once();
    let _ = request.respond(json_resp(200, state_json(ctx)));
}

/// Full donation history for one jar (newest first), fetched on demand by the stats page
/// for client-side analytics — not part of the 4s state poll.
fn history_json(ctx: &Ctx, jar: usize) -> serde_json::Value {
    let st = ctx.state.lock().unwrap();
    let items: Vec<serde_json::Value> = match st.jars.get(jar) {
        Some(j) => j.history.iter().map(|h| json!({
            "id": h.id, "name": h.name, "amount": h.amount, "message": h.message,
            "youtubeId": h.youtube_id, "mediaUrl": h.media_url, "bitLabel": h.bit_label, "bitCode": h.bit_code, "matched": h.matched, "at": h.at })).collect(),
        None => Vec::new(),
    };
    json!({ "jar": jar, "count": items.len(), "items": items })
}

/* ---------------------------------------------------------------- marathon timer */

fn timer_json(ctx: &Ctx) -> serde_json::Value {
    let cfg = ctx.cfg();
    let st = ctx.state.lock().unwrap();
    let t = &st.timer;
    let log: Vec<serde_json::Value> = t.log.iter().take(60).map(|e| json!({
        "source": e.source, "label": e.label, "amount": e.amount, "seconds": e.seconds, "at": e.at })).collect();
    json!({
        "running": t.running, "endsAt": t.ends_at, "remainingMs": t.remaining(), "now": now_ms(),
        "subs": t.subs, "money": t.money, "log": log,
        "cfg": {
            "perAmount": cfg.timer.per_amount, "perSeconds": cfg.timer.per_seconds,
            "minDonation": cfg.timer.min_donation, "pollSeconds": cfg.timer.poll_seconds,
            "twitchTier1": cfg.timer.twitch_tier1, "twitchTier2": cfg.timer.twitch_tier2, "twitchTier3": cfg.timer.twitch_tier3,
            "color": cfg.timer.color, "accent": cfg.timer.accent, "outline": cfg.timer.outline,
            "size": cfg.timer.size, "font": cfg.timer.font,
            "link": timer_link(&cfg)
        }
    })
}

fn timer_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("");
    let seconds = v.get("seconds").and_then(|x| x.as_f64()).unwrap_or(0.0);
    let ms = (seconds * 1000.0).round() as i64;
    {
        let mut st = ctx.state.lock().unwrap();
        let t = &mut st.timer;
        match action {
            "set" => t.set(ms),
            "add" => { t.add(ms); t.push_log(TimerEvent { source: "manual".into(), label: String::new(), amount: 0.0, seconds: ms / 1000, at: now_ms() }); }
            "sub" => { t.add(-ms); t.push_log(TimerEvent { source: "manual".into(), label: String::new(), amount: 0.0, seconds: -ms / 1000, at: now_ms() }); }
            "pause" => t.pause(),
            "resume" | "start" => t.resume(),
            "reset" => { t.running = false; t.remaining_ms = 0; t.ends_at = 0; }
            _ => {}
        }
    }
    ctx.save();
    let _ = request.respond(json_resp(200, timer_json(ctx)));
}

fn timer_settings_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let mut cfg = ctx.cfg();
    let t = &mut cfg.timer;
    if let Some(x) = v.get("perAmount").and_then(|x| x.as_f64()) { if x > 0.0 { t.per_amount = x; } }
    if let Some(x) = v.get("perSeconds").and_then(|x| x.as_f64()) { if x >= 0.0 { t.per_seconds = x; } }
    if let Some(x) = v.get("minDonation").and_then(|x| x.as_f64()) { if x >= 0.0 { t.min_donation = x; } }
    if let Some(x) = v.get("pollSeconds").and_then(|x| x.as_u64()) { t.poll_seconds = x.clamp(15, 86_400); }
    if let Some(x) = v.get("twitchTier1").and_then(|x| x.as_u64()) { t.twitch_tier1 = x; }
    if let Some(x) = v.get("twitchTier2").and_then(|x| x.as_u64()) { t.twitch_tier2 = x; }
    if let Some(x) = v.get("twitchTier3").and_then(|x| x.as_u64()) { t.twitch_tier3 = x; }
    if let Some(s) = v.get("color").and_then(|x| x.as_str()) { if !s.trim().is_empty() { t.color = s.trim().into(); } }
    if let Some(s) = v.get("accent").and_then(|x| x.as_str()) { if !s.trim().is_empty() { t.accent = s.trim().into(); } }
    if let Some(s) = v.get("outline").and_then(|x| x.as_str()) { if !s.trim().is_empty() { t.outline = s.trim().into(); } }
    if let Some(x) = v.get("size").and_then(|x| x.as_f64()) { if x > 0.0 { t.size = x; } }
    if let Some(s) = v.get("font").and_then(|x| x.as_str()) { if !s.trim().is_empty() { t.font = s.trim().into(); } }
    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg.clone();
    log("timer settings saved");
    let _ = request.respond(json_resp(200, json!({ "ok": true })));
}

/* ---------------------------------------------------------------- features */

fn tts_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let mut cfg = ctx.cfg();
    let t = &mut cfg.tts;
    t.mode = if v.get("mode").and_then(|x| x.as_str()) == Some("off") { "off".into() } else { "google:uk".into() };
    t.voice = "google:uk".into();
    t.azure_key.clear();
    t.azure_region.clear();
    if let Some(s) = v.get("readMode").and_then(|x| x.as_str()) { if !s.trim().is_empty() { t.read_mode = s.trim().into(); } }
    if let Some(n) = v.get("speed").and_then(|x| x.as_f64()) { t.speed = n.clamp(0.5, 2.0); }
    if let Some(s) = v.get("dir").and_then(|x| x.as_str()) { if !s.trim().is_empty() { t.dir = s.trim().into(); } }
    if let Some(s) = v.get("azureKey").and_then(|x| x.as_str()) { t.azure_key = s.trim().into(); }
    if let Some(s) = v.get("azureRegion").and_then(|x| x.as_str()) { t.azure_region = s.trim().into(); }
    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg.clone();
    log("tts settings saved");
    let _ = request.respond(json_resp(200, json!({ "ok": true })));
}

fn features_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let mut cfg = ctx.cfg();
    let f = &mut cfg.features;
    if let Some(b) = v.get("donate").and_then(|x| x.as_bool()) { f.donate = b; }
    if let Some(b) = v.get("stats").and_then(|x| x.as_bool()) { f.stats = b; }
    if let Some(b) = v.get("overlay").and_then(|x| x.as_bool()) { f.overlay = b; }
    if let Some(b) = v.get("bits").and_then(|x| x.as_bool()) { f.bits = b; }
    if let Some(b) = v.get("bitsOverlay").and_then(|x| x.as_bool()) { f.bits_overlay = b; }
    if let Some(b) = v.get("youtube").and_then(|x| x.as_bool()) { f.youtube = b; }
    if let Some(b) = v.get("timer").and_then(|x| x.as_bool()) { f.timer = b; }
    if let Some(b) = v.get("auction").and_then(|x| x.as_bool()) { f.auction = b; }
    if let Some(b) = v.get("qr").and_then(|x| x.as_bool()) { f.qr = b; }
    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg.clone();
    log("features saved");
    let _ = request.respond(json_resp(200, json!({ "ok": true })));
}

/* ---------------------------------------------------------------- auction */

fn auction_json(ctx: &Ctx) -> serde_json::Value {
    let cfg = ctx.cfg();
    let st = ctx.state.lock().unwrap();
    let a = &st.auction;
    let (ui, settings) = auction_links(&cfg);
    json!({
        "active": a.active, "title": a.title, "endsAt": a.ends_at, "spin": a.spin, "now": now_ms(),
        "winner": a.winner, "bids": a.bids, "total": a.total(),
        "cfg": {
            "timerSeconds": cfg.auction.timer_seconds, "autoEnd": cfg.auction.auto_end,
            "minBid": cfg.auction.min_bid, "rewardCosts": cfg.auction.reward_costs,
            "rewardTitle": cfg.auction.reward_title, "rewardPrompt": cfg.auction.reward_prompt,
            "title": cfg.auction.title, "color": cfg.auction.color, "accent": cfg.auction.accent,
            "outline": cfg.auction.outline, "font": cfg.auction.font,
        },
        "links": { "ui": ui, "settings": settings }
    })
}

fn auc_sync_cfg(c: &AuctionCfg, a: &mut crate::auction::AuctionState) {
    a.settings.mode = c.mode.clone();
    a.settings.source = c.source.clone();
    a.settings.currency = c.currency.clone();
    if c.timer_seconds > 0 { a.settings.timer_seconds = c.timer_seconds; }
    a.settings.show_total_time = c.show_total_time;
    a.settings.leader_change_points = c.leader_change_points;
    a.settings.leader_change_donations = c.leader_change_donations;
    a.settings.new_position_points = c.new_position_points;
    a.settings.new_position_donations = c.new_position_donations;
    a.settings.add_existing_points = c.add_existing_points;
    a.settings.add_existing_donations = c.add_existing_donations;
    a.settings.time_more_than_min = c.time_more_than_min;
}
fn auc_json(ctx: &Ctx) -> serde_json::Value {
    let c = ctx.cfg().auction.clone();
    let mut st = ctx.state.lock().unwrap();
    auc_sync_cfg(&c, &mut st.auc);
    let mut v = serde_json::to_value(&st.auc).unwrap_or(json!({}));
    if let Some(s) = v.get_mut("settings").and_then(|x| x.as_object_mut()) {
        s.insert("timer".into(), json!({
            "showTotalTime": c.show_total_time,
            "leaderChangePoints": c.leader_change_points, "leaderChangeDonations": c.leader_change_donations,
            "newPositionPoints": c.new_position_points, "newPositionDonations": c.new_position_donations,
            "addExistingPoints": c.add_existing_points, "addExistingDonations": c.add_existing_donations,
            "timeMoreThanMin": c.time_more_than_min,
        }));
        s.insert("appearance".into(), json!({
            "background": "default", "geometry": "default", "primaryColor": "#ffffff",
            "showTotalAmount": false, "showWinningChances": false, "showViewerNames": true,
            "autoParseLinks": false, "hideValues": false,
        }));
    }
    v
}
fn auc_mut<F: FnOnce(&mut crate::auction::AuctionState)>(ctx: &Ctx, f: F) {
    let c = ctx.cfg().auction.clone();
    { let mut st = ctx.state.lock().unwrap(); auc_sync_cfg(&c, &mut st.auc); f(&mut st.auc); }
    ctx.save();
}

fn auction_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("");
    let cfg = ctx.cfg();
    {
        let mut st = ctx.state.lock().unwrap();
        let a = &mut st.auction;
        match action {
            "start" => {
                a.active = true; a.winner = None;
                a.title = if cfg.auction.title.trim().is_empty() { "Auction".into() } else { cfg.auction.title.clone() };
                a.ends_at = if cfg.auction.timer_seconds > 0 { now_ms() + (cfg.auction.timer_seconds as i64) * 1000 } else { 0 };
            }
            "stop" => { a.active = false; a.ends_at = 0; }
            "clear" => { a.bids.clear(); a.winner = None; }
            "reset" => { a.active = false; a.ends_at = 0; a.bids.clear(); a.winner = None; a.title.clear(); }
            "remove" => { if let Some(name) = v.get("name").and_then(|x| x.as_str()) { let k = donor_key(name); a.bids.retain(|b| donor_key(&b.name) != k); } }
            "bid" => { let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("manual").to_string();
                let amount = v.get("amount").and_then(|x| x.as_f64()).unwrap_or(0.0); a.add_bid(&name, amount, "manual"); }
            "top" => { a.winner = a.bids.iter().cloned().max_by(|x, y| x.amount.partial_cmp(&y.amount).unwrap_or(std::cmp::Ordering::Equal)); }
            "spin" => { a.winner = weighted_pick(&a.bids); a.spin += 1; }
            _ => {}
        }
    }
    ctx.save();
    let _ = request.respond(json_resp(200, auction_json(ctx)));
}

fn auction_settings_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let mut cfg = ctx.cfg();
    let a = &mut cfg.auction;
    if let Some(x) = v.get("timerSeconds").and_then(|x| x.as_u64()) { a.timer_seconds = x; }
    if let Some(b) = v.get("autoEnd").and_then(|x| x.as_bool()) { a.auto_end = b; }
    if let Some(x) = v.get("minBid").and_then(|x| x.as_f64()) { if x >= 0.0 { a.min_bid = x; } }
    if let Some(arr) = v.get("rewardCosts").and_then(|x| x.as_array()) { a.reward_costs = arr.iter().filter_map(|x| x.as_u64()).collect(); }
    if let Some(s) = v.get("rewardTitle").and_then(|x| x.as_str()) { a.reward_title = s.trim().into(); }
    if let Some(s) = v.get("rewardPrompt").and_then(|x| x.as_str()) { a.reward_prompt = s.trim().into(); }
    if let Some(s) = v.get("title").and_then(|x| x.as_str()) { a.title = s.trim().into(); }
    if let Some(s) = v.get("color").and_then(|x| x.as_str()) { if !s.trim().is_empty() { a.color = s.trim().into(); } }
    if let Some(s) = v.get("accent").and_then(|x| x.as_str()) { if !s.trim().is_empty() { a.accent = s.trim().into(); } }
    if let Some(s) = v.get("outline").and_then(|x| x.as_str()) { if !s.trim().is_empty() { a.outline = s.trim().into(); } }
    if let Some(s) = v.get("font").and_then(|x| x.as_str()) { a.font = s.trim().into(); }
    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg.clone();
    log("auction settings saved");
    let _ = request.respond(json_resp(200, json!({ "ok": true })));
}

/* ---------------------------------------------------------------- twitch */

fn twitch_json(_ctx: &Ctx, port: u16) -> serde_json::Value {
    let s = load_twitch_at(port);
    json!({
        "clientId": s.client_id, "clientSecret": s.client_secret,
        "hasSecret": !s.client_secret.is_empty(), "authorized": !s.access_token.is_empty(),
        "connected": crate::twitch::is_connected(port),
        "login": s.login, "broadcasterId": s.broadcaster_id,
        "redirect": crate::twitch::redirect_uri()
    })
}

fn twitch_post(_ctx: &Ctx, mut request: tiny_http::Request, port: u16) {
    let v = read_body(&mut request);
    let mut sec = load_twitch_at(port);
    if let Some(s) = v.get("clientId").and_then(|x| x.as_str()) { sec.client_id = s.trim().to_string(); }
    if let Some(s) = v.get("secret").and_then(|x| x.as_str()) { sec.client_secret = s.trim().to_string(); }
    save_twitch_at(port, &sec);
    log(&format!("twitch settings saved [{}]", port));
    let _ = request.respond(json_resp(200, json!({ "ok": true })));
}

fn twitch_callback(ctx: &Ctx, request: tiny_http::Request, full: &str, port: u16) {
    let code = query(full, "code").unwrap_or_default();
    let state = query(full, "state").unwrap_or_default();
    let err = query(full, "error").unwrap_or_default();
    let page = |msg: &str, ok: bool| {
        let color = if ok { "#39d98a" } else { "#ff6b6b" };
        format!("<!doctype html><meta charset=\"utf-8\"><title>Twitch — LocalMonoDonate</title>\
            <body style=\"background:#0a0a0b;color:#ededed;font-family:system-ui;display:flex;height:100vh;align-items:center;justify-content:center;margin:0\">\
            <div style=\"text-align:center\"><div style=\"font-size:44px;font-weight:800;color:{}\">{}</div>\
            <div style=\"color:#7d7d86;margin-top:8px\">LocalMonoDonate</div></div></body>", color, msg)
    };
    let _ = ctx;
    if !err.is_empty() { let _ = request.respond(body_resp(200, "text/html; charset=utf-8", page("Доступ відхилено", false).into_bytes())); return; }
    if code.is_empty() { let _ = request.respond(body_resp(400, "text/html; charset=utf-8", page("Немає коду", false).into_bytes())); return; }
    if !crate::twitch::check_state(&state) { log("twitch: oauth state mismatch (continuing)"); }
    match crate::twitch::exchange_code_at(port, &code) {
        Ok(s) => { log(&format!("twitch[{}]: authorized as {}", port, s.login));
            let _ = request.respond(body_resp(200, "text/html; charset=utf-8", page("Авторизовано \u{2713}", true).into_bytes())); }
        Err(e) => { log(&format!("twitch[{}]: auth failed: {}", port, e));
            let _ = request.respond(body_resp(200, "text/html; charset=utf-8", page("Помилка авторизації", false).into_bytes())); }
    }
}

/* ---------------------------------------------------------------- streamelements */

fn se_json(ctx: &Ctx, port: u16) -> serde_json::Value {
    let cfg = ctx.cfg();
    let sec = load_streamelements_at(port);
    json!({
        "enabled": cfg.streamelements.enabled,
        "accountId": cfg.streamelements.account_id,
        "token": sec.token,
        "hasToken": !sec.token.trim().is_empty()
    })
}

fn se_post(ctx: &Ctx, mut request: tiny_http::Request, port: u16) {
    let v = read_body(&mut request);
    let mut cfg = ctx.cfg();
    if let Some(b) = v.get("enabled").and_then(|x| x.as_bool()) { cfg.streamelements.enabled = b; }
    if let Some(s) = v.get("accountId").and_then(|x| x.as_str()) { cfg.streamelements.account_id = s.trim().to_string(); }
    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg;
    if let Some(t) = v.get("token").and_then(|x| x.as_str()) {
        let mut sec = load_streamelements_at(port);
        sec.token = t.trim().to_string();
        save_streamelements_at(port, &sec);
    }
    log(&format!("streamelements settings saved [{}]", port));
    let _ = request.respond(json_resp(200, json!({ "ok": true })));
}

fn se_test(ctx: &Ctx, mut request: tiny_http::Request, port: u16) {
    let v = read_body(&mut request);
    let cfg = ctx.cfg();
    let account = v.get("accountId").and_then(|x| x.as_str()).map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()).unwrap_or_else(|| cfg.streamelements.account_id.clone());
    let token = v.get("token").and_then(|x| x.as_str()).map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()).unwrap_or_else(|| load_streamelements_at(port).token);
    match crate::streamelements::channel(&account, &token) {
        Ok(ch) => {
            log("streamelements: connection ok");
            let _ = request.respond(json_resp(200, json!({ "ok": true, "channel": ch })));
        }
        Err(e) => {
            log(&format!("streamelements: test failed: {}", e));
            let _ = request.respond(json_resp(200, json!({ "ok": false, "error": e })));
        }
    }
}

/* ---------------------------------------------------------------- config */

fn qr_get(ctx: &Ctx, request: tiny_http::Request) {
    let cfg = ctx.cfg();
    let _ = request.respond(json_resp(200, json!({
        "qrWidgets": cfg.qr_widgets, "qrAlign": cfg.qr_align, "qrSize": cfg.qr_size
    })));
}

fn qr_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let mut cfg = ctx.cfg();
    if let Some(arr) = v.get("qrWidgets").and_then(|x| x.as_array()) {
        let widgets = normalize_qr_widgets(&cfg, arr);
        cfg.widget_jar = widgets[0].jar.clone();
        cfg.qr_widgets = widgets;
    }
    if let Some(s) = v.get("qrAlign").and_then(|x| x.as_str()) { cfg.qr_align = normalize_qr_align(s); }
    if let Some(n) = v.get("qrSize").and_then(|x| x.as_u64()) { cfg.qr_size = (n as u32).min(200); }
    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg.clone();
    log("qr widget settings saved");
    let _ = request.respond(json_resp(200, json!({
        "ok": true, "qrWidgets": cfg.qr_widgets, "qrAlign": cfg.qr_align, "qrSize": cfg.qr_size
    })));
}

fn config_get(ctx: &Ctx, public: bool) -> serde_json::Value {
    let cfg = ctx.cfg();
    let (donate, stats, overlay) = links(&cfg);
    let (bits_page, bits_overlay) = bits_links(&cfg);
    // when Tailscale is connected, its node name (== nickname) is the public base
    let ts = crate::tailscale::status();
    let public_base = match ts.get("dnsName").and_then(|x| x.as_str()) {
        Some(d) if !d.is_empty() => format!("https://{}", d),
        _ => cfg.public_base.clone(),
    };
    let public_donate = if public_base.is_empty() {
        donate.clone()
    } else {
        let base = if public_base.starts_with("http") { public_base.clone() } else { format!("https://{}", public_base) };
        format!("{}/{}/donation", base.trim_end_matches('/'), sanitize_name(&cfg.nickname))
    };
    let qr_links: Vec<String> = (0..MAX_QR_WIDGETS).map(|i| qr_link_at(&cfg, i)).collect();
    let mut v = json!({
        "nickname": cfg.nickname, "overlayname": cfg.overlayname,
        "publicDonate": public_donate,
        "statsToken": cfg.stats_token, "port": cfg.port, "publicPort": cfg.public_port, "publicBase": public_base,
        "tailscale": ts,
        "token": cfg.token, "bg": cfg.bg, "bitsBg": cfg.bits_bg, "bitsPageBg": cfg.bits_page_bg, "bitsGrid": cfg.bits_grid, "jars": cfg.jars, "jarNames": cfg.jar_names, "jarMine": cfg.jar_mine, "maxJars": MAX_JARS, "widgetJar": cfg.widget_jar,
        "qrWidgets": cfg.qr_widgets, "qrAlign": cfg.qr_align, "qrSize": cfg.qr_size, "qrLinks": qr_links,
        "features": cfg.features,
        "links": { "donate": donate, "stats": stats, "overlay": overlay, "qr": qr_link(&cfg), "bitsPage": bits_page, "bitsOverlay": bits_overlay, "youtube": youtube_link(&cfg) }
    });
    // the public listener must never hand out the Monobank token or the stats link
    if public {
        if let Some(o) = v.as_object_mut() {
            o.remove("token");
            o.remove("statsToken");
            o.remove("qrWidgets");
            o.remove("qrAlign");
            o.remove("qrSize");
            o.remove("qrLinks");
            if let Some(l) = o.get_mut("links").and_then(|x| x.as_object_mut()) {
                for k in ["stats", "overlay", "bitsOverlay", "youtube", "qr"] { l.remove(k); }
            }
        }
    }
    v
}

fn valid_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn normalize_qr_widgets(cfg: &Config, arr: &[serde_json::Value]) -> Vec<QrWidgetCfg> {
    let mut widgets: Vec<QrWidgetCfg> = arr.iter().take(MAX_QR_WIDGETS)
        .filter_map(|x| serde_json::from_value::<QrWidgetCfg>(x.clone()).ok())
        .map(|mut w| {
            w.jar = w.jar.trim().to_string();
            if !cfg.jars.iter().any(|j| j == &w.jar) { w.jar = cfg.jars.first().cloned().unwrap_or_default(); }
            if !valid_hex_color(&w.color) { w.color = "#101217".into(); }
            if !matches!(w.style.as_str(), "both" | "qr" | "goal") { w.style = "both".into(); }
            w.radius = w.radius.min(64);
            if !valid_hex_color(&w.accent) { w.accent = "#8b5cf6".into(); }
            w
        })
        .collect();
    if widgets.is_empty() {
        widgets.push(QrWidgetCfg { jar: cfg.jars.first().cloned().unwrap_or_default(), ..Default::default() });
    }
    widgets
}

fn normalize_qr_align(s: &str) -> String {
    match s.trim().to_ascii_lowercase().as_str() {
        "top" | "mid" | "bottom" => s.trim().to_ascii_lowercase(),
        _ => "bottom".into(),
    }
}

fn config_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let mut cfg = ctx.cfg();
    set_secrets_port(cfg.port);
    if let Some(s) = v.get("nickname").and_then(|x| x.as_str()) { let s = sanitize_name(s); if !s.is_empty() { cfg.nickname = s; } }
    if let Some(s) = v.get("overlayname").and_then(|x| x.as_str()) { let s = sanitize_name(s); if !s.is_empty() { cfg.overlayname = s; } }
    if let Some(s) = v.get("token").and_then(|x| x.as_str()) { cfg.token = s.trim().to_string(); save_token(&cfg.token); }
    if let Some(arr) = v.get("jars").and_then(|x| x.as_array()) {
        let list: Vec<String> = arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
        let norm = normalize_jars(&list);
        // an empty list is a valid choice: the previous jars must not survive
        cfg.jars = norm;
    }
    if let Some(arr) = v.get("jarNames").and_then(|x| x.as_array()) {
        cfg.jar_names = arr.iter().map(|x| x.as_str().unwrap_or("").trim().to_string()).collect();
    }
    if let Some(arr) = v.get("jarMine").and_then(|x| x.as_array()) {
        cfg.jar_mine = arr.iter().map(|x| x.as_bool().unwrap_or(true)).collect();
    }
    if let Some(s) = v.get("widgetJar").and_then(|x| x.as_str()) { cfg.widget_jar = s.trim().to_string(); }
    if let Some(arr) = v.get("qrWidgets").and_then(|x| x.as_array()) {
        let widgets = normalize_qr_widgets(&cfg, arr);
        cfg.widget_jar = widgets[0].jar.clone();
        cfg.qr_widgets = widgets;
    }
    if let Some(s) = v.get("qrAlign").and_then(|x| x.as_str()) { cfg.qr_align = normalize_qr_align(s); }
    if let Some(n) = v.get("qrSize").and_then(|x| x.as_u64()) { cfg.qr_size = (n as u32).min(200); }
    if let Some(s) = v.get("publicBase").and_then(|x| x.as_str()) {
        let s = s.trim().trim_end_matches('/');
        // accept a bare host ("ratcorporation.us.kg") or a full URL
        cfg.public_base = if s.is_empty() || s.starts_with("http") { s.to_string() } else { format!("https://{}", s) };
    }
    if let Some(n) = v.get("publicPort").and_then(|x| x.as_u64()) { cfg.public_port = n as u16; }
    // stats code is created only when the user asks for it (Setup -> Stats -> â†»)

    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg.clone();
    ctx.sync_jars();
    {
        let mut st = ctx.state.lock().unwrap();
        for j in st.jars.iter_mut() { j.statement_account.clear(); j.last_ts = 0; }
    }
    let _ = ctx.refresh_jars();
    ctx.poll_once();

    let (donate, stats, overlay) = links(&cfg);
    log(&format!("config saved: nick={} overlay={} jars={:?}", cfg.nickname, cfg.overlayname, cfg.jars));
    let _ = request.respond(json_resp(200, json!({ "ok": true,
        "links": { "donate": donate, "stats": stats, "overlay": overlay } })));
}

fn newstats_post(ctx: &Ctx, request: tiny_http::Request) {
    let mut cfg = ctx.cfg();
    cfg.stats_token = gen_stats_token();
    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg.clone();
    let (_, stats, _) = links(&cfg);
    log("stats token regenerated");
    let _ = request.respond(json_resp(200, json!({ "ok": true, "statsToken": cfg.stats_token, "stats": stats })));
}

fn bg_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let url = v.get("url").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
    let which = v.get("which").and_then(|x| x.as_str()).unwrap_or("page");
    let mut cfg = ctx.cfg();
    match which {
        "bits" => cfg.bits_bg = url.clone(),
        "bitspage" => cfg.bits_page_bg = url.clone(),
        "bitspagedefault" => cfg.bits_page_bg = String::new(),
        "bitsdefault" => cfg.bits_bg = String::new(),
        _ => cfg.bg = url.clone(),
    }
    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg.clone();
    let _ = request.respond(json_resp(200, json!({ "ok": true, "which": which, "url": url })));
}

fn bits_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    let mut cfg = ctx.cfg();
    if let Some(arr) = v.get("bits").and_then(|x| x.as_array()) {
        let mut bits: Vec<Bit> = Vec::new();
        for b in arr {
            if let Ok(mut bit) = serde_json::from_value::<Bit>(b.clone()) {
                if bit.slot >= 25 { continue; }
                if bit.code.trim().is_empty() { bit.code = gen_bit_code(); }
                if bit.kind != "sound" && bit.kind != "video" && bit.kind != "image" && bit.kind != "alert" { bit.kind = "alert".into(); }
                bits.push(bit);
            }
        }
        cfg.bits = bits;
    }
    if let Some(g) = v.get("grid").and_then(|x| x.as_u64()) { cfg.bits_grid = (g as u32).clamp(2, 7); }
    save_config(&ctx.paths, &cfg);
    *ctx.cfg.lock().unwrap() = cfg.clone();
    log(&format!("bits saved: {} configured, grid {}", cfg.bits.len(), cfg.bits_grid));
    let _ = request.respond(json_resp(200, json!({ "ok": true, "bits": cfg.bits, "grid": cfg.bits_grid })));
}

/* ---------------------------------------------------------------- media */

fn safe_name(name: &str) -> Option<String> {
    if name.is_empty() || name.len() > 128 { return None; }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) { return None; }
    if name.contains("..") { return None; }
    Some(name.to_string())
}

fn mime_of(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "gif" => "image/gif",
        "webp" => "image/webp", "avif" => "image/avif",
        "mp4" => "video/mp4", "webm" => "video/webm", "mov" => "video/quicktime",
        "mp3" => "audio/mpeg", "wav" => "audio/wav", "ogg" => "audio/ogg", "m4a" => "audio/mp4",
        _ => "application/octet-stream",
    }
}

fn upload(ctx: &Ctx, mut request: tiny_http::Request, full: &str) {
    let name = full.split('?').nth(1).unwrap_or("")
        .split('&').find_map(|kv| kv.strip_prefix("name="))
        .unwrap_or("file");
    let Some(n) = safe_name(name) else {
        let _ = request.respond(json_resp(400, json!({ "ok": false, "error": "bad name" })));
        return;
    };
    let mut buf = Vec::new();
    let _ = request.as_reader().read_to_end(&mut buf);
    let dir = ctx.paths.base.join("media");
    let _ = std::fs::create_dir_all(&dir);
    match std::fs::write(dir.join(&n), &buf) {
        Ok(_) => { let _ = request.respond(json_resp(200, json!({ "ok": true, "url": format!("/media/{}", n) }))); }
        Err(e) => { let _ = request.respond(json_resp(500, json!({ "ok": false, "error": e.to_string() }))); }
    }
}

/* ---------------------------------------------------------------- notify */

fn notify_post(ctx: &Ctx, mut request: tiny_http::Request) {
    let v = read_body(&mut request);
    match serde_json::from_value::<NotifyRules>(v) {
        Ok(n) => {
            let mut cfg = ctx.cfg();
            cfg.notify = normalize_rules(n);
            // the default variation's range sets the jar/donor-form limits
            if let Some(d) = cfg.notify.rules.iter().find(|r| r.id == "default") {
                if d.min > 0.0 { cfg.min_amount = d.min; }
                if d.max > 0.0 { cfg.max_amount = d.max; }
            }
            save_config(&ctx.paths, &cfg);
            *ctx.cfg.lock().unwrap() = cfg;
            log("notify rules saved");
            let _ = request.respond(json_resp(200, json!({ "ok": true })));
        }
        Err(e) => { let _ = request.respond(json_resp(400, json!({ "ok": false, "error": e.to_string() }))); }
    }
}

/* ---------------------------------------------------------------- serve */

fn serve_page(ctx: &Ctx, host: &str, path: &str, public: bool) -> Option<Vec<u8>> {
    let page = if public { route_public(&ctx.cfg(), path)? } else { route_by_host(&ctx.cfg(), host, path)? };
    let html: &str = match page {
        Page::Donor => include_str!("../public/form.html"),
        Page::Stats => include_str!("../public/check.html"),
        Page::Overlay => include_str!("../public/overlay.html"),
        Page::Qr => include_str!("../public/qr.html"),
        Page::QrEdit => include_str!("../public/qr_edit.html"),
        Page::Setup => include_str!("../public/setup.html"),
        Page::Notify => include_str!("../public/notify.html"),
        Page::BitsOverlay => include_str!("../public/bits_overlay.html"),
        Page::BitsDonor => include_str!("../public/bits.html"),
        Page::BitsEdit => include_str!("../public/bits_edit.html"),
        Page::DonateEdit => include_str!("../public/donate_edit.html"),
        Page::YouTube => include_str!("../public/youtube.html"),
        Page::TimerAdmin => include_str!("../public/timer_admin.html"),
        Page::TimerOverlay => include_str!("../public/timer.html"),
        Page::AuctionUi => include_str!("../public/auc/index.html"),
        Page::AuctionSettings => include_str!("../public/auc_settings.html"),
    };
    Some(html.as_bytes().to_vec())
}

/// what the public (tunnel-facing) listener may serve — never admin, config writes, or overlays
fn public_allowed(method: Method, path: &str, cfg: &Config) -> bool {
    match (method, path) {
        (Method::Get, "/favicon.ico") | (Method::Get, "/app.css") | (Method::Get, "/alert.js")
        | (Method::Get, "/head.js") | (Method::Get, "/i18n.js") | (Method::Get, "/notify.js")
        | (Method::Get, "/notify.css") | (Method::Get, "/notify-ui.js") | (Method::Get, "/auc.js") | (Method::Get, "/qrcode.js")
        | (Method::Get, "/auc.css") | (Method::Get, "/assets/default.jpg") | (Method::Get, "/rat.mp4")
        | (Method::Get, "/api/state") | (Method::Get, "/api/notify") | (Method::Get, "/api/config")
        | (Method::Get, "/api/bits")
        | (Method::Post, "/api/submit")
        | (Method::Post, "/api/test")
        | (Method::Post, "/api/replay")
        | (Method::Post, "/api/youtube") => true,
        (Method::Get, p) if p.starts_with("/media/") => true,
        (Method::Get, p) if p.starts_with("/media-x/") => true,
        (Method::Get, p) if p.starts_with("/tts/") => true,
        (Method::Get, p) => route_public(cfg, p).is_some(),
        _ => false,
    }
}

fn handle(ctx: Arc<Ctx>, mut request: tiny_http::Request, port: u16) {
    let method = request.method().clone();
    let full = request.url().to_string();
    let path = full.split('?').next().unwrap_or("").to_string();
    let host = request.headers().iter()
        .find(|h| h.field.equiv("Host"))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_default();
    // public listener = donor / bits / auction pages only (secured by port, not by host header)
    let public = { let c = ctx.cfg(); c.public_port != 0 && port == c.public_port && port != c.port };
    if public && !public_allowed(method.clone(), &path, &ctx.cfg()) {
        let _ = request.respond(body_resp(404, "text/plain; charset=utf-8", b"not found".to_vec()));
        return;
    }

    if method == Method::Get && path == "/favicon.ico" {
        let _ = request.respond(Response::empty(StatusCode(204)));
        return;
    }
    if method == Method::Get && path == "/app.css" {
        let _ = request.respond(body_resp(200, "text/css; charset=utf-8", include_str!("../public/app.css").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/alert.js" {
        let _ = request.respond(body_resp(200, "application/javascript; charset=utf-8", include_str!("../public/alert.js").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/notify.js" {
        let _ = request.respond(body_resp(200, "application/javascript; charset=utf-8", include_str!("../public/notify.js").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/notify-ui.js" {
        let _ = request.respond(body_resp(200, "application/javascript; charset=utf-8", include_str!("../public/notify-ui.js").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/notify.css" {
        let _ = request.respond(body_resp(200, "text/css; charset=utf-8", include_str!("../public/notify.css").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/head.js" {
        let _ = request.respond(body_resp(200, "application/javascript; charset=utf-8", include_str!("../public/head.js").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/qrcode.js" {
        let _ = request.respond(body_resp(200, "application/javascript; charset=utf-8", include_str!("../public/qrcode.js").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/i18n.js" {
        let _ = request.respond(body_resp(200, "application/javascript; charset=utf-8", include_str!("../public/i18n.js").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/auc.js" {
        let _ = request.respond(body_resp(200, "application/javascript; charset=utf-8", include_str!("../public/auc/auc.js").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/auc.css" {
        let _ = request.respond(body_resp(200, "text/css; charset=utf-8", include_str!("../public/auc/auc.css").as_bytes().to_vec()));
        return;
    }
    if method == Method::Get && path == "/assets/default.jpg" {
        let _ = request.respond(body_resp(200, "image/jpeg", include_bytes!("../public/assets/default.jpg").to_vec()));
        return;
    }
    if method == Method::Get && path == "/rat.mp4" {
        file_resp(request, include_bytes!("../public/assets/rat.mp4").to_vec(), "video/mp4");
        return;
    }
    if method == Method::Get && path.starts_with("/tts/") {
        let name = path.trim_start_matches("/tts/");
        let safe = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
        let f = ctx.paths.base.join("tts").join(name);
        if safe && f.exists() {
            if let Ok(bytes) = std::fs::read(&f) {
                let _ = request.respond(body_resp(200, "audio/wav", bytes));
                return;
            }
        }
        let _ = request.respond(Response::empty(StatusCode(404)));
        return;
    }
    if path == "/api/theme" {
        if method == Method::Post {
            let v = read_body(&mut request);
            let light = v.get("light").and_then(|x| x.as_bool()).unwrap_or(false);
            LIGHT_THEME.store(light, Ordering::Relaxed);
            let _ = request.respond(json_resp(200, json!({ "ok": true })));
        } else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/tailscale" {
        if method == Method::Get { let _ = request.respond(json_resp(200, crate::tailscale::status_for(ctx.cfg().public_port))); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/tailscale/auth" {
        if method == Method::Post {
            let url = crate::tailscale::login_url();
            let _ = request.respond(json_resp(200, json!({ "ok": url.is_some(), "url": url })));
        } else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/tailscale/funnel" {
        if method == Method::Post {
            let r = crate::tailscale::funnel_enable(ctx.cfg().public_port);
            let _ = request.respond(json_resp(200, r));
        } else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/tailscale/install" {
        if method == Method::Post {
            // downloads the current release; bundled MSI is only the offline fallback
            let msi = std::env::current_exe().ok()
                .and_then(|p| p.parent().map(|d| d.join("tailscale-setup.msi")))
                .unwrap_or_else(|| ctx.paths.base.join("tailscale-setup.msi"));
            match crate::tailscale::install(&msi) {
                Ok(_) => { let _ = request.respond(json_resp(200, json!({ "ok": true, "started": true }))); }
                Err(e) => { let _ = request.respond(json_resp(200, json!({ "ok": false, "error": e }))); }
            }
        } else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/ports" {
        if method == Method::Get {
            let (app, auc, pubp) = { let c = ctx.cfg(); (c.port, c.auction_port, c.public_port) };
            let body = json!({
                "app": { "port": app, "free": crate::model::BOUND_PORTS.lock().unwrap().contains(&app) || crate::model::port_free(app) },
                "auction": { "port": auc, "free": auc == 0 || crate::model::BOUND_PORTS.lock().unwrap().contains(&auc) || crate::model::port_free(auc) },
                "public": { "port": pubp, "free": pubp == 0 || crate::model::BOUND_PORTS.lock().unwrap().contains(&pubp) || crate::model::port_free(pubp) },
                "oauth": crate::twitch::OAUTH_PORT
            });
            let _ = request.respond(json_resp(200, body));
        } else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/ports/pick" {
        if method == Method::Post {
            let v = read_body(&mut request);
            let which = v.get("which").and_then(|x| x.as_str()).unwrap_or("");
            let mut cfg = ctx.cfg();
            let skip = [cfg.port, cfg.auction_port, cfg.public_port, crate::twitch::OAUTH_PORT];
            let default = match which {
                "app" => 27183u16,
                "auction" => 27184u16,
                "public" => 27185u16,
                _ => { let _ = request.respond(json_resp(400, json!({ "ok": false, "error": "unknown listener" }))); return; }
            };
            let port = crate::model::pick_free_port(default, &skip);
            match which {
                "app" => cfg.port = port,
                "auction" => cfg.auction_port = port,
                _ => cfg.public_port = port,
            }
            save_config(&ctx.paths, &cfg);
            log(&format!("ports: {} moved to {}", which, port));
            let _ = request.respond(json_resp(200, json!({ "ok": true, "port": port, "restart": true })));
        } else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/app/restart" {
        if method == Method::Post {
            let _ = request.respond(json_resp(200, json!({ "ok": true })));
            log("restart requested from the setup page");
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(500));
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                std::process::exit(0);
            });
        } else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if method == Method::Get && path == "/api/poll" { poll_events(&ctx, request, &full); return; }
    if path == "/api/qr" {
        if method == Method::Get { qr_get(&ctx, request); }
        else if method == Method::Post { qr_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/config" {
        if method == Method::Get { let _ = request.respond(json_resp(200, config_get(&ctx, public))); }
        else if method == Method::Post { config_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }

    if path == "/api/newstats" {
        if method == Method::Post { newstats_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/bg" {
        if method == Method::Post { bg_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/bits" {
        if method == Method::Get { let _ = request.respond(json_resp(200, json!({ "bits": ctx.cfg().bits, "mode": ctx.cfg().bits_mode, "grid": ctx.cfg().bits_grid, "jars": ctx.cfg().jars }))); }
        else if method == Method::Post { bits_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }

    if path == "/api/bits/mode" {
        if method == Method::Post {
            let v = read_body(&mut request);
            let m = v.get("mode").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if !["single", "order", "adhd"].contains(&m.as_str()) {
                let _ = request.respond(json_resp(400, json!({ "ok": false, "error": "unknown mode" })));
                return;
            }
            { let mut cfg = ctx.cfg(); cfg.bits_mode = m.clone(); save_config(&ctx.paths, &cfg); *ctx.cfg.lock().unwrap() = cfg.clone(); }
            log(&format!("bits mode: {}", m));
            let _ = request.respond(json_resp(200, json!({ "ok": true, "mode": m })));
        } else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }

    if path == "/api/fonts" {
        let _ = request.respond(json_resp(200, json!({ "fonts": list_fonts() })));
        return;
    }
    if path == "/api/timer" {
        if method == Method::Get { let _ = request.respond(json_resp(200, timer_json(&ctx))); }
        else if method == Method::Post { timer_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/timer/settings" {
        if method == Method::Post { timer_settings_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/features" {
        if method == Method::Get { let _ = request.respond(json_resp(200, serde_json::to_value(&ctx.cfg().features).unwrap_or(json!({})))); }
        else if method == Method::Post { features_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/auction" {
        if method == Method::Get { let _ = request.respond(json_resp(200, auction_json(&ctx))); }
        else if method == Method::Post { auction_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/auction/settings" {
        if method == Method::Post { auction_settings_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    /* ---- merged AUC API (lots / bids / wheel) ---- */
    if path == "/api/auc/state" {
        let _ = request.respond(json_resp(200, auc_json(&ctx)));
        return;
    }
    if path == "/api/auc/emotes" && method == Method::Get { let _ = request.respond(json_resp(200, json!({ "emotes": [] }))); return; }
    if path == "/api/auc/bids" && method == Method::Get { let st = ctx.state.lock().unwrap(); let _ = request.respond(json_resp(200, serde_json::to_value(&st.auc.log).unwrap_or(json!([])))); return; }
    if path == "/api/auc/history" && method == Method::Get { let _ = request.respond(json_resp(200, json!([]))); return; }
    if path == "/api/auc/settings" && method == Method::Get {
        let _ = request.respond(json_resp(200, serde_json::to_value(&ctx.cfg().auction).unwrap_or(json!({}))));
        return;
    }
    if path.starts_with("/api/auc/") && method == Method::Post {
        let v = read_body(&mut request);
        let g = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let gf = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
        match path.as_str() {
            "/api/auc/lots" => { let t = g("title"); if !t.trim().is_empty() { auc_mut(&ctx, |a| { a.make_lot(t.trim(), true); }); } }
            "/api/auc/lots/delete" => { let id = g("lotId"); auc_mut(&ctx, |a| { a.lots.retain(|l| l.id != id); a.log.retain(|b| b.lot_id != id); a.recompute(); }); }
            "/api/auc/lots/pin" => { let id = g("lotId"); auc_mut(&ctx, |a| { if let Some(l) = a.lots.iter_mut().find(|l| l.id == id) { l.pinned = !l.pinned; } }); }
            "/api/auc/lots/adjust" => { let id = g("lotId"); let d = gf("delta"); auc_mut(&ctx, |a| { if let Some(l) = a.lots.iter_mut().find(|l| l.id == id) { l.adjust += d; } a.recompute(); }); }
            "/api/auc/bid" => { let (u, ui, s, rid) = (g("username"), g("userInput"), g("source"), g("redemptionId")); let amt = gf("amount"); let src = if s.is_empty() { "manual".to_string() } else { s }; auc_mut(&ctx, |a| { let _ = a.intake(&u, amt, &ui, &src, &rid); }); }
            "/api/auc/pending/approve" => { let id = g("id"); let lid = g("lotId"); auc_mut(&ctx, |a| {
                if let Some(i) = a.pending.iter().position(|b| b.id == id) {
                    let b = a.pending.remove(i);
                    let (lid2, lt) = if !lid.is_empty() {
                        let t = a.lots.iter().find(|l| l.id == lid).map(|l| l.title.clone()).unwrap_or_default();
                        (lid.clone(), t)
                    } else if !b.lot_id.is_empty() { (b.lot_id.clone(), b.lot_title.clone()) }
                    else { let t = if b.user_input.trim().is_empty() { b.username.clone() } else { b.user_input.clone() }; let nid = a.make_lot(&t, false).id.clone(); (nid, t) };
                    a.apply_bid(b, &lid2, &lt);
                }
            }); }
            "/api/auc/pending/decline" => { let id = g("id"); auc_mut(&ctx, |a| { a.pending.retain(|b| b.id != id); }); }
            "/api/auc/winner" => { let m = g("mode"); let lid = g("lotId"); auc_mut(&ctx, |a| { a.pick_winner(if m.is_empty() { "top" } else { &m }, if lid.is_empty() { None } else { Some(&lid) }); }); }
            "/api/auc/start" => { auc_mut(&ctx, |a| { a.status = "open".into(); a.running = true; a.winner = None; a.ends_at = now_ms() + a.remaining_ms.max(0); if a.total_ms <= 0 { a.total_ms = a.remaining_ms; } }); }
            "/api/auc/pause" => { auc_mut(&ctx, |a| { a.remaining_ms = a.remaining(); a.running = false; a.ends_at = 0; a.status = "paused".into(); }); }
            "/api/auc/timer/reset" => { let sec = ctx.cfg().auction.timer_seconds as i64; auc_mut(&ctx, |a| { a.running = false; a.ends_at = 0; a.remaining_ms = sec * 1000; a.total_ms = sec * 1000; }); }
            "/api/auc/timer" => { let s = gf("seconds"); auc_mut(&ctx, |a| { let d = (s * 1000.0) as i64; if a.running && a.ends_at > 0 { a.ends_at = (a.ends_at + d).max(now_ms()); } else { a.remaining_ms = (a.remaining_ms + d).max(0); } a.total_ms = (a.total_ms + d).max(0); }); }
            "/api/auc/settings" => {
                let mut cfg = ctx.cfg();
                if let Some(x) = v.get("mode").and_then(|x| x.as_str()) { cfg.auction.mode = x.into(); }
                if let Some(x) = v.get("source").and_then(|x| x.as_str()) { cfg.auction.source = x.into(); }
                if let Some(x) = v.get("currency").and_then(|x| x.as_str()) { cfg.auction.currency = x.into(); }
                if let Some(x) = v.get("timerSeconds").and_then(|x| x.as_u64()) { cfg.auction.timer_seconds = x; }
                if let Some(x) = v.get("showTotalTime").and_then(|x| x.as_bool()) { cfg.auction.show_total_time = x; }
                if let Some(x) = v.get("leaderChangePoints").and_then(|x| x.as_f64()) { cfg.auction.leader_change_points = x; }
                if let Some(x) = v.get("leaderChangeDonations").and_then(|x| x.as_f64()) { cfg.auction.leader_change_donations = x; }
                if let Some(x) = v.get("newPositionPoints").and_then(|x| x.as_f64()) { cfg.auction.new_position_points = x; }
                if let Some(x) = v.get("newPositionDonations").and_then(|x| x.as_f64()) { cfg.auction.new_position_donations = x; }
                if let Some(x) = v.get("addExistingPoints").and_then(|x| x.as_f64()) { cfg.auction.add_existing_points = x; }
                if let Some(x) = v.get("addExistingDonations").and_then(|x| x.as_f64()) { cfg.auction.add_existing_donations = x; }
                if let Some(x) = v.get("timeMoreThanMin").and_then(|x| x.as_f64()) { cfg.auction.time_more_than_min = x; }
                if let Some(x) = v.get("rewardTitle").and_then(|x| x.as_str()) { cfg.auction.reward_title = x.into(); }
                if let Some(arr) = v.get("rewardCosts").and_then(|x| x.as_array()) { cfg.auction.reward_costs = arr.iter().filter_map(|x| x.as_u64()).collect(); }
                save_config(&ctx.paths, &cfg);
                *ctx.cfg.lock().unwrap() = cfg.clone();
            }
            _ => {}
        }
        let _ = request.respond(json_resp(200, auc_json(&ctx)));
        return;
    }
    if path == "/api/twitch" {
        if method == Method::Get { let _ = request.respond(json_resp(200, twitch_json(&ctx, port))); }
        else if method == Method::Post { twitch_post(&ctx, request, port); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/twitch/auth-url" {
        let _ = request.respond(json_resp(200, json!({ "url": crate::twitch::auth_url(port) })));
        return;
    }
    if path == "/twitch/callback" { twitch_callback(&ctx, request, &full, port); return; }
    if path == "/api/streamelements" {
        if method == Method::Get { let _ = request.respond(json_resp(200, se_json(&ctx, port))); }
        else if method == Method::Post { se_post(&ctx, request, port); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/streamelements/test" { se_test(&ctx, request, port); return; }
    if path == "/api/update" {
        if method == Method::Get {
            let v = update_info().lock().unwrap().clone();
            let _ = request.respond(json_resp(200, v));
        } else {
            let _ = request.respond(json_resp(405, json!({ "ok": false })));
        }
        return;
    }
    if path == "/api/youtube" {
        if method == Method::Post { youtube_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/notify" {
        if method == Method::Get { let _ = request.respond(json_resp(200, serde_json::to_value(&ctx.cfg().notify).unwrap_or(json!({})))); }
        else if method == Method::Post { notify_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if path == "/api/tts" {
        if method == Method::Get {
            let c = ctx.cfg();
            let _ = request.respond(json_resp(200, json!({ "tts": { "mode": c.tts.mode, "voice": c.tts.default_voice(), "readMode": c.tts.read_mode, "speed": c.tts.speed, "dir": c.tts.dir, "allowed": c.tts.allowed_in(&c.tts.dir), "picks": c.tts.donor_picks(), "hasAzure": c.tts.has_azure(), "azureRegion": c.tts.azure_region }, "voices": [{"id": "google:uk", "label": "Google (українська)", "lang": "uk"}] })));
        }
        else if method == Method::Post { tts_post(&ctx, request); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if method == Method::Get && path.starts_with("/media-x/") {
        media_x(request, &path[9..]);
        return;
    }
    if path == "/api/upload" {
        if method == Method::Post { upload(&ctx, request, &full); }
        else { let _ = request.respond(json_resp(405, json!({ "ok": false }))); }
        return;
    }
    if method == Method::Get && path.starts_with("/media/") {
        match safe_name(&path[7..]) {
            Some(name) => match std::fs::read(ctx.paths.base.join("media").join(&name)) {
                Ok(bytes) => { file_resp(request, bytes, mime_of(&name)); }
                Err(_) => { let _ = request.respond(body_resp(404, "text/plain; charset=utf-8", b"not found".to_vec())); }
            },
            None => { let _ = request.respond(body_resp(400, "text/plain; charset=utf-8", b"bad name".to_vec())); }
        }
        return;
    }

    match (method, path.as_str()) {
        (Method::Get, "/api/state") => {
            let mut v = state_json(&ctx);
            if public {
                if let Some(o) = v.as_object_mut() { o.remove("links"); o.remove("hasToken"); if let Some(arr) = o.get_mut("jars").and_then(|x| x.as_array_mut()) { for j in arr.iter_mut() { if let Some(jo) = j.as_object_mut() { jo.retain(|k, _| k == "code" || k == "title"); } } } }
            }
            let _ = request.respond(json_resp(200, v));
        }
        (Method::Get, "/api/history") => {
            let jar = query(&full, "jar").and_then(|v| v.parse().ok()).unwrap_or(0);
            let _ = request.respond(json_resp(200, history_json(&ctx, jar)));
        }
        (Method::Post, "/api/submit") => submit(&ctx, request),
        (Method::Post, "/api/test") => test(&ctx, request),
        (Method::Post, "/api/replay") => replay(&ctx, request),
        (Method::Post, "/api/refresh") => refresh(&ctx, request),
        (Method::Get, _) => match serve_page(&ctx, &host, &path, public) {
            Some(bytes) => { let _ = request.respond(body_resp(200, "text/html; charset=utf-8", bytes)); }
            None => { let _ = request.respond(body_resp(404, "text/plain; charset=utf-8", b"not found".to_vec())); }
        },
        _ => { let _ = request.respond(body_resp(404, "text/plain; charset=utf-8", b"not found".to_vec())); }
    }
}

pub fn serve(ctx: Arc<Ctx>) {
    let cfg = ctx.cfg();
    let mut ports: Vec<u16> = vec![cfg.port];
    if cfg.auction_port > 0 && cfg.auction_port != cfg.port { ports.push(cfg.auction_port); }
    for port in ports {
        let ctx = ctx.clone();
        let host = cfg.host.clone();
        std::thread::spawn(move || serve_port(ctx, &host, port));
    }
    // public listener (0.0.0.0): donor / bits / auction only — point a tunnel at it
    if cfg.public_port > 0 && cfg.public_port != cfg.port && cfg.public_port != cfg.auction_port {
        let ctx = ctx.clone();
        let p = cfg.public_port;
        std::thread::spawn(move || serve_port(ctx, "0.0.0.0", p));
    }
}

fn serve_port(ctx: Arc<Ctx>, host: &str, port: u16) {
    let addr = format!("{}:{}", host, port);
    let server = match tiny_http::Server::http(&addr) {
        Ok(s) => s,
        Err(e) => { log(&format!("cannot bind {}: {}", addr, e)); return; }
    };
    log(&format!("listening on {}", addr));
    for request in server.incoming_requests() {
        let ctx = ctx.clone();
        std::thread::spawn(move || handle(ctx, request, port));
    }
}

#[cfg(test)]
mod media_tests {
    use super::{twitch_clip_slug, x_status_id};

    #[test]
    fn extracts_twitch_clip_slugs() {
        for url in [
            "https://clips.twitch.tv/RepleteEnthusiasticMageCeilingCat",
            "https://clips.twitch.tv/RepleteEnthusiasticMageCeilingCat?tt_content=clip",
            "https://www.twitch.tv/clip/RepleteEnthusiasticMageCeilingCat",
            "https://twitch.tv/s1mple/clip/RepleteEnthusiasticMageCeilingCat",
            "https://www.twitch.tv/s1mple/clip/RepleteEnthusiasticMageCeilingCat",
            "https://m.twitch.tv/s1mple/clip/RepleteEnthusiasticMageCeilingCat",
            "https://clips.twitch.tv/embed?clip=RepleteEnthusiasticMageCeilingCat&parent=example.com",
        ] {
            assert_eq!(twitch_clip_slug(url).as_deref(), Some("RepleteEnthusiasticMageCeilingCat"), "url: {url}");
        }
        assert_eq!(twitch_clip_slug("https://example.com/clip/x"), None);
    }

    #[test]
    fn extracts_x_status_ids() {
        for url in [
            "https://x.com/NASA/status/1446120853117177872",
            "https://www.x.com/NASA/status/1446120853117177872",
            "https://twitter.com/NASA/status/1446120853117177872",
            "https://mobile.twitter.com/NASA/status/1446120853117177872",
            "https://x.com/i/status/1446120853117177872",
            "https://x.com/NASA/status/1446120853117177872/video/1",
            "https://x.com/NASA/status/1446120853117177872?s=20",
        ] {
            assert_eq!(x_status_id(url).as_deref(), Some("1446120853117177872"), "url: {url}");
        }
        assert_eq!(x_status_id("https://x.com/NASA"), None);
    }
}

#[cfg(test)]
mod update_tests {
    use super::version_gt;

    #[test]
    fn newer_release_wins_and_equal_is_silent() {
        assert!(version_gt("v0.2.4", "0.2.3"));
        assert!(!version_gt("v0.2.3", "0.2.3"));
        assert!(!version_gt("0.2.3", "v0.2.4"));
        assert!(version_gt("v0.10.0", "0.9.9"));
        assert!(version_gt("v1.0", "0.99.99"));
        assert!(!version_gt("v0.2.4-beta", "0.2.4"));
    }
}
