use crate::{
    app::{App, Options},
    model::{self, page},
};
use std::sync::{Mutex, MutexGuard};

static DATA_DIR_LOCK: Mutex<()> = Mutex::new(());

fn data_dir_guard() -> MutexGuard<'static, ()> {
    DATA_DIR_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn isolated_data_dir(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "nexora-isle-ui-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn shell_uses_the_four_retained_pages_and_navigation() {
    let _guard = data_dir_guard();
    let data = isolated_data_dir("pages");
    std::env::set_var("NEXORA_ISLE_DATA_DIR", &data);

    let ctx = eframe::egui::Context::default();
    let mut app = App::new(&ctx, Options::default());

    assert_eq!(model::page::COUNT, 7);
    assert_eq!(app.settings.page_order, model::page::ALL);
    assert_eq!(app.page(), page::MUSIC);

    app.show_page(page::SHELF);
    assert_eq!(app.page(), page::SHELF);
    app.show_page(page::CLIPBOARD);
    assert_eq!(app.page(), page::CLIPBOARD);
    app.show_page(page::MAIL);
    assert_eq!(app.page(), page::MAIL);
    assert_eq!(page::retained_name(page::SHELF), "文件口袋");
    assert_eq!(page::retained_name(page::CLIPBOARD), "剪贴板");
    assert_eq!(page::retained_name(page::MONITOR), "监控");
    assert_eq!(crate::pages::CLIP_ROWS, 5);
    assert_eq!(crate::pages::MAIL_ROWS, 5);

    std::env::remove_var("NEXORA_ISLE_DATA_DIR");
    let _ = std::fs::remove_dir_all(data);
}

#[test]
fn default_settings_do_not_offer_removed_features() {
    let _guard = data_dir_guard();
    let settings = model::Settings::default();
    assert_eq!(settings.visible_pages.len(), 7);
    assert_eq!(settings.page_order.len(), 7);
    assert_eq!(settings.monitor_cpu_threshold, 85);
    assert_eq!(settings.monitor_memory_threshold, 85);
    assert_eq!(settings.monitor_gpu_threshold, 90);
    assert_eq!(settings.page_order, page::ALL);
    assert_eq!(settings.island_shape, model::IslandShape::Notch);
    assert!(settings.pet);
}

#[test]
fn switching_pages_does_not_move_page_contents_horizontally() {
    let _guard = data_dir_guard();
    let data = isolated_data_dir("page-motion");
    std::env::set_var("NEXORA_ISLE_DATA_DIR", &data);

    let ctx = eframe::egui::Context::default();
    let mut app = App::new(&ctx, Options::default());
    app.go(page::MAIL, 1.0);

    assert_eq!(app.page_transition_offset(), 0.0);

    std::env::remove_var("NEXORA_ISLE_DATA_DIR");
    let _ = std::fs::remove_dir_all(data);
}

#[test]
fn minimal_mode_suppresses_resident_clock_and_expression_during_hover() {
    let _guard = data_dir_guard();
    let data = isolated_data_dir("minimal-transition");
    std::env::set_var("NEXORA_ISLE_DATA_DIR", &data);
    let ctx = eframe::egui::Context::default();
    let mut app = App::new(&ctx, Options::default());

    app.set_minimal(true);
    assert!(!app.resident_decoration_visible());
    assert!(!app.pet_enabled());
    app.set_minimal(false);
    assert!(app.resident_decoration_visible());

    std::env::remove_var("NEXORA_ISLE_DATA_DIR");
    let _ = std::fs::remove_dir_all(data);
}

#[test]
fn music_and_video_pages_only_appear_while_that_media_is_active() {
    let _guard = data_dir_guard();
    let data = isolated_data_dir("dynamic-pages");
    std::env::set_var("NEXORA_ISLE_DATA_DIR", &data);
    let ctx = eframe::egui::Context::default();
    let mut app = App::new(&ctx, Options::default());

    assert!(!app.page_available(page::MUSIC));
    assert!(!app.page_available(page::VIDEO));
    app.media = crate::smtc::NowPlaying {
        title: "Track".into(),
        artist: "Artist".into(),
        playing: true,
        playback_type: crate::smtc::MediaKind::Music,
        ..Default::default()
    };
    assert!(app.page_available(page::MUSIC));
    app.media = Default::default();
    app.video = crate::smtc::NowPlaying {
        title: "Video".into(),
        source: "msedge.exe".into(),
        playing: true,
        playback_type: crate::smtc::MediaKind::Video,
        ..Default::default()
    };
    assert!(app.page_available(page::VIDEO));

    std::env::remove_var("NEXORA_ISLE_DATA_DIR");
    let _ = std::fs::remove_dir_all(data);
}

