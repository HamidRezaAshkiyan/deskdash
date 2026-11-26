use crate::models::{BarEdge, MediaSession, Rect, SystemTheme, WindowInfo};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_SZ,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_LWIN,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ShellExecuteW, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW,
    ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, FindWindowW, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW,
    GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, PostMessageW, SendMessageTimeoutW, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_BOTTOM, HWND_TOPMOST, SMTO_NORMAL, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, WM_CLOSE,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

static NATIVE_TASKBAR_HIDDEN: AtomicBool = AtomicBool::new(false);

struct WindowHitState {
    hwnd: isize,
    edit_mode: bool,
    rects: Vec<Rect>,
    window_origin: (f64, f64),
    is_click_through: bool,
}

fn hit_registry() -> &'static Mutex<HashMap<String, WindowHitState>> {
    static REG: OnceLock<Mutex<HashMap<String, WindowHitState>>> = OnceLock::new();
    REG.get_or_init(|| {
        start_hit_test_loop();
        Mutex::new(HashMap::new())
    })
}

fn start_hit_test_loop() {
    std::thread::spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_millis(24));
        let mut pt = POINT::default();
        let ok = unsafe { GetCursorPos(&mut pt).is_ok() };
        if !ok {
            continue;
        }
        let mut reg = hit_registry().lock();
        for state in reg.values_mut() {
            let should_pass_through = if state.edit_mode {
                false
            } else {
                let rel_x = pt.x as f64 - state.window_origin.0;
                let rel_y = pt.y as f64 - state.window_origin.1;
                !super::point_in_rects(rel_x, rel_y, &state.rects)
            };
            if should_pass_through != state.is_click_through {
                apply_click_through(HWND(state.hwnd as *mut _), should_pass_through);
                state.is_click_through = should_pass_through;
            }
        }
    });
}

