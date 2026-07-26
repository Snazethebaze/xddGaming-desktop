// Prevent a console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// ============================================================================
//  PoE Wishlist Overlay (Tauri) — the desktop client.
//
//  Flow: global hotkey -> copy the hovered item (Ctrl+C) -> parse candidate
//  names -> POST them to our Supabase Edge Function (wishlist-lookup) with the
//  shared group token -> position a transparent overlay near the cursor and
//  emit the matches to the webview to render.
//
//  All secrets/logic stay here (Rust): the token never reaches the webview, and
//  the app talks ONLY to the Edge Function (read-only, minimal projection).
// ============================================================================

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Mouse, Settings as EnigoSettings};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

// Non-secret backend coordinates (publishable key is safe to ship).
const ENDPOINT: &str = "https://zizbfhyhjcpslblfyyvv.supabase.co/functions/v1/wishlist-lookup";
const APIKEY: &str = "sb_publishable_LigWktHU_OfOLYxRM3LWqQ_uli0-Z4J";

const DEFAULT_HOTKEY: &str = "Alt+W";

// Overlay window size (keep in sync with tauri.conf.json) for cursor clamping.
const WIN_W: i32 = 360;
const WIN_H: i32 = 280;
// Toast window size (keep in sync with tauri.conf.json) for corner placement.
const TOAST_W: i32 = 340;
const TOAST_H: i32 = 380;

fn default_true() -> bool {
    true
}
fn default_corner() -> String {
    "bottom-right".to_string()
}
fn default_poll() -> u32 {
    25
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct Settings {
    token: String, // the token IS the league; the server resolves the league from it
    hotkey: String,
    #[serde(default)]
    identity_id: String, // which group member "you" are (for Found + notifications)
    #[serde(default)]
    identity_name: String,
    #[serde(default = "default_true")]
    toast_enabled: bool,
    #[serde(default = "default_true")]
    toast_sound: bool,
    #[serde(default = "default_corner")]
    toast_corner: String, // top-left | top-right | bottom-left | bottom-right
    #[serde(default = "default_poll")]
    poll_secs: u32,
}

#[derive(Clone, Serialize)]
struct LookupPayload {
    state: String,   // "pending" | "result" | "empty" | "error"
    item: String,    // the item name we looked up (header), when known
    me: String,      // my identity_id, so the overlay hides "Found" on my own rows
    matches: serde_json::Value,
    message: String,
}

#[derive(Serialize)]
struct Person {
    id: String,
    display_name: String,
}

#[derive(Serialize)]
struct PeopleResult {
    league: String,
    people: Vec<Person>,
}

// ---------------------------------------------------------------------------
// Settings persistence (app config dir / settings.json)
// ---------------------------------------------------------------------------
fn config_file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

fn load_settings(app: &AppHandle) -> Settings {
    if let Some(p) = config_file(app) {
        if let Ok(raw) = std::fs::read_to_string(&p) {
            if let Ok(mut s) = serde_json::from_str::<Settings>(&raw) {
                if s.hotkey.trim().is_empty() {
                    s.hotkey = DEFAULT_HOTKEY.to_string();
                }
                return s;
            }
        }
    }
    Settings {
        token: String::new(),
        hotkey: DEFAULT_HOTKEY.to_string(),
        identity_id: String::new(),
        identity_name: String::new(),
        toast_enabled: true,
        toast_sound: true,
        toast_corner: default_corner(),
        poll_secs: default_poll(),
    }
}

fn write_settings(app: &AppHandle, s: &Settings) -> Result<(), String> {
    let p = config_file(app).ok_or("no config dir")?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(&p, json).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Hotkey (re)registration
// ---------------------------------------------------------------------------
fn register_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.register(hotkey)
        .map_err(|e| format!("Couldn't register '{hotkey}': {e}"))
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------
fn show_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn position_overlay(app: &AppHandle, mx: i32, my: i32) {
    if let Some(w) = app.get_webview_window("overlay") {
        let (ww, wh) = match w.outer_size() {
            Ok(sz) if sz.width > 0 => (sz.width as i32, sz.height as i32),
            _ => (WIN_W, WIN_H),
        };
        let (mut x, mut y) = (mx + 18, my + 18);
        if let Ok(Some(mon)) = w.current_monitor() {
            let size = mon.size();
            let pos = mon.position();
            let right = pos.x + size.width as i32;
            let bottom = pos.y + size.height as i32;
            if x + ww > right {
                x = mx - ww - 18;
            }
            if y + wh > bottom {
                y = bottom - wh - 8;
            }
            if x < pos.x {
                x = pos.x + 8;
            }
            if y < pos.y {
                y = pos.y + 8;
            }
        }
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
}

fn emit_overlay(
    app: &AppHandle,
    state: &str,
    item: &str,
    me: &str,
    matches: serde_json::Value,
    message: &str,
) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.emit(
            "lookup",
            LookupPayload {
                state: state.to_string(),
                item: item.to_string(),
                me: me.to_string(),
                matches,
                message: message.to_string(),
            },
        );
    }
}

fn show_overlay(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.show();
        let _ = w.set_always_on_top(true);
    }
}

