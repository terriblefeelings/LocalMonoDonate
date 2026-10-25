//! Windows notification-area icon: closing/minimising the window hides it here,
//! the tray menu can reopen it or exit the process.
#![cfg(windows)]

use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Duration;
use tao::event_loop::EventLoopProxy;
use tao::platform::windows::WindowExtWindows;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, ShowWindow, TrackPopupMenu, TranslateMessage,
    HWND_MESSAGE, IDI_APPLICATION, MF_STRING, SW_HIDE, SW_RESTORE, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WM_APP, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONDBLCLK, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
};

#[derive(Clone, Copy, Debug)]
pub enum TrayEvent {
    Show,
    Quit,
}

const WM_TRAY: u32 = WM_APP + 1;
const ID_OPEN: usize = 1;
const ID_EXIT: usize = 2;
static PROXY: OnceLock<Mutex<Option<EventLoopProxy<TrayEvent>>>> = OnceLock::new();

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn notify(event: TrayEvent) {
    if let Some(cell) = PROXY.get() {
        if let Ok(guard) = cell.lock() {
            if let Some(proxy) = guard.as_ref() {
                let _ = proxy.send_event(event);
            }
        }
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() { return; }
    let open = wide("Open");
    let exit = wide("Exit");
    AppendMenuW(menu, MF_STRING, ID_OPEN, open.as_ptr());
    AppendMenuW(menu, MF_STRING, ID_EXIT, exit.as_ptr());
    let mut point = std::mem::zeroed();
    GetCursorPos(&mut point);
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_RETURNCMD,
        point.x,
        point.y,
        0,
        hwnd,
        std::ptr::null(),
    );
    DestroyMenu(menu);
    PostMessageW(hwnd, WM_NULL, 0, 0);
    if cmd == ID_OPEN as i32 {
        notify(TrayEvent::Show);
    } else if cmd == ID_EXIT as i32 {
        notify(TrayEvent::Quit);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                show_menu(hwnd);
            } else if event == WM_LBUTTONDBLCLK {
                notify(TrayEvent::Show);
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn run_icon(ready: mpsc::Sender<bool>) {
    let instance = GetModuleHandleW(std::ptr::null());
    let class_name = wide("LocalMonoDonateTray");
    let window_title = wide("LocalMonoDonate");
    let wc = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };
    if RegisterClassW(&wc) == 0 {
        let _ = ready.send(false); return;
    }
    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        window_title.as_ptr(),
        0,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        std::ptr::null_mut(),
        instance,
        std::ptr::null(),
    );
    if hwnd.is_null() {
        let _ = ready.send(false); return;
    }
    let mut icon = LoadIconW(instance, 1 as *const u16);
    if icon.is_null() {
        icon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
    }
    let mut data: NOTIFYICONDATAW = std::mem::zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = icon;
    for (slot, ch) in data.szTip.iter_mut().zip("LocalMonoDonate".encode_utf16()) {
        *slot = ch;
    }
    if Shell_NotifyIconW(NIM_ADD, &data) == 0 {
        let _ = ready.send(false); return;
    }
    let _ = ready.send(true);
    let mut msg: windows_sys::Win32::UI::WindowsAndMessaging::MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    Shell_NotifyIconW(NIM_DELETE, &data);
}

/// Hide the app window while keeping the tray icon and all background services alive.
pub fn hide(window: &tao::window::Window) {
    let hwnd = window.hwnd() as isize as *mut std::ffi::c_void;
    unsafe { ShowWindow(hwnd, SW_HIDE); }
}

/// Bring the hidden window back from the notification area.
pub fn show(window: &tao::window::Window) {
    let hwnd = window.hwnd() as isize as *mut std::ffi::c_void;
    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
        SetForegroundWindow(hwnd);
    }
}

/// Install the tray icon. Returns false when Windows could not create it;
/// the window then keeps the normal close-to-exit behaviour.
pub fn start(proxy: EventLoopProxy<TrayEvent>) -> bool {
    let cell = PROXY.get_or_init(|| Mutex::new(None));
    *cell.lock().unwrap() = Some(proxy);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        unsafe { run_icon(tx) };
    });
    rx.recv_timeout(Duration::from_secs(3)).unwrap_or(false)
}
