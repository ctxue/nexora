use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::ManagerExt as AutostartExt;

use crate::{
    clipboard::ClipboardHistory,
    commands::{apps::start_file_index, settings::open_settings_window},
    config::{EVENT_TRAY_SETTINGS_CHANGED, TRAY_ID},
    launcher::show_launcher,
    state::FileIndex,
};

static TRAY_LANGUAGE_ZH: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TraySettingsChanged {
    launch_at_startup: bool,
    clipboard_history: bool,
}

struct MenuLabels {
    show: &'static str,
    settings: &'static str,
    autostart: &'static str,
    clipboard: &'static str,
    refresh: &'static str,
    quit: &'static str,
}

fn menu_labels(language: &str) -> MenuLabels {
    if language == "zh" {
        MenuLabels {
            show: "打开 NEXORA",
            settings: "设置…",
            autostart: "开机自启",
            clipboard: "剪贴板历史",
            refresh: "刷新文件索引",
            quit: "退出 NEXORA",
        }
    } else {
        MenuLabels {
            show: "Show NEXORA",
            settings: "Settings…",
            autostart: "Launch at startup",
            clipboard: "Clipboard history",
            refresh: "Refresh file index",
            quit: "Quit NEXORA",
        }
    }
}

pub fn build_menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let labels = menu_labels(language);
    let autostart_checked = app.autolaunch().is_enabled().unwrap_or(false);
    let clipboard_checked = app
        .try_state::<ClipboardHistory>()
        .map(|history| history.capture_enabled())
        .unwrap_or(false);

    let show = MenuItem::with_id(app, "show", labels.show, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", labels.settings, true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        labels.autostart,
        true,
        autostart_checked,
        None::<&str>,
    )?;
    let clipboard = CheckMenuItem::with_id(
        app,
        "clipboard",
        labels.clipboard,
        true,
        clipboard_checked,
        None::<&str>,
    )?;
    let refresh = MenuItem::with_id(app, "refresh-files", labels.refresh, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", labels.quit, true, None::<&str>)?;
    let appearance_separator = PredefinedMenuItem::separator(app)?;
    let quit_separator = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &show,
            &settings,
            &appearance_separator,
            &autostart,
            &clipboard,
            &refresh,
            &quit_separator,
            &quit,
        ],
    )
}

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let menu = build_menu(app.handle(), "en")?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_launcher(app),
            "settings" => {
                if let Err(error) = open_settings_window(app.clone()) {
                    eprintln!("{error}");
                }
            }
            "autostart" => {
                let autolaunch = app.autolaunch();
                let enabled = autolaunch.is_enabled().unwrap_or(false);
                let result = if enabled {
                    autolaunch.disable()
                } else {
                    autolaunch.enable()
                };
                if let Err(error) = result {
                    eprintln!("Could not update startup setting: {error}");
                }
                emit_settings_changed(app);
                if let Err(error) = refresh(app) {
                    eprintln!("{error}");
                }
            }
            "clipboard" => {
                let history = app.state::<ClipboardHistory>();
                history.set_capture_enabled(!history.capture_enabled());
                emit_settings_changed(app);
                if let Err(error) = refresh(app) {
                    eprintln!("{error}");
                }
            }
            "refresh-files" => {
                let index = app.state::<FileIndex>();
                start_file_index(app.clone(), index);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_launcher(tray.app_handle());
            }
        });

    builder = builder.icon(tauri::image::Image::from_bytes(include_bytes!(
        "../icons/tray-icon.png"
    ))?);

    builder.build(app)?;
    Ok(())
}

pub fn set_language(app: &AppHandle, language: &str) -> Result<(), String> {
    TRAY_LANGUAGE_ZH.store(language == "zh", Ordering::Relaxed);
    refresh(app)
}

pub fn refresh(app: &AppHandle) -> Result<(), String> {
    let tray = app
        .tray_by_id(TRAY_ID)
        .ok_or_else(|| "Tray icon is not available".to_string())?;
    let language = if TRAY_LANGUAGE_ZH.load(Ordering::Relaxed) {
        "zh"
    } else {
        "en"
    };
    let menu = build_menu(app, language).map_err(|error| error.to_string())?;
    tray.set_menu(Some(menu)).map_err(|error| error.to_string())
}

fn emit_settings_changed(app: &AppHandle) {
    let launch_at_startup = app.autolaunch().is_enabled().unwrap_or(false);
    let clipboard_history = app
        .try_state::<ClipboardHistory>()
        .map(|history| history.capture_enabled())
        .unwrap_or(false);
    let _ = app.emit(
        EVENT_TRAY_SETTINGS_CHANGED,
        TraySettingsChanged {
            launch_at_startup,
            clipboard_history,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localized_menu_exposes_high_frequency_toggles() {
        let labels = menu_labels("zh");

        assert_eq!(labels.autostart, "开机自启");
        assert_eq!(labels.clipboard, "剪贴板历史");
        assert_eq!(labels.refresh, "刷新文件索引");
    }
}
