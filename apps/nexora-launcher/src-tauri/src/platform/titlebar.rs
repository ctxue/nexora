//! Native title bar coloring.
//!
//! The settings window keeps native decorations, so DWM paints its caption. It
//! is recolored with the page chrome to make the title bar and the body read as
//! one surface, and flipped for the dark theme.

use tauri::WebviewWindow;
use windows::Win32::{
    Foundation::HWND,
    Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
    },
};

use crate::config::{
    DARK_SETTINGS_CHROME, DARK_TITLE_TEXT, LIGHT_SETTINGS_CHROME, LIGHT_TITLE_TEXT,
};

fn set_attribute(
    hwnd: HWND,
    attribute: windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE,
    value: u32,
) {
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            attribute,
            &value as *const u32 as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

/// Applies the chrome colors matching the requested theme.
pub fn apply(window: &WebviewWindow, dark: bool) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let handle = HWND(hwnd.0);

    let (caption, text) = if dark {
        (DARK_SETTINGS_CHROME, DARK_TITLE_TEXT)
    } else {
        (LIGHT_SETTINGS_CHROME, LIGHT_TITLE_TEXT)
    };

    set_attribute(handle, DWMWA_CAPTION_COLOR, caption);
    set_attribute(handle, DWMWA_TEXT_COLOR, text);
    set_attribute(handle, DWMWA_USE_IMMERSIVE_DARK_MODE, u32::from(dark));
}
