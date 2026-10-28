//! LMDUpdate - self-contained updater.
//!
//! pack.ps1 appends the update payload (a zip of the app files) to this exe
//! followed by an 8-byte length and the `LMDZ` magic. The updater finds the
//! running LocalMonoDonate, waits for it, copies the payload over the app
//! folder and starts it again.
//!
//! Auto-update calls it with `--pid <pid> --dir <folder> --quiet`; a user can
//! just double-click the downloaded file.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_ICONERROR, MB_ICONQUESTION, MB_OK, MB_YESNO,
};

const MAGIC: &[u8; 4] = b"LMDZ";

fn message(text: &str, flags: u32) -> i32 {
    #[cfg(windows)]
    {
        let text: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let title: Vec<u16> = "LMD Update".encode_utf16().chain(std::iter::once(0)).collect();
        return unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), flags) };
    }
    #[cfg(not(windows))]
    {
        let _ = (text, flags);
        0
    }
}

/// The zip appended by pack.ps1: [zip][length u64][LMDZ].
fn payload() -> Option<Vec<u8>> {
    let exe = std::fs::read(std::env::current_exe().ok()?).ok()?;
    if exe.len() < 12 { return None; }
    let tail = &exe[exe.len() - 12..];
    if &tail[8..12] != MAGIC { return None; }
    let len = u64::from_le_bytes(tail[..8].try_into().ok()?) as usize;
    if len == 0 || len + 12 > exe.len() { return None; }
    Some(exe[exe.len() - 12 - len..exe.len() - 12].to_vec())
}

#[cfg(windows)]
fn process_path(pid: u32) -> Option<PathBuf> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() { return None; }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 { return None; }
        Some(PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))
    }
}

#[cfg(windows)]
fn find_running() -> Option<(u32, PathBuf)> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE { return None; }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = None;
        let mut ok = Process32FirstW(snap, &mut entry);
        while ok != 0 {
            let name = String::from_utf16_lossy(&entry.szExeFile);
            let name = name.trim_end_matches('\0');
            if name.eq_ignore_ascii_case("LocalMonoDonate.exe") {
                if let Some(path) = process_path(entry.th32ProcessID) {
                    found = Some((entry.th32ProcessID, path));
                    break;
                }
            }
            ok = Process32NextW(snap, &mut entry);
        }
        CloseHandle(snap);
        found
    }
}

#[cfg(windows)]
fn terminate(pid: u32) {
    if pid == 0 { return; }
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, 0, pid);
        if !h.is_null() {
            TerminateProcess(h, 1);
            WaitForSingleObject(h, 5000);
            CloseHandle(h);
        }
    }
}

/// Run the PowerShell helper that waits, extracts the payload and restarts.
fn apply(zip: &Path, install: &Path, pid: u32) -> bool {
    let script = std::env::temp_dir().join(format!("LMD-update-{}.ps1", std::process::id()));
    let log = std::env::temp_dir().join(format!("LMD-update-{}.log", std::process::id()));
    let body = format!(
        r#"$ErrorActionPreference = 'Stop'
$appPid = {pid}
$zip = {zip}
$dst = {dst}
$log = {log}
try {{
  if ($appPid -gt 0) {{ Wait-Process -Id $appPid -Timeout 60 -ErrorAction SilentlyContinue }}
  Start-Sleep -Milliseconds 700
  $tmp = Join-Path $env:TEMP ('LMD-apply-' + [guid]::NewGuid().ToString('N'))
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  [System.IO.Compression.ZipFile]::ExtractToDirectory($zip, $tmp)
  Get-ChildItem -LiteralPath $tmp -Force | Where-Object {{ $_.Name -ne 'update-version.txt' }} | ForEach-Object {{
    Copy-Item -LiteralPath $_.FullName -Destination $dst -Recurse -Force
  }}
  Start-Process -FilePath (Join-Path $dst 'LocalMonoDonate.exe') -WorkingDirectory $dst
  'ok' | Out-File -LiteralPath $log -Append -Encoding ascii
  Start-Sleep -Seconds 1
  Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}} catch {{
  "fail: $_" | Out-File -LiteralPath $log -Append -Encoding ascii
  if (Test-Path -LiteralPath (Join-Path $dst 'LocalMonoDonate.exe')) {{
    Start-Process -FilePath (Join-Path $dst 'LocalMonoDonate.exe') -WorkingDirectory $dst
  }}
}}
"#,
        pid = pid,
        zip = ps_str(zip.to_string_lossy().as_ref()),
        dst = ps_str(install.to_string_lossy().as_ref()),
        log = ps_str(log.to_string_lossy().as_ref())
    );
    if std::fs::write(&script, body).is_err() { return false; }
    let mut cmd = Command::new("powershell.exe");
    cmd.current_dir(std::env::temp_dir())
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-File"])
        .arg(&script)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let ok = cmd.status().map(|s| s.success()).unwrap_or(false);
    let result = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_file(&script);
    let success = result.lines().last().map(|l| l.trim() == "ok").unwrap_or(false);
    if success || !ok { let _ = std::fs::remove_file(&log); }
    success
}

fn ps_str(s: &str) -> String { format!("'{}'", s.replace('\'', "''")) }

fn main() {
    let mut pid = 0u32;
    let mut dir: Option<PathBuf> = None;
    let mut quiet = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--pid" => pid = args.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            "--dir" => dir = args.next().map(PathBuf::from),
            "--quiet" => quiet = true,
            _ => {}
        }
    }

    let payload = match payload() {
        Some(p) => p,
        None => {
            message("This file has no update data.\n\nDownload LMD-Update-<version>.exe from the release page.", MB_ICONERROR | MB_OK);
            return;
        }
    };

    let (pid, install) = if let Some(dir) = dir {
        (pid, dir)
    } else if let Some((p, exe)) = find_running() {
        (p, exe.parent().map(|d| d.to_path_buf()).unwrap_or_default())
    } else {
        message("LocalMonoDonate is not running.\n\nStart it, then run this updater again: the running app tells the updater where it is installed.", MB_ICONERROR | MB_OK);
        return;
    };

    if !install.join("LocalMonoDonate.exe").exists() {
        message(&format!("LocalMonoDonate.exe was not found in:\n{}", install.display()), MB_ICONERROR | MB_OK);
        return;
    }

    if !quiet {
        let label = std::env::current_exe().ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| "update".into());
        let text = format!(
            "LocalMonoDonate found:\n{}\n\nApply {}?\n\nThe app will close, its files will be replaced and it will start again.",
            install.display(), label
        );
        if message(&text, MB_ICONQUESTION | MB_YESNO) != IDYES { return; }
        #[cfg(windows)]
        terminate(pid);
    }

    let zip = std::env::temp_dir().join(format!("LMD-update-{}.zip", std::process::id()));
    if std::fs::write(&zip, &payload).is_err() {
        message("Could not write the update to the temporary folder.", MB_ICONERROR | MB_OK);
        return;
    }
    let ok = apply(&zip, &install, pid);
    let _ = std::fs::remove_file(&zip);
    if !quiet {
        if ok {
            message("Update applied. LocalMonoDonate is starting.", MB_OK);
        } else {
            message("The update could not be applied. Start LocalMonoDonate and try again; your old files were kept.", MB_ICONERROR | MB_OK);
        }
    }
}