fn position_toast(app: &AppHandle, corner: &str) {
    let Some(w) = app.get_webview_window("toast") else {
        return;
    };
    // Use the window's real PHYSICAL size (logical * scale factor) so corner
    // placement is correct on HiDPI/scaled displays.
    let (ww, wh) = match w.outer_size() {
        Ok(sz) if sz.width > 0 => (sz.width as i32, sz.height as i32),
        _ => (TOAST_W, TOAST_H),
    };
    let (mx, my, mw, mh) = if let Ok(Some(mon)) = app.primary_monitor() {
        let s = mon.size();
        let p = mon.position();
        (p.x, p.y, s.width as i32, s.height as i32)
    } else {
        (0, 0, 1920, 1080)
    };
    let m = 16;
    let (x, y) = match corner {
        "top-left" => (mx + m, my + m),
        "top-right" => (mx + mw - ww - m, my + m),
        "bottom-left" => (mx + m, my + mh - wh - m),
        _ => (mx + mw - ww - m, my + mh - wh - m), // bottom-right
    };
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

// Play the Windows "asterisk" notification chime. Native (not webview) so it's
// reliable — WebView2 blocks audio without a user gesture. Linked from user32
// directly (this windows-sys build doesn't expose MessageBeep).
#[cfg(windows)]
fn play_ding() {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBeep(utype: u32) -> i32;
    }
    const MB_ICONASTERISK: u32 = 0x0000_0040;
    unsafe {
        MessageBeep(MB_ICONASTERISK);
    }
}
#[cfg(not(windows))]
fn play_ding() {}

fn show_toast(app: &AppHandle, corner: &str, sound: bool, finds: Vec<serde_json::Value>) {
    if let Some(w) = app.get_webview_window("toast") {
        position_toast(app, corner);
        let _ = w.emit("notify", serde_json::json!({ "corner": corner, "finds": finds }));
        let _ = w.show();
        let _ = w.set_always_on_top(true);
        if sound {
            play_ding();
        }
    }
}

// True while Path of Exile has a window open (running), regardless of focus.
#[cfg(windows)]
fn poe_running() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
    let class: Vec<u16> = "POEWindowClass\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
    !hwnd.is_null()
}
#[cfg(not(windows))]
fn poe_running() -> bool {
    true
}

