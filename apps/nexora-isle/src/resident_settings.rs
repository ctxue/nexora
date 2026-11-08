use crate::{
    bar_layout::{valid_order, BATTERY, BELL, CLOCK, DEFAULT_ORDER, MUSIC, PET},
    i18n::Language,
    lucide::{self, Icon},
    settings_theme::palette,
};
use eframe::egui::{self, Color32, RichText, Sense, Stroke};

impl crate::app::App {
    pub(crate) fn resident_order_settings(&mut self, ui: &mut egui::Ui, language: Language) {
        let default = if self.settings.show_battery {
            DEFAULT_ORDER.to_vec()
        } else {
            vec![PET, CLOCK, MUSIC, BELL, BATTERY]
        };
        let mut order = if valid_order(&self.settings.resident_order) {
            self.settings.resident_order.clone()
        } else {
            default.clone()
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(language.text("岛上项目顺序", "Island item order"))
                    .size(11.5)
                    .color(ui.visuals().text_color()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(language.text("恢复默认", "Reset"))
                    .clicked()
                {
                    self.settings.resident_order.clear();
                    order = default.clone();
                }
            });
        });
        ui.label(
            RichText::new(language.text(
                "拖动整行调整顺序。按条件显示的组件会保留位置，满足条件时自动出现。",
                "Drag a row to reorder it. Conditional items keep their slot and appear when relevant.",
            ))
            .size(10.5)
            .color(palette(ui.ctx()).muted),
        );
        ui.add_space(7.0);

        let palette = palette(ui.ctx());
        let mut movement = None;
        for (index, &slot) in order.clone().iter().enumerate() {
            let row = ui
                .push_id(("resident-order", slot), |ui| {
                    let (rect, response) = ui
                        .allocate_exact_size(egui::vec2(ui.available_width(), 38.0), Sense::drag());
                    let response = response.on_hover_cursor(egui::CursorIcon::Grab);
                    let dragged = response.dragged();
                    if response.hovered() || dragged {
                        ui.painter().rect_filled(
                            rect,
                            7.0,
                            if dragged {
                                palette.surface_selected
                            } else {
                                palette.surface_hover
                            },
                        );
                    }
                    ui.painter().rect_stroke(
                        rect,
                        7.0,
                        Stroke::new(1.0_f32, palette.line_soft),
                        egui::StrokeKind::Inside,
                    );
                    lucide::paint(
                        ui.painter(),
                        egui::Rect::from_center_size(
                            egui::pos2(rect.left() + 16.0, rect.center().y),
                            egui::vec2(13.0, 13.0),
                        ),
                        Icon::GripVertical,
                        palette.subtle,
                    );
                    ui.painter().text(
                        egui::pos2(rect.left() + 31.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        resident_label(slot, language),
                        egui::FontId::proportional(11.5),
                        palette.text,
                    );
                    ui.painter().text(
                        egui::pos2(rect.right() - 12.0, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        resident_state(self, slot, language),
                        egui::FontId::proportional(10.5),
                        palette.muted,
                    );
                    response
                })
                .inner;
            row.dnd_set_drag_payload(slot);
            if let Some(payload) = row.dnd_hover_payload::<usize>() {
                let payload = *payload;
                if payload != slot {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    let payload_index = order
                        .iter()
                        .position(|value| *value == payload)
                        .unwrap_or(index);
                    let y = if payload_index < index {
                        row.rect.bottom()
                    } else {
                        row.rect.top()
                    };
                    ui.painter().line_segment(
                        [
                            egui::pos2(row.rect.left(), y),
                            egui::pos2(row.rect.right(), y),
                        ],
                        Stroke::new(2.0_f32, Color32::from_rgb(70, 150, 255)),
                    );
                }
            }
            if let Some(payload) = row.dnd_release_payload::<usize>() {
                movement = Some((*payload, index));
            }
        }
        if let Some((slot, to)) = movement {
            let from = order.iter().position(|value| *value == slot).unwrap();
            if from < to {
                order[from..=to].rotate_left(1);
            } else if to < from {
                order[to..=from].rotate_right(1);
            }
            self.settings.resident_order = order;
        }
    }
}

fn resident_label(slot: usize, language: Language) -> &'static str {
    match slot {
        BATTERY => language.text("电量", "Battery"),
        PET => language.text("岛屿表情", "Island face"),
        CLOCK => language.text("时间", "Time"),
        MUSIC => language.text("音乐", "Music"),
        BELL => language.text("提醒", "Alerts"),
        _ => "",
    }
}

fn resident_state(app: &crate::app::App, slot: usize, language: Language) -> &'static str {
    match slot {
        BATTERY if !app.settings.show_battery => language.text("已关闭", "Off"),
        BATTERY if app.resident_power.0.is_none() => language.text("无电池时隐藏", "Hidden"),
        BATTERY => language.text("按设备显示", "Automatic"),
        PET if !app.settings.pet => language.text("已关闭", "Off"),
        PET => language.text("收起时显示", "When collapsed"),
        CLOCK => language.text("始终显示", "Always visible"),
        MUSIC if app.media.is_music() => language.text("正在显示", "Visible now"),
        MUSIC => language.text("播放时显示", "During playback"),
        BELL if app.t < app.notification_bell_until => language.text("正在显示", "Visible now"),
        BELL => language.text("有提醒时显示", "When notified"),
        _ => "",
    }
}
