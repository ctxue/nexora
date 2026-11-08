use tauri::{AppHandle, Manager, WebviewWindow};

use crate::{launcher::watch_drag_release, state::LauncherState};

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn start_window_drag(
    window: WebviewWindow,
    state: tauri::State<'_, LauncherState>,
) -> Result<(), String> {
    if let Ok(mut dragging) = state.dragging.lock() {
        *dragging = true;
    }

    let result = window.start_dragging();
    watch_drag_release(window.app_handle().clone());

    result.map_err(|error| format!("Could not start dragging: {error}"))
}
