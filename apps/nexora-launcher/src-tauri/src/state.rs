use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Instant,
};

use crate::models::FileEntry;

#[derive(Clone, Default)]
pub struct FileIndex {
    pub files: Arc<Mutex<Vec<FileEntry>>>,
    pub indexing: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct LauncherState {
    pub shown_at: Mutex<Option<Instant>>,
    pub last_toggle: Mutex<Option<Instant>>,
    pub dragging: Mutex<bool>,
    pub selected_text: Mutex<String>,
}

#[derive(Default)]
pub struct ActiveShortcut {
    pub current: Mutex<Option<String>>,
    pub error: Mutex<Option<String>>,
}
