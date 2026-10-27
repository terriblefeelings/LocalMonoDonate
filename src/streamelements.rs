//! StreamElements integration: mirror live Monobank donations as StreamElements tips so
//! the channel's SE alert box fires, and post a matching "tip" activity so the donation
//! shows up in the Activity feed and the dashboard statistics. The account id lives in
//! config.json, the JWT in the per-port secrets file
//! (%APPDATA%\LocalMonoDonate\<port>\streamelements.json).
use crate::model::*;
use serde_json::{json, Value};
use std::time::Duration;

const API: &str = "https://api.streamelements.com/kappa/v2";

fn post_json(url: &str, token: &str, body: &str) -> Result<(), String> {
    match ureq::post(url)
        .set("Authorization", &format!("Bearer {}", token.trim()))
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(15))
        .send_string(body)
    {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(code, r)) => {
            let t = r.into_string().unwrap_or_default();
            let t: String = t.chars().take(200).collect();
            Err(format!("HTTP {} {}", code, t))
        }
        Err(e) => Err(e.to_string()),
    }
}

fn get_json(url: &str, token: &str) -> Result<Value, String> {
    let mut req = ureq::get(url).set("Accept", "application/json").timeout(Duration::from_secs(15));
    if !token.trim().is_empty() { req = req.set("Authorization", &format!("Bearer {}", token.trim())); }
    match req.call() {
        Ok(r) => {
            let t = r.into_string().map_err(|e| e.to_string())?;
            serde_json::from_str(&t).map_err(|e| e.to_string())
        }
        Err(ureq::Error::Status(401, _)) => Err("401 unauthorized - check the JWT token".into()),
        Err(ureq::Error::Status(code, r)) => {
            let t = r.into_string().unwrap_or_default();
            let t: String = t.chars().take(200).collect();
            Err(format!("HTTP {} {}", code, t))
        }
        Err(e) => Err(e.to_string()),
    }
}

/// GET /channels/{id} - used by the setup page "Test connection".
pub fn channel(account_id: &str, token: &str) -> Result<Value, String> {
    if account_id.trim().is_empty() { return Err("account id is empty".into()); }
    if token.trim().is_empty() { return Err("token is empty".into()); }
    let v = get_json(&format!("{}/channels/{}", API, account_id.trim()), token)?;
    Ok(json!({
        "id": v["_id"],
        "username": v["username"],
        "alias": v["alias"],
        "displayName": v["displayName"]
    }))
}

/// POST /activities/{channel} - this is what the Activity feed and the dashboard stats read.
/// Stable per-donor SE identity. SE groups tippers by email, so one placeholder email
/// merged every Monobank donor into a single row; hashing the donor key keeps one row per
/// person and can never merge two different names.
fn donor_identity(name: &str) -> (String, String) {
    let key = donor_key(name);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in key.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x0000_0100_0000_01b3); }
    let slug = format!("{:016x}", h);
    (format!("monobank:{}", slug), format!("{}@monobank.local", slug))
}

fn activity_body(name: &str, amount: f64, message: &str) -> Value {
    json!({
        "isMock": false,
        "provider": "streamelements",
        "type": "tip",
        "data": {
            "amount": (amount * 100.0).round() / 100.0,
            "username": name,
            "displayName": name,
            "message": message.chars().take(300).collect::<String>()
        }
    })
}

fn post_activity(account_id: &str, token: &str, name: &str, amount: f64, message: &str) -> Result<(), String> {
    let body = activity_body(name, amount, message).to_string();
    post_json(&format!("{}/activities/{}", API, account_id.trim()), token, &body)
}

/// POST /tips/{channel} keeps the real donation currency, and the matching activity
/// posts the same amount in the channel currency (SE does not convert activities).
pub fn send_tip(port: u16, cfg: &Config, name: &str, amount: f64, currency: &str, message: &str) -> Result<(), String> {
    let se = &cfg.streamelements;
    if !se.enabled { return Ok(()); }
    if se.account_id.trim().is_empty() { return Err("no account id".into()); }
    let sec = load_streamelements_at(port);
    if sec.token.trim().is_empty() { return Err("no token".into()); }
    let whole = amount.round() as i64;
    // test tips are sent with 0 so StreamElements does not count them in statistics
    let user = if name.trim().is_empty() { "Анонім" } else { name.trim() };
    let (uid, email) = donor_identity(user);
    let body = json!({
        "user": { "userId": uid, "username": user, "email": email },
        "provider": "monobank",
        "message": message.chars().take(300).collect::<String>(),
        "amount": whole,
        "currency": currency
    }).to_string();
    post_json(&format!("{}/tips/{}", API, se.account_id.trim()), &sec.token, &body)?;

    if let Err(e) = post_activity(&se.account_id, &sec.token, user, amount, message) {
        log(&format!("streamelements: activity: {}", e));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn donor_identity_is_unique_per_name_and_stable() {
        let a = donor_identity("goodfeelings");
        let b = donor_identity("test555");
        assert_ne!(a.0, b.0);
        assert_ne!(a.1, b.1);
        assert_eq!(a, donor_identity(" Goodfeelings "));  // trim + case only
        assert_ne!(a, donor_identity("pen dra"));   // different nickname = different donor
        assert_ne!(a, donor_identity("goodfeelings2"));
    }

    #[test]
    fn activity_keeps_the_channel_currency_amount() {
        let body = activity_body("goodfeelings", 10.0, "hi");
        assert_eq!(body["data"]["amount"].as_f64(), Some(10.0));
    }
}

/// Fire-and-forget: the HTTP timeouts must never stall the Monobank poll loop.
pub fn spawn_tip(port: u16, cfg: Config, name: String, amount: f64, currency: String, message: String) {
    if !cfg.streamelements.enabled { return; }
    std::thread::spawn(move || match send_tip(port, &cfg, &name, amount, &currency, &message) {
        Ok(()) => log(&format!("streamelements: tip {:.0} {} from {}", amount, currency, name)),
        Err(e) => log(&format!("streamelements: {}", e)),
    });
}
