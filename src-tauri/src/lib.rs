mod commands;
mod marketplace;
mod models;
mod permissions;
mod platform;
mod protocol;
mod providers;
mod state;
mod storage;
mod surfaces;
mod tray;

use crate::models::{WidgetInstance, WidgetPermissions};
use crate::state::AppState;
use crate::storage::StorageManager;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager, WindowEvent};
use uuid::Uuid;

fn resolve_bundled_widgets_dir(app: &tauri::App) -> Option<PathBuf> {
    if let Ok(res_dir) = app.path().resource_dir() {
        let candidate = res_dir.join("resources").join("widgets");
        if candidate.is_dir() {
            return Some(candidate);
        }
        let alt = res_dir.join("widgets");
        if alt.is_dir() {
            return Some(alt);
        }
    }
    // Dev fallback relative to repo root
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("resources").join("widgets"));
    dev_path.filter(|p| p.is_dir())
}

fn seed_default_layout_if_empty(state: &AppState) {
    let should_seed = {
        let cfg = state.config.read();
        cfg.widgets.is_empty() && cfg.bars.is_empty()
    };
    if !should_seed {
        return;
    }

    let starters = [
        ("clock", 64.0, 64.0, 320.0, 150.0, WidgetPermissions::default()),
        (
            "system-monitor",
            64.0,
            236.0,
            320.0,
            220.0,
            WidgetPermissions {
                system: true,
                ..Default::default()
            },
        ),
        (
            "sticky-notes",
            410.0,
            64.0,
            280.0,
            240.0,
            WidgetPermissions {
                storage: true,
                ..Default::default()
            },
        ),
    ];

    let mut cfg = state.config.write();
    for (idx, (id, x, y, w, h, perms)) in starters.into_iter().enumerate() {
        if let Ok(manifest) = state.storage.get_widget_manifest(id) {
            cfg.widgets.push(WidgetInstance {
                instance_id: format!("starter-{}", Uuid::new_v4().simple()),
                widget_id: manifest.id.clone(),
                monitor_id: "primary".into(),
                x,
                y,
                width: w,
                height: h,
                z_index: (idx + 1) as i32,
                opacity: 1.0,
                locked: false,
                enabled: true,
                settings: manifest.default_settings_map(),
                granted_permissions: perms,
            });
        }
    }
    drop(cfg);
    let _ = state.save();
}

pub fn run() {
    // Recovery CLI flag: restore Windows taskbar immediately and exit
    if std::env::args().any(|a| a == "--restore-taskbar") {
        let _ = platform::set_native_taskbar_visible(true);
        return;
    }

    // Panic hook guarantees the native Windows taskbar is never left hidden
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = platform::set_native_taskbar_visible(true);
        prev_hook(info);
    }));

    let storage_holder: Arc<parking_lot::RwLock<Option<Arc<StorageManager>>>> =
        Arc::new(parking_lot::RwLock::new(None));
    let protocol_storage = Arc::clone(&storage_holder);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .register_uri_scheme_protocol("dd-widget", move |_ctx, request| {
            let uri = request.uri();
            let host = uri.host().unwrap_or("");
            let raw_path = uri.path();

            // On Windows WebView2, custom schemes arrive as http://dd-widget.localhost/<widget_id>/<path>
            // On macOS/Linux, they arrive as dd-widget://<widget_id>/<path>
            let (widget_id, path): (&str, &str) = if host == "dd-widget.localhost" {
                let trimmed = raw_path.trim_start_matches('/');
                if trimmed == "__dd__/sdk.js" {
                    ("", "/__dd__/sdk.js")
                } else if let Some((id, rest)) = trimmed.split_once('/') {
                    (id, rest)
                } else {
                    (trimmed, "")
                }
            } else {
                (host, raw_path)
            };

            let guard = protocol_storage.read();
            let payload = if let Some(ref storage) = *guard {
                protocol::resolve_widget_asset(storage, widget_id, path)
            } else {
                protocol::ProtocolPayload {
                    status: 503,
                    content_type: "text/plain; charset=utf-8".into(),
                    body: b"Storage initializing".to_vec(),
                }
            };

            tauri::http::Response::builder()
                .status(payload.status)
                .header("Content-Type", payload.content_type)
                .header("Content-Security-Policy", protocol::WIDGET_CSP)
                .header("Access-Control-Allow-Origin", "*")
                .body(payload.body)
                .unwrap()
        })
        .setup(move |app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app_data_dir");
            let bundled = resolve_bundled_widgets_dir(app);
            let storage = StorageManager::new(data_dir, bundled)
                .expect("Failed to initialize DeskDash storage");

            // Crash recovery: if a previous run left the taskbar lockfile, restore taskbar now
            if storage.taskbar_lock_path().exists() {
                let _ = platform::set_native_taskbar_visible(true);
                let _ = std::fs::remove_file(storage.taskbar_lock_path());
            }

            let state = AppState::new(storage);
            *storage_holder.write() = Some(Arc::clone(&state.storage));
            seed_default_layout_if_empty(&state);

            app.manage(state);
            let _ = tray::setup_tray(app.handle());

            // On Windows, auto-spawn Desktop Canvases and Custom Bars on startup
            #[cfg(windows)]
            {
                let st = app.state::<AppState>();
                let _ = surfaces::sync_canvases(app.handle(), &st);
                let _ = surfaces::sync_bars(app.handle(), &st);
            }

            // Register Ctrl+Alt+E global shortcut to toggle Edit Mode
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
                let handle = app.handle().clone();
                let _ = app
                    .global_shortcut()
                    .on_shortcut("Ctrl+Alt+E", move |_app, _shortcut, event| {
                        if event.state() == ShortcutState::Pressed {
                            let st = handle.state::<AppState>();
                            let next = !st.config.read().edit_mode;
                            st.config.write().edit_mode = next;
                            let _ = st.save();
                            let _ = surfaces::sync_canvases(&handle, &st);
                            let _ = handle.emit("deskdash:edit-mode-changed", next);
                            let _ = handle.emit("deskdash:config-updated", &*st.config.read());
                        }
                    });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "manager" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let st = window.app_handle().state::<AppState>();
                    if st.config.read().settings.minimize_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        surfaces::cleanup_all_surfaces(&st);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::get_monitors,
            commands::list_installed_widgets,
            commands::audit_widget_permissions,
            commands::add_widget_instance,
            commands::update_widget_instance,
            commands::remove_widget_instance,
            commands::add_bar_instance,
            commands::update_bar_instance,
            commands::remove_bar_instance,
            commands::set_edit_mode,
            commands::update_settings,
            commands::update_interactive_rects,
            commands::restore_native_taskbar,
            commands::open_data_dir,
            commands::open_canvas_preview,
            commands::list_marketplace,
            commands::install_marketplace_widget,
            commands::import_local_widget,
            commands::uninstall_widget,
            commands::dd_invoke,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DeskDash");
}
