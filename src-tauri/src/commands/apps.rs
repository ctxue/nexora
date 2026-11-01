//! Commands that feed the launcher: apps, quick paths, the file index, icons.

use std::path::PathBuf;

use tauri::{AppHandle, Emitter};

use crate::{
    config::EVENT_FILES_INDEXED,
    icons,
    models::{AppEntry, FileEntry, QuickPaths},
    scanning,
    state::FileIndex,
};

#[tauri::command]
pub fn list_apps() -> Vec<AppEntry> {
    scanning::scan_apps()
}

#[tauri::command]
pub fn get_quick_paths() -> QuickPaths {
    let path_string = |path: Option<PathBuf>| {
        path.map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
    };

    QuickPaths {
        home: path_string(dirs::home_dir()),
        desktop: path_string(dirs::desktop_dir()),
        documents: path_string(dirs::document_dir()),
        downloads: path_string(dirs::download_dir()),
    }
}

#[tauri::command]
pub fn start_file_index(app: AppHandle, index: tauri::State<'_, FileIndex>) -> bool {
    if index
        .indexing
        .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        return false;
    }

    let index = index.inner().clone();
    let app = app.clone();

    std::thread::spawn(move || {
        let files = scanning::scan_files();
        let count = files.len();

        if let Ok(mut guard) = index.files.lock() {
            *guard = files;
        }
        index
            .indexing
            .store(false, std::sync::atomic::Ordering::SeqCst);

        let _ = app.emit(EVENT_FILES_INDEXED, count);
    });

    true
}

#[tauri::command]
pub fn search_files(
    query: String,
    limit: usize,
    index: tauri::State<'_, FileIndex>,
) -> Vec<FileEntry> {
    scanning::search_files(&query, limit, index.inner())
}

#[tauri::command]
pub fn get_icons(paths: Vec<String>) -> Vec<icons::IconPayload> {
    icons::icons_for_paths(paths)
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Path is empty".to_string());
    }

    if path.starts_with("shell:") {
        std::process::Command::new("explorer.exe")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Could not open shell location: {error}"))
    } else {
        open::that(path).map_err(|error| format!("Could not open path: {error}"))
    }
}
