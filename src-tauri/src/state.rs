//! Long-lived state managed by Tauri.

use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Instant,
};

use crate::models::FileEntry;

/// In-memory file index plus its indexing flag.
#[derive(Clone, Default)]
pub struct FileIndex {
    pub files: Arc<Mutex<Vec<FileEntry>>>,
    pub indexing: Arc<AtomicBool>,
}

/// Launcher window bookkeeping.
#[derive(Default)]
pub struct LauncherState {
    pub shown_at: Mutex<Option<Instant>>,
    pub last_toggle: Mutex<Option<Instant>>,
    pub dragging: Mutex<bool>,
}

/// Currently registered global shortcut and the last error, if any.
#[derive(Default)]
pub struct ActiveShortcut {
    pub current: Mutex<Option<String>>,
    pub error: Mutex<Option<String>>,
}
