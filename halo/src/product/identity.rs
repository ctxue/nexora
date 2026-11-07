//! Stable public and machine-facing product identity.

pub const DISPLAY_NAME: &str = "NEXORA HALO";
pub const COMPACT_NAME: &str = "HALO";
pub const CHINESE_NAME: &str = "星环";
pub const EXECUTABLE_NAME: &str = "nexora-halo.exe";
pub const WINDOW_TITLE: &str = "NEXORA HALO";
pub const SETTINGS_TITLE: &str = "NEXORA HALO 设置";
pub const SETTINGS_TITLE_EN: &str = "NEXORA HALO Settings";
pub const TRAY_TOOLTIP: &str = "NEXORA HALO · 星环";
pub const TRAY_WINDOW_CLASS: &str = "NexoraHaloTray";
pub const TRAY_WINDOW_TITLE: &str = "NEXORA HALO 托盘";
pub const SINGLE_INSTANCE_MUTEX: &str = "Local\\NexoraHalo.SingleInstance";
pub const AUTOSTART_VALUE_NAME: &str = "NexoraHalo";
pub const UNINSTALL_KEY_NAME: &str = "NexoraHalo";
pub const DATA_ENVIRONMENT_VARIABLE: &str = "NEXORA_HALO_DATA_DIR";
pub const CREDENTIAL_NAMESPACE: &str = "NexoraHalo";
pub const USER_AGENT: &str = concat!("NEXORA-HALO/", env!("CARGO_PKG_VERSION"));
pub const REPOSITORY: &str = "https://github.com/ctxue/nexora";
pub const RELEASE_API: &str = "https://api.github.com/repos/ctxue/nexora/releases/latest";
pub const RELEASES_URL: &str = "https://github.com/ctxue/nexora/releases/latest";
