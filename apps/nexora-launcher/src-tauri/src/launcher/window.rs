use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};

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

pub fn show_launcher(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LAUNCHER_WINDOW) else {
        return;
    };
    apply_window_icon(&window);

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

    let _ = window.emit(EVENT_LAUNCHER_HIDDEN, ());
    let _ = window.hide();
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
        if window.is_visible().unwrap_or(false) {
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
