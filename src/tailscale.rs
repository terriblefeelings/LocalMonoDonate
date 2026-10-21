//! Tailscale integration: the app drives the Tailscale CLI so a user can get a
//! public `https://<nickname>.<tailnet>.ts.net` link with one "Connect" click —
//! no Cloudflare, no domain, works behind CGNAT.

use serde_json::{json, Value};
use std::process::{Command, Stdio};

const MSI: &str = r"C:\Program Files\Tailscale\tailscale.exe";

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
        let _ = cmd().args(["set", "--hostname", name]).output();
    }
}

pub fn funnel(port: u16) {
    let _ = cmd().args(["funnel", "--bg", &port.to_string()]).output();
}

/// true when Tailscale is connected (so the caller can point the funnel at us)
pub fn running() -> bool {
    status().get("running").and_then(|x| x.as_bool()).unwrap_or(false)
}

/// keep Funnel pointed at the public port (node name stays fixed: <nickname> goes in the path)
pub fn ensure(port: u16) {
    if running() { funnel(port); }
}
