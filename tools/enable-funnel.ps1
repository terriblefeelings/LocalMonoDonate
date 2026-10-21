<#
.SYNOPSIS
  Sets up Tailscale Funnel for LocalMonoDonate so the donation page is reachable by viewers.

.DESCRIPTION
  Read-only checks + the one command the app itself cannot do reliably:
    tailscale funnel --bg <publicPort>

  It finds tailscale.exe, verifies the node is logged in, optionally resets old
  funnel rules, optionally renames the node, publishes the public port and prints
  the final donation URL. If the tailnet blocks Funnel or HTTPS certificates are
  off, it prints the exact admin-console steps instead of failing silently.

.PARAMETER Port
  Public port to publish. Defaults to publicPort from
  %APPDATA%\LocalMonoDonate\config.json, or 8080.

.PARAMETER Nickname
  Donation-page path segment. Defaults to nickname from config.json.

.PARAMETER Hostname
  Optional: rename the Tailscale node (tailscale set --hostname ...).

.PARAMETER Reset
  Run "tailscale funnel reset" before publishing.

.PARAMETER StatusOnly
  Only print status/funnel state; change nothing.

.PARAMETER NoOpen
  Do not open the admin console / donation page in the browser.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File enable-funnel.ps1

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File enable-funnel.ps1 -Nickname mystream -Reset
#>
param(
  [int]$Port = 0,
  [string]$Nickname = '',
  [string]$Hostname = '',
  [switch]$Reset,
  [switch]$StatusOnly,
  [switch]$NoOpen
)

$ErrorActionPreference = 'Stop'

function Write-Step([string]$text) { Write-Host "==> $text" -ForegroundColor Cyan }
function Write-Ok([string]$text)   { Write-Host "  OK  $text" -ForegroundColor Green }
function Write-Warn([string]$text) { Write-Host "  !   $text" -ForegroundColor Yellow }
function Write-Err([string]$text)  { Write-Host "  X   $text" -ForegroundColor Red }

function Open-Url([string]$url) {
  if ($NoOpen) { return }
  try { Start-Process $url | Out-Null } catch { }
}

function Test-LocalPort([int]$port) {
  $client = New-Object System.Net.Sockets.TcpClient
  try {
    $iar = $client.BeginConnect('127.0.0.1', $port, $null, $null)
    if (-not $iar.AsyncWaitHandle.WaitOne(800)) { return $false }
    $client.EndConnect($iar)
    return $true
  } catch { return $false } finally { $client.Close() }
}

# ---- read LocalMonoDonate config for defaults --------------------------------
$cfg = $null
$cfgPath = Join-Path $env:APPDATA 'LocalMonoDonate\config.json'
if (Test-Path $cfgPath) {
  try { $cfg = Get-Content $cfgPath -Raw | ConvertFrom-Json } catch { }
}
if (-not $Port) {
  $Port = if ($cfg -and $cfg.publicPort) { [int]$cfg.publicPort } else { 8080 }
}
if (-not $Nickname -and $cfg) { $Nickname = [string]$cfg.nickname }
if ($Port -lt 1 -or $Port -gt 65535) { Write-Err "Invalid port: $Port"; exit 2 }

# ---- locate tailscale.exe ---------------------------------------------------
$candidates = @(
  (Join-Path $env:ProgramFiles 'Tailscale\tailscale.exe')
)
if (${env:ProgramFiles(x86)}) { $candidates += (Join-Path ${env:ProgramFiles(x86)} 'Tailscale\tailscale.exe') }
$ts = $candidates | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $ts) {
  $cmd = Get-Command 'tailscale.exe' -ErrorAction SilentlyContinue
  if ($cmd) { $ts = $cmd.Source }
}
if (-not $ts) {
  Write-Err 'Tailscale is not installed.'
  Write-Host '  Install it from https://tailscale.com/download/windows (or run the bundled tailscale-setup.msi).'
  Open-Url 'https://tailscale.com/download/windows'
  exit 1
}
Write-Step "Tailscale: $ts"

# ---- node status ------------------------------------------------------------
function Get-TsStatus {
  try {
    $raw = & $ts status --json 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $raw) { return $null }
    return ($raw | ConvertFrom-Json)
  } catch { return $null }
}

