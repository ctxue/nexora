//! NEXORA ISLE: a focused Windows desktop information island.
//!
//! - `app` / `pages`：与平台无关的界面与动效（外壳与录制器共用）
//! - `anim` / `paint` / `shape` / `theme`：弹簧、矢量绘制、面板轮廓、配色字体
//! - `model` / `lyrics` / `smtc` / `icons`：数据、歌词、系统媒体、文件图标
//! - `native` / `tray`：仅 Windows 的窗口区域、命中穿透与托盘

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