fn apply_click_through(hwnd: HWND, click_through: bool) {
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new_ex = if click_through {
            ex | (WS_EX_TRANSPARENT.0 as isize)
        } else {
            ex & !(WS_EX_TRANSPARENT.0 as isize)
        };
        if new_ex != ex {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex);
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Pins a transparent desktop canvas window at the desktop layer behind normal windows
/// while keeping it resilient to Win+D and invisible in Alt+Tab / taskbar.
pub fn attach_desktop_canvas(
    label: &str,
    hwnd_raw: isize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    let hwnd = HWND(hwnd_raw as *mut _);
    unsafe {
        // Trigger Progman WorkerW spawn so desktop wallpaper layer exists on Win10/Win11
        let progman_name = wide("Progman");
        if let Ok(progman) = FindWindowW(PCWSTR(progman_name.as_ptr()), PCWSTR::null()) {
            let mut _res: usize = 0;
            let _ = SendMessageTimeoutW(
                progman,
                0x052C,
                WPARAM(0x0000000D),
                LPARAM(1),
                SMTO_NORMAL,
                1000,
                Some(&mut _res),
            );
        }

        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let desired = ex
            | (WS_EX_TOOLWINDOW.0 as isize)
            | (WS_EX_NOACTIVATE.0 as isize)
            | (WS_EX_TRANSPARENT.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired);

        let _ = SetWindowPos(
            hwnd,
            HWND_BOTTOM,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE,
        );
    }

    hit_registry().lock().insert(
        label.to_string(),
        WindowHitState {
            hwnd: hwnd_raw,
            edit_mode: false,
            rects: Vec::new(),
            window_origin: (x as f64, y as f64),
            is_click_through: true,
        },
    );
    Ok(())
}

pub fn register_bar_hit_surface(label: &str, hwnd_raw: isize, x: i32, y: i32) {
    let hwnd = HWND(hwnd_raw as *mut _);
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let desired = ex | (WS_EX_TOOLWINDOW.0 as isize) | (WS_EX_NOACTIVATE.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
    hit_registry().lock().insert(
        label.to_string(),
        WindowHitState {
            hwnd: hwnd_raw,
            edit_mode: false,
            rects: Vec::new(),
            window_origin: (x as f64, y as f64),
            is_click_through: false,
        },
    );
}

pub fn unregister_hit_surface(label: &str) {
    hit_registry().lock().remove(label);
}

pub fn update_interactive_rects(label: &str, rects: Vec<Rect>) {
    if let Some(state) = hit_registry().lock().get_mut(label) {
        state.rects = rects;
    }
}

pub fn set_edit_mode_for_canvas(label: &str, enabled: bool) {
    if let Some(state) = hit_registry().lock().get_mut(label) {
        state.edit_mode = enabled;
        let hwnd = HWND(state.hwnd as *mut _);
        unsafe {
            if enabled {
                apply_click_through(hwnd, false);
                state.is_click_through = false;
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            } else {
                let _ = SetWindowPos(
                    hwnd,
                    HWND_BOTTOM,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
}

pub fn register_appbar(
    hwnd_raw: isize,
    edge: BarEdge,
    thickness: i32,
    mon_x: i32,
    mon_y: i32,
    mon_w: i32,
    mon_h: i32,
) -> Result<Rect, String> {
    let hwnd = HWND(hwnd_raw as *mut _);
    let u_edge = match edge {
        BarEdge::Top => ABE_TOP,
        BarEdge::Bottom => ABE_BOTTOM,
        BarEdge::Left => ABE_LEFT,
        BarEdge::Right => ABE_RIGHT,
    };

    let rc = match edge {
        BarEdge::Top => RECT {
            left: mon_x,
            top: mon_y,
            right: mon_x + mon_w,
            bottom: mon_y + thickness,
        },
        BarEdge::Bottom => RECT {
            left: mon_x,
            top: mon_y + mon_h - thickness,
            right: mon_x + mon_w,
            bottom: mon_y + mon_h,
        },
        BarEdge::Left => RECT {
            left: mon_x,
            top: mon_y,
            right: mon_x + thickness,
            bottom: mon_y + mon_h,
        },
        BarEdge::Right => RECT {
            left: mon_x + mon_w - thickness,
            top: mon_y,
            right: mon_x + mon_w,
            bottom: mon_y + mon_h,
        },
    };

    let mut abd = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: 0,
        uEdge: u_edge,
        rc,
        lParam: LPARAM(0),
    };

    unsafe {
        SHAppBarMessage(ABM_NEW, &mut abd);
        SHAppBarMessage(ABM_QUERYPOS, &mut abd);
        SHAppBarMessage(ABM_SETPOS, &mut abd);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            abd.rc.left,
            abd.rc.top,
            abd.rc.right - abd.rc.left,
            abd.rc.bottom - abd.rc.top,
            SWP_NOACTIVATE,
        );
    }

    Ok(Rect {
        x: abd.rc.left as f64,
        y: abd.rc.top as f64,
        width: (abd.rc.right - abd.rc.left) as f64,
        height: (abd.rc.bottom - abd.rc.top) as f64,
    })
}

pub fn unregister_appbar(hwnd_raw: isize) {
    let hwnd = HWND(hwnd_raw as *mut _);
    let mut abd = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    };
    unsafe {
        SHAppBarMessage(ABM_REMOVE, &mut abd);
    }
}

pub fn set_native_taskbar_visible(visible: bool) -> Result<(), String> {
    let cmd = if visible { SW_SHOW } else { SW_HIDE };
    unsafe {
        let primary_cls = wide("Shell_TrayWnd");
        if let Ok(hwnd) = FindWindowW(PCWSTR(primary_cls.as_ptr()), PCWSTR::null()) {
            let _ = ShowWindow(hwnd, cmd);
        }

        let secondary_cls = wide("Shell_SecondaryTrayWnd");
        let mut prev = HWND::default();
        loop {
            match FindWindowExW(
                HWND::default(),
                prev,
                PCWSTR(secondary_cls.as_ptr()),
                PCWSTR::null(),
            ) {
                Ok(hwnd) if !hwnd.0.is_null() => {
                    let _ = ShowWindow(hwnd, cmd);
                    prev = hwnd;
                }
                _ => break,
            }
        }
    }
    NATIVE_TASKBAR_HIDDEN.store(!visible, Ordering::SeqCst);
    Ok(())
}

pub fn is_native_taskbar_hidden() -> bool {
    NATIVE_TASKBAR_HIDDEN.load(Ordering::SeqCst)
}

pub fn list_open_windows() -> Vec<WindowInfo> {
    struct Ctx {
        focused: HWND,
        items: Vec<WindowInfo>,
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam.0 as *mut Ctx);
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if (ex & WS_EX_TOOLWINDOW.0) != 0 {
            return BOOL(1);
        }

        let mut cloaked: u32 = 0;
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            std::mem::size_of::<u32>() as u32,
        );
        if cloaked != 0 {
            return BOOL(1);
        }

        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return BOOL(1);
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let read = GetWindowTextW(hwnd, &mut buf);
        if read <= 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&buf[..read as usize]);
        if title == "Program Manager" || title == "DeskDash Canvas" || title == "DeskDash Bar" {
            return BOOL(1);
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let exe_path = query_exe_path(pid).unwrap_or_default();
        let app_name = std::path::Path::new(&exe_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| title.clone());

        ctx.items.push(WindowInfo {
            id: format!("{}", hwnd.0 as isize),
            title,
            app_name,
            exe_path,
            is_focused: hwnd == ctx.focused,
            is_minimized: IsIconic(hwnd).as_bool(),
            icon_data_url: None,
        });

        BOOL(1)
    }

    let mut ctx = Ctx {
        focused: unsafe { GetForegroundWindow() },
        items: Vec::new(),
    };
    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM((&mut ctx as *mut Ctx) as isize));
    }
    ctx.items
}

fn query_exe_path(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .is_ok();
        let _ = windows::Win32::Foundation::CloseHandle(handle);
        if ok && size > 0 {
            Some(String::from_utf16_lossy(&buf[..size as usize]))
        } else {
            None
        }
    }
}

