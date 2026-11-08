use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{
    config::{JUNK_EXECUTABLE_MARKERS, MAX_PORTABLE_APPS, PORTABLE_SCAN_MAX_DEPTH},
    models::AppEntry,
    platform::dialogs,
};

fn is_junk_executable(name: &str) -> bool {
    let name = name.to_lowercase();
    JUNK_EXECUTABLE_MARKERS
        .iter()
        .any(|marker| name.contains(marker))
}

fn normalize_token(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn looks_like_main_executable(path: &Path, stem: &str) -> bool {
    let token = normalize_token(stem);
    if token.len() < 2 {
        return false;
    }

    let mut folders = Vec::new();
    let mut current = path.parent();
    for _ in 0..2 {
        let Some(directory) = current else {
            break;
        };
        if let Some(name) = directory.file_name().and_then(|name| name.to_str()) {
            folders.push(normalize_token(name));
        }
        current = directory.parent();
    }

    folders
        .iter()
        .any(|folder| folder == &token || (token.len() >= 3 && folder.contains(&token)))
}

#[tauri::command]
pub fn pick_executable() -> Option<String> {
    dialogs::ask_executable().map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_file() -> Option<String> {
    dialogs::ask_file_any().map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_folder() -> Option<String> {
    dialogs::ask_folder().map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn scan_folder_apps(folder: String) -> Vec<AppEntry> {
    let root = PathBuf::from(folder.trim());
    if !root.is_dir() {
        return Vec::new();
    }

    let mut apps = Vec::new();
    let mut seen = HashSet::new();
    let current_exe = std::env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().to_string());

    for entry in WalkDir::new(&root)
        .max_depth(PORTABLE_SCAN_MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if apps.len() >= MAX_PORTABLE_APPS {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let is_exe = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        if !is_exe {
            continue;
        }

        let path_string = path.to_string_lossy().to_string();
        if let Some(current) = &current_exe {
            if path_string.eq_ignore_ascii_case(current) {
                continue;
            }
        }

        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let name = stem.trim();
        if name.is_empty() || is_junk_executable(name) {
            continue;
        }
        if !looks_like_main_executable(path, name) {
            continue;
        }

        if !seen.insert(name.to_lowercase()) {
            continue;
        }

        apps.push(AppEntry {
            id: format!("app:{path_string}"),
            name: name.to_string(),
            path: path_string,
            source: "Portable".to_string(),
        });
    }

    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    apps
}

#[tauri::command]
pub fn scan_index_folders(folders: Vec<String>) -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut seen_names = HashSet::new();

    for folder in folders {
        for app in scan_folder_apps(folder) {
            let path_key = app.path.to_lowercase();
            let name_key = app.name.to_lowercase();
            if !seen_paths.insert(path_key) || !seen_names.insert(name_key) {
                continue;
            }
            apps.push(AppEntry {
                source: "Index".to_string(),
                ..app
            });
            if apps.len() >= MAX_PORTABLE_APPS {
                break;
            }
        }
    }

    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    apps
}

#[tauri::command]
pub fn launch_program(program: String, args: String) -> Result<(), String> {
    let program = program.trim();
    if program.is_empty() {
        return Err("Program is empty".to_string());
    }

    let mut command = std::process::Command::new(program);
    let args = args.trim();
    if !args.is_empty() {
        use std::os::windows::process::CommandExt;
        command.raw_arg(args);
    }

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not start program: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_executables_and_skips_junk() {
        let root = std::env::temp_dir().join(format!("v2rayN-scan-{}", std::process::id()));
        let app_dir = root.join("v2rayN-windows-64");
        let helper_dir = root.join("LDPlayer");
        std::fs::create_dir_all(&app_dir).expect("create app dir");
        std::fs::create_dir_all(&helper_dir).expect("create helper dir");

        std::fs::write(app_dir.join("v2rayN.exe"), b"").expect("write exe");
        std::fs::write(app_dir.join("uninstall.exe"), b"").expect("write uninstaller");
        std::fs::write(helper_dir.join("adb.exe"), b"").expect("write helper");
        std::fs::write(root.join("notes.txt"), b"").expect("write text");

        let apps = scan_folder_apps(root.to_string_lossy().to_string());
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "v2rayN");
        assert!(apps[0].path.ends_with("v2rayN.exe"));
    }

    #[test]
    #[ignore = "reads a real folder; set NEXORA_SCAN_DIR"]
    fn scans_a_real_folder() {
        let Ok(folder) = std::env::var("NEXORA_SCAN_DIR") else {
            return;
        };

        let apps = scan_folder_apps(folder);
        for app in &apps {
            println!("app: {} -> {}", app.name, app.path);
        }
        assert!(!apps.is_empty(), "no executables found");
    }
}
