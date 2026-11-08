//! Stable public and machine-facing product identity.

pub const DISPLAY_NAME: &str = "NEXORA ISLE · 星屿";
pub const COMPACT_NAME: &str = "ISLE";
pub const CHINESE_NAME: &str = "星屿";
pub const EXECUTABLE_NAME: &str = "nexora-isle.exe";
pub const WINDOW_TITLE: &str = "NEXORA ISLE · 星屿";
pub const SETTINGS_TITLE: &str = "NEXORA ISLE · 星屿 设置";
pub const SETTINGS_TITLE_EN: &str = "NEXORA ISLE · 星屿 Settings";
pub const TRAY_TOOLTIP: &str = "NEXORA ISLE · 星屿";
pub const TRAY_WINDOW_CLASS: &str = "NexoraIsleTray";
pub const TRAY_WINDOW_TITLE: &str = "NEXORA ISLE · 星屿 托盘";
pub const SINGLE_INSTANCE_MUTEX: &str = "Local\\NexoraIsle.SingleInstance";
pub const AUTOSTART_VALUE_NAME: &str = "NexoraIsle";
pub const UNINSTALL_KEY_NAME: &str = "NexoraIsle";
pub const DATA_ENVIRONMENT_VARIABLE: &str = "NEXORA_ISLE_DATA_DIR";
pub const CREDENTIAL_NAMESPACE: &str = "NexoraIsle";
pub const USER_AGENT: &str = concat!("NEXORA-ISLE/", env!("CARGO_PKG_VERSION"));
pub const REPOSITORY: &str = "https://github.com/ctxue/nexora";
pub const RELEASE_API: &str = "https://api.github.com/repos/ctxue/nexora/releases/latest";
pub const RELEASES_URL: &str = "https://github.com/ctxue/nexora/releases/latest";
