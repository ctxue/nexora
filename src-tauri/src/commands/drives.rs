//! Logical drives, so a query like `D`, `D:` or `D://` can open the drive.

use serde::Serialize;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};

// GetDriveTypeW returns one of these values (winbase.h). Only the kinds that
// can be opened as a folder are exposed; empty optical drives are skipped.
const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;
const DRIVE_RAMDISK: u32 = 6;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveEntry {
    /// Drive letter without the colon, e.g. `D`.
    pub letter: String,
    /// Root path, e.g. `D:\`.
    pub path: String,
    /// `fixed` / `removable` / `remote` / `ramdisk`.
    pub kind: String,
}

fn drive_kind(value: u32) -> Option<&'static str> {
    match value {
        DRIVE_REMOVABLE => Some("removable"),
        DRIVE_FIXED => Some("fixed"),
        DRIVE_REMOTE => Some("remote"),
        DRIVE_RAMDISK => Some("ramdisk"),
        _ => None,
    }
}

#[tauri::command]
pub fn list_drives() -> Vec<DriveEntry> {
    let mask = unsafe { GetLogicalDrives() };
    let mut drives = Vec::new();

    for index in 0..26u32 {
        if mask & (1 << index) == 0 {
            continue;
        }

        let letter = char::from(b'A' + index as u8);
        let path = format!("{letter}:\\");
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let drive_type = unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) };

        let Some(kind) = drive_kind(drive_type) else {
            continue;
        };

        drives.push(DriveEntry {
            letter: letter.to_string(),
            path,
            kind: kind.to_string(),
        });
    }

    drives
}
