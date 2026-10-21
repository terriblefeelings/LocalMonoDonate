# AGENTS.md — LocalMonoDonate

Rust-застосунок в одному exe для донат-алертів Monobank з аукціоном, який віддає локальні веб-сторінки
(`/setup`, `/timeradmin`, `/auctionsettings`, `/auction`, `/qr`, …).

## Збірка та публікація

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\scoop\apps\mingw\current\bin;$env:Path"
Stop-Process -Name LocalMonoDonate -Force -ErrorAction SilentlyContinue
cargo build --release
Copy-Item target\release\LocalMonoDonate.exe J:\MONOBANK\LocalMonoDonate.exe -Force
Remove-Item target\release\LocalMonoDonate.exe -Force -ErrorAction SilentlyContinue
```

Сторінки/JS вбудовані через `include_str!` — **після редагування HTML/JS/CSS потрібна перезбірка**, щоб
exe віддавав зміни.

## Тести (запускати після кожної зміни)

```powershell
cd J:\MONOBANK\AUC
node _alltest.mjs       # 187 глибоких HTTP-перевірок (сторінки, API, аукціон, донати, таймер, біти, QR)
node combotest.mjs      # 33 комбінації режимів аукціону × джерело × валюта + продовження таймера
node _deeptest.mjs      # 15 кліків у браузері + тема/мова
node capture_pages.mjs  # 53 скріншоти -> J:\MONOBANK\shots
```

Тести працюють із **тимчасовою копією** exe (власний конфіг/порт); після них прибирайте процеси:
`Stop-Process -Name LocalMonoDonate,chrome -Force`.

Перед запуском задайте `$env:LMD_DATA=<тимчасова папка>` — без цього `Paths::new()` візьме
`%APPDATA%\LocalMonoDonate` і копія читатиме реальний профіль (не той порт, тести падають за таймаутом).

## Домовленості

- **Не запускайте AUC Node-сервер** — користувач запускає його через `the auction script`.
- Конфіг застосунку: `%APPDATA%\LocalMonoDonate\config.json` (ніколи в репозиторії/папці проєкту).
  Секрети — в `%APPDATA%\LocalMonoDonate\<port>\` (`token`, `twitch.json`, `streamelements.json`) —
  ніколи не комітити й не друкувати.
- Не робіть JSON round-trip через PowerShell для config/data, поки застосунок зупинений
  (подвійне кодування кирилиці).
- Текст UI — EN + UA (`window.I18N`, `data-i18n`); типово EN.
- Документація репозиторію — українською (`README.md`, `docs/FAQ.md`, `docs/PRIVACY.md`), код і
  коментарі — англійською.
