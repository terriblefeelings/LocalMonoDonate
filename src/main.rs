#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod auction;
mod icon;
mod model;
mod mono;
mod streamelements;
mod tailscale;
mod tray;
mod tts;
mod twitch;
mod web;

use model::*;

/// A public Funnel route must not outlive the app: reset it on the way out.
fn ts_cleanup() {
    if std::env::var_os("LMD_NO_TAILSCALE").is_none() { tailscale::reset_funnel(); }
}

/// OBS keeps its browser sources loaded until the scene is refreshed, so a source
/// that opened before the app never recovers on its own.
/// Twitch redirects the browser to a fixed loopback port; when the app itself had to
/// move, forward that one request so the registered redirect URI keeps working.
fn serve_oauth_shim(main_port: u16) {
    let addr = ("127.0.0.1", crate::twitch::OAUTH_PORT);
    let server = match tiny_http::Server::http(addr) {
        Ok(s) => s,
        Err(e) => {
            log(&format!("twitch oauth port {} unavailable: {}", crate::twitch::OAUTH_PORT, e));
            return;
        }
    };
    log(&format!("twitch oauth shim 127.0.0.1:{} -> {}", crate::twitch::OAUTH_PORT, main_port));
    for request in server.incoming_requests() {
        let url = request.url().to_string();
        if !url.starts_with("/twitch/callback") {
            let _ = request.respond(tiny_http::Response::empty(404));
            continue;
        }
        let target = format!("http://127.0.0.1:{}{}", main_port, url);
        let header = tiny_http::Header::from_bytes(&b"Location"[..], target.as_bytes()).unwrap();
        let _ = request.respond(tiny_http::Response::empty(302).with_header(header));
    }
}

#[cfg(windows)]
fn obs_running() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE { return false; }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snap, &mut entry) != 0;
        let mut found = false;
        while more {
            let end = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..end]).to_ascii_lowercase();
            if name == "obs64.exe" || name == "obs32.exe" { found = true; break; }
            more = Process32NextW(snap, &mut entry) != 0;
        }
        CloseHandle(snap);
        found
    }
}

#[cfg(not(windows))]
fn obs_running() -> bool { false }

#[cfg(windows)]
fn warn_obs_start_order() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONWARNING, MB_OK, MB_SETFOREGROUND, MB_TOPMOST,
    };
    let text: Vec<u16> = "LocalMonoDonate потрібно запускати до OBS.\n\nStart LocalMonoDonate before OBS.\0"
        .encode_utf16().collect();
    let caption: Vec<u16> = "LocalMonoDonate\0".encode_utf16().collect();
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(),
            MB_OK | MB_ICONWARNING | MB_TOPMOST | MB_SETFOREGROUND);
    }
}

#[cfg(not(windows))]
fn warn_obs_start_order() {}

