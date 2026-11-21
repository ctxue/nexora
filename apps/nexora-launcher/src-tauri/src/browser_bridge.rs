use std::io::{Read, Write};
use std::path::PathBuf;

pub const NATIVE_HOST_NAME: &str = "com.nexora.bridge";
pub const EXTENSION_ID: &str = "hgehbplobdpkmacjkbmglbmijdjnembd";

fn bridge_directory() -> Option<PathBuf> {
    dirs::data_local_dir().map(|path| path.join("NEXORA").join("browser-bridge"))
}

fn write_atomic(path: &std::path::Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, contents).map_err(|error| error.to_string())?;
    std::fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn read_message<R: Read>(reader: &mut R) -> Result<Option<serde_json::Value>, String> {
    let mut length = [0u8; 4];
    match reader.read_exact(&mut length) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.to_string()),
    }
    let length = u32::from_le_bytes(length) as usize;
    if length > 16 * 1024 * 1024 {
        return Err("Browser bridge message is too large".to_string());
    }
    let mut payload = vec![0u8; length];
    reader
        .read_exact(&mut payload)
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&payload)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn write_message<W: Write>(writer: &mut W, value: &serde_json::Value) -> Result<(), String> {
    let payload = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let length = u32::try_from(payload.len()).map_err(|_| "Bridge response is too large")?;
    writer
        .write_all(&length.to_le_bytes())
        .and_then(|_| writer.write_all(&payload))
        .and_then(|_| writer.flush())
        .map_err(|error| error.to_string())
}

fn handle_message(message: serde_json::Value) -> serde_json::Value {
    let Some(directory) = bridge_directory() else {
        return serde_json::json!({ "type": "error", "error": "data directory unavailable" });
    };
    let message_type = message.get("type").and_then(|value| value.as_str());

    match message_type {
        Some("snapshot") => {
            let payload = serde_json::to_vec(&message).unwrap_or_default();
            if let Err(error) = write_atomic(&directory.join("tabs.json"), &payload) {
                return serde_json::json!({ "type": "error", "error": error });
            }
            serde_json::json!({ "type": "ack" })
        }
        Some("poll") => {
            let command_path = directory.join("command.json");
            let command = std::fs::read(&command_path)
                .ok()
                .and_then(|raw| serde_json::from_slice::<serde_json::Value>(&raw).ok());
            if command.is_some() {
                std::fs::remove_file(command_path).ok();
            }
            serde_json::json!({ "type": "command", "command": command })
        }
        _ => serde_json::json!({ "type": "error", "error": "unsupported message" }),
    }
}

pub fn run_native_host_if_requested() -> bool {
    if !std::env::args().skip(1).any(|argument| {
        argument == "--native-messaging" || argument.starts_with("chrome-extension://")
    }) {
        return false;
    }

    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    while let Ok(Some(message)) = read_message(&mut stdin) {
        if write_message(&mut stdout, &handle_message(message)).is_err() {
            break;
        }
    }
    true
}

pub fn queue_tab_activation(tab_id: i64) -> Result<(), String> {
    let directory = bridge_directory().ok_or_else(|| "data directory unavailable".to_string())?;
    let payload = serde_json::to_vec(&serde_json::json!({
        "type": "activateTab",
        "tabId": tab_id
    }))
    .map_err(|error| error.to_string())?;
    write_atomic(&directory.join("command.json"), &payload)
}
