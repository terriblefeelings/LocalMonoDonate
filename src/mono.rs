use serde::Deserialize;
use std::time::Duration;

const BASE: &str = "https://api.monobank.ua";

#[derive(Deserialize, Clone)]
pub struct JarInfo {
    pub id: String,
    #[serde(rename = "sendId", default)]
    pub send_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub balance: Option<i64>,
    #[serde(default)]
    pub goal: Option<i64>,
    #[serde(rename = "currencyCode", default)]
    pub currency_code: Option<i64>,
}

#[derive(Deserialize)]
pub struct ClientInfo {
    #[serde(default)]
    pub jars: Vec<JarInfo>,
}

#[derive(Deserialize, Clone)]
pub struct StatementItem {
    pub id: String,
    pub time: i64,
    pub amount: i64,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(rename = "counterName", default)]
    pub counter_name: Option<String>,
}

/// Read a response body as UTF-8. ureq's `into_string()` decodes by the
/// Content-Type charset and falls back to Latin-1, which mangles Cyrillic
/// (Monobank sends `application/json` without `charset=utf-8`).
fn body_utf8(r: ureq::Response) -> Result<String, String> {
    use std::io::Read;
    let mut buf = Vec::new();
    r.into_reader().read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn get_json(url: &str, token: &str) -> Result<serde_json::Value, String> {
    match ureq::get(url).set("X-Token", token).timeout(Duration::from_secs(20)).call() {
        Ok(r) => {
            let text = body_utf8(r)?;
            serde_json::from_str(&text).map_err(|e| e.to_string())
        }
        Err(ureq::Error::Status(code, r)) => {
            let body = body_utf8(r).unwrap_or_default();
            let body: String = body.chars().take(200).collect();
            Err(format!("HTTP {} {}", code, body))
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn client_info(token: &str) -> Result<ClientInfo, String> {
    let v = get_json(&format!("{}/personal/client-info", BASE), token)?;
    serde_json::from_value(v).map_err(|e| e.to_string())
}

pub fn statement(account: &str, token: &str, from: i64, to: i64) -> Result<Vec<StatementItem>, String> {
    let v = get_json(&format!("{}/personal/statement/{}/{}/{}", BASE, account, from, to), token)?;
    serde_json::from_value(v).map_err(|e| e.to_string())
}

/// short share id: "jar/AbCdEfGh12" -> "AbCdEfGh12"
pub fn short_send_id(send_id: &str) -> String {
    send_id.split('/').next_back().unwrap_or("").to_string()
}

/// best-effort (views, seconds) for a YouTube video id; None if it can't be determined.
pub fn youtube_meta(id: &str) -> Option<(u64, u64)> {
    let url = format!("https://www.youtube.com/watch?v={}&hl=en&gl=US", id);
    let r = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Safari/537.36")
        .set("Accept-Language", "en-US,en;q=0.9")
        .timeout(Duration::from_secs(12)).call().ok()?;
    let html = body_utf8(r).ok()?;
    let views = num_after(&html, "\"viewCount\":\"");
    let secs = num_after(&html, "\"lengthSeconds\":\"");
    match (views, secs) { (Some(v), Some(s)) => Some((v, s)), _ => None }
}

fn num_after(hay: &str, key: &str) -> Option<u64> {
    let i = hay.find(key)? + key.len();
    let digits: String = hay[i..].chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() { None } else { digits.parse().ok() }
}
