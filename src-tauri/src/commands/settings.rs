//! Commands for the settings window: opening it, theming it, and moving the
//! configuration in and out as JSON.

use tauri::{AppHandle, Manager};

use crate::{
    config::{LAUNCHER_WINDOW, SETTINGS_WINDOW},
    platform::{dialogs, titlebar},
};

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window(LAUNCHER_WINDOW) {
        let _ = main.hide();
    }

    let window = app
        .get_webview_window(SETTINGS_WINDOW)
        .ok_or_else(|| "Settings window is not available".to_string())?;
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}

#[tauri::command]
pub fn set_app_theme(app: AppHandle, theme: String) -> Result<(), String> {
    if let Some(settings) = app.get_webview_window(SETTINGS_WINDOW) {
        titlebar::apply(&settings, theme == "dark");
    }
    Ok(())
}

/// Opens the native save dialog and writes the exported configuration there.
#[tauri::command]
pub fn export_settings(json: String) -> Result<Option<String>, String> {
    let Some(path) = dialogs::ask_save_path() else {
        return Ok(None);
    };

    std::fs::write(&path, json).map_err(|error| format!("Could not write the file: {error}"))?;
    Ok(Some(path.to_string_lossy().to_string()))
}

/// Opens the native open dialog and returns the selected configuration file.
#[tauri::command]
pub fn import_settings() -> Result<Option<String>, String> {
    let Some(path) = dialogs::ask_open_path() else {
        return Ok(None);
    };

    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Could not read the file: {error}"))?;
    Ok(Some(contents))
}

#[tauri::command]
pub fn set_tray_language(app: AppHandle, language: String) -> Result<(), String> {
    crate::tray::set_language(&app, &language)
}

#[tauri::command]
pub fn reset_window_position(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(LAUNCHER_WINDOW)
        .ok_or_else(|| "Launcher window is not available".to_string())?;

    window
        .center()
        .map_err(|error| format!("Could not center window: {error}"))?;
    let _ = window.set_focus();
    Ok(())
}
