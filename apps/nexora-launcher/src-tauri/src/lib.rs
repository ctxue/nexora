//! NEXORA launcher backend library.
//!
//! Tauri setup, Win32 integration, scanning, commands, and persisted state are
//! separated by responsibility. The binary entry point only starts this crate.

mod app;
mod apps;
mod clipboard;
mod commands;
mod config;
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
