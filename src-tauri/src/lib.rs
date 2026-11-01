//! Crate entry point.
//!
//! The binary only boots the Windows application; every concern lives in its
//! own module so the codebase stays navigable:
//!
//! - [`app`]          bootstraps Tauri (plugins, setup, window events)
//! - [`config`]       constants: sizes, limits, colors, labels, fallbacks
//! - [`models`]       serde payloads shared with the frontend
//! - [`state`]        state managed by Tauri
//! - [`commands`]     `#[tauri::command]` handlers, grouped by domain
//! - [`scanning`]     start-menu / file indexing and fuzzy search
//! - [`launcher`]     showing, hiding, and focusing the launcher window
//! - [`platform`]     Windows integration (hooks, wndproc, DWM, dialogs)
//! - [`tray`]         tray icon and its menu
//!
//! Files at the crate root (`apps`, `icons`, `shell_icon`) are thin wrappers
//! around Win32 / shell APIs used by the modules above.

mod app;
mod apps;
mod clipboard;
mod commands;
mod config;
mod icons;
mod launcher;
mod models;
mod platform;
mod scanning;
mod shell_icon;
mod state;
mod tray;

pub use app::run;
