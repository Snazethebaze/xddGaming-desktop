# PoE Wishlist Overlay (desktop app)

A native overlay for the **xddGaming** Path of Exile wishlist board, built with
**Tauri (Rust) + React**. Hover an item in Path of Exile, press your hotkey, and a dark
PoE-themed panel appears by your cursor showing **who in the group wants it** —
like Awakened PoE Trade, but the answer is "who wants this." You can mark items
**Found** from the panel, and get a **corner toast** when someone finds one of
yours.

- Featherweight & non-intrusive: one global hotkey, sleeps otherwise; a
  transparent, always-on-top panel that doesn't steal game focus. Click anywhere
  outside it to dismiss.
- **Token-first onboarding** — paste your league token, pick your name, done. The
  **token *is* the league** (the server resolves the league from it), so there's
  no league field to get wrong.
- **Found** (one-click, optional note) writes to the same `finds` table the
  website's 🔔 reads — the two apps stay in sync automatically.
- **Toasts** (with optional sound + corner) when someone finds your item. The
  notifier only polls **while PoE is running**, so it costs nothing when idle.
- Runs in the **system tray** (right-click → Settings / Quit). Settings apply
  instantly (no Save button) and persist across restarts.

---

## One-time setup (build toolchain)

You already have **Node** and the **WebView2 runtime**. You still need **Rust**
and the **MSVC C++ build tools** (the linker Rust uses on Windows):

```powershell
winget install Rustlang.Rustup
rustup default stable-msvc
winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

(Restart the terminal afterwards so `cargo` is on `PATH`.)

---

## Run it (dev)

```powershell
cd desktop
npm install                       # once
npm run tauri icon app-icon.png   # once — generates src-tauri/icons/*
npm run tauri dev
```

The **first** `tauri dev` compiles all the Rust crates (several minutes); later
runs are fast. On first launch the setup window asks for your **league token**
(copy it from the top bar of the web app) → **Connect** → pick **your name**.
Then in PoE (**Windowed** or **Windowed Fullscreen**), hover an item and press
your hotkey (default **Alt+W**).

> The hotkey is **global** (works everywhere and consumes that combo
> system-wide), so pick something uncommon.

Settings are tabbed — **Connection** (league + your name), **Hotkey**, **Alerts**
(toast on/off, sound, corner, poll interval).

---

## Build a distributable installer

```powershell
npm run tauri build
```

Produces an NSIS installer under `src-tauri/target/release/bundle/nsis/`. Friends
run it once — **no Rust, Node, or AutoHotkey** needed. (Unsigned builds show a
Windows SmartScreen "unknown publisher" prompt until code-signed; normal for a
small app.)

---

## Auto-update & releasing

The app **auto-updates**: on launch it checks this repo's latest release manifest
(`.../releases/latest/download/latest.json`), and if there's a newer **signed**
version it downloads, verifies the signature against the baked-in public key,
installs, and relaunches. Silently no-ops if there's no update or you're offline.

To cut a release (maintainer):

1. Bump `version` in `src-tauri/tauri.conf.json` (and `package.json` to match).
2. From `desktop/`, run:
   ```powershell
   powershell -File scripts/publish.ps1
   ```
   That builds a **signed** installer and publishes a GitHub release (installer +
   `latest.json`). Installed apps pick it up on their next launch.

**⚠️ The signing key is critical.** It lives at `~/.tauri/xddgaming_updater.key`
(never in the repo). **Back it up.** If it's lost, you can't sign updates and
auto-update breaks for everyone — recovering means shipping a new install with a
new public key. The matching public key is in `tauri.conf.json` → `plugins.updater.pubkey`.

## How it works

```
[hotkey] -> Rust: Ctrl+C, read item off clipboard, parse candidate names
         -> Rust: POST names to the wishlist-lookup Edge Function (token only)
         -> server resolves the league FROM the token, returns who wants it
         -> Rust: position + show the overlay, emit to the webview to render
[Found]  -> Rust: POST {finder, want} -> a finds row (same one the web 🔔 reads)
[toast]  -> Rust polls "notifications" for finds targeting you, while PoE runs
```

- **`src-tauri/src/main.rs`** — hotkey, clipboard copy + parse, the HTTP calls,
  window positioning, click-outside watch, tray, the notification poller, and
  settings persistence (`settings.json` in the app config dir).
- **`src/Overlay.tsx`** — the result panel (Found buttons, optional note).
- **`src/Settings.tsx`** — token onboarding + tabbed settings (auto-applying).
- **`src/Toast.tsx`** — the corner notification window.
- **`src/theme.css`** — mirrors the web app's gold/dark palette.

### Security
The base tables are never touched directly. The app authenticates with a
**per-league token** (kept in Rust, never in the webview); the server resolves the
league via the `league_tokens` table and returns only minimal fields
(display name, note, priority; who found what). Identity is client-declared (you
pick your name) — fine for a trusted private group. Backend details + the
token-rotation SQL live in the private xddGaming web-app repo.

### Notes / known iteration points
- Rare/magic items aren't matched (the group wishlists named items); the Edge
  Function owns matching, so that can improve without rebuilding the app.
