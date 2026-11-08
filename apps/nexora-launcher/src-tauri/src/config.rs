use std::time::Duration;

pub const MAX_FILES: usize = 20_000;

pub const MAX_APPS: usize = 400;

pub const START_MENU_MAX_DEPTH: usize = 6;
pub const FILE_SCAN_MAX_DEPTH: usize = 5;

pub const CONFIG_DIALOG_BUFFER: usize = 1024;

pub const CONFIG_EXPORT_FILE_NAME: &str = "nexora-settings.json";

pub const CONFIG_EXPORT_EXTENSION: &str = "json";

pub const CONFIG_DIALOG_FILTER: &str = "JSON file\0*.json\0All files\0*.*\0\0";

pub const EXE_DIALOG_FILTER: &str = "Applications\0*.exe\0All files\0*.*\0\0";
pub const ANY_FILE_DIALOG_FILTER: &str = "All files\0*.*\0\0";

pub const ADD_APP_TITLE: &str = "Add application";
pub const PICK_FILE_TITLE: &str = "Select a file";
pub const PICK_FOLDER_TITLE: &str = "Select a folder";

pub const MAX_PORTABLE_APPS: usize = 200;
pub const PORTABLE_SCAN_MAX_DEPTH: usize = 3;

pub const JUNK_EXECUTABLE_MARKERS: [&str; 13] = [
    "unins",
    "uninstall",
    "setup",
    "installer",
    "update",
    "updater",
    "crash",
    "report",
    "vcredist",
    "dxsetup",
    "readme",
    "remove",
    "helper",
];

pub const CONFIG_EXPORT_TITLE: &str = "Export NEXORA configuration";
pub const CONFIG_IMPORT_TITLE: &str = "Import NEXORA configuration";

pub const FALLBACK_SHORTCUT: &str = "Ctrl+Shift+Alt+Q";
pub const DEFAULT_SHORTCUT: &str = "Alt+Space";

pub const TOGGLE_DEBOUNCE_MS: u64 = 150;

pub const FOCUS_GRACE: Duration = Duration::from_millis(150);

pub const FOCUS_RETRY_INTERVAL: Duration = Duration::from_millis(10);
pub const FOCUS_RETRY_ATTEMPTS: usize = 40;
pub const DRAG_POLL_INTERVAL: Duration = Duration::from_millis(30);
pub const DRAG_MAX_DURATION: Duration = Duration::from_secs(5);

pub const LIGHT_SETTINGS_CHROME: u32 = 0x00F0_EDED; // #EDEDF0
pub const DARK_SETTINGS_CHROME: u32 = 0x0016_1313; // #131316
pub const LIGHT_TITLE_TEXT: u32 = 0x001B_1818; // #18181B
pub const DARK_TITLE_TEXT: u32 = 0x00F5_F4F4; // #F4F4F5

pub const LAUNCHER_WINDOW: &str = "main";
pub const SETTINGS_WINDOW: &str = "settings";
pub const TRAY_ID: &str = "main-tray";

pub const EVENT_LAUNCHER_SHOWN: &str = "launcher-shown";
pub const EVENT_LAUNCHER_HIDDEN: &str = "launcher-hidden";
pub const EVENT_LAUNCHER_DRAG_ENDED: &str = "launcher-drag-ended";
pub const EVENT_FILES_INDEXED: &str = "files-indexed";
pub const EVENT_SHORTCUT_CAPTURED: &str = "shortcut-captured";
pub const EVENT_CLIPBOARD_UPDATED: &str = "clipboard-updated";

pub const JUNK_SHORTCUT_MARKERS: [&str; 14] = [
    "uninstall",
    "readme",
    "release notes",
    "documentation",
    "changelog",
    "license",
    "website",
    "remove ",
    "卸载",
    "帮助",
    "说明",
    "网站",
    "更新日志",
    "许可证",
];

pub const IGNORED_DIRS: [&str; 10] = [
    "node_modules",
    ".git",
    ".svn",
    "target",
    "appdata",
    "windows",
    "program files",
    "program files (x86)",
    ".cache",
    "vendor",
];

pub const IGNORED_EXTENSIONS: [&str; 11] = [
    "tmp",
    "log",
    "dll",
    "sys",
    "msi",
    "cab",
    "pak",
    "dat",
    "lock",
    "crdownload",
    "part",
];

pub const AUTOSTART_FLAG: &str = "--autostart";
