//! Clipboard history with explicit privacy boundaries.
//!
//! Text may be persisted locally after the user enables history. Images remain
//! memory-only, disappear on exit, and sensitive clipboard markers are skipped.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const MAX_RECENT: usize = 30;
pub const MAX_CHARS: usize = 20_000;
pub const MAX_PINNED: usize = 20;
pub const MAX_IMAGES: usize = 24;
pub const MAX_PINNED_IMAGES: usize = 12;
pub const MAX_IMAGE_PIXELS: u64 = 40_000_000;
pub const MAX_IMAGE_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clip {
    pub text: String,
    pub at: String,
    pub pinned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageClip {
    pub id: String,
    pub at: String,
    pub width: u32,
    pub height: u32,
    pub pinned: bool,
    pub png: Arc<Vec<u8>>,
}

#[derive(Debug, Default)]
pub struct History {
    pub items: Vec<Clip>,
    pub images: Vec<ImageClip>,
    pub version: u64,
    path: Option<std::path::PathBuf>,
}

impl History {
    pub fn push(&mut self, text: &str, at: &str) -> bool {
        if text.trim().is_empty() || text.chars().count() > MAX_CHARS {
            return false;
        }
        let pinned = match self.items.iter().position(|c| c.text == text) {
            Some(0) => return false,
            Some(i) => self.items.remove(i).pinned,
            None => false,
        };
        self.items.insert(
            0,
            Clip {
                text: text.to_string(),
                at: at.to_string(),
                pinned,
            },
        );
        let mut recent = 0;
        self.items.retain(|c| {
            c.pinned || {
                recent += 1;
                recent <= MAX_RECENT
            }
        });
        self.changed();
        true
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
            self.changed();
        }
    }

    pub fn clear_recent(&mut self) {
        self.items.retain(|c| c.pinned);
        self.changed();
    }

    pub fn pin_to_front(&mut self, index: usize) -> bool {
        let Some(mut item) = self.items.get(index).cloned() else {
            return false;
        };
        let pinned_count = self.items.iter().filter(|entry| entry.pinned).count();
        if !item.pinned && pinned_count >= MAX_PINNED {
            return false;
        }
        self.items.remove(index);
        item.pinned = true;
        self.items.insert(0, item);
        self.changed();
        true
    }

    pub fn unpin(&mut self, index: usize) -> bool {
        let Some(item) = self.items.get_mut(index) else {
            return false;
        };
        if !item.pinned {
            return false;
        }
        item.pinned = false;
        self.changed();
        true
    }

    pub fn push_image(&mut self, image: image::DynamicImage, at: &str) -> bool {
        let (width, height) = (image.width(), image.height());
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
            return false;
        }
        let rgba = image.to_rgba8();
        let digest = ring::digest::digest(&ring::digest::SHA256, rgba.as_raw());
        let id = digest
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let pinned = match self.images.iter().position(|item| item.id == id) {
            Some(0) => return false,
            Some(index) => self.images.remove(index).pinned,
            None => false,
        };
        let mut encoded = std::io::Cursor::new(Vec::new());
        if image
            .write_to(&mut encoded, image::ImageFormat::Png)
            .is_err()
            || encoded.get_ref().len() > MAX_IMAGE_BYTES
        {
            return false;
        }
        self.images.insert(
            0,
            ImageClip {
                id,
                at: at.to_string(),
                width,
                height,
                pinned,
                png: Arc::new(encoded.into_inner()),
            },
        );
        let mut recent = 0;
        self.images.retain(|item| {
            item.pinned || {
                recent += 1;
                recent <= MAX_IMAGES
            }
        });
        self.changed();
        true
    }

    pub fn remove_image(&mut self, index: usize) {
        if index >= self.images.len() {
            return;
        }
        self.images.remove(index);
        self.changed();
    }

    pub fn clear_images(&mut self) {
        self.images.clear();
        self.changed();
    }

    pub fn toggle_image_pin(&mut self, index: usize) -> bool {
        let pinned_count = self.images.iter().filter(|item| item.pinned).count();
        let Some(item) = self.images.get_mut(index) else {
            return false;
        };
        if !item.pinned && pinned_count >= MAX_PINNED_IMAGES {
            return false;
        }
        item.pinned = !item.pinned;
        self.changed();
        true
    }

    fn to_json(&self) -> Vec<u8> {
        let text: Vec<_> = self
            .items
            .iter()
            .map(|c| serde_json::json!({"pinned": c.pinned,"text": c.text, "at": c.at}))
            .collect();
        serde_json::to_vec(&serde_json::json!({"text": text})).unwrap_or_default()
    }

    pub fn from_json(raw: &[u8]) -> Self {
        let value: serde_json::Value =
            serde_json::from_slice(raw).unwrap_or(serde_json::Value::Null);
        let text_rows = if let Some(rows) = value.as_array() {
            rows.clone()
        } else {
            value
                .get("text")
                .and_then(|rows| rows.as_array())
                .cloned()
                .unwrap_or_default()
        };
        let items = text_rows
            .iter()
            .filter_map(|row| {
                Some(Clip {
                    text: row.get("text")?.as_str()?.to_string(),
                    at: row
                        .get("at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    pinned: row.get("pinned").and_then(|v| v.as_bool()).unwrap_or(true),
                })
            })
            .filter(|c| !c.text.is_empty())
            .take(MAX_PINNED + MAX_RECENT)
            .collect();
        Self {
            items,
            images: Vec::new(),
            ..Self::default()
        }
    }

    fn path() -> std::path::PathBuf {
        crate::model::config_dir().join("clipboard.json")
    }

    pub fn load() -> Self {
        Self::load_from(Self::path())
    }

    fn load_from(path: std::path::PathBuf) -> Self {
        let mut history = std::fs::read(&path)
            .map(|raw| Self::from_json(&raw))
            .unwrap_or_default();
        history.path = Some(path);
        history.images.clear();
        history.changed();
        history
    }

    fn changed(&mut self) {
        self.version += 1;
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = crate::model::atomic_write(path, &self.to_json());
        }
    }
}

