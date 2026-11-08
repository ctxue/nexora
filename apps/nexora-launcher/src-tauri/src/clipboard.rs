//! Local clipboard history protection and persistence.
//!
//! Persisted text is encrypted with current-user DPAPI. Sensitive-looking
//! content is filtered before it reaches the history store.

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
use windows::core::{w, PCWSTR};
use windows::Win32::{
    Foundation::{GlobalFree, LocalFree, HGLOBAL, HLOCAL},
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
            IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
        Ole::CF_UNICODETEXT,
    },
};

use crate::config::EVENT_CLIPBOARD_UPDATED;

const MAX_CLIPBOARD_ITEMS: usize = 200;
const MAX_CLIPBOARD_CHARS: usize = 32_000;
const CLIPBOARD_FILE_MAGIC: &[u8] = b"NEXORA-CLIPBOARD\x01";

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
        if text.trim().is_empty() || contains_sensitive_content(text) {
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
        let legacy_path = path.with_file_name("clipboard-history.json");
        let source_path = if path.exists() {
            Some(path.clone())
        } else if legacy_path.exists() {
            Some(legacy_path.clone())
        } else {
            None
        };
        let mut loaded_plaintext = false;
        let mut loaded_encrypted = false;

        if let Some(source_path) = source_path.as_ref() {
            if let Ok(raw) = fs::read(source_path) {
                let plaintext = if raw.starts_with(CLIPBOARD_FILE_MAGIC) {
                    let plaintext = dpapi_unprotect(&raw[CLIPBOARD_FILE_MAGIC.len()..]).ok();
                    loaded_encrypted = plaintext.is_some();
                    plaintext
                } else {
                    loaded_plaintext = true;
                    Some(raw)
                };

                if let Some(plaintext) = plaintext {
                    if let Ok(entries) = serde_json::from_slice::<Vec<ClipboardEntry>>(&plaintext) {
                        for entry in entries.into_iter().rev() {
                            if store.record(&entry.text, entry.created_at) && entry.pinned {
                                let id = store.entries[0].id.clone();
                                store.toggle_pin(&id);
                            }
                        }
                    }
                }
            }
        }

        let history = Self {
            store: Arc::new(Mutex::new(store)),
            path,
            capture_enabled: Arc::new(AtomicBool::new(false)),
        };

        if let Some(source_path) = source_path.as_ref() {
            let migrated_from_legacy = source_path != &history.path;
            if loaded_plaintext || migrated_from_legacy {
                let persisted = history.persist().is_ok();
                if persisted && migrated_from_legacy {
                    let _ = fs::remove_file(source_path);
                }
            }
        }

        if loaded_encrypted && legacy_path != history.path && legacy_path.exists() {
            let _ = fs::remove_file(&legacy_path);
        }

        history
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
            let _ = self.persist();
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
            let _ = self.persist();
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
            let _ = self.persist();
        }
        changed
    }

    pub(crate) fn clear(&self) {
        if let Ok(mut store) = self.store.lock() {
            store.clear();
        }
        let _ = self.persist();
    }

    pub(crate) fn set_capture_enabled(&self, enabled: bool) {
        self.capture_enabled.store(enabled, Ordering::Relaxed);
    }

    fn capture_enabled(&self) -> bool {
        self.capture_enabled.load(Ordering::Relaxed)
    }

    fn persist(&self) -> Result<(), String> {
        let entries = self.list();
        let json = serde_json::to_vec(&entries).map_err(|error| error.to_string())?;
        let encrypted = dpapi_protect(&json)?;
        let mut payload = Vec::with_capacity(CLIPBOARD_FILE_MAGIC.len() + encrypted.len());
        payload.extend_from_slice(CLIPBOARD_FILE_MAGIC);
        payload.extend_from_slice(&encrypted);
        write_file_atomically(&self.path, &payload)
    }
}

