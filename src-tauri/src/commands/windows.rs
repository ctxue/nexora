//! Open-window discovery and control.

use crate::{models::WindowEntry, windows};

#[tauri::command]
pub fn list_windows() -> Vec<WindowEntry> {
    windows::list_windows()
}

#[tauri::command]
pub fn control_window(window_id: String, action: String) -> Result<(), String> {
    windows::control_window(&window_id, &action)
}
