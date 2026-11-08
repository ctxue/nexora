use std::path::PathBuf;

use tauri::Manager;
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::HWND,
        System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED},
        UI::{
            Controls::Dialogs::{
                GetOpenFileNameW, GetSaveFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR,
                OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
            },
            Shell::{
                SHBrowseForFolderW, SHGetPathFromIDListW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS,
                BROWSEINFOW,
            },
        },
    },
};

use crate::{
    config::{
        ADD_APP_TITLE, ANY_FILE_DIALOG_FILTER, CONFIG_DIALOG_BUFFER, CONFIG_DIALOG_FILTER,
        CONFIG_EXPORT_EXTENSION, CONFIG_EXPORT_FILE_NAME, CONFIG_EXPORT_TITLE, CONFIG_IMPORT_TITLE,
        EXE_DIALOG_FILTER, LAUNCHER_WINDOW, PICK_FILE_TITLE, PICK_FOLDER_TITLE, SETTINGS_WINDOW,
    },
    platform::hooks::hook_app,
};

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn owner_window() -> HWND {
    hook_app()
        .and_then(|app| {
            app.get_webview_window(SETTINGS_WINDOW)
                .or_else(|| app.get_webview_window(LAUNCHER_WINDOW))
        })
        .and_then(|window| window.hwnd().ok())
        .map(|handle| HWND(handle.0))
        .unwrap_or_default()
}

fn start_directory() -> Vec<u16> {
    let directory = dirs::document_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_default();
    wide(&directory.to_string_lossy())
}

fn buffer_path(buffer: &[u16]) -> PathBuf {
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    PathBuf::from(String::from_utf16_lossy(&buffer[..end]))
}

pub fn ask_save_path() -> Option<PathBuf> {
    let mut buffer = vec![0u16; CONFIG_DIALOG_BUFFER];
    let suggested = wide(CONFIG_EXPORT_FILE_NAME);
    buffer[..suggested.len()].copy_from_slice(&suggested);

    let filter = wide(CONFIG_DIALOG_FILTER);
    let directory = start_directory();
    let title = wide(CONFIG_EXPORT_TITLE);
    let extension = wide(CONFIG_EXPORT_EXTENSION);

    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner_window(),
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        lpstrInitialDir: PCWSTR(directory.as_ptr()),
        lpstrTitle: PCWSTR(title.as_ptr()),
        lpstrDefExt: PCWSTR(extension.as_ptr()),
        Flags: OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };

    if !unsafe { GetSaveFileNameW(&mut dialog) }.as_bool() {
        return None;
    }

    Some(buffer_path(&buffer))
}

fn ask_file(filter: &str, title: &str) -> Option<PathBuf> {
    let mut buffer = vec![0u16; CONFIG_DIALOG_BUFFER];
    let filter = wide(filter);
    let directory = start_directory();
    let title = wide(title);

    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner_window(),
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        lpstrInitialDir: PCWSTR(directory.as_ptr()),
        lpstrTitle: PCWSTR(title.as_ptr()),
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };

    if !unsafe { GetOpenFileNameW(&mut dialog) }.as_bool() {
        return None;
    }

    Some(buffer_path(&buffer))
}

pub fn ask_open_path() -> Option<PathBuf> {
    ask_file(CONFIG_DIALOG_FILTER, CONFIG_IMPORT_TITLE)
}

pub fn ask_executable() -> Option<PathBuf> {
    ask_file(EXE_DIALOG_FILTER, ADD_APP_TITLE)
}

pub fn ask_file_any() -> Option<PathBuf> {
    ask_file(ANY_FILE_DIALOG_FILTER, PICK_FILE_TITLE)
}

pub fn ask_folder() -> Option<PathBuf> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };

    let title = wide(PICK_FOLDER_TITLE);
    let mut display = vec![0u16; 260];

    let browse = BROWSEINFOW {
        hwndOwner: owner_window(),
        pszDisplayName: PWSTR(display.as_mut_ptr()),
        lpszTitle: PCWSTR(title.as_ptr()),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
        ..Default::default()
    };

    let pidl = unsafe { SHBrowseForFolderW(&browse) };
    if pidl.is_null() {
        if initialized {
            unsafe { CoUninitialize() };
        }
        return None;
    }

    let mut path = [0u16; 260];
    let ok = unsafe { SHGetPathFromIDListW(pidl, &mut path) }.as_bool();
    unsafe { CoTaskMemFree(Some(pidl as *const _)) };

    if initialized {
        unsafe { CoUninitialize() };
    }

    if !ok {
        return None;
    }

    let end = path
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(path.len());
    Some(PathBuf::from(String::from_utf16_lossy(&path[..end])))
}
