//! Isolated storage paths for NEXORA ISLE.

use std::ffi::OsString;
use std::path::PathBuf;

use super::identity::{DATA_ENVIRONMENT_VARIABLE, DISPLAY_NAME};

pub const LEGACY_DATA_ENVIRONMENT_VARIABLE: &str = "NEXORA_HALO_DATA_DIR";

fn resolve_data_dir(
    appdata: PathBuf,
    current_override: Option<OsString>,
    legacy_override: Option<OsString>,
) -> PathBuf {
    current_override
        .or(legacy_override)
        .map(PathBuf::from)
        .unwrap_or_else(|| appdata.join("NEXORA").join("Isle"))
}

fn migrate_legacy_data_dir(current: PathBuf, legacy: PathBuf) -> PathBuf {
    if current.exists() || !legacy.exists() {
        return current;
    }
    if let Some(parent) = current.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return legacy;
        }
    }
    if std::fs::rename(&legacy, &current).is_ok() {
        current
    } else {
        // A previous process may still hold files in the legacy directory.
        // Keep using it so settings remain available, then retry on next launch.
        legacy
    }
}

pub fn data_dir() -> PathBuf {
    let current_override = std::env::var_os(DATA_ENVIRONMENT_VARIABLE);
    let legacy_override = std::env::var_os(LEGACY_DATA_ENVIRONMENT_VARIABLE);
    let overridden = current_override.is_some() || legacy_override.is_some();
    let appdata = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let resolved = resolve_data_dir(appdata, current_override, legacy_override);
    if overridden {
        resolved
    } else {
        migrate_legacy_data_dir(resolved.clone(), resolved.with_file_name("Halo"))
    }
}

pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let directory = data_dir();
    std::fs::create_dir_all(&directory)?;
    Ok(directory)
}

pub fn diagnostic_label() -> &'static str {
    DISPLAY_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nexora-isle-data-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn isle_data_override_wins_over_the_legacy_override() {
        assert_eq!(
            resolve_data_dir(
                PathBuf::from("C:\\Users\\Test\\AppData\\Roaming"),
                Some(OsString::from("C:\\IsleData")),
                Some(OsString::from("C:\\LegacyHaloData")),
            ),
            PathBuf::from("C:\\IsleData")
        );
    }

    #[test]
    fn legacy_override_remains_supported() {
        assert_eq!(
            resolve_data_dir(
                PathBuf::from("C:\\Users\\Test\\AppData\\Roaming"),
                None,
                Some(OsString::from("C:\\LegacyHaloData")),
            ),
            PathBuf::from("C:\\LegacyHaloData")
        );
    }

    #[test]
    fn default_data_directory_uses_isle() {
        assert_eq!(
            resolve_data_dir(
                PathBuf::from("C:\\Users\\Test\\AppData\\Roaming"),
                None,
                None
            ),
            PathBuf::from("C:\\Users\\Test\\AppData\\Roaming\\NEXORA\\Isle")
        );
    }

    #[test]
    fn legacy_data_directory_is_migrated_when_possible() {
        let base = temp_path("migration");
        let current = base.join("Isle");
        let legacy = base.join("Halo");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("settings.ini"), b"enabled=1").unwrap();

        assert_eq!(
            migrate_legacy_data_dir(current.clone(), legacy.clone()),
            current
        );
        assert!(current.join("settings.ini").exists());
        let _ = std::fs::remove_dir_all(base);
    }
}
