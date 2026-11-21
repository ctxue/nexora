//! NEXORA launcher backend library.
//!
//! Tauri setup, Win32 integration, scanning, commands, and persisted state are
//! separated by responsibility. The binary entry point only starts this crate.

mod app;
mod apps;
mod browser_bridge;
mod browser_data;
mod clipboard;
mod commands;
mod config;
mod github;
mod icons;
mod launcher;
mod models;
mod platform;
mod preview;
mod scanning;
mod shell_icon;
mod state;
mod tray;
mod windows;

pub use app::run;
pub use browser_bridge::run_native_host_if_requested;