#[derive(Clone)]
pub struct Shared {
    pub history: Arc<Mutex<History>>,
    pub enabled: Arc<AtomicBool>,
    pub images_enabled: Arc<AtomicBool>,
}

impl Shared {
    pub fn new(enabled: bool, images_enabled: bool) -> Self {
        clear_legacy_image_files();
        Self {
            history: Arc::new(Mutex::new(History::load())),
            enabled: Arc::new(AtomicBool::new(enabled)),
            images_enabled: Arc::new(AtomicBool::new(images_enabled)),
        }
    }
    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Relaxed);
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
    pub fn set_images_enabled(&self, on: bool) {
        self.images_enabled.store(on, Ordering::Relaxed);
    }
    pub fn images_enabled(&self) -> bool {
        self.images_enabled.load(Ordering::Relaxed)
    }
    pub fn lock(&self) -> std::sync::MutexGuard<'_, History> {
        self.history.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn record_update(&self, text: &str, from_history: bool) -> bool {
        !from_history && self.record(text)
    }
    pub fn record(&self, text: &str) -> bool {
        if !self.is_enabled() {
            return false;
        }
        let at = chrono::Local::now().format("%H:%M").to_string();
        self.lock().push(text, &at)
    }
    pub fn record_image(&self, image: image::DynamicImage) -> bool {
        if !self.is_enabled() || !self.images_enabled() {
            return false;
        }
        let at = chrono::Local::now().format("%H:%M").to_string();
        self.lock().push_image(image, &at)
    }
}

impl Default for Shared {
    fn default() -> Self {
        Self::new(true, true)
    }
}

pub fn preview(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(windows)]
pub fn watch(shared: Shared, repaint: impl Fn() + Send + 'static) {
    std::thread::Builder::new()
        .name("nexora-isle-clipboard".into())
        .spawn(move || {
            let _ = win::run(shared, Box::new(repaint));
        })
        .ok();
}

#[cfg(not(windows))]
pub fn watch(_shared: Shared, _repaint: impl Fn() + Send + 'static) {}

#[cfg(windows)]
const HISTORY_COPY_FORMAT: &str = "NexoraIsle.ClipboardHistoryCopy";

fn legacy_image_dir() -> std::path::PathBuf {
    crate::model::config_dir().join("clipboard-images")
}

fn clear_legacy_image_files_at(directory: &std::path::Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(directory) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => {}
    }

    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
    }
    std::fs::remove_dir(directory)
}

