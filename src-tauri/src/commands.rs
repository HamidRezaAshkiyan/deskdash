use crate::marketplace;
use crate::models::{
    AppConfig, AppSettings, BarEdge, BarInstance, DdInvokeRequest, DdInvokeResponse,
    InstalledWidget, MarketplaceItem, MonitorInfo, PermissionAudit, Rect, SurfaceKind,
    WidgetInstance, WidgetManifest, WidgetPermissions,
};
use crate::permissions::{Capability, PermissionGate};
use crate::platform;
use crate::providers::network::ProxyFetchRequest;
use crate::state::AppState;
use crate::surfaces;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config.read().clone()
}

#[tauri::command]
pub fn get_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    surfaces::enumerate_monitors(&app)
}

#[tauri::command]
pub fn list_installed_widgets(state: State<'_, AppState>) -> Vec<InstalledWidget> {
    state.storage.list_installed_widgets()
}

#[tauri::command]
pub fn audit_widget_permissions(
    state: State<'_, AppState>,
    widget_id: String,
) -> Result<PermissionAudit, String> {
    let manifest = state.storage.get_widget_manifest(&widget_id)?;
    Ok(PermissionGate::audit(&manifest, None))
}

#[tauri::command]
pub fn add_widget_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    widget_id: String,
    monitor_id: String,
    granted_permissions: WidgetPermissions,
) -> Result<WidgetInstance, String> {
    let manifest = state.storage.get_widget_manifest(&widget_id)?;
    if manifest.kind != SurfaceKind::Widget {
        return Err(format!(
            "'{}' is a taskbar surface (kind: bar); use add_bar_instance instead",
            widget_id
        ));
    }

    let inst = {
        let mut cfg = state.config.write();
        let offset = (cfg.widgets.len() as f64 % 6.0) * 32.0;
        let next_z = cfg.widgets.iter().map(|w| w.z_index).max().unwrap_or(0) + 1;
        let inst = WidgetInstance {
            instance_id: format!("w-{}", Uuid::new_v4().simple()),
            widget_id: manifest.id.clone(),
            monitor_id,
            x: 80.0 + offset,
            y: 80.0 + offset,
            width: manifest.default_size.width,
            height: manifest.default_size.height,
            z_index: next_z,
            opacity: 1.0,
            locked: false,
            enabled: true,
            settings: manifest.default_settings_map(),
            granted_permissions,
        };
        cfg.widgets.push(inst.clone());
        inst
    };

    state.save()?;
    let _ = surfaces::sync_canvases(&app, &state);
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(inst)
}

#[tauri::command]
pub fn update_widget_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance: WidgetInstance,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        if let Some(existing) = cfg
            .widgets
            .iter_mut()
            .find(|w| w.instance_id == instance.instance_id)
        {
            *existing = instance;
        } else {
            return Err("Widget instance not found".into());
        }
    }
    state.save()?;
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(())
}

#[tauri::command]
pub fn remove_widget_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        cfg.widgets.retain(|w| w.instance_id != instance_id);
    }
    state.save()?;
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(())
}

#[tauri::command]
pub fn add_bar_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    widget_id: String,
    monitor_id: String,
    edge: Option<BarEdge>,
    hide_native_taskbar: Option<bool>,
    granted_permissions: WidgetPermissions,
) -> Result<BarInstance, String> {
    let manifest = state.storage.get_widget_manifest(&widget_id)?;
    if manifest.kind != SurfaceKind::Bar {
        return Err(format!("'{}' is not a taskbar (kind: bar)", widget_id));
    }
    let bar_cfg = manifest.bar.clone();
    let inst = BarInstance {
        instance_id: format!("b-{}", Uuid::new_v4().simple()),
        widget_id: manifest.id.clone(),
        monitor_id,
        edge: edge
            .or_else(|| bar_cfg.as_ref().map(|b| b.edge))
            .unwrap_or(BarEdge::Bottom),
        thickness: bar_cfg.as_ref().map(|b| b.thickness).unwrap_or(56.0),
        reserve_space: bar_cfg.as_ref().map(|b| b.reserve_space).unwrap_or(true),
        hide_native_taskbar: hide_native_taskbar
            .or_else(|| bar_cfg.as_ref().map(|b| b.hide_native_taskbar))
            .unwrap_or(false),
        enabled: true,
        settings: manifest.default_settings_map(),
        granted_permissions,
    };

    state.config.write().bars.push(inst.clone());
    state.save()?;
    surfaces::sync_bars(&app, &state)?;
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(inst)
}

