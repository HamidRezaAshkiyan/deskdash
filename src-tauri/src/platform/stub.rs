use crate::models::{BarEdge, MediaSession, Rect, SystemTheme, WindowInfo};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static NATIVE_TASKBAR_HIDDEN: AtomicBool = AtomicBool::new(false);
static MOCK_MEDIA_PLAYING: AtomicBool = AtomicBool::new(true);

fn rects_store() -> &'static Mutex<HashMap<String, Vec<Rect>>> {
    static STORE: OnceLock<Mutex<HashMap<String, Vec<Rect>>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn attach_desktop_canvas(
    _label: &str,
    _hwnd_raw: isize,
    _x: i32,
    _y: i32,
    _width: i32,
    _height: i32,
) -> Result<(), String> {
    Ok(())
}

pub fn register_bar_hit_surface(_label: &str, _hwnd_raw: isize, _x: i32, _y: i32) {}

pub fn unregister_hit_surface(label: &str) {
    rects_store().lock().remove(label);
}

pub fn update_interactive_rects(label: &str, rects: Vec<Rect>) {
    rects_store().lock().insert(label.to_string(), rects);
}

pub fn set_edit_mode_for_canvas(_label: &str, _enabled: bool) {}

pub fn register_appbar(
    _hwnd_raw: isize,
    edge: BarEdge,
    thickness: i32,
    mon_x: i32,
    mon_y: i32,
    mon_w: i32,
    mon_h: i32,
) -> Result<Rect, String> {
    let rect = match edge {
        BarEdge::Top => Rect {
            x: mon_x as f64,
            y: mon_y as f64,
            width: mon_w as f64,
            height: thickness as f64,
        },
        BarEdge::Bottom => Rect {
            x: mon_x as f64,
            y: (mon_y + mon_h - thickness) as f64,
            width: mon_w as f64,
            height: thickness as f64,
        },
        BarEdge::Left => Rect {
            x: mon_x as f64,
            y: mon_y as f64,
            width: thickness as f64,
            height: mon_h as f64,
        },
        BarEdge::Right => Rect {
            x: (mon_x + mon_w - thickness) as f64,
            y: mon_y as f64,
            width: thickness as f64,
            height: mon_h as f64,
        },
    };
    Ok(rect)
}

pub fn unregister_appbar(_hwnd_raw: isize) {}

pub fn set_native_taskbar_visible(visible: bool) -> Result<(), String> {
    NATIVE_TASKBAR_HIDDEN.store(!visible, Ordering::SeqCst);
    Ok(())
}

#[allow(dead_code)]
pub fn is_native_taskbar_hidden() -> bool {
    NATIVE_TASKBAR_HIDDEN.load(Ordering::SeqCst)
}

pub fn list_open_windows() -> Vec<WindowInfo> {
    vec![
        WindowInfo {
            id: "mock-1".into(),
            title: "Visual Studio Code — deskdash".into(),
            app_name: "Code".into(),
            exe_path: "C:\\Program Files\\Microsoft VS Code\\Code.exe".into(),
            is_focused: true,
            is_minimized: false,
            icon_data_url: None,
        },
        WindowInfo {
            id: "mock-2".into(),
            title: "Windows Terminal".into(),
            app_name: "WindowsTerminal".into(),
            exe_path: "wt.exe".into(),
            is_focused: false,
            is_minimized: false,
            icon_data_url: None,
        },
        WindowInfo {
            id: "mock-3".into(),
            title: "Spotify".into(),
            app_name: "Spotify".into(),
            exe_path: "Spotify.exe".into(),
            is_focused: false,
            is_minimized: true,
            icon_data_url: None,
        },
    ]
}

pub fn focus_window(_id: &str) -> Result<(), String> {
    Ok(())
}

pub fn minimize_window(_id: &str) -> Result<(), String> {
    Ok(())
}

pub fn close_window(_id: &str) -> Result<(), String> {
    Ok(())
}

pub fn open_start_menu() -> Result<(), String> {
    Ok(())
}

pub fn launch_app(_path: &str, _args: &[String]) -> Result<(), String> {
    Ok(())
}

pub fn is_foreground_fullscreen() -> bool {
    false
}

pub fn get_media_session() -> Option<MediaSession> {
    Some(MediaSession {
        title: "Midnight City (Preview)".into(),
        artist: "M83".into(),
        album: "Hurry Up, We're Dreaming".into(),
        is_playing: MOCK_MEDIA_PLAYING.load(Ordering::SeqCst),
        source_app: "Spotify.exe".into(),
        thumbnail_data_url: None,
    })
}

pub fn media_control(action: &str) -> Result<(), String> {
    if action == "playPause" {
        let cur = MOCK_MEDIA_PLAYING.load(Ordering::SeqCst);
        MOCK_MEDIA_PLAYING.store(!cur, Ordering::SeqCst);
    }
    Ok(())
}

pub fn set_autostart(_enabled: bool) -> Result<(), String> {
    Ok(())
}

pub fn get_system_theme() -> SystemTheme {
    SystemTheme::default()
}