fn dpapi_protect(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(plaintext.len()).map_err(|_| "Clipboard history is too large")?,
        pbData: plaintext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptProtectData(
            &input,
            PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|error| format!("Could not encrypt clipboard history: {error}"))?;
        Ok(copy_and_free_blob(output))
    }
}

fn dpapi_unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(ciphertext.len()).map_err(|_| "Clipboard history is too large")?,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|error| format!("Could not decrypt clipboard history: {error}"))?;
        Ok(copy_and_free_blob(output))
    }
}

unsafe fn copy_and_free_blob(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    if blob.pbData.is_null() {
        return Vec::new();
    }

    let bytes = unsafe { std::slice::from_raw_parts(blob.pbData, blob.cbData as usize) }.to_vec();
    let _ = unsafe { LocalFree(Some(HLOCAL(blob.pbData.cast()))) };
    bytes
}

fn write_file_atomically(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let temp_path = path.with_extension(format!("tmp-{}-{}", std::process::id(), now_millis()));
    let result = (|| {
        let mut file = fs::File::create(&temp_path).map_err(|error| error.to_string())?;
        use std::io::Write;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| error.to_string())?;
        fs::rename(&temp_path, path).map_err(|error| error.to_string())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn contains_sensitive_content(text: &str) -> bool {
    let lower = text.to_lowercase();

    if lower.contains("-----begin private key-----")
        || lower.contains("-----begin rsa private key-----")
        || lower.contains("-----begin ec private key-----")
        || lower.contains("-----begin openssh private key-----")
    {
        return true;
    }

    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower_compact = compact.to_lowercase();
    let providers = [
        "sk-ant-",
        "sk-proj-",
        "sk-live-",
        "sk_live_",
        "rk_live_",
        "ghp_",
        "gho_",
        "ghu_",
        "ghs_",
        "ghr_",
        "github_pat_",
        "xoxb-",
        "xoxp-",
        "xoxa-",
        "xoxr-",
        "akia",
        "asia",
        "aiza",
    ];

    if providers.iter().any(|prefix| {
        lower_compact
            .split(|character: char| {
                !character.is_ascii_alphanumeric() && character != '_' && character != '-'
            })
            .any(|token| token.starts_with(prefix) && token.len() >= prefix.len() + 12)
    }) {
        return true;
    }

    if compact.split_whitespace().any(is_json_web_token) {
        return true;
    }

    if lower.contains("authorization: bearer ") || has_uri_credentials(&compact) {
        return true;
    }

    has_secret_assignment(&lower)
}

fn is_json_web_token(value: &str) -> bool {
    let value = value.trim_matches(|character: char| {
        !character.is_ascii_alphanumeric()
            && character != '.'
            && character != '_'
            && character != '-'
    });
    let mut parts = value.split('.');
    let segments = [
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
    ];

    parts.next().is_none()
        && segments.iter().all(|segment| {
            segment.len() >= 8
                && segment
                    .chars()
                    .all(|value| value.is_ascii_alphanumeric() || value == '_' || value == '-')
        })
        && segments[0].starts_with("eyJ")
}

fn has_secret_assignment(text: &str) -> bool {
    let sensitive_keys = [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "access_key",
        "private_key",
        "client_secret",
        "密码",
        "密钥",
        "令牌",
        "口令",
        "私钥",
    ];

    text.lines().any(|line| {
        let Some((key, value)) = line.split_once(['=', ':', '＝', '：']) else {
            return false;
        };
        let key = key
            .trim()
            .trim_matches(|character: char| {
                matches!(
                    character,
                    '"' | '\'' | '-' | '{' | '}' | '[' | ']' | '(' | ')' | ','
                )
            })
            .trim();
        let value = value
            .trim()
            .trim_matches(|character: char| matches!(character, '"' | '\'' | ',' | ';'))
            .trim();

        (sensitive_keys.contains(&key)
            || sensitive_keys.iter().any(|candidate| {
                if !candidate.is_ascii() {
                    return key.ends_with(candidate);
                }
                key.strip_suffix(candidate)
                    .is_some_and(|prefix| prefix.is_empty() || prefix.ends_with(['_', '.', '-']))
            }))
            && value.chars().count() >= 6
    })
}

fn has_uri_credentials(text: &str) -> bool {
    text.split_whitespace().any(|value| {
        let Some((_, remainder)) = value.split_once("://") else {
            return false;
        };
        let authority = remainder.split('/').next().unwrap_or_default();
        let Some((user_info, _)) = authority.rsplit_once('@') else {
            return false;
        };
        let Some((user, password)) = user_info.split_once(':') else {
            return false;
        };
        !user.is_empty() && password.chars().count() >= 4
    })
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

fn should_record_clipboard(excluded: bool, include_in_history: Option<u32>) -> bool {
    !excluded && include_in_history != Some(0)
}

fn clipboard_units_to_read(byte_len: usize) -> usize {
    (byte_len / std::mem::size_of::<u16>()).min(MAX_CLIPBOARD_CHARS + 1)
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

            if let Ok(text) = read_clipboard_text() {
                sequence = next_sequence;
                if let Some(text) = text {
                    if history.record_text(&text) {
                        notify(&app);
                    }
                }
            }
        }
    });
}

