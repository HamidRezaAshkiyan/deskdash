# AGENTS.md — DeskDash

## Overview

**DeskDash** is a native Windows 10/11 desktop widget & alternative taskbar platform built with **Rust + Tauri v2 (WebView2) + Win32/WinRT FFI + React 19 / TypeScript / Tailwind 4**. It renders interactive HTML/CSS/JS widgets directly on the desktop layer behind normal windows, enforces a sandboxed `dd` JS SDK with per-widget permission approval, and supports `"kind": "bar"` alternative taskbars using Win32 `SHAppBarMessage` and `<dd-bar>` click-through island geometry.

## Tech Stack & Requirements

- **Backend (Rust, edition `2024`)**: Tauri `2` (`tray-icon`, `image-png`, `protocol-asset`), `tauri-plugin-opener`, `tauri-plugin-dialog`, `tauri-plugin-global-shortcut`, `tokio`, `sysinfo 0.33`, `reqwest 0.12`, `zip 2`, `arboard 3`, `parking_lot`, `windows 0.58` (Windows-only Win32/WinRT FFI).
- **Frontend**: React `19.1`, TypeScript `5.8`, Vite `6.3` (multi-entry: `index.html`, `canvas.html`, `bar.html`), Tailwind CSS `4.1`, Zustand `5`, `lucide-react`.
- **Target**: Windows 10/11 (`x86_64-pc-windows-msvc` NSIS `currentUser`); macOS/Linux stub platform included for local preview and automated tests.

## Repository Structure

```text
deskdash/
├── index.html / canvas.html / bar.html    # Multi-entry Tauri WebView surfaces
├── resources/widgets/                     # 7 built-in plain HTML/CSS/JS widgets & taskbars
│   ├── clock/                             # Glass Clock (12/24h + optional Jalali date)
│   ├── weather/                           # Live Weather (Open-Meteo allowlisted network)
│   ├── system-monitor/                    # CPU/RAM/Disk/Net sparklines
│   ├── media-player/                      # Windows SMTC playback controls
│   ├── sticky-notes/                      # Persistent notes via dd.widget.storage
│   ├── floating-dock/                     # Floating glass dock (kind: bar, <dd-bar>)
│   └── top-status-bar/                    # Top status rail (kind: bar, <dd-bar>)
├── scripts/
│   ├── dev.sh                             # macOS/Linux helper script
│   └── dev.ps1                            # Windows PowerShell 7 helper script
├── src/
│   ├── sdk/dd-sdk.js                      # Sandboxed widget SDK + <dd-bar> web component
│   ├── canvas/CanvasApp.tsx               # Per-monitor desktop canvas + Edit Mode
│   ├── bar/BarApp.tsx                     # Alternative taskbar host + hit-rect bridge
│   └── manager/                           # Manager & Marketplace UI (React + Zustand)
└── src-tauri/src/
    ├── lib.rs / main.rs                   # Tauri setup, dd-widget:// protocol, panic/recovery guards
    ├── models.rs                          # Manifest, permissions, layout & telemetry models
    ├── permissions.rs                     # Default-deny PermissionGate & host allowlist
    ├── protocol.rs                        # Path-safe dd-widget:// handler & CSP/SDK injection
    ├── storage.rs                         # Config & per-instance KV persistence
    ├── surfaces.rs                        # Multi-monitor Canvas & Bar window orchestrator
    ├── marketplace.rs                     # Registry catalog, SHA-256 & zip-slip safe installer
    ├── commands.rs                        # Tauri IPC commands + dd_invoke SDK bridge
    ├── tray.rs                            # System tray controls
    ├── providers/                         # System metrics (sysinfo) & allowlisted HTTP proxy
    └── platform/                          # windows.rs (Win32/WinRT) + stub.rs (macOS preview)
```

## Development & Verification Commands

```bash
# Type-check and build all 3 frontend surfaces
npx tsc --noEmit && npm run build

# Run Rust unit/integration tests
cd src-tauri && cargo test

# Verify Windows Win32/WinRT compilation from macOS
cd src-tauri && cargo check --target x86_64-pc-windows-gnu

# Launch Manager & preview mode
npm run tauri:dev
```

## Safety & Conventions

- **Taskbar Safety**: `platform::windows::set_native_taskbar_visible` is guarded by a panic hook, exit cleanup (`surfaces::cleanup_all_surfaces`), `.taskbar-hidden.lock` startup recovery, a tray recovery menu item, and `deskdash --restore-taskbar`.
- **Sandboxed Widgets**: Widgets are served from `dd-widget://` (or `http://dd-widget.localhost/` on Windows WebView2) with `connect-src 'none'` CSP so all network access flows through `dd.network.fetch` and `PermissionGate::require_network`.
