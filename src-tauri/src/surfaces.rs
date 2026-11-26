use crate::models::{BarEdge, BarInstance, MonitorInfo};
use crate::platform;
use crate::state::AppState;
use std::fs;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub fn enumerate_monitors(app: &AppHandle) -> Vec<MonitorInfo> {
    let primary_name = app
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|m| m.name().cloned());

    match app.available_monitors() {
        Ok(monitors) if !monitors.is_empty() => monitors
            .into_iter()
            .enumerate()
            .map(|(idx, m)| {
                let name = m
                    .name()
                    .cloned()
                    .unwrap_or_else(|| format!("Monitor {}", idx + 1));
                let is_primary = primary_name
                    .as_deref()
                    .map(|p| p == name)
                    .unwrap_or(idx == 0);
                let pos = m.position();
                let size = m.size();
                MonitorInfo {
                    id: if is_primary {
                        "primary".to_string()
                    } else {
                        format!("monitor-{}", idx)
                    },
                    name,
                    x: pos.x,
                    y: pos.y,
                    width: size.width,
                    height: size.height,
                    scale_factor: m.scale_factor(),
                    is_primary,
                }
            })
            .collect(),
        _ => vec![MonitorInfo {
            id: "primary".into(),
            name: "Primary Display".into(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
            is_primary: true,
        }],
    }
}

#[cfg(windows)]
fn raw_hwnd(window: &WebviewWindow) -> Option<isize> {
    window.hwnd().ok().map(|h| h.0 as isize)
}

#[cfg(not(windows))]
fn raw_hwnd(_window: &WebviewWindow) -> Option<isize> {
    Some(0)
}

/// Ensures every active monitor with widgets (or the primary monitor) has a Desktop Canvas window.
pub fn sync_canvases(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let monitors = enumerate_monitors(app);
    let edit_mode = state.config.read().edit_mode;

    for mon in &monitors {
        let label = format!("canvas-{}", mon.id);
        if let Some(existing) = app.get_webview_window(&label) {
            platform::set_edit_mode_for_canvas(&label, edit_mode);
            let _ = existing.emit("deskdash:config-updated", &*state.config.read());
            continue;
        }

        let url = format!("canvas.html?monitorId={}", mon.id);
        let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
            .title("DeskDash Canvas")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .skip_taskbar(true)
            .resizable(false)
            .position(mon.x as f64 / mon.scale_factor, mon.y as f64 / mon.scale_factor)
            .inner_size(
                mon.width as f64 / mon.scale_factor,
                mon.height as f64 / mon.scale_factor,
            );

        if let Ok(win) = builder.build() {
            if let Some(hwnd) = raw_hwnd(&win) {
                let _ = platform::attach_desktop_canvas(
                    &label,
                    hwnd,
                    mon.x,
                    mon.y,
                    mon.width as i32,
                    mon.height as i32,
                );
                platform::set_edit_mode_for_canvas(&label, edit_mode);
            }
        }
    }
    Ok(())
}

/// Synchronizes all enabled `"kind": "bar"` alternative taskbar windows, AppBar reservations,
/// and native Windows taskbar visibility.
pub fn sync_bars(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let monitors = enumerate_monitors(app);
    let bars: Vec<BarInstance> = state.config.read().bars.clone();
    let enabled_labels: std::collections::HashSet<String> = bars
        .iter()
        .filter(|b| b.enabled)
        .map(|b| format!("bar-{}", b.instance_id))
        .collect();

    // Close removed or disabled bars and unregister their AppBars
    let active_keys: Vec<String> = state.active_appbars.lock().keys().cloned().collect();
    for label in active_keys {
        if !enabled_labels.contains(&label) {
            if let Some(hwnd) = state.active_appbars.lock().remove(&label) {
                platform::unregister_appbar(hwnd);
            }
            platform::unregister_hit_surface(&label);
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.close();
            }
        }
    }

    let mut wants_hide_native = false;

    for bar in &bars {
        if !bar.enabled {
            continue;
        }
        if bar.hide_native_taskbar {
            wants_hide_native = true;
        }
        let mon = monitors
            .iter()
            .find(|m| m.id == bar.monitor_id)
            .or_else(|| monitors.first());
        let Some(mon) = mon else { continue };

        let label = format!("bar-{}", bar.instance_id);
        let (bx, by, bw, bh) = compute_bar_geometry(mon, bar.edge, bar.thickness);

        if let Some(existing) = app.get_webview_window(&label) {
            let _ = existing.emit("deskdash:bar-updated", bar);
            continue;
        }

        let url = format!("bar.html?instanceId={}", bar.instance_id);
        let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
            .title("DeskDash Bar")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .position(bx / mon.scale_factor, by / mon.scale_factor)
            .inner_size(bw / mon.scale_factor, bh / mon.scale_factor);

        if let Ok(win) = builder.build() {
            if let Some(hwnd) = raw_hwnd(&win) {
                platform::register_bar_hit_surface(&label, hwnd, bx as i32, by as i32);
                if bar.reserve_space {
                    let _ = platform::register_appbar(
                        hwnd,
                        bar.edge,
                        (bar.thickness * mon.scale_factor) as i32,
                        mon.x,
                        mon.y,
                        mon.width as i32,
                        mon.height as i32,
                    );
                    state.active_appbars.lock().insert(label.clone(), hwnd);
                }
            }
        }
    }

    set_native_taskbar_with_lock(state, !wants_hide_native)?;
    Ok(())
}

pub fn set_native_taskbar_with_lock(state: &AppState, visible: bool) -> Result<(), String> {
    let lock = state.storage.taskbar_lock_path();
    if !visible {
        let _ = fs::write(&lock, "hidden");
        platform::set_native_taskbar_visible(false)?;
    } else {
        platform::set_native_taskbar_visible(true)?;
        if lock.exists() {
            let _ = fs::remove_file(&lock);
        }
    }
    Ok(())
}

/// Called on graceful exit or crash recovery so the user is never left without a taskbar.
pub fn cleanup_all_surfaces(state: &AppState) {
    let appbars: Vec<(String, isize)> = state.active_appbars.lock().drain().collect();
    for (label, hwnd) in appbars {
        platform::unregister_appbar(hwnd);
        platform::unregister_hit_surface(&label);
    }
    let _ = set_native_taskbar_with_lock(state, true);
}

fn compute_bar_geometry(mon: &MonitorInfo, edge: BarEdge, thickness: f64) -> (f64, f64, f64, f64) {
    let t = (thickness * mon.scale_factor).max(32.0);
    let mx = mon.x as f64;
    let my = mon.y as f64;
    let mw = mon.width as f64;
    let mh = mon.height as f64;
    match edge {
        BarEdge::Top => (mx, my, mw, t),
        BarEdge::Bottom => (mx, my + mh - t, mw, t),
        BarEdge::Left => (mx, my, t, mh),
        BarEdge::Right => (mx + mw - t, my, t, mh),
    }
}
