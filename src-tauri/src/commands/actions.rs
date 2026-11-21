use std::process::Command;

use windows::{
    core::{w, PCWSTR},
    Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
};

use crate::state::LauncherState;

#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Path is empty".to_string());
    }
    Command::new("explorer.exe")
        .arg(format!("/select,{path}"))
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not show the item in Explorer: {error}"))
}

#[tauri::command]
pub fn open_with(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Path is empty".to_string());
    }
    Command::new("rundll32.exe")
        .args(["shell32.dll,OpenAs_RunDLL", path])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open the Open With dialog: {error}"))
}

#[tauri::command]
pub fn run_as_admin(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Path is empty".to_string());
    }
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("runas"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err("The administrator launch was cancelled or blocked".to_string())
    }
}

#[tauri::command]
pub fn read_clipboard_text() -> Result<String, String> {
    crate::clipboard::read_clipboard_text()
        .map(|value| value.unwrap_or_default())
        .map_err(|_| "The clipboard is temporarily unavailable".to_string())
}

#[tauri::command]
pub fn get_selected_text(state: tauri::State<'_, LauncherState>) -> String {
    state
        .selected_text
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default()
}
