use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows::{
    core::Error,
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWINDOWATTRIBUTE,
        },
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
        UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON},
        UI::Shell::{
            SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
        },
        UI::WindowsAndMessaging::{
            GetClassNameW, GetForegroundWindow, GetShellWindow, GetWindowRect, IsWindowVisible,
        },
    },
};

use crate::{
    config::{
        EVENT_LAUNCHER_DRAG_ENDED, EVENT_LAUNCHER_HIDDEN, EVENT_LAUNCHER_SHOWN,
        FOCUS_RETRY_INTERVAL, LAUNCHER_WINDOW, TOGGLE_DEBOUNCE_MS,
    },
    launcher::focus::{focus_launcher, wait_for_input_then_replay, CAPTURE_KEYS, EARLY_KEYS},
    state::LauncherState,
};

pub fn apply_window_icon(window: &WebviewWindow) {
    if let Ok(icon) = tauri::image::Image::from_bytes(include_bytes!("../../icons/128x128.png")) {
        let _ = window.set_icon(icon);
    }
}

fn set_launcher_window_transitions(
    window: HWND,
    apply: impl FnOnce(HWND, DWMWINDOWATTRIBUTE, u32) -> Result<(), Error>,
) -> Result<(), Error> {
    apply(window, DWMWA_TRANSITIONS_FORCEDISABLED, 1)
}

fn disable_launcher_window_transitions(window: &WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };

    let _ = set_launcher_window_transitions(HWND(hwnd.0), |target, attribute, value| unsafe {
        DwmSetWindowAttribute(
            target,
            attribute,
            &value as *const u32 as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        )
    });
}

fn rect_covers_monitor(window: RECT, monitor: RECT) -> bool {
    const TOLERANCE: i32 = 8;
    window.left <= monitor.left + TOLERANCE
        && window.top <= monitor.top + TOLERANCE
        && window.right >= monitor.right - TOLERANCE
        && window.bottom >= monitor.bottom - TOLERANCE
}

fn is_shell_window_class(class_name: &str) -> bool {
    matches!(
        class_name.to_ascii_lowercase().as_str(),
        "progman" | "workerw" | "shell_traywnd" | "shell_secondarytraywnd"
    )
}

fn foreground_is_fullscreen() -> bool {
    unsafe {
        if matches!(
            SHQueryUserNotificationState(),
            Ok(QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
        ) {
            return true;
        }

        let window = GetForegroundWindow();
        let shell_window = GetShellWindow();
        if window.0.is_null() || window.0 == shell_window.0 || !IsWindowVisible(window).as_bool() {
            return false;
        }

        let mut class_buffer = [0u16; 256];
        let class_length = GetClassNameW(window, &mut class_buffer);
        if class_length > 0 {
            let class_name = String::from_utf16_lossy(&class_buffer[..class_length as usize]);
            if is_shell_window_class(&class_name) {
                return false;
            }
        }

        let mut window_rect = RECT::default();
        if GetWindowRect(window, &mut window_rect).is_err() {
            return false;
        }

        let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
        if monitor.0.is_null() {
            return false;
        }

        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        if !GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            return false;
        }

        rect_covers_monitor(window_rect, monitor_info.rcMonitor)
    }
}

pub fn launcher_shortcut_allowed(app: &AppHandle) -> bool {
    let guarded = app
        .try_state::<LauncherState>()
        .map(|state| {
            state
                .disable_in_fullscreen
                .load(std::sync::atomic::Ordering::Relaxed)
        })
        .unwrap_or(false);

    !guarded || !foreground_is_fullscreen()
}