#[test]
fn sustained_cpu_load_creates_a_monitor_event() {
    let _guard = data_dir_guard();
    let data = isolated_data_dir("monitor-alert");
    std::env::set_var("NEXORA_ISLE_DATA_DIR", &data);
    let ctx = eframe::egui::Context::default();
    let mut app = App::new(&ctx, Options::default());
    app.settings.monitor_cpu_threshold = 80;
    app.settings.monitor_memory_threshold = 90;
    app.settings.monitor_gpu_threshold = 95;

    for at_secs in 0..=5 {
        app.monitor_snapshot(crate::monitor::Snapshot {
            sample: crate::monitor::Sample {
                at_secs: at_secs as f64,
                cpu_percent: 91.0,
                ..Default::default()
            },
            top_cpu: vec![crate::monitor::GroupUsage {
                name: "busy.exe".into(),
                cpu_percent: 91.0,
                ..Default::default()
            }],
            ..Default::default()
        });
    }
    assert_eq!(app.monitor_events.len(), 1);
    assert_eq!(app.monitor_events[0].app, "busy.exe");
    assert!(
        !app.alerts.is_empty(),
        "monitor alerts must use the default island notification queue"
    );
    app.tick_alerts();
    assert!(
        app.notice.is_some(),
        "the next frame should show the monitor alert on the collapsed island"
    );
    assert!(crate::monitor::load_events()
        .iter()
        .any(|event| event.app == "busy.exe"));

    std::env::remove_var("NEXORA_ISLE_DATA_DIR");
    let _ = std::fs::remove_dir_all(data);
}

#[test]
fn clipboard_wheel_capture_is_limited_to_the_content_region() {
    assert!(crate::app::clipboard_content_contains(eframe::egui::pos2(
        209.0, 120.0
    )));
    assert!(!crate::app::clipboard_content_contains(eframe::egui::pos2(
        20.0, 120.0
    )));
    assert!(!crate::app::clipboard_content_contains(eframe::egui::pos2(
        209.0, 240.0
    )));
}

#[test]
fn image_viewer_expands_the_island_only_while_viewing() {
    let _guard = data_dir_guard();
    let data = isolated_data_dir("image-viewer-height");
    std::env::set_var("NEXORA_ISLE_DATA_DIR", &data);
    let ctx = eframe::egui::Context::default();
    let mut app = App::new(&ctx, Options::default());
    app.show_page(page::IMAGES);
    let normal_width = app.panel_width();
    let normal_height = app.expanded_height();
    app.image_view_open = true;
    let viewer_width = app.panel_width();
    let viewer_height = app.expanded_height();
    assert!(
        viewer_width >= normal_width + 150.0,
        "viewer should use a substantially wider island"
    );
    assert!(
        viewer_height >= normal_height + 150.0,
        "viewer should use a substantially taller island"
    );
    app.w.snap(crate::app::PW);
    app.sync_panel_layout();
    assert!(
        app.w.pos < crate::app::IMAGE_VIEW_W - 1.0,
        "viewer resize must animate instead of snapping"
    );
    assert_eq!(app.w.target, crate::app::IMAGE_VIEW_W);
    app.image_view_open = false;
    assert_eq!(app.panel_width(), normal_width);
    assert_eq!(app.expanded_height(), normal_height);

    std::env::remove_var("NEXORA_ISLE_DATA_DIR");
    let _ = std::fs::remove_dir_all(data);
}

#[test]
fn reopening_the_same_image_page_keeps_the_active_viewer() {
    let _guard = data_dir_guard();
    let data = isolated_data_dir("image-viewer-persist");
    std::env::set_var("NEXORA_ISLE_DATA_DIR", &data);
    let ctx = eframe::egui::Context::default();
    let mut app = App::new(&ctx, Options::default());
    app.show_page(page::IMAGES);
    app.image_view_open = true;
    app.image_view_index = 2;
    app.show_page(page::IMAGES);
    assert!(app.image_view_open);
    assert_eq!(app.image_view_index, 2);

    std::env::remove_var("NEXORA_ISLE_DATA_DIR");
    let _ = std::fs::remove_dir_all(data);
}

#[test]
fn content_origin_stays_on_the_final_axis_from_the_first_open_frame() {
    let _guard = data_dir_guard();
    let data = isolated_data_dir("content-origin");
    std::env::set_var("NEXORA_ISLE_DATA_DIR", &data);
    let ctx = eframe::egui::Context::default();
    let mut app = App::new(&ctx, Options::default());
    app.settings.dock = model::Dock::Top;
    let win = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(100.0, 0.0),
        eframe::egui::vec2(crate::app::WIN_W, crate::app::WIN_H),
    );

    app.open = true;
    app.w.snap(220.0);
    app.h.snap(160.0);
    let first = app.animated_content_origin(win).x;
    assert_eq!(first, win.center().x - crate::app::PW / 2.0);

    app.w.snap(crate::app::PW * 1.05);
    assert_eq!(app.animated_content_origin(win).x, first);

    std::env::remove_var("NEXORA_ISLE_DATA_DIR");
    let _ = std::fs::remove_dir_all(data);
}
