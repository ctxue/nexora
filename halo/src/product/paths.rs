//! Isolated storage paths for NEXORA HALO.

use std::path::PathBuf;

use super::identity::{DATA_ENVIRONMENT_VARIABLE, DISPLAY_NAME};

pub fn data_dir() -> PathBuf {
    if let Some(root) = std::env::var_os(DATA_ENVIRONMENT_VARIABLE) {
        return PathBuf::from(root);
    }

    let appdata = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    appdata.join("NEXORA").join("Halo")
}

pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let directory = data_dir();
    std::fs::create_dir_all(&directory)?;
    Ok(directory)
}

pub fn diagnostic_label() -> &'static str {
    DISPLAY_NAME
}