// Close the overlay when the user clicks anywhere outside it — without stealing
// game focus. Polls the mouse only while the overlay is actually visible.
fn start_dismiss_watch(app: AppHandle) {
    #[cfg(windows)]
    std::thread::spawn(move || {
        #[repr(C)]
        struct Point {
            x: i32,
            y: i32,
        }
        #[link(name = "user32")]
        extern "system" {
            fn GetAsyncKeyState(vkey: i32) -> i16;
            fn GetCursorPos(p: *mut Point) -> i32;
        }
        const VK_LBUTTON: i32 = 0x01;
        const VK_RBUTTON: i32 = 0x02;
        loop {
            let Some(w) = app.get_webview_window("overlay") else {
                std::thread::sleep(Duration::from_millis(300));
                continue;
            };
            if w.is_visible().unwrap_or(false) {
                let down = unsafe {
                    (GetAsyncKeyState(VK_LBUTTON) as u16 & 0x8000) != 0
                        || (GetAsyncKeyState(VK_RBUTTON) as u16 & 0x8000) != 0
                };
                if down {
                    let mut pt = Point { x: 0, y: 0 };
                    if unsafe { GetCursorPos(&mut pt) } != 0 {
                        if let (Ok(pos), Ok(sz)) = (w.outer_position(), w.outer_size()) {
                            let inside = pt.x >= pos.x
                                && pt.x <= pos.x + sz.width as i32
                                && pt.y >= pos.y
                                && pt.y <= pos.y + sz.height as i32;
                            if !inside {
                                let _ = w.hide();
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(35));
            } else {
                std::thread::sleep(Duration::from_millis(250));
            }
        }
    });
    #[cfg(not(windows))]
    let _ = app;
}

// Poll the Edge Function for "finds" targeting my identity and toast the new ones.
// ONLY polls while PoE is running (idle usage ~= 0). Re-seeds at the start of each
// play session and on identity change, so we toast only finds that happen *during*
// the session — no burst of old ones. The website's bell stays authoritative; we
// never acknowledge here.
fn start_notifications_poll(app: AppHandle) {
    std::thread::spawn(move || {
        let mut seen: HashSet<String> = HashSet::new();
        let mut seeded_for = String::new();
        let mut was_running = false;
        loop {
            let s = load_settings(&app);
            let secs = s.poll_secs.clamp(10, 300);
            let running = poe_running();

            if running
                && s.toast_enabled
                && !s.identity_id.trim().is_empty()
                && !s.token.trim().is_empty()
            {
                let body = serde_json::json!({ "action": "notifications", "owner_id": s.identity_id });
                if let Ok(v) = call_fn(&s.token, body) {
                    if let Some(finds) = v.get("finds").and_then(|f| f.as_array()) {
                        // Seed (no toast) when a session starts or identity changes.
                        let seeding = !was_running || seeded_for != s.identity_id;
                        if seeding {
                            seen.clear();
                        }
                        let mut fresh = Vec::new();
                        for f in finds {
                            if let Some(id) = f.get("id").and_then(|i| i.as_str()) {
                                if seen.insert(id.to_string()) && !seeding {
                                    fresh.push(f.clone());
                                }
                            }
                        }
                        seeded_for = s.identity_id.clone();
                        if !fresh.is_empty() {
                            show_toast(&app, &s.toast_corner, s.toast_sound, fresh);
                        }
                    }
                }
            }

            was_running = running;
            std::thread::sleep(Duration::from_secs(secs as u64));
        }
    });
}

// ---------------------------------------------------------------------------
// Item copy + parse
// ---------------------------------------------------------------------------
fn copy_hovered_item(enigo: &mut Enigo) -> String {
    let mut cb = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let prev = cb.get_text().ok();
    let _ = cb.set_text(String::new()); // clear so we can detect the game's copy

    let _ = enigo.key(Key::Control, Direction::Press);
    let _ = enigo.key(Key::Unicode('c'), Direction::Click);
    let _ = enigo.key(Key::Control, Direction::Release);

    let mut text = String::new();
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(40));
        if let Ok(t) = cb.get_text() {
            if !t.trim().is_empty() {
                text = t;
                break;
            }
        }
    }

    if let Some(p) = prev {
        let _ = cb.set_text(p); // restore the user's clipboard
    }
    text
}

fn push_unique(out: &mut Vec<String>, v: &str) {
    let v = v.trim();
    if v.is_empty() || out.iter().any(|e| e.eq_ignore_ascii_case(v)) {
        return;
    }
    out.push(v.to_string());
}

// PoE Ctrl+C format: "Item Class: X" / "Rarity: Y" / <name> / <base?> / "----".
// Send name, base, and "name base"; the Edge Function owns the actual matching.
fn parse_candidates(text: &str) -> Vec<String> {
    let cleaned = text.replace('\r', "");
    let lines: Vec<&str> = cleaned.lines().collect();

    let mut r_idx = None;
    for (i, l) in lines.iter().enumerate() {
        if l.trim_start().starts_with("Rarity:") {
            r_idx = Some(i);
            break;
        }
    }
    let Some(ri) = r_idx else {
        return Vec::new();
    };

    let name = lines.get(ri + 1).map(|s| s.trim().to_string()).unwrap_or_default();
    let mut base = String::new();
    if let Some(nb) = lines.get(ri + 2) {
        let nb = nb.trim();
        if !nb.is_empty() && !nb.starts_with("---") {
            base = nb.to_string();
        }
    }

    let mut out = Vec::new();
    push_unique(&mut out, &name);
    if !base.is_empty() {
        push_unique(&mut out, &base);
        push_unique(&mut out, &format!("{name} {base}"));
    }
    out
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------
// One entry point to the Edge Function for every action (lookup/people/found).
// Returns the parsed JSON body; the caller reads matches/people/ok/code from it.
fn call_fn(token: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(ENDPOINT)
        .header("apikey", APIKEY)
        .header("x-wishlist-token", token)
        .json(&body)
        .send()
        .map_err(|_| "Can't reach the server (offline?)".to_string())?;

    let status = resp.status().as_u16();
    if status == 401 {
        return Err("Unauthorized — check your token in Settings.".to_string());
    }
    if status >= 500 {
        return Err(format!("Server error ({status})"));
    }
    // 2xx and 4xx bodies both carry useful JSON (ok / code / error).
    resp.json::<serde_json::Value>()
        .map_err(|_| "Bad response from server".to_string())
}

// ---------------------------------------------------------------------------
// Main lookup flow (runs on a background thread)
// ---------------------------------------------------------------------------
fn do_lookup(app: &AppHandle) {
    let settings = load_settings(app);
    if settings.token.trim().is_empty() {
        show_settings(app);
        return;
    }

    let me = settings.identity_id.clone();

    let mut enigo = match Enigo::new(&EnigoSettings::default()) {
        Ok(e) => e,
        Err(_) => return,
    };
    let (mx, my) = enigo.location().unwrap_or((200, 200));

    // Show the panel immediately with a loading state near the cursor.
    position_overlay(app, mx, my);
    emit_overlay(app, "pending", "", &me, serde_json::json!([]), "");
    show_overlay(app);

    let text = copy_hovered_item(&mut enigo);
    let names = parse_candidates(&text);
    if names.is_empty() {
        emit_overlay(
            app,
            "empty",
            "",
            &me,
            serde_json::json!([]),
            "Hover an item, then press your hotkey.",
        );
        return;
    }
    let item = names.first().cloned().unwrap_or_default();

    let body = serde_json::json!({ "action": "lookup", "names": names });
    match call_fn(&settings.token, body) {
        Ok(v) => {
            let matches = v.get("matches").cloned().unwrap_or_else(|| serde_json::json!([]));
            let empty = matches.as_array().map(|a| a.is_empty()).unwrap_or(true);
            if empty {
                emit_overlay(app, "empty", &item, &me, serde_json::json!([]), "");
            } else {
                emit_overlay(app, "result", &item, &me, matches, "");
            }
        }
        Err(e) => emit_overlay(app, "error", &item, &me, serde_json::json!([]), &e),
    }
}

// ---------------------------------------------------------------------------
// Commands (called from the settings/overlay webviews)
// ---------------------------------------------------------------------------
#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    load_settings(&app)
}

// Auto-applying settings write (no Save button in the UI). Persists everything and
// re-registers the hotkey best-effort (an invalid hotkey won't block saving the
// rest). The league is NOT stored — it's derived from the token by the server.
#[tauri::command]
fn update_settings(
    app: AppHandle,
    token: String,
    hotkey: String,
    identity_id: String,
    identity_name: String,
    toast_enabled: bool,
    toast_sound: bool,
    toast_corner: String,
    poll_secs: u32,
) -> Result<(), String> {
    let hotkey = hotkey.trim();
    let s = Settings {
        token: token.trim().to_string(),
        hotkey: if hotkey.is_empty() { DEFAULT_HOTKEY.to_string() } else { hotkey.to_string() },
        identity_id: identity_id.trim().to_string(),
        identity_name: identity_name.trim().to_string(),
        toast_enabled,
        toast_sound,
        toast_corner: if toast_corner.trim().is_empty() { default_corner() } else { toast_corner.trim().to_string() },
        poll_secs: poll_secs.clamp(10, 300),
    };
    write_settings(&app, &s)?;
    let _ = register_hotkey(&app, &s.hotkey);
    Ok(())
}

// Connect: validate the token and return its league + the players on that board
// (for the identity picker). Errors if the token is invalid.
#[tauri::command]
fn list_people(token: String) -> Result<PeopleResult, String> {
    let v = call_fn(&token, serde_json::json!({ "action": "people" }))?;
    let league = v.get("league").and_then(|l| l.as_str()).unwrap_or("").to_string();
    let arr = v.get("people").and_then(|p| p.as_array()).cloned().unwrap_or_default();
    let people = arr
        .into_iter()
        .filter_map(|p| {
            Some(Person {
                id: p.get("id")?.as_str()?.to_string(),
                display_name: p.get("display_name")?.as_str()?.to_string(),
            })
        })
        .collect();
    Ok(PeopleResult { league, people })
}

// Report that you found a want. Returns "ok" or "duplicate"; Err carries a
// user-facing message (self-find, offline, etc.).
#[tauri::command]
fn mark_found(app: AppHandle, want_id: String, note: Option<String>) -> Result<String, String> {
    let s = load_settings(&app);
    if s.identity_id.trim().is_empty() {
        return Err("Pick your name in Settings first.".to_string());
    }
    let mut body = serde_json::json!({
        "action": "found",
        "finder_id": s.identity_id,
        "want_id": want_id,
    });
    if let Some(n) = note {
        if !n.trim().is_empty() {
            body["note"] = serde_json::json!(n.trim());
        }
    }
    let v = call_fn(&s.token, body)?;
    if v.get("ok").and_then(|b| b.as_bool()).unwrap_or(false) {
        return Ok("ok".to_string());
    }
    match v.get("code").and_then(|c| c.as_str()).unwrap_or("error") {
        "duplicate" => Ok("duplicate".to_string()),
        "self" => Err("That's your own want.".to_string()),
        _ => Err(v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("Couldn't send that.")
            .to_string()),
    }
}

#[tauri::command]
fn close_overlay(app: AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.hide();
    }
}

