//! Bringing the launcher to the foreground and keeping its input alive.
//!
//! Two Windows behaviours make this non-trivial:
//! * a background process may only take the foreground if it owns the latest
//!   input, so the focus call borrows the foreground thread's input queue;
//! * the webview lives in a child window, and until it owns the keyboard the
//!   characters land on the frame (which beeps). Those keys are buffered and
//!   replayed once the webview is ready.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};

use tauri::WebviewWindow;
use windows::Win32::{
    Foundation::HWND,
    System::Threading::{AttachThreadInput, GetCurrentThreadId},
    UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
            VIRTUAL_KEY,
        },
        WindowsAndMessaging::{
            BringWindowToTop, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
            SetForegroundWindow, GUITHREADINFO,
        },
    },
};

use crate::config::{EARLY_KEY_QUIET_POLL, EARLY_KEY_QUIET_POLLS, EARLY_KEY_QUIET_TARGET};

/// Set while the launcher is visible but the webview may not accept keys yet.
pub static CAPTURE_KEYS: AtomicBool = AtomicBool::new(false);
/// Keys typed before the webview owned the keyboard, replayed once it does.
pub static EARLY_KEYS: Mutex<Vec<(u32, u32, u32)>> = Mutex::new(Vec::new());

/// Raises the launcher and moves keyboard focus into the webview.
pub fn focus_launcher(window: &WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let handle = HWND(hwnd.0);

    unsafe {
        let foreground_thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let current_thread = GetCurrentThreadId();
        let attached = foreground_thread != 0
            && foreground_thread != current_thread
            && AttachThreadInput(foreground_thread, current_thread, true).as_bool();

        let _ = SetForegroundWindow(handle);
        let _ = BringWindowToTop(handle);

        if attached {
            let _ = AttachThreadInput(foreground_thread, current_thread, false);
        }
    }

    // The webview is a child window: give it the keyboard, not just the frame.
    let _ = window.as_ref().set_focus();
}

/// True once the webview child owns the keyboard for the launcher.
pub fn has_keyboard_focus(hwnd: HWND) -> bool {
    unsafe {
        let thread = GetWindowThreadProcessId(hwnd, None);
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        GetGUIThreadInfo(thread, &mut info).is_ok()
            && !info.hwndFocus.is_invalid()
            && info.hwndFocus != hwnd
    }
}

/// Replays keys buffered during warm-up through SendInput, so the IME composes
/// them exactly as if the user had typed straight into the focused input.
pub fn replay_early_keys() {
    CAPTURE_KEYS.store(false, Ordering::Relaxed);

    let keys = EARLY_KEYS
        .lock()
        .map(|mut keys| std::mem::take(&mut *keys))
        .unwrap_or_default();
    if keys.is_empty() {
        return;
    }

    let mut inputs = Vec::with_capacity(keys.len() * 2);
    for (virtual_key, scan_code, flags) in keys {
        for key_up in [false, true] {
            let mut input = INPUT {
                r#type: INPUT_KEYBOARD,
                ..Default::default()
            };
            input.Anonymous.ki = KEYBDINPUT {
                wVk: VIRTUAL_KEY(virtual_key as u16),
                wScan: scan_code as u16,
                dwFlags: if key_up {
                    KEYBD_EVENT_FLAGS(flags | KEYEVENTF_KEYUP.0)
                } else {
                    KEYBD_EVENT_FLAGS(flags)
                },
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(input);
        }
    }

    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Waits until the webview owns the keyboard, then until the current burst of
/// typing stops, so the replay keeps the typed order.
pub fn wait_for_input_then_replay(window: &WebviewWindow) {
    let Ok(handle) = window.hwnd() else {
        CAPTURE_KEYS.store(false, Ordering::Relaxed);
        return;
    };
    let hwnd = HWND(handle.0);

    for _ in 0..crate::config::FOCUS_RETRY_ATTEMPTS {
        if has_keyboard_focus(hwnd) {
            break;
        }
        std::thread::sleep(crate::config::FOCUS_RETRY_INTERVAL);
    }

    let mut quiet = 0;
    let mut previous_len = EARLY_KEYS.lock().map(|keys| keys.len()).unwrap_or(0);
    for _ in 0..EARLY_KEY_QUIET_POLLS {
        std::thread::sleep(Duration::from_millis(
            EARLY_KEY_QUIET_POLL.as_millis() as u64
        ));
        let length = EARLY_KEYS.lock().map(|keys| keys.len()).unwrap_or(0);
        if length == previous_len {
            quiet += 1;
            if quiet >= EARLY_KEY_QUIET_TARGET {
                break;
            }
        } else {
            quiet = 0;
            previous_len = length;
        }
    }

    replay_early_keys();
}