fn clear_legacy_image_files() {
    let directory = legacy_image_dir();
    let _ = clear_legacy_image_files_at(&directory);
    std::thread::spawn(move || {
        for delay in [100_u64, 250, 500, 1_000, 2_000, 4_000, 8_000, 15_000] {
            if !directory.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(delay));
            if clear_legacy_image_files_at(&directory).is_ok() {
                return;
            }
        }
    });
}

#[cfg(windows)]
pub fn copy_history(text: &str) -> bool {
    use clipboard_win::{raw, register_format, Clipboard};
    let Some(format) = register_format(HISTORY_COPY_FORMAT) else {
        return false;
    };
    let Ok(_clipboard) = Clipboard::new_attempts(10) else {
        return false;
    };
    raw::set_string(text).is_ok() && raw::set_without_clear(format.get(), &[1]).is_ok()
}

#[cfg(windows)]
mod win {
    use super::Shared;
    use std::cell::RefCell;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HGLOBAL, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::DataExchange::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows::Win32::System::Ole::{CF_DIB, CF_DIBV5, CF_UNICODETEXT};
    use windows::Win32::UI::WindowsAndMessaging::*;

    const WM_CLIPBOARDUPDATE: u32 = 0x031D;

    struct State {
        shared: Shared,
        repaint: Box<dyn Fn() + Send>,
    }
    thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if msg == WM_CLIPBOARDUPDATE {
            STATE.with(|state| {
                if let Some(state) = state.borrow().as_ref() {
                    if state.shared.is_enabled() {
                        let changed = match read_capture(hwnd, state.shared.images_enabled()) {
                            Some(Captured::Text(text, from_history)) => {
                                state.shared.record_update(&text, from_history)
                            }
                            Some(Captured::Image(image)) => state.shared.record_image(image),
                            None => false,
                        };
                        if changed {
                            (state.repaint)();
                        }
                    }
                }
            });
            return LRESULT(0);
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }

    unsafe fn excluded() -> bool {
        let exclude = RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing"));
        let ignore = RegisterClipboardFormatW(w!("Clipboard Viewer Ignore"));
        if (exclude != 0 && IsClipboardFormatAvailable(exclude).is_ok())
            || (ignore != 0 && IsClipboardFormatAvailable(ignore).is_ok())
        {
            return true;
        }
        let history = RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory"));
        if history == 0 || IsClipboardFormatAvailable(history).is_err() {
            return false;
        }
        let Ok(handle) = GetClipboardData(history) else {
            return true;
        };
        let memory = HGLOBAL(handle.0);
        let ptr = GlobalLock(memory) as *const u32;
        if ptr.is_null() {
            return true;
        }
        let allowed = GlobalSize(memory) >= 4 && *ptr != 0;
        let _ = GlobalUnlock(memory);
        !allowed
    }

    enum Captured {
        Text(String, bool),
        Image(image::DynamicImage),
    }

