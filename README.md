# PoE Wishlist Overlay

**An in-game overlay for the xddGaming Path of Exile private-league wishlist.**
Hover an item in Path of Exile, tap a key, and instantly see **who in the group
wants it** — then mark it found so they know. Like Awakened PoE Trade, but the
answer is "who wants this," not a price.

```
┌─────────────────────────────────────┐
│  ● Rebuke of the Vaal          [x]  │
│  2 people want this                 │
├─────────────────────────────────────┤
│  (S) Snaze          [ HIGH ]  Found │
│      "not vaaled please"            │
│  (T) thecrow.       [ MED ]   Found │
└─────────────────────────────────────┘
```

---

## ⬇ Download

### [**Download for Windows →**](https://github.com/Snazethebaze/xddGaming-desktop/releases/latest)

Grab the `…-setup.exe` from the latest release and run it. Windows 10/11.

> It's not code-signed, so Windows SmartScreen may say *"unknown publisher"* —
> click **More info → Run anyway**. The app then **keeps itself updated** automatically.

---

## What it does

- **Look up any item** — hover it in PoE, press your hotkey, and a panel by your
  cursor shows everyone who wants it, their priority, and any notes.
- **Found it?** One click tells the owner — it shows up in the web board's
  notifications, attributed to you (add an optional note like "msg me on Discord").
- **Get pinged** — a corner toast (with optional sound) pops when someone finds
  one of *your* items.
- **Stays out of the way** — lives in the system tray, doesn't steal game focus,
  and closes the moment you click back into the game. Featherweight; it does
  nothing until you press the key.

## Setup (about 30 seconds)

1. **Install and run it** — a small setup window opens.
2. **Paste your league token** — copy it from the top bar of the wishlist website.
3. **Pick your name** from the list → done.

Now, in PoE (**Windowed** or **Windowed Fullscreen**), hover an item and press
**Alt+W** (you can change the hotkey in Settings).

> This is the companion app for a **private group's** wishlist board — you need a
> token from that site to use it.

## How it works

When you press the hotkey, the app uses Path of Exile's built-in "copy item"
(Ctrl+C), reads the item's name, and asks the wishlist server who wants it — then
draws the panel next to your cursor. **Found** reports and the notifications read
and write the *same* data as the website, so the desktop app and the site always
agree. It talks to **one** server only, and stores nothing beyond your local
settings (your token, name, and preferences).

## Privacy & security

- No accounts, no telemetry, no tracking. Fully open source — read every line here.
- It only contacts the wishlist's server, which returns just the minimum needed
  (who wants an item, their note and priority).
- Your token is stored locally on your PC and never leaves it except to that server.

---

## Built with

**[Tauri](https://tauri.app) (Rust) + React** — a tiny (~2 MB) native app, not an
Electron behemoth. Cross-platform toolkit; currently shipped for Windows.

Developer & release instructions live in **[MAINTAINING.md](MAINTAINING.md)**.
