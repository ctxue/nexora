//! Shared visual primitives for compact mailbox-style lists.

use crate::paint::{Card, H};
use crate::theme::W;
use eframe::egui::Color32;

pub const ROW_HEIGHT: f32 = 21.5;
pub const PRIMARY_SIZE: f32 = 11.0;
pub const SECONDARY_SIZE: f32 = 10.5;
pub const TIME_SIZE: f32 = 10.0;

pub fn paint_hover(c: &Card, x: f32, y: f32, w: f32, h: f32, hovered: bool) {
    if hovered {
        c.fill(x, y, w, h, 6.0, Color32::from_gray(27));
    }
}

pub fn paint_separator(c: &Card, left: f32, right: f32, row_top: f32) {
    c.seg(
        (left, row_top + ROW_HEIGHT - 0.5),
        (right, row_top + ROW_HEIGHT - 0.5),
        0.5,
        Color32::from_gray(40),
    );
}

pub fn time(c: &Card, right: f32, baseline: f32, value: &str) {
    c.text(
        right,
        baseline,
        value,
        TIME_SIZE,
        W::Regular,
        Color32::from_gray(170),
        H::Right,
    );
}

pub fn scrollbar(c: &Card, x: f32, top: f32, track: f32, first: usize, max: usize) {
    if max == 0 {
        return;
    }
    let thumb = (track / (max + 5) as f32 * 5.0).max(12.0);
    let y = top + (track - thumb) * first as f32 / max as f32;
    c.fill(x, top, 4.0, track, 2.0, Color32::from_gray(50));
    c.fill(x, y, 4.0, thumb, 2.0, Color32::from_gray(180));
}
