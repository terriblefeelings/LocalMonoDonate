//! Tailscale integration: the app drives the Tailscale CLI so a user can get a
//! public `https://<nickname>.<tailnet>.ts.net` link with one "Connect" click —
//! no Cloudflare, no domain, works behind CGNAT.

use crate::model::log;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const MSI: &str = r"C:\Program Files\Tailscale\tailscale.exe";
/// Always-current Windows installer, so no specific Tailscale version is pinned.
/// The bundled MSI is only an offline fallback.
const LATEST_MSI: &str = "https://pkgs.tailscale.com/stable/tailscale-setup-latest-amd64.msi";
/// every user's node gets this name: https://ratcorporation.<tailnet>.ts.net
pub const HOSTNAME: &str = "ratcorporation";

fn exe() -> String {
    if std::path::Path::new(MSI).exists() { MSI.to_string() } else { "tailscale".to_string() }
}

/// spawn without a console window (a GUI app spawning tailscale.exe would flash a terminal)
fn cmd() -> Command {
    let mut c = Command::new(exe());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

pub fn installed() -> bool {
    Path::new(MSI).exists()
        || cmd().arg("version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// installed / logged in / public DNS name
pub fn status() -> Value {
    let installed = installed();
    let (state, dns) = match cmd().args(["status", "--json"]).output() {
        Ok(o) if o.status.success() => {
            let v: Value = serde_json::from_slice(&o.stdout).unwrap_or(Value::Null);
            (
                v.get("BackendState").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                v.get("Self").and_then(|s| s.get("DNSName")).and_then(|x| x.as_str())
                    .unwrap_or("").trim_end_matches('.').to_string(),
            )
        }
        _ => (String::new(), String::new()),
    };
    json!({ "installed": installed, "state": state, "dnsName": dns, "running": state == "Running" })
}

/// installed + login state + whether Funnel currently points at our public port
pub fn status_for(port: u16) -> Value {
    let mut v = status();
    let running = v.get("running").and_then(|x| x.as_bool()).unwrap_or(false);
    if let Some(o) = v.as_object_mut() {
        o.insert("publicPort".into(), json!(port));
        o.insert("funnel".into(), json!(running && port > 0 && funnel_ok(port)));
    }
    v
}

/// Download the current installer; `None` when offline or the download is truncated.
fn download_latest() -> Option<PathBuf> {
    let dst = std::env::temp_dir().join("tailscale-setup-latest-amd64.msi");
    let resp = ureq::get(LATEST_MSI).timeout(Duration::from_secs(180)).call().ok()?;
    let mut f = std::fs::File::create(&dst).ok()?;
    std::io::copy(&mut resp.into_reader(), &mut f).ok()?;
    // never hand a partial file to msiexec
    if std::fs::metadata(&dst).ok()?.len() > 1_000_000 { Some(dst) } else { None }
}

/// Install the current Tailscale release (downloaded on demand), falling back to
/// the bundled MSI when there is no network.
pub fn install(bundled: &Path) -> Result<(), String> {
    if installed() { return Ok(()); }
    let msi = download_latest().unwrap_or_else(|| bundled.to_path_buf());
    run_msi(&msi)
}

/// Run the MSI elevated. Returns as soon as the UAC prompt/installer starts;
/// the setup page polls status() until Tailscale appears.
fn run_msi(msi: &Path) -> Result<(), String> {
    if !msi.exists() { return Err("installer not found".into()); }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // msiexec needs administrator rights; -Verb RunAs raises the UAC prompt.
        let path = msi.display().to_string().replace('\'', "''");
        let script = format!(
            "Start-Process -FilePath msiexec.exe -ArgumentList '/i \"{}\" /qb /norestart' -Verb RunAs",
            path
        );
        Command::new("powershell.exe")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = msi;
        Err("Windows only".into())
    }
}

/// Latest published Windows release, e.g. "1.104.2" — a 3 KB JSON, no installer download.
fn latest_version() -> Option<String> {
    use std::io::Read;
    let resp = ureq::get("https://pkgs.tailscale.com/stable/?mode=json")
        .timeout(Duration::from_secs(20))
        .call().ok()?;
    let mut body = String::new();
    resp.into_reader().take(64 * 1024).read_to_string(&mut body).ok()?;
    let v: Value = serde_json::from_str(&body).ok()?;
    v.get("MSIsVersion").and_then(|x| x.as_str()).map(str::to_string)
}

fn installed_version() -> Option<String> {
    let o = cmd().arg("version").output().ok()?;
    String::from_utf8_lossy(&o.stdout).split_whitespace().next().map(str::to_string)
}

fn is_older(a: &str, b: &str) -> bool {
    let p = |s: &str| -> Vec<u32> { s.split('.').map(|n| n.parse().unwrap_or(0)).collect() };
    let (x, y) = (p(a), p(b));
    match (0..3).find(|&i| x.get(i) != y.get(i)) {
        Some(i) => x.get(i).unwrap_or(&0) < y.get(i).unwrap_or(&0),
        None => false,
    }
}

/// Runs on every start: users who never reinstall the app and never check for
/// updates still get a current Tailscale client.
pub fn auto_update() {
    if !installed() { return; }
    let (Some(cur), Some(latest)) = (installed_version(), latest_version()) else { return; };
    if !is_older(&cur, &latest) { return; }
    log(&format!("tailscale {} -> {}", cur, latest));
    match download_latest() {
        Some(msi) => match run_msi(&msi) {
            Ok(_) => log(&format!("tailscale update to {} started", latest)),
            Err(e) => log(&format!("tailscale update failed: {}", e)),
        },
        None => log("tailscale update: download failed"),
    }
}

/// pull the first login/enable URL out of Tailscale CLI output
fn first_url(text: &str) -> Option<String> {
    let i = text.find("https://login.tailscale.com/")?;
    let url: String = text[i..].chars().take_while(|c| !c.is_whitespace()).collect();
    Some(url.trim_end_matches(|c: char| matches!(c, '.' | ',' | ')' | ']' | ';')).to_string())
}

/// start `tailscale up` detached and return the login URL it prints
pub fn login_url() -> Option<String> {
    let tmp = std::env::temp_dir().join("lmd-tailscale-login.txt");
    let _ = std::fs::remove_file(&tmp);
    let f = std::fs::File::create(&tmp).ok()?;
    let g = f.try_clone().ok()?;
    let _ = cmd().arg("up")
        .stdout(Stdio::from(f)).stderr(Stdio::from(g)).spawn().ok()?;
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Ok(s) = std::fs::read_to_string(&tmp) {
            if let Some(url) = first_url(&s) { return Some(url); }
        }
    }
    None
}

/// the Tailscale hostname is the nickname => `<nickname>.<tailnet>.ts.net`
pub fn set_hostname(name: &str) {
    if !name.is_empty() {
        match cmd().args(["set", "--hostname", name]).output() {
            Ok(o) if !o.status.success() => {
                let e = String::from_utf8_lossy(&o.stderr);
                log(&format!("tailscale set --hostname failed: {}", e.trim()));
            }
            Err(e) => log(&format!("tailscale set --hostname failed: {}", e)),
            _ => {}
        }
    }
}

/// current node FQDN without the trailing dot ("" when unknown)
pub fn dns_name() -> String {
    match cmd().args(["status", "--json"]).output() {
        Ok(o) if o.status.success() => {
            let v: Value = serde_json::from_slice(&o.stdout).unwrap_or(Value::Null);
            v.get("Self")
                .and_then(|s| s.get("DNSName"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim_end_matches('.')
                .to_string()
        }
        _ => String::new(),
    }
}

fn funnel_status() -> String {
    match cmd().args(["funnel", "status"]).output() {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            s
        }
        Err(_) => String::new(),
    }
}

/// true only when the funnel has exactly one target and it is our public port.
/// An extra stale rule (e.g. a browser debug port) makes this false so ensure() resets it.
pub fn funnel_ok(port: u16) -> bool {
    let st = funnel_status();
    let targets: Vec<&str> = st
        .lines()
        .filter_map(|l| l.find("127.0.0.1:").map(|i| &l[i..]))
        .collect();
    targets.len() == 1 && targets[0].starts_with(&format!("127.0.0.1:{}", port))
}

pub fn reset_funnel() {
    match cmd().args(["funnel", "reset"]).output() {
        Ok(o) if !o.status.success() => {
            let e = String::from_utf8_lossy(&o.stderr);
            log(&format!("tailscale funnel reset failed: {}", e.trim()));
        }
        Err(e) => log(&format!("tailscale funnel reset failed: {}", e)),
        _ => {}
    }
}

/// enable Funnel for the public port and report the result (including the
/// enable/HTTPS URL Tailscale prints when the tailnet is not configured yet)
pub fn funnel_enable(port: u16) -> Value {
    if port == 0 { return json!({ "ok": false, "error": "public port is disabled" }); }
    match cmd().args(["funnel", "--bg", &port.to_string()]).output() {
        Ok(o) => {
            let mut text = String::from_utf8_lossy(&o.stdout).to_string();
            text.push_str(&String::from_utf8_lossy(&o.stderr));
            let url = first_url(&text);
            if o.status.success() {
                json!({ "ok": true, "url": url, "funnel": funnel_ok(port) })
            } else {
                log(&format!("tailscale funnel --bg {} failed: {}", port, text.trim()));
                json!({ "ok": false, "url": url, "error": text.trim() })
            }
        }
        Err(e) => {
            log(&format!("tailscale funnel --bg {} failed: {}", port, e));
            json!({ "ok": false, "error": e.to_string() })
        }
    }
}

pub fn funnel(port: u16) {
    let r = funnel_enable(port);
    if r.get("ok").and_then(|x| x.as_bool()) != Some(true) {
        log(&format!("tailscale funnel {}: {}", port, r));
    }
}

/// true when Tailscale is connected (so the caller can point the funnel at us)
pub fn running() -> bool {
    status().get("running").and_then(|x| x.as_bool()).unwrap_or(false)
}

/// keep Funnel pointed at the public port.
/// Node name is forced to [HOSTNAME]; stale rules (e.g. an old browser-debug port that
/// answers "Host header is specified and is not an IP address or localhost") are dropped
/// by resetting before publishing.
pub fn ensure(port: u16) {
    if !running() { return; }
    let dns = dns_name().to_ascii_lowercase();
    if !dns.starts_with(&format!("{}.", HOSTNAME)) {
        set_hostname(HOSTNAME);
        std::thread::sleep(std::time::Duration::from_millis(800));
    }
    if funnel_ok(port) { return; }
    reset_funnel();
    funnel(port);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulls_the_first_login_url_out_of_cli_output() {
        let text = "To authenticate, visit:\n\n\thttps://login.tailscale.com/a/abc123\n\nSuccess.";
        assert_eq!(first_url(text).as_deref(), Some("https://login.tailscale.com/a/abc123"));
        assert_eq!(first_url("Funnel is not enabled. Visit https://login.tailscale.com/f/funnel?node=xyz.").as_deref(),
            Some("https://login.tailscale.com/f/funnel?node=xyz"));
        assert_eq!(first_url("no url here"), None);
    }

    #[test]
    fn funnel_is_rejected_when_the_public_port_is_off() {
        assert_eq!(funnel_enable(0).get("ok").and_then(|x| x.as_bool()), Some(false));
    }

    #[test]
    fn compares_versions_numerically() {
        assert!(is_older("1.104.1", "1.104.2"));
        assert!(is_older("1.99.9", "1.104.0"));
        assert!(!is_older("1.104.2", "1.104.2"));
        assert!(!is_older("1.105.0", "1.104.9"));
    }
}
