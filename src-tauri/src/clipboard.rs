use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use windows::Win32::{
    Foundation::{GlobalFree, HGLOBAL},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
            IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
        Ole::CF_UNICODETEXT,
    },
};

use crate::config::EVENT_CLIPBOARD_UPDATED;

const MAX_CLIPBOARD_ITEMS: usize = 200;
const MAX_CLIPBOARD_CHARS: usize = 32_000;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEntry {
    pub id: String,
    pub text: String,
    pub created_at: i64,
    pub pinned: bool,
}

#[derive(Debug)]
struct ClipboardStore {
    entries: Vec<ClipboardEntry>,
    max_items: usize,
}

impl ClipboardStore {
    fn new(max_items: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_items: max_items.clamp(1, MAX_CLIPBOARD_ITEMS),
        }
    }

    fn entries(&self) -> &[ClipboardEntry] {
        &self.entries
    }

    fn record(&mut self, text: &str, created_at: i64) -> bool {
        if text.trim().is_empty() {
            return false;
        }

        let text = text.chars().take(MAX_CLIPBOARD_CHARS).collect::<String>();
        self.entries.retain(|entry| entry.text != text);

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        created_at.hash(&mut hasher);

        self.entries.insert(
            0,
            ClipboardEntry {
                id: format!("{created_at:x}-{:x}", hasher.finish()),
                text,
                created_at,
                pinned: false,
            },
        );

        while self.entries.len() > self.max_items {
            if let Some(index) = self.entries.iter().rposition(|entry| !entry.pinned) {
                self.entries.remove(index);
            } else {
                self.entries.pop();
            }
        }

        true
    }

    fn toggle_pin(&mut self, id: &str) -> bool {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            entry.pinned = !entry.pinned;
            return true;
        }
        false
    }

    fn remove(&mut self, id: &str) -> bool {
        let previous_len = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != previous_len
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Clone)]
pub struct ClipboardHistory {
    store: Arc<Mutex<ClipboardStore>>,
    path: PathBuf,
    capture_enabled: Arc<AtomicBool>,
}

impl ClipboardHistory {
    pub fn new(path: PathBuf) -> Self {
        let mut store = ClipboardStore::new(MAX_CLIPBOARD_ITEMS);
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(entries) = serde_json::from_str::<Vec<ClipboardEntry>>(&raw) {
                for entry in entries.into_iter().rev() {
                    if store.record(&entry.text, entry.created_at) && entry.pinned {
                        let id = store.entries[0].id.clone();
                        store.toggle_pin(&id);
                    }
                }
            }
        }