$status = Get-TsStatus
if (-not $status -or $status.BackendState -ne 'Running') {
  Write-Warn "Tailscale is not connected (state: $($status.BackendState))."
  Write-Step 'Trying to start the Tailscale service...'
  try { Start-Service Tailscale -ErrorAction Stop; Write-Ok 'service started' } catch { Write-Warn "could not start service: $($_.Exception.Message)" }
  Start-Sleep -Seconds 4
  $status = Get-TsStatus
}

$dnsName = ''
if ($status -and $status.Self -and $status.Self.DNSName) { $dnsName = ([string]$status.Self.DNSName).TrimEnd('.') }

if (-not $status -or $status.BackendState -ne 'Running') {
  Write-Err 'Tailscale is still not connected.'
  Write-Host "  Log in once:  & `"$ts`" up"
  Write-Host '  (or open the Tailscale tray icon and sign in), then run this script again.'
  exit 1
}
Write-Ok "connected, node: $dnsName"

# ---- status-only mode -------------------------------------------------------
if ($StatusOnly) {
  Write-Step 'funnel status'
  & $ts funnel status 2>&1 | ForEach-Object { "  $_" }
  exit 0
}

# ---- reset ------------------------------------------------------------------
if ($Reset) {
  Write-Step 'Resetting previous funnel rules'
  & $ts funnel reset 2>&1 | ForEach-Object { "  $_" }
}

# ---- optional hostname ------------------------------------------------------
if ($Hostname) {
  Write-Step "Setting node hostname to '$Hostname'"
  & $ts set --hostname $Hostname 2>&1 | ForEach-Object { "  $_" }
  Start-Sleep -Seconds 1
  $status = Get-TsStatus
  if ($status -and $status.Self -and $status.Self.DNSName) { $dnsName = ([string]$status.Self.DNSName).TrimEnd('.') }
}

# ---- the app must listen on the public port ---------------------------------
if (Test-LocalPort $Port) {
  Write-Ok "local port $Port is listening"
} else {
  Write-Warn "local port $Port is not listening - start LocalMonoDonate.exe first."
  Write-Host "         Funnel can be created anyway; the page will be 502 until the app runs."
}

# ---- publish ----------------------------------------------------------------
Write-Step "Publishing port $Port with Tailscale Funnel"
$out = & $ts funnel --bg $Port 2>&1
$outText = ($out | Out-String)
$out | ForEach-Object { "  $_" }
$funnelExit = $LASTEXITCODE

Write-Step 'funnel status'
$funnelStatus = (& $ts funnel status 2>&1 | Out-String)
$funnelStatus.Trim() -split "`n" | ForEach-Object { "  $_" }

$urls = [regex]::Matches($funnelStatus, 'https://[^\s:]+') | ForEach-Object { $_.Value } | Select-Object -Unique
$base = if ($urls) { $urls[0] } elseif ($dnsName) { "https://$dnsName" } else { '' }
$donation = if ($base -and $Nickname) { "$base/$Nickname/donation" } elseif ($base) { "$base/<nickname>/donation" } else { '' }

if ($funnelExit -ne 0 -or $outText -match 'not enabled|HTTPS is not enabled|certificate|Funnel is not') {
  Write-Err 'Funnel could not be enabled automatically.'
  Write-Host ''
  Write-Host '  One-time tailnet setup, then run this script again:' -ForegroundColor Yellow
  Write-Host '   1. HTTPS certificates: https://login.tailscale.com/admin/dns  ->  HTTPS Certificates -> Enable'
  Write-Host '   2. Allow Funnel (Access controls -> nodeAttrs):'
  Write-Host '        "nodeAttrs": [ { "target": ["autogroup:member"], "attr": ["funnel"] } ]'
  Write-Host '      or use https://login.tailscale.com/f/funnel'
  Open-Url 'https://login.tailscale.com/admin/dns'
  exit 1
}

Write-Host ''
Write-Ok "public base: $base"
if ($donation) {
  Write-Host "  Donation page: $donation" -ForegroundColor Green
  Write-Host "  Local check:   http://127.0.0.1:$Port/$Nickname/donation"
  Open-Url $donation
} else {
  Write-Warn 'Nickname is unknown - pass -Nickname <name> to print the full donation URL.'
}
Write-Host ''
Write-Host "To undo:  & `"$ts`" funnel reset"
exit 0
