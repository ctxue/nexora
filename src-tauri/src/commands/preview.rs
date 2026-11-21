use crate::preview::{read_preview, PreviewData};

#[tauri::command]
pub fn preview_file(path: String) -> Result<PreviewData, String> {
    read_preview(&path)
}
