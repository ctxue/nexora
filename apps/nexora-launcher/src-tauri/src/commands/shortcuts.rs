use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::{
    models::ShortcutStatus,
    platform::hooks::{set_alt_space_enabled, set_shortcut_capture as set_capture},
    state::ActiveShortcut,
};

#[tauri::command]
pub fn set_global_shortcut(app: AppHandle, shortcut: String) -> Result<(), String> {
    let shortcut = shortcut.trim();
    if shortcut.is_empty() {
        return Err("shortcut_invalid".to_string());
    }

    let shortcuts = app.global_shortcut();
    let state = app.state::<ActiveShortcut>();
    let previous = state
        .current
        .lock()
        .ok()
        .and_then(|current| current.clone());

    if previous.as_deref() == Some(shortcut) {
        if let Ok(mut error) = state.error.lock() {
            *error = None;
        }
        return Ok(());
    }

    if shortcuts.register(shortcut).is_err() {
        if let Ok(mut error) = state.error.lock() {
            *error = Some("conflict".to_string());
        }
        return Err("shortcut_conflict".to_string());
    }

    if let Some(previous) = previous.as_deref() {
        let _ = shortcuts.unregister(previous);
    }

    if let Ok(mut current) = state.current.lock() {
        *current = Some(shortcut.to_string());
    }
    if let Ok(mut error) = state.error.lock() {
        *error = None;
    }
    set_alt_space_enabled(shortcut.eq_ignore_ascii_case(crate::config::DEFAULT_SHORTCUT));

    Ok(())
}

#[tauri::command]
pub fn set_shortcut_capture(active: bool) {
    set_capture(active);
}

#[tauri::command]
pub fn get_shortcut_status(app: AppHandle) -> ShortcutStatus {
    let state = app.state::<ActiveShortcut>();
    ShortcutStatus {
        current: state
            .current
            .lock()
            .ok()
            .and_then(|current| current.clone()),
        error: state.error.lock().ok().and_then(|error| error.clone()),
    }
}