    unsafe fn read_capture(hwnd: HWND, images_enabled: bool) -> Option<Captured> {
        let has_text = IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).is_ok();
        let png = if images_enabled {
            RegisterClipboardFormatW(w!("PNG"))
        } else {
            0
        };
        let image_formats = if images_enabled {
            {
                [png, CF_DIBV5.0 as u32, CF_DIB.0 as u32]
                    .into_iter()
                    .filter(|format| *format != 0 && IsClipboardFormatAvailable(*format).is_ok())
                    .collect::<Vec<_>>()
            }
        } else {
            Vec::new()
        };
        if !has_text && image_formats.is_empty() {
            return None;
        }
        let mut opened = false;
        for _ in 0..5 {
            if OpenClipboard(hwnd).is_ok() {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if !opened {
            return None;
        }
        let text = (|| {
            if excluded() {
                return None;
            }
            for format in image_formats {
                if let Some(bytes) = clipboard_blob(format) {
                    let image = if format == png {
                        image::load_from_memory(&bytes).ok()
                    } else {
                        decode_dib(&bytes)
                    };
                    if let Some(image) = image {
                        return Some(Captured::Image(image));
                    }
                }
            }
            if !has_text {
                return None;
            }
            let handle = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
            let memory = HGLOBAL(handle.0);
            let ptr = GlobalLock(memory) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let max = (GlobalSize(memory) / 2).min(super::MAX_CHARS * 2 + 1);
            let mut len = 0;
            while len < max && *ptr.add(len) != 0 {
                len += 1;
            }
            let text = (len <= super::MAX_CHARS * 2)
                .then(|| String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len)));
            let _ = GlobalUnlock(memory);
            let from_history = clipboard_win::register_format(super::HISTORY_COPY_FORMAT)
                .is_some_and(|format| IsClipboardFormatAvailable(format.get()).is_ok());
            text.map(|text| Captured::Text(text, from_history))
        })();
        let _ = CloseClipboard();
        text
    }

    unsafe fn clipboard_blob(format: u32) -> Option<Vec<u8>> {
        let handle = GetClipboardData(format).ok()?;
        let memory = HGLOBAL(handle.0);
        let size = GlobalSize(memory);
        if !(40..=super::MAX_IMAGE_BYTES).contains(&size) {
            return None;
        }
        let ptr = GlobalLock(memory) as *const u8;
        if ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
        let _ = GlobalUnlock(memory);
        Some(bytes)
    }

    pub(crate) fn decode_dib(bytes: &[u8]) -> Option<image::DynamicImage> {
        use image::ImageDecoder;
        let decoder =
            image::codecs::bmp::BmpDecoder::new_without_file_header(std::io::Cursor::new(bytes))
                .ok()?;
        let (width, height) = decoder.dimensions();
        if width == 0
            || height == 0
            || u64::from(width) * u64::from(height) > super::MAX_IMAGE_PIXELS
        {
            return None;
        }
        image::DynamicImage::from_decoder(decoder).ok()
    }

    pub fn run(shared: Shared, repaint: Box<dyn Fn() + Send>) -> windows::core::Result<()> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = w!("NexoraIsleClipboardWatcher");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: instance.into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                PCWSTR::null(),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                instance,
                None,
            )?;
            STATE.with(|state| *state.borrow_mut() = Some(State { shared, repaint }));
            AddClipboardFormatListener(hwnd)?;
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = RemoveClipboardFormatListener(hwnd);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_image(rgba: [u8; 4]) -> image::DynamicImage {
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(2, 2, image::Rgba(rgba)))
    }

    #[test]
    fn image_history_is_memory_only_and_never_reloads_from_json() {
        let mut history = History::default();
        assert!(history.push_image(solid_image([12, 34, 56, 255]), "08:00"));
        let raw = String::from_utf8(history.to_json()).unwrap();
        assert!(!raw.contains("\"images\""), "图片记录不能写入持久化 JSON");
        assert!(
            History::from_json(raw.as_bytes()).images.is_empty(),
            "重启后不能恢复任何图片"
        );
    }

    #[test]
    fn clearing_images_removes_pinned_and_recent_entries() {
        let mut history = History::default();
        assert!(history.push_image(solid_image([10, 20, 30, 255]), "08:00"));
        assert!(history.push_image(solid_image([40, 50, 60, 255]), "08:01"));
        history.images[1].pinned = true;
        history.clear_images();
        assert!(history.images.is_empty());
    }

    #[test]
    fn legacy_image_cleanup_removes_the_directory_and_is_idempotent() {
        let directory = std::env::temp_dir().join(format!(
            "nexora-isle-legacy-images-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("stale.png"), b"not a real image").unwrap();

        clear_legacy_image_files_at(&directory).unwrap();
        assert!(!directory.exists(), "旧图片目录应被完整删除");
        clear_legacy_image_files_at(&directory).unwrap();
    }

    #[test]
    fn history_copy_preserves_order_but_external_copy_promotes() {
        let shared = Shared {
            history: Default::default(),
            enabled: Arc::new(AtomicBool::new(true)),
            images_enabled: Arc::new(AtomicBool::new(true)),
        };
        shared.lock().items.clear();
        shared.record("较早的一条");
        shared.record("最近的一条");
        let before = shared.lock().items.clone();
        assert!(!shared.record_update("较早的一条", true));
        assert_eq!(shared.lock().items, before);
        assert!(shared.record_update("较早的一条", false));
        assert_eq!(shared.lock().items[0].text, "较早的一条");
    }
    #[test]
    fn history_dedupes_caps_and_keeps_pins() {
        let mut h = History::default();
        assert!(!h.push("   \n", "09:00"), "空白不记录");
        assert!(h.push("a", "09:00") && h.push("b", "09:01"));
        assert!(!h.push("b", "09:02"), "与最新一条相同不重复记录");
        assert!(h.push("a", "09:03"));
        assert_eq!(
            h.items.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(h.items[0].at, "09:03");
        h.items[1].pinned = true; // b
        for i in 0..(MAX_RECENT + 5) {
            h.push(&format!("n{i}"), "10:00");
        }
        assert_eq!(h.items.iter().filter(|c| !c.pinned).count(), MAX_RECENT);
        assert!(
            h.items.iter().any(|c| c.text == "b" && c.pinned),
            "固定的不会被挤掉"
        );
        h.clear_recent();
        assert_eq!(h.items.len(), 1);
        assert!(
            !h.push(&"x".repeat(MAX_CHARS + 1), "10:00"),
            "超长内容不记录"
        );
    }

    #[test]
    fn reopening_restores_history_and_respects_manual_cleanup() {
        let path =
            std::env::temp_dir().join(format!("nexora-isle-clip-{}.json", std::process::id()));
        std::fs::write(&path, br#"[{"text":"old","at":"08:00"}]"#).unwrap();
        let mut history = History::load_from(path.clone());
        assert!(history.items[0].pinned);
        history.push("保留", "09:00");
        history.push("删除", "09:01");
        history.remove(0);
        let mut reopened = History::load_from(path.clone());
        assert_eq!(reopened.items, history.items);
        reopened.clear_recent();
        let final_history = History::load_from(path.clone());
        assert_eq!(final_history.items.len(), 1);
        assert_eq!(final_history.items[0].text, "old");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn text_round_trip_and_preview_ignores_memory_images() {
        let mut h = History::default();
        h.push("第一行\n第二行  末尾", "08:00");
        h.push("临时", "08:01");
        h.items[1].pinned = true;
        h.images.push(ImageClip {
            id: "abc".into(),
            at: "08:02".into(),
            width: 640,
            height: 360,
            pinned: true,
            png: Arc::new(Vec::new()),
        });
        let back = History::from_json(&h.to_json());
        assert_eq!(back.items, h.items);
        assert!(back.images.is_empty(), "图片不能从持久化历史恢复");
        assert_eq!(preview(&back.items[1].text), "第一行 第二行 末尾");
        assert!(History::from_json(b"not json").items.is_empty());
    }

    #[test]
    fn pinning_moves_an_entry_to_the_front_and_unpin_keeps_its_position() {
        let mut h = History::default();
        h.push("first", "08:00");
        h.push("second", "08:01");
        h.push("third", "08:02");
        assert!(h.pin_to_front(1));
        assert_eq!(
            h.items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            ["second", "third", "first"]
        );
        assert!(h.items[0].pinned);
        assert!(h.unpin(0));
        assert_eq!(
            h.items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            ["second", "third", "first"]
        );
        assert!(!h.items[0].pinned);
    }

    #[cfg(windows)]
    #[test]
    fn dib_clipboard_payload_decodes_two_opaque_pixels() {
        let mut dib = Vec::new();
        for value in [40u32, 2, 1, (24 << 16) | 1, 0, 8, 0, 0, 0, 0] {
            dib.extend_from_slice(&value.to_le_bytes());
        }
        dib.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        let image = win::decode_dib(&dib).expect("valid clipboard DIB");
        assert_eq!((image.width(), image.height()), (2, 1));
        let rgba = image.to_rgba8();
        assert_eq!(rgba.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(rgba.get_pixel(1, 0).0, [0, 255, 0, 255]);
    }
}
