# Game Focus Proxy (Drop-in Hook for Windows)

A zero-overhead, drop-in proxy DLL for Windows games that keeps windowed games fully active, retains controller/gamepad responsiveness, and prevents audio muting when you switch focus or multitask.

---

## What It Solves

- **Window Focus Spoofing**: Intercepts `GetForegroundWindow`, `GetActiveWindow`, `GetFocus`, and rewrites `WM_ACTIVATE` / `WM_KILLFOCUS` so the game engine believes it is always the focused foreground window.
- **Audio Stays Active**: Bypasses the internal "mute on lost focus" routines built into Unreal, Unity, and custom engines.
- **Background Gamepad Input**:
  - **XInput (Xbox controllers)**: Continues reading inputs because the game never enters its paused background polling state.
  - **DirectInput8**: Intercepts `SetCooperativeLevel` and replaces `DISCL_FOREGROUND` with `DISCL_BACKGROUND`.
  - **SDL2 / SDL3**: Automatically applies `SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1`.
- **Optional Cursor Freedom**: Intercepts `ClipCursor` so the mouse cursor is never trapped inside the window when you want to move to another monitor.

---

## Quick Start (How to Test on Your Gaming PC)

1. **Locate your game folder**:
   - For a Steam game, right click game -> **Manage** -> **Browse local files**.
   - Find the folder containing the main game `.exe` (for Unreal Engine games, look in `GameName/Binaries/Win64/`).

2. **Copy the files**:
   - Copy `version.dll` (from `target/x86_64-pc-windows-gnu/release/version.dll`) directly into the folder next to the game executable.
   - (Optional) Copy `focus_hook.ini` next to it to customize behavior. If omitted, default settings are automatically generated.

3. **Launch the game in Windowed or Borderless Windowed mode**.

4. **Test**:
   - Alt-Tab or click onto your second monitor/browser.
   - Sound should continue playing.
   - Moving your gamepad thumbsticks or pressing buttons should still control the game in real-time.

5. **Uninstall / Disable**:
   - Simply delete or rename `version.dll`.

---

## Configuration (`focus_hook.ini`)

```ini
[Settings]
; Master toggle (false disables all hooks)
enabled = true

; Spoof foreground focus so the engine thinks it's active
spoof_focus = true

; Prevent game from muting audio in background
keep_audio = true

; Force DirectInput/XInput/SDL controllers to work in background
background_controller = true

; Release mouse cursor bounds so it can leave the window freely
unlock_cursor = false

; Write debug events to focus_hook.log
log_debug = true
```

---

## Build from Source

On Linux (cross-compile) or Windows (native):
```bash
cargo build --release --target x86_64-pc-windows-gnu
```
The compiled DLL is output to:
`target/x86_64-pc-windows-gnu/release/version.dll`
