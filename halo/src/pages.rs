//! Four retained pages: music, Pocket, clipboard, and mail.
//! 坐标全部是面板局部逻辑点（419×140），文字 y 是基线。

use crate::anim::{lerp, smoothstep, Spring};
use crate::app::{App, Cmd, Env, CONTENT_LEFT, CONTENT_RIGHT, CONTENT_WIDTH};
use crate::compact_list as compact;
use crate::lucide::{self, Icon};
use crate::mail;
use crate::paint::{self, capsule, Card, H};
use crate::smtc::Transport;
use crate::theme::{self, size, W};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};

/// 剪贴板分栏一屏显示的条数。
pub(crate) const CLIP_ROWS: usize = 5;
pub(crate) const IMAGE_COLS: usize = 5;
pub(crate) const MAIL_ROWS: usize = 5;
const IMAGE_PREVIEW_MAX: Vec2 = Vec2::new(330.0, 128.0);

fn mmss(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn draw_texture_fit(c: &Card, texture: &egui::TextureHandle, rect: Rect, radius: f32) {
    let source = texture.size_vec2();
    if source.x <= 0.0 || source.y <= 0.0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let source_ratio = source.x / source.y;
    let target_ratio = rect.width() / rect.height();
    let uv = if source_ratio > target_ratio {
        Rect::from_center_size(
            Pos2::new(0.5, 0.5),
            egui::vec2(target_ratio / source_ratio, 1.0),
        )
    } else {
        Rect::from_center_size(
            Pos2::new(0.5, 0.5),
            egui::vec2(1.0, source_ratio / target_ratio),
        )
    };
    c.painter.add(
        egui::epaint::RectShape::filled(rect, radius * c.s, Color32::WHITE)
            .with_texture(texture.id(), uv),
    );
}

fn image_preview_size(width: u32, height: u32) -> Vec2 {
    if width == 0 || height == 0 {
        return Vec2::ZERO;
    }
    let source = Vec2::new(width as f32, height as f32);
    let scale = (IMAGE_PREVIEW_MAX.x / source.x)
        .min(IMAGE_PREVIEW_MAX.y / source.y)
        .min(1.0);
    source * scale
}

fn draw_image_preview(
    c: &Card,
    texture: &egui::TextureHandle,
    item: &crate::clipboard::ImageClip,
    anchor_x: f32,
    viewport_width: f32,
) -> Option<Rect> {
    let size = image_preview_size(item.width, item.height);
    if size.x <= 0.0 || size.y <= 0.0 {
        return None;
    }
    let center_x = anchor_x.clamp(size.x / 2.0 + 12.0, viewport_width - size.x / 2.0 - 12.0);
    let top = 146.0;
    let screen_rect = c.rect(center_x - size.x / 2.0, top, size.x, size.y);
    let layer = egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new(("clipboard-image-preview", item.id.as_str())),
    );
    let clip = screen_rect.expand(2.0).intersect(c.ui.ctx().screen_rect());
    let painter = c.ui.ctx().layer_painter(layer).with_clip_rect(clip);
    let preview = Card::on_painter(c.ui, painter, c.origin, clip);
    draw_texture_fit(&preview, texture, screen_rect, 0.0);
    Some(screen_rect.expand(2.0))
}

#[test]
fn image_preview_keeps_the_source_aspect_ratio_within_screen_bounds() {
    let landscape = image_preview_size(692, 292);
    assert!((landscape.x / landscape.y - 692.0 / 292.0).abs() < 0.001);
    assert!(landscape.x <= IMAGE_PREVIEW_MAX.x && landscape.y <= IMAGE_PREVIEW_MAX.y);

    let portrait = image_preview_size(600, 1200);
    assert!((portrait.x / portrait.y - 0.5).abs() < 0.001);
    assert!(portrait.x <= IMAGE_PREVIEW_MAX.x && portrait.y <= IMAGE_PREVIEW_MAX.y);

    assert_eq!(image_preview_size(120, 80), Vec2::new(120.0, 80.0));
    assert_eq!(image_preview_size(0, 80), Vec2::ZERO);
}

