# DeskDash for Windows

A native Windows 10/11 desktop customization platform built with **Rust + Tauri v2 (WebView2) + Win32/WinRT + React 19** — turning your desktop background into a live, interactive widget dashboard with custom alternative taskbars and a sandboxed HTML/CSS/JS SDK.

## Features

- **Live Desktop Wallpaper Widgets**: Transparent per-monitor canvases pinned behind normal windows with dynamic per-widget click-through so empty wallpaper clicks reach your desktop icons.
- **Interactive Edit Mode (`Ctrl+Alt+E`)**: Drag, resize, snap-to-grid, lock, and adjust widget opacity across multiple monitors.
- **Sandboxed `dd` SDK & Explicit Permissions**: Widgets are plain HTML/CSS/JS with zero build step. Every capability (`system`, `media`, `taskbar`, `storage`, `clipboard`, `launcher`) and network domain allowlist is reviewed and approved before running.
- **Custom Alternative Taskbars (`kind: "bar"`)**: Dockable floating docks and top status rails with `<dd-bar>` click-through islands, Win32 `SHAppBarMessage` work-area reservation, and crash-safe native taskbar hiding/restoration.
- **7 Built-in Starter Widgets & Taskbars**: Glass Clock (with optional Jalali date), Live Weather (Open-Meteo), System Monitor, Media Controls (Windows SMTC), Sticky Notes, Floating Glass Dock, and Top Status Rail.
- **Marketplace & Local Importer**: Browse JSON-backed registries with SHA-256 verification and zip-slip protection, or import local widget folders directly.

## Quick Start

### Windows (PowerShell 7)
```powershell
npm install
./scripts/dev.ps1 dev
```

### macOS (Preview Mode & Tests)
```bash
npm install
./scripts/dev.sh test
./scripts/dev.sh check-win
```

### Emergency Taskbar Recovery (Windows)
If you ever need to force-restore the native Windows taskbar:
```powershell
./scripts/dev.ps1 restore-taskbar
```
