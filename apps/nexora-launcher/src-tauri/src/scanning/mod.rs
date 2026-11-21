mod search;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{
    apps,
    config::{
        FILE_SCAN_MAX_DEPTH, IGNORED_DIRS, IGNORED_EXTENSIONS, JUNK_SHORTCUT_MARKERS, MAX_APPS,
        MAX_FILES, START_MENU_MAX_DEPTH,
    },
    models::{AppEntry, FileEntry},
};

pub use search::{search_file_entries, search_files};

fn start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(data_dir) = dirs::data_dir() {
        roots.push(
            data_dir
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }

    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(
            PathBuf::from(program_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }

    roots
}

fn is_junk_shortcut(name: &str) -> bool {
    let name = name.to_lowercase();
    JUNK_SHORTCUT_MARKERS
        .iter()
        .any(|marker| name.contains(marker))
}

pub fn scan_apps() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let mut seen = HashSet::new();

    for (name, path) in apps::scan_apps_folder() {
        if name.starts_with('{') || is_junk_shortcut(&name) {
            continue;
        }

        let key = name.to_lowercase();
        if !seen.insert(key) {
            continue;
        }

        apps.push(AppEntry {
            id: format!("app:{path}"),
            name,
            path,
            source: "Apps".to_string(),
        });
    }

    for root in start_menu_roots() {
        if !root.exists() {
            continue;
        }

        for entry in WalkDir::new(root)
            .max_depth(START_MENU_MAX_DEPTH)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let is_shortcut = path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("lnk"))
                .unwrap_or(false);

            if !is_shortcut {
                continue;
            }

            let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() || is_junk_shortcut(name) {
                continue;
            }

            let key = name.to_lowercase();
            if !seen.insert(key) {
                if let Some(existing) = apps
                    .iter_mut()
                    .find(|app| app.name.eq_ignore_ascii_case(name))
                {
                    existing.path = path.to_string_lossy().to_string();
                    existing.id = format!("app:{}", existing.path);
                    existing.source = "Start Menu".to_string();
                }
                continue;
            }

            apps.push(AppEntry {
                id: format!("app:{}", path.display()),
                name: name.to_string(),
                path: path.to_string_lossy().to_string(),
                source: "Start Menu".to_string(),
            });
        }
    }

    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    apps.truncate(MAX_APPS);
    apps
}

fn is_ignored_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_lowercase();

    IGNORED_DIRS.contains(&name.as_str())
}

fn is_ignored_extension(extension: &str) -> bool {
    IGNORED_EXTENSIONS.contains(&extension)
}

fn collect_path_files(
    root: &Path,
    max_depth: usize,
    include_directories: bool,
    files: &mut Vec<FileEntry>,
    seen: &mut HashSet<String>,
) {
    for entry in WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !is_ignored_dir(entry.path()))
        .filter_map(Result::ok)
    {
        if files.len() >= MAX_FILES {
            break;
        }
        let is_directory = entry.file_type().is_dir();
        if !entry.file_type().is_file() && !(include_directories && is_directory) {
            continue;
        }
        if entry.path() == root {
            continue;
        }

        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }

        let extension = if is_directory {
            String::new()
        } else {
            path.extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("")
                .to_lowercase()
        };
        if !is_directory && (extension.is_empty() || is_ignored_extension(&extension)) {
            continue;
        }

        let path_string = path.to_string_lossy().to_string();
        if !seen.insert(path_string.clone()) {
            continue;
        }

        let parent = path
            .parent()
            .map(|parent| parent.to_string_lossy().to_string())
            .unwrap_or_default();
        let search_name = name.to_lowercase();
        let search_text = format!("{} {}", search_name, path_string.to_lowercase());

        files.push(FileEntry {
            id: format!("file:{path_string}"),
            name: name.to_string(),
            path: path_string,
            extension,
            parent,
            is_directory,
            search_name,
            search_text,
        });
    }
}

fn sort_path_files(files: &mut [FileEntry]) {
    files.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.search_name.cmp(&b.search_name))
            .then_with(|| a.path.cmp(&b.path))
    });
}

pub fn scan_path_files(root: &Path, max_depth: usize) -> Vec<FileEntry> {
    if !root.is_dir() {
        return Vec::new();
    }

    let mut files = Vec::new();
    let mut seen = HashSet::new();
    collect_path_files(root, max_depth, true, &mut files, &mut seen);
    sort_path_files(&mut files);
    files
}

pub fn scan_direct_path_files(root: &Path, limit: usize) -> Vec<FileEntry> {
    if !root.is_dir() {
        return Vec::new();
    }

    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut files = Vec::new();

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if is_ignored_dir(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let is_directory = file_type.is_dir();
        if !is_directory && !file_type.is_file() {
            continue;
        }

        let extension = if is_directory {
            String::new()
        } else {
            path.extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("")
                .to_lowercase()
        };
        if !is_directory && (extension.is_empty() || is_ignored_extension(&extension)) {
            continue;
        }

        let path_string = path.to_string_lossy().to_string();
        let parent = path
            .parent()
            .map(|parent| parent.to_string_lossy().to_string())
            .unwrap_or_default();
        let search_name = name.to_lowercase();
        files.push(FileEntry {
            id: format!("file:{path_string}"),
            name: name.to_string(),
            path: path_string.clone(),
            extension,
            parent,
            is_directory,
            search_name,
            search_text: format!("{} {}", name.to_lowercase(), path_string.to_lowercase()),
        });
    }

    sort_path_files(&mut files);
    files.truncate(limit.max(1));
    files
}

pub fn scan_files() -> Vec<FileEntry> {
    let mut roots = Vec::new();
    for root in [
        dirs::desktop_dir(),
        dirs::document_dir(),
        dirs::download_dir(),
        dirs::picture_dir(),
    ]
    .into_iter()
    .flatten()
    {
        if root.exists() {
            roots.push(root);
        }
    }

    let mut files = Vec::new();
    let mut seen = HashSet::new();

    for root in roots {
        collect_path_files(&root, FILE_SCAN_MAX_DEPTH, false, &mut files, &mut seen);
        if files.len() >= MAX_FILES {
            break;
        }
    }

    sort_path_files(&mut files);
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_files_below_an_arbitrary_path() {
        let root = std::env::temp_dir().join(format!("nexora-path-scan-{}", std::process::id()));
        let nested = root.join("Projects").join("Nested");
        std::fs::create_dir_all(&nested).expect("create path fixture");
        std::fs::write(root.join("root.txt"), b"root").expect("write root file");
        std::fs::write(nested.join("child.md"), b"child").expect("write nested file");

        let files = scan_path_files(&root, 4);
        std::fs::remove_dir_all(&root).ok();

        assert!(files.iter().any(|file| file.name == "root.txt"));
        assert!(files.iter().any(|file| file.name == "child.md"));
        assert!(files
            .iter()
            .any(|entry| entry.name == "Projects" && entry.is_directory));
    }
}
