//! Low-level keyboard hook.
//!
//! The hook does three things:
//! 1. buffers characters typed before the launcher's webview owns the keyboard
//!    (see [`crate::launcher::focus`]);
//! 2. swallows `Alt+Space` when it is the active shortcut, so no application
//!    opens its system menu, and shows the launcher off the hook thread;
//! 3. forwards space combinations to the settings window while recording.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::{
    Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_SPACE,
        },
        WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
            WM_KEYDOWN, WM_SYSKEYDOWN,
        },
    },
};

use crate::{
    config::{EVENT_SHORTCUT_CAPTURED, SETTINGS_WINDOW},
    launcher::{toggle_launcher, CAPTURE_KEYS, EARLY_KEYS},
};

/// True while `Alt+Space` is the active global shortcut.
static ALT_SPACE_ENABLED: AtomicBool = AtomicBool::new(false);
/// True while the settings window records a new shortcut.
static SHORTCUT_CAPTURE: AtomicBool = AtomicBool::new(false);
/// The hook needs an `AppHandle` to emit events and toggle the launcher.
static HOOK_APP: OnceLock<AppHandle> = OnceLock::new();

pub fn set_alt_space_enabled(enabled: bool) {
    ALT_SPACE_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn set_shortcut_capture(active: bool) {
    SHORTCUT_CAPTURE.store(active, Ordering::Relaxed);
}

pub fn is_capturing() -> bool {
    SHORTCUT_CAPTURE.load(Ordering::Relaxed)
}

pub fn hook_app() -> Option<&'static AppHandle> {
    HOOK_APP.get()
}

/// `(ctrl, alt, shift, win)` currently held down.
pub fn modifier_state() -> (bool, bool, bool, bool) {
    let down = |key: i32| unsafe { GetAsyncKeyState(key) < 0 };
    (
        down(VK_CONTROL.0 as i32),
        down(VK_MENU.0 as i32),
        down(VK_SHIFT.0 as i32),
        down(VK_LWIN.0 as i32) || down(VK_RWIN.0 as i32),
    )
}

/// Builds `Ctrl+Alt+Shift+Space`-style labels from the live key state.
pub fn space_shortcut_label() -> String {
    let (ctrl, alt, shift, win) = modifier_state();
    let mut modifiers = Vec::new();
    if ctrl {
        modifiers.push("Ctrl");
    }
    if alt {
        modifiers.push("Alt");
    }
    if shift {
        modifiers.push("Shift");
    }
    if win {
        modifiers.push("Super");
    }

    format!("{}+Space", modifiers.join("+"))
}

fn emit_shortcut(shortcut: &str) {
    if let Some(app) = hook_app() {
        if let Some(settings) = app.get_webview_window(SETTINGS_WINDOW) {
            let _ = settings.emit(EVENT_SHORTCUT_CAPTURED, shortcut.to_string());
        }
    }
}

/// Buffers a printable character typed before the webview can take it.
fn capture_early_key(keyboard: &KBDLLHOOKSTRUCT) -> bool {
    let key = keyboard.vkCode;
    let (ctrl, _, _, win) = modifier_state();
    let printable =
        (0x20..=0xFE).contains(&key) && !matches!(key, 0x10..=0x12 | 0x5B..=0x5D | 0xA0..=0xA5);

    if ctrl || win || !printable {
        return false;
    }

    if let Ok(mut keys) = EARLY_KEYS.lock() {
        keys.push((key, keyboard.scanCode, keyboard.flags.0));
    }
    true
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && (wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize) {
        let keyboard = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

        if CAPTURE_KEYS.load(Ordering::Relaxed) && capture_early_key(keyboard) {
            return LRESULT(1);
        }

        if ALT_SPACE_ENABLED.load(Ordering::Relaxed) && keyboard.vkCode == VK_SPACE.0 as u32 {
            let (ctrl, alt, shift, win) = modifier_state();

            if SHORTCUT_CAPTURE.load(Ordering::Relaxed) && (ctrl || alt || shift || win) {
                emit_shortcut(&space_shortcut_label());
                return LRESULT(1);
            }

            if alt {
                if let Some(app) = hook_app() {
                    // Capture from this keystroke on, then show the window off
                    // the hook thread: blocking here would let the first
                    // characters slip past the buffer and reach the old window.
                    CAPTURE_KEYS.store(true, Ordering::Relaxed);
                    let app = app.clone();
                    std::thread::spawn(move || toggle_launcher(&app));
                }
                return LRESULT(1);
            }
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}

/// Installs the hook on a dedicated thread that pumps its own message loop.
pub fn install(app: AppHandle) {
    let _ = HOOK_APP.set(app);

    std::thread::spawn(|| unsafe {
        let module = GetModuleHandleW(None).unwrap_or_default();
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_proc),
            Some(HINSTANCE(module.0)),
            0,
        );

        if hook.is_ok() {
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {}
        }
    });
}
