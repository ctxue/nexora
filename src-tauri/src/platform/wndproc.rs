use std::sync::atomic::{AtomicIsize, Ordering};

use tauri::{Emitter, Manager};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::VK_SPACE,
        WindowsAndMessaging::{
            CallWindowProcW, DefWindowProcW, SetWindowLongPtrW, GWLP_WNDPROC, SC_KEYMENU,
            WM_KEYDOWN, WM_SYSCOMMAND, WM_SYSKEYDOWN, WNDPROC,
        },
    },
};

use crate::{
    config::{EVENT_SHORTCUT_CAPTURED, SETTINGS_WINDOW},
    platform::hooks::{hook_app, is_capturing, modifier_state, space_shortcut_label},
};

static PREVIOUS_WNDPROC: AtomicIsize = AtomicIsize::new(0);

fn emit_shortcut(shortcut: &str) {
    if let Some(app) = hook_app() {
        if let Some(settings) = app.get_webview_window(SETTINGS_WINDOW) {
            let _ = settings.emit(EVENT_SHORTCUT_CAPTURED, shortcut.to_string());
        }
    }
}

unsafe extern "system" fn settings_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if is_capturing() && message == WM_SYSCOMMAND && (wparam.0 & 0xFFF0) == SC_KEYMENU as usize {
        emit_shortcut("Alt+Space");
        return LRESULT(0);
    }

    if is_capturing()
        && (message == WM_KEYDOWN || message == WM_SYSKEYDOWN)
        && wparam.0 == VK_SPACE.0 as usize
    {
        let (ctrl, alt, shift, win) = modifier_state();
        if ctrl || alt || shift || win {
            emit_shortcut(&space_shortcut_label());
            return LRESULT(0);
        }
    }

    let previous = PREVIOUS_WNDPROC.load(Ordering::Relaxed);
    if previous != 0 {
        let procedure: WNDPROC = std::mem::transmute(previous);
        CallWindowProcW(procedure, hwnd, message, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}

pub fn install(app: &tauri::AppHandle) {
    let Some(settings) = app.get_webview_window(SETTINGS_WINDOW) else {
        return;
    };
    let Ok(hwnd) = settings.hwnd() else {
        return;
    };

    let previous = unsafe {
        SetWindowLongPtrW(
            HWND(hwnd.0),
            GWLP_WNDPROC,
            settings_wndproc as *const () as isize,
        )
    };
    PREVIOUS_WNDPROC.store(previous, Ordering::Relaxed);
}
