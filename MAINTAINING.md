# Maintaining / developing

Dev, build, and release notes for the PoE Wishlist Overlay (Tauri + React).
User-facing info is in [README.md](README.md).

## Toolchain (one-time)

Needs **Node** + **WebView2** (preinstalled on Win11), plus **Rust** and the
**MSVC C++ build tools**:

```powershell
winget install Rustlang.Rustup
rustup default stable-msvc
winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

## Run (dev)

```powershell
npm install
npm run tauri icon app-icon.png   # once — regenerates src-tauri/icons/*
npm run tauri dev
```

First `tauri dev` compiles all crates (several minutes); later runs are fast.

## Architecture

```
[hotkey] -> Rust: Ctrl+C, read item off clipboard, parse candidate names
         -> Rust: POST names to the wishlist-lookup Edge Function (token only)
         -> server resolves the league FROM the token, returns who wants it
         -> Rust: position + show the overlay, emit to the webview to render
[Found]  -> Rust: POST {finder, want} -> a finds row (same one the web bell reads)
[toast]  -> Rust polls "notifications" for finds targeting you, while PoE runs
```

- **`src-tauri/src/main.rs`** — hotkey, clipboard copy + parse, HTTP calls, window
  positioning, click-outside watch, tray, notification poller, updater check, and
  settings persistence (`settings.json` in the OS app-config dir).
- **`src/Overlay.tsx`** — the result panel (Found buttons, optional note).
- **`src/Settings.tsx`** — token onboarding + tabbed, auto-applying settings.
- **`src/Toast.tsx`** — the corner notification window.
- **`src/theme.css`** — mirrors the web app's gold/dark palette.

**The token IS the league.** Each token maps 1:1 to a private league on the server
(`league_tokens` table); the client sends only the token and the server resolves
the league. Identity is client-declared (you pick your name) — fine for a trusted
private group. The notifier only polls **while PoE is running**, so idle usage ≈ 0.

## Build a standalone installer

```powershell
npm run tauri build
```

Produces an NSIS installer under `src-tauri/target/release/bundle/nsis/`.

## Auto-update & releasing

The app checks this repo's latest release manifest
(`.../releases/latest/download/latest.json`) on launch; if a newer **signed**
build exists it downloads, verifies the signature against the baked-in public key
(`tauri.conf.json` → `plugins.updater.pubkey`), installs, and relaunches.

To cut a release:

1. Bump `version` in `src-tauri/tauri.conf.json` (and `package.json` to match).
2. Run:
   ```powershell
   powershell -File scripts/publish.ps1
   ```
   It builds a **signed** installer and publishes a GitHub release (installer +
   `latest.json`). Installed apps update on their next launch.

### ⚠️ The signing key is critical

It lives at `~/.tauri/xddgaming_updater.key` (never in the repo). **Back it up.**
If it's lost you can't sign updates and auto-update breaks for everyone —
recovery means shipping a fresh install with a new keypair (new
`plugins.updater.pubkey`).

## Notes

- Rare/magic items aren't matched (the group wishlists named items); the Edge
  Function owns matching, so that can improve without rebuilding the app.
- Backend (the `wishlist-lookup` Edge Function, RLS, token-rotation SQL) lives in
  the private xddGaming web-app repo.
