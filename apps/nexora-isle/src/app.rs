//! Primary NEXORA ISLE application state and frame renderer.
//!
//! Frame time is supplied by the shell so animations are deterministic. Panel
//! geometry, content geometry, notification badges, and hit regions are kept
//! separate because their animation and clipping rules differ.

use crate::anim::{self, lerp, smoothstep, Spring};
use crate::bar_layout::{self as bar, Layout};
use crate::extras::{Alert, PetState};
use crate::lucide::{self, Icon};
use crate::lyrics;
use crate::mail;
use crate::model;
use crate::paint::{self, Card, H};
use crate::shape;
use crate::smtc::{NowPlaying, Transport};
use crate::theme::{self, size, W};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

pub const PW: f32 = 419.0; // 展开面板宽
pub const IMAGE_VIEW_W: f32 = 600.0; // 图片查看态宽度
pub const PH: f32 = 160.0; // 展开面板高：顶部标题栏 + 内容区
pub const BAR_W: f32 = 163.0; // 收起药丸宽
pub const BAR_H: f32 = 30.0; // 收起药丸高
pub(crate) const HUD_W: f32 = 260.0;
pub(crate) const LYRIC_BAR_W: f32 = 300.0;
pub(crate) const HUD_TRACK_X: f32 = 48.0;
pub(crate) const HUD_TRACK_W: f32 = 150.0;
pub const BAR_W_MIN: f32 = 44.0; // 极简小条
pub const BAR_H_MIN: f32 = 4.5;
pub const FLARE: f32 = 17.6; // 顶部外翻肩宽
pub const FLARE_H: f32 = 16.0; // 外翻收敛高度
pub const RAD: f32 = 33.0; // 底部圆角
pub const WIN_W: f32 = 640.0; // 窗口（透明画布）
pub const CANVAS_W: f32 = WIN_W + 320.0; // 保留右侧通知及阴影的完整空间，原岛仍按 WIN_W 定位。
pub const WIN_H: f32 = 390.0;
pub const PILL_GAP: f32 = 6.0;

fn pill_shadow(painter: &egui::Painter, rect: Rect, radius: f32, strength: f32) {
    if strength <= 0.01 {
        return;
    }
    for (grow, a) in [(10.0f32, 0.035f32), (6.5, 0.05), (3.5, 0.07), (1.5, 0.09)] {
        let r = Rect::from_min_max(
            rect.min - Vec2::new(grow * 0.6, grow * 0.3),
            rect.max + Vec2::new(grow * 0.6, grow),
        );
        painter.rect_filled(
            r,
            radius + grow * 0.8,
            Color32::from_black_alpha((255.0 * a * strength) as u8),
        );
    }
}
pub(crate) const PAGE_NAMES: [&str; model::page::COUNT] =
    ["音乐", "视频", "监控", "文件口袋", "图片", "剪贴板", "邮箱"];
const PAGE_SETTINGS_TABS: [usize; model::page::COUNT] = [1, 1, 12, 5, 6, 6, 8];

const HOVER_INTENT: f64 = 0.20;
const MINIMAL_HOVER_INTENT: f64 = 0.40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointerExpandAction {
    Keep,
    Open,
    Close,
}

fn pointer_expand_action(
    expand_on_hover: bool,
    open: bool,
    inside: bool,
    clicked: bool,
    hover_elapsed: Option<f64>,
    hover_intent: f64,
    drag_hot: bool,
) -> PointerExpandAction {
    let triggered = if expand_on_hover {
        hover_elapsed.is_some_and(|elapsed| elapsed >= hover_intent)
    } else {
        inside && clicked
    };
    if !open && (drag_hot || triggered) {
        PointerExpandAction::Open
    } else {
        PointerExpandAction::Keep
    }
}

fn pointer_collapse_action(open: bool, inside: bool) -> PointerExpandAction {
    if open && !inside {
        PointerExpandAction::Close
    } else {
        PointerExpandAction::Keep
    }
}

const LEAVE_DELAY: f64 = 0.12;
const SWELL: (f32, f32) = (1.075, 1.13);
const OPEN_W: (f32, f32) = (330.0, 0.70);
const OPEN_H: (f32, f32) = (290.0, 0.74);
const CLOSE: (f32, f32) = (620.0, 1.0);
const SWELL_K: (f32, f32) = (420.0, 0.8);

pub struct Options {
    pub fetch_lyrics: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { fetch_lyrics: true }
    }
}

#[derive(Clone, Default)]
pub struct Env {
    pub t: f64,
    pub clock: String,
    pub battery: Option<u8>,
    pub charging: bool,
    pub network: Option<bool>,
    pub cursor: Option<Pos2>,
    pub browser_at_top: Option<bool>,
    pub media: NowPlaying,
    pub volume: Option<(u32, bool)>,
    pub bt: Vec<crate::bt::Device>,
    pub bt_ready: bool,
    pub screen: Option<Rect>,
    pub opaque_surface: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Messenger {
    Wechat,
    Qq,
}
impl Messenger {
    pub fn name(self) -> &'static str {
        self.name_for(crate::i18n::Language::Zh)
    }

    pub fn name_for(self, language: crate::i18n::Language) -> &'static str {
        match self {
            Self::Wechat => language.text("微信", "WeChat"),
            Self::Qq => "QQ",
        }
    }
}