pub fn focus_window(id: &str) -> Result<(), String> {
    let raw: isize = id.parse().map_err(|_| "Invalid window handle".to_string())?;
    let hwnd = HWND(raw as *mut _);
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

pub fn minimize_window(id: &str) -> Result<(), String> {
    let raw: isize = id.parse().map_err(|_| "Invalid window handle".to_string())?;
    let hwnd = HWND(raw as *mut _);
    unsafe {
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
    }
    Ok(())
}

pub fn close_window(id: &str) -> Result<(), String> {
    let raw: isize = id.parse().map_err(|_| "Invalid window handle".to_string())?;
    let hwnd = HWND(raw as *mut _);
    unsafe {
        let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
    }
    Ok(())
}

pub fn open_start_menu() -> Result<(), String> {
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_LWIN,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_LWIN,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

pub fn launch_app(path: &str, args: &[String]) -> Result<(), String> {
    let op = wide("open");
    let file = wide(path);
    let joined = args.join(" ");
    let params = wide(&joined);
    unsafe {
        let res = ShellExecuteW(
            HWND::default(),
            PCWSTR(op.as_ptr()),
            PCWSTR(file.as_ptr()),
            if args.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(params.as_ptr())
            },
            PCWSTR::null(),
            SW_SHOW,
        );
        if (res.0 as isize) <= 32 {
            return Err(format!("ShellExecuteW failed for '{}'", path));
        }
    }
    Ok(())
}

pub fn is_foreground_fullscreen() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        let mut rect = RECT::default();
        if GetWindowRect(fg, &mut rect).is_err() {
            return false;
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        w >= 1920 && h >= 1080 && rect.left <= 0 && rect.top <= 0
    }
}

pub fn get_media_session() -> Option<MediaSession> {
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };

    let mgr = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()?
        .get()
        .ok()?;
    let session = mgr.GetCurrentSession().ok()?;
    let props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
    let playback = session.GetPlaybackInfo().ok()?;
    let status = playback.PlaybackStatus().ok()?;

    Some(MediaSession {
        title: props.Title().map(|s| s.to_string()).unwrap_or_default(),
        artist: props.Artist().map(|s| s.to_string()).unwrap_or_default(),
        album: props.AlbumTitle().map(|s| s.to_string()).unwrap_or_default(),
        is_playing: status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing,
        source_app: session
            .SourceAppUserModelId()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        thumbnail_data_url: None,
    })
}

pub fn media_control(action: &str) -> Result<(), String> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;

    let mgr = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;
    let session = mgr
        .GetCurrentSession()
        .map_err(|_| "No active media session".to_string())?;

    match action {
        "playPause" => {
            let _ = session
                .TryTogglePlayPauseAsync()
                .map_err(|e| e.to_string())?
                .get();
        }
        "next" => {
            let _ = session
                .TrySkipNextAsync()
                .map_err(|e| e.to_string())?
                .get();
        }
        "prev" => {
            let _ = session
                .TrySkipPreviousAsync()
                .map_err(|e| e.to_string())?
                .get();
        }
        _ => return Err(format!("Unknown media action '{}'", action)),
    }
    Ok(())
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let subkey = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let value_name = wide("DeskDash");
    unsafe {
        let mut hkey = HKEY::default();
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_WRITE,
            &mut hkey,
        )
        .ok()
        .map_err(|e| format!("RegOpenKeyExW failed: {}", e))?;

        let result = if enabled {
            let exe = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .to_string();
            let quoted = wide(&format!("\"{}\"", exe));
            let bytes = std::slice::from_raw_parts(
                quoted.as_ptr() as *const u8,
                quoted.len() * std::mem::size_of::<u16>(),
            );
            RegSetValueExW(hkey, PCWSTR(value_name.as_ptr()), 0, REG_SZ, Some(bytes))
                .ok()
                .map_err(|e| format!("RegSetValueExW failed: {}", e))
        } else {
            let _ = RegDeleteValueW(hkey, PCWSTR(value_name.as_ptr()));
            Ok(())
        };
        let _ = RegCloseKey(hkey);
        result
    }
}

pub fn get_system_theme() -> SystemTheme {
    let subkey = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let val_name = wide("AppsUseLightTheme");
    let mut is_dark = true;
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        )
        .is_ok()
        {
            let mut data: u32 = 0;
            let mut size = std::mem::size_of::<u32>() as u32;
            let mut reg_type = REG_DWORD;
            if RegQueryValueExW(
                hkey,
                PCWSTR(val_name.as_ptr()),
                None,
                Some(&mut reg_type),
                Some((&mut data as *mut u32).cast()),
                Some(&mut size),
            )
            .is_ok()
            {
                is_dark = data == 0;
            }
            let _ = RegCloseKey(hkey);
        }
    }
    SystemTheme {
        is_dark,
        accent_hex: "#6366f1".to_string(),
    }
}
