use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use tauri::{AppHandle, Emitter};

use crate::{
    config::EVENT_FILES_INDEXED,
    icons,
    models::{AppEntry, FileEntry, QuickPaths},
    scanning,
    state::{FileIndex, PathFileCache},
};

const PATH_INDEX_TTL: Duration = Duration::from_secs(300);
const PATH_SEARCH_MAX_DEPTH: usize = 8;

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
pub async fn search_files(
    query: String,
    limit: usize,
    index: tauri::State<'_, FileIndex>,
) -> Result<Vec<FileEntry>, String> {
    let index = index.inner().clone();
    tauri::async_runtime::spawn_blocking(move || scanning::search_files(&query, limit, &index))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn search_files_in_path(
    query: String,
    root: String,
    limit: usize,
    cache: tauri::State<'_, PathFileCache>,
) -> Result<Vec<FileEntry>, String> {
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || search_cached_path(&root, &query, limit, &cache))
        .await
        .map_err(|error| error.to_string())
}

fn search_cached_path(
    root: &str,
    query: &str,
    limit: usize,
    cache: &PathFileCache,
) -> Vec<FileEntry> {
    let root_path = Path::new(root.trim());
    if !root_path.is_dir() {
        return Vec::new();
    }
    if query.trim().is_empty() {
        return scanning::scan_direct_path_files(root_path, limit);
    }

    let root_key = root_path
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase();
    let mut roots = match cache.roots.lock() {
        Ok(roots) => roots,
        Err(_) => return Vec::new(),
    };

    let is_stale = roots
        .get(&root_key)
        .map(|cached| cached.indexed_at.elapsed() >= PATH_INDEX_TTL)
        .unwrap_or(true);

    if is_stale {
        roots.insert(
            root_key.clone(),
            crate::state::CachedPathFiles {
                files: scanning::scan_path_files(root_path, PATH_SEARCH_MAX_DEPTH),
                indexed_at: std::time::Instant::now(),
            },
        );
    }

    roots
        .get(&root_key)
        .map(|cached| scanning::search_file_entries(query, limit, &cached.files))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_path_search_lists_root_files_and_folders() {
        let root = std::env::temp_dir().join(format!("nexora-root-search-{}", std::process::id()));
        let folder = root.join("Folder");
        let nested = folder.join("Nested");
        std::fs::create_dir_all(&nested).expect("create path fixture");
        std::fs::write(root.join("root.txt"), b"root").expect("write root file");
        std::fs::write(nested.join("child.md"), b"child").expect("write nested file");

        let cache = PathFileCache::default();
        let results = search_cached_path(root.to_str().expect("utf8 root"), "", 8, &cache);
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(results[0].name, "Folder");
        assert!(results[0].is_directory);
        assert!(results.iter().any(|entry| entry.name == "root.txt"));
        assert!(!results.iter().any(|entry| entry.name == "child.md"));
    }

    #[test]
    fn empty_path_search_handles_a_trailing_root_separator() {
        let root = std::env::temp_dir().join(format!(
            "nexora-root-search-trailing-{}",
            std::process::id()
        ));
        let folder = root.join("Font");
        std::fs::create_dir_all(&folder).expect("create path fixture");

        let cache = PathFileCache::default();
        let root_with_separator = format!("{}\\", root.display());
        let results = search_cached_path(&root_with_separator, "", 8, &cache);
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(
            results.first().map(|entry| entry.name.as_str()),
            Some("Font")
        );
        assert!(results[0].is_directory);
    }
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
