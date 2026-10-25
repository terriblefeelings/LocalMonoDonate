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

fn main() {
    let base = Paths::new();
    init_logging(&base);
    let mut cfg = load_config(&base);
    // the configured public port may already be taken (another app, a second copy, a dev
    // server). Pick the next free one, persist it and let Tailscale Funnel follow.
    if cfg.public_port != 0 {
        let busy = cfg.public_port == cfg.port
            || cfg.public_port == cfg.auction_port
            || !port_free(cfg.public_port);
        if busy {
            let old = cfg.public_port;
            let skip = [cfg.port, cfg.auction_port];
            cfg.public_port = pick_free_port(old, &skip);
            save_config(&base, &cfg);
            log(&format!("public port {} is busy, using {}", old, cfg.public_port));
        }
    }
    let headless = std::env::args().any(|a| a == "--headless");

    log("LocalMonoDonate starting");

    let main_port = cfg.port;
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

    if headless {
        loop { std::thread::sleep(std::time::Duration::from_secs(3600)); }
    }

    std::thread::sleep(std::time::Duration::from_millis(500));
    if let Err(e) = run_window() {
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

fn run_window() -> wry::Result<()> {
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
        .with_url("http://127.0.0.1:8787/setup")
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