        Self {
            store: Arc::new(Mutex::new(store)),
            path,
            capture_enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    pub(crate) fn list(&self) -> Vec<ClipboardEntry> {
        self.store
            .lock()
            .map(|store| store.entries().to_vec())
            .unwrap_or_default()
    }

    fn record_text(&self, text: &str) -> bool {
        let changed = self
            .store
            .lock()
            .map(|mut store| store.record(text, now_millis()))
            .unwrap_or(false);
        if changed {
            self.persist();
        }
        changed
    }

    pub(crate) fn toggle_pin(&self, id: &str) -> bool {
        let changed = self
            .store
            .lock()
            .map(|mut store| store.toggle_pin(id))
            .unwrap_or(false);
        if changed {
            self.persist();
        }
        changed
    }

    pub(crate) fn remove(&self, id: &str) -> bool {
        let changed = self
            .store
            .lock()
            .map(|mut store| store.remove(id))
            .unwrap_or(false);
        if changed {
            self.persist();
        }
        changed
    }

    pub(crate) fn clear(&self) {
        if let Ok(mut store) = self.store.lock() {
            store.clear();
        }
        self.persist();
    }

    pub(crate) fn set_capture_enabled(&self, enabled: bool) {
        self.capture_enabled.store(enabled, Ordering::Relaxed);
    }

    fn capture_enabled(&self) -> bool {
        self.capture_enabled.load(Ordering::Relaxed)
    }

    fn persist(&self) {
        let entries = self.list();
        if let Ok(json) = serde_json::to_vec(&entries) {
            if let Some(parent) = self.path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&self.path, json);
        }
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

fn notify(app: &AppHandle) {
    let _ = app.emit(EVENT_CLIPBOARD_UPDATED, ());
}

pub fn start_monitor(app: AppHandle, history: ClipboardHistory) {
    thread::spawn(move || {
        let mut sequence = unsafe { GetClipboardSequenceNumber() };

        loop {
            thread::sleep(Duration::from_millis(400));
            if !history.capture_enabled() {
                continue;
            }

            let next_sequence = unsafe { GetClipboardSequenceNumber() };
            if next_sequence == sequence {
                continue;
            }

            match read_clipboard_text() {
                Ok(text) => {
                    sequence = next_sequence;
                    if let Some(text) = text {
                        if history.record_text(&text) {
                            notify(&app);
                        }
                    }
                }
                Err(()) => {
                    // Another application can briefly own the clipboard.
                    // Retry on the next poll without advancing the sequence.
                }
            }
        }
    });
}

fn read_clipboard_text() -> Result<Option<String>, ()> {
    unsafe {
        OpenClipboard(None).map_err(|_| ())?;

        let result = if IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).is_err() {
            Ok(None)
        } else {
            let handle = GetClipboardData(CF_UNICODETEXT.0 as u32).map_err(|_| ())?;
            let memory = HGLOBAL(handle.0);
            let pointer = GlobalLock(memory) as *const u16;
            if pointer.is_null() {
                Err(())
            } else {
                let units = GlobalSize(memory) / std::mem::size_of::<u16>();
                let slice = std::slice::from_raw_parts(pointer, units);
                let length = slice
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(slice.len());
                let text = String::from_utf16_lossy(&slice[..length]);
                let _ = GlobalUnlock(memory);
                Ok((!text.trim().is_empty()).then_some(text))
            }
        };

        let _ = CloseClipboard();
        result
    }
}

pub(crate) fn write_clipboard_text(text: &str) -> Result<(), String> {
    let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = units.len() * std::mem::size_of::<u16>();

    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|error| error.to_string())?;
        let pointer = GlobalLock(memory) as *mut u16;
        if pointer.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err("Could not lock clipboard memory".to_string());
        }
        std::ptr::copy_nonoverlapping(units.as_ptr(), pointer, units.len());
        let _ = GlobalUnlock(memory);

        if OpenClipboard(None).is_err() {
            let _ = GlobalFree(Some(memory));
            return Err("Could not open the clipboard".to_string());
        }
        if let Err(error) = EmptyClipboard() {
            let _ = CloseClipboard();
            let _ = GlobalFree(Some(memory));
            return Err(error.to_string());
        }

        let handle = windows::Win32::Foundation::HANDLE(memory.0);
        if let Err(error) = SetClipboardData(CF_UNICODETEXT.0 as u32, Some(handle)) {
            let _ = CloseClipboard();
            let _ = GlobalFree(Some(memory));
            return Err(error.to_string());
        }

        let _ = CloseClipboard();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_new_text_first_and_deduplicates_existing_text() {
        let mut store = ClipboardStore::new(3);

        assert!(store.record("first", 10));
        assert!(store.record("second", 20));
        assert!(store.record("first", 30));

        let entries = store.entries();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.text.as_str())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
        assert_eq!(entries[0].created_at, 30);
    }

    #[test]
    fn ignores_blank_text_and_keeps_pinned_entries_when_trimming() {
        let mut store = ClipboardStore::new(3);
        store.record("pinned", 10);
        let pinned_id = store.entries()[0].id.clone();
        store.toggle_pin(&pinned_id);
        store.record("old", 20);
        store.record("middle", 30);
        store.record("new", 40);
        store.record("   \n ", 50);

        assert_eq!(
            store
                .entries()
                .iter()
                .map(|entry| entry.text.as_str())
                .collect::<Vec<_>>(),
            ["new", "middle", "pinned"]
        );
    }

    #[test]
    fn removes_individual_entries_and_clears_history() {
        let mut store = ClipboardStore::new(5);
        store.record("keep", 10);
        let keep_id = store.entries()[0].id.clone();
        store.toggle_pin(&keep_id);
        store.record("remove", 20);
        let remove_id = store.entries()[0].id.clone();

        assert!(store.remove(&remove_id));
        assert!(!store.remove(&remove_id));

        store.record("temporary", 30);
        store.clear();

        assert!(store.entries().is_empty());
    }

    #[test]
    fn persists_entries_and_pin_state_between_instances() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join(format!("nexora-clipboard-{}", std::process::id()));
        let path = dir.join("clipboard.json");
        std::fs::create_dir_all(&dir).expect("create clipboard test dir");

        let first = ClipboardHistory::new(path.clone());
        first.record_text("remember me");
        let id = first.list()[0].id.clone();
        first.toggle_pin(&id);
        drop(first);

        let restored = ClipboardHistory::new(path);
        let entries = restored.list();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "remember me");
        assert!(entries[0].pinned);

        std::fs::remove_dir_all(&dir).ok();
    }
}
