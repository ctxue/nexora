use crate::app::{App, Env, BAR_H_MIN, BAR_W_MIN, PILL_GAP};
use crate::model::{Dock, IslandShape};
use crate::paint::{self, Card, H};
use crate::theme::{self, W};
use eframe::egui::{Color32, Pos2, Rect, Vec2};

pub const SIDE_W: f32 = 500.0;
pub const SIDE_H: f32 = 600.0;
pub const SIDE_BAR_W: f32 = 36.0;
pub const SIDE_HALF_EXTENT: f32 = 150.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SideItem {
    Clock,
    Battery,
    Music,
    Bell,
}

impl SideItem {
    fn height(self) -> f32 {
        match self {
            Self::Clock => 32.0,
            Self::Battery => 20.0,
            Self::Music => 18.0,
            Self::Bell => 22.0,
        }
    }
}

impl App {
    pub(crate) fn dock(&self) -> Dock {
        self.settings.dock
    }
    pub(crate) fn side(&self) -> bool {
        self.settings.dock.is_side()
    }

    pub(crate) fn side_items(&self) -> Vec<SideItem> {
        let mut items = vec![SideItem::Clock];
        if self.settings.show_battery && self.resident_power.0.is_some() {
            items.push(SideItem::Battery);
        }
        if self.media.is_music() {
            items.push(SideItem::Music);
        }
        if self.t < self.notification_bell_until {
            items.push(SideItem::Bell);
        }
        items
    }

    pub(crate) fn side_bar_len(&self) -> f32 {
        14.0 + self.side_items().iter().map(|i| i.height()).sum::<f32>()
    }

    pub(crate) fn side_base_size(&self, minimal: bool) -> (f32, f32) {
        if minimal {
            (BAR_H_MIN, BAR_W_MIN)
        } else {
            (SIDE_BAR_W, self.side_bar_len().min(SIDE_H - 24.0))
        }
    }

    fn side_gap(&self) -> f32 {
        if self.settings.island_shape == IslandShape::Pill {
            PILL_GAP
        } else {
            0.0
        }
    }

    pub(crate) fn island_rect(&self, win: Rect, w: f32, h: f32) -> Rect {
        let gap = self.side_gap();
        match self.dock() {
            Dock::Top => Rect::from_min_size(
                Pos2::new(win.center().x - w / 2.0, win.top() + gap),
                Vec2::new(w, h),
            ),
            Dock::Left => Rect::from_min_size(
                Pos2::new(win.left() + gap, win.center().y - h / 2.0),
                Vec2::new(w, h),
            ),
            Dock::Right => Rect::from_min_size(
                Pos2::new(win.right() - gap - w, win.center().y - h / 2.0),
                Vec2::new(w, h),
            ),
        }
    }

    pub(crate) fn side_hit_rect(&self, win: Rect, p: f32) -> Rect {
        let rect = if p > 0.04 || self.open {
            self.island_rect(win, self.w.pos, self.h.pos)
                .expand2(Vec2::new(4.0, 2.0))
        } else {
            let (w, h) = (self.w.pos, self.h.pos);
            self.island_rect(win, w, h).expand2(Vec2::new(4.0, 6.0))
        };
        match self.dock() {
            Dock::Left => Rect::from_min_max(Pos2::new(win.left(), rect.top()), rect.max),
            Dock::Right => Rect::from_min_max(rect.min, Pos2::new(win.right(), rect.bottom())),
            Dock::Top => rect,
        }
    }

    pub(crate) fn side_bar(&mut self, c: &Card, width: f32, height: f32, env: &Env) {
        let items = self.side_items();
        let total: f32 = items.iter().map(|i| i.height()).sum();
        let cx = width / 2.0;
        let fitted = c.scaled(
            cx,
            height / 2.0,
            ((SIDE_H - 24.0) / (total + 14.0)).min(1.0),
        );
        let c = &fitted;
        let mut y = (height - total) / 2.0;
        for item in items {
            let h = item.height();
            let mid = y + h / 2.0;
            match item {
                SideItem::Clock => {
                    let (hour, minute) = side_clock(&self.settings.clock_text(&env.clock));
                    c.text(cx, mid - 1.0, &hour, 13.5, W::Bold, theme::WHITE, H::Center);
                    c.text(
                        cx,
                        mid + 13.0,
                        &minute,
                        13.5,
                        W::Bold,
                        theme::WHITE,
                        H::Center,
                    );
                }
                SideItem::Battery => paint::battery(c, cx, mid, env.battery, env.charging, 0.72),
                SideItem::Music => {
                    paint::playback_bars(c, cx, mid, self.t, self.media.playing, theme::WHITE)
                }
                SideItem::Bell => {
                    paint::bell(
                        &c.scaled(cx, mid, 1.1),
                        cx,
                        mid,
                        theme::WHITE,
                        paint::bell_swing(self.t - self.notification_bell_at),
                    );
                    c.circle(cx + 4.0, mid - 4.5, 2.0, Color32::from_rgb(255, 72, 72));
                }
            }
            y += h;
        }
    }
}

pub(crate) fn side_clock(text: &str) -> (String, String) {
    let digits: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ':')
        .collect();
    match digits.split_once(':') {
        Some((h, m)) if !h.is_empty() => (h.to_string(), m.chars().take(2).collect()),
        _ => ("--".into(), "--".into()),
    }
}

pub(crate) fn side_bar_rect(panel: Rect, width: f32, height: f32, dock: Dock) -> Rect {
    let left = match dock {
        Dock::Right => panel.right() - width,
        Dock::Left | Dock::Top => panel.left(),
    };
    Rect::from_min_size(
        Pos2::new(left, panel.center().y - height / 2.0),
        Vec2::new(width, height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_splits_into_two_lines() {
        assert_eq!(side_clock("09:41"), ("09".into(), "41".into()));
        assert_eq!(side_clock("下午 9:05"), ("9".into(), "05".into()));
        assert_eq!(side_clock("--:--"), ("--".into(), "--".into()));
    }

    #[test]
    fn collapsed_side_content_stays_attached_to_its_edge_while_expanding() {
        let narrow = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(36.0, 80.0));
        let wide = Rect::from_min_size(Pos2::new(100.0, 60.0), Vec2::new(419.0, 160.0));
        let left_narrow = side_bar_rect(narrow, 36.0, 80.0, Dock::Left);
        let left_wide = side_bar_rect(wide, 36.0, 80.0, Dock::Left);
        assert_eq!(left_narrow.min, left_wide.min);

        let right_narrow = Rect::from_min_max(Pos2::new(64.0, 100.0), Pos2::new(100.0, 180.0));
        let right_wide = Rect::from_min_max(Pos2::new(-319.0, 60.0), Pos2::new(100.0, 220.0));
        let right_narrow = side_bar_rect(right_narrow, 36.0, 80.0, Dock::Right);
        let right_wide = side_bar_rect(right_wide, 36.0, 80.0, Dock::Right);
        assert_eq!(right_narrow.min, right_wide.min);
    }
}
