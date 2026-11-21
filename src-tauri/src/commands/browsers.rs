use std::path::{Path, PathBuf};

use serde::Serialize;
use winreg::{enums::*, RegKey};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserInfo {
    pub id: String,
    pub name: String,
    pub exe: String,
}

fn command_executable(command: &str) -> Option<String> {
    let command = command.trim();
    let exe = if let Some(rest) = command.strip_prefix('"') {
        rest.split('"').next()?
    } else {
        command.split_whitespace().next()?
    };

    let exe = exe.trim();
    if exe.to_lowercase().ends_with(".exe") {
        Some(exe.to_string())
    } else {
        None
    }
}

fn push_unique(browsers: &mut Vec<BrowserInfo>, exe: &str, name: &str, id: &str) {
    if !Path::new(exe).exists() {
        return;
    }
    if browsers
        .iter()
        .any(|browser| browser.exe.eq_ignore_ascii_case(exe))
    {
        return;
    }

    browsers.push(BrowserInfo {
        id: id.to_string(),
        name: name.to_string(),
        exe: exe.to_string(),
    });
}

pub fn browsers() -> Vec<BrowserInfo> {
    let mut found = Vec::new();

    for hive in [
        RegKey::predef(HKEY_LOCAL_MACHINE),
        RegKey::predef(HKEY_CURRENT_USER),
    ] {
        let Ok(clients) =
            hive.open_subkey_with_flags(r"SOFTWARE\Clients\StartMenuInternet", KEY_READ)
        else {
            continue;
        };

        for id in clients.enum_keys().flatten() {
            let Ok(client) = clients.open_subkey_with_flags(&id, KEY_READ) else {
                continue;
            };

            let name = client
                .get_value::<String, _>("")
                .or_else(|_| client.get_value::<String, _>("LocalizedString"))
                .ok()
                .filter(|value| !value.trim().is_empty() && !value.starts_with('@'))
                .unwrap_or_else(|| id.clone());

            let Ok(command) = client.open_subkey_with_flags(r"shell\open\command", KEY_READ) else {
                continue;
            };
            let Ok(raw) = command.get_value::<String, _>("") else {
                continue;
            };
            let Some(exe) = command_executable(&raw) else {
                continue;
            };

            if exe.to_lowercase().ends_with("iexplore.exe") {
                continue;
            }

            push_unique(&mut found, &exe, name.trim(), &id);
        }
    }

    for (name, path) in [
        (
            "Microsoft Edge",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        ),
        (
            "Microsoft Edge",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ),
        (
            "Google Chrome",
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        ),
        (
            "Google Chrome",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ),
        (
            "Mozilla Firefox",
            r"C:\Program Files\Mozilla Firefox\firefox.exe",
        ),
    ] {
        push_unique(&mut found, path, name, path);
    }

    found.sort_by_key(|a| a.name.to_lowercase());
    found
}

fn is_web_url(url: &str) -> bool {
    let lower = url.trim().to_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

#[tauri::command]
pub fn open_url(url: String, browser: Option<String>, new_window: bool) -> Result<(), String> {
    let url = url.trim().to_string();
    if !is_web_url(&url) {
        return Err("Only http(s) URLs can be opened".to_string());
    }

    let Some(exe) = browser.filter(|value| !value.trim().is_empty() && value != "default") else {
        return open::that(&url).map_err(|error| format!("Could not open URL: {error}"));
    };

    let path = PathBuf::from(&exe);
    if !path.exists() {
        return Err(format!("Browser not found: {exe}"));
    }

    let mut command = std::process::Command::new(&path);
    if new_window {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_lowercase();
        command.arg(if file_name == "firefox.exe" {
            "-new-window"
        } else {
            "--new-window"
        });
    }
    command.arg(&url);

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not launch browser: {error}"))
}

#[tauri::command]
pub fn list_browsers() -> Vec<BrowserInfo> {
    browsers()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registry_command_lines() {
        assert_eq!(
            command_executable(
                r#""C:\Program Files\Microsoft\Edge\Application\msedge.exe" --single-argument %1"#
            ),
            Some(r"C:\Program Files\Microsoft\Edge\Application\msedge.exe".to_string())
        );
        assert_eq!(
            command_executable(r#""C:\Firefox\firefox.exe" -osint -url "%1""#),
            Some(r"C:\Firefox\firefox.exe".to_string())
        );
        assert_eq!(command_executable("not-an-exe --flag"), None);
    }

    #[test]
    fn discovers_installed_browsers() {
        let found = browsers();
        for browser in &found {
            println!("browser: {} -> {}", browser.name, browser.exe);
        }
        assert!(
            !found.is_empty(),
            "expected at least one registered browser"
        );
    }
}
