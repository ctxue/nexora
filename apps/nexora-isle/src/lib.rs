//! NEXORA ISLE library surface.
//!
//! The root crate owns the state machine, rendering, persistence, and Windows
//! integration. `main` only supplies the native shell; tests and recording
//! tools drive the same `App` surface without platform-specific entry points.

pub mod anim;
pub mod app;
mod avoidance;
mod bar_layout;
pub mod bt;
pub mod clipboard;
mod compact_list;
mod controls;
pub mod display_layout;
mod extras;
pub mod i18n;
pub mod icons;
pub mod input;
mod lucide;
pub mod lyrics;
pub mod mail;
mod mail_privacy;
mod mail_reader;
pub mod model;
pub mod monitor;
mod monitor_ui;
pub mod network;
pub mod otp;
mod pages;
pub mod paint;
mod pet;
pub mod poll_clock;
pub mod product;
mod resident_settings;
pub mod resources;
pub mod settings_theme;
pub mod shape;
pub mod side_dock;
pub mod smtc;
#[cfg(windows)]
pub mod system;
pub mod theme;
mod updates;

#[cfg(windows)]
pub mod native;
#[cfg(windows)]
pub mod tray;
#[cfg(windows)]
pub mod wechat;

#[cfg(test)]
mod ui_tests;