fn main() {
    let base = Paths::new();
    init_logging(&base);
    let mut cfg = load_config(&base);
    let headless = std::env::args().any(|a| a == "--headless");

    // Dedicated, uncommon ports per listener. 8787 stays reserved for the Twitch
    // callback; legacy defaults are migrated once so links stop shifting.
    let oauth = crate::twitch::OAUTH_PORT;
    const LEGACY_APP: [u16; 3] = [8787, 8789, 8080];
    const LEGACY_AUC: [u16; 2] = [8788, 8790];
    const LEGACY_PUB: [u16; 1] = [8080];
    let mut moved = false;

    let app_target = if LEGACY_APP.contains(&cfg.port) || cfg.port == oauth { 27183 } else { cfg.port };
    if app_target != cfg.port || !port_free(app_target) {
        let old = cfg.port;
        cfg.port = pick_free_port(app_target, &[cfg.auction_port, oauth]);
        log(&format!("port {} is reserved or busy, using {}", old, cfg.port));
        moved = true;
    }

    if cfg.auction_port != 0 {
        let target = if LEGACY_AUC.contains(&cfg.auction_port) || cfg.auction_port == oauth {
            27184
        } else {
            cfg.auction_port
        };
        if target != cfg.auction_port || target == cfg.port || !port_free(target) {
            let old = cfg.auction_port;
            cfg.auction_port = pick_free_port(target, &[cfg.port, oauth]);
            log(&format!("auction port {} is reserved or busy, using {}", old, cfg.auction_port));
            moved = true;
        }
    }

    if cfg.public_port != 0 {
        let target = if LEGACY_PUB.contains(&cfg.public_port) { 27185 } else { cfg.public_port };
        let busy = target != cfg.public_port
            || target == cfg.port
            || target == cfg.auction_port
            || target == oauth
            || !port_free(target);
        if busy {
            let old = cfg.public_port;
            let skip = [cfg.port, cfg.auction_port, oauth];
            cfg.public_port = pick_free_port(target, &skip);
            log(&format!("public port {} is busy, using {}", old, cfg.public_port));
            moved = true;
        }
    }
    if moved { save_config(&base, &cfg); }

    log("LocalMonoDonate starting");
    if !headless && obs_running() {
        // OBS caches browser sources, so the app has to be up before OBS loads them
        log("OBS is already running: showing the start-order warning");
        std::thread::spawn(warn_obs_start_order);
    }

    let main_port = cfg.port;
    *model::BOUND_PORTS.lock().unwrap() = vec![cfg.port, cfg.auction_port, cfg.public_port, crate::twitch::OAUTH_PORT];
    for legacy in [8787u16, 8789u16] { model::migrate_port_secrets(legacy, cfg.port); }
    let auc_port = cfg.auction_port;
    let public_port = cfg.public_port;
    let ctx = app::Ctx::new(cfg, base.clone());
    // LMD_NO_TAILSCALE=1 keeps tests from renaming the node / touching funnel rules
    if std::env::var_os("LMD_NO_TAILSCALE").is_none() {
        std::thread::spawn(crate::tailscale::auto_update);
        std::thread::spawn(crate::tailscale::cleanup_bundled_msi);
        if public_port == 0 {
            // no public route wanted: drop one left behind by a previous run
            std::thread::spawn(crate::tailscale::reset_funnel);
        } else {
            std::thread::spawn(move || crate::tailscale::ensure(public_port));
            // Tailscale may come up after the app; keep the funnel attached without a restart
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
                if crate::tailscale::running() && !crate::tailscale::funnel_ok(public_port) {
                    crate::tailscale::ensure(public_port);
                }
            });
        }
    }
    { let c = ctx.clone(); std::thread::spawn(move || crate::tts::warm(&c.cfg().tts.dir)); }
    { let c = ctx.clone(); std::thread::spawn(move || app::run_poller(c)); }
    { let c = ctx.clone(); std::thread::spawn(move || twitch::run(c, main_port, false)); }
    if auc_port > 0 && auc_port != main_port {
        let c = ctx.clone(); std::thread::spawn(move || twitch::run(c, auc_port, true));
    }
    if !headless && std::env::var_os("LMD_NO_UPDATE").is_none() { web::start_update_check(); }
    { let c = ctx.clone(); std::thread::spawn(move || web::serve(c)); }
    if main_port != crate::twitch::OAUTH_PORT {
        std::thread::spawn(move || serve_oauth_shim(main_port));
    }

    if headless {
        loop { std::thread::sleep(std::time::Duration::from_secs(3600)); }
    }

    std::thread::sleep(std::time::Duration::from_millis(500));
    if let Err(e) = run_window(main_port) {
        log(&format!("window error: {}", e));
        // WebView2 runtime missing => run the bundled evergreen bootstrapper next to the exe
        if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
            let boot = dir.join("MicrosoftEdgeWebview2Setup.exe");
            if boot.exists() {
                log("starting WebView2 bootstrapper");
                let _ = std::process::Command::new(boot).args(["/silent", "/install"]).spawn();
            }
        }
    }
}

/// open a URL in the default system browser (ShellExecute)
fn open_external(url: &str) {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    let op: Vec<u16> = "open\0".encode_utf16().collect();
    let file: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), 1); }
}

