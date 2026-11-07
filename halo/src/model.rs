//! Persistent NEXORA HALO state and data models.
//!
//! Data is stored under `%APPDATA%\NEXORA\Halo`. The
//! `NEXORA_HALO_DATA_DIR` environment variable overrides the root for tests and
//! portable use.

use std::fs;
use std::path::{Path, PathBuf};

pub mod page;

pub fn config_dir() -> PathBuf {
    crate::product::paths::data_dir()
}

fn read_kv(path: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Ok(text) = fs::read_to_string(path) {
        for line in text.trim_start_matches('\u{feff}').lines() {
            if let Some((k, v)) = line.split_once('=') {
                out.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
    }
    out
}

fn write_kv(path: &Path, pairs: &[(String, String)]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body: String = pairs.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    atomic_write(path, body.as_bytes())
}

pub(crate) fn atomic_write(path: &Path, body: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let (tmp, mut file) = loop {
        let mut name = path
            .file_name()
            .ok_or_else(|| std::io::Error::other("缺少文件名"))?
            .to_os_string();
        name.push(format!(
            ".{}-{}.tmp",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let tmp = path.with_file_name(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(file) => break (tmp, file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    };
    let result = file.write_all(body).and_then(|_| file.sync_all());
    drop(file);
    let result = result.and_then(|_| replace_file(&tmp, path));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn replace_file(tmp: &Path, path: &Path) -> std::io::Result<()> {
    let result = fs::rename(tmp, path);
    #[cfg(windows)]
    if result
        .as_ref()
        .is_err_and(|e| e.kind() == std::io::ErrorKind::CrossesDevices)
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_COPY_ALLOWED, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        // Windows 目录映射可能将同目录替换报告为跨卷，交给系统复制并替换。
        let source: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
        let target: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        return unsafe {
            MoveFileExW(
                PCWSTR(source.as_ptr()),
                PCWSTR(target.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_COPY_ALLOWED | MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(std::io::Error::other);
    }
    result
}

/// 岛体形状：贴着屏幕顶边的刘海，或悬浮在顶边下方的胶囊。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IslandShape {
    #[default]
    Notch,
    Pill,
}

impl IslandShape {
    pub const ALL: [IslandShape; 2] = [IslandShape::Notch, IslandShape::Pill];
    pub fn key(self) -> &'static str {
        match self {
            Self::Notch => "notch",
            Self::Pill => "pill",
        }
    }
    pub fn parse(value: &str) -> Self {
        match value {
            "pill" => Self::Pill,
            _ => Self::Notch,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Notch => "刘海",
            Self::Pill => "胶囊",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Notch => "贴着屏幕顶边，顶部两侧向外柔和过渡，像屏幕刘海。",
            Self::Pill => "悬浮在屏幕顶边下方，四角圆润，收起时是完整的胶囊。",
        }
    }
    pub fn label_for(self, language: crate::i18n::Language) -> &'static str {
        match self {
            Self::Notch => language.text("刘海", "Notch"),
            Self::Pill => language.text("胶囊", "Pill"),
        }
    }
    pub fn description_for(self, language: crate::i18n::Language) -> &'static str {
        language.text(
            self.description(),
            match self {
                Self::Notch => "Dock against the top edge with soft side transitions.",
                Self::Pill => "Float below the top edge with fully rounded corners.",
            },
        )
    }
}

/// 岛体停靠的屏幕边缘：顶部为横向刘海 / 胶囊，左右两侧为竖向排列。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dock {
    #[default]
    Top,
    Left,
    Right,
}

impl Dock {
    pub const ALL: [Dock; 3] = [Dock::Top, Dock::Left, Dock::Right];
    pub fn key(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
    pub fn parse(value: &str) -> Self {
        match value {
            "left" => Self::Left,
            "right" => Self::Right,
            _ => Self::Top,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Top => "顶部",
            Self::Left => "左侧",
            Self::Right => "右侧",
        }
    }
    pub fn label_for(self, language: crate::i18n::Language) -> &'static str {
        match self {
            Self::Top => language.text("顶部", "Top"),
            Self::Left => language.text("左侧", "Left"),
            Self::Right => language.text("右侧", "Right"),
        }
    }
    pub fn is_side(self) -> bool {
        self != Self::Top
    }
}

/// 岛体大小（百分比）的范围与步进。
pub const ISLAND_SIZE_MIN: u32 = 80;
pub const ISLAND_SIZE_MAX: u32 = 140;
pub const ISLAND_SIZE_STEP: u32 = 5;
/// 水平位置：-100 = 最左，0 = 居中，100 = 最右（按可移动范围的百分比）。
pub const ISLAND_OFFSET_MAX: i32 = 100;

pub fn island_size_value(value: u32) -> u32 {
    let value = value.clamp(ISLAND_SIZE_MIN, ISLAND_SIZE_MAX);
    (value + ISLAND_SIZE_STEP / 2) / ISLAND_SIZE_STEP * ISLAND_SIZE_STEP
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub language: crate::i18n::Language,
    pub display_id: String,
    /// 岛体形状、大小百分比和水平位置。
    pub island_shape: IslandShape,
    pub island_size: u32,
    pub island_offset: i32,
    /// 停靠边缘；左右停靠时 `island_offset` 表示上下位置。
    pub dock: Dock,
    pub settings_theme: crate::settings_theme::SettingsTheme,
    pub pet: bool,
    pub minimal: bool,
    pub browser_minimal: bool,
    pub avoid_browsers: bool,
    pub avoid_apps: Vec<(String, String)>,
    pub visible_pages: [bool; page::COUNT],
    pub page_order: [usize; page::COUNT],
    pub show_battery: bool,
    pub clock_12h: bool,
    pub clock_period: u8,
    pub resident_order: Vec<usize>,
    pub system_hud: bool,
    pub bluetooth_notifications: bool,
    pub read_notifications: bool,
    pub wechat_notifications: bool,
    /// 微信缩在托盘时，点提醒通过系统托盘按钮唤起微信（默认关闭）。
    pub wechat_tray_open: bool,
    pub notification_preview: bool,
    pub notifications: bool,
    pub sound: bool,
    pub lyrics_enabled: bool,
    pub island_lyrics: bool,
    pub lyric_delay_ms: i32,
    pub hide_fullscreen: bool,
    /// Mail preview: master switch, verification-code highlighting, poll interval.
    pub mail_enabled: bool,
    pub mail_code: bool,
    pub mail_code_hide_seconds: u32,
    pub mail_interval: u32,
    /// 记录最近复制的文字（最近记录与固定条目均在本机保存）。
    pub clipboard_history: bool,
    /// 记录复制或截取的图片（原图保存在本机数据目录）。
    pub clipboard_images: bool,
    /// System monitor page and threshold alerts.
    pub monitor_enabled: bool,
    pub monitor_alerts: bool,
    pub monitor_cpu_threshold: u32,
    pub monitor_memory_threshold: u32,
    pub monitor_gpu_threshold: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: crate::i18n::Language::Zh,
            display_id: String::new(),
            island_shape: IslandShape::Notch,
            island_size: 100,
            island_offset: 0,
            dock: Dock::Top,
            settings_theme: Default::default(),
            pet: true,
            minimal: false,
            browser_minimal: false,
            avoid_browsers: true,
            avoid_apps: Vec::new(),
            visible_pages: [true; page::COUNT],
            page_order: page::ALL,
            show_battery: true,
            clock_12h: false,
            clock_period: 0,
            resident_order: Vec::new(),
            system_hud: true,
            bluetooth_notifications: true,
            read_notifications: false,
            wechat_notifications: false,
            wechat_tray_open: false,
            notification_preview: false,
            notifications: true,
            sound: false,
            lyrics_enabled: true,
            island_lyrics: false,
            lyric_delay_ms: -350,
            hide_fullscreen: true,
            mail_enabled: true,
            mail_code: true,
            mail_code_hide_seconds: 60,
            mail_interval: 120,
            clipboard_history: true,
            clipboard_images: true,
            monitor_enabled: true,
            monitor_alerts: true,
            monitor_cpu_threshold: 85,
            monitor_memory_threshold: 85,
            monitor_gpu_threshold: 90,
        }
    }
}

impl Settings {
    /// 主窗口（灵动岛）的界面缩放倍率。
    pub fn island_scale(&self) -> f32 {
        island_size_value(self.island_size) as f32 / 100.0
    }

    /// 水平位置，-1..=1。
    pub fn island_offset_ratio(&self) -> f32 {
        self.island_offset
            .clamp(-ISLAND_OFFSET_MAX, ISLAND_OFFSET_MAX) as f32
            / ISLAND_OFFSET_MAX as f32
    }

    pub fn clock_text(&self, clock: &str) -> String {
        use chrono::Timelike;
        let Ok(time) = chrono::NaiveTime::parse_from_str(clock.get(..5).unwrap_or(""), "%H:%M")
        else {
            return "--:--".into();
        };
        let (pm, hour) = time.hour12();
        let digits = if self.clock_12h {
            format!("{}:{:02}", hour, time.minute())
        } else {
            time.format("%H:%M").to_string()
        };
        let period = match self.clock_period {
            1 => {
                if pm {
                    "下午"
                } else {
                    "上午"
                }
            }
            2 => match time.hour() {
                0..=5 => "凌晨",
                6..=10 => "上午",
                11..=12 => "中午",
                13..=16 => "下午",
                17..=18 => "傍晚",
                _ => "晚上",
            },
            _ => return digits,
        };
        format!("{period} {digits}")
    }

    pub fn load() -> Self {
        let mut s = Self::default();
        let values = read_kv(&config_dir().join("settings.txt"));
        let layout_version = values
            .iter()
            .find(|(key, _)| key == "layout_version")
            .and_then(|(_, value)| value.parse::<u32>().ok())
            .unwrap_or(0);
        for (k, v) in values {
            match k.as_str() {
                "language" => s.language = crate::i18n::Language::parse(&v),
                "settings_theme" => {
                    s.settings_theme = crate::settings_theme::SettingsTheme::parse(&v)
                }
                "display_id" => s.display_id = v,
                "island_shape" => s.island_shape = IslandShape::parse(&v),
                "dock" => s.dock = Dock::parse(&v),
                "island_size" => s.island_size = island_size_value(v.parse::<u32>().unwrap_or(100)),
                "island_offset" => {
                    s.island_offset = v
                        .parse::<i32>()
                        .unwrap_or(0)
                        .clamp(-ISLAND_OFFSET_MAX, ISLAND_OFFSET_MAX)
                }
                "pet" => s.pet = v == "1" || v.eq_ignore_ascii_case("true"),
                "minimal" => s.minimal = v == "1" || v.eq_ignore_ascii_case("true"),
                "browser_minimal" => s.browser_minimal = v == "1",
                "avoid_browsers" => s.avoid_browsers = v != "0",
                "avoid_apps" => s.avoid_apps = serde_json::from_str(&v).unwrap_or_default(),
                "resident_order" => {
                    let order: Vec<usize> = v.split(',').filter_map(|n| n.parse().ok()).collect();
                    if crate::bar_layout::valid_order(&order) {
                        s.resident_order = order;
                    }
                }
                "page_order" => {
                    if let Ok(order) = v
                        .split(',')
                        .map(str::parse::<usize>)
                        .collect::<Result<Vec<_>, _>>()
                    {
                        if order.len() == page::COUNT
                            && page::ALL.iter().all(|page| order.contains(page))
                        {
                            s.page_order.copy_from_slice(&order);
                        } else if layout_version >= 3 && order.len() == 6 {
                            let previous: [usize; 6] =
                                order.try_into().unwrap_or([0, 1, 2, 3, 4, 5]);
                            s.page_order = page::migrate_current_v6_order(previous);
                        } else if layout_version >= 2 && order.len() == 4 {
                            let previous: [usize; 4] = order.try_into().unwrap_or([0, 1, 2, 3]);
                            s.page_order = page::migrate_current_v4_order(previous);
                        } else if order.len() == 3 {
                            let previous: [usize; 3] = order.try_into().unwrap_or([0, 1, 2]);
                            s.page_order = page::migrate_v3_order(previous);
                        } else if order.len() == 4 {
                            let previous: [usize; 4] = order.try_into().unwrap_or([0, 1, 2, 3]);
                            s.page_order = page::migrate_v4_order(previous);
                        } else if order.len() == 8 && (0..8).all(|page| order.contains(&page)) {
                            let legacy: [usize; 8] =
                                order.try_into().unwrap_or([0, 1, 2, 3, 4, 5, 6, 7]);
                            s.page_order = page::migrate_legacy_order(legacy);
                        }
                    }
                }
                "clock_period" => {
                    s.clock_period = v.parse::<u8>().ok().filter(|v| *v <= 2).unwrap_or(0)
                }
                "clock_12h" => s.clock_12h = v == "1",
                "show_battery" => s.show_battery = v != "0",
                "visible_pages"
                    if (3..=8).contains(&v.len()) && v.bytes().all(|b| b == b'0' || b == b'1') =>
                {
                    if v.len() == page::COUNT {
                        for (enabled, value) in s.visible_pages.iter_mut().zip(v.bytes()) {
                            *enabled = value == b'1';
                        }
                    } else if layout_version >= 3 && v.len() == 6 {
                        s.visible_pages = page::migrate_current_v6_visibility(v.as_bytes());
                    } else if layout_version >= 2 && v.len() == 4 {
                        s.visible_pages = page::migrate_current_v4_visibility(v.as_bytes());
                    } else if v.len() == 3 {
                        s.visible_pages = page::migrate_v3_visibility(v.as_bytes());
                    } else if v.len() == 4 {
                        s.visible_pages = page::migrate_v4_visibility(v.as_bytes());
                    } else {
                        s.visible_pages = page::migrate_legacy_visibility(v.as_bytes());
                    }
                }
                "read_notifications" => s.read_notifications = v == "1",
                "wechat_notifications" => s.wechat_notifications = v == "1",
                "wechat_tray_open" => s.wechat_tray_open = v == "1",
                "notification_preview" => s.notification_preview = v == "1",
                "system_hud" => s.system_hud = v != "0",
                "bluetooth_notifications" => s.bluetooth_notifications = v != "0",
                "notifications" => s.notifications = v != "0",
                "sound" => s.sound = v == "1",
                "lyrics_enabled" => s.lyrics_enabled = v != "0",
                "island_lyrics" => s.island_lyrics = v == "1",
                "lyric_delay_ms" => {
                    s.lyric_delay_ms = v.parse::<i32>().unwrap_or(-350).clamp(-5000, 5000)
                }
                "hide_fullscreen" => s.hide_fullscreen = v != "0",
                "mail_enabled" => s.mail_enabled = v != "0",
                "mail_code" => s.mail_code = v != "0",
                "mail_code_hide_seconds" => {
                    s.mail_code_hide_seconds = v.parse::<u32>().unwrap_or(60).clamp(15, 300)
                }
                "mail_interval" => {
                    s.mail_interval = match v.parse::<u32>().unwrap_or(120) {
                        60 => 60,
                        300 => 300,
                        _ => 120,
                    }
                }
                "clipboard_history" => s.clipboard_history = v != "0",
                "clipboard_images" => s.clipboard_images = v != "0",
                "monitor_enabled" => s.monitor_enabled = v != "0",
                "monitor_alerts" => s.monitor_alerts = v != "0",
                "monitor_cpu_threshold" => {
                    s.monitor_cpu_threshold = v.parse::<u32>().unwrap_or(85).clamp(50, 100)
                }
                "monitor_memory_threshold" => {
                    s.monitor_memory_threshold = v.parse::<u32>().unwrap_or(85).clamp(50, 100)
                }
                "monitor_gpu_threshold" => {
                    s.monitor_gpu_threshold = v.parse::<u32>().unwrap_or(90).clamp(50, 100)
                }
                _ => {}
            }
        }
        if !s.visible_pages.contains(&true) {
            s.visible_pages[page::MUSIC] = true;
        }
        s
    }

    pub fn save(&self) {
        let _ = write_kv(
            &config_dir().join("settings.txt"),
            &[
                ("language".into(), self.language.key().into()),
                ("layout_version".into(), "4".into()),
                ("display_id".into(), self.display_id.clone()),
                ("island_shape".into(), self.island_shape.key().into()),
                ("dock".into(), self.dock.key().into()),
                (
                    "island_size".into(),
                    island_size_value(self.island_size).to_string(),
                ),
                (
                    "island_offset".into(),
                    self.island_offset
                        .clamp(-ISLAND_OFFSET_MAX, ISLAND_OFFSET_MAX)
                        .to_string(),
                ),
                ("settings_theme".into(), self.settings_theme.key().into()),
                (
                    "pet".into(),
                    if self.pet {
                        "1".to_string()
                    } else {
                        "0".to_string()
                    },
                ),
                (
                    "minimal".into(),
                    if self.minimal {
                        "1".to_string()
                    } else {
                        "0".to_string()
                    },
                ),
                (
                    "browser_minimal".into(),
                    (self.browser_minimal as u8).to_string(),
                ),
                (
                    "avoid_browsers".into(),
                    (self.avoid_browsers as u8).to_string(),
                ),
                (
                    "avoid_apps".into(),
                    serde_json::to_string(&self.avoid_apps).unwrap_or_default(),
                ),
                (
                    "visible_pages".into(),
                    self.visible_pages
                        .iter()
                        .map(|enabled| if *enabled { '1' } else { '0' })
                        .collect(),
                ),
                (
                    "page_order".into(),
                    self.page_order
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                ),
                (
                    "resident_order".into(),
                    self.resident_order
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                ),
                ("clock_period".into(), self.clock_period.to_string()),
                ("clock_12h".into(), (self.clock_12h as u8).to_string()),
                ("show_battery".into(), (self.show_battery as u8).to_string()),
                (
                    "read_notifications".into(),
                    (self.read_notifications as u8).to_string(),
                ),
                (
                    "wechat_notifications".into(),
                    (self.wechat_notifications as u8).to_string(),
                ),
                (
                    "wechat_tray_open".into(),
                    (self.wechat_tray_open as u8).to_string(),
                ),
                (
                    "notification_preview".into(),
                    (self.notification_preview as u8).to_string(),
                ),
                ("system_hud".into(), (self.system_hud as u8).to_string()),
                (
                    "bluetooth_notifications".into(),
                    (self.bluetooth_notifications as u8).to_string(),
                ),
                (
                    "notifications".into(),
                    (self.notifications as u8).to_string(),
                ),
                ("sound".into(), (self.sound as u8).to_string()),
                (
                    "lyrics_enabled".into(),
                    (self.lyrics_enabled as u8).to_string(),
                ),
                (
                    "island_lyrics".into(),
                    (self.island_lyrics as u8).to_string(),
                ),
                ("lyric_delay_ms".into(), self.lyric_delay_ms.to_string()),
                (
                    "hide_fullscreen".into(),
                    (self.hide_fullscreen as u8).to_string(),
                ),
                ("mail_enabled".into(), (self.mail_enabled as u8).to_string()),
                ("mail_code".into(), (self.mail_code as u8).to_string()),
                (
                    "mail_code_hide_seconds".into(),
                    self.mail_code_hide_seconds.to_string(),
                ),
                ("mail_interval".into(), self.mail_interval.to_string()),
                (
                    "clipboard_history".into(),
                    (self.clipboard_history as u8).to_string(),
                ),
                (
                    "clipboard_images".into(),
                    (self.clipboard_images as u8).to_string(),
                ),
                (
                    "monitor_enabled".into(),
                    (self.monitor_enabled as u8).to_string(),
                ),
                (
                    "monitor_alerts".into(),
                    (self.monitor_alerts as u8).to_string(),
                ),
                (
                    "monitor_cpu_threshold".into(),
                    self.monitor_cpu_threshold.to_string(),
                ),
                (
                    "monitor_memory_threshold".into(),
                    self.monitor_memory_threshold.to_string(),
                ),
                (
                    "monitor_gpu_threshold".into(),
                    self.monitor_gpu_threshold.to_string(),
                ),
            ],
        );
    }
}

#[derive(Clone, Debug, Default)]
pub struct Shelf {
    pub items: Vec<PathBuf>,
}

impl Shelf {
    pub fn load() -> Self {
        let mut s = Self::default();
        if let Ok(text) = fs::read_to_string(config_dir().join("shelf.txt")) {
            for line in text.lines() {
                let p = PathBuf::from(line.trim());
                if !line.trim().is_empty() && !s.items.contains(&p) {
                    s.items.push(p);
                }
            }
        }
        s
    }

    pub fn save(&self) {
        let body: String = self
            .items
            .iter()
            .map(|p| format!("{}\n", p.display()))
            .collect();
        let path = config_dir().join("shelf.txt");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = atomic_write(&path, body.as_bytes());
    }

    /// Record file locations without copying file contents.
    pub fn add(&mut self, src: &Path) -> std::io::Result<()> {
        if !src.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "文件不存在",
            ));
        }
        if !self.items.iter().any(|p| p == src) {
            self.items.push(src.to_path_buf());
            self.save();
        }
        Ok(())
    }

    pub fn remove(&mut self, p: &Path) {
        self.items.retain(|item| item != p);
        self.save();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.save();
    }
}

pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= KB * KB * KB {
        format!("{:.1} GB", b / (KB * KB * KB))
    } else if b >= KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_page_order_is_migrated_to_retained_pages() {
        assert_eq!(
            page::migrate_legacy_order([5, 1, 3, 0, 7, 4, 2, 6]),
            [
                page::MAIL,
                page::SHELF,
                page::MUSIC,
                page::VIDEO,
                page::MONITOR,
                page::IMAGES,
                page::CLIPBOARD
            ]
        );
        assert_eq!(
            page::migrate_v4_order([3, 2, 0, 1]),
            [
                page::MAIL,
                page::MUSIC,
                page::SHELF,
                page::VIDEO,
                page::MONITOR,
                page::IMAGES,
                page::CLIPBOARD
            ]
        );
        assert_eq!(
            page::migrate_v3_order([2, 0, 1]),
            [
                page::MAIL,
                page::MUSIC,
                page::SHELF,
                page::VIDEO,
                page::MONITOR,
                page::IMAGES,
                page::CLIPBOARD
            ]
        );
        assert_eq!(
            page::migrate_current_v4_order([page::MAIL, page::CLIPBOARD, page::MUSIC, page::SHELF]),
            [
                page::MAIL,
                page::CLIPBOARD,
                page::MUSIC,
                page::VIDEO,
                page::SHELF,
                page::IMAGES,
                page::MONITOR
            ]
        );
        assert_eq!(
            page::migrate_current_v6_order([
                page::MAIL,
                page::CLIPBOARD,
                page::MUSIC,
                page::VIDEO,
                page::SHELF,
                page::IMAGES
            ]),
            [
                page::MAIL,
                page::CLIPBOARD,
                page::MUSIC,
                page::VIDEO,
                page::MONITOR,
                page::SHELF,
                page::IMAGES
            ]
        );
    }

    #[test]
    fn legacy_visibility_drops_removed_features() {
        assert_eq!(
            page::migrate_legacy_visibility(b"10101011"),
            [true, false, false, false, false, true, true]
        );
        assert_eq!(
            page::migrate_v4_visibility(b"1011"),
            [true, false, false, true, false, true, true]
        );
        assert_eq!(
            page::migrate_v3_visibility(b"101"),
            [true, false, false, true, false, true, true]
        );
        assert_eq!(
            page::migrate_current_v4_visibility(b"1011"),
            [true, false, true, true, true, true, true]
        );
        assert_eq!(
            page::migrate_current_v6_visibility(b"101010"),
            [true, false, true, false, true, false, true]
        );
    }

    #[test]
    fn island_appearance_values_are_normalized() {
        assert_eq!(island_size_value(100), 100);
        assert_eq!(island_size_value(10), ISLAND_SIZE_MIN);
        assert_eq!(island_size_value(999), ISLAND_SIZE_MAX);
        assert_eq!(island_size_value(113), 115);
        assert_eq!(island_size_value(112), 110);
        for shape in IslandShape::ALL {
            assert_eq!(IslandShape::parse(shape.key()), shape);
        }
        assert_eq!(Settings::default().island_shape, IslandShape::Notch);
        let mut s = Settings::default();
        assert_eq!((s.island_scale(), s.island_offset_ratio()), (1.0, 0.0));
        s.island_size = 125;
        s.island_offset = -250;
        assert_eq!((s.island_scale(), s.island_offset_ratio()), (1.25, -1.0));
    }

    #[test]
    fn quality_save_preserves_neighbor_and_cleans_failed_writes() {
        let dir =
            std::env::temp_dir().join(format!("nexora-halo-quality-save-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.txt");
        let neighbor = dir.join("settings.tmp");
        fs::write(&neighbor, b"other data").unwrap();
        let saved = atomic_write(&path, b"settings");
        let untouched = fs::read(&neighbor).ok() == Some(b"other data".to_vec());
        let blocked = dir.join("blocked.txt");
        fs::create_dir_all(&blocked).unwrap();
        assert!(atomic_write(&blocked, b"cannot replace directory").is_err());
        let leftovers = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path() != path && e.path() != neighbor && e.path() != blocked)
            .count();
        let failed_tmp = blocked.with_extension("tmp").exists();
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                fs::remove_file(entry.path()).unwrap();
            }
        }
        fs::remove_dir(blocked).unwrap();
        fs::remove_dir(dir).unwrap();
        assert!(
            saved.is_ok() && untouched,
            "保存不能覆盖或移走同名 .tmp 文件"
        );
        assert!(
            !failed_tmp && leftovers == 0,
            "失败的保存应清除本次临时文件"
        );
    }

    #[test]
    fn atomic_write_replaces_existing_settings() {
        let dir = std::env::temp_dir().join(format!(
            "nexora-halo-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.txt");
        atomic_write(&path, b"setting=first\n").unwrap();
        atomic_write(&path, b"setting=second\n").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"setting=second\n");
        assert!(
            !path.with_extension("tmp").exists(),
            "保存后不应遗留临时配置"
        );
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn quality_concurrent_saves_publish_one_complete_value() {
        let dir = std::env::temp_dir().join(format!(
            "nexora-halo-quality-concurrent-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cache.lrc");
        let gate = std::sync::Arc::new(std::sync::Barrier::new(4));
        let workers: Vec<_> = (0..4u8)
            .map(|byte| {
                let path = path.clone();
                let gate = gate.clone();
                std::thread::spawn(move || {
                    gate.wait();
                    atomic_write(&path, &vec![byte; 128 * 1024])
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
        let content = fs::read(&path).unwrap();
        assert_eq!(content.len(), 128 * 1024);
        assert!(content.iter().all(|b| *b == content[0]));
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn human_size_formats_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536 * 1024), "1.5 MB");
        assert_eq!(human_size(2 * 1024 * 1024 * 1024), "2.0 GB");
    }
}

/// 仅保存应用标识与开关，不保存通知内容。
#[derive(Clone, Debug, Default)]
pub struct NotificationApps {
    pub items: Vec<(String, String, bool)>,
}
impl NotificationApps {
    pub fn load() -> Self {
        let path = config_dir().join("notification-apps.json");
        let Ok(raw) = fs::read(path) else {
            return Self::default();
        };
        let Ok(json) = serde_json::from_slice::<serde_json::Value>(&raw) else {
            return Self::default();
        };
        let items = json
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| {
                Some((
                    a.get("id")?.as_str()?.to_owned(),
                    a.get("name")?.as_str()?.to_owned(),
                    a.get("enabled")?.as_bool()?,
                ))
            })
            .collect();
        Self { items }
    }
    pub fn save(&self) {
        let list: Vec<_> = self
            .items
            .iter()
            .map(|(id, name, enabled)| serde_json::json!({"id":id,"name":name,"enabled":enabled}))
            .collect();
        let path = config_dir().join("notification-apps.json");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_vec(&list) {
            let _ = atomic_write(&path, &data);
        }
    }
    pub fn observe(&mut self, id: &str, name: &str) -> bool {
        if let Some(item) = self.items.iter().find(|a| a.0 == id) {
            return item.2;
        }
        let enabled = true;
        self.items.push((id.to_string(), name.to_string(), enabled));
        self.save();
        enabled
    }
}
