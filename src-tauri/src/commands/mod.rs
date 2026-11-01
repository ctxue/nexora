//! `#[tauri::command]` handlers, one module per domain.
//!
//! Every handler is re-exported here so `app.rs` can hand a single list to
//! `tauri::generate_handler!`.

pub mod apps;
pub mod browsers;
pub mod drives;
pub mod library;
pub mod plugins;
pub mod settings;
pub mod shortcuts;
pub mod startup;
pub mod window;

pub use settings::open_settings_window;
