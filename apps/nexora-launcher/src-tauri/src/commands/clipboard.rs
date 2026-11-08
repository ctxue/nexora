//! Commands for reading and editing the local clipboard history.

use tauri::{AppHandle, Emitter, State};

use crate::{
    clipboard::{write_clipboard_text, ClipboardEntry, ClipboardHistory},
    config::EVENT_CLIPBOARD_UPDATED,
};

fn notify(app: &AppHandle) {
    let _ = app.emit(EVENT_CLIPBOARD_UPDATED, ());
}

#[tauri::command]
pub fn list_clipboard(history: State<'_, ClipboardHistory>) -> Vec<ClipboardEntry> {
    history.list()
}

#[tauri::command]
pub fn copy_clipboard(text: String) -> Result<(), String> {
    write_clipboard_text(&text)
}

#[tauri::command]
pub fn toggle_clipboard_pin(
    app: AppHandle,
    history: State<'_, ClipboardHistory>,
    id: String,
) -> Result<(), String> {
    if !history.toggle_pin(&id) {
        return Err("Clipboard item not found".to_string());
    }
    notify(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_clipboard_item(
    app: AppHandle,
    history: State<'_, ClipboardHistory>,
    id: String,
) -> Result<(), String> {
    if !history.remove(&id) {
        return Err("Clipboard item not found".to_string());
    }
    notify(&app);
    Ok(())
}

#[tauri::command]
pub fn clear_clipboard(app: AppHandle, history: State<'_, ClipboardHistory>) {
    history.clear();
    notify(&app);
}

#[tauri::command]
pub fn set_clipboard_capture(history: State<'_, ClipboardHistory>, enabled: bool) {
    history.set_capture_enabled(enabled);
}