pub(crate) fn read_clipboard_text() -> Result<Option<String>, ()> {
    unsafe {
        OpenClipboard(None).map_err(|_| ())?;

        let excluded_format =
            RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing"));
        let excluded = excluded_format != 0 && IsClipboardFormatAvailable(excluded_format).is_ok();
        let include_format = RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory"));
        let include_in_history = if include_format == 0 {
            None
        } else {
            read_clipboard_dword(include_format)
        };

        let result = if !should_record_clipboard(excluded, include_in_history)
            || IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).is_err()
        {
            Ok(None)
        } else {
            let handle = GetClipboardData(CF_UNICODETEXT.0 as u32).map_err(|_| ())?;
            let memory = HGLOBAL(handle.0);
            let pointer = GlobalLock(memory) as *const u16;
            if pointer.is_null() {
                Err(())
            } else {
                let units = clipboard_units_to_read(GlobalSize(memory));
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

fn read_clipboard_dword(format: u32) -> Option<u32> {
    unsafe {
        if IsClipboardFormatAvailable(format).is_err() {
            return None;
        }
        let handle = GetClipboardData(format).ok()?;
        let memory = HGLOBAL(handle.0);
        let pointer = GlobalLock(memory) as *const u32;
        if pointer.is_null() {
            return None;
        }
        let value = (GlobalSize(memory) >= std::mem::size_of::<u32>()).then(|| pointer.read());
        let _ = GlobalUnlock(memory);
        value
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

    fn test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{name}-{}", std::process::id()))
    }

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
    fn refuses_likely_secrets_before_they_enter_the_store() {
        let mut store = ClipboardStore::new(5);

        assert!(!store.record(
            "OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuvwxyz123456",
            10,
        ));
        assert!(!store.record(
            "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC\n-----END PRIVATE KEY-----",
            20,
        ));
        assert!(!store.record("DATABASE_PASSWORD=hunter2-super-secret", 30));
        assert!(store.record("Meeting notes for Tuesday", 40));

        assert_eq!(
            store
                .entries()
                .iter()
                .map(|entry| entry.text.as_str())
                .collect::<Vec<_>>(),
            ["Meeting notes for Tuesday"]
        );
    }

    #[test]
    fn recognizes_common_secret_shapes_without_blocking_regular_prose() {
        let secrets = [
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature12345",
            "AKIAABCDEFGHIJKLMNOP",
            r#"{"password":"s3cr3t-value"}"#,
            "postgres://user:p@ssw0rd@db.example.com/app",
            "MY_API_KEY=abcdef1234567890",
            "sk_live_abcdefghijklm",
            "数据库密码：Abc123456",
            "API密钥＝abcdef123456",
        ];

        for secret in secrets {
            assert!(
                contains_sensitive_content(secret),
                "expected sensitive content to be rejected: {secret}"
            );
        }
        assert!(!contains_sensitive_content(
            "Meeting notes about API key rotation and password policy"
        ));
    }

    #[test]
    fn starts_with_capture_paused_until_the_user_enables_it() {
        let dir = test_dir("nexora-clipboard-paused");
        let path = dir.join("clipboard-history.bin");
        let history = ClipboardHistory::new(path);

        assert!(!history.capture_enabled());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn respects_windows_clipboard_history_opt_out_formats() {
        assert!(!should_record_clipboard(true, Some(0)));
        assert!(!should_record_clipboard(true, None));
        assert!(should_record_clipboard(false, Some(1)));
        assert!(should_record_clipboard(false, None));
    }

    #[test]
    fn caps_clipboard_reads_before_allocating_oversized_text() {
        assert_eq!(
            clipboard_units_to_read(std::mem::size_of::<u16>() * 10_000_000),
            MAX_CLIPBOARD_CHARS + 1
        );
        assert_eq!(clipboard_units_to_read(std::mem::size_of::<u16>() * 42), 42);
    }

    #[test]
    fn encrypts_clipboard_text_at_rest() {
        let dir = test_dir("nexora-clipboard-encrypted");
        let path = dir.join("clipboard-history.bin");
        std::fs::create_dir_all(&dir).expect("create clipboard test dir");

        let history = ClipboardHistory::new(path.clone());
        assert!(history.record_text("encrypted reminder"));
        let raw = std::fs::read(&path).expect("read encrypted history");

        assert!(
            !String::from_utf8_lossy(&raw).contains("encrypted reminder"),
            "plaintext must not remain in the history file"
        );

        let restored = ClipboardHistory::new(path);
        assert_eq!(restored.list()[0].text, "encrypted reminder");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrates_legacy_plaintext_history_and_drops_secrets() {
        let dir = test_dir("nexora-clipboard-migration");
        let legacy_path = dir.join("clipboard-history.json");
        let encrypted_path = dir.join("clipboard-history.bin");
        std::fs::create_dir_all(&dir).expect("create clipboard test dir");
        std::fs::write(
            &legacy_path,
            serde_json::to_vec(&vec![
                ClipboardEntry {
                    id: "secret".to_string(),
                    text: "GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz123456".to_string(),
                    created_at: 20,
                    pinned: false,
                },
                ClipboardEntry {
                    id: "safe".to_string(),
                    text: "Keep the release checklist".to_string(),
                    created_at: 10,
                    pinned: true,
                },
            ])
            .expect("serialize legacy history"),
        )
        .expect("write legacy history");

        let restored = ClipboardHistory::new(encrypted_path.clone());
        let entries = restored.list();
        let encrypted = std::fs::read(&encrypted_path).expect("read encrypted history");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Keep the release checklist");
        assert!(entries[0].pinned);
        assert!(!legacy_path.exists());
        assert!(!String::from_utf8_lossy(&encrypted).contains("ghp_"));
        assert!(!String::from_utf8_lossy(&encrypted).contains("Keep the release checklist"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn removes_stale_plaintext_after_encrypted_history_exists() {
        let dir = test_dir("nexora-clipboard-stale");
        let legacy_path = dir.join("clipboard-history.json");
        let encrypted_path = dir.join("clipboard-history.bin");
        std::fs::create_dir_all(&dir).expect("create clipboard test dir");

        let first = ClipboardHistory::new(encrypted_path.clone());
        first.record_text("safe encrypted entry");
        drop(first);
        std::fs::write(
            &legacy_path,
            r#"[{"id":"old","text":"PASSWORD=old-secret-value","createdAt":1,"pinned":false}]"#,
        )
        .expect("write stale legacy history");

        let restored = ClipboardHistory::new(encrypted_path);

        assert_eq!(restored.list()[0].text, "safe encrypted entry");
        assert!(!legacy_path.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn persists_entries_and_pin_state_between_instances() {
        let dir = test_dir("nexora-clipboard-persist");
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