// Give the overlay keyboard focus so the optional-note field can be typed into
// (it's shown non-activating, so it needs an explicit focus to receive typing).
#[tauri::command]
fn focus_overlay(app: AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.set_focus();
    }
}

#[tauri::command]
fn hide_toast(app: AppHandle) {
    if let Some(w) = app.get_webview_window("toast") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    show_settings(&app);
}

// ---------------------------------------------------------------------------
// Tray
// ---------------------------------------------------------------------------
// Auto-update: check the GitHub release manifest, and if a newer signed build is
// available, download + install it and relaunch. Silently no-ops if there's no
// update or the endpoint is unreachable (e.g. before the first release exists).
async fn try_update(app: AppHandle) {
    use tauri_plugin_updater::UpdaterExt;
    let updater = match app.updater() {
        Ok(u) => u,
        Err(_) => return,
    };
    if let Ok(Some(update)) = updater.check().await {
        if update
            .download_and_install(|_chunk, _total| {}, || {})
            .await
            .is_ok()
        {
            app.restart();
        }
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("PoE Wishlist Overlay")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let handle = app.clone();
                        std::thread::spawn(move || do_lookup(&handle));
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            list_people,
            mark_found,
            close_overlay,
            focus_overlay,
            hide_toast,
            open_settings
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            build_tray(&handle)?;

            let s = load_settings(&handle);
            if s.token.trim().is_empty() {
                show_settings(&handle);
            } else if let Err(e) = register_hotkey(&handle, &s.hotkey) {
                eprintln!("hotkey error: {e}");
                show_settings(&handle);
            }

            start_notifications_poll(handle.clone());
            start_dismiss_watch(handle.clone());

            // Check GitHub for a newer signed release on launch; install + relaunch.
            let updater_handle = handle.clone();
            tauri::async_runtime::spawn(async move { try_update(updater_handle).await });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing a window should never quit the app (tray → Quit does that).
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
