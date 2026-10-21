# Builds a clean, distributable copy of LocalMonoDonate.
# Nothing from this machine's setup is included: no config.json, no %APPDATA% secrets,
# no logs, no user media. A recipient starts with empty fields.
#
#   powershell -File pack.ps1                          # exe + WebView2 bits only (~9 MB)
#   powershell -File pack.ps1 -WithTts                 # + tts-engine from .\tts-engine
#   powershell -File pack.ps1 -WithTts -TtsEngine D:\tts-engine-lite
#
# The portable tts-engine (venv2, ffmpeg.exe, serve.py; Google/Azure voices only) is a
# ~260 MB runtime that is not stored in git - pass its folder with -TtsEngine.
param(
  [switch]$WithTts,
  [string]$TtsEngine = (Join-Path $PSScriptRoot 'tts-engine')
)
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$dist = Join-Path $root 'dist'
$app = Join-Path $dist 'LocalMonoDonate'
Remove-Item $dist -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $app | Out-Null

Push-Location $root
& cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
Pop-Location

Copy-Item (Join-Path $root 'target\release\LocalMonoDonate.exe') $app
Copy-Item (Join-Path $root 'WebView2Loader.dll') $app
Copy-Item (Join-Path $root 'tools\enable-funnel.ps1') $app
$boot = Join-Path $root 'MicrosoftEdgeWebview2Setup.exe'
if (Test-Path $boot) { Copy-Item $boot $app }
$ts = Join-Path $root 'tailscale-setup.msi'
if (Test-Path $ts) { Copy-Item $ts $app }
if ($WithTts) {
  if (-not (Test-Path (Join-Path $TtsEngine 'serve.py'))) {
    throw "tts-engine not found at '$TtsEngine' (pass -TtsEngine <folder>)"
  }
  $dst = Join-Path $app 'tts-engine'
  New-Item -ItemType Directory -Force $dst | Out-Null
  Copy-Item (Join-Path $TtsEngine 'venv2') $dst -Recurse
  Copy-Item (Join-Path $TtsEngine 'ffmpeg.exe') $dst
  Copy-Item (Join-Path $TtsEngine 'serve.py') $dst
}

# reference template only - deliberately empty, the app never reads it
@'
{
  "nickname": "",
  "overlayname": "overlay",
  "jars": [],
  "token": "",
  "features": { "donate": true, "stats": true, "overlay": true, "bits": true, "bitsOverlay": true, "youtube": true, "timer": true, "auction": true },
  "streamelements": { "enabled": false, "accountId": "" }
}
'@ | Set-Content (Join-Path $app 'config.example.json') -Encoding UTF8

Copy-Item (Join-Path $root 'README.md') (Join-Path $app 'README.md')

# zip can not store pre-1980 timestamps (WebView2Loader.dll ships with one)
Get-ChildItem $app -Recurse -File | ForEach-Object { $_.LastWriteTime = Get-Date }
$zip = Join-Path $dist ($(if ($WithTts) { 'MONOBANK-lite-0.2.3.zip' } else { 'LocalMonoDonate-0.2.3.zip' }))
Push-Location $dist
& tar.exe -a -c -f $zip 'LocalMonoDonate'
if ($LASTEXITCODE -ne 0) { throw 'zip failed' }
Pop-Location
"zip: $zip"
Get-ChildItem $app | Select-Object Name, @{n='MB';e={[math]::Round($_.Length/1MB,2)}}
