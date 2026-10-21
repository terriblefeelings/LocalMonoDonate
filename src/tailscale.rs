//! Tailscale integration: the app drives the Tailscale CLI so a user can get a
//! public `https://<nickname>.<tailnet>.ts.net` link with one "Connect" click —
//! no Cloudflare, no domain, works behind CGNAT.

use crate::model::log;
use serde_json::{json, Value};
use std::process::{Command, Stdio};

const MSI: &str = r"C:\Program Files\Tailscale\tailscale.exe";
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

/// installed / logged in / public DNS name
pub fn status() -> Value {
    let installed = std::path::Path::new(MSI).exists()
        || cmd().arg("version").output().map(|o| o.status.success()).unwrap_or(false);
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
            if let Some(i) = s.find("https://login.tailscale.com/") {
                let url: String = s[i..].chars().take_while(|c| !c.is_whitespace()).collect();
                return Some(url);
            }
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

pub fn funnel(port: u16) {
    match cmd().args(["funnel", "--bg", &port.to_string()]).output() {
        Ok(o) if !o.status.success() => {
            let e = String::from_utf8_lossy(&o.stderr);
            log(&format!("tailscale funnel --bg {} failed: {}", port, e.trim()));
        }
        Err(e) => log(&format!("tailscale funnel --bg {} failed: {}", port, e)),
        _ => {}
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