/// 由标题生成稳定的封面渐变色。
fn cover_colors(title: &str) -> (Color32, Color32) {
    let mut h: u64 = 1469598103934665603;
    for b in title.bytes() {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    let palette = [
        (
            Color32::from_rgb(255, 94, 98),
            Color32::from_rgb(255, 153, 102),
        ),
        (
            Color32::from_rgb(86, 204, 242),
            Color32::from_rgb(47, 128, 237),
        ),
        (
            Color32::from_rgb(168, 85, 247),
            Color32::from_rgb(236, 72, 153),
        ),
        (
            Color32::from_rgb(52, 211, 153),
            Color32::from_rgb(16, 120, 200),
        ),
        (
            Color32::from_rgb(251, 191, 36),
            Color32::from_rgb(239, 68, 68),
        ),
    ];
    palette[(h % palette.len() as u64) as usize]
}

impl App {
    // ================= 视频 =================
    pub(crate) fn page_video(&mut self, c: &Card, _env: &Env, cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        let np = self.video.clone();
        let has = np.is_video();
        if !has {
            c.text(
                209.5,
                74.0,
                language.text("浏览器播放视频时会自动出现", "Browser video appears here"),
                12.0,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            return;
        }
        let art = c.rect(44.0, 44.0, 126.0, 76.0);
        if let Some((image, texture)) = &self.video_artwork {
            let [w, h] = image.size;
            let (uw, vh) = if w > h {
                (h as f32 / w as f32, 1.0)
            } else {
                (1.0, w as f32 / h as f32)
            };
            let uv = Rect::from_center_size(Pos2::new(0.5, 0.5), egui::vec2(uw, vh));
            c.painter.add(
                egui::epaint::RectShape::filled(art, 11.0, Color32::WHITE)
                    .with_texture(texture.id(), uv),
            );
        } else {
            let (a, b) = cover_colors(&np.title);
            let cc = c.clipped(44.0, 44.0, 126.0, 76.0);
            cc.vgrad(44.0, 44.0, 126.0, 76.0, 11.0, a, b);
            lucide::card(&cc, Icon::PlayCircle, 107.0, 82.0, 32.0, theme::WHITE);
        }
        let shade = c.clipped(44.0, 98.0, 126.0, 22.0);
        shade.fill(
            44.0,
            98.0,
            126.0,
            22.0,
            10.0,
            Color32::from_black_alpha(130),
        );

        c.text(
            180.0,
            61.0,
            &c.fit(&np.title, 13.0, W::Semibold, 205.0),
            13.0,
            W::Semibold,
            theme::WHITE,
            H::Left,
        );
        let source = if np.source.to_ascii_lowercase().contains("edge") {
            "Microsoft Edge".to_string()
        } else if np.source.to_ascii_lowercase().contains("chrome") {
            "Google Chrome".to_string()
        } else if np.source.to_ascii_lowercase().contains("firefox") {
            "Firefox".to_string()
        } else {
            np.source.clone()
        };
        c.text(
            180.0,
            78.0,
            &c.fit(&source, 10.5, W::Regular, 205.0),
            10.5,
            W::Regular,
            theme::GRAY,
            H::Left,
        );
        let frac = if np.duration > 0.0 {
            (np.position / np.duration).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        c.fill(180.0, 91.0, 205.0, 3.5, 1.75, Color32::from_gray(55));
        if frac > 0.0 {
            c.fill(180.0, 91.0, 205.0 * frac, 3.5, 1.75, theme::WHITE);
        }
        let return_rect = c.hit(180.0, 105.0, 195.0, 30.0, "video-return");
        let hover = c.hover_t(&return_rect).0;
        c.fill(
            180.0,
            105.0,
            195.0,
            30.0,
            9.0,
            paint::mix(Color32::from_gray(35), Color32::from_gray(48), hover),
        );
        c.text(
            277.5,
            120.0,
            language.text("回到浏览器继续播放", "Return to browser"),
            10.5,
            W::Semibold,
            theme::WHITE,
            H::Center,
        );
        if c.hit(44.0, 44.0, 331.0, 98.0, "video-card").clicked() || return_rect.clicked() {
            cmds.push(Cmd::FocusMedia(np.source.clone()));
        }
    }

    // ================= 音乐 =================
    pub(crate) fn page_music(&mut self, c: &Card, _env: &Env, cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        let np = self.media.clone();
        let has = !np.title.is_empty();
        let with_lyrics = has && self.show_lyrics;
        let ctx = c.ui.ctx().clone();
        let playing_t = ctx.animate_bool_with_time(egui::Id::new("np-playing"), np.playing, 0.22);
        let art_size = if with_lyrics { 44.0 } else { 62.0 };
        let art_y = if with_lyrics { 23.0 } else { 13.0 };
        if let Some((image, texture)) = &self.music_artwork {
            let [w, h] = image.size;
            let (uw, vh) = if w > h {
                (h as f32 / w as f32, 1.0)
            } else {
                (1.0, w as f32 / h as f32)
            };
            let uv = Rect::from_center_size(Pos2::new(0.5, 0.5), egui::vec2(uw, vh));
            c.painter.add(
                egui::epaint::RectShape::filled(
                    c.rect(44.0, art_y, art_size, art_size),
                    10.0 * c.s,
                    Color32::WHITE,
                )
                .with_texture(texture.id(), uv),
            );
        } else if has {
            let (a, b) = cover_colors(&np.title);
            let cc = c.clipped(44.0, art_y, art_size, art_size);
            cc.vgrad(44.0, art_y, art_size, art_size, 10.0, a, b);
            lucide::card(
                &cc,
                Icon::Music,
                44.0 + art_size / 2.0,
                art_y + art_size / 2.0,
                art_size * 0.39,
                Color32::from_white_alpha(235),
            );
        } else {
            c.fill(44.0, 13.0, 62.0, 62.0, 13.0, Color32::from_rgb(25, 25, 27));
            lucide::card(
                c,
                Icon::Music,
                75.0,
                44.0,
                26.0,
                Color32::from_rgb(110, 110, 114),
            );
        }

        if !has {
            c.text(
                121.0,
                47.0,
                language.text("暂无音乐", "No music playing"),
                14.0,
                W::Bold,
                theme::WHITE,
                H::Left,
            );
        } else {
            let text_x = if with_lyrics { 99.0 } else { 121.0 };
            let text_width = if with_lyrics { 123.0 } else { 220.0 };
            let title_size = if with_lyrics { 12.0 } else { 14.0 };
            c.text(
                text_x,
                if with_lyrics { 41.0 } else { 53.0 },
                &c.fit(&np.title, title_size, W::Bold, text_width),
                title_size,
                W::Bold,
                theme::WHITE,
                H::Left,
            );
            c.text(
                text_x,
                if with_lyrics { 57.0 } else { 67.0 },
                &c.fit(&np.artist, size::BODY, W::Regular, text_width),
                size::BODY,
                W::Regular,
                theme::GRAY,
                H::Left,
            );
            if with_lyrics {
                self.draw_music_lyrics(c);
            }
        }

        let tc = if has {
            theme::WHITE
        } else {
            Color32::from_rgb(78, 78, 82)
        };
        let (bx, bw, by) = if with_lyrics {
            (44.0, 176.0, 82.0)
        } else {
            (96.0, 221.0, 92.6)
        };
        let bar = c.hit_drag(bx - 4.0, by - 7.0, bw + 8.0, 14.0, "m-seek");
        let seek_position = if has
            && np.duration > 0.0
            && (bar.is_pointer_button_down_on() || bar.clicked() || bar.drag_stopped())
        {
            bar.interact_pointer_pos()
                .or_else(|| ctx.input(|i| i.pointer.interact_pos()))
                .map(|pointer| {
                    ((c.to_local(pointer).x - bx) / bw).clamp(0.0, 1.0) as f64 * np.duration
                })
        } else {
            None
        };
        let position = seek_position.unwrap_or(np.position);
        let frac = if np.duration > 0.0 {
            (position / np.duration).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        let (time_y, time_size, right) = if with_lyrics {
            (100.0, 10.0, 220.0)
        } else {
            (97.0, 15.0, 375.0)
        };
        c.text(
            44.0,
            time_y,
            &mmss(position),
            time_size,
            W::Regular,
            tc,
            H::Left,
        );
        let remaining = if has && np.duration <= 0.0 {
            "--:--".into()
        } else {
            format!("-{}", mmss((np.duration - position).max(0.0)))
        };
        c.text(
            right,
            time_y,
            &remaining,
            time_size,
            W::Regular,
            tc,
            H::Right,
        );
        let (bh, _) = c.hover_t(&bar);
        let bh = if has { bh } else { 0.0 };
        let th = lerp(3.5, 5.0, bh);
        c.fill(
            bx,
            by - th / 2.0,
            bw,
            th,
            th / 2.0,
            if has {
                Color32::from_gray(90)
            } else {
                theme::TRACK
            },
        );
        if frac > 0.0 {
            c.fill(
                bx,
                by - th / 2.0,
                (bw * frac).max(th),
                th,
                th / 2.0,
                theme::WHITE,
            );
        }
        if bh > 0.01 {
            c.faded(bh).circle(bx + bw * frac, by, 4.5, theme::WHITE);
        }
        if bar.clicked() || bar.drag_stopped() {
            if let Some(position) = seek_position {
                cmds.push(Cmd::Media(Transport::Seek(position)));
            }
        }

        let y = 118.0;
        let ic = if has {
            theme::WHITE
        } else {
            Color32::from_rgb(88, 88, 92)
        };
        let btn = |x: f32, w: f32, id: &str| {
            let response = c.hit(x - w / 2.0, y - 12.0, w, 24.0, id);
            let (hover, pressed) = c.hover_t(&response);
            if has && hover > 0.01 {
                c.faded(hover)
                    .circle(x, y, 12.5, Color32::from_white_alpha(20));
            }
            (response, c.scaled(x, y, 1.0 - 0.1 * pressed))
        };
        let (lyrics_button, lyrics_card) = btn(74.0, 26.0, "m-lyr");
        lyrics_card.text(
            74.0,
            y + 5.0,
            language.text("词", "L"),
            14.0,
            W::Semibold,
            if !has {
                ic
            } else if self.show_lyrics {
                theme::WHITE
            } else {
                theme::GRAY
            },
            H::Center,
        );
        let (previous, previous_card) = btn(162.0, 30.0, "m-prev");
        lucide::card(&previous_card, Icon::SkipBack, 162.0, y, 14.0, ic);
        let (play, play_card) = btn(210.5, 30.0, "m-play");
        paint::play_pause(&play_card, 211.0, y, 15.0, playing_t, ic);
        let (next, next_card) = btn(258.0, 30.0, "m-next");
        lucide::card(&next_card, Icon::SkipForward, 258.0, y, 14.0, ic);
        let (volume_button, volume_card) = btn(367.0, 26.0, "m-volume");
        let (volume, muted) = self.volume_ui.unwrap_or((0, false));
        lucide::card(
            &volume_card,
            if muted {
                Icon::VolumeOff
            } else {
                Icon::VolumeOn
            },
            367.0,
            y,
            16.0,
            ic,
        );
        if self.volume_ui.is_some()
            && volume_button
                .on_hover_text(language.text(
                    if muted { "取消静音" } else { "静音" },
                    if muted { "Unmute" } else { "Mute" },
                ))
                .clicked()
        {
            self.volume_ui = Some((volume, !muted));
            self.volume_quiet_until = self.t + 0.9;
            cmds.push(Cmd::ToggleMute);
        }
        if has {
            if previous.clicked() {
                cmds.push(Cmd::Media(Transport::Prev));
            }
            if play.clicked() {
                cmds.push(Cmd::Media(Transport::Toggle));
            }
            if next.clicked() {
                cmds.push(Cmd::Media(Transport::Next));
            }
        }
        if lyrics_button
            .on_hover_text(if self.show_lyrics {
                language.text("隐藏歌词", "Hide lyrics")
            } else {
                language.text("显示歌词", "Show lyrics")
            })
            .clicked()
        {
            if !has {
                self.say(language.text("先放一首歌吧", "Start a song first"));
            } else {
                self.show_lyrics = !self.show_lyrics;
                self.settings.lyrics_enabled = self.show_lyrics;
                self.settings.save();
            }
        }
        // 岛屿歌词是横向的，只在顶部停靠时提供。
        if has && !self.side() {
            let (button, button_card) = capsule(
                c,
                279.0,
                108.0,
                68.0,
                20.0,
                "m-island-lyrics",
                if self.settings.island_lyrics {
                    Color32::from_gray(39)
                } else {
                    Color32::from_gray(22)
                },
                Color32::from_gray(47),
            );
            button_card.text(
                313.0,
                122.0,
                language.text("岛屿歌词", "Lyrics"),
                10.0,
                W::Semibold,
                if self.settings.island_lyrics {
                    theme::WHITE
                } else {
                    theme::GRAY
                },
                H::Center,
            );
            if button
                .on_hover_text(if self.settings.island_lyrics {
                    language.text("关闭收起时的歌词", "Hide lyrics when collapsed")
                } else {
                    language.text(
                        "收起时显示当前歌词",
                        "Show the current lyric while collapsed",
                    )
                })
                .clicked()
            {
                self.settings.island_lyrics = !self.settings.island_lyrics;
                self.settings.save();
            }
        }
    }

    fn draw_music_lyrics(&mut self, c: &Card) {
        let language = self.settings.language;
        let col = c.clipped(243.0, 28.0, 136.0, 74.0);
        c.seg((233.0, 32.0), (233.0, 98.0), 1.0, Color32::from_gray(35));
        if let Some(lyrics) = &self.ly.lyrics {
            let current = if lyrics.synced {
                lyrics.index_at(self.lyric_position())
            } else {
                Some(0)
            };
            let target = current.unwrap_or(0);
            let id = egui::Id::new("music-lyrics-scroll");
            if !self.ly.scroll_animation_started {
                c.ui.ctx().animate_value_with_time(id, target as f32, 0.0);
                self.ly.scroll_animation_started = true;
            }
            let position = c.ui.ctx().animate_value_with_time(id, target as f32, 0.3);
            let first = (position.floor() as usize).saturating_sub(2);
            for (index, line) in lyrics.lines.iter().enumerate().skip(first).take(6) {
                let distance = index as f32 - position;
                if distance.abs() > 2.2 {
                    continue;
                }
                let active = current == Some(index);
                let ink = if active { theme::WHITE } else { theme::GRAY };
                let row = col.faded((1.0 - distance.abs() * 0.35).clamp(0.0, 1.0));
                let mut job = egui::text::LayoutJob::simple(
                    line.text.clone(),
                    row.font(
                        if active { 12.0 } else { 10.0 },
                        if active { W::Semibold } else { W::Regular },
                    ),
                    ink,
                    132.0 * c.s,
                );
                job.wrap.max_rows = if active { 3 } else { 1 };
                job.wrap.break_anywhere = true;
                let galley = row.painter.layout_job(job);
                let y = 64.0 + distance * 29.0 - galley.size().y / (2.0 * c.s);
                row.painter.galley(row.pos(246.0, y), galley, ink);
            }
            if !lyrics.synced {
                col.text(
                    246.0,
                    39.0,
                    language.text("文本歌词", "Plain lyrics"),
                    8.0,
                    W::Regular,
                    theme::GRAY,
                    H::Left,
                );
            }
        } else {
            let status = if self.ly.loading {
                language.text("正在查找歌词…", "Finding lyrics…")
            } else if let Some(error) = &self.ly.error {
                error.label_for(language)
            } else {
                language.text("暂无歌词", "No lyrics")
            };
            let galley = col.painter.layout(
                status.into(),
                col.font(10.0, W::Regular),
                theme::GRAY,
                130.0 * c.s,
            );
            col.painter.galley(
                col.pos(246.0, 61.0 - galley.size().y / (2.0 * c.s)),
                galley,
                theme::GRAY,
            );
            if self.ly.error.is_some() {
                col.text(
                    246.0,
                    87.0,
                    language.text("点击重试", "Retry"),
                    9.0,
                    W::Regular,
                    theme::GRAY,
                    H::Left,
                );
                if c.hit(243.0, 28.0, 136.0, 74.0, "lyrics-retry")
                    .on_hover_text(
                        language.text("重新查询当前歌曲的歌词", "Search for lyrics again"),
                    )
                    .clicked()
                {
                    self.ly.key.clear();
                }
            }
        }
    }

    // ================= 文件口袋 =================
    pub(crate) fn page_shelf(&mut self, c: &Card, _env: &Env, cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        let n = self.shelf.items.len();
        let drag = self.drag_a;
        if n > 5 {
            let position = if language.is_en() {
                format!(
                    "{}–{} / {n} · scroll to browse",
                    self.shelf_first + 1,
                    (self.shelf_first + 5).min(n)
                )
            } else {
                format!(
                    "{}–{} / {n} · 滚轮查看",
                    self.shelf_first + 1,
                    (self.shelf_first + 5).min(n)
                )
            };
            c.text(
                209.5,
                121.0,
                &position,
                10.5,
                W::Regular,
                theme::DIM,
                H::Center,
            );
        }

        let body = c.faded(lerp(1.0, 0.14, drag));
        if n == 0 {
            let a = 1.0 - drag;
            if a > 0.0 {
                let ec = body.faded(a);
                ec.dashed(
                    CONTENT_LEFT,
                    58.0,
                    CONTENT_WIDTH,
                    66.0,
                    12.0,
                    1.2,
                    Color32::from_gray(70),
                    0.0,
                );
                lucide::card(
                    &ec,
                    Icon::Folder,
                    209.5,
                    81.0,
                    26.0,
                    Color32::from_gray(200),
                );
                ec.text(
                    209.5,
                    105.0,
                    language.text("把文件拖到这儿", "Drop files here"),
                    12.0,
                    W::Semibold,
                    theme::WHITE,
                    H::Center,
                );
                ec.text(
                    209.5,
                    119.0,
                    language.text(
                        "只记位置，不复制文件 · 点一下打开",
                        "Stores a shortcut, not a copy · Click to open",
                    ),
                    size::TINY,
                    W::Regular,
                    theme::GRAY,
                    H::Center,
                );
            }
        } else {
            self.shelf_row(&body, cmds);
        }
        if drag > 0.002 {
            self.drag_overlay(&c.faded(drag));
        }
    }

    fn shelf_row(&mut self, c: &Card, cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        let n = self.shelf.items.len();
        let slot = 68.0;
        let center = 209.5;
        let visible = n.min(5) as f32;
        let scroll = self.shelf_scroll.pos;
        let row = c.clipped(CONTENT_LEFT, 54.0, CONTENT_WIDTH, 72.0);
        let dt = self.dt;
        let items = self.shelf.items.clone();
        self.icons.retain(&items);
        self.shelf_x.retain(|p, _| items.contains(p));
        for (i, p) in items.iter().enumerate() {
            let target = center + (i as f32 - scroll - (visible - 1.0) / 2.0) * slot;
            let born = self.shelf_born.get(p).copied();
            let s = self.shelf_x.entry(p.clone()).or_insert_with(|| {
                // 新放入的从右侧滑进来；启动时已有的直接就位
                let start = if born.is_some() {
                    target + slot * 0.9
                } else {
                    target
                };
                Spring::new(start, 230.0, 0.9)
            });
            s.target = target;
            s.step(dt);
            let x = s.pos;
            if x < 0.0 || x > 420.0 {
                continue;
            }
            let appear = born
                .map(|b| smoothstep(0.0, 0.35, (self.t - b) as f32))
                .unwrap_or(1.0);
            let edge = smoothstep(26.0, 60.0, x) * smoothstep(392.0, 358.0, x);
            let a = appear * edge;
            if a <= 0.01 {
                continue;
            }
            let r = row.hit_drag(x - 31.0, 54.0, 62.0, 70.0, ("shelf", p));
            let show_hint = self.shelf_menu_rect.is_none()
                && !r.context_menu_opened()
                && !r.secondary_clicked();
            if show_hint {
                r.clone().on_hover_text(p.display().to_string());
            }
            let (h, pr) = row.hover_t(&r);
            let cc = row.faded(a).scaled(x, 88.0, 1.0 - 0.04 * pr);
            if h > 0.01 {
                cc.fill(
                    x - 31.0,
                    54.0,
                    62.0,
                    70.0,
                    10.0,
                    Color32::from_white_alpha((16.0 * h) as u8),
                );
            }
            let lift = -2.0 * h + (1.0 - appear) * 8.0;
            self.file_icon_at(&cc, p, x, 75.0 + lift, 32.0);
            let name = p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let name = cc.fit(&name, size::SMALL, W::Semibold, 60.0);
            cc.text(
                x,
                105.0,
                &name,
                size::SMALL,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            let meta = self
                .icons
                .get(c.ui.ctx(), p)
                .map(|preview| preview.metadata)
                .unwrap_or_default();
            let meta = if language.is_en() {
                match meta.as_str() {
                    "文件夹" => "Folder",
                    "已移动" => "Moved",
                    _ => &meta,
                }
                .to_string()
            } else {
                meta
            };
            cc.text(
                x,
                119.0,
                &meta,
                size::TINY,
                W::Regular,
                theme::GRAY,
                H::Center,
            );
            if r.drag_started() && p.exists() {
                cmds.push(Cmd::DragFile(p.clone()));
            }
            let remove = row.hit(x + 15.0, 54.0, 17.0, 18.0, ("shelf-remove", p));
            let remove_card = row.faded(a);
            remove_card.circle(x + 23.5, 63.0, 7.5, Color32::from_rgb(55, 40, 46));
            lucide::card(
                &remove_card,
                Icon::Close,
                x + 23.5,
                63.0,
                11.0,
                theme::WHITE,
            );
            let mut delete = remove.clicked();
            if show_hint {
                remove.on_hover_text(language.text(
                    "从文件口袋移除，保留原文件",
                    "Remove from Pocket and keep the source file",
                ));
            }
            r.context_menu(|ui| {
                if ui.button(language.text("打开", "Open")).clicked() {
                    if p.exists() {
                        cmds.push(Cmd::Open(p.clone()));
                    }
                    ui.close_menu();
                }
                if ui
                    .button(language.text("在文件夹中显示", "Show in folder"))
                    .clicked()
                {
                    cmds.push(Cmd::Reveal(p.clone()));
                    ui.close_menu();
                }
                if ui
                    .button(language.text("复制完整路径", "Copy full path"))
                    .clicked()
                {
                    cmds.push(Cmd::Copy(p.display().to_string()));
                    ui.close_menu();
                }
                if ui
                    .button(language.text("从文件口袋移除", "Remove from Pocket"))
                    .clicked()
                {
                    delete = true;
                    ui.close_menu();
                }
                self.shelf_menu_rect =
                    Some(egui::Frame::menu(ui.style()).widget_rect(ui.min_rect()));
            });
            if delete {
                self.shelf.remove(p);
                self.icons.remove(p);
                self.shelf_x.remove(p);
                self.shelf_born.remove(p);
                self.shelf_first = self
                    .shelf_first
                    .min(self.shelf.items.len().saturating_sub(5));
                self.pet.react(crate::extras::PetState::Play);
            } else if r.clicked() {
                if p.exists() {
                    cmds.push(Cmd::Open(p.clone()));
                } else {
                    self.say(language.text(
                        "文件已移动或删除，可从文件口袋移除",
                        "The file was moved or deleted. You can remove it from Pocket.",
                    ));
                }
            }
        }
    }

    // ================= 剪贴板（文件口袋第二个分栏） =================
    pub(crate) fn page_clipboard(&mut self, c: &Card, _env: &Env, _cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        if !self.settings.clipboard_history {
            c.dashed(
                CONTENT_LEFT,
                58.0,
                CONTENT_WIDTH,
                66.0,
                12.0,
                1.2,
                Color32::from_gray(70),
                0.0,
            );
            c.text(
                209.5,
                80.0,
                language.text("剪贴板记录已关闭", "Clipboard history is off"),
                12.0,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            c.text(
                209.5,
                94.0,
                language.text(
                    "开启后，最近复制的文字会出现在这里",
                    "Turn it on to show recently copied text here",
                ),
                size::TINY,
                W::Regular,
                theme::GRAY,
                H::Center,
            );
            let (r, bc) = capsule(
                c,
                179.5,
                100.0,
                60.0,
                20.0,
                "clip-enable",
                theme::BTN,
                theme::BTN_HOVER,
            );
            bc.centered_text(
                209.5,
                110.0,
                language.text("开启", "Enable"),
                size::SMALL,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            if r.clicked() {
                self.settings.clipboard_history = true;
                self.clipboard.set_enabled(true);
                self.settings.save();
            }
            return;
        }
        let items = self.clipboard.lock().items.clone();
        let n = items.len();
        self.clip_first = self.clip_first.min(n.saturating_sub(CLIP_ROWS));

        if n == 0 {
            c.text(
                209.5,
                76.0,
                language.text("复制的文字会出现在这里", "Copied text will appear here"),
                12.0,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            c.text(
                209.5,
                94.0,
                language.text(
                    "记录保存在本机，重新打开仍可查看",
                    "History is stored locally and survives restarts",
                ),
                size::TINY,
                W::Regular,
                theme::GRAY,
                H::Center,
            );
            return;
        }

        // 超过一屏时右侧细滚动条，滚轮查看。
        if n > CLIP_ROWS {
            let (top, track) = (38.0, 104.0);
            compact::scrollbar(c, 384.0, top, track, self.clip_first, n - CLIP_ROWS);
        }
        let mut action: Option<(usize, u8)> = None; // 0 复制 1 固定 2 删除
        for (row, (index, clip)) in items
            .iter()
            .enumerate()
            .skip(self.clip_first)
            .take(CLIP_ROWS)
            .enumerate()
        {
            let row_top = 36.0 + row as f32 * 21.5;
            let center_y = row_top + 10.75;
            let row_width = CONTENT_WIDTH - 14.0;
            let r = c.hit(
                CONTENT_LEFT,
                row_top,
                row_width,
                21.5,
                ("clip", index, clip.text.len(), clip.at.as_str()),
            );
            let (hover, _) = c.hover_t(&r);
            compact::paint_hover(c, CONTENT_LEFT, row_top, row_width, 21.0, hover > 0.5);
            let text = c.fit(
                &crate::clipboard::preview(&clip.text),
                compact::PRIMARY_SIZE,
                W::Semibold,
                250.0,
            );
            c.text(
                CONTENT_LEFT + 10.0,
                row_top + 14.0,
                &text,
                compact::PRIMARY_SIZE,
                W::Semibold,
                theme::WHITE,
                H::Left,
            );
            compact::time(c, 306.0, row_top + 14.0, &clip.at);
            let tip: String = clip.text.chars().take(400).collect();
            let r = r.on_hover_text(format!(
                "{tip}{}\n{}",
                if clip.text.chars().count() > 400 {
                    "…"
                } else {
                    ""
                },
                language.text("点击复制", "Click to copy")
            ));
            if clip.pinned {
                r.clone().context_menu(|ui| {
                    if ui.button(language.text("取消固定", "Unpin")).clicked() {
                        action = Some((index, 3));
                        ui.close_menu();
                    }
                });
            }
            let pin = c
                .hit(316.0, row_top + 2.0, 22.0, 18.0, ("clip-pin", index))
                .on_hover_text(if clip.pinned {
                    language.text("取消固定", "Unpin")
                } else {
                    language.text("固定到第一位", "Pin to first")
                });
            if pin.hovered() {
                c.fill(
                    316.0,
                    row_top + 2.0,
                    22.0,
                    18.0,
                    5.0,
                    Color32::from_white_alpha(12),
                );
            }
            lucide::card(
                c,
                if clip.pinned {
                    Icon::PinFilled
                } else {
                    Icon::Pin
                },
                327.0,
                center_y,
                12.0,
                if clip.pinned || pin.hovered() {
                    theme::WHITE
                } else {
                    Color32::from_white_alpha(150)
                },
            );
            let delete = c
                .hit(340.0, row_top + 2.0, 22.0, 18.0, ("clip-delete", index))
                .on_hover_text(language.text("删除这条记录", "Delete this entry"));
            if delete.hovered() {
                c.fill(
                    340.0,
                    row_top + 2.0,
                    22.0,
                    18.0,
                    5.0,
                    Color32::from_white_alpha(12),
                );
            }
            lucide::card(
                c,
                Icon::Close,
                351.0,
                center_y,
                10.5,
                if delete.hovered() {
                    theme::WHITE
                } else {
                    theme::DIM
                },
            );
            if row + 1 < CLIP_ROWS.min(n - self.clip_first) {
                compact::paint_separator(c, CONTENT_LEFT + 10.0, CONTENT_RIGHT - 22.0, row_top);
            }
            if delete.clicked() {
                action = Some((index, 2));
            } else if pin.clicked() {
                action = Some((index, 1));
            } else if r.clicked() {
                action = Some((index, 0));
            }
        }
        match action {
            Some((index, 0)) => {
                #[cfg(windows)]
                let copied = crate::clipboard::copy_history(&items[index].text);
                #[cfg(windows)]
                self.say(if copied {
                    language.text("已复制", "Copied")
                } else {
                    language.text("复制失败，请重试", "Copy failed. Try again.")
                });
                #[cfg(windows)]
                self.pet.react(crate::extras::PetState::Idle);
                #[cfg(not(windows))]
                _cmds.push(Cmd::Copy(items[index].text.clone()));
            }
            Some((index, 1)) => {
                let was_pinned = items.get(index).is_some_and(|item| item.pinned);
                let changed = if was_pinned {
                    self.clipboard.lock().unpin(index)
                } else {
                    self.clipboard.lock().pin_to_front(index)
                };
                if changed {
                    if !was_pinned {
                        self.clip_first = 0;
                    }
                    self.pet.react(if was_pinned {
                        crate::extras::PetState::Idle
                    } else {
                        crate::extras::PetState::Naming
                    });
                } else {
                    self.say(if language.is_en() {
                        format!("You can pin up to {} entries", crate::clipboard::MAX_PINNED)
                    } else {
                        format!("最多固定 {} 条", crate::clipboard::MAX_PINNED)
                    });
                }
            }
            Some((index, 2)) => {
                self.clipboard.lock().remove(index);
                self.pet.react(crate::extras::PetState::Play);
            }
            Some((index, _)) => {
                self.clipboard.lock().unpin(index);
                self.pet.react(crate::extras::PetState::Idle);
            }
            None => {}
        }
    }

    // ================= 图片剪贴板 =================
    pub(crate) fn page_images(&mut self, c: &Card, _env: &Env, cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        if !self.settings.clipboard_images {
            self.image_disabled(c);
            return;
        }
        let items = self.clipboard.lock().images.clone();
        let n = items.len();
        self.image_first = self.image_first.min(n.saturating_sub(IMAGE_COLS));
        let live: std::collections::HashSet<_> = items.iter().map(|item| item.id.clone()).collect();
        self.image_textures.retain(|id, _| live.contains(id));
        if n == 0 {
            c.dashed(
                CONTENT_LEFT,
                58.0,
                CONTENT_WIDTH,
                66.0,
                12.0,
                1.2,
                Color32::from_gray(70),
                0.0,
            );
            lucide::card(c, Icon::Image, 209.5, 81.0, 24.0, Color32::from_gray(200));
            c.text(
                209.5,
                105.0,
                language.text(
                    "复制或截取的图片会出现在这里",
                    "Copied images will appear here",
                ),
                12.0,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            c.text(
                209.5,
                119.0,
                language.text("悬浮预览 · 点击打开", "Hover to preview · Click to open"),
                size::TINY,
                W::Regular,
                theme::GRAY,
                H::Center,
            );
            return;
        }
        c.fill(CONTENT_LEFT, 54.0, CONTENT_WIDTH, 72.0, 12.0, theme::CARD);
        if n > IMAGE_COLS {
            let label = if language.is_en() {
                format!(
                    "{}–{} / {n} · scroll",
                    self.image_first + 1,
                    (self.image_first + IMAGE_COLS).min(n)
                )
            } else {
                format!(
                    "{}–{} / {n} · 滚轮查看",
                    self.image_first + 1,
                    (self.image_first + IMAGE_COLS).min(n)
                )
            };
            c.text(209.5, 121.0, &label, 9.5, W::Regular, theme::DIM, H::Center);
        }
        let mut hovered = None;
        let mut action = None;
        for (slot, (index, item)) in items
            .iter()
            .enumerate()
            .skip(self.image_first)
            .take(IMAGE_COLS)
            .enumerate()
        {
            let x = 73.5 + slot as f32 * 68.0;
            let r = c.hit(x - 31.0, 54.0, 62.0, 72.0, ("image", item.id.clone()));
            let (hover, pressed) = c.hover_t(&r);
            if hover > 0.01 {
                c.fill(
                    x - 31.0,
                    54.0,
                    62.0,
                    72.0,
                    10.0,
                    Color32::from_white_alpha((18.0 * hover) as u8),
                );
                hovered = Some((index, item.clone(), x));
            }
            if let Some(texture) = self.image_texture(c, item) {
                draw_texture_fit(
                    c,
                    &texture,
                    Rect::from_center_size(
                        c.pos(x, 88.0 - pressed * 1.0),
                        Vec2::new(54.0, 54.0) * c.s,
                    ),
                    8.0,
                );
            } else {
                c.fill(x - 27.0, 61.0, 54.0, 54.0, 8.0, Color32::from_gray(42));
                lucide::card(c, Icon::Image, x, 88.0, 20.0, theme::GRAY);
            }
            if item.pinned {
                lucide::card(c, Icon::Pin, x + 22.0, 62.0, 11.0, theme::BLUE);
            }
            r.context_menu(|ui| {
                if ui.button(language.text("打开图片", "Open image")).clicked() {
                    action = Some((index, 0));
                    ui.close_menu();
                }
                if ui
                    .button(language.text("在文件夹中显示", "Show in folder"))
                    .clicked()
                {
                    action = Some((index, 1));
                    ui.close_menu();
                }
                if ui
                    .button(if item.pinned {
                        language.text("取消固定", "Unpin")
                    } else {
                        language.text("固定", "Pin")
                    })
                    .clicked()
                {
                    action = Some((index, 2));
                    ui.close_menu();
                }
                if ui
                    .button(language.text("删除记录", "Remove entry"))
                    .clicked()
                {
                    action = Some((index, 3));
                    ui.close_menu();
                }
                self.shelf_menu_rect =
                    Some(egui::Frame::menu(ui.style()).widget_rect(ui.min_rect()));
            });
            if r.clicked() {
                action = Some((index, 0));
            }
        }
        if let Some((_, item, anchor_x)) = hovered {
            if let Some(texture) = self.image_texture(c, &item) {
                if let Some(rect) = draw_image_preview(c, &texture, &item, anchor_x, 419.0) {
                    self.image_preview_active = true;
                    self.image_preview_rect = Some(rect);
                }
            }
        }
        if let Some((index, action)) = action {
            let Some(item) = items.get(index).cloned() else {
                return;
            };
            match action {
                0 => cmds.push(Cmd::Open(item.path())),
                1 => cmds.push(Cmd::Reveal(item.path())),
                2 => {
                    if !self.clipboard.lock().toggle_image_pin(index) {
                        self.say(if language.is_en() {
                            format!(
                                "You can pin up to {} images",
                                crate::clipboard::MAX_PINNED_IMAGES
                            )
                        } else {
                            format!("最多固定 {} 张图片", crate::clipboard::MAX_PINNED_IMAGES)
                        });
                    }
                }
                _ => self.clipboard.lock().remove_image(index),
            }
        }
    }

    fn image_disabled(&mut self, c: &Card) {
        let language = self.settings.language;
        c.text(
            209.5,
            80.0,
            language.text("图片记录已关闭", "Image history is off"),
            12.0,
            W::Semibold,
            theme::WHITE,
            H::Center,
        );
        c.text(
            209.5,
            94.0,
            language.text(
                "开启后，复制的图片会保存在本机",
                "Turn it on to keep copied images locally",
            ),
            size::TINY,
            W::Regular,
            theme::GRAY,
            H::Center,
        );
        let (r, bc) = capsule(
            c,
            179.5,
            102.0,
            60.0,
            20.0,
            "image-enable",
            theme::BTN,
            theme::BTN_HOVER,
        );
        bc.centered_text(
            209.5,
            112.0,
            language.text("开启", "Enable"),
            size::SMALL,
            W::Semibold,
            theme::WHITE,
            H::Center,
        );
        if r.clicked() {
            self.settings.clipboard_images = true;
            self.clipboard.set_images_enabled(true);
            self.settings.save();
        }
    }

    fn image_texture(
        &mut self,
        c: &Card,
        item: &crate::clipboard::ImageClip,
    ) -> Option<egui::TextureHandle> {
        if let Some((_, texture)) = self.image_textures.get(&item.id) {
            return Some(texture.clone());
        }
        let decoded = image::open(item.path()).ok()?;
        let preview = decoded.thumbnail(1024, 1024).to_rgba8();
        let image = std::sync::Arc::new(egui::ColorImage::from_rgba_unmultiplied(
            [preview.width() as usize, preview.height() as usize],
            preview.as_raw(),
        ));
        let texture = c.ui.ctx().load_texture(
            format!("clipboard-image-{}", item.id),
            image.as_ref().clone(),
            egui::TextureOptions::LINEAR,
        );
        self.image_textures
            .insert(item.id.clone(), (image, texture.clone()));
        Some(texture)
    }

    fn file_icon_at(&mut self, c: &Card, p: &std::path::Path, cx: f32, cy: f32, sz: f32) {
        let tex = self
            .icons
            .get(c.ui.ctx(), p)
            .and_then(|preview| preview.image);
        if let Some(tex) = tex {
            let s = tex.size_vec2();
            let k = sz / s.x.max(s.y);
            let r = Rect::from_center_size(c.pos(cx, cy), s * k * c.s);
            c.painter.image(
                tex.id(),
                r,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            paint::file_glyph(c, paint::file_kind(p), cx, cy, sz);
        }
    }

    /// 拖入遮罩：流动虚线框 + 跟随光标的柔光 + 上下轻跳的托盘图标。
    fn drag_overlay(&self, c: &Card) {
        let language = self.settings.language;
        let (x, y, w, h) = (CONTENT_LEFT, 58.0, CONTENT_WIDTH, 66.0);
        c.fill(x, y, w, h, 12.0, Color32::from_black_alpha(110));
        if let Some(p) = self.cursor_local {
            let px = p.x.clamp(x + 20.0, x + w - 20.0);
            let py = p.y.clamp(y + 10.0, y + h - 10.0);
            c.clipped(x, y, w, h).glow(
                px,
                py,
                90.0,
                Color32::from_rgba_unmultiplied(120, 170, 255, 34),
            );
        }
        c.dashed(
            x,
            y,
            w,
            h,
            12.0,
            1.4,
            Color32::from_white_alpha(230),
            self.t as f32 * 14.0,
        );
        lucide::card(c, Icon::Archive, 209.5, 86.0, 28.0, theme::WHITE);
        c.text(
            209.5,
            112.0,
            language.text("松手放进文件口袋", "Release to add to Pocket"),
            12.0,
            W::Semibold,
            theme::WHITE,
            H::Center,
        );
    }

    // ---------------------------------------------------------------
    // 邮件：有验证码大卡时分栏，否则显示紧凑单行列表。
    // ---------------------------------------------------------------
    fn visible_mail_code(&self) -> Option<&mail::Item> {
        self.mail
            .code_item()
            .filter(|item| self.settings.mail_code && !self.mail_code_hidden(item))
    }
    pub(crate) fn mail_list_bounds(&self) -> Rect {
        let left = if self.visible_mail_code().is_some() {
            CONTENT_LEFT + 150.0
        } else {
            CONTENT_LEFT
        };
        Rect::from_min_max(
            Pos2::new(left, 36.0),
            Pos2::new(CONTENT_RIGHT, 36.0 + MAIL_ROWS as f32 * 21.5),
        )
    }
    pub(crate) fn page_mail(&mut self, c: &Card, _env: &Env, cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        if self.mail.accounts.is_empty() {
            c.text(
                CONTENT_LEFT,
                60.0,
                language.text("还没有连接邮箱", "No mailbox connected"),
                13.0,
                W::Bold,
                theme::WHITE,
                H::Left,
            );
            let hint = c.fit(
                language.text(
                    "连接邮箱后，新邮件和验证码会直接出现在岛屿上",
                    "Connect a mailbox to receive new-mail summaries and verification codes.",
                ),
                10.5,
                W::Regular,
                CONTENT_WIDTH - 10.0,
            );
            c.text(
                CONTENT_LEFT,
                77.0,
                &hint,
                10.5,
                W::Regular,
                theme::GRAY,
                H::Left,
            );
            let (go, gc) = capsule(
                c,
                CONTENT_LEFT,
                88.0,
                96.0,
                26.0,
                "mail-setup",
                theme::BTN,
                theme::BTN_HOVER,
            );
            gc.centered_text(
                CONTENT_LEFT + 48.0,
                101.0,
                language.text("配置邮箱", "Set up"),
                size::BODY,
                W::Regular,
                theme::WHITE,
                H::Center,
            );
            if go.clicked() {
                self.open_mail_settings();
            }
            return;
        }

        // 点开邮件后整块换成正文摘要，左上「返回」回到列表。
        if let Some((account, uid)) = self.mail.open_uid.clone() {
            match self
                .mail
                .recent
                .iter()
                .find(|m| m.uid == uid && m.account.eq_ignore_ascii_case(&account))
                .cloned()
            {
                Some(item) => {
                    self.mail_detail(c, &item, cmds);
                    return;
                }
                None => self.mail.open_uid = None,
            }
        }
        let code = self.visible_mail_code().cloned();
        if let Some(raw) = &code {
            c.fill(CONTENT_LEFT, 36.0, 146.0, 86.0, 11.0, theme::CARD);
            let hidden = self.mail_code_hidden(raw);
            let item = self.display_mail(raw);
            let value = item.code.as_deref().unwrap_or("••••••");
            c.text(
                CONTENT_LEFT + 10.0,
                51.0,
                language.text("验证码", "Verification code"),
                10.5,
                W::Regular,
                theme::GRAY,
                H::Left,
            );
            c.text(
                CONTENT_LEFT + 136.0,
                51.0,
                &c.fit(&item.from, 10.5, W::Semibold, 78.0),
                10.5,
                W::Semibold,
                theme::WHITE,
                H::Right,
            );
            c.text(
                CONTENT_LEFT + 10.0,
                75.0,
                value,
                23.0,
                W::Bold,
                theme::GREEN,
                H::Left,
            );
            c.text(
                CONTENT_LEFT + 10.0,
                91.0,
                &c.fit(&item.subject, 10.5, W::Regular, 126.0),
                10.5,
                W::Regular,
                Color32::from_gray(190),
                H::Left,
            );
            c.text(
                CONTENT_LEFT + 10.0,
                108.0,
                if hidden {
                    language.text("已隐藏 · 点击查看", "Hidden · Click to view")
                } else {
                    language.text("点击复制", "Click to copy")
                },
                10.5,
                W::Regular,
                theme::DIM,
                H::Left,
            );
            c.text(
                CONTENT_LEFT + 136.0,
                108.0,
                item.time.rsplit(' ').next().unwrap_or(&item.time),
                10.0,
                W::Regular,
                theme::GRAY,
                H::Right,
            );
            c.hit(CONTENT_LEFT, 38.0, 134.0, 16.0, "mail-code-source")
                .on_hover_text(if language.is_en() {
                    format!(
                        "From: {}\n{}\nMailbox: {}\n{}",
                        item.from, item.subject, item.account, item.time
                    )
                } else {
                    format!(
                        "来自：{}\n{}\n收件邮箱：{}\n{}",
                        item.from, item.subject, item.account, item.time
                    )
                });
            if c.hit(
                CONTENT_LEFT + 4.0,
                56.0,
                138.0,
                38.0,
                ("mail-code", item.uid),
            )
            .on_hover_text(if language.is_en() {
                format!("Copy the verification code from {}", item.from)
            } else {
                format!("复制 {} 的验证码", item.from)
            })
            .clicked()
            {
                if hidden {
                    self.reveal_mail_code(raw);
                } else {
                    cmds.push(Cmd::Copy(value.to_string()));
                    self.code_copied(value);
                }
            }
        }
        let list_bounds = self.mail_list_bounds();
        let list_x = list_bounds.left();
        let list_w = list_bounds.width();
        let row_w = list_w - 12.0;
        let row_h = compact::ROW_HEIGHT;
        if self.mail.recent.is_empty() {
            c.text(
                list_x + 6.0,
                66.0,
                language.text("还没有收到邮件", "No mail received yet"),
                11.0,
                W::Regular,
                theme::GRAY,
                H::Left,
            );
            return;
        }
        let mut content = egui::Ui::new(
            c.ui.ctx().clone(),
            egui::Id::new(("mail-list", c.live)),
            egui::UiBuilder::new()
                .max_rect(c.rect(
                    list_bounds.left(),
                    list_bounds.top(),
                    list_bounds.width(),
                    list_bounds.height(),
                ))
                .layer_id(c.painter.layer_id())
                .style(c.ui.style().clone()),
        );
        content.set_clip_rect(
            c.rect(
                list_bounds.left(),
                list_bounds.top(),
                list_bounds.width(),
                list_bounds.height(),
            )
            .intersect(c.painter.clip_rect()),
        );
        content.set_opacity(c.painter.opacity());
        if !c.live {
            content.disable();
        }
        content.spacing_mut().item_spacing.y = 0.0;
        content.spacing_mut().scroll.floating = false;
        content.spacing_mut().scroll.bar_width = 4.0 * c.s;
        content.visuals_mut().widgets.inactive.bg_fill = Color32::from_gray(80);
        egui::ScrollArea::vertical()
            .id_salt("mail-list-scroll")
            .max_height(list_bounds.height() * c.s)
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show_rows(
                &mut content,
                row_h * c.s,
                self.mail.recent.len(),
                |ui, range| {
                    let rows: Vec<_> = self.mail.recent[range]
                        .iter()
                        .map(|item| (self.mail_code_hidden(item), self.display_mail(item)))
                        .collect();
                    for (hidden, item) in &rows {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(row_w * c.s, row_h * c.s),
                            egui::Sense::hover(),
                        );
                        let mut row = Card::new(ui, rect.min, ui.clip_rect());
                        row.s = c.s;
                        row.live = c.live;
                        let hit = row.hit(
                            0.0,
                            0.0,
                            row_w,
                            row_h - 1.0,
                            ("mail-row", item.uid, item.account.as_str()),
                        );
                        let selected = code.as_ref().is_some_and(|m| {
                            m.uid == item.uid && m.account.eq_ignore_ascii_case(&item.account)
                        });
                        if hit.hovered() || selected {
                            compact::paint_hover(&row, 0.0, 0.0, row_w, row_h - 2.0, true);
                        }
                        if selected {
                            row.fill(0.0, 6.0, 2.0, 20.0, 1.0, theme::GREEN);
                        }
                        row.text(
                            6.0,
                            14.0,
                            &row.fit(&item.from, compact::PRIMARY_SIZE, W::Semibold, 62.0),
                            compact::PRIMARY_SIZE,
                            W::Semibold,
                            theme::WHITE,
                            H::Left,
                        );
                        let subject_w = row_w - 118.0 - if *hidden { 34.0 } else { 0.0 };
                        let width = row.text(
                            74.0,
                            14.0,
                            &row.fit(
                                &item.subject,
                                compact::SECONDARY_SIZE,
                                W::Regular,
                                subject_w,
                            ),
                            compact::SECONDARY_SIZE,
                            W::Regular,
                            Color32::from_gray(200),
                            H::Left,
                        );
                        if *hidden {
                            row.fill(78.0 + width, 5.0, 30.0, 12.0, 3.0, theme::BTN);
                            row.text(
                                93.0 + width,
                                14.0,
                                language.text("已隐藏", "Hidden"),
                                8.5,
                                W::Regular,
                                theme::GRAY,
                                H::Center,
                            );
                        }
                        compact::time(
                            &row,
                            row_w - 6.0,
                            14.0,
                            item.time.rsplit(' ').next().unwrap_or(&item.time),
                        );
                        compact::paint_separator(&row, 6.0, row_w - 6.0, 0.0);
                        hit.clone().on_hover_text(if language.is_en() {
                            format!(
                                "{}\n{}\nMailbox: {}\n{}",
                                item.from, item.subject, item.account, item.time
                            )
                        } else {
                            format!(
                                "{}\n{}\n收件邮箱：{}\n{}",
                                item.from, item.subject, item.account, item.time
                            )
                        });
                        if hit.clicked() {
                            self.mail.open_uid = Some((item.account.clone(), item.uid));
                        }
                    }
                },
            );
    }

    /// 岛内邮件详情可滚动，也可以进入独立窗口阅读全文。
    pub(crate) fn mail_detail(&mut self, c: &Card, raw: &mail::Item, cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        let hidden = self.mail_code_hidden(raw);
        let display = self.display_mail(raw);
        let item = &display;
        c.fill(CONTENT_LEFT, 36.0, CONTENT_WIDTH, 188.0, 11.0, theme::CARD);
        let back = c.hit(330.0, 42.0, 44.0, 14.0, ("mail-back", item.uid));
        let bh = c.hover_t(&back).0;
        c.text(
            CONTENT_RIGHT - 6.0,
            50.0,
            language.text("返回", "Back"),
            10.5,
            W::Regular,
            if bh > 0.5 { theme::WHITE } else { theme::GRAY },
            H::Right,
        );
        if back.clicked() {
            self.mail.open_uid = None;
            return;
        }
        let from = c.fit(&item.from, 11.5, W::Semibold, 210.0);
        c.text(46.0, 50.0, &from, 11.5, W::Semibold, theme::WHITE, H::Left);
        let read = c
            .hit(
                266.0,
                42.0,
                60.0,
                14.0,
                ("mail-read", &item.account, item.uid),
            )
            .on_hover_text(language.text(
                "在独立窗口阅读完整正文",
                "Open the full message in a separate window",
            ));
        c.text(
            324.0,
            50.0,
            language.text("阅读全文", "Read full"),
            10.5,
            W::Regular,
            if read.hovered() {
                theme::BLUE
            } else {
                theme::GRAY
            },
            H::Right,
        );
        if read.clicked() {
            self.mail.reading = Some(raw.clone());
            self.mail.reader_focus = true;
        }
        let subject = c.fit(&item.subject, 13.5, W::Semibold, 328.0);
        c.text(
            46.0,
            73.0,
            &subject,
            13.5,
            W::Semibold,
            theme::WHITE,
            H::Left,
        );
        c.hit(
            46.0,
            58.0,
            328.0,
            20.0,
            ("mail-subject", &item.account, item.uid),
        )
        .on_hover_text(&item.subject);
        c.text(
            CONTENT_RIGHT - 6.0,
            92.0,
            &item.time,
            10.0,
            W::Regular,
            Color32::from_gray(170),
            H::Right,
        );
        let account = if language.is_en() {
            format!("Mailbox: {}", item.account)
        } else {
            format!("收件邮箱：{}", item.account)
        };
        c.text(
            46.0,
            92.0,
            &c.fit(&account, 10.0, W::Regular, 230.0),
            10.0,
            W::Regular,
            Color32::from_gray(175),
            H::Left,
        );
        c.hit(
            46.0,
            80.0,
            230.0,
            16.0,
            ("mail-account", &item.account, item.uid),
        )
        .on_hover_text(&account);

        // 正文和链接共用滚动区域，会议与验证码操作保留在底部。
        let mut content = egui::Ui::new(
            c.ui.ctx().clone(),
            egui::Id::new(("mail-content", &item.account, item.uid, c.live)),
            egui::UiBuilder::new()
                .max_rect(c.rect(46.0, 104.0, 328.0, 90.0))
                .layer_id(c.painter.layer_id())
                .style(c.ui.style().clone()),
        );
        content.set_clip_rect(
            c.rect(46.0, 104.0, 328.0, 90.0)
                .intersect(c.painter.clip_rect()),
        );
        content.set_opacity(c.painter.opacity());
        if !c.live {
            content.disable();
        }
        content.spacing_mut().item_spacing.y = 6.0 * c.s;
        content.style_mut().spacing.scroll.floating = false;
        content.spacing_mut().scroll.bar_width = 4.0 * c.s;
        egui::ScrollArea::vertical()
            .id_salt(("mail-body", &item.account, item.uid))
            .max_height(90.0 * c.s)
            .auto_shrink([false, false])
            .show(&mut content, |ui| {
                if !item.summary.is_empty() {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(item.summary.as_ref())
                                .font(c.font(12.0, W::Regular))
                                .color(Color32::from_gray(220)),
                        )
                        .wrap(),
                    );
                } else if item.links.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            language.text("（这封邮件没有正文）", "(This message has no body)"),
                        )
                        .font(c.font(12.0, W::Regular))
                        .color(theme::DIM),
                    );
                }
                for link in &item.links {
                    let text = format!("{} ↗", c.fit(&link.label, 12.0, W::Regular, 295.0));
                    if ui
                        .add(egui::Link::new(
                            egui::RichText::new(text)
                                .font(c.font(12.0, W::Regular))
                                .color(theme::BLUE),
                        ))
                        .on_hover_text(&link.url)
                        .clicked()
                    {
                        cmds.push(Cmd::OpenUrl(link.url.clone()));
                    }
                }
            });

        // Bottom row: verification code and copy affordance.
        c.seg(
            (46.0, 199.0),
            (CONTENT_RIGHT - 6.0, 199.0),
            0.6,
            Color32::from_gray(48),
        );
        let y = 214.0;
        if hidden {
            c.text(
                46.0,
                y,
                language.text(
                    "验证码已隐藏（不代表失效）",
                    "Verification code hidden (it may still be valid)",
                ),
                11.0,
                W::Regular,
                theme::GRAY,
                H::Left,
            );
            c.text(
                CONTENT_RIGHT - 6.0,
                y,
                language.text("查看 15 秒", "Show for 15 s"),
                10.5,
                W::Regular,
                theme::BLUE,
                H::Right,
            );
            if c.hit(290.0, y - 14.0, 86.0, 24.0, ("mail-reveal", item.uid))
                .clicked()
            {
                self.reveal_mail_code(raw);
            }
        } else if let Some(code) = &item.code {
            let label = if language.is_en() {
                format!("Code {code}")
            } else {
                format!("验证码 {code}")
            };
            let w = c.text(46.0, y, &label, 13.0, W::Semibold, theme::GREEN, H::Left);
            c.text(
                46.0 + w + 6.0,
                y,
                language.text("点击复制", "Click to copy"),
                10.5,
                W::Regular,
                theme::DIM,
                H::Left,
            );
            let hit = c.hit(
                42.0,
                y - 14.0,
                w + 60.0,
                24.0,
                ("mail-detail-code", item.uid),
            );
            if hit.clicked() {
                cmds.push(Cmd::Copy(code.clone()));
                self.code_copied(code);
            }
        }
    }
}