pub fn show_launcher(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LAUNCHER_WINDOW) else {
        return;
    };
    apply_window_icon(&window);
    disable_launcher_window_transitions(&window);

    if let Some(state) = app.try_state::<LauncherState>() {
        if let Ok(mut selected) = state.selected_text.lock() {
            *selected = crate::platform::selection::capture_focused_selection();
        }
        if let Ok(mut shown_at) = state.shown_at.lock() {
            *shown_at = Some(Instant::now());
        }
        if let Ok(mut dragging) = state.dragging.lock() {
            *dragging = false;
        }
    }

    let _ = window.eval(
        "document.querySelector('.search-input')?.focus({ preventScroll: true }); window.focus()",
    );
    CAPTURE_KEYS.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = window.set_always_on_top(true);
    let _ = window.unminimize();
    let _ = window.show();
    focus_launcher(&window);
    let _ = window.emit(EVENT_LAUNCHER_SHOWN, ());

    let window = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FOCUS_RETRY_INTERVAL * 2);
        focus_launcher(&window);
        let _ =
            window.eval("document.querySelector('.search-input')?.focus({ preventScroll: true })");
        wait_for_input_then_replay(&window);
    });
}

pub fn hide_launcher(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LAUNCHER_WINDOW) else {
        return;
    };

    if let Some(state) = app.try_state::<LauncherState>() {
        if let Ok(mut selected) = state.selected_text.lock() {
            selected.clear();
        }
    }

    CAPTURE_KEYS.store(false, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut keys) = EARLY_KEYS.lock() {
        keys.clear();
    }

    let _ = window.hide();
    let _ = window.emit(EVENT_LAUNCHER_HIDDEN, ());
}

pub fn toggle_launcher(app: &AppHandle) {
    if let Some(state) = app.try_state::<LauncherState>() {
        if let Ok(mut last_toggle) = state.last_toggle.lock() {
            let now = Instant::now();
            if last_toggle
                .map(|previous| {
                    now.duration_since(previous) < Duration::from_millis(TOGGLE_DEBOUNCE_MS)
                })
                .unwrap_or(false)
            {
                return;
            }
            *last_toggle = Some(now);
        }
    }

    if let Some(window) = app.get_webview_window(LAUNCHER_WINDOW) {
        let visible = window.is_visible().unwrap_or(false);
        if !visible && !launcher_shortcut_allowed(app) {
            CAPTURE_KEYS.store(false, std::sync::atomic::Ordering::Relaxed);
            if let Ok(mut keys) = EARLY_KEYS.lock() {
                keys.clear();
            }
            return;
        }

        if visible {
            hide_launcher(app);
        } else {
            show_launcher(app);
        }
    }
}

pub fn watch_drag_release(app: AppHandle) {
    std::thread::spawn(move || {
        let started = Instant::now();
        loop {
            std::thread::sleep(crate::config::DRAG_POLL_INTERVAL);
            let pressed = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } < 0;
            if !pressed || started.elapsed() > crate::config::DRAG_MAX_DURATION {
                break;
            }
        }

        if let Some(state) = app.try_state::<LauncherState>() {
            if let Ok(mut dragging) = state.dragging.lock() {
                *dragging = false;
            }
        }
        let _ = app.emit(EVENT_LAUNCHER_DRAG_ENDED, ());
    });
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

    use windows::Win32::Foundation::{HWND, RECT};

    use super::*;

    #[test]
    fn disables_dwm_transitions_for_the_launcher_window() {
        let hwnd = HWND(std::ptr::NonNull::<c_void>::dangling().as_ptr());
        let mut applied = None;

        let result = set_launcher_window_transitions(hwnd, |target, attribute, value| {
            applied = Some((target, attribute, value));
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(applied, Some((hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1)));
    }

    #[test]
    fn detects_windows_that_cover_the_full_monitor() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 2560,
            bottom: 1440,
        };
        let fullscreen = RECT {
            left: -8,
            top: -8,
            right: 2568,
            bottom: 1448,
        };
        let windowed = RECT {
            left: 0,
            top: 0,
            right: 2560,
            bottom: 1392,
        };

        assert!(rect_covers_monitor(fullscreen, monitor));
        assert!(!rect_covers_monitor(windowed, monitor));
    }

    #[test]
    fn ignores_windows_shell_hosts_when_checking_fullscreen() {
        for class_name in [
            "Progman",
            "WorkerW",
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
        ] {
            assert!(is_shell_window_class(class_name));
        }
        assert!(!is_shell_window_class("Chrome_WidgetWin_1"));
        assert!(!is_shell_window_class("UnrealWindow"));
    }
}