#[tauri::command]
pub fn update_bar_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    bar: BarInstance,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        if let Some(existing) = cfg
            .bars
            .iter_mut()
            .find(|b| b.instance_id == bar.instance_id)
        {
            *existing = bar;
        } else {
            return Err("Bar instance not found".into());
        }
    }
    state.save()?;
    surfaces::sync_bars(&app, &state)?;
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(())
}

#[tauri::command]
pub fn remove_bar_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        cfg.bars.retain(|b| b.instance_id != instance_id);
    }
    state.save()?;
    surfaces::sync_bars(&app, &state)?;
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(())
}

#[tauri::command]
pub fn set_edit_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.config.write().edit_mode = enabled;
    state.save()?;
    surfaces::sync_canvases(&app, &state)?;
    let _ = app.emit("deskdash:edit-mode-changed", enabled);
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(())
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    let autostart = settings.autostart;
    state.config.write().settings = settings;
    state.save()?;
    let _ = platform::set_autostart(autostart);
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(())
}

#[tauri::command]
pub fn update_interactive_rects(window_label: String, rects: Vec<Rect>) {
    platform::update_interactive_rects(&window_label, rects);
}

#[tauri::command]
pub fn restore_native_taskbar(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        for bar in &mut cfg.bars {
            bar.hide_native_taskbar = false;
        }
    }
    state.save()?;
    surfaces::set_native_taskbar_with_lock(&state, true)?;
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(())
}

#[tauri::command]
pub fn open_data_dir(state: State<'_, AppState>) -> Result<String, String> {
    let path = state.storage.data_dir().to_string_lossy().to_string();
    Ok(path)
}

#[tauri::command]
pub fn open_canvas_preview(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    surfaces::sync_canvases(&app, &state)?;
    if let Some(win) = app.get_webview_window("canvas-primary") {
        let _ = win.show();
    }
    Ok(())
}

#[tauri::command]
pub async fn list_marketplace(state: State<'_, AppState>) -> Result<Vec<MarketplaceItem>, String> {
    let urls = state.config.read().settings.registry_urls.clone();
    Ok(marketplace::fetch_marketplace_catalog(&state.storage, &urls).await)
}

#[tauri::command]
pub async fn install_marketplace_widget(
    state: State<'_, AppState>,
    download_url: String,
    sha256: Option<String>,
) -> Result<WidgetManifest, String> {
    marketplace::install_from_url(&state.storage, &download_url, sha256.as_deref()).await
}

#[tauri::command]
pub fn import_local_widget(
    state: State<'_, AppState>,
    path: String,
) -> Result<WidgetManifest, String> {
    marketplace::install_from_local_path(&state.storage, &PathBuf::from(path))
}

#[tauri::command]
pub fn uninstall_widget(
    app: AppHandle,
    state: State<'_, AppState>,
    widget_id: String,
) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        cfg.widgets.retain(|w| w.widget_id != widget_id);
        cfg.bars.retain(|b| b.widget_id != widget_id);
    }
    state.storage.remove_widget(&widget_id)?;
    state.save()?;
    let _ = surfaces::sync_bars(&app, &state);
    let _ = app.emit("deskdash:config-updated", &*state.config.read());
    Ok(())
}

