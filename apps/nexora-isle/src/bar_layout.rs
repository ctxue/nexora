//! Top-bar slot layout with continuous movement when retained controls change.

use crate::anim::{smoothstep, Spring};

pub(crate) const CLOCK: usize = 0;
pub(crate) const BATTERY: usize = 1;
pub(crate) const MUSIC: usize = 2;
pub(crate) const BELL: usize = 3;
pub(crate) const PET: usize = 4;
pub(crate) const SLOT_COUNT: usize = 5;

const EDGE_INSET: f32 = 12.0;
const SLOT_GAP: f32 = 10.0;
const WIDTHS: [f32; SLOT_COUNT] = [44.0, 20.0, 14.0, 20.0, 28.0];
pub(crate) const DEFAULT_ORDER: [usize; SLOT_COUNT] = [BATTERY, PET, CLOCK, MUSIC, BELL];

pub(crate) fn valid_order(order: &[usize]) -> bool {
    order.len() == SLOT_COUNT
        && DEFAULT_ORDER
            .iter()
            .all(|slot| order.iter().filter(|value| *value == slot).count() == 1)
}

pub(crate) fn animations_enabled() -> bool {
    #[cfg(windows)]
    {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ENABLED.get_or_init(|| {
            use windows::Win32::{Foundation::BOOL, UI::WindowsAndMessaging::*};
            let mut enabled = BOOL(1);
            unsafe {
                let _ = SystemParametersInfoW(
                    SPI_GETCLIENTAREAANIMATION,
                    0,
                    Some((&mut enabled as *mut BOOL).cast()),
                    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                );
            }
            enabled.as_bool()
        })
    }
    #[cfg(not(windows))]
    {
        true
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Layout {
    pub width: f32,
    pub x: Vec<Option<f32>>,
    pub widths: Vec<f32>,
}

impl Layout {
    pub fn new(battery: bool, music: bool, bell: bool, pet: bool) -> Self {
        fn place(cursor: &mut f32, width: f32) -> f32 {
            let x = *cursor + width / 2.0;
            *cursor += width + SLOT_GAP;
            x
        }

        let mut x = vec![None; SLOT_COUNT];
        let mut cursor = EDGE_INSET;
        for slot in [BATTERY, CLOCK, MUSIC, BELL, PET] {
            let visible = match slot {
                BATTERY => battery,
                CLOCK => true,
                MUSIC => music,
                BELL => bell,
                PET => pet,
                _ => false,
            };
            if visible {
                x[slot] = Some(place(&mut cursor, WIDTHS[slot]));
            }
        }

        let width = (cursor - SLOT_GAP + EDGE_INSET).max(96.0);
        for value in x.iter_mut().flatten() {
            *value -= width / 2.0;
        }
        Self {
            width,
            x,
            widths: WIDTHS.to_vec(),
        }
    }

    pub fn widen_clock(&mut self, extra: f32) {
        let Some(clock) = self.x[CLOCK] else {
            return;
        };
        for (slot, x) in self.x.iter_mut().enumerate() {
            if slot != CLOCK {
                if let Some(x) = x {
                    *x += if *x < clock {
                        -extra / 2.0
                    } else {
                        extra / 2.0
                    };
                }
            }
        }
        self.width += extra;
    }

    pub fn reorder(&mut self, order: &[usize]) {
        if !valid_order(order) {
            return;
        }
        let original = self.x.clone();
        let mut cursor = -self.width / 2.0 + EDGE_INSET;
        for slot in order.iter().copied() {
            let Some(center) = original[slot] else {
                continue;
            };
            let left = center - self.widths[slot] / 2.0;
            let right = center + self.widths[slot] / 2.0;
            self.x[slot] = Some(cursor + center - left);
            cursor += right - left + SLOT_GAP;
        }
    }

    pub fn clearance(&self, slot: usize) -> f32 {
        if self.x.get(slot).copied().flatten().is_none() {
            return 0.0;
        }
        self.pet_clearance(slot)
    }

    pub fn pet_clearance(&self, slot: usize) -> f32 {
        match (self.x[slot], self.x[PET]) {
            (Some(x), Some(pet)) => smoothstep(
                self.widths[slot] / 2.0 + 14.0,
                self.widths[slot] / 2.0 + 24.0,
                (x - pet).abs(),
            ),
            _ => 1.0,
        }
    }
}

pub(crate) struct Motion {
    pub target: Layout,
    positions: Vec<Spring>,
    initialized: bool,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            target: Layout::new(false, false, false, false),
            positions: vec![Spring::new(0.0, 360.0, 0.76); SLOT_COUNT],
            initialized: false,
        }
    }
}

impl Motion {
    pub fn step(&mut self, target: Layout, dt: f32, animate: bool) {
        self.positions
            .resize(target.x.len(), Spring::new(0.0, 360.0, 0.76));
        for (i, x) in target.x.iter().enumerate() {
            if let Some(x) = x {
                let spring = &mut self.positions[i];
                spring.target = *x;
                if !self.initialized
                    || self.target.x.get(i).copied().flatten().is_none()
                    || !animate
                {
                    spring.snap(*x);
                } else {
                    spring.step(dt);
                }
            }
        }
        self.target = target;
        self.initialized = true;
    }

    pub fn current(&self) -> Layout {
        let mut layout = self.target.clone();
        for (i, x) in layout.x.iter_mut().enumerate() {
            if x.is_some() {
                *x = Some(self.positions[i].pos);
            }
        }
        layout
    }

    pub fn position(&self, slot: usize) -> f32 {
        self.positions[slot].pos
    }

    pub fn settled(&self) -> bool {
        self.target
            .x
            .iter()
            .enumerate()
            .all(|(i, x)| x.is_none() || self.positions[i].settled(100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_components_never_overlap() {
        for battery in [false, true] {
            for music in [false, true] {
                for bell in [false, true] {
                    for pet in [false, true] {
                        let layout = Layout::new(battery, music, bell, pet);
                        let mut boxes: Vec<_> = layout
                            .x
                            .iter()
                            .enumerate()
                            .filter_map(|(i, x)| {
                                x.map(|x| (x - layout.widths[i] / 2.0, x + layout.widths[i] / 2.0))
                            })
                            .collect();
                        boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
                        assert!(boxes.windows(2).all(|pair| pair[1].0 - pair[0].1 >= 6.9));
                    }
                }
            }
        }
    }

    #[test]
    fn custom_order_preserves_only_retained_slots() {
        assert!(!valid_order(&[CLOCK, CLOCK, MUSIC, BELL, PET]));
        assert!(!valid_order(&[CLOCK, MUSIC]));
        let order = [PET, MUSIC, CLOCK, BELL, BATTERY];
        let mut layout = Layout::new(true, true, false, true);
        layout.reorder(&order);
        let positions: Vec<_> = order.iter().filter_map(|slot| layout.x[*slot]).collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
