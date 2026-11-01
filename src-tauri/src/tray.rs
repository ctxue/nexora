//! Tray icon and its localized menu.

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

use crate::{commands::settings::open_settings_window, config::TRAY_ID, launcher::show_launcher};

fn menu_labels(language: &str) -> (&'static str, &'static str, &'static str) {
    if language == "zh" {
        ("打开 NEXORA", "设置…", "退出 NEXORA")
    } else {
        ("Show NEXORA", "Settings…", "Quit NEXORA")
    }
}

pub fn build_menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let (show_label, settings_label, quit_label) = menu_labels(language);

    let show = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", settings_label, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;

    Menu::with_items(app, &[&show, &settings, &quit])
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

    // Embed the tray icon explicitly. Relying on the executable's cached icon
    // made Windows keep showing the previous design.
    builder = builder.icon(tauri::image::Image::from_bytes(include_bytes!(
        "../icons/tray-icon.png"
    ))?);

    builder.build(app)?;
    Ok(())
}

/// Rebuilds the menu after a language change.
pub fn set_language(app: &AppHandle, language: &str) -> Result<(), String> {
    let tray = app
        .tray_by_id(TRAY_ID)
        .ok_or_else(|| "Tray icon is not available".to_string())?;
    let menu = build_menu(app, language).map_err(|error| error.to_string())?;
    tray.set_menu(Some(menu)).map_err(|error| error.to_string())
}
