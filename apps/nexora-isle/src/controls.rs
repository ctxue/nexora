use std::hash::Hash;

use eframe::egui::{pos2, vec2, Color32, Rect, Response};

use crate::lucide::{self, Icon};
use crate::paint::Card;
use crate::theme;

const PIN_HIT_SIZE: (f32, f32) = (22.0, 18.0);
const PIN_GLYPH_SIZE: f32 = 12.0;

pub(crate) fn pin_button_rect(cx: f32, cy: f32) -> Rect {
    Rect::from_center_size(pos2(cx, cy), vec2(PIN_HIT_SIZE.0, PIN_HIT_SIZE.1))
}

pub(crate) fn pin_button(c: &Card, pinned: bool, cx: f32, cy: f32, id: impl Hash) -> Response {
    let rect = pin_button_rect(cx, cy);
    let response = c.hit(rect.left(), rect.top(), rect.width(), rect.height(), id);
    if response.hovered() {
        c.fill(
            rect.left(),
            rect.top(),
            rect.width(),
            rect.height(),
            5.0,
            Color32::from_white_alpha(12),
        );
    }
    lucide::card(
        c,
        if pinned { Icon::PinFilled } else { Icon::Pin },
        cx,
        cy,
        PIN_GLYPH_SIZE,
        if pinned || response.hovered() {
            theme::WHITE
        } else {
            Color32::from_white_alpha(150)
        },
    );
    response
}

pub(crate) fn pin_badge(c: &Card, cx: f32, cy: f32, size: f32) {
    lucide::card(c, Icon::PinFilled, cx, cy, size, theme::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_button_hit_target_stays_centered_and_reusable() {
        let rect = pin_button_rect(100.0, 60.0);
        assert_eq!(rect.center(), eframe::egui::pos2(100.0, 60.0));
        assert_eq!(rect.width(), PIN_HIT_SIZE.0);
        assert_eq!(rect.height(), PIN_HIT_SIZE.1);
        assert_eq!(PIN_GLYPH_SIZE, 12.0);
    }
}