pub(crate) fn messenger_alerts_enabled(settings: &model::Settings, source: Messenger) -> bool {
    settings.notifications
        && match source {
            Messenger::Wechat => settings.wechat_notifications,
            Messenger::Qq => settings.qq_notifications,
        }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unread {
    pub source: Messenger,
    pub pid: u32,
    pub window: isize,
}
impl Unread {
    pub fn of(source: Messenger) -> Self {
        Self {
            source,
            pid: 0,
            window: 0,
        }
    }
}

pub(crate) const UNREAD_W: f32 = 40.0;
pub(crate) const UNREAD_GAP: f32 = 8.0;
pub(crate) const UNREAD_SLOT: f32 = 36.0;
fn unread_step(_horizontal: bool) -> f32 {
    22.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VideoSite {
    YouTube,
    Bilibili,
    Douyin,
    IQiyi,
    Youku,
    TencentVideo,
    Netflix,
    Twitch,
    Browser,
}

fn video_site(media: &NowPlaying) -> VideoSite {
    let text = format!("{} {}", media.title, media.album).to_lowercase();
    if text.contains("youtube") {
        VideoSite::YouTube
    } else if text.contains("bilibili") || text.contains("哔哩哔哩") {
        VideoSite::Bilibili
    } else if text.contains("douyin") || text.contains("抖音") {
        VideoSite::Douyin
    } else if text.contains("iqiyi") || text.contains("爱奇艺") {
        VideoSite::IQiyi
    } else if text.contains("youku") || text.contains("优酷") {
        VideoSite::Youku
    } else if text.contains("v.qq.com") || text.contains("腾讯视频") {
        VideoSite::TencentVideo
    } else if text.contains("netflix") {
        VideoSite::Netflix
    } else if text.contains("twitch") {
        VideoSite::Twitch
    } else {
        VideoSite::Browser
    }
}

fn draw_video_session_badge(
    c: &Card,
    media: &NowPlaying,
    artwork: Option<&(std::sync::Arc<egui::ColorImage>, egui::TextureHandle)>,
    cx: f32,
    cy: f32,
    size: f32,
) {
    let rect = c.rect(cx - size / 2.0, cy - size / 2.0, size, size);
    if let Some((image, texture)) = artwork {
        let [width, height] = image.size;
        let uv_size = if width > height {
            egui::vec2(height as f32 / width as f32, 1.0)
        } else {
            egui::vec2(1.0, width as f32 / height as f32)
        };
        let uv = Rect::from_center_size(Pos2::new(0.5, 0.5), uv_size);
        c.painter.add(
            egui::epaint::RectShape::filled(rect, 5.0 * c.s, Color32::WHITE)
                .with_texture(texture.id(), uv),
        );
    } else {
        match video_site(media) {
            VideoSite::YouTube => {
                c.fill(
                    cx - size / 2.0,
                    cy - size / 2.0,
                    size,
                    size,
                    5.0,
                    Color32::from_rgb(255, 38, 38),
                );
                c.poly(
                    &[(cx - 2.5, cy - 4.0), (cx + 4.0, cy), (cx - 2.5, cy + 4.0)],
                    Color32::WHITE,
                );
            }
            VideoSite::Bilibili => {
                c.fill(
                    cx - size / 2.0,
                    cy - size / 2.0,
                    size,
                    size,
                    5.0,
                    Color32::from_rgb(251, 114, 153),
                );
                c.centered_text(cx, cy, "B", size * 0.55, W::Bold, Color32::WHITE, H::Center);
            }
            VideoSite::Douyin => {
                c.fill(
                    cx - size / 2.0,
                    cy - size / 2.0,
                    size,
                    size,
                    5.0,
                    Color32::from_rgb(24, 24, 27),
                );
                c.centered_text(
                    cx,
                    cy,
                    "抖",
                    size * 0.45,
                    W::Bold,
                    Color32::WHITE,
                    H::Center,
                );
            }
            VideoSite::IQiyi => {
                c.fill(
                    cx - size / 2.0,
                    cy - size / 2.0,
                    size,
                    size,
                    5.0,
                    Color32::from_rgb(0, 190, 6),
                );
                c.centered_text(cx, cy, "i", size * 0.58, W::Bold, Color32::WHITE, H::Center);
            }
            VideoSite::Youku => {
                c.fill(
                    cx - size / 2.0,
                    cy - size / 2.0,
                    size,
                    size,
                    5.0,
                    Color32::from_rgb(22, 119, 255),
                );
                c.centered_text(
                    cx,
                    cy,
                    "优",
                    size * 0.45,
                    W::Bold,
                    Color32::WHITE,
                    H::Center,
                );
            }
            VideoSite::TencentVideo => {
                c.fill(
                    cx - size / 2.0,
                    cy - size / 2.0,
                    size,
                    size,
                    5.0,
                    Color32::from_rgb(255, 122, 0),
                );
                c.centered_text(
                    cx,
                    cy,
                    "腾",
                    size * 0.45,
                    W::Bold,
                    Color32::WHITE,
                    H::Center,
                );
            }
            VideoSite::Netflix => {
                c.fill(
                    cx - size / 2.0,
                    cy - size / 2.0,
                    size,
                    size,
                    5.0,
                    Color32::from_rgb(8, 8, 10),
                );
                c.centered_text(
                    cx,
                    cy,
                    "N",
                    size * 0.55,
                    W::Bold,
                    Color32::from_rgb(229, 9, 20),
                    H::Center,
                );
            }
            VideoSite::Twitch => {
                c.fill(
                    cx - size / 2.0,
                    cy - size / 2.0,
                    size,
                    size,
                    5.0,
                    Color32::from_rgb(145, 70, 255),
                );
                c.centered_text(cx, cy, "T", size * 0.55, W::Bold, Color32::WHITE, H::Center);
            }
            VideoSite::Browser => {
                c.fill(
                    cx - size / 2.0,
                    cy - size / 2.0,
                    size,
                    size,
                    5.0,
                    Color32::from_gray(42),
                );
                lucide::card(c, Icon::PlayCircle, cx, cy, size * 0.7, Color32::WHITE);
                return;
            }
        }
    }
    c.circle(
        cx + size * 0.31,
        cy + size * 0.31,
        2.1,
        if media.playing {
            theme::GREEN
        } else {
            theme::GRAY
        },
    );
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PageNavLayout {
    dots_left: f32,
}

pub(crate) const CONTENT_LEFT: f32 = 30.0;
pub(crate) const CONTENT_RIGHT: f32 = PW - CONTENT_LEFT;
pub(crate) const CONTENT_WIDTH: f32 = CONTENT_RIGHT - CONTENT_LEFT;
pub(crate) const NAV_ZONE: f32 = 18.0;
const NOTICE_STACK_GAP: f32 = 6.0;
const HEADER_ARROW_LEFT_X: f32 = 14.0;
const HEADER_ARROW_RIGHT_X: f32 = PW - 14.0;
const HEADER_LEFT_X: f32 = CONTENT_LEFT;
const HEADER_RIGHT_X: f32 = CONTENT_RIGHT;

fn right_anchored_x(current_width: f32, reference_width: f32, reference_x: f32) -> f32 {
    current_width + reference_x - reference_width
}

fn opened_content_width(current_width: f32, target_width: f32, stable: bool) -> f32 {
    if stable {
        target_width
    } else {
        current_width
    }
}

fn resident_music_icon_visible(
    lyrics_active: bool,
    has_music: bool,
    progress: f32,
    has_music_slot: bool,
) -> bool {
    has_music && !lyrics_active && (progress > 0.1 || has_music_slot)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TopCenterNoticeLayout {
    pub(crate) rect: Rect,
}

impl TopCenterNoticeLayout {
    pub(crate) fn at(area: Rect, y: f32, size: Vec2) -> Self {
        let size = Vec2::new(
            size.x.clamp(0.0, area.width()),
            size.y.clamp(0.0, area.height()),
        );
        let y = y.clamp(area.top(), (area.bottom() - size.y).max(area.top()));
        Self {
            rect: Rect::from_min_size(Pos2::new(area.center().x - size.x / 2.0, y), size),
        }
    }

    pub(crate) fn below(area: Rect, anchor: Rect, size: Vec2) -> Self {
        Self::at(area, anchor.bottom() + NOTICE_STACK_GAP, size)
    }
}

fn page_nav_layout(page_count: usize, center_x: f32) -> PageNavLayout {
    const DOT_SLOT: f32 = 16.0;
    const DOT_GAP: f32 = 4.0;
    let total = DOT_SLOT * page_count as f32 + DOT_GAP * page_count.saturating_sub(1) as f32;
    let dots_left = center_x - total / 2.0;
    PageNavLayout { dots_left }
}

fn content_bounds(panel_height: f32, panel_width: f32) -> Rect {
    Rect::from_min_max(
        Pos2::new(0.0, 33.0),
        Pos2::new(panel_width, (panel_height - NAV_ZONE).max(33.0)),
    )
}

#[test]
fn top_center_notice_layout_centers_and_stacks_alerts() {
    let area = Rect::from_min_size(Pos2::new(40.0, 12.0), Vec2::new(640.0, 270.0));
    let notice = TopCenterNoticeLayout::at(area, 48.0, Vec2::new(334.0, 44.0));
    assert_eq!(
        notice.rect,
        Rect::from_min_size(Pos2::new(193.0, 48.0), Vec2::new(334.0, 44.0))
    );

    let details = TopCenterNoticeLayout::below(area, notice.rect, Vec2::new(260.0, 48.0));
    assert_eq!(
        details.rect,
        Rect::from_min_size(Pos2::new(230.0, 98.0), Vec2::new(260.0, 48.0))
    );
}

pub(crate) fn clipboard_content_contains(cursor: Pos2) -> bool {
    Rect::from_min_max(
        Pos2::new(CONTENT_LEFT, 36.0),
        Pos2::new(CONTENT_RIGHT, 148.0),
    )
    .contains(cursor)
}

pub(crate) fn image_content_contains(cursor: Pos2) -> bool {
    Rect::from_min_max(
        Pos2::new(CONTENT_LEFT, 36.0),
        Pos2::new(CONTENT_RIGHT, 108.0),
    )
    .contains(cursor)
}

fn side_nav_center_y(panel_height: f32) -> f32 {
    panel_height / 2.0
}

pub(crate) fn centered_window_position(monitor: Vec2, window: Vec2) -> Pos2 {
    Pos2::new(
        ((monitor.x - window.x) * 0.5).max(0.0),
        ((monitor.y - window.y) * 0.5).max(0.0),
    )
}

pub(crate) fn centered_viewport_position(ctx: &egui::Context, window: Vec2) -> Option<Pos2> {
    ctx.input(|input| input.viewport().monitor_size)
        .map(|monitor| centered_window_position(monitor, window))
}

pub(crate) fn centered_decorated_viewport_position(
    ctx: &egui::Context,
    inner: Vec2,
) -> Option<Pos2> {
    centered_viewport_position(ctx, inner + Vec2::new(16.0, 39.0))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Media(Transport),
    FocusMedia(String),
    Open(PathBuf),
    ViewMessenger(Unread),
    OpenUrl(String),
    Reveal(PathBuf),
    Copy(String),
    Notify(String, String),
    RequestNotifications,
    EnableWechatAlerts,
    DisconnectNotifications,
    ToggleAutostart,
    AddFiles,
    DragFile(PathBuf),
    SaveImageAs {
        suggested_name: String,
        png: std::sync::Arc<Vec<u8>>,
    },
    SetVolume(u32),
    ToggleMute,
    MailSync,
    MailTest(mail::Account, String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum WindowShape {
    Outline(Vec<Pos2>),
    Notch {
        rect: Rect,
        flare: f32,
        flare_h: f32,
        radius: f32,
    },
    Docked {
        rect: Rect,
        dock: model::Dock,
        flare: f32,
        flare_h: f32,
        radius: f32,
    },
    Round(Rect, f32),
}

impl WindowShape {
    pub fn outline(&self) -> Vec<Pos2> {
        match self {
            Self::Outline(points) => points.clone(),
            Self::Notch {
                rect,
                flare,
                flare_h,
                radius,
            } => shape::outline(*rect, *flare, *flare_h, *radius),
            Self::Docked {
                rect,
                dock,
                flare,
                flare_h,
                radius,
            } => shape::outline_docked(*rect, *dock, *flare, *flare_h, *radius),
            Self::Round(r, radius) => shape::outline(*r, 0.0, 0.5, *radius),
        }
    }
}

pub struct FrameOut {
    pub region: Rect,
    pub window_shape: Option<Vec<WindowShape>>,
    pub hit: Rect,
    pub notice_hit: Option<Rect>,
    pub file_drop_zone: Option<Rect>,
    pub expanded: bool,
    pub editing: bool,
    pub animating: bool,
    pub repaint_after: std::time::Duration,
    pub cmds: Vec<Cmd>,
}

const NATIVE_RETRIES: u32 = 12;
const NETWORK_RETRIES: u32 = 10;

fn lyric_key_matches(key: &str, media: &NowPlaying) -> bool {
    use std::io::Write;
    let Some(rest) = key
        .strip_prefix(media.title.as_str())
        .and_then(|rest| rest.strip_prefix('|'))
    else {
        return false;
    };
    let Some(rest) = rest
        .strip_prefix(media.artist.as_str())
        .and_then(|rest| rest.strip_prefix('|'))
    else {
        return false;
    };
    let mut buffer = [0u8; 32];
    let mut cursor = std::io::Cursor::new(&mut buffer[..]);
    if write!(cursor, "{:.0}", media.duration).is_err() {
        return rest == format!("{:.0}", media.duration);
    }
    let written = cursor.position() as usize;
    rest.as_bytes() == &buffer[..written]
}

#[test]
fn lyric_key_matches_the_formatted_key_without_allocating() {
    for (title, artist, duration) in [
        ("歌", "手", 120.4),
        ("A|B", "C", 0.0),
        ("歌", "", 239.5),
        ("歌", "手", 240.5),
        ("", "", f64::NAN),
        ("t", "a", 1e30),
    ] {
        let media = NowPlaying {
            title: title.into(),
            artist: artist.into(),
            duration,
            ..Default::default()
        };
        let key = format!("{}|{}|{:.0}", title, artist, duration);
        assert!(lyric_key_matches(&key, &media), "{key}");
        assert!(!lyric_key_matches(&format!("{key}0"), &media));
        assert!(!lyric_key_matches(&format!("x{key}"), &media));
    }
}

#[test]
fn collapsed_video_badge_uses_the_detected_site_or_browser_fallback() {
    let mut media = NowPlaying {
        title: "Video - YouTube".into(),
        ..Default::default()
    };
    assert_eq!(video_site(&media), VideoSite::YouTube);
    media.title = "示例视频 - 哔哩哔哩".into();
    assert_eq!(video_site(&media), VideoSite::Bilibili);
    media.title = "示例视频 - 抖音".into();
    assert_eq!(video_site(&media), VideoSite::Douyin);
    media.title = "迷雾剧场 - 爱奇艺".into();
    assert_eq!(video_site(&media), VideoSite::IQiyi);
    media.title = "经典片段 - 优酷".into();
    assert_eq!(video_site(&media), VideoSite::Youku);
    media.title = "纪录片 - 腾讯视频".into();
    assert_eq!(video_site(&media), VideoSite::TencentVideo);
    media.title = "Series - Netflix".into();
    assert_eq!(video_site(&media), VideoSite::Netflix);
    media.title = "Live - Twitch".into();
    assert_eq!(video_site(&media), VideoSite::Twitch);
    media.title = "Learn Rust".into();
    assert_eq!(video_site(&media), VideoSite::Browser);
}

#[test]
fn page_header_keeps_the_indicator_centered() {
    let layout = page_nav_layout(4, PW / 2.0);
    assert!((layout.dots_left + 38.0 - PW / 2.0).abs() < 0.01);
    assert_eq!((HEADER_ARROW_LEFT_X, HEADER_ARROW_RIGHT_X), (14.0, 405.0));
    assert_eq!(PW - HEADER_RIGHT_X, HEADER_LEFT_X);
}

#[test]
fn right_anchored_icons_keep_their_inset_as_the_island_expands() {
    assert_eq!(right_anchored_x(348.0, 348.0, 327.0), 327.0);
    assert_eq!(right_anchored_x(419.0, 348.0, 327.0), 398.0);
    assert_eq!(right_anchored_x(419.0, 161.0, 145.0), 403.0);
}

#[test]
fn opened_content_uses_the_final_axis_from_the_first_animation_frame() {
    assert_eq!(opened_content_width(BAR_W, PW, true), PW);
    assert_eq!(opened_content_width(PW * 1.05, PW, true), PW);
    assert_eq!(opened_content_width(BAR_W, PW, false), BAR_W);
}

#[test]
fn click_mode_expands_only_after_a_click_while_hover_mode_keeps_the_old_timing() {
    assert_eq!(
        pointer_expand_action(false, false, true, false, Some(2.0), HOVER_INTENT, false,),
        PointerExpandAction::Keep
    );
    assert_eq!(
        pointer_expand_action(false, false, true, true, Some(0.0), HOVER_INTENT, false,),
        PointerExpandAction::Open
    );
    assert_eq!(
        pointer_expand_action(true, false, true, false, Some(0.19), HOVER_INTENT, false,),
        PointerExpandAction::Keep
    );
    assert_eq!(
        pointer_expand_action(true, false, true, false, Some(0.20), HOVER_INTENT, false,),
        PointerExpandAction::Open
    );
}

#[test]
fn leaving_island_closes_immediately_regardless_of_open_mode() {
    assert_eq!(
        pointer_collapse_action(true, false),
        PointerExpandAction::Close
    );
    assert_eq!(
        pointer_collapse_action(true, false),
        PointerExpandAction::Close
    );
    assert_eq!(
        pointer_collapse_action(true, true),
        PointerExpandAction::Keep
    );
}

#[test]
fn wechat_and_qq_alerts_have_independent_switches() {
    let mut settings = model::Settings::default();
    assert!(messenger_alerts_enabled(&settings, Messenger::Wechat));
    assert!(messenger_alerts_enabled(&settings, Messenger::Qq));
    settings.wechat_notifications = false;
    assert!(!messenger_alerts_enabled(&settings, Messenger::Wechat));
    assert!(messenger_alerts_enabled(&settings, Messenger::Qq));
    settings.notifications = false;
    assert!(!messenger_alerts_enabled(&settings, Messenger::Qq));
}

#[test]
fn lyrics_header_owns_the_only_music_icon_during_transition() {
    assert!(!resident_music_icon_visible(true, true, 0.2, true));
    assert!(!resident_music_icon_visible(true, true, 0.0, true));
    assert!(resident_music_icon_visible(false, true, 0.2, true));
    assert!(resident_music_icon_visible(false, true, 0.0, true));
    assert!(!resident_music_icon_visible(false, true, 0.0, false));
}

#[test]
fn content_and_transient_copy_stay_inside_their_layout_region() {
    assert_eq!(
        content_bounds(116.0, PW),
        Rect::from_min_max(Pos2::new(0.0, 33.0), Pos2::new(PW, 98.0))
    );
    assert_eq!(
        content_bounds(10.0, PW),
        Rect::from_min_max(Pos2::new(0.0, 33.0), Pos2::new(PW, 33.0))
    );
    assert_eq!(side_nav_center_y(116.0), 58.0);
}

#[test]
fn popup_windows_are_centered_on_the_active_monitor() {
    assert_eq!(
        centered_window_position(egui::vec2(2560.0, 1440.0), egui::vec2(820.0, 620.0)),
        egui::pos2(870.0, 410.0)
    );
    assert_eq!(
        centered_window_position(egui::vec2(640.0, 480.0), egui::vec2(820.0, 620.0)),
        egui::Pos2::ZERO
    );
    assert_eq!(
        centered_window_position(
            egui::vec2(2560.0, 1440.0),
            egui::vec2(820.0, 620.0) + egui::vec2(16.0, 39.0)
        ),
        egui::pos2(862.0, 390.5)
    );
}

#[derive(Default)]
pub(crate) struct LyricsState {
    pub key: String,
    pub island_animation_started: bool,
    pub scroll_animation_started: bool,
    pub loading: bool,
    pub error: Option<lyrics::FetchError>,
    pub retry_at: Option<f64>,
    pub song_id: Option<u64>,
    pub attempts: u32,
    pub lyrics: Option<lyrics::Lyrics>,
    pub rx: Option<std::sync::mpsc::Receiver<Result<(lyrics::Lyrics, bool), lyrics::FetchError>>>,
    pub native_rx: Option<std::sync::mpsc::Receiver<Option<lyrics::Lyrics>>>,
}

#[derive(Default)]
pub(crate) struct MailState {
    pub privacy: crate::mail_privacy::Privacy,
    pub accounts: Vec<mail::Account>,
    pub status: HashMap<String, mail::Status>,
    pub recent: Vec<mail::Item>,
    pub note: String,
    pub testing: Option<String>,
    pub form: MailForm,
    pub open_uid: Option<(String, u32)>,
    pub reading: Option<mail::Item>,
    pub reader_focus: bool,
}

#[derive(Default)]
pub(crate) struct MailForm {
    pub provider: mail::Provider,
    pub custom: mail::CustomImap,
    pub email: String,
    pub password: String,
    pub show: bool,
}

impl MailState {
    pub fn code_item(&self) -> Option<&mail::Item> {
        self.recent.iter().find(|m| m.code.is_some())
    }
    pub fn status_of(&self, email: &str) -> mail::Status {
        self.status
            .get(&email.to_ascii_lowercase())
            .cloned()
            .unwrap_or(mail::Status::Idle)
    }
}

pub struct App {
    pub opts: Options,
    pub settings: model::Settings,
    pub(crate) shelf: model::Shelf,

    pub(crate) w: Spring,
    pub(crate) h: Spring,
    pub(crate) open: bool,
    hover_since: Option<f64>,
    leave_since: Option<f64>,
    pub hover_lock: bool,
    pub(crate) reveal: f32,

    pub(crate) page: usize,
    page_from: usize,
    page_t0: f64,
    dots: [Spring; model::page::COUNT],
    wheel_delta: f32,
    wheel_at: f64,
    wheel_page_at: f64,

    pub(crate) drag_hot: bool,
    pub(crate) drag_a: f32,
    drag_saved: Option<usize>,

    pub(crate) shelf_scroll: Spring,
    pub(crate) shelf_first: usize,
    pub(crate) shelf_menu_rect: Option<Rect>,
    pub(crate) shelf_x: HashMap<PathBuf, Spring>,
    pub(crate) shelf_born: HashMap<PathBuf, f64>,
    pub(crate) icons: crate::icons::Cache,
    pub(crate) avoidance: crate::avoidance::Picker,
    pub(crate) confirm_clear: Option<f64>,
    pub clipboard: crate::clipboard::Shared,
    pub(crate) clip_first: usize,
    pub(crate) image_first: usize,
    pub(crate) image_view_open: bool,
    pub(crate) image_view_index: usize,
    pub(crate) image_textures:
        HashMap<String, (std::sync::Arc<egui::ColorImage>, egui::TextureHandle)>,
    pub(crate) clip_confirm_clear: Option<f64>,
    last_clipboard_version: u64,
    last_clipboard_count: usize,
    last_clipboard_head: Option<String>,
    last_missing_shelf_files: usize,
    pub(crate) monitor_history: crate::monitor::History,
    pub(crate) monitor_latest: crate::monitor::Snapshot,
    pub(crate) monitor_metric: crate::monitor::Metric,
    pub(crate) monitor_show_events: bool,
    pub(crate) monitor_events: std::collections::VecDeque<crate::monitor::MonitorEvent>,
    monitor_alert_state: crate::monitor::AlertState,

    pub(crate) ly: LyricsState,
    pub(crate) ly_worker: Option<std::thread::JoinHandle<()>>,
    pub(crate) show_lyrics: bool,
    island_lyric_layout: Option<std::sync::Arc<egui::Galley>>,
    media_anchor: (String, f64, f64, bool),
    pub(crate) media: NowPlaying,
    pub(crate) music_artwork: Option<(std::sync::Arc<egui::ColorImage>, egui::TextureHandle)>,
    pub(crate) video: NowPlaying,
    pub(crate) video_artwork: Option<(std::sync::Arc<egui::ColorImage>, egui::TextureHandle)>,

    forced_open_until: f64,
    pub settings_open: bool,
    pub displays: Vec<crate::display_layout::Display>,
    browser_at_top: bool,
    browser_minimal_active: bool,
    screen: Option<Rect>,
    pub(crate) settings_focus_requested: bool,
    pub(crate) settings_tab: usize,
    pub wechat_status: String,
    pub(crate) notification_apps: model::NotificationApps,
    pub listener_status: String,
    pub(crate) notification_bell_until: f64,
    pub(crate) notification_bell_at: f64,
    pub(crate) notice: Option<Alert>,
    pub(crate) hud_notice: Option<Alert>,
    hud_alpha: f32,
    notice_width: Spring,
    notice_details: bool,
    pub pending_messengers: Vec<Unread>,
    messenger_scroll: f32,
    pub(crate) alerts: std::collections::VecDeque<Alert>,
    pub(crate) history: std::collections::VecDeque<Alert>,
    pub(crate) pending: Vec<Cmd>,
    pub(crate) pet: crate::pet::Pet,
    toast: Option<(String, f64)>,
    last_cursor: Option<Pos2>,
    pet_a: f32,
    last_t: Option<f64>,
    pub(crate) t: f64,
    pub(crate) dt: f32,
    pub(crate) cursor_local: Option<Pos2>,
    started: bool,
    pub(crate) volume_ui: Option<(u32, bool)>,
    pub(crate) volume_quiet_until: f64,
    pub(crate) volume_a: f32,
    pub(crate) bt_shown: Vec<crate::bt::Device>,
    bt_initialized: bool,
    pub(crate) mail: MailState,
    pub(crate) updates: crate::updates::State,
    pub(crate) resident: bar::Motion,
    pub(crate) resident_power: (Option<u8>, bool),
}

impl App {
    pub fn new(ctx: &egui::Context, opts: Options) -> Self {
        theme::load_fonts(ctx);
        egui_extras::install_image_loaders(ctx);
        ctx.set_theme(egui::ThemePreference::Dark);
        ctx.set_visuals(egui::Visuals::dark());
        ctx.style_mut(|s| {
            s.interaction.selectable_labels = false;
            s.visuals.text_cursor.stroke = egui::Stroke::new(1.5_f32, theme::BLUE);
            s.visuals.selection.bg_fill = theme::alpha_blue(0.45);
        });
        let settings = model::Settings::load();
        let clipboard_on = settings.clipboard_history;
        let clipboard_images = settings.clipboard_images;
        let (bw, bh) = if settings.minimal {
            (BAR_W_MIN, BAR_H_MIN)
        } else {
            (BAR_W, BAR_H)
        };
        let initial_lyrics = settings.lyrics_enabled;
        let language = settings.language;
        let initial_page = settings
            .page_order
            .iter()
            .copied()
            .find(|page| settings.visible_pages[*page])
            .unwrap_or(0);
        let clipboard = crate::clipboard::Shared::new(clipboard_on, clipboard_images);
        let (last_clipboard_version, last_clipboard_count, last_clipboard_head) = {
            let history = clipboard.lock();
            (
                history.version,
                history.items.len(),
                history.items.first().map(|item| item.text.clone()),
            )
        };
        let shelf = model::Shelf::load();
        let last_missing_shelf_files = shelf.items.iter().filter(|path| !path.exists()).count();
        Self {
            opts,
            settings,
            shelf,
            w: Spring::new(bw, CLOSE.0, CLOSE.1),
            h: Spring::new(bh, CLOSE.0, CLOSE.1),
            open: false,
            hover_since: None,
            leave_since: None,
            hover_lock: false,
            reveal: 0.0,
            page: initial_page,
            page_from: initial_page,
            page_t0: -10.0,
            dots: std::array::from_fn(|i| {
                Spring::new(if i == initial_page { 16.0 } else { 4.0 }, 380.0, 0.86)
            }),
            wheel_delta: 0.0,
            wheel_at: -1.0,
            wheel_page_at: -1.0,
            drag_hot: false,
            drag_a: 0.0,
            drag_saved: None,
            shelf_scroll: Spring::new(0.0, 300.0, 0.9),
            shelf_first: 0,
            shelf_menu_rect: None,
            shelf_x: HashMap::new(),
            shelf_born: HashMap::new(),
            icons: Default::default(),
            avoidance: Default::default(),
            confirm_clear: None,
            clipboard,
            clip_first: 0,
            image_first: 0,
            image_view_open: false,
            image_view_index: 0,
            image_textures: HashMap::new(),
            clip_confirm_clear: None,
            last_clipboard_version,
            last_clipboard_count,
            last_clipboard_head,
            last_missing_shelf_files,
            monitor_history: Default::default(),
            monitor_latest: Default::default(),
            monitor_metric: crate::monitor::Metric::Cpu,
            monitor_show_events: false,
            monitor_events: crate::monitor::load_events().into(),
            monitor_alert_state: Default::default(),
            ly: LyricsState::default(),
            ly_worker: None,
            show_lyrics: initial_lyrics,
            island_lyric_layout: None,
            media_anchor: (String::new(), 0.0, 0.0, false),
            media: NowPlaying::default(),
            music_artwork: None,
            video: NowPlaying::default(),
            video_artwork: None,
            forced_open_until: 0.0,
            settings_open: false,
            displays: Vec::new(),
            browser_at_top: false,
            browser_minimal_active: false,
            screen: None,
            settings_focus_requested: false,
            settings_tab: 0,
            wechat_status: String::new(),
            notification_apps: model::NotificationApps::load(),
            listener_status: language.text("尚未连接", "Not connected").into(),
            notification_bell_until: 0.0,
            notification_bell_at: -10.0,
            notice: None,
            hud_notice: None,
            hud_alpha: 0.0,
            notice_width: Spring::new(0.0, CLOSE.0, CLOSE.1),
            notice_details: false,
            pending_messengers: Vec::new(),
            messenger_scroll: 0.0,
            alerts: Default::default(),
            history: Default::default(),
            pending: vec![],
            pet: Default::default(),
            toast: None,
            last_cursor: None,
            pet_a: 0.0,
            last_t: None,
            t: 0.0,
            dt: 0.0,
            cursor_local: None,
            started: false,
            volume_ui: None,
            volume_quiet_until: 0.0,
            volume_a: 0.0,
            bt_shown: Vec::new(),
            bt_initialized: false,
            mail: MailState {
                accounts: mail::load_accounts(),
                ..Default::default()
            },
            updates: Default::default(),
            resident: Default::default(),
            resident_power: (None, false),
        }
    }

    pub fn say(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), self.t));
    }

    pub(crate) fn code_copied(&mut self, _code: &str) {
        self.say(
            self.settings
                .language
                .text("验证码已复制", "Verification code copied"),
        );
    }

    pub(crate) fn mail_code_hidden(&self, item: &mail::Item) -> bool {
        self.mail
            .privacy
            .hidden(item, self.t, self.settings.mail_code_hide_seconds)
    }
    pub(crate) fn reveal_mail_code(&mut self, item: &mail::Item) {
        self.mail.privacy.reveal(item, self.t);
    }
    pub(crate) fn display_mail(&self, item: &mail::Item) -> mail::Item {
        let mut safe = item.clone();
        if self.settings.language.is_en() && safe.subject == "（无主题）" {
            safe.subject = "(No subject)".into();
        }
        if self.mail_code_hidden(item) {
            if let Some(code) = &item.code {
                safe.subject = crate::mail_privacy::mask(&safe.subject, code);
                safe.from = crate::mail_privacy::mask(&safe.from, code);
                safe.summary = self.mail.privacy.masked_body(item);
                safe.links.clear(); // Avoid tooltip/URL/copy leaks while hidden.
                safe.code = None;
            }
        }
        safe
    }

    pub fn mail_event(&mut self, ev: mail::Event) {
        let watching = |email: &str| {
            self.mail
                .accounts
                .iter()
                .any(|a| a.enabled && a.email.eq_ignore_ascii_case(email))
        };
        match ev {
            mail::Event::Status { account, status } => {
                if !watching(&account) {
                    return;
                }
                self.mail
                    .status
                    .insert(account.to_ascii_lowercase(), status);
            }
            mail::Event::Mail { item, new } => {
                if !watching(&item.account) {
                    return;
                }
                self.mail
                    .privacy
                    .observe(&item, new, self.t, chrono::Utc::now().timestamp());
                self.mail.recent.retain(|m| {
                    !(m.account.eq_ignore_ascii_case(&item.account) && m.uid == item.uid)
                });
                self.mail.recent.insert(0, item);
                self.mail.recent.truncate(60);
                self.mail.privacy.retain(&self.mail.recent);
                if new && self.settings.mail_enabled {
                    let language = self.settings.language;
                    let item = &self.mail.recent[0];
                    let body = format!("{} · {}", item.from, item.subject);
                    let body = item
                        .code
                        .as_ref()
                        .map(|code| crate::mail_privacy::mask(&body, code))
                        .unwrap_or(body);
                    self.alert(language.text("新邮件", "New mail"), body, 5, false);
                    self.notification_bell_at = self.t;
                    self.notification_bell_until = self.t + 6.0;
                    self.pet.react(PetState::Idle);
                }
            }
            mail::Event::Test { account, ok, note } => {
                self.mail.testing = None;
                let language = self.settings.language;
                let detail = if ok {
                    language.text("连接成功", "Connected").to_string()
                } else {
                    note
                };
                let detail = if detail.chars().count() > 120 {
                    format!("{}…", detail.chars().take(120).collect::<String>())
                } else {
                    detail
                };
                self.mail.note = if language.is_en() {
                    format!("{account}: {detail}")
                } else {
                    format!("{account}：{detail}")
                };
                self.pet
                    .react(if ok { PetState::Naming } else { PetState::Idle });
                self.say(if ok {
                    if language.is_en() {
                        format!("{account} connected")
                    } else {
                        format!("{account} 连接成功")
                    }
                } else {
                    if language.is_en() {
                        format!("{account} connection failed")
                    } else {
                        format!("{account} 连接失败")
                    }
                });
            }
        }
    }

    pub fn monitor_snapshot(&mut self, snapshot: crate::monitor::Snapshot) {
        self.monitor_history.push(snapshot.sample.clone());
        if self.settings.monitor_alerts && self.settings.monitor_enabled {
            let fired = self.monitor_alert_state.observe(
                &snapshot.sample,
                self.settings.monitor_cpu_threshold,
                self.settings.monitor_memory_threshold,
                self.settings.monitor_gpu_threshold,
            );
            for metric in fired {
                let (value, threshold, app) = match metric {
                    crate::monitor::AlertMetric::Cpu => (
                        snapshot.sample.cpu_percent,
                        self.settings.monitor_cpu_threshold as f32,
                        snapshot
                            .top_cpu
                            .first()
                            .map(|item| item.name.clone())
                            .unwrap_or_default(),
                    ),
                    crate::monitor::AlertMetric::Memory => (
                        snapshot.sample.memory_percent,
                        self.settings.monitor_memory_threshold as f32,
                        snapshot
                            .top_memory
                            .first()
                            .map(|item| item.name.clone())
                            .unwrap_or_default(),
                    ),
                    crate::monitor::AlertMetric::Gpu => (
                        snapshot.sample.gpu_percent.unwrap_or(0.0),
                        self.settings.monitor_gpu_threshold as f32,
                        snapshot
                            .top_cpu
                            .iter()
                            .find(|item| item.gpu_percent.is_some())
                            .map(|item| item.name.clone())
                            .unwrap_or_default(),
                    ),
                };
                let event = crate::monitor::MonitorEvent {
                    at_ms: (snapshot.sample.at_secs * 1000.0) as i64,
                    metric,
                    value,
                    threshold,
                    app: app.clone(),
                };
                self.monitor_events.push_front(event);
                self.monitor_events.truncate(crate::monitor::MAX_EVENTS);
                let chronological: Vec<_> = self.monitor_events.iter().rev().cloned().collect();
                crate::monitor::save_events(&chronological);
                let (zh, en) = metric.title();
                let title = self.settings.language.text(zh, en);
                let body = if self.settings.language.is_en() {
                    format!("{value:.0}% (threshold {threshold:.0}%) · {app}")
                } else {
                    format!("{value:.0}%（阈值 {threshold:.0}%）· {app}")
                };
                self.alert(title, body, model::page::MONITOR, false);
            }
        }
        self.monitor_latest = snapshot;
    }

    pub fn open_mail_settings(&mut self) {
        self.settings_tab = 8;
        self.open_settings();
    }

    pub fn mail_accounts(&self) -> &[mail::Account] {
        &self.mail.accounts
    }

    pub(crate) fn mail_accounts_changed(&mut self) -> Result<(), String> {
        mail::save_accounts(&self.mail.accounts)?;
        self.pending.push(Cmd::MailSync);
        Ok(())
    }
    fn sync_device(&mut self, env: &Env) {
        if self.t >= self.volume_quiet_until {
            if let Some(v) = env.volume {
                self.volume_ui = Some(v);
            }
        }
        if !self.settings.bluetooth_notifications {
            self.bt_shown.clear();
            self.bt_initialized = false;
            self.alerts.retain(|a| {
                a.source != crate::extras::AlertSource::Internal || a.title != "蓝牙设备"
            });
            if self.notice.as_ref().is_some_and(|a| {
                a.source == crate::extras::AlertSource::Internal && a.title == "蓝牙设备"
            }) {
                self.notice = None;
            }
            return;
        }
        if !env.bt_ready {
            return;
        }
        if !self.bt_initialized {
            self.bt_initialized = true;
            self.bt_shown = env.bt.clone();
            return;
        }
        let connected: Vec<_> = env
            .bt
            .iter()
            .filter(|d| !self.bt_shown.iter().any(|old| old.id == d.id))
            .cloned()
            .collect();
        let disconnected: Vec<_> = self
            .bt_shown
            .iter()
            .filter(|d| !env.bt.iter().any(|new| new.id == d.id))
            .cloned()
            .collect();
        for device in &connected {
            self.bluetooth_alert(device, true);
        }
        for device in &disconnected {
            self.bluetooth_alert(device, false);
        }
        self.bt_shown = env.bt.clone();
    }

    pub fn show_page(&mut self, page: usize) {
        if !self.settings.visible_pages[page] {
            self.settings_tab = PAGE_SETTINGS_TABS[page];
            self.open_settings();
            return;
        }
        let was_viewer = self.image_view_open;
        let keep_viewer = self.image_view_open && self.page == page && page == model::page::IMAGES;
        if !keep_viewer {
            self.image_view_open = false;
            self.image_view_index = 0;
        }
        self.notice = None;
        self.go(page, 1.0);
        self.open = true;
        self.forced_open_until = self.t + 4.0;
        self.w.tune(OPEN_W.0, OPEN_W.1);
        self.h.tune(OPEN_H.0, OPEN_H.1);
        self.w.target = self.panel_width();
        self.h.target = self.expanded_height();
        if was_viewer {
            self.sync_panel_layout();
        }
    }
    pub fn open_settings(&mut self) {
        self.settings_open = true;
        self.settings_focus_requested = true;
    }
    pub fn page(&self) -> usize {
        self.page
    }
    pub fn notice_visible(&self) -> bool {
        self.notice.is_some()
    }

    pub(crate) fn page_available(&self, page: usize) -> bool {
        self.settings.visible_pages[page]
            && match page {
                model::page::MUSIC => self.media.is_music(),
                model::page::VIDEO => self.video.is_video(),
                model::page::MONITOR => self.settings.monitor_enabled,
                _ => true,
            }
    }

    fn available_pages(&self) -> Vec<usize> {
        self.settings
            .page_order
            .iter()
            .copied()
            .filter(|page| self.page_available(*page))
            .collect()
    }

    fn reconcile_dynamic_page(&mut self) {
        if self.page_available(self.page) {
            return;
        }
        if let Some(page) = self.available_pages().first().copied() {
            self.go(page, 1.0);
        }
    }

    #[cfg(test)]
    pub(crate) fn page_transition_offset(&self) -> f32 {
        0.0
    }

    pub(crate) fn set_page_visible(&mut self, page: usize, visible: bool) {
        if !visible
            && self
                .settings
                .visible_pages
                .iter()
                .filter(|enabled| **enabled)
                .count()
                == 1
            && self.settings.visible_pages[page]
        {
            return;
        }
        self.settings.visible_pages[page] = visible;
        if !self.settings.visible_pages[self.page] {
            if let Some(first) = self
                .settings
                .page_order
                .iter()
                .copied()
                .find(|page| self.settings.visible_pages[*page])
            {
                self.go(first, 1.0);
            }
        }
    }

    fn navigate_page(&mut self, next: bool) {
        let position = self
            .settings
            .page_order
            .iter()
            .position(|page| *page == self.page)
            .unwrap();
        for offset in 1..=PAGE_NAMES.len() {
            let page = self.settings.page_order[(position
                + if next {
                    offset
                } else {
                    PAGE_NAMES.len() - offset
                })
                % PAGE_NAMES.len()];
            if self.page_available(page) {
                self.go(page, if next { 1.0 } else { -1.0 });
                break;
            }
        }
    }

    pub fn foreground_changed(&mut self) {
        self.hover_lock = true;
    }

    pub(crate) fn go(&mut self, page: usize, _dir: f32) {
        if page == self.page || !self.settings.visible_pages[page] {
            return;
        }
        let was_viewer = self.image_view_open;
        self.image_view_open = false;
        self.image_view_index = 0;
        self.page_from = self.page;
        self.page = page;
        self.page_t0 = self.t;
        if self.open {
            self.w.target = self.panel_width();
            self.h.target = self.expanded_height();
            if was_viewer {
                self.sync_panel_layout();
            }
        }
        self.dots_target();
    }

    fn page_e(&self) -> f32 {
        (self.t - self.page_t0) as f32
    }

    fn dots_target(&mut self) {
        for (i, d) in self.dots.iter_mut().enumerate() {
            d.target = if i == self.page { 16.0 } else { 4.0 };
        }
    }

    fn island_lyrics_active(&self) -> bool {
        if self.side() {
            return false;
        }
        self.settings.island_lyrics
            && self.settings.visible_pages[model::page::MUSIC]
            && self.media.is_music()
            && !self.browser_minimal_active
            && lyric_key_matches(&self.ly.key, &self.media)
            && self.ly.lyrics.as_ref().is_some_and(|lyrics| {
                lyrics.synced && lyrics.lines.iter().any(|line| !line.text.trim().is_empty())
            })
    }
    pub(crate) fn compact_notices(&self) -> bool {
        self.island_lyrics_active()
            && !self.open
            && !self.notice.as_ref().is_some_and(|a| {
                self.settings.notification_preview
                    && matches!(a.source, crate::extras::AlertSource::External(_))
            })
    }

    pub(crate) fn lyric_position(&self) -> f32 {
        (self.media.position - self.settings.lyric_delay_ms as f64 / 1000.0) as f32
    }

    pub(crate) fn island_lyric(&self) -> Option<(usize, &str)> {
        if !self.island_lyrics_active() {
            return None;
        }
        if !lyric_key_matches(&self.ly.key, &self.media) {
            return None;
        }
        let lyrics = self.ly.lyrics.as_ref().filter(|lyrics| lyrics.synced)?;
        let index = lyrics.index_at(self.lyric_position())?;
        let text = lyrics.lines.get(index)?.text.trim();
        (!text.is_empty()).then_some((index, text))
    }

    fn update_island_lyric_layout(&mut self, ctx: &egui::Context) {
        if !self.island_lyrics_active() {
            self.island_lyric_layout = None;
            return;
        }
        let current = self.island_lyric();
        let language = self.settings.language;
        let text = if let Some((_, line)) = current {
            line
        } else if self.ly.loading || !lyric_key_matches(&self.ly.key, &self.media) {
            language.text("正在查找歌词…", "Finding lyrics…")
        } else if let Some(lyrics) = &self.ly.lyrics {
            if !lyrics.synced {
                language.text("暂无同步歌词", "No synced lyrics")
            } else if lyrics.index_at(self.lyric_position()).is_none() {
                language.text("前奏", "Intro")
            } else {
                language.text("间奏", "Interlude")
            }
        } else {
            ""
        };
        let ink = if current.is_some() {
            theme::WHITE
        } else {
            theme::GRAY
        };
        let layout = |text: String| {
            let mut job = egui::text::LayoutJob::simple(
                text,
                theme::font(11.0, W::Semibold),
                ink,
                LYRIC_BAR_W - 80.0,
            );
            job.halign = egui::Align::Center;
            job.wrap.break_anywhere = true;
            ctx.fonts(|fonts| fonts.layout_job(job))
        };
        let mut galley = layout(text.to_owned());
        if let Some((index, _)) = current.filter(|_| galley.rows.len() > 1) {
            let lyrics = self.ly.lyrics.as_ref().unwrap();
            let start = lyrics.lines[index].t;
            let end = lyrics
                .lines
                .iter()
                .skip(index + 1)
                .find(|line| line.t > start)
                .map_or(self.media.duration as f32, |line| line.t);
            let progress = if end > start {
                ((self.lyric_position() - start) / (end - start)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let total: usize = galley.rows.iter().map(|row| row.glyphs.len()).sum();
            let mut count = 0;
            let page = galley
                .rows
                .iter()
                .position(|row| {
                    count += row.glyphs.len();
                    progress * (total as f32) < count as f32
                })
                .unwrap_or(galley.rows.len() - 1);
            let segment = galley.rows[page]
                .glyphs
                .iter()
                .map(|glyph| glyph.chr)
                .collect::<String>();
            galley = layout(segment.trim().to_owned());
        }
        self.island_lyric_layout = Some(galley);
    }

    pub(crate) fn effective_minimal(&self) -> bool {
        self.settings.minimal || self.browser_minimal_active
    }
    pub(crate) fn pet_enabled(&self) -> bool {
        self.settings.pet && !self.effective_minimal() && !self.side()
    }
    pub(crate) fn resident_decoration_visible(&self) -> bool {
        !self.effective_minimal()
    }
    pub(crate) fn pet_space(&self) -> f32 {
        if self.pet_enabled() {
            44.0
        } else {
            0.0
        }
    }
    fn lyric_bar_width(&self) -> f32 {
        LYRIC_BAR_W + if self.pet_enabled() { 48.0 } else { 0.0 }
    }
    fn notice_size(&self, a: &Alert) -> (f32, f32) {
        let (width, height) =
            if a.source == crate::extras::AlertSource::Internal && a.title == "蓝牙设备" {
                (HUD_W, 48.0)
            } else if a.is_hud() {
                (HUD_W, 38.0)
            } else {
                (334.0, 44.0)
            };
        (width, height)
    }
    pub fn set_pet(&mut self, enabled: bool) {
        self.settings.pet = enabled;
        if enabled {
            self.settings.minimal = false;
        } else if !self.settings.minimal {
            self.settings.minimal = true;
        }
        self.settings.save();
    }
    pub fn set_minimal(&mut self, enabled: bool) {
        self.settings.minimal = enabled;
        if enabled {
            self.settings.pet = false;
        } else if !self.settings.pet {
            self.settings.pet = true;
        }
        self.settings.save();
    }
    fn base_size(&self) -> (f32, f32) {
        if !self.compact_notices() && !self.open {
            if let Some(a) = &self.notice {
                let (width, height) = self.notice_size(a);
                return (width + self.pet_space(), height);
            }
        }
        if self.side() {
            return self.side_base_size(
                self.effective_minimal()
                    && self.t >= self.notification_bell_until
                    && (self.browser_minimal_active || self.pending_messengers.is_empty()),
            );
        }
        if self.island_lyrics_active() {
            (self.lyric_bar_width(), BAR_H)
        } else if self.effective_minimal() && self.t >= self.notification_bell_until {
            (BAR_W_MIN, BAR_H_MIN)
        } else {
            (self.resident.target.width.min(WIN_W - 32.0), BAR_H)
        }
    }

    pub fn progress(&self) -> f32 {
        let (bw, bh) = self.base_size();
        if self.side() {
            return (self.w.pos - bw) / (self.panel_width() - bw).max(1.0);
        }
        (self.h.pos - bh) / (self.expanded_height() - bh)
    }

    pub(crate) fn expanded_height(&self) -> f32 {
        if (self.page == model::page::MAIL && self.mail.open_uid.is_some())
            || self.page == model::page::MONITOR
        {
            236.0 + NAV_ZONE
        } else if self.page == model::page::CLIPBOARD {
            154.0 + NAV_ZONE
        } else if self.page == model::page::IMAGES && self.image_view_open {
            276.0 + NAV_ZONE
        } else if matches!(self.page, model::page::SHELF | model::page::IMAGES) {
            120.0 + NAV_ZONE
        } else if self.page == model::page::MAIL {
            150.0 + NAV_ZONE
        } else {
            PH + NAV_ZONE
        }
    }

    pub(crate) fn panel_width(&self) -> f32 {
        if self.page == model::page::IMAGES && self.image_view_open {
            IMAGE_VIEW_W
        } else {
            PW
        }
    }

    pub(crate) fn sync_panel_layout(&mut self) {
        self.w.tune(OPEN_W.0, OPEN_W.1);
        self.h.tune(OPEN_H.0, OPEN_H.1);
        self.w.target = self.panel_width();
        self.h.target = self.expanded_height();
    }

    pub(crate) fn animated_content_origin(&self, win: Rect) -> Pos2 {
        self.island_rect(
            win,
            self.content_layout_width(),
            opened_content_width(
                self.h.pos,
                self.expanded_height(),
                self.open || self.reveal > 0.0,
            ),
        )
        .min
    }

    pub(crate) fn content_layout_width(&self) -> f32 {
        opened_content_width(
            self.w.pos,
            self.panel_width(),
            self.open || self.reveal > 0.0,
        )
    }

    pub fn frame(&mut self, ui: &mut egui::Ui, win: Rect, env: &Env) -> FrameOut {
        let ctx = ui.ctx().clone();
        let dt = match self.last_t {
            Some(t0) => (env.t - t0).clamp(0.0, 0.1) as f32,
            None => 0.0,
        };
        let elapsed = self
            .last_t
            .map(|at| (env.t - at).max(0.0) as f32)
            .unwrap_or(0.0);
        self.last_t = Some(env.t);
        self.t = env.t;
        self.dt = dt;
        if let Some(at_top) = env.browser_at_top {
            self.browser_at_top = at_top;
        }
        self.screen = env.screen;
        let browser_minimal = self.settings.browser_minimal && self.browser_at_top;
        if browser_minimal && !self.browser_minimal_active {
            self.open = false;
            self.hover_lock = true;
            self.hover_since = None;
            self.leave_since = None;
            self.forced_open_until = 0.0;
        }
        self.browser_minimal_active = browser_minimal;
        if !self.started {
            self.started = true;
            self.page_t0 = env.t - 10.0;
        }
        let mut cmds = Vec::new();

        self.tick(dt, elapsed);
        self.sync_device(env);
        self.tick_alerts();
        let hud_active = self.notice.clone();
        if hud_active.as_ref().map(|a| &a.title) != self.hud_notice.as_ref().map(|a| &a.title)
            || hud_active.is_none()
        {
            self.notice_details = false;
        }
        if let Some(a) = &hud_active {
            self.hud_notice = Some(a.clone());
        }
        self.hud_alpha = anim::approach(
            self.hud_alpha,
            if hud_active.is_some() { 1.0 } else { 0.0 },
            if hud_active.is_some() { 16.0 } else { 8.0 },
            dt,
        );
        if hud_active.is_none() && self.hud_alpha < 0.003 {
            self.hud_notice = None;
        }
        let notice_target = if !self
            .notice
            .as_ref()
            .is_some_and(|a| a.messenger().is_some())
            && self.compact_notices()
            && (self.notice.is_some() || self.t < self.notification_bell_until)
        {
            44.0
        } else {
            0.0
        };
        self.notice_width.target = notice_target;
        if self.compact_notices() {
            self.notice_width.step(dt);
        } else {
            self.notice_width.snap(0.0);
            self.notice_details = false;
        }
        self.update_media(env, &ctx);
        self.reconcile_dynamic_page();
        let bell = self.t < self.notification_bell_until && !self.compact_notices();
        self.resident_power = (env.battery, env.charging);
        let mut layout = Layout::new(
            self.settings.show_battery && env.battery.is_some(),
            (self.media.is_music() || self.video.is_video()) && !bell,
            bell,
            self.pet_enabled(),
        );
        layout.reorder(&self.settings.resident_order);
        if self.settings.clock_period != 0 {
            layout.widen_clock(40.0);
        }
        if self.pet_enabled() {
            if let Some(a) = self
                .notice
                .as_ref()
                .filter(|_| !self.compact_notices() && !self.open)
            {
                layout.x[bar::PET] = Some(22.0 - (self.notice_size(a).0 + self.pet_space()) / 2.0);
            } else if self.island_lyrics_active() {
                layout.x[bar::PET] = Some(60.0 - self.lyric_bar_width() / 2.0);
            }
        }
        self.resident.step(
            layout,
            dt,
            ctx.style().animation_time > 0.0 && bar::animations_enabled(),
        );
        if self.opts.fetch_lyrics {
            self.poll_lyrics();
        }
        self.update_island_lyric_layout(&ctx);

        let cx = win.center().x;
        let pill = self.settings.island_shape == model::IslandShape::Pill;
        let top = win.top() + if pill { PILL_GAP } else { 0.0 };
        let side = self.side();
        let dock = self.dock();

        let (hovering_files, dropped) = ctx.input(|i| {
            (
                self.settings.visible_pages[model::page::SHELF] && !i.raw.hovered_files.is_empty(),
                i.raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect::<Vec<_>>(),
            )
        });
        if !dropped.is_empty() && self.settings.visible_pages[model::page::SHELF] {
            self.on_drop(dropped);
            self.show_page(model::page::SHELF);
        } else if hovering_files && !self.drag_hot {
            self.drag_hot = true;
            self.pet.react(PetState::Play);
            if self.page != model::page::SHELF {
                self.drag_saved = Some(self.page);
                self.go(
                    model::page::SHELF,
                    if self.page < model::page::SHELF {
                        1.0
                    } else {
                        -1.0
                    },
                );
            }
        } else if !hovering_files && self.drag_hot {
            self.drag_hot = false;
            if let Some(p) = self.drag_saved.take() {
                self.go(p, if p < model::page::SHELF { -1.0 } else { 1.0 });
            }
        }
        self.drag_a = anim::approach(self.drag_a, if self.drag_hot { 1.0 } else { 0.0 }, 16.0, dt);

        let p_prev = self.progress();
        let hit = if side {
            self.side_hit_rect(win, p_prev)
        } else {
            self.hit_rect(cx, top, p_prev)
        };
        let previous_menu = self.shelf_menu_rect.take();
        let inside = env
            .cursor
            .is_some_and(|c| hit.contains(c) || previous_menu.is_some_and(|r| r.contains(c)));
        let on_notice = env.cursor.is_some_and(|c| {
            self.messenger_badge_rect(win, self.island_rect(win, self.w.pos, self.h.pos))
                .is_some_and(|r| r.contains(c))
                || self.notice_rect(win, top).is_some_and(|r| r.contains(c))
                || self
                    .notice_details_rect(win, top)
                    .is_some_and(|r| r.contains(c))
        });
        if !inside {
            self.hover_lock = false;
        }
        if env.cursor != self.last_cursor || ctx.input(|i| !i.events.is_empty()) {
            self.last_cursor = env.cursor;
        }
        if inside && self.open && self.reveal > 0.95 {
            let (scroll, left, right, esc) = ctx.input(|i| {
                (
                    i.raw_scroll_delta.y,
                    i.key_pressed(egui::Key::ArrowLeft),
                    i.key_pressed(egui::Key::ArrowRight),
                    i.key_pressed(egui::Key::Escape),
                )
            });
            if esc {
                if self.image_view_open && self.page == model::page::IMAGES {
                    self.image_view_open = false;
                    self.image_view_index = 0;
                    self.sync_panel_layout();
                } else {
                    self.hover_lock = true;
                }
            }
            if left || right {
                self.wheel_delta = 0.0;
                if self.image_view_open && self.page == model::page::IMAGES {
                    let count = self.clipboard.lock().images.len();
                    self.image_view_index = crate::pages::wrapped_image_index(
                        self.image_view_index,
                        count,
                        if right { 1 } else { -1 },
                    );
                } else {
                    self.navigate_page(right);
                }
            } else if scroll != 0.0 && !self.image_view_open {
                let cursor = env.cursor.unwrap() - self.animated_content_origin(win).to_vec2();
                let mail_body = self.page == model::page::MAIL
                    && self.mail.open_uid.is_some()
                    && Rect::from_min_max(Pos2::new(46.0, 104.0), Pos2::new(374.0, 194.0))
                        .contains(cursor);
                let mail_list = self.page == model::page::MAIL
                    && !self.mail.accounts.is_empty()
                    && self.mail.open_uid.is_none()
                    && self.mail_list_bounds().contains(cursor);
                if mail_body || mail_list {
                    self.wheel_delta = 0.0;
                } else {
                    if self.t - self.wheel_at > 0.18 || self.wheel_delta * scroll < 0.0 {
                        self.wheel_delta = 0.0;
                    }
                    self.wheel_at = self.t;
                    self.wheel_delta += scroll;
                    if self.wheel_delta.abs() >= 8.0 {
                        let next = self.wheel_delta < 0.0;
                        self.wheel_delta = 0.0;
                        let list = match self.page {
                            model::page::CLIPBOARD if clipboard_content_contains(cursor) => Some((
                                &mut self.clip_first,
                                self.clipboard
                                    .lock()
                                    .items
                                    .len()
                                    .saturating_sub(crate::pages::CLIP_ROWS),
                            )),
                            model::page::IMAGES if image_content_contains(cursor) => Some((
                                &mut self.image_first,
                                self.clipboard
                                    .lock()
                                    .images
                                    .len()
                                    .saturating_sub(crate::pages::IMAGE_COLS),
                            )),
                            model::page::SHELF
                                if Rect::from_min_max(
                                    Pos2::new(CONTENT_LEFT, 28.0),
                                    Pos2::new(CONTENT_RIGHT, 128.0),
                                )
                                .contains(cursor) =>
                            {
                                Some((
                                    &mut self.shelf_first,
                                    self.shelf.items.len().saturating_sub(5),
                                ))
                            }
                            _ => None,
                        };
                        let content_scrolls = list.as_ref().is_some_and(|(_, max)| *max > 0);
                        let scrolled = list.is_some_and(|(first, max)| {
                            let target = if next {
                                (*first + 1).min(max)
                            } else {
                                first.saturating_sub(1)
                            };
                            let changed = target != *first;
                            *first = target;
                            changed
                        });
                        if !scrolled && !content_scrolls && self.t - self.wheel_page_at >= 0.20 {
                            self.navigate_page(next);
                            self.wheel_page_at = self.t;
                        }
                        ctx.input_mut(|i| {
                            i.raw_scroll_delta = Vec2::ZERO;
                            i.smooth_scroll_delta = Vec2::ZERO;
                        });
                    }
                }
            }
        } else {
            self.wheel_delta = 0.0;
        }
        let clicked = ctx.input(|i| i.pointer.primary_clicked());
        let pointer_eligible =
            !self.hover_lock && (self.compact_notices() || self.notice.is_none() || self.open);
        let hover_elapsed = self.hover_since.map(|since| (env.t - since).max(0.0));
        let intent = if self.effective_minimal() {
            MINIMAL_HOVER_INTENT
        } else {
            HOVER_INTENT
        };
        let expand_action = pointer_expand_action(
            self.settings.expand_on_hover,
            self.open,
            inside,
            clicked && pointer_eligible,
            hover_elapsed,
            intent,
            self.drag_hot,
        );
        if !self.settings.expand_on_hover && expand_action == PointerExpandAction::Open {
            self.open = true;
            self.leave_since = None;
            self.w.tune(OPEN_W.0, OPEN_W.1);
            self.h.tune(OPEN_H.0, OPEN_H.1);
            self.w.target = self.panel_width();
            self.h.target = self.expanded_height();
        }
        let collapse_action = pointer_collapse_action(self.open, inside && !self.hover_lock);
        if collapse_action == PointerExpandAction::Close {
            let (bw, bh) = self.base_size();
            self.open = false;
            self.leave_since = None;
            self.hover_since = None;
            self.w.tune(CLOSE.0, CLOSE.1);
            self.h.tune(CLOSE.0, CLOSE.1);
            self.w.target = bw;
            self.h.target = bh;
            self.confirm_clear = None;
        }
        let want = (inside
            && pointer_eligible
            && (self.compact_notices() || self.notice.is_none() || self.open))
            || self.open && on_notice
            || (self.open && self.t < self.forced_open_until)
            || self.drag_hot;
        self.morph(want, inside, env.t, dt);
        self.update_pet(env, cx);

        let p = self.progress();
        let (w, h) = (self.w.pos, self.h.pos);
        let panel = self.island_rect(win, w, h);
        let messenger_rect = self.messenger_badge_rect(win, panel);
        let appear = self.messenger_appear();
        let outside_badge = messenger_rect.filter(|_| appear > 0.02);
        let pc = anim::clamp01(p);
        let painter = ui.painter_at(ui.max_rect().expand(40.0));
        let feather = 1.0 / ctx.pixels_per_point();
        let body = messenger_rect
            .filter(|_| outside_badge.is_none())
            .map_or(panel, |badge| panel.union(badge));
        let minimal_pill = self.effective_minimal()
            && !self.island_lyrics_active()
            && self.notice.is_none()
            && self.t >= self.notification_bell_until
            && pc < 0.02;
        let flare = if minimal_pill || pill {
            0.0
        } else {
            lerp(5.0, FLARE, pc)
        };
        let k = (w / PW).clamp(0.3, 1.1);
        let thickness = if side {
            crate::side_dock::SIDE_BAR_W
        } else {
            BAR_H
        };
        let radius = if minimal_pill {
            BAR_H_MIN / 2.0
        } else if pill {
            (w.min(h) * 0.5).min(lerp(thickness * 0.5, 26.0, pc))
        } else {
            lerp(12.5, RAD * k.min(1.0), pc)
        };
        let flare_h = lerp(8.0, FLARE_H, pc);
        let shadow_k = 1.0;
        let island_shadow = |r: Rect, flare: f32, flare_h: f32, radius: f32, strength: f32| {
            if pill {
                pill_shadow(&painter, r, radius, strength)
            } else if side {
                shape::shadow_docked(&painter, r, dock, flare, flare_h, radius, strength)
            } else {
                shape::shadow(&painter, r, flare, flare_h, radius, strength)
            }
        };
        island_shadow(
            body,
            flare,
            flare_h,
            radius,
            smoothstep(0.1, 0.9, pc) * shadow_k,
        );
        let notice_rect = self.notice_rect(win, top);
        if let Some(r) = notice_rect {
            pill_shadow(&painter, r, 12.5, 0.5 * shadow_k);
        }
        if let Some(r) = outside_badge {
            let appear = self.messenger_appear();
            painter.rect_filled(r, r.width() / 2.0, paint::alpha(Color32::BLACK, appear));
        }
        self.paint_body(&painter, body, flare, flare_h, radius, feather);
        if let Some(r) = notice_rect {
            painter.rect_filled(r, 12.5, Color32::BLACK);
        }
        let reveal = if self.open {
            smoothstep(0.50, 0.97, p)
        } else {
            smoothstep(0.55, 0.92, p)
        };
        self.reveal = reveal;
        let origin = self.animated_content_origin(win);
        let clip = panel.intersect(win);
        let base = Card::new(ui, origin, clip);
        self.cursor_local = env.cursor.map(|p| base.to_local(p));

        let banner = !self.compact_notices() && self.notice.is_some() && !self.open;
        let restoring_lyrics = !self.open
            && self.notice.is_none()
            && self.island_lyrics_active()
            && self
                .hud_notice
                .as_ref()
                .is_some_and(|a| a.source != crate::extras::AlertSource::Internal);
        let fading_hud = !self.compact_notices()
            && !self.open
            && self.hud_notice.as_ref().is_some_and(|a| a.is_hud())
            && (banner || self.hud_alpha > 0.003);
        if fading_hud {
            let from = self.resident.target.width;
            let to = HUD_W + self.pet_space();
            let width_progress = if (to - from).abs() < 1.0 {
                if banner {
                    1.0
                } else {
                    0.0
                }
            } else {
                ((self.w.pos - from) / (to - from)).clamp(0.0, 1.0)
            };
            let (opacity, header_opacity) = if banner {
                (
                    self.hud_alpha * smoothstep(0.45, 0.85, width_progress),
                    1.0 - smoothstep(0.05, 0.40, width_progress),
                )
            } else {
                (
                    self.hud_alpha * smoothstep(0.20, 0.60, width_progress),
                    1.0 - smoothstep(0.05, 0.25, width_progress),
                )
            };
            if !minimal_pill || pc > 0.0 {
                self.resident_header(&base.faded(header_opacity), panel, cx, pc, reveal, env);
            }
            if let Some(a) = self.hud_notice.clone() {
                let overlay = Card::new(
                    ui,
                    panel.min + Vec2::new(self.pet_space(), 0.0),
                    panel.intersect(ui.max_rect()),
                )
                .faded(opacity);
                let overlay = if opacity < 0.85 || !banner {
                    overlay.ghost()
                } else {
                    overlay
                };
                self.draw_notice(&overlay, panel, &a, &mut cmds);
            }
        } else if banner {
            if let Some(a) = self.notice.clone() {
                self.draw_notice(
                    &Card::new(
                        ui,
                        panel.min + Vec2::new(self.pet_space(), 0.0),
                        panel.intersect(ui.max_rect()),
                    ),
                    panel,
                    &a,
                    &mut cmds,
                );
            }
        } else if !minimal_pill || pc > 0.0 {
            self.resident_header(
                &base,
                panel,
                cx,
                if restoring_lyrics { 0.0 } else { pc },
                reveal,
                env,
            );
        }
        if let Some(rect) = notice_rect {
            let c = Card::new(ui, rect.min, rect.intersect(ui.max_rect()));
            if let Some(a) = self.hud_notice.clone() {
                let opacity = self.hud_alpha * smoothstep(0.45, 0.95, self.notice_width.pos / 44.0);
                let c = c.faded(opacity);
                let c = if self.notice.is_none() || opacity < 0.85 {
                    c.ghost()
                } else {
                    c
                };
                if a.source != crate::extras::AlertSource::Internal {
                    paint::bell(
                        &c.scaled(18.0, 15.0, 0.85),
                        18.0,
                        15.0,
                        theme::WHITE,
                        paint::bell_swing(self.t - a.at),
                    );
                    c.circle(22.0, 10.5, 2.0, Color32::from_rgb(255, 72, 72));
                } else {
                    crate::extras::notice_badge(&c, 18.0, 15.0, &a);
                }
                let button = c
                    .hit(0.0, 0.0, 36.0, BAR_H, "notice-badge")
                    .on_hover_text(format!("{}\n{}", a.title, a.body));
                if button.clicked() {
                    self.notice_details = !self.notice_details;
                }
            } else if self.t < self.notification_bell_until {
                paint::bell(
                    &c.scaled(20.0, 14.0, 1.2),
                    20.0,
                    14.0,
                    theme::WHITE,
                    paint::bell_swing(self.t - self.notification_bell_at),
                );
                c.circle(24.0, 9.5, 2.0, Color32::from_rgb(255, 72, 72));
                if c.hit(6.0, 0.0, 28.0, 30.0, "bell-mail").clicked() {
                    self.show_page(model::page::MAIL);
                }
            }
        }
        if let Some(rect) = messenger_rect {
            let c = Card::new(ui, rect.min, rect.intersect(ui.max_rect()));
            if outside_badge.is_some() {
                let c = c.scaled(0.0, 0.0, appear);
                self.draw_unread_capsule(&ctx, &c, rect, env.cursor, &mut cmds);
            } else {
                let horizontal = !(self.side() && self.notice.is_none());
                let scale = self.messenger_stack_appear();
                self.draw_unread_stack(
                    &c.scaled(0.0, 0.0, scale).faded(scale),
                    horizontal,
                    &mut cmds,
                );
            }
        }
        let notice_details = self.notice_details_rect(win, top);
        if let Some(rect) = notice_details {
            let radius = 10.0;
            shape::shadow(&painter, rect, 0.0, 0.0, radius, 0.5 * shadow_k);
            painter.rect_filled(rect, radius, Color32::BLACK);
            self.draw_notice(
                &Card::new(ui, rect.min, rect.intersect(ui.max_rect())),
                rect,
                &self.hud_notice.clone().unwrap(),
                &mut cmds,
            );
            if ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer.interact_pos().is_some_and(|p| {
                        !rect.contains(p) && !notice_rect.is_some_and(|r| r.contains(p))
                    })
            }) {
                self.notice_details = false;
            }
        }
        let pet_alpha = self.pet_a * (1.0 - smoothstep(0.0, 0.3, pc));
        if pet_alpha > 0.004 && !minimal_pill && self.resident_decoration_visible() {
            let y = (h.min(self.base_size().1) - BAR_H).max(0.0) / 2.0;
            let fit = if self.island_lyrics_active() || self.notice.is_some() {
                1.0
            } else {
                ((WIN_W - 32.0) / self.resident.target.width).min(1.0)
            };
            self.pet.draw(
                &base
                    .scaled(cx - origin.x, 15.0, fit)
                    .shifted(0.0, y)
                    .faded(pet_alpha),
                cx - origin.x + self.resident.position(bar::PET),
                self.t,
            );
        }
        if reveal > 0.004 && !banner && !restoring_lyrics {
            let c = base.shifted(0.0, (1.0 - reveal) * -5.0).faded(reveal);
            let c = if reveal < 0.6 { c.ghost() } else { c };
            self.panel_body(&c, env, &mut cmds);
        }
        let floating_toast = if self.toast.is_some() {
            let toast_top = top + self.h.pos.min(self.expanded_height()) + 8.0;
            self.toast_layer(
                &Card::new(ui, win.min, win.intersect(ui.max_rect())),
                win,
                toast_top,
            )
        } else {
            None
        };
        let rel = |r: Rect| r.translate(-win.min.to_vec2());
        let mut region = if side {
            let r = if p > 0.02 || self.open {
                let height = self.expanded_height().max(self.h.pos) * 1.08 + FLARE * 2.0 + 24.0;
                self.island_rect(win, self.panel_width() + 20.0, height)
                    .union(panel.expand(20.0))
            } else {
                let (bw, bh) = self.base_size();
                self.island_rect(win, bw * SWELL.1 + 14.0, bh * SWELL.0 + FLARE + 16.0)
                    .union(panel.expand(12.0))
            };
            if dock == model::Dock::Left {
                Rect::from_min_max(Pos2::new(win.left(), r.top()), r.max)
            } else {
                Rect::from_min_max(r.min, Pos2::new(win.right(), r.bottom()))
            }
        } else if p > 0.02 || self.open {
            let width = self.w.pos;
            let height = self.expanded_height().max(self.h.pos) * 1.08 + 16.0;
            Rect::from_center_size(
                Pos2::new(cx, top + height / 2.0),
                Vec2::new(width + FLARE * 2.0 + 56.0, height),
            )
        } else {
            let (bw, bh) = self.base_size();
            Rect::from_center_size(
                Pos2::new(cx, top + (bh * SWELL.1 + 12.0) / 2.0),
                Vec2::new(bw * SWELL.0 + 24.0, bh * SWELL.1 + 12.0),
            )
        };
        if let Some(r) = messenger_rect {
            region = region.union(r.expand(5.0));
        }
        if let Some(rect) = notice_rect {
            region = region.union(rect.expand2(Vec2::new(12.0, 8.0)));
        }
        if let Some(rect) = notice_details {
            region = region.union(rect.expand(12.0));
        }
        if pill && !side {
            region.min.y = region.min.y.min(win.top());
        }
        let mut tooltips = Vec::new();
        ctx.memory(|memory| {
            for layer in memory.areas().visible_layer_ids() {
                if layer.order == egui::Order::Tooltip {
                    if let Some(rect) = memory.area_rect(layer.id) {
                        region = region.union(rect.expand(12.0));
                        tooltips.push(rect);
                    }
                }
            }
        });
        let shape_region = env.opaque_surface.then(|| {
            let outline = |r: Rect, flare: f32, flare_h: f32, radius: f32| {
                if pill {
                    WindowShape::Round(rel(r), radius.min(r.height().min(r.width()) * 0.5))
                } else if side {
                    WindowShape::Docked {
                        rect: rel(r),
                        dock,
                        flare,
                        flare_h,
                        radius,
                    }
                } else {
                    WindowShape::Notch {
                        rect: rel(r),
                        flare,
                        flare_h,
                        radius,
                    }
                }
            };
            let mut shapes = vec![outline(body, flare, flare_h, radius)];
            if let Some(r) = outside_badge {
                shapes.push(WindowShape::Round(rel(r), r.width() / 2.0));
            }
            if let Some(r) = notice_rect {
                shapes.push(WindowShape::Round(rel(r), 12.5));
            }
            if let Some(r) = notice_details {
                let radius = 10.0;
                shapes.push(WindowShape::Round(rel(r), radius));
            }
            if let Some(r) = floating_toast {
                shapes.push(WindowShape::Round(rel(r), 11.0));
            }
            shapes.extend(tooltips.iter().map(|r| WindowShape::Round(rel(*r), 6.0)));
            shapes
        });
        let transitioning = !self.w.settled(self.panel_width())
            || !self.h.settled(self.expanded_height())
            || self.page_e() < 0.7
            || self.dots.iter().any(|d| !d.settled(16.0))
            || self.hover_since.is_some() && !self.open
            || self.leave_since.is_some()
            || (self.drag_a > 0.001 && self.drag_a < 0.999)
            || self.drag_hot
            || (self.pet_a > 0.001 && self.pet_a < 0.999)
            || !self.resident.settled()
            || self.open
                && self.page == model::page::SHELF
                && self.shelf_x.values().any(|s| !s.settled(68.0))
            || self.toast.is_some()
            || self.t < self.notification_bell_until && self.t - self.notification_bell_at < 0.95
            || self.notice.as_ref().is_some_and(|a| {
                a.source != crate::extras::AlertSource::Internal && self.t - a.at < 0.95
            })
            || self.hud_alpha > 0.003 && self.hud_alpha < 0.997
            || !self.notice_width.settled(1.0);
        let pet_visible =
            self.pet_a > 0.01 && !self.open && !minimal_pill && self.resident_decoration_visible();
        let ambient = self.media.playing
            && (self.open && self.page == model::page::MUSIC
                || !self.open && !minimal_pill && self.notice.is_none() && self.media.is_music())
            || self.video.playing
                && (self.open && self.page == model::page::VIDEO
                    || !self.open && !minimal_pill && self.notice.is_none());
        let animating = transitioning || ambient || pet_visible;
        let repaint_after = std::time::Duration::from_secs_f64(if transitioning || pet_visible {
            1.0 / 60.0
        } else if ambient {
            1.0 / 30.0
        } else {
            1.0
        });
        cmds.append(&mut self.pending);
        FrameOut {
            region: rel(floating_toast
                .into_iter()
                .chain(self.shelf_menu_rect.map(|r| {
                    r.union(r + ctx.style().visuals.popup_shadow.margin())
                        .expand(1.0 / ctx.pixels_per_point())
                }))
                .fold(region, Rect::union)
                .intersect(ui.max_rect())),
            window_shape: shape_region,
            hit: rel(if side {
                self.side_hit_rect(win, self.progress())
            } else {
                self.hit_rect(cx, top, self.progress())
            }),
            notice_hit: notice_rect
                .into_iter()
                .chain(notice_details)
                .chain(messenger_rect)
                .chain(self.shelf_menu_rect)
                .reduce(|a, b| a.union(b))
                .map(rel),
            file_drop_zone: self.settings.visible_pages[model::page::SHELF].then(|| {
                rel(if side {
                    self.island_rect(win, PW, self.expanded_height())
                } else {
                    Rect::from_min_size(Pos2::new(cx - PW / 2.0, top), Vec2::new(PW, 96.0))
                })
            }),
            expanded: self.open,
            editing: false,
            animating,
            repaint_after,
            cmds,
        }
    }

    fn hit_rect(&self, cx: f32, top: f32, p: f32) -> Rect {
        let r = self.hit_rect_base(cx, top, p);
        if self.settings.island_shape == model::IslandShape::Pill {
            Rect::from_min_max(Pos2::new(r.left(), r.top() - PILL_GAP), r.max)
        } else {
            r
        }
    }

    fn hit_rect_base(&self, cx: f32, top: f32, p: f32) -> Rect {
        if self.effective_minimal() && !self.open {
            let (width, height) = self.base_size();
            let width = self.w.pos.min(width);
            let height = self.h.pos.min(height);
            Rect::from_min_size(Pos2::new(cx - width / 2.0, top), Vec2::new(width, height))
        } else if self.notice.is_some() && !self.compact_notices() && !self.open {
            let (w, h) = self.base_size();
            Rect::from_min_size(
                Pos2::new(cx - (w + 8.0) / 2.0, top),
                Vec2::new(w + 8.0, h + 6.0),
            )
        } else if p > 0.04 || self.open {
            let width = self.w.pos;
            let height = self.h.pos;
            Rect::from_min_size(
                Pos2::new(cx - width / 2.0 - 4.0, top),
                Vec2::new(width + 8.0, height + 2.0),
            )
        } else {
            let (bw, bh) = self.base_size();
            let (hit_w, hit_h) = (bw + 12.0, bh + 4.0);
            Rect::from_min_size(Pos2::new(cx - hit_w / 2.0, top), Vec2::new(hit_w, hit_h))
        }
    }

    pub(crate) fn messenger_appear(&self) -> f32 {
        smoothstep(0.0, 1.0, anim::clamp01((self.progress() - 0.7) / 0.3))
    }
    fn messenger_stack_appear(&self) -> f32 {
        1.0 - smoothstep(0.04, 0.30, self.progress())
    }
    pub(crate) fn unread_tag(&self, i: usize) -> Option<usize> {
        let source = self.pending_messengers.get(i)?.source;
        let same = self
            .pending_messengers
            .iter()
            .filter(|u| u.source == source)
            .count();
        (same > 1).then(|| {
            self.pending_messengers[..=i]
                .iter()
                .filter(|u| u.source == source)
                .count()
        })
    }
    fn unread_id(&self, i: usize) -> String {
        let name = self.pending_messengers[i].source.name();
        match self.unread_tag(i) {
            Some(n) if n > 1 => format!("{name}{n}"),
            _ => name.to_string(),
        }
    }
    fn unread_capsule_height(&self, panel_h: f32) -> f32 {
        let n = self.pending_messengers.len();
        if n >= 3 {
            panel_h
        } else {
            4.0 + UNREAD_SLOT * n as f32
        }
    }
    pub(crate) fn unread_top_inset(&self) -> f32 {
        if self.dock() == model::Dock::Top {
            UNREAD_GAP
        } else {
            0.0
        }
    }
    fn unread_slot(&self, height: f32) -> f32 {
        if self.pending_messengers.len() >= 3 {
            (height - 4.0) / 3.0
        } else {
            UNREAD_SLOT
        }
    }
    fn unread_sources(&self) -> Vec<Messenger> {
        let mut sources = Vec::new();
        for u in &self.pending_messengers {
            if !sources.contains(&u.source) {
                sources.push(u.source);
            }
        }
        sources
    }
    fn unread_stack_len(&self, horizontal: bool) -> f32 {
        let icons = self.unread_sources().len().min(2) as f32;
        32.0 + unread_step(horizontal) * (icons - 1.0)
    }

    fn messenger_badge_rect(&self, win: Rect, panel: Rect) -> Option<Rect> {
        if self.pending_messengers.is_empty() || self.browser_minimal_active && !self.open {
            return None;
        }
        if !self.open
            && !self.compact_notices()
            && self.pending_messengers.len() == 1
            && self.notice.as_ref().and_then(|a| a.messenger())
                == self.pending_messengers.first().map(|u| u.source)
        {
            return None;
        }
        let bounds = self.screen.map_or(win, |screen| screen.intersect(win));
        let appear = self.messenger_appear();
        if appear > 0.02 {
            let full = self.island_rect(win, self.w.pos, self.h.pos);
            let inset = self.unread_top_inset();
            let height = self.unread_capsule_height(full.height() - inset);
            let left = self.dock() == model::Dock::Right
                || self.dock() == model::Dock::Top
                    && full.right() + UNREAD_GAP + UNREAD_W > bounds.right();
            let width = UNREAD_W * appear;
            let x = if left {
                panel.left() - UNREAD_GAP * appear - width
            } else {
                panel.right() + UNREAD_GAP * appear
            };
            return Some(Rect::from_min_size(
                Pos2::new(x, panel.top() + inset),
                Vec2::new(width, height * appear),
            ));
        }
        if self.effective_minimal() {
            return None;
        }
        let scale = self.messenger_stack_appear();
        if scale < 0.02 {
            return None;
        }
        let side_column = self.side() && self.notice.is_none();
        let length = self.unread_stack_len(!side_column);
        let (width, height) = if side_column {
            (32.0, length)
        } else {
            (length, BAR_H)
        };
        let (width, height) = (width * scale, height * scale);
        let (x, y) = if side_column {
            (panel.center().x - width / 2.0, panel.bottom())
        } else {
            let right = panel.right() + self.notice_width.pos;
            let y = if self.notice.is_some() && !self.compact_notices() {
                panel.center().y - BAR_H / 2.0
            } else {
                panel.top()
            };
            (
                if right + width <= bounds.right() {
                    right
                } else {
                    panel.left() - width
                },
                y,
            )
        };
        Some(Rect::from_min_size(
            Pos2::new(x, y),
            Vec2::new(width, height),
        ))
    }

    fn draw_unread_capsule(
        &mut self,
        ctx: &egui::Context,
        c: &Card,
        rect: Rect,
        cursor: Option<Pos2>,
        cmds: &mut Vec<Cmd>,
    ) {
        let n = self.pending_messengers.len();
        let height = rect.height() / c.s;
        let slot = self.unread_slot(height);
        let max_scroll = (n as f32 - 3.0).max(0.0) * slot;
        if max_scroll > 0.0 && cursor.is_some_and(|p| rect.contains(p)) {
            let delta = ctx.input(|i| i.raw_scroll_delta.y);
            self.messenger_scroll -= delta * 0.5;
        }
        self.messenger_scroll = self.messenger_scroll.clamp(0.0, max_scroll);
        let c = c.faded(self.messenger_appear());
        let inner = c.clipped(0.0, 2.0, UNREAD_W, height - 4.0);
        let x = UNREAD_W / 2.0;
        for i in 0..n {
            let u = self.pending_messengers[i];
            let cy = 2.0 + slot * (i as f32 + 0.5) - self.messenger_scroll;
            if cy < -slot || cy > height + slot {
                continue;
            }
            let scale = if u.source == Messenger::Wechat {
                1.0
            } else {
                1.12
            };
            crate::extras::messenger_icon(&inner.scaled(x, cy, scale), x, cy, u.source);
            inner.circle(x + 10.0, cy - 9.0, 2.6, Color32::from_rgb(255, 72, 72));
            if let Some(tag) = self.unread_tag(i) {
                inner.circle(x + 10.0, cy + 8.5, 6.0, Color32::from_gray(238));
                inner.centered_text(
                    x + 10.0,
                    cy + 8.5,
                    &tag.to_string(),
                    8.5,
                    W::Bold,
                    Color32::from_gray(28),
                    H::Center,
                );
            }
            let language = self.settings.language;
            let source_name = u.source.name_for(language);
            let name = match self.unread_tag(i) {
                Some(n) if language.is_en() => format!("{source_name} · Account {n}"),
                Some(n) => format!("{source_name} · 账号 {n}"),
                None => source_name.to_string(),
            };
            let button = inner
                .hit(0.0, cy - slot / 2.0, UNREAD_W, slot, self.unread_id(i))
                .on_hover_text(if language.is_en() {
                    format!("View {name}")
                } else {
                    format!("查看{name}")
                });
            if button.clicked() {
                cmds.push(Cmd::ViewMessenger(u));
            }
        }
        if max_scroll > 0.0 {
            let ink = paint::alpha(theme::WHITE, 0.35);
            if self.messenger_scroll > 0.5 {
                c.fill(x - 6.0, 4.0, 12.0, 2.0, 1.0, ink);
            }
            if self.messenger_scroll < max_scroll - 0.5 {
                c.fill(x - 6.0, height - 6.0, 12.0, 2.0, 1.0, ink);
            }
        }
    }

    fn draw_unread_stack(&mut self, c: &Card, horizontal: bool, cmds: &mut Vec<Cmd>) {
        let sources = self.unread_sources();
        let count = self.pending_messengers.len();
        let icons = sources.len().min(2);
        let step = unread_step(horizontal);
        let at = |k: usize| {
            if horizontal {
                (16.0 + step * k as f32, 14.0)
            } else {
                (16.0, 14.0 + step * k as f32)
            }
        };
        for (k, &source) in sources.iter().take(icons).enumerate() {
            let (x, y) = at(k);
            let scale = if source == Messenger::Wechat {
                0.85
            } else {
                0.92
            };
            crate::extras::messenger_icon(&c.scaled(x, y, scale), x, y, source);
        }
        for k in 0..icons {
            let (x, y) = at(k);
            c.circle(x + 8.0, y - 7.0, 2.6, Color32::from_rgb(255, 59, 48));
        }
        let (w, h) = if horizontal {
            (self.unread_stack_len(true), BAR_H)
        } else {
            (32.0, self.unread_stack_len(false))
        };
        if count == 1 {
            let language = self.settings.language;
            let u = self.pending_messengers[0];
            let name = u.source.name_for(language);
            let button = c
                .hit(0.0, 0.0, w, h, name)
                .on_hover_text(if language.is_en() {
                    format!("View {name}")
                } else {
                    format!("查看{name}")
                });
            if button.clicked() {
                cmds.push(Cmd::ViewMessenger(u));
            }
        } else {
            let language = self.settings.language;
            let button =
                c.hit(0.0, 0.0, w, h, "messenger-stack")
                    .on_hover_text(if language.is_en() {
                        format!("{count} unread · Expand to choose")
                    } else {
                        format!("{count} 条未读 · 展开挑选")
                    });
            if button.clicked() {
                self.reveal_unread();
            }
        }
    }
    pub(crate) fn reveal_unread(&mut self) {
        self.notice = None;
        self.open = true;
        self.forced_open_until = self.t + 4.0;
        self.w.tune(OPEN_W.0, OPEN_W.1);
        self.h.tune(OPEN_H.0, OPEN_H.1);
        self.w.target = self.panel_width();
        self.h.target = self.expanded_height();
    }
    fn notice_rect(&self, win: Rect, top: f32) -> Option<Rect> {
        if !self.compact_notices() {
            return None;
        }
        let width = self.notice_width.pos - 8.0;
        (width > 0.5).then(|| {
            TopCenterNoticeLayout::at(win, top + BAR_H + NOTICE_STACK_GAP, Vec2::new(width, BAR_H))
                .rect
        })
    }
    fn notice_details_rect(&self, win: Rect, top: f32) -> Option<Rect> {
        let a = self
            .hud_notice
            .as_ref()
            .filter(|_| self.notice_details && self.notice.is_some())?;
        let badge = self.notice_rect(win, top)?;
        let (width, height) = self.notice_size(a);
        Some(TopCenterNoticeLayout::below(win, badge, Vec2::new(width, height)).rect)
    }

    fn paint_body(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        flare: f32,
        flare_h: f32,
        radius: f32,
        feather: f32,
    ) {
        if self.settings.island_shape == model::IslandShape::Pill {
            let radius = radius.min(rect.height().min(rect.width()) * 0.5);
            painter.rect_filled(rect, radius, Color32::BLACK);
            return;
        }
        let dock = self.dock();
        if dock.is_side() {
            shape::fill_docked(
                painter,
                rect,
                dock,
                flare,
                flare_h,
                radius,
                Color32::BLACK,
                feather,
            );
            return;
        }
        shape::fill(
            painter,
            rect,
            flare,
            flare_h,
            radius,
            Color32::BLACK,
            feather,
        );
    }

    fn morph(&mut self, want: bool, inside: bool, t: f64, dt: f32) {
        let (bw, bh) = self.base_size();
        if want {
            self.leave_since = None;
            let since = *self.hover_since.get_or_insert(t);
            let intent = if self.effective_minimal() {
                MINIMAL_HOVER_INTENT
            } else {
                HOVER_INTENT
            };
            if !self.open
                && (self.drag_hot || (self.settings.expand_on_hover && t - since >= intent))
            {
                self.open = true;
                self.w.tune(OPEN_W.0, OPEN_W.1);
                self.h.tune(OPEN_H.0, OPEN_H.1);
                self.w.target = self.panel_width();
                self.h.target = self.expanded_height();
                if self.drag_hot {
                    self.w.vel = self.w.vel.max(600.0);
                    self.h.vel = self.h.vel.max(260.0);
                }
            } else if !self.open {
                self.w.tune(SWELL_K.0, SWELL_K.1);
                self.h.tune(SWELL_K.0, SWELL_K.1);
                let (sw, sh) = if self.side() {
                    (SWELL.1, SWELL.0)
                } else {
                    SWELL
                };
                self.w.target = bw * sw;
                self.h.target = bh * sh;
            }
        } else {
            self.hover_since = None;
            if self.open && self.settings.expand_on_hover {
                let since = *self.leave_since.get_or_insert(t);
                if t - since >= LEAVE_DELAY {
                    self.open = false;
                    self.leave_since = None;
                    self.w.tune(CLOSE.0, CLOSE.1);
                    self.h.tune(CLOSE.0, CLOSE.1);
                    self.w.target = bw;
                    self.h.target = bh;
                    self.confirm_clear = None;
                }
            } else if !self.open {
                self.w.tune(CLOSE.0, CLOSE.1);
                self.h.tune(CLOSE.0, CLOSE.1);
                self.w.target = bw;
                self.h.target = bh;
            }
        }
        let _ = inside;
        if self.open {
            self.h.target = self.expanded_height();
        }
        let (prev_w, prev_h) = (self.w.pos, self.h.pos);
        self.w.step(dt);
        self.h.step(dt);
        if !self.open {
            if prev_w >= bw && self.w.pos < bw {
                self.w.snap(bw);
            }
            if prev_h >= bh && self.h.pos < bh {
                self.h.snap(bh);
            }
        }
        for d in &mut self.dots {
            d.step(dt);
        }
    }

    fn shared_header(&mut self, base: &Card, cx: f32, p: f32, env: &Env) {
        let local_cx = cx - base.origin.x;
        let fit = ((WIN_W - 32.0) / self.resident.target.width).min(1.0);
        let fitted = base.scaled(local_cx, 15.0, lerp(fit, 1.0, p));
        let base = &fitted;
        let lyric = self.island_lyric().map(|(index, _)| index);
        let lyric_alpha = if self.island_lyrics_active() {
            1.0 - smoothstep(0.0, 0.35, p)
        } else {
            0.0
        };
        if let Some(galley) = self
            .island_lyric_layout
            .clone()
            .filter(|_| lyric_alpha > 0.003)
        {
            let c = base.faded(lyric_alpha);
            let content_width = self.lyric_bar_width();
            let width = self.w.pos.max(content_width);
            let left = local_cx - width / 2.0;
            let center_y = BAR_H / 2.0;
            if let Some((image, texture)) = &self.music_artwork {
                let [width, height] = image.size;
                let uv_size = if width > height {
                    egui::vec2(height as f32 / width as f32, 1.0)
                } else {
                    egui::vec2(1.0, width as f32 / height as f32)
                };
                let uv = Rect::from_center_size(Pos2::new(0.5, 0.5), uv_size);
                c.painter.add(
                    egui::epaint::RectShape::filled(
                        c.rect(left + 12.0, center_y - 10.0, 20.0, 20.0),
                        5.0 * c.s,
                        Color32::WHITE,
                    )
                    .with_texture(texture.id(), uv),
                );
            } else {
                c.fill(left + 12.0, center_y - 10.0, 20.0, 20.0, 5.0, theme::CARD);
                lucide::card(&c, Icon::Music, left + 22.0, center_y, 13.0, theme::WHITE);
            }
            let index = lyric.unwrap_or(0);
            let id = egui::Id::new("island-lyric-line");
            if !self.ly.island_animation_started {
                c.ui.ctx().animate_value_with_time(id, index as f32, 0.0);
                self.ly.island_animation_started = true;
            }
            let shown = c.ui.ctx().animate_value_with_time(id, index as f32, 0.18);
            let fade = 1.0 - (index as f32 - shown).abs().min(1.0);
            let position = c.pos(
                left + content_width - LYRIC_BAR_W / 2.0,
                center_y - galley.size().y / 2.0 + (1.0 - fade) * 3.0,
            );
            c.painter.galley(
                position,
                galley,
                if lyric.is_some() {
                    theme::WHITE
                } else {
                    theme::GRAY
                },
            );
            paint::playback_bars(
                &c,
                right_anchored_x(self.w.pos, content_width, content_width - 21.0),
                center_y,
                self.t,
                self.media.playing,
                theme::WHITE,
            );
        }
        if lyric_alpha >= 0.997 {
            return;
        }
        if !self.resident_decoration_visible() {
            return;
        }
        if p > 0.35 {
            return;
        }
        let header = base.faded(1.0 - lyric_alpha);
        let base = &header;
        let m = anim::ease_in_out(smoothstep(0.0, 0.85, p));
        let clock = self.settings.clock_text(&env.clock);
        let show_battery = self.settings.show_battery && env.battery.is_some();

        let clock_w = paint::clock_width(base, &clock, size::CLOCK);
        let battery_space = if show_battery { 24.0 } else { 0.0 };
        let battery_gap = if show_battery { 8.0 } else { 0.0 };
        let group = battery_space + battery_gap + clock_w;
        let gx = local_cx - group / 2.0 + if show_battery { 6.0 } else { 0.0 };
        let hb = (gx + 8.5, 18.6);
        let hc = gx + battery_space + battery_gap;
        let layout = self.resident.current();
        let pb = (local_cx + layout.x[bar::BATTERY].unwrap_or(0.0), 15.0);
        let pcx = local_cx + layout.x[bar::CLOCK].unwrap()
            - paint::clock_width(base, &clock, size::PILL_CLOCK) / 2.0;

        if self.effective_minimal() && self.t >= self.notification_bell_until && p < 0.02 {
            return;
        }
        let bk = lerp(1.0, 0.8, m);
        let bpos = (lerp(pb.0, hb.0, m), lerp(pb.1, hb.1, m));
        if show_battery {
            paint::battery(
                &base.faded(lerp(layout.clearance(bar::BATTERY), 1.0, m)),
                bpos.0,
                bpos.1,
                env.battery,
                env.charging,
                bk,
            );
        }

        let cpos = (lerp(pcx, hc, m), lerp(21.0, 23.0, m));
        let a_pill = (1.0 - smoothstep(0.25, 0.6, m)) * layout.clearance(bar::CLOCK);
        let a_head = smoothstep(0.35, 0.75, m);
        if a_pill > 0.0 {
            paint::clock(
                &base.faded(a_pill),
                cpos.0,
                cpos.1,
                &clock,
                size::PILL_CLOCK,
            );
        }
        if a_head > 0.0 {
            paint::clock(
                &base.faded(a_head),
                cpos.0,
                cpos.1 - 0.5,
                &clock,
                size::CLOCK,
            );
        }
        if self.video.is_video() {
            let x = right_anchored_x(
                self.w.pos,
                layout.width,
                layout.width / 2.0 + layout.x[bar::MUSIC].unwrap_or(0.0),
            );
            draw_video_session_badge(
                base,
                &self.video,
                self.video_artwork.as_ref(),
                x,
                15.0,
                17.0,
            );
        } else if resident_music_icon_visible(
            self.island_lyrics_active(),
            self.media.is_music(),
            p,
            layout.x[bar::MUSIC].is_some(),
        ) {
            let x = right_anchored_x(
                self.w.pos,
                layout.width,
                layout.width / 2.0 + layout.x[bar::MUSIC].unwrap_or(0.0),
            );
            paint::playback_bars(
                base,
                x,
                lerp(15.0, 18.0, m),
                self.t,
                self.media.playing,
                theme::WHITE,
            );
        }
        if self.t < self.notification_bell_until && !self.compact_notices() && p < 0.35 {
            if let Some(x) = layout.x[bar::BELL] {
                let x = right_anchored_x(self.w.pos, layout.width, layout.width / 2.0 + x);
                paint::bell(
                    &base.scaled(x, 14.0, 1.2),
                    x,
                    14.0,
                    theme::WHITE,
                    paint::bell_swing(self.t - self.notification_bell_at),
                );
                base.circle(x + 4.0, 9.5, 2.0, Color32::from_rgb(255, 72, 72));
                if base.hit(x - 14.0, 3.0, 28.0, 25.0, "bell-mail").clicked() {
                    self.show_page(model::page::MAIL);
                }
            }
        }
    }

    fn resident_header(
        &mut self,
        base: &Card,
        panel: Rect,
        cx: f32,
        p: f32,
        reveal: f32,
        env: &Env,
    ) {
        if !self.side() {
            self.shared_header(base, cx, p, env);
            return;
        }
        let bar_alpha = 1.0 - smoothstep(0.0, 0.3, p);
        let minimal = self.effective_minimal() && self.t >= self.notification_bell_until;
        if bar_alpha > 0.004 && !minimal {
            let (bar_w, bar_h) = self.base_size();
            let bar_rect = crate::side_dock::side_bar_rect(panel, bar_w, bar_h, self.dock());
            let c = base
                .shifted(
                    bar_rect.min.x - base.origin.x,
                    bar_rect.min.y - base.origin.y,
                )
                .faded(bar_alpha);
            let c = if bar_alpha < 0.8 { c.ghost() } else { c };
            self.side_bar(&c, bar_rect.width(), bar_rect.height(), env);
        }
        if reveal > 0.004 {
            self.shared_header(
                &base.faded(reveal),
                base.origin.x + self.panel_width() / 2.0,
                1.0,
                env,
            );
        }
    }

    fn update_pet(&mut self, env: &Env, cx: f32) {
        let cx = cx + self.resident.position(bar::PET);
        let allowed = self.pet_enabled();
        if !allowed {
            self.pet.reaction = None;
        }
        let available = allowed && !self.open && !self.drag_hot;
        let shown = available;
        self.pet_a = anim::approach(self.pet_a, if shown { 1.0 } else { 0.0 }, 9.0, self.dt);
        let visible = available && self.pet_a > 0.7 && self.progress() < 0.1;
        self.pet.advance(self.dt, visible);
        let state = self.pet_state();
        let gaze = if env.cursor.is_some() {
            crate::pet::gaze_for_cursor(env.cursor, cx)
        } else {
            [
                (self.t as f32 * 0.55).sin() * 0.85,
                (self.t as f32 * 0.31).cos() * 0.35,
            ]
        };
        self.pet.step(state, self.t, self.dt, gaze, visible);
    }

    fn panel_body(&mut self, c: &Card, env: &Env, cmds: &mut Vec<Cmd>) {
        self.expanded_header(c, cmds);
        let content_dy = match self.page {
            model::page::MUSIC => 24.0,
            model::page::VIDEO => 0.0,
            model::page::MONITOR => 0.0,
            model::page::IMAGES if self.image_view_open => 0.0,
            model::page::SHELF | model::page::IMAGES => -18.0,
            model::page::CLIPBOARD => 0.0,
            model::page::MAIL => 0.0,
            _ => 0.0,
        };
        let viewing = self.page == model::page::IMAGES && self.image_view_open;
        let page_width = if viewing {
            self.content_layout_width()
        } else {
            PW
        };
        let x_offset = ((self.content_layout_width() - page_width) / 2.0).max(0.0);
        let bounds = content_bounds(self.h.pos, page_width);
        let content = c
            .clipped(
                bounds.left() + x_offset,
                bounds.top(),
                bounds.width(),
                bounds.height(),
            )
            .shifted(x_offset, content_dy);
        let e = self.page_e();
        let in_tx = e < 0.55 && self.page_from != self.page;
        if in_tx {
            let old_a = 1.0 - smoothstep(0.0, 0.16, e);
            let new_a = smoothstep(0.03, 0.28, e);
            if old_a > 0.0 {
                let oc = content.faded(old_a).ghost();
                self.render_page(self.page_from, &oc, env, &mut Vec::new());
            }
            let nc = content.faded(new_a);
            let nc = if e < 0.12 { nc.ghost() } else { nc };
            self.render_page(self.page, &nc, env, cmds);
        } else {
            self.render_page(self.page, &content, env, cmds);
        }
    }

    fn expanded_header(&mut self, c: &Card, cmds: &mut Vec<Cmd>) {
        if self.page == model::page::MONITOR {
            self.monitor_metric_tabs(c);
            self.nav(c);
            self.header_actions(c, cmds);
            return;
        }
        let title = match self.page {
            model::page::MUSIC => self.settings.language.text("音乐", "Music"),
            model::page::VIDEO => self.settings.language.text("视频", "Video"),
            model::page::MONITOR => self.settings.language.text("监控", "Monitor"),
            model::page::SHELF => self.settings.language.text("文件口袋", "Pocket"),
            model::page::IMAGES => self.settings.language.text("图片", "Images"),
            model::page::CLIPBOARD => self.settings.language.text("剪贴板", "Clipboard"),
            model::page::MAIL => self.settings.language.text("邮箱", "Mail"),
            _ => self.settings.language.text("ISLE", "ISLE"),
        };
        c.text(
            HEADER_LEFT_X,
            25.0,
            title,
            12.0,
            W::Semibold,
            theme::WHITE,
            H::Left,
        );
        self.nav(c);
        self.header_actions(c, cmds);
    }

    fn header_actions(&mut self, c: &Card, cmds: &mut Vec<Cmd>) {
        if self.page == model::page::MONITOR {
            let label = if self.monitor_show_events {
                self.settings.language.text("总览", "Overview")
            } else {
                self.settings.language.text("事件", "Events")
            };
            if self.header_action(c, label, HEADER_RIGHT_X, "monitor-view") {
                self.monitor_show_events = !self.monitor_show_events;
            }
        } else if self.page == model::page::SHELF {
            let add_right = if self.shelf.items.is_empty() {
                HEADER_RIGHT_X
            } else {
                HEADER_RIGHT_X - 46.0
            };
            if self.header_action(
                c,
                self.settings.language.text("添加", "Add"),
                add_right,
                "header-add",
            ) {
                cmds.push(Cmd::AddFiles);
            }
            if !self.shelf.items.is_empty() {
                let confirming = self.confirm_clear.is_some();
                let label = if confirming {
                    self.settings.language.text("确认", "Confirm")
                } else {
                    self.settings.language.text("清空", "Clear")
                };
                if self.header_action(c, label, HEADER_RIGHT_X, "header-clear") {
                    if confirming {
                        self.shelf.clear();
                        self.icons.clear();
                        self.shelf_born.clear();
                        self.shelf_x.clear();
                        self.shelf_first = 0;
                        self.confirm_clear = None;
                        self.say(self.settings.language.text(
                            "文件口袋已清空（文件本身没动）",
                            "Pocket cleared. Source files were not changed.",
                        ));
                        self.pet.react(PetState::Play);
                    } else {
                        self.confirm_clear = Some(self.t);
                    }
                }
            }
        } else if self.page == model::page::IMAGES {
            let total = self.clipboard.lock().images.len();
            if self.image_view_open {
                let header_right = self.content_layout_width() - CONTENT_LEFT;
                let close_label = self.settings.language.text("关闭", "Close");
                if self.header_action(c, close_label, header_right, "header-image-view-close") {
                    self.image_view_open = false;
                    self.image_view_index = 0;
                    self.sync_panel_layout();
                }
            } else {
                let header_right = HEADER_RIGHT_X;
                let clear_label = self.settings.language.text("清空", "Clear");
                if total > 0 {
                    let view_label = self.settings.language.text("查看", "View");
                    let view_right = header_right - c.text_w(clear_label, 10.5, W::Semibold) - 18.0;
                    if self.header_action(c, view_label, view_right, "header-image-view") {
                        self.image_view_open = true;
                        self.image_view_index = 0;
                        self.sync_panel_layout();
                    }
                    if self.header_action(c, clear_label, header_right, "header-image-clear") {
                        self.clipboard.lock().clear_images();
                        self.image_first = 0;
                        self.say(
                            self.settings
                                .language
                                .text("临时图片已全部清空", "All temporary images were cleared."),
                        );
                    }
                }
            }
        } else if self.page == model::page::CLIPBOARD {
            let recent = self
                .clipboard
                .lock()
                .items
                .iter()
                .filter(|clip| !clip.pinned)
                .count();
            if recent > 0 {
                let confirming = self.clip_confirm_clear.is_some_and(|at| self.t - at < 3.0);
                if !confirming {
                    self.clip_confirm_clear = None;
                }
                let label = if confirming {
                    self.settings.language.text("确认", "Confirm")
                } else {
                    self.settings.language.text("清空", "Clear")
                };
                if self.header_action(c, label, HEADER_RIGHT_X, "header-clip-clear") {
                    if confirming {
                        self.clipboard.lock().clear_recent();
                        self.clip_first = 0;
                        self.clip_confirm_clear = None;
                        self.say(self.settings.language.text(
                            "剪贴板记录已清空（固定的保留）",
                            "Clipboard history cleared. Pinned entries were kept.",
                        ));
                        self.pet.react(PetState::Play);
                    } else {
                        self.clip_confirm_clear = Some(self.t);
                    }
                }
            }
        }
    }

    fn header_action(&self, c: &Card, label: &str, right: f32, id: &str) -> bool {
        let width = c.text_w(label, 10.5, W::Semibold);
        let x = right - width;
        let response = c.hit(x - 5.0, 7.0, width + 10.0, 24.0, id);
        c.text(
            x,
            25.0,
            label,
            10.5,
            W::Semibold,
            if response.hovered() {
                theme::WHITE
            } else {
                theme::GRAY
            },
            H::Left,
        );
        response.clicked()
    }

    fn nav(&mut self, c: &Card) {
        let pages: Vec<_> = self
            .settings
            .page_order
            .iter()
            .copied()
            .filter(|page| self.page_available(*page))
            .collect();
        if pages.len() < 2 {
            return;
        }
        if self.image_view_open {
            return;
        }
        let dot_y = (self.h.pos - 9.0).max(35.0);
        let nav_y = if self.side() {
            side_nav_center_y(self.h.pos)
        } else {
            (self.expanded_height() - NAV_ZONE) / 2.0
        };
        let navigation = page_nav_layout(pages.len(), self.w.pos / 2.0);
        if !self.image_view_open {
            let l = c.hit(8.0, nav_y - 13.0, 26.0, 26.0, "nav-l");
            let r = c.hit(
                HEADER_ARROW_RIGHT_X - 13.0,
                nav_y - 13.0,
                26.0,
                26.0,
                "nav-r",
            );
            let (lh, lp) = c.hover_t(&l);
            let (rh, rp) = c.hover_t(&r);
            lucide::card(
                &c.scaled(HEADER_ARROW_LEFT_X, nav_y, 1.0 - 0.08 * lp),
                Icon::ChevronLeft,
                HEADER_ARROW_LEFT_X,
                nav_y,
                13.0,
                paint::mix(theme::GRAY, theme::WHITE, lh),
            );
            lucide::card(
                &c.scaled(HEADER_ARROW_RIGHT_X, nav_y, 1.0 - 0.08 * rp),
                Icon::ChevronRight,
                HEADER_ARROW_RIGHT_X,
                nav_y,
                13.0,
                paint::mix(theme::GRAY, theme::WHITE, rh),
            );
            if l.clicked() {
                self.navigate_page(false);
            }
            if r.clicked() {
                self.navigate_page(true);
            }
        }
        const DOT_SLOT: f32 = 16.0;
        const DOT_GAP: f32 = 4.0;
        let mut x = navigation.dots_left;
        let mut clicked = None;
        for i in pages {
            let w = self.dots[i].pos.max(3.0);
            let dot_x = x + (DOT_SLOT - w) / 2.0;
            let on = anim::remap(w, 4.0, 16.0);
            let col = paint::mix(Color32::from_rgb(92, 92, 96), theme::WHITE, on);
            c.fill(dot_x, dot_y - 2.0, w, 4.0, 2.0, col);
            let hr = c.hit(x - 2.0, dot_y - 6.0, DOT_SLOT + 4.0, 12.0, ("dot", i));
            hr.clone()
                .on_hover_text(model::page::retained_name_for(i, self.settings.language));
            if hr.hovered() && i != self.page {
                c.fill(
                    dot_x - 1.0,
                    dot_y - 3.0,
                    w + 2.0,
                    6.0,
                    3.0,
                    Color32::from_white_alpha(28),
                );
            }
            if hr.clicked() {
                clicked = Some(i);
            }
            x += DOT_SLOT + DOT_GAP;
        }
        if let Some(i) = clicked {
            let position = |page| {
                self.settings
                    .page_order
                    .iter()
                    .position(|p| *p == page)
                    .unwrap()
            };
            let dir = if position(i) > position(self.page) {
                1.0
            } else {
                -1.0
            };
            self.go(i, dir);
        }
    }

    fn render_page(&mut self, page: usize, c: &Card, env: &Env, cmds: &mut Vec<Cmd>) {
        match page {
            model::page::MUSIC => self.page_music(c, env, cmds),
            model::page::VIDEO => self.page_video(c, env, cmds),
            model::page::MONITOR => self.page_monitor(c, env, cmds),
            model::page::SHELF => self.page_shelf(c, env, cmds),
            model::page::IMAGES if self.image_view_open => {
                self.page_image_viewer(c, self.content_layout_width(), cmds)
            }
            model::page::IMAGES => self.page_images(c, env, cmds),
            model::page::CLIPBOARD => self.page_clipboard(c, env, cmds),
            model::page::MAIL => self.page_mail(c, env, cmds),
            _ => {}
        }
    }

    fn toast_layer(&mut self, c: &Card, area: Rect, anchor_y: f32) -> Option<Rect> {
        let (msg, at) = self.toast.clone()?;
        let e = (self.t - at) as f32;
        if e > 2.4 {
            self.toast = None;
            return None;
        }
        let a = smoothstep(0.0, 0.16, e) * (1.0 - smoothstep(2.1, 2.4, e));
        let drop = (1.0 - anim::ease_out_cubic(e / 0.3)) * -6.0;
        let msg = c.fit(&msg, size::BODY, W::Regular, 310.0);
        let w = c.text_w(&msg, size::BODY, W::Regular) + 26.0;
        let tc = c.faded(a);
        let rect = TopCenterNoticeLayout::at(area, anchor_y + drop, Vec2::new(w, 22.0)).rect;
        let local = rect.min - c.origin.to_vec2();
        tc.fill(
            local.x,
            local.y,
            rect.width(),
            rect.height(),
            11.0,
            Color32::from_rgb(44, 44, 48),
        );
        tc.text(
            local.x + rect.width() / 2.0,
            local.y + 15.0,
            &msg,
            size::BODY,
            W::Regular,
            theme::WHITE,
            H::Center,
        );
        Some(rect)
    }

    fn tick(&mut self, dt: f32, _elapsed: f32) {
        if self.toast.as_ref().is_some_and(|(_, at)| self.t - at > 2.4) {
            self.toast = None;
        }
        if self.clipboard.is_enabled() != self.settings.clipboard_history {
            self.clipboard.set_enabled(self.settings.clipboard_history);
        }
        if self.clipboard.images_enabled() != self.settings.clipboard_images {
            self.clipboard
                .set_images_enabled(self.settings.clipboard_images);
        }
        let (clip_version, clip_count, clip_head) = {
            let history = self.clipboard.lock();
            (
                history.version,
                history.items.len(),
                history.items.first().map(|item| item.text.clone()),
            )
        };
        if clip_version > self.last_clipboard_version
            && clip_count >= self.last_clipboard_count
            && clip_head != self.last_clipboard_head
        {
            self.pet.react(PetState::Idle);
        }
        self.last_clipboard_version = clip_version;
        self.last_clipboard_count = clip_count;
        self.last_clipboard_head = clip_head;

        let missing = self
            .shelf
            .items
            .iter()
            .filter(|path| !path.exists())
            .count();
        if missing > self.last_missing_shelf_files {
            self.pet.react(PetState::Idle);
        }
        self.last_missing_shelf_files = missing;

        self.shelf_scroll.target = self.shelf_first as f32;
        self.shelf_scroll.step(dt);
        if let Some(at) = self.confirm_clear {
            if self.t - at > 3.0 {
                self.confirm_clear = None;
            }
        }
    }

    fn update_media(&mut self, env: &Env, ctx: &egui::Context) {
        let m = &env.media;
        if m.is_video() {
            self.media = NowPlaying::default();
            self.media_anchor = (String::new(), 0.0, 0.0, false);
            self.music_artwork = None;
            self.ly = LyricsState::default();
            let mut video = m.clone();
            if video.playing {
                if let Some(sampled) = video.sampled_at {
                    video.position += sampled.elapsed().as_secs_f64();
                }
                if video.duration > 0.0 {
                    video.position = video.position.min(video.duration);
                }
            }
            self.video = video;
            if let Some(image) = &self.video.artwork {
                if !self
                    .video_artwork
                    .as_ref()
                    .is_some_and(|(old, _)| std::sync::Arc::ptr_eq(old, image))
                {
                    self.video_artwork = Some((
                        image.clone(),
                        ctx.load_texture(
                            "video-artwork",
                            image.clone(),
                            egui::TextureOptions::LINEAR,
                        ),
                    ));
                }
            } else {
                self.video_artwork = None;
            }
            return;
        }
        if !m.is_music() {
            self.media = NowPlaying::default();
            self.media_anchor = (String::new(), 0.0, 0.0, false);
            self.music_artwork = None;
            self.video = NowPlaying::default();
            self.video_artwork = None;
            self.ly = LyricsState::default();
            return;
        }
        self.video = NowPlaying::default();
        self.video_artwork = None;
        let a = &self.media_anchor;
        if a.0 != m.title || (a.1 - m.position).abs() > 0.01 || a.3 != m.playing {
            self.media_anchor = (m.title.clone(), m.position, env.t, m.playing);
        }
        let mut np = m.clone();
        if m.playing {
            let (_, pos, _, _) = &self.media_anchor;
            np.position = if let Some(sampled) = m.sampled_at {
                m.position + sampled.elapsed().as_secs_f64()
            } else {
                *pos
            };
            if m.duration > 0.0 {
                np.position = np.position.min(m.duration);
            }
        }
        let other_song =
            matches!((self.media.song_id,np.song_id),(Some(old),Some(new)) if old!=new);
        if self.media.source != np.source || other_song {
            self.ly = LyricsState::default();
        }
        self.media = np;
        if let Some(image) = &self.media.artwork {
            if !self
                .music_artwork
                .as_ref()
                .is_some_and(|(old, _)| std::sync::Arc::ptr_eq(old, image))
            {
                self.music_artwork = Some((
                    image.clone(),
                    ctx.load_texture("music-artwork", image.clone(), egui::TextureOptions::LINEAR),
                ));
            }
        } else {
            self.music_artwork = None;
        }
    }

    fn poll_lyrics(&mut self) {
        if !self.media.is_music() {
            self.ly = LyricsState::default();
            return;
        }
        if !self.show_lyrics && !self.settings.island_lyrics {
            self.ly = LyricsState::default();
            return;
        }
        let np = &self.media;
        if np.title.is_empty() {
            self.ly = LyricsState::default();
            return;
        }
        let key = format!("{}|{}|{:.0}", np.title, np.artist, np.duration);
        let id_arrived = key == self.ly.key
            && !self.ly.loading
            && self.ly.song_id.is_none()
            && np.song_id.is_some()
            && !self.ly.lyrics.as_ref().is_some_and(|lyrics| lyrics.synced);
        if key != self.ly.key || id_arrived || self.ly.retry_at.is_some_and(|at| self.t >= at) {
            if self
                .ly_worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished())
            {
                return;
            }
            let Some(lease) = crate::resources::LYRICS.acquire() else {
                return;
            };
            let native_only = key == self.ly.key
                && np.source.to_ascii_lowercase().contains("kugou")
                && self.ly.error == Some(lyrics::FetchError::NotFound);
            let cached = if key == self.ly.key && np.source.to_ascii_lowercase().contains("kugou") {
                self.ly.lyrics.take()
            } else {
                None
            };
            let attempts = if key == self.ly.key {
                self.ly.attempts.saturating_add(1)
            } else {
                0
            };
            self.ly = LyricsState {
                key,
                loading: cached.is_none() && !native_only,
                lyrics: cached.clone(),
                error: native_only.then_some(lyrics::FetchError::NotFound),
                song_id: np.song_id,
                attempts,
                ..Default::default()
            };
            let (tx, rx) = std::sync::mpsc::channel();
            self.ly.rx = Some(rx);
            let (source, title, artist, dur) = (
                np.source.clone(),
                np.title.clone(),
                np.artist.clone(),
                np.duration,
            );
            let song_id = np.song_id;
            if source.to_ascii_lowercase().contains("kugou") || lyrics::is_soda(&source) {
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                self.ly.native_rx = Some(rx);
                let (source, title, artist) = (source.clone(), title.clone(), artist.clone());
                crate::resources::NATIVE_LYRICS.spawn(move || {
                    let started = std::time::Instant::now();
                    let mut wait = std::time::Duration::from_millis(500);
                    let mut last: Option<((std::time::SystemTime, u64), std::time::Instant)> = None;
                    loop {
                        let print = lyrics::native_fingerprint(&source);
                        let unchanged = print.is_some_and(|print| {
                            last.is_some_and(|(seen, at)| {
                                seen == print && at.elapsed() < std::time::Duration::from_secs(15)
                            })
                        });
                        let found = if unchanged {
                            None
                        } else {
                            if let Some(print) = print {
                                last = Some((print, std::time::Instant::now()));
                            }
                            lyrics::read_native(&source, &title, &artist, dur)
                        };
                        let done = found.is_some();
                        match tx.try_send(found) {
                            Ok(()) => {
                                if done {
                                    break;
                                }
                            }
                            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => break,
                            Err(std::sync::mpsc::TrySendError::Full(_)) => {}
                        }
                        if started.elapsed() >= std::time::Duration::from_secs(90) {
                            break;
                        }
                        std::thread::sleep(wait);
                        wait = (wait * 2).min(std::time::Duration::from_secs(5));
                    }
                });
            }
            self.ly_worker = lease.spawn(move || {
                let result = if native_only {
                    lyrics::kugou_cache::read(&title, &artist, dur)
                        .map(|lyrics| (lyrics, false))
                        .ok_or(lyrics::FetchError::NotFound)
                } else {
                    lyrics::fetch_for_display(&source, &title, &artist, dur, song_id, cached)
                };
                let _ = tx.send(result);
            });
            if self.ly_worker.is_none() {
                self.ly.loading = false;
                self.ly.key.clear();
            }
        }
        if let Some(rx) = &self.ly.native_rx {
            let mut found = None;
            while let Ok(result) = rx.try_recv() {
                if result.is_some() {
                    found = result;
                }
            }
            if let Some(lyrics) = found {
                self.ly.lyrics = Some(lyrics);
                self.ly.loading = false;
                self.ly.error = None;
                self.ly.retry_at = None;
                self.ly.rx = None;
                self.ly.native_rx = None;
            }
        }
        if let Some(rx) = &self.ly.rx {
            if let Ok(r) = rx.try_recv() {
                self.ly.loading = false;
                match r {
                    Ok((l, waiting_native)) => {
                        self.ly.error = None;
                        self.ly.retry_at = None;
                        if (waiting_native || lyrics::is_soda(&self.media.source) && !l.synced)
                            && self.ly.attempts < NATIVE_RETRIES
                        {
                            self.ly.retry_at = Some(self.t + 5.0);
                        }
                        self.ly.lyrics = Some(l);
                    }
                    Err(error) => {
                        let native = lyrics::is_soda(&self.media.source)
                            || self.media.source.to_ascii_lowercase().contains("kugou")
                                && error == lyrics::FetchError::NotFound;
                        if native {
                            if self.ly.attempts < NATIVE_RETRIES {
                                self.ly.retry_at = Some(self.t + 5.0);
                            }
                        } else if matches!(
                            error,
                            lyrics::FetchError::Unavailable(_)
                                | lyrics::FetchError::InvalidResponse(_)
                        ) && self.ly.attempts < NETWORK_RETRIES
                        {
                            self.ly.retry_at = Some(self.t + 30.0);
                        }
                        self.ly.error = Some(error);
                    }
                }
                self.ly.rx = None;
            }
        }
    }

    pub fn set_lyrics(&mut self, title: &str, artist: &str, duration: f64, l: lyrics::Lyrics) {
        self.ly = LyricsState {
            key: format!("{title}|{artist}|{duration:.0}"),
            lyrics: Some(l),
            ..Default::default()
        };
    }

    pub fn on_drop(&mut self, files: Vec<PathBuf>) {
        if !self.settings.visible_pages[model::page::SHELF] || files.is_empty() {
            return;
        }
        self.drag_hot = false;
        self.drag_saved = None;
        if self.page != model::page::SHELF {
            self.go(model::page::SHELF, 1.0);
        }
        let (mut added, mut duplicate, mut failed) = (0, 0, 0);
        for p in files {
            if !p.exists() {
                failed += 1;
                continue;
            }
            if self.shelf.items.contains(&p) {
                duplicate += 1;
                continue;
            }
            if self.shelf.add(&p).is_ok() {
                self.shelf_born.insert(p.clone(), self.t);
                added += 1;
            } else {
                failed += 1;
            }
        }
        let language = self.settings.language;
        let message = if added == 0 && failed == 0 {
            if duplicate == 1 {
                language
                    .text("该文件已在文件口袋中", "This file is already in Pocket")
                    .into()
            } else {
                language
                    .text(
                        "这些文件已在文件口袋中",
                        "These files are already in Pocket",
                    )
                    .into()
            }
        } else if added == 0 && duplicate == 0 {
            language
                .text(
                    "文件不存在或无法访问",
                    "The file does not exist or cannot be accessed",
                )
                .into()
        } else {
            let mut parts = Vec::new();
            if added > 0 {
                parts.push(if language.is_en() {
                    format!(
                        "Added {added} {}",
                        if added == 1 { "file" } else { "files" }
                    )
                } else {
                    format!("已添加 {added} 个文件")
                });
            }
            if duplicate > 0 {
                parts.push(if language.is_en() {
                    format!("{duplicate} already added")
                } else {
                    format!("{duplicate} 个已存在")
                });
            }
            if failed > 0 {
                parts.push(if language.is_en() {
                    format!("{failed} could not be added")
                } else {
                    format!("{failed} 个无法添加")
                });
            }
            parts.join(language.text("，", ", "))
        };
        self.say(message);
        if added > 0 {
            self.pet.react(PetState::Naming);
            self.shelf_first = self.shelf.items.len().saturating_sub(5);
        }
    }
}
