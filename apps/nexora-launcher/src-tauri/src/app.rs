//! Tauri application bootstrap.
//!
//! Plugins, managed state, startup setup, and window event routing are composed
//! here; feature logic remains in the `commands` and `launcher` modules.

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_window_state::StateFlags as WindowStateFlags;

use crate::{
    clipboard::{start_monitor, ClipboardHistory},
    commands,
    config::{
        AUTOSTART_FLAG, DEFAULT_SHORTCUT, EVENT_LAUNCHER_HIDDEN, FALLBACK_SHORTCUT, FOCUS_GRACE,
        LAUNCHER_WINDOW, SETTINGS_WINDOW,
    },
    launcher::{apply_window_icon, show_launcher, toggle_launcher, CAPTURE_KEYS},
    platform::{hooks, titlebar, wndproc},
    state::{ActiveShortcut, FileIndex, LauncherState},
    tray,
};

fn register_initial_shortcut(app: &tauri::AppHandle) {
    let shortcuts = app.global_shortcut();
    let mut error = None;

    let initial = if shortcuts.register(DEFAULT_SHORTCUT).is_ok() {
        Some(DEFAULT_SHORTCUT)
    } else {
        error = Some("conflict".to_string());
        if shortcuts.register(FALLBACK_SHORTCUT).is_ok() {
            Some(FALLBACK_SHORTCUT)
        } else {
            None
        }
    };

    let state = app.state::<ActiveShortcut>();
    if let Some(initial) = initial {
        if let Ok(mut current) = state.current.lock() {
            *current = Some(initial.to_string());
        }
    }
    if let Ok(mut state_error) = state.error.lock() {
        *state_error = error;
    }

    hooks::set_alt_space_enabled(initial == Some(DEFAULT_SHORTCUT));
}

fn has_shortcut_error(app: &tauri::AppHandle) -> bool {
    app.state::<ActiveShortcut>()
        .error
        .lock()
        .ok()
        .and_then(|error| error.clone())
        .is_some()
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    register_initial_shortcut(app.handle());
    hooks::install(app.handle().clone());
    wndproc::install(app.handle());

    if let Some(settings) = app.get_webview_window(SETTINGS_WINDOW) {
        titlebar::apply(&settings, false);
    }

    for label in [LAUNCHER_WINDOW, SETTINGS_WINDOW] {
        if let Some(window) = app.get_webview_window(label) {
            apply_window_icon(&window);
        }
    }

    tray::setup(app)?;

    let plugin_root = app.path().app_data_dir()?.join("plugins");
    std::fs::create_dir_all(&plugin_root)?;
    app.manage(commands::plugins::PluginHost::new(plugin_root));
    let clipboard_path = app.path().app_data_dir()?.join("clipboard-history.bin");
    let clipboard = ClipboardHistory::new(clipboard_path);
    app.manage(clipboard.clone());
    start_monitor(app.handle().clone(), clipboard);

    if let Some(settings) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = settings.hide();
    }

    let launched_at_startup = std::env::args().any(|arg| arg == AUTOSTART_FLAG);
    if !launched_at_startup {
        if has_shortcut_error(app.handle()) {
            if let Err(error) = commands::open_settings_window(app.handle().clone()) {
                eprintln!("{error}");
            }
        } else {
            show_launcher(app.handle());
        }
    }

    Ok(())
}

fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if window.label() == SETTINGS_WINDOW {
                hooks::set_shortcut_capture(false);
                let _ = window.hide();
            } else {
                if let Some(state) = window.try_state::<LauncherState>() {
                    if let Ok(mut selected) = state.selected_text.lock() {
                        selected.clear();
                    }
                }
                let _ = window.emit(EVENT_LAUNCHER_HIDDEN, ());
                let _ = window.hide();
            }
        }
        WindowEvent::Focused(false) => {
            if window.label() != LAUNCHER_WINDOW {
                return;
            }
            let Some(state) = window.try_state::<LauncherState>() else {
                return;
            };

            let dragging = state.dragging.lock().map(|value| *value).unwrap_or(false);
            let shown_recently = state
                .shown_at
                .lock()
                .ok()
                .and_then(|value| *value)
                .map(|shown_at| shown_at.elapsed() < FOCUS_GRACE)
                .unwrap_or(false);

            if dragging || shown_recently {
                return;
            }

            if let Some(state) = window.try_state::<LauncherState>() {
                if let Ok(mut selected) = state.selected_text.lock() {
                    selected.clear();
                }
            }
            let _ = window.emit(EVENT_LAUNCHER_HIDDEN, ());
            let _ = window.hide();
        }
        _ => {}
    }
}

use tauri::Emitter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(FileIndex::default())
        .manage(LauncherState::default())
        .manage(ActiveShortcut::default())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_FLAG]),
        ))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    WindowStateFlags::POSITION
                        | WindowStateFlags::SIZE
                        | WindowStateFlags::MAXIMIZED,
                )
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_launcher(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        CAPTURE_KEYS.store(true, std::sync::atomic::Ordering::Relaxed);
                        toggle_launcher(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::apps::list_apps,
            commands::apps::get_quick_paths,
            commands::apps::start_file_index,
            commands::apps::search_files,
            commands::apps::get_icons,
            commands::apps::open_path,
            commands::actions::reveal_path,
            commands::actions::open_with,
            commands::actions::run_as_admin,
            commands::actions::read_clipboard_text,
            commands::actions::get_selected_text,
            commands::browsers::list_browsers,
            commands::browsers::open_url,
            commands::drives::list_drives,
            commands::library::pick_executable,
            commands::library::pick_file,
            commands::library::pick_folder,
            commands::library::scan_folder_apps,
            commands::library::scan_index_folders,
            commands::library::launch_program,
            commands::plugins::list_plugins,
            commands::plugins::install_plugin,
            commands::plugins::uninstall_plugin,
            commands::plugins::stop_plugin,
            commands::plugins::plugins_dir,
            commands::plugins::search_plugin,
            commands::preview::preview_file,
            commands::windows::list_windows,
            commands::windows::control_window,
            commands::clipboard::list_clipboard,
            commands::clipboard::copy_clipboard,
            commands::clipboard::toggle_clipboard_pin,
            commands::clipboard::delete_clipboard_item,
            commands::clipboard::clear_clipboard,
            commands::clipboard::set_clipboard_capture,
            commands::window::quit_app,
            commands::settings::open_settings_window,
            commands::shortcuts::set_shortcut_capture,
            commands::shortcuts::get_shortcut_status,
            commands::startup::get_launch_at_startup,
            commands::startup::set_launch_at_startup,
            commands::window::start_window_drag,
            commands::shortcuts::set_global_shortcut,
            commands::settings::set_tray_language,
            commands::settings::set_app_theme,
            commands::settings::export_settings,
            commands::settings::import_settings,
            commands::settings::reset_window_position
        ])
        .setup(|app| setup(app))
        .on_window_event(on_window_event)
        .build(tauri::generate_context!())
        .expect("error while building NEXORA");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(host) = app_handle.try_state::<commands::plugins::PluginHost>() {
                host.shutdown_all();
            }
        }
    });
}