/// Unified permission-gated SDK bridge invoked by Canvas/Bar hosts on behalf of sandboxed widgets.
#[tauri::command]
pub async fn dd_invoke(
    state: State<'_, AppState>,
    req: DdInvokeRequest,
) -> Result<DdInvokeResponse, String> {
    let Some(perms) = state.get_instance_permissions(&req.instance_id) else {
        return Ok(DdInvokeResponse::err(format!(
            "Unknown widget instance '{}'",
            req.instance_id
        )));
    };

    let result: Result<serde_json::Value, String> = match req.method.as_str() {
        "widget.getConfig" => {
            let map = state
                .get_instance_settings(&req.instance_id)
                .unwrap_or_default();
            Ok(serde_json::to_value(map).unwrap_or_default())
        }
        "theme.get" => Ok(serde_json::to_value(platform::get_system_theme()).unwrap_or_default()),
        "storage.get" => {
            PermissionGate::require(&perms, Capability::Storage)?;
            let key = req
                .params
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'key'")?;
            let kv = state.storage.read_widget_kv(&req.instance_id)?;
            Ok(kv.get(key).cloned().unwrap_or(serde_json::Value::Null))
        }
        "storage.set" => {
            PermissionGate::require(&perms, Capability::Storage)?;
            let key = req
                .params
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'key'")?
                .to_string();
            let val = req
                .params
                .get("value")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let mut kv = state.storage.read_widget_kv(&req.instance_id)?;
            kv.insert(key, val);
            state.storage.write_widget_kv(&req.instance_id, &kv)?;
            Ok(serde_json::json!(true))
        }
        "storage.delete" => {
            PermissionGate::require(&perms, Capability::Storage)?;
            let key = req
                .params
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'key'")?;
            let mut kv = state.storage.read_widget_kv(&req.instance_id)?;
            kv.remove(key);
            state.storage.write_widget_kv(&req.instance_id, &kv)?;
            Ok(serde_json::json!(true))
        }
        "storage.list" => {
            PermissionGate::require(&perms, Capability::Storage)?;
            let kv: HashMap<String, serde_json::Value> =
                state.storage.read_widget_kv(&req.instance_id)?;
            Ok(serde_json::to_value(kv).unwrap_or_default())
        }
        "system.getStats" => {
            PermissionGate::require(&perms, Capability::System)?;
            let pause = state.config.read().settings.auto_pause_fullscreen;
            if pause && platform::is_foreground_fullscreen() {
                return Ok(DdInvokeResponse::success(serde_json::json!({ "paused": true })));
            }
            let stats = state.metrics.sample();
            Ok(serde_json::to_value(stats).unwrap_or_default())
        }
        "media.getSession" => {
            PermissionGate::require(&perms, Capability::Media)?;
            let session = platform::get_media_session();
            Ok(serde_json::to_value(session).unwrap_or_default())
        }
        "media.playPause" => {
            PermissionGate::require(&perms, Capability::Media)?;
            platform::media_control("playPause")?;
            Ok(serde_json::json!(true))
        }
        "media.next" => {
            PermissionGate::require(&perms, Capability::Media)?;
            platform::media_control("next")?;
            Ok(serde_json::json!(true))
        }
        "media.prev" => {
            PermissionGate::require(&perms, Capability::Media)?;
            platform::media_control("prev")?;
            Ok(serde_json::json!(true))
        }
        "network.fetch" => {
            let fetch_req: ProxyFetchRequest = serde_json::from_value(req.params)
                .map_err(|e| format!("Invalid network.fetch params: {}", e))?;
            let resp = state.network.fetch(&perms, fetch_req).await?;
            Ok(serde_json::to_value(resp).unwrap_or_default())
        }
        "clipboard.readText" => {
            PermissionGate::require(&perms, Capability::Clipboard)?;
            let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
            let text = cb.get_text().unwrap_or_default();
            Ok(serde_json::json!(text))
        }
        "clipboard.writeText" => {
            PermissionGate::require(&perms, Capability::Clipboard)?;
            let text = req
                .params
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
            cb.set_text(text.to_string()).map_err(|e| e.to_string())?;
            Ok(serde_json::json!(true))
        }
        "taskbar.listWindows" => {
            PermissionGate::require(&perms, Capability::Taskbar)?;
            let wins = platform::list_open_windows();
            Ok(serde_json::to_value(wins).unwrap_or_default())
        }
        "taskbar.focusWindow" => {
            PermissionGate::require(&perms, Capability::Taskbar)?;
            let id = req
                .params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("Missing window 'id'")?;
            platform::focus_window(id)?;
            Ok(serde_json::json!(true))
        }
        "taskbar.minimizeWindow" => {
            PermissionGate::require(&perms, Capability::Taskbar)?;
            let id = req
                .params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("Missing window 'id'")?;
            platform::minimize_window(id)?;
            Ok(serde_json::json!(true))
        }
        "taskbar.closeWindow" => {
            PermissionGate::require(&perms, Capability::Taskbar)?;
            let id = req
                .params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("Missing window 'id'")?;
            platform::close_window(id)?;
            Ok(serde_json::json!(true))
        }
        "taskbar.openStartMenu" => {
            PermissionGate::require(&perms, Capability::Taskbar)?;
            platform::open_start_menu()?;
            Ok(serde_json::json!(true))
        }
        "launcher.launchApp" => {
            PermissionGate::require(&perms, Capability::Launcher)?;
            let path = req
                .params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing app 'path'")?;
            let args: Vec<String> = req
                .params
                .get("args")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            platform::launch_app(path, &args)?;
            Ok(serde_json::json!(true))
        }
        other => Err(format!("Unsupported dd SDK method '{}'", other)),
    };

    match result {
        Ok(val) => Ok(DdInvokeResponse::success(val)),
        Err(msg) => Ok(DdInvokeResponse::err(msg)),
    }
}
