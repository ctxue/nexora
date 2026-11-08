//! NOMI expressions and scene-aware expression selection.

use crate::anim::{approach, hash01, lerp, smoothstep, Spring};
use crate::paint::Card;
use eframe::egui::Color32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NomiCategory {
    Idle,
    Naming,
    Play,
}

impl NomiCategory {
    pub const ALL: [NomiCategory; 3] = [Self::Idle, Self::Naming, Self::Play];

    pub fn label_for(self, language: crate::i18n::Language) -> &'static str {
        match self {
            Self::Idle => language.text("闲暇", "Idle"),
            Self::Naming => language.text("起名", "Naming"),
            Self::Play => language.text("玩耍", "Play"),
        }
    }

    pub fn expressions(self) -> &'static [NomiExpression] {
        match self {
            Self::Idle => &IDLE_EXPRESSIONS,
            Self::Naming => &NAMING_EXPRESSIONS,
            Self::Play => &PLAY_EXPRESSIONS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NomiExpression {
    Neutral,
    HeartHands,
    Lick,
    Hearts,
    Stars,
    Drink,
}

const IDLE_EXPRESSIONS: [NomiExpression; 3] = [
    NomiExpression::Neutral,
    NomiExpression::HeartHands,
    NomiExpression::Lick,
];
const NAMING_EXPRESSIONS: [NomiExpression; 2] = [NomiExpression::Hearts, NomiExpression::Stars];
const PLAY_EXPRESSIONS: [NomiExpression; 1] = [NomiExpression::Drink];

impl NomiExpression {
    pub const ALL: [NomiExpression; 6] = [
        NomiExpression::Neutral,
        NomiExpression::HeartHands,
        NomiExpression::Lick,
        NomiExpression::Hearts,
        NomiExpression::Stars,
        NomiExpression::Drink,
    ];

    pub fn category(self) -> NomiCategory {
        match self {
            Self::Neutral | Self::HeartHands | Self::Lick => NomiCategory::Idle,
            Self::Hearts | Self::Stars => NomiCategory::Naming,
            Self::Drink => NomiCategory::Play,
        }
    }

    pub fn label_for(self, language: crate::i18n::Language) -> &'static str {
        match self {
            Self::Neutral => language.text("默认", "Default"),
            Self::HeartHands => language.text("比心", "Heart hands"),
            Self::Lick => language.text("舔嘴唇", "Licking lips"),
            Self::Hearts => language.text("爱心", "Hearts"),
            Self::Stars => language.text("星星眼", "Star eyes"),
            Self::Drink => language.text("喝饮料", "Drink"),
        }
    }

    fn tracks_pointer(self) -> bool {
        self == Self::Neutral
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PetState {
    Idle,
    Naming,
    Play,
}

impl PetState {
    pub fn category(self) -> NomiCategory {
        match self {
            Self::Idle => NomiCategory::Idle,
            Self::Naming => NomiCategory::Naming,
            Self::Play => NomiCategory::Play,
        }
    }
}

pub(crate) fn gaze_for_cursor(cursor: Option<eframe::egui::Pos2>, cx: f32) -> [f32; 2] {
    let Some(cursor) = cursor else {
        return [0.0, 0.0];
    };
    [
        ((cursor.x - cx) / 180.0).clamp(-1.0, 1.0),
        ((cursor.y - 15.0) / 120.0).clamp(-1.0, 1.0),
    ]
}

/// 0 = 完全睁开，1 = 完全合眼。
fn blink_fraction(phase: f32) -> f32 {
    const CLOSE_END: f32 = 0.075;
    const HOLD_END: f32 = 0.12;
    const OPEN_END: f32 = 0.195;
    if phase < 0.0 || phase >= OPEN_END {
        0.0
    } else if phase < CLOSE_END {
        smoothstep(0.0, CLOSE_END, phase)
    } else if phase < HOLD_END {
        1.0
    } else {
        1.0 - smoothstep(HOLD_END, OPEN_END, phase)
    }
}

fn pick_expression(scene: PetState, seed: u64, current: Option<NomiExpression>) -> NomiExpression {
    let pool = scene.category().expressions();
    let start = (hash01(seed) * pool.len() as f32).floor() as usize % pool.len();
    if pool.len() > 1 && current == Some(pool[start]) {
        pool[(start + 1) % pool.len()]
    } else {
        pool[start]
    }
}

pub(crate) struct Pet {
    pub state: PetState,
    pub reaction: Option<(PetState, f32)>,
    current: NomiExpression,
    previous: NomiExpression,
    transition: f32,
    scene_age: f32,
    next_expression_at: f32,
    seed: u64,
    gaze: [Spring; 2],
    blink_clock: f32,
    blink_started_at: f32,
    double_blink_at: Option<f32>,
    next_blink_at: f32,
}

impl Default for Pet {
    fn default() -> Self {
        Self {
            state: PetState::Idle,
            reaction: None,
            current: NomiExpression::Neutral,
            previous: NomiExpression::Neutral,
            transition: 1.0,
            scene_age: 0.0,
            next_expression_at: 8.0,
            seed: 0x4e4f_4d49,
            gaze: [Spring::new(0.0, 680.0, 0.78); 2],
            blink_clock: 0.0,
            blink_started_at: -1.0,
            double_blink_at: None,
            next_blink_at: 1.4,
        }
    }
}

impl Pet {
    pub fn react(&mut self, state: PetState) {
        self.reaction = Some((state, 2.8));
        self.seed = self.seed.wrapping_add(0x9e37_79b9);
        self.change_expression(state, self.seed);
        self.scene_age = 0.0;
    }

    pub fn advance(&mut self, dt: f32, visible: bool) {
        if visible {
            if let Some((_, remaining)) = &mut self.reaction {
                *remaining -= dt;
                if *remaining <= 0.0 {
                    self.reaction = None;
                }
            }
        }
    }

    fn change_expression(&mut self, state: PetState, seed: u64) {
        let next = if state == PetState::Idle {
            NomiExpression::Neutral
        } else {
            pick_expression(state, seed, Some(self.current))
        };
        if next != self.current {
            self.previous = self.current;
            self.current = next;
            self.transition = 0.0;
        }
        self.next_expression_at = 5.0 + hash01(seed.wrapping_add(17)) * 5.0;
    }

    pub fn step(&mut self, state: PetState, t: f64, dt: f32, gaze: [f32; 2], visible: bool) {
        if state != self.state {
            self.state = state;
            self.scene_age = 0.0;
            self.seed = self.seed.wrapping_add(0x517c_c1b7);
            self.change_expression(state, (t * 1000.0) as u64 ^ self.seed);
        }
        if visible {
            self.scene_age += dt;
            if self.scene_age >= self.next_expression_at {
                self.scene_age = 0.0;
                self.seed = self.seed.wrapping_add(0xa24b_aed4);
                self.change_expression(state, self.seed);
            }
            self.advance_blink(dt);
        }
        self.transition = approach(self.transition, 1.0, 14.0, dt);
        let target = if visible { gaze } else { [0.0, 0.0] };
        for (axis, target) in self.gaze.iter_mut().zip(target) {
            axis.target = target;
            axis.step(dt);
        }
    }

    pub fn draw(&self, c: &Card, cx: f32, t: f64) {
        let gaze = self.gaze_position();
        let blink = self.blink_fraction();
        if self.transition < 0.999 && self.previous != self.current {
            draw_expression_frame(
                &c.faded(1.0 - self.transition),
                self.previous,
                cx,
                self.eye_anchor_y(),
                t,
                gaze,
                blink,
            );
        }
        draw_expression_frame(
            &c.faded(self.transition.clamp(0.0, 1.0)),
            self.current,
            cx,
            self.eye_anchor_y(),
            t,
            gaze,
            blink,
        );
    }

    fn gaze_position(&self) -> [f32; 2] {
        [self.gaze[0].pos, self.gaze[1].pos]
    }

    fn eye_anchor_y(&self) -> f32 {
        15.0
    }

    fn blink_fraction(&self) -> f32 {
        blink_fraction(self.blink_clock - self.blink_started_at)
    }

    fn advance_blink(&mut self, dt: f32) {
        self.blink_clock += dt;
        if let Some(second) = self.double_blink_at {
            if self.blink_clock >= second {
                self.start_blink(false);
            }
        } else if self.blink_clock >= self.next_blink_at {
            let random = hash01(self.seed ^ (self.blink_clock * 1000.0).round().to_bits() as u64);
            self.start_blink(random < 0.22);
        }
    }

    fn start_blink(&mut self, double: bool) {
        self.blink_started_at = self.blink_clock;
        self.double_blink_at = double.then_some(self.blink_clock + 0.32);
        let random =
            hash01(self.seed ^ 0x8f3a_2d1c ^ (self.blink_clock * 1000.0).round().to_bits() as u64);
        self.next_blink_at = self.blink_clock + 3.0 + random * 4.2;
    }
}

pub(crate) fn draw_expression(
    c: &Card,
    expression: NomiExpression,
    cx: f32,
    cy: f32,
    t: f64,
    gaze: [f32; 2],
) {
    draw_expression_frame(c, expression, cx, cy, t, gaze, 0.0);
}

fn draw_expression_frame(
    c: &Card,
    expression: NomiExpression,
    cx: f32,
    cy: f32,
    t: f64,
    gaze: [f32; 2],
    blink: f32,
) {
    let white = Color32::from_rgb(249, 246, 238);
    c.circle(cx, cy, 13.6, Color32::from_rgb(9, 9, 11));

    if expression.tracks_pointer() {
        draw_default_eyes(c, cx, cy, gaze, blink, white);
        return;
    }

    // Heart and star expressions contain no mouth, so center the complete
    // eye-and-symbol composition inside the face instead of the upper half.
    let face_y = face_anchor_y(expression, cy);
    draw_happy_eyes(c, cx, face_y - 3.5, white);
    let pulse = (t as f32 * 2.2).sin() * 0.45;
    match expression {
        NomiExpression::Neutral => unreachable!(),
        NomiExpression::HeartHands => {
            draw_heart(c, cx, cy + 6.0, 2.0, white);
            c.arc(cx - 4.3, cy + 7.4, 2.8, 3.3, 5.9, 1.0, white);
            c.arc(cx + 4.3, cy + 7.4, 2.8, 3.5, 6.1, 1.0, white);
        }
        NomiExpression::Lick => {
            c.arc(
                cx,
                cy + 4.4,
                4.0,
                0.15,
                std::f32::consts::PI - 0.15,
                1.1,
                white,
            );
            c.path(
                &[
                    (cx + 0.4, cy + 7.8),
                    (cx + 2.6, cy + 9.0),
                    (cx + 4.0, cy + 7.5),
                ],
                1.0,
                white,
            );
        }
        NomiExpression::Hearts => {
            draw_heart(c, cx + 4.8, face_y - 7.5, 1.3 + pulse * 0.08, white);
            draw_heart(c, cx + 7.2, face_y - 4.6, 1.05, white);
            draw_heart(c, cx + 7.8, face_y - 8.4, 0.8, white);
        }
        NomiExpression::Stars => {
            draw_star(c, cx - 2.0, face_y - 7.0, 1.8, white);
            draw_star(c, cx + 3.0, face_y - 8.5, 1.3, white);
            draw_star(c, cx + 7.0, face_y - 5.0, 1.0, white);
        }
        NomiExpression::Drink => {
            let bob = pulse * 0.3;
            c.path(
                &[
                    (cx - 8.0, cy + 3.0 + bob),
                    (cx - 2.2, cy + 3.0 + bob),
                    (cx - 3.0, cy + 9.0 + bob),
                    (cx - 7.0, cy + 9.0 + bob),
                ],
                1.0,
                white,
            );
            c.seg((cx - 6.7, cy + 5.0), (cx - 3.5, cy + 5.0), 0.8, white);
            c.seg((cx - 6.2, cy + 3.2), (cx - 7.6, cy - 3.0), 0.8, white);
        }
    }
}

fn face_anchor_y(expression: NomiExpression, cy: f32) -> f32 {
    if matches!(expression, NomiExpression::Hearts | NomiExpression::Stars) {
        cy + 6.0
    } else {
        cy
    }
}

fn draw_default_eyes(c: &Card, cx: f32, cy: f32, gaze: [f32; 2], blink: f32, color: Color32) {
    let blink = blink.clamp(0.0, 1.0);
    let eye_height = lerp(8.8, 1.0, blink);
    let eye_width = lerp(4.9, 5.4, blink);
    let eye_y = cy + blink * 0.35;
    let dx = gaze[0] * 1.35;
    let dy = gaze[1] * 1.75;
    let pupil_alpha = (1.0 - smoothstep(0.3, 0.68, blink)).clamp(0.0, 1.0);
    for x in [-5.4f32, 5.4] {
        let eye_x = cx + x;
        c.fill(
            eye_x - eye_width / 2.0,
            eye_y - eye_height / 2.0,
            eye_width,
            eye_height,
            eye_width / 2.0,
            color,
        );
        if pupil_alpha > 0.01 {
            let pupil = Color32::from_rgba_premultiplied(9, 9, 11, (255.0 * pupil_alpha) as u8);
            let pupil_x = eye_x + dx;
            let pupil_y = eye_y + dy;
            c.circle(pupil_x, pupil_y, 1.22, pupil);
            c.circle(
                pupil_x - 0.28,
                pupil_y - 0.34,
                0.28,
                Color32::from_rgba_unmultiplied(249, 246, 238, (225.0 * pupil_alpha) as u8),
            );
        }
    }
}

fn draw_happy_eyes(c: &Card, cx: f32, y: f32, color: Color32) {
    c.arc(cx - 5.5, y, 2.6, 3.35, 6.05, 1.35, color);
    c.arc(cx + 5.5, y, 2.6, 3.35, 6.05, 1.35, color);
}

fn draw_heart(c: &Card, cx: f32, cy: f32, size: f32, color: Color32) {
    c.circle(cx - size * 0.45, cy - size * 0.18, size * 0.55, color);
    c.circle(cx + size * 0.45, cy - size * 0.18, size * 0.55, color);
    c.poly(
        &[
            (cx - size * 0.95, cy - size * 0.05),
            (cx, cy + size * 1.15),
            (cx + size * 0.95, cy - size * 0.05),
        ],
        color,
    );
}

fn draw_star(c: &Card, cx: f32, cy: f32, radius: f32, color: Color32) {
    let points: Vec<_> = (0..10)
        .map(|i| {
            let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
            let r = if i % 2 == 0 { radius } else { radius * 0.42 };
            (cx + angle.cos() * r, cy + angle.sin() * r)
        })
        .collect();
    c.path(&points, 0.7, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_six_retained_expressions_are_present() {
        assert_eq!(NomiExpression::ALL.len(), 6);
        assert_eq!(NomiCategory::Idle.expressions().len(), 3);
        assert_eq!(NomiCategory::Naming.expressions().len(), 2);
        assert_eq!(NomiCategory::Play.expressions().len(), 1);
        for (index, expression) in NomiExpression::ALL.iter().enumerate() {
            assert!(
                NomiExpression::ALL[..index]
                    .iter()
                    .all(|other| other != expression),
                "duplicate NOMI expression: {expression:?}"
            );
        }
    }

    #[test]
    fn only_the_default_expression_tracks_the_pointer() {
        assert!(NomiExpression::Neutral.tracks_pointer());
        for expression in NomiExpression::ALL {
            if expression != NomiExpression::Neutral {
                assert!(!expression.tracks_pointer(), "{expression:?}");
            }
        }
    }

    #[test]
    fn eye_only_expressions_are_vertically_centered() {
        let center = 15.0;
        assert_eq!(face_anchor_y(NomiExpression::Hearts, center), center + 6.0);
        assert_eq!(face_anchor_y(NomiExpression::Stars, center), center + 6.0);
        for expression in [
            NomiExpression::HeartHands,
            NomiExpression::Lick,
            NomiExpression::Drink,
        ] {
            assert_eq!(face_anchor_y(expression, center), center);
        }
    }

    #[test]
    fn every_scene_selects_from_its_own_expression_group() {
        for state in [PetState::Idle, PetState::Naming, PetState::Play] {
            for seed in 0..64 {
                assert_eq!(
                    pick_expression(state, seed, None).category(),
                    state.category()
                );
            }
        }
    }

    #[test]
    fn multi_expression_scenes_do_not_repeat_back_to_back() {
        for state in [PetState::Idle, PetState::Naming] {
            for &current in state.category().expressions() {
                for seed in 0..64 {
                    assert_ne!(pick_expression(state, seed, Some(current)), current);
                }
            }
        }
    }

    #[test]
    fn pet_starts_with_the_centered_default_eyes() {
        let pet = Pet::default();
        assert_eq!(pet.current, NomiExpression::Neutral);
    }

    #[test]
    fn pointer_gaze_is_normalized_and_clamped_across_the_desktop() {
        let center = 320.0;
        assert_eq!(gaze_for_cursor(None, center), [0.0, 0.0]);
        assert_eq!(
            gaze_for_cursor(Some(eframe::egui::pos2(20.0, -200.0)), center),
            [-1.0, -1.0]
        );
        assert_eq!(
            gaze_for_cursor(Some(eframe::egui::pos2(900.0, 400.0)), center),
            [1.0, 1.0]
        );
    }

    #[test]
    fn gaze_uses_a_velocity_spring_and_reaches_the_pointer_quickly() {
        let mut pet = Pet::default();
        for _ in 0..12 {
            pet.step(PetState::Idle, 1.0, 1.0 / 60.0, [1.0, -0.5], true);
        }
        let gaze = pet.gaze_position();
        assert!(gaze[0] > 0.72, "horizontal gaze {}", gaze[0]);
        assert!(gaze[1] < -0.32, "vertical gaze {}", gaze[1]);
        for _ in 0..90 {
            pet.step(PetState::Idle, 2.5, 1.0 / 60.0, [0.0, 0.0], true);
        }
        let settled = pet.gaze_position();
        assert!(settled[0].abs() < 0.02 && settled[1].abs() < 0.02);
    }

    #[test]
    fn blink_curve_closes_quickly_and_reopens_smoothly() {
        assert_eq!(blink_fraction(-0.1), 0.0);
        assert!(blink_fraction(0.08) > 0.95);
        assert!(blink_fraction(0.12) > 0.95);
        assert!(blink_fraction(0.22) < 0.05);
        assert_eq!(blink_fraction(0.3), 0.0);
    }

    #[test]
    fn idle_pet_blinks_and_reopens_on_its_own() {
        let mut pet = Pet::default();
        let mut saw_closed = false;
        let mut reopened_after_blink = false;
        for frame in 0..(60 * 4) {
            pet.step(
                PetState::Idle,
                frame as f64 / 60.0,
                1.0 / 60.0,
                [0.0, 0.0],
                true,
            );
            let blink = pet.blink_fraction();
            if blink > 0.9 {
                saw_closed = true;
            } else if saw_closed && blink < 0.05 {
                reopened_after_blink = true;
            }
        }
        assert!(saw_closed, "the idle eyes never blinked");
        assert!(reopened_after_blink, "the idle eyes stayed closed");
    }

    #[test]
    fn idle_eye_anchor_stays_on_the_pixel_grid() {
        let mut pet = Pet::default();
        for frame in 0..180 {
            pet.step(
                PetState::Idle,
                frame as f64 / 60.0,
                1.0 / 60.0,
                [0.0, 0.0],
                true,
            );
            assert_eq!(pet.eye_anchor_y(), 15.0);
        }
    }

    #[test]
    fn idle_state_never_rotates_into_special_expressions() {
        let mut pet = Pet::default();
        for frame in 0..(60 * 30) {
            pet.step(
                PetState::Idle,
                frame as f64 / 60.0,
                1.0 / 60.0,
                [0.8, 0.2],
                true,
            );
            assert_eq!(pet.current, NomiExpression::Neutral);
        }
    }
}
