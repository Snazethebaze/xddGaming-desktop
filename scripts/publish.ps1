# Release + publish the PoE Wishlist Overlay to GitHub Releases (auto-update).
#
# Usage:  bump "version" in src-tauri/tauri.conf.json (and package.json to match),
#         then from the desktop/ folder run:   powershell -File scripts/publish.ps1
#
# Prereqs: Rust toolchain on PATH, `gh` authenticated, and the updater signing key
#          at ~/.tauri/xddgaming_updater.key (KEEP IT SAFE - losing it breaks updates).

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent) # -> desktop/

$repo = "Snazethebaze/xddGaming-desktop"
$keyPath = Join-Path $env:USERPROFILE ".tauri\xddgaming_updater.key"
if (-not (Test-Path $keyPath)) { throw "Signing key not found at $keyPath" }

$version = (Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json).version
Write-Host "Building + publishing v$version ..." -ForegroundColor Cyan

# --- signed build (CI=true makes the signer non-interactive) ---
$env:CI = "true"
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $keyPath -Raw)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw "tauri build failed" }

# --- assemble the release assets ---
$bundle = "src-tauri/target/release/bundle/nsis"
$src = Join-Path $bundle "PoE Wishlist Overlay_$($version)_x64-setup.exe"
if (-not (Test-Path "$src.sig")) { throw "Signature not found - was the key set? ($src.sig)" }
$sig = (Get-Content "$src.sig" -Raw).Trim()

$asset = "xddGaming-Overlay_$($version)_x64-setup.exe" # no spaces -> clean URL
$rel = Join-Path $env:TEMP "xddgaming-release"
if (Test-Path $rel) { Remove-Item $rel -Recurse -Force }
New-Item -ItemType Directory -Force $rel | Out-Null
Copy-Item $src (Join-Path $rel $asset) -Force

$url = "https://github.com/$repo/releases/download/v$version/$asset"
$pubdate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
$manifest = [ordered]@{
  version   = $version
  notes     = "PoE Wishlist Overlay $version."
  pub_date  = $pubdate
  platforms = [ordered]@{ "windows-x86_64" = [ordered]@{ signature = $sig; url = $url } }
} | ConvertTo-Json -Depth 6
$manifestPath = Join-Path $rel "latest.json"
[System.IO.File]::WriteAllText($manifestPath, $manifest, (New-Object System.Text.UTF8Encoding $false))

# --- publish (this release becomes "latest", which the app checks) ---
gh release create "v$version" (Join-Path $rel $asset) $manifestPath --repo $repo `
  --title "v$version" --notes "Auto-update release. Installed apps update themselves on next launch."
if ($LASTEXITCODE -ne 0) { throw "gh release create failed" }

Write-Host "Published v$version -> https://github.com/$repo/releases/tag/v$version" -ForegroundColor Green
