use crate::state::AppState;
use crate::surfaces;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let open_i = MenuItem::with_id(app, "open_manager", "Open DeskDash Manager", true, None::<&str>)?;
    let edit_i = MenuItem::with_id(app, "toggle_edit", "Toggle Edit Mode (Ctrl+Alt+E)", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let restore_i = MenuItem::with_id(app, "restore_taskbar", "Restore Windows Taskbar", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit DeskDash", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open_i, &edit_i, &sep1, &restore_i, &sep2, &quit_i])?;

    let mut builder = TrayIconBuilder::with_id("deskdash-tray")
        .menu(&menu)
        .tooltip("DeskDash — Live Desktop Widgets & Taskbars")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_manager" => {
                if let Some(win) = app.get_webview_window("manager") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "toggle_edit" => {
                let state = app.state::<AppState>();
                let next = !state.config.read().edit_mode;
                state.config.write().edit_mode = next;
                let _ = state.save();
                let _ = surfaces::sync_canvases(app, &state);
                let _ = app.emit("deskdash:edit-mode-changed", next);
                let _ = app.emit("deskdash:config-updated", &*state.config.read());
            }
            "restore_taskbar" => {
                let state = app.state::<AppState>();
                let _ = surfaces::set_native_taskbar_with_lock(&state, true);
            }
            "quit" => {
                let state = app.state::<AppState>();
                surfaces::cleanup_all_surfaces(&state);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("manager") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
