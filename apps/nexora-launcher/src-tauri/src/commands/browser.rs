use std::path::PathBuf;

use serde::Serialize;
use tauri::Manager;
use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
use winreg::RegKey;

use crate::browser_bridge::{queue_tab_activation, EXTENSION_ID, NATIVE_HOST_NAME};
use crate::browser_data::{read_browser_entries, BrowserEntry};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserBridgeStatus {
    pub extension_id: String,
    pub extension_dir: String,
    pub installed: bool,
    pub snapshot_available: bool,
}

#[tauri::command]
pub async fn list_browser_entries(
    bookmarks: bool,
    history: bool,
    tabs: bool,
) -> Result<Vec<BrowserEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || read_browser_entries(bookmarks, history, tabs))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn activate_browser_tab(tab_id: i64) -> Result<(), String> {
    queue_tab_activation(tab_id)
}

fn extension_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resource = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?
        .join("browser-extension");
    if resource.is_dir() {
        return Ok(resource);
    }
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|root| root.join("browser-extension"))
        .ok_or_else(|| "Browser extension directory not found".to_string())?;
    if source.is_dir() {
        Ok(source)
    } else {
        Err("Browser extension directory not found".to_string())
    }
}

fn bridge_manifest_path() -> Result<PathBuf, String> {
    dirs::data_local_dir()
        .map(|path| {
            path.join("NEXORA")
                .join("browser-bridge")
                .join(format!("{NATIVE_HOST_NAME}.json"))
        })
        .ok_or_else(|| "data directory unavailable".to_string())
}

fn register_native_host(key_path: &str, manifest: &str) -> Result<(), String> {
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = current_user
        .create_subkey_with_flags(key_path, KEY_WRITE)
        .map_err(|error| error.to_string())?;
    key.set_value("", &manifest.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn install_browser_bridge() -> Result<BrowserBridgeStatus, String> {
    let manifest_path = bridge_manifest_path()?;
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let executable =
        std::env::current_exe().map_err(|error| format!("Could not locate NEXORA: {error}"))?;
    let manifest = serde_json::json!({
        "name": NATIVE_HOST_NAME,
        "description": "NEXORA Browser Companion",
        "path": executable.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{EXTENSION_ID}/")]
    });
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let manifest_value = manifest_path.to_string_lossy().to_string();
    register_native_host(
        &format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{NATIVE_HOST_NAME}"),
        &manifest_value,
    )?;
    register_native_host(
        &format!(r"Software\Google\Chrome\NativeMessagingHosts\{NATIVE_HOST_NAME}"),
        &manifest_value,
    )?;
    Ok(BrowserBridgeStatus {
        extension_id: EXTENSION_ID.to_string(),
        extension_dir: String::new(),
        installed: true,
        snapshot_available: bridge_manifest_path()?.exists(),
    })
}

#[tauri::command]
pub fn browser_bridge_status(app: tauri::AppHandle) -> Result<BrowserBridgeStatus, String> {
    let extension_dir = extension_directory(&app)?;
    let installed = bridge_manifest_path()?.exists();
    let snapshot_available = dirs::data_local_dir()
        .map(|path| {
            path.join("NEXORA")
                .join("browser-bridge")
                .join("tabs.json")
                .exists()
        })
        .unwrap_or(false);
    Ok(BrowserBridgeStatus {
        extension_id: EXTENSION_ID.to_string(),
        extension_dir: extension_dir.to_string_lossy().to_string(),
        installed,
        snapshot_available,
    })
}

#[tauri::command]
pub fn open_browser_extension_dir(app: tauri::AppHandle) -> Result<(), String> {
    let directory = extension_directory(&app)?;
    open::that(directory).map_err(|error| error.to_string())
}
