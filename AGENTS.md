# AGENTS.md — LocalMonoDonate

Rust single-exe Monobank donation-alert app with an auction feature,
serving local web pages (`/setup`, `/timeradmin`, `/auctionsettings`, `/auction`, …).

## Build & publish

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\scoop\apps\mingw\current\bin;$env:Path"
Stop-Process -Name LocalMonoDonate -Force -ErrorAction SilentlyContinue
cargo build --release
Copy-Item target\release\LocalMonoDonate.exe J:\MONOBANK\LocalMonoDonate.exe -Force
Remove-Item target\release\LocalMonoDonate.exe -Force -ErrorAction SilentlyContinue
```

Pages/JS are embedded with `include_str!` — **a rebuild is required** for the exe to serve
edited HTML/JS/CSS.

## Tests (run after every change)

```powershell
cd J:\MONOBANK\AUC
node _alltest.mjs       # 179 deep HTTP assertions (pages, APIs, auction, donations, timer, bits)
node combotest.mjs      # 33  auction mode x source x currency + timer extensions
node _deeptest.mjs      # 15  browser click-through + theme/lang
node capture_pages.mjs  # 53  screenshots -> J:\MONOBANK\shots
```

Tests run against a **temp copy** of the exe (own config/port); kill strays after:
`Stop-Process -Name LocalMonoDonate,chrome -Force`.

Set `$env:LMD_DATA=<temp dir>` before running them — without it `Paths::new()` falls back to
`%APPDATA%\LocalMonoDonate` and the copy reads the real profile (wrong port, tests time out).

## Conventions

- **Do not start the AUC Node server** — the user runs it via `the auction script`.
- App config: `%APPDATA%\LocalMonoDonate\config.json` (never in the repo/project folder). Secrets live in `%APPDATA%\LocalMonoDonate\<port>\`
  (`token`, `twitch.json`, `streamelements.json`) — never commit or print them.
- Avoid PowerShell JSON round-trips on config/data files while the app is stopped
  (double-encodes Cyrillic).
- Keep UI text EN + UA (`window.I18N`, `data-i18n`); default EN.
