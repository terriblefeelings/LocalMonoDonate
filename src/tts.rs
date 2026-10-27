//! Donation text-to-speech: talks to the local tts-test daemon (serve.py).
//!
//! The daemon is a persistent child process (warm model), spoken to over
//! stdin/stdout with one JSON line per request. It serves both Google voices
//! ("google:uk"/"google:us") and StyleTTS2-UA voices (e.g. "Тарас Василюк").

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct Daemon {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

static DAEMON: Mutex<Option<Daemon>> = Mutex::new(None);

fn spawn(dir: &str) -> Option<Daemon> {
    let mut base = PathBuf::from(dir);
    // a relative engine dir is resolved next to the exe (bundled tts-engine)
    if !base.is_absolute() {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(d) = exe.parent() { base = d.join(&base); }
        }
    }
    let py = base.join("venv2").join("Scripts").join("python.exe");
    let serve = base.join("serve.py");
    if !py.exists() || !serve.exists() {
        return None;
    }
    let mut cmd = Command::new(py);
    cmd.arg("-X").arg("utf8").arg(serve)
        .env("PYTHONUTF8", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let mut child = cmd.spawn().ok()?;
    let stdin = child.stdin.take()?;
    let stdout = BufReader::new(child.stdout.take()?);
    Some(Daemon { child, stdin, stdout })
}

fn ensure(dir: &str) -> Result<(), String> {
    let mut g = DAEMON.lock().unwrap();
    let dead = match g.as_mut() {
        Some(d) => d.child.try_wait().ok().flatten().is_some(),
        None => true,
    };
    if dead {
        *g = spawn(dir);
    }
    if g.is_none() {
        return Err(format!("tts daemon unavailable (check {}\\serve.py)", dir));
    }
    Ok(())
}

/// Synthesize `text` with `voice` into `out` (absolute wav path). Blocking.
/// `key`/`region` are required only for Azure voices.
pub fn synth(dir: &str, voice: &str, text: &str, out: &Path, key: &str, region: &str, speed: f64) -> Result<(), String> {
    ensure(dir)?;
    let mut g = DAEMON.lock().unwrap();
    let d = g.as_mut().ok_or("tts daemon unavailable")?;
    let req = serde_json::json!({
        "voice": voice,
        "text": text,
        "out": out.to_string_lossy(),
        "key": key,
        "region": region,
        "speed": speed,
    });
    writeln!(d.stdin, "{}", req).map_err(|e| e.to_string())?;
    d.stdin.flush().ok();
    let mut line = String::new();
    d.stdout.read_line(&mut line).map_err(|e| e.to_string())?;
    if line.is_empty() {
        *g = None;
        return Err("tts daemon closed".into());
    }
    let v: serde_json::Value = serde_json::from_str(line.trim()).map_err(|e| e.to_string())?;
    if v.get("ok").and_then(|x| x.as_bool()) == Some(true) {
        Ok(())
    } else {
        Err(v.get("error").and_then(|x| x.as_str()).unwrap_or("tts error").to_string())
    }
}

/// Warm the daemon (spawn it and wait until the model is loaded). Call at startup.
pub fn warm(dir: &str) {
    if ensure(dir).is_err() {
        return;
    }
    let mut g = DAEMON.lock().unwrap();
    if let Some(d) = g.as_mut() {
        if writeln!(d.stdin, "{{\"cmd\":\"ping\"}}").is_ok() {
            d.stdin.flush().ok();
            let mut line = String::new();
            let _ = d.stdout.read_line(&mut line);
        }
    }
}

/// The kept StyleTTS2-UA voices (exact names from the model's voices.json).
pub const CUSTOM_VOICES: [&str; 12] = [
    "Анастасія Павленко", "Вʼячеслав Дудко", "Влада Муравець", "Денис Денисенко",
    "Катерина Потапенко", "Кирило Татарченко", "Людмила Чиркова", "Марта Мольфар",
    "Павло Буковський", "Роман Куліш", "Тарас Василюк", "Юрій Вихованець",
];

/// Azure Speech voices offered: Ukrainian and US, one male + one female each.
pub const AZURE_UK: [&str; 2] = ["uk-UA-PolinaNeural", "uk-UA-OstapNeural"];
pub const AZURE_US: [&str; 2] = ["en-US-JennyNeural", "en-US-GuyNeural"];
pub const AZURE_VOICES: [&str; 4] = [
    "uk-UA-PolinaNeural", "uk-UA-OstapNeural", "en-US-JennyNeural", "en-US-GuyNeural",
];

/// true when the local StyleTTS2 model ships next to the TTS engine (full builds only)
pub fn style_available(dir: &str) -> bool {
    let mut base = std::path::PathBuf::from(dir);
    if !base.is_absolute() {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(d) = exe.parent() { base = d.join(&base); }
        }
    }
    base.join("styletts_istftnet_fp16.onnx").exists()
}

/// Voices offered in the UI: Google UA/EN (default), Azure when keys are set, and the
/// StyleTTS2-UA names only when the model is present (the lite build ships Google/Azure).
pub fn voices(dir: &str, azure_ok: bool) -> Vec<serde_json::Value> {
    let mut v = vec![
        serde_json::json!({ "id": "google:uk", "label": "Google (українська)", "lang": "uk" }),
        serde_json::json!({ "id": "google:us", "label": "Google (English US)", "lang": "en" }),
        serde_json::json!({ "id": "edge:uk-UA-PolinaNeural", "label": "Microsoft Polina (нейро)", "lang": "uk" }),
        serde_json::json!({ "id": "edge:uk-UA-OstapNeural", "label": "Microsoft Ostap (нейро)", "lang": "uk" }),
        serde_json::json!({ "id": "edge:en-US-JennyNeural", "label": "Microsoft Jenny (neural)", "lang": "en" }),
        serde_json::json!({ "id": "edge:en-US-GuyNeural", "label": "Microsoft Guy (neural)", "lang": "en" }),
    ];
    if azure_ok {
        for name in AZURE_VOICES {
            v.push(serde_json::json!({ "id": format!("azure:{}", name), "label": format!("Azure {}", name), "lang": if name.starts_with("uk") { "uk" } else { "en" } }));
        }
    }
    if style_available(dir) {
        for name in CUSTOM_VOICES {
            v.push(serde_json::json!({ "id": name, "label": name, "lang": "uk" }));
        }
    }
    v
}

/// Build the spoken text for a donation according to `read_mode`:
/// "message" | "announcement" | "both".
pub fn spoken_text(read_mode: &str, name: &str, money: &str, message: &str, money_text: &str) -> String {
    let announce = money_text.replace("%donate%", name).replace("%money%", money);
    let announce = announce.trim();
    let msg = message.trim();
    match read_mode {
        "message" => msg.to_string(),
        "both" => {
            if announce.is_empty() { msg.to_string() }
            else if msg.is_empty() { announce.to_string() }
            else { format!("{}. {}", announce, msg) }
        }
        _ => announce.to_string(), // "announcement" (default)
    }
}
