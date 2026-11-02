//! Top-level window discovery and management for the window switcher.

use std::path::Path;

use windows::{
    core::{BOOL, PWSTR},
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM},
        Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
        System::Threading::{
            GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
            IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindowAsync, SW_MINIMIZE,
            SW_RESTORE, WM_CLOSE,
        },
    },
};

use crate::models::WindowEntry;

struct WindowCollector {
    current_pid: u32,
    windows: Vec<WindowEntry>,
}

pub fn should_include_window(
    title: &str,
    visible: bool,
    current_pid: u32,
    process_pid: u32,
) -> bool {
    visible && !title.trim().is_empty() && current_pid != process_pid
}

fn window_title(hwnd: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; length as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..copied.max(0) as usize])
}

fn process_details(pid: u32) -> (String, Option<String>) {
    if pid == 0 {
        return ("Unknown".to_string(), None);
    }

    let Ok(process) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) })
    else {
        return ("Unknown".to_string(), None);
    };

    let mut buffer = vec![0u16; 32_768];
    let mut size = buffer.len() as u32;
    let path = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    }
    .ok()
    .map(|_| String::from_utf16_lossy(&buffer[..size as usize]));
    let _ = unsafe { CloseHandle(process) };

    let name = path
        .as_deref()
        .and_then(|value| Path::new(value).file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("Unknown")
        .to_string();
    (name, path)
}

unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let collector = unsafe { &mut *(lparam.0 as *mut WindowCollector) };
    let visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    if !visible {
        return BOOL(1);
    }

    let mut cloaked = 0u32;
    let cloaked_result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
    };
    if cloaked_result.is_ok() && cloaked != 0 {
        return BOOL(1);
    }

    let title = window_title(hwnd);
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if !should_include_window(&title, visible, collector.current_pid, pid) {
        return BOOL(1);
    }

    let (process_name, process_path) = process_details(pid);
    collector.windows.push(WindowEntry {
        id: (hwnd.0 as isize).to_string(),
        title,
        process_name,
        process_path,
        minimized: unsafe { IsIconic(hwnd).as_bool() },
    });
    BOOL(1)
}

pub fn list_windows() -> Vec<WindowEntry> {
    let mut collector = WindowCollector {
        current_pid: unsafe { GetCurrentProcessId() },
        windows: Vec::new(),
    };
    let _ = unsafe {
        EnumWindows(
            Some(collect_window),
            LPARAM((&mut collector as *mut WindowCollector) as isize),
        )
    };
    collector.windows
}

fn parse_hwnd(value: &str) -> Result<HWND, String> {
    value
        .parse::<isize>()
        .map(|value| HWND(value as *mut std::ffi::c_void))
        .map_err(|_| "Invalid window handle".to_string())
}

pub fn control_window(window_id: &str, action: &str) -> Result<(), String> {
    let hwnd = parse_hwnd(window_id)?;
    match action {
        "activate" => {
            unsafe {
                let _ = ShowWindowAsync(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
            }
            Ok(())
        }
        "minimize" => {
            unsafe {
                let _ = ShowWindowAsync(hwnd, SW_MINIMIZE);
            }
            Ok(())
        }
        "close" => unsafe {
            PostMessageW(Some(hwnd), WM_CLOSE, Default::default(), Default::default())
                .map_err(|error| format!("Could not close window: {error}"))
        },
        _ => Err("Unsupported window action".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::should_include_window;

    #[test]
    fn excludes_hidden_empty_and_own_windows() {
        assert!(!should_include_window("Notepad", false, 10, 20));
        assert!(!should_include_window("   ", true, 10, 20));
        assert!(!should_include_window("NEXORA", true, 10, 10));
    }

    #[test]
    fn includes_other_visible_titled_windows() {
        assert!(should_include_window("Untitled - Notepad", true, 10, 20));
    }
}