fn run_window(main_port: u16) -> wry::Result<()> {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::window::{Icon, Theme, WindowBuilder};
    use wry::WebViewBuilder;

    let event_loop = EventLoopBuilder::<tray::TrayEvent>::with_user_event().build();
    let tray_proxy = event_loop.create_proxy();
    let icon = Icon::from_rgba(icon::rgba(false), icon::ICON_W, icon::ICON_H).ok();
    let start_light = LIGHT_THEME.load(Ordering::Relaxed);
    // muted e-ink paper / ink, so no white flash and the native frame matches
    let bg = if start_light { (250u8, 248, 239, 255) } else { (15u8, 14, 13, 255) };

    let window = WindowBuilder::new()
        .with_title("LocalMonoDonate")
        .with_window_icon(icon)
        .with_inner_size(LogicalSize::new(1280.0, 860.0))
        .with_theme(Some(if start_light { Theme::Light } else { Theme::Dark }))
        .with_background_color(bg)
        .build(&event_loop)
        .expect("failed to build window");

    paint_frame(&window, start_light);

    let data_dir = Paths::new().base.join("WebView2");
    let mut web_context = wry::WebContext::new(Some(data_dir));
    let _webview = WebViewBuilder::with_web_context(&mut web_context)
        .with_url(&format!("http://127.0.0.1:{}/setup", main_port))
        .with_background_color(bg)
        // open every link / window.open in the user's default system browser (never inside WebView2)
        .with_new_window_req_handler(|url| { open_external(&url); false })
        .build(&window)?;

    let tray_ok = tray::start(tray_proxy);

    let mut cur_light = start_light;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(500));
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                if tray_ok { log("tray: hide on close"); tray::hide(&window); } else { log("tray: close exits"); ts_cleanup(); *control_flow = ControlFlow::Exit; }
            }
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                if tray_ok && window.is_minimized() { log("tray: hide on minimize"); tray::hide(&window); }
            }
            Event::UserEvent(tray::TrayEvent::Show) => { log("tray: show"); tray::show(&window); }
            Event::UserEvent(tray::TrayEvent::Quit) => { ts_cleanup(); *control_flow = ControlFlow::Exit },
            Event::NewEvents(_) => {
                let l = LIGHT_THEME.load(Ordering::Relaxed);
                if l != cur_light {
                    cur_light = l;
                    window.set_theme(Some(if l { Theme::Light } else { Theme::Dark }));
                    paint_frame(&window, l);
                    if let Ok(ic) = Icon::from_rgba(icon::rgba(l), icon::ICON_W, icon::ICON_H) {
                        window.set_window_icon(Some(ic));
                    }
                }
            }
            _ => {}
        }
    });
}

/// Paint the native title bar in the same e-ink tones as the page.
#[cfg(windows)]
fn paint_frame(window: &tao::window::Window, light: bool) {
    use tao::platform::windows::WindowExtWindows;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
    };
    // DWM wants COLORREF: 0x00BBGGRR
    let (caption, text, border) = if light {
        (0x00EFF8FAu32, 0x000F0F10u32, 0x00CED8DAu32) // #FAF8EF / #100F0F / #DAD8CE
    } else {
        (0x000D0E0Fu32, 0x00E5F0F2u32, 0x00292C2Eu32) // #0F0E0D / #F2F0E5 / #2E2C29
    };
    unsafe {
        let hwnd = window.hwnd();
        for (attr, value) in [
            (DWMWA_CAPTION_COLOR, caption),
            (DWMWA_TEXT_COLOR, text),
            (DWMWA_BORDER_COLOR, border),
        ] {
            DwmSetWindowAttribute(
                hwnd as *mut core::ffi::c_void,
                attr as u32,
                &value as *const u32 as *mut core::ffi::c_void,
                4,
            );
        }
    }
}

#[cfg(not(windows))]
fn paint_frame(_window: &tao::window::Window, _light: bool) {}
