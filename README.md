# Game Focus Manager (Windows)

A lightweight, zero-overhead desktop utility and drop-in proxy hook that keeps windowed games fully active in the background when multitasking, prevents audio muting, keeps controllers/gamepads responsive, and optionally frees your mouse cursor across monitors.

---

## Features

- **Automatic Multi-Drive Scanning**:
  - **Steam**: Reads `libraryfolders.vdf` and `.acf` manifests across all installed drives (C:, D:, secondary SSDs/HDDs) and locates game executables automatically.
  - **Epic Games Launcher**: Scans all `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests\*.item` files.
  - **Custom Games**: Click **"➕ Add Custom Game"** to select any standalone `.exe` (GOG, emulators, Xbox app, standalone games).
- **One-Click Deploy & Clean Removal**:
  - Click **"Enable"** on any game: the manager drops `version.dll` and generates `focus_hook.ini` directly into the game folder.
  - Click **"Disable"**: cleanly removes `version.dll`.
- **System Tray Integration**:
  - Minimizing or closing the window sends it to the Windows system tray with quick options: *Show*, *Rescan*, and *Quit*.
- **In-Game Hooks**:
  - **Focus Spoofing**: Intercepts `GetForegroundWindow`, `GetActiveWindow`, `GetFocus`, and rewrites `WM_ACTIVATE` / `WM_KILLFOCUS` so game engines believe they are always the focused foreground window.
  - **Audio in Background**: Bypasses the "mute when lost focus" routine.
  - **Controller Input**: XInput games stay active, DirectInput8 cooperative levels are set to background (`DISCL_BACKGROUND`), and SDL2/3 environment variables (`SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1`) are injected.
  - **Unlock Mouse Cursor**: Intercepts `ClipCursor` so the mouse cursor is never trapped inside the game window when you want to move across multiple monitors.

---

## Quick Start

1. Copy the `dist/` folder or `game-focus-manager.exe` to your gaming PC.
2. Launch `game-focus-manager.exe`.
3. It will automatically scan your Steam libraries across all drives and Epic Games.
4. Click **"Enable"** next to any game you want to play in the background.
5. Launch the game in **Windowed** or **Borderless Windowed** mode.

---

## Architecture & Project Structure

```text
game-focus/
├── crates/
│   ├── proxy/       # The in-game version.dll proxy hook (MinHook detours)
│   ├── scanner/     # Multi-drive Steam VDF & Epic manifest discovery engine
│   └── manager/     # Native DirectX/Win32 egui GUI with system tray
└── dist/            # Compiled ready-to-run Windows binaries
```

### Building from Source

Using Rust and the `x86_64-pc-windows-gnu` target:
```bash
# 1. Compile proxy DLL
cargo build --release -p game-focus-proxy --target x86_64-pc-windows-gnu

# 2. Compile desktop manager (embeds the compiled proxy DLL)
cargo build --release -p game-focus-manager --target x86_64-pc-windows-gnu
```
