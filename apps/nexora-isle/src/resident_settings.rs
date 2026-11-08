use crate::{
    bar_layout::{move_item, valid_order, BATTERY, BELL, CLOCK, DEFAULT_ORDER, MUSIC, PET},
    i18n::Language,
    lucide::{self, Icon},
    model::page,
    settings_theme::Palette,
};
use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke};

impl crate::app::App {
    pub(crate) fn island_layout_editor(&mut self, ui: &mut egui::Ui, language: Language) {
        let palette = crate::settings_theme::palette(ui.ctx());
        self.layout_preview(ui, language, palette);
        ui.add_space(10.0);

        self.section_heading(
            ui,
            language.text("收起状态", "Collapsed island"),
            language.text(
                "拖动卡片调整时间、电量、播放与提醒的位置",
                "Drag cards to arrange the clock, battery, playback, and alerts",
            ),
            palette,
        );
        self.resident_order_settings(ui, language, palette);
        ui.add_space(12.0);

        self.section_heading(
            ui,
            language.text("展开导航", "Expanded navigation"),
            language.text(
                "点击卡片显示或隐藏页面，拖动卡片调整顺序",
                "Click a card to show or hide it, and drag to change the order",
            ),
            palette,
        );
        self.page_order_settings(ui, language, palette);
        if self
            .settings
            .visible_pages
            .iter()
            .filter(|enabled| **enabled)
            .count()
            == 1
        {
            ui.add_space(5.0);
            ui.label(
                RichText::new(language.text(
                    "至少保留一个页面。隐藏页面不会删除其中的数据。",
                    "At least one page must stay visible. Hidden pages keep their data.",
                ))
                .size(10.0)
                .color(palette.muted),
            );
        }
    }

    fn section_heading(&self, ui: &mut egui::Ui, title: &str, description: &str, palette: Palette) {
        ui.label(
            RichText::new(title)
                .size(12.0)
                .strong()
                .color(palette.text_title),
        );
        ui.label(RichText::new(description).size(10.0).color(palette.muted));
        ui.add_space(7.0);
    }

    fn layout_preview(&self, ui: &mut egui::Ui, language: Language, palette: Palette) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 126.0), Sense::hover());
        ui.painter().rect_filled(rect, 10.0, palette.chrome);
        ui.painter().rect_stroke(
            rect,
            10.0,
            Stroke::new(1.0_f32, palette.line_soft),
            egui::StrokeKind::Inside,
        );

        let screen = Rect::from_min_max(
            egui::pos2(rect.left() + 68.0, rect.top() + 14.0),
            egui::pos2(rect.right() - 14.0, rect.bottom() - 14.0),
        );
        ui.painter()
            .rect_filled(screen, 8.0, Color32::from_rgb(9, 10, 12));
        ui.painter().text(
            egui::pos2(rect.left() + 14.0, screen.top() + 26.0),
            egui::Align2::LEFT_CENTER,
            language.text("收起", "Collapsed"),
            egui::FontId::proportional(10.0),
            palette.muted,
        );
        ui.painter().text(
            egui::pos2(rect.left() + 14.0, screen.top() + 78.0),
            egui::Align2::LEFT_CENTER,
            language.text("展开", "Expanded"),
            egui::FontId::proportional(10.0),
            palette.muted,
        );

        let order = if valid_order(&self.settings.resident_order) {
            self.settings.resident_order.clone()
        } else {
            DEFAULT_ORDER.to_vec()
        };
        let visible_residents: Vec<_> = order
            .iter()
            .copied()
            .filter(|slot| self.resident_preview_visible(*slot))
            .collect();
        let island_w = (38.0 + visible_residents.len() as f32 * 19.0)
            .clamp(112.0, (screen.width() - 42.0).max(112.0));
        let travel = ((screen.width() - island_w) * 0.5).max(0.0);
        let island_cx = screen.center().x + self.settings.island_offset_ratio() * travel;
        let island = Rect::from_center_size(
            egui::pos2(island_cx, screen.top() + 28.0),
            egui::vec2(island_w, 22.0),
        );
        ui.painter().rect_filled(island, 11.0, Color32::BLACK);
        self.paint_resident_preview(ui, island, &visible_residents, palette);

        let pages: Vec<_> = self
            .settings
            .page_order
            .iter()
            .copied()
            .filter(|page| self.settings.visible_pages[*page])
            .collect();
        let panel_w = (screen.width() * 0.76).min(270.0);
        let panel = Rect::from_center_size(
            egui::pos2(screen.center().x, screen.bottom() - 30.0),
            egui::vec2(panel_w, 42.0),
        );
        ui.painter().rect_filled(panel, 12.0, Color32::BLACK);
        let gap = (panel.width() - 30.0) / pages.len().max(1) as f32;
        for (index, page) in pages.iter().copied().enumerate() {
            let center = egui::pos2(
                panel.left() + 15.0 + gap * (index as f32 + 0.5),
                panel.center().y,
            );
            lucide::paint(
                ui.painter(),
                Rect::from_center_size(center, egui::vec2(13.0, 13.0)),
                page_icon(page),
                palette.text_soft,
            );
        }
    }

    fn paint_resident_preview(
        &self,
        ui: &egui::Ui,
        island: Rect,
        slots: &[usize],
        palette: Palette,
    ) {
        let gap = (island.width() - 32.0) / slots.len().max(1) as f32;
        for (index, slot) in slots.iter().copied().enumerate() {
            let center = egui::pos2(
                island.left() + 16.0 + gap * (index as f32 + 0.5),
                island.center().y,
            );
            match slot {
                CLOCK => {
                    ui.painter().text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        "10:08",
                        egui::FontId::proportional(7.5),
                        palette.text_soft,
                    );
                }
                BATTERY => {
                    let body = Rect::from_center_size(center, egui::vec2(12.0, 6.0));
                    ui.painter().rect_stroke(
                        body,
                        1.5,
                        Stroke::new(1.0_f32, palette.text_soft),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().rect_filled(
                        Rect::from_min_size(body.min + egui::vec2(2.0, 2.0), egui::vec2(6.0, 2.0)),
                        0.5,
                        palette.text_soft,
                    );
                }
                _ => lucide::paint(
                    ui.painter(),
                    Rect::from_center_size(center, egui::vec2(11.0, 11.0)),
                    resident_icon(slot),
                    palette.text_soft,
                ),
            }
        }
    }

    fn resident_preview_visible(&self, slot: usize) -> bool {
        match slot {
            BATTERY => self.settings.show_battery,
            PET => self.pet_enabled(),
            CLOCK => true,
            MUSIC => self.media.is_music() || self.video.is_video(),
            BELL => self.t < self.notification_bell_until,
            _ => false,
        }
    }

    fn resident_order_settings(&mut self, ui: &mut egui::Ui, language: Language, palette: Palette) {
        let default = if self.settings.show_battery {
            DEFAULT_ORDER.to_vec()
        } else {
            vec![PET, CLOCK, MUSIC, BELL, BATTERY]
        };
        let mut order = if valid_order(&self.settings.resident_order) {
            self.settings.resident_order.clone()
        } else {
            default
        };

        #[derive(Clone)]
        struct ResidentDrag(usize);

        let gap = 8.0;
        let tile_w = ((ui.available_width() - gap) * 0.5).max(170.0);
        let mut movement = None;
        egui::Grid::new("resident-layout-grid")
            .num_columns(2)
            .spacing(egui::vec2(gap, gap))
            .show(ui, |ui| {
                for (index, slot) in order.clone().into_iter().enumerate() {
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(tile_w, 56.0), Sense::click_and_drag());
                    let hovered = response.hovered();
                    let dragged = response.dragged();
                    ui.painter().rect_filled(
                        rect,
                        8.0,
                        if dragged {
                            palette.surface_selected
                        } else if hovered {
                            palette.surface_hover
                        } else {
                            palette.surface_subtle
                        },
                    );
                    ui.painter().rect_stroke(
                        rect,
                        8.0,
                        Stroke::new(1.0_f32, palette.line_soft),
                        egui::StrokeKind::Inside,
                    );
                    let icon_center = egui::pos2(rect.left() + 23.0, rect.center().y);
                    ui.painter()
                        .circle_filled(icon_center, 14.0, palette.surface);
                    lucide::paint(
                        ui.painter(),
                        Rect::from_center_size(icon_center, egui::vec2(14.0, 14.0)),
                        resident_icon(slot),
                        palette.text_soft,
                    );
                    ui.painter().text(
                        egui::pos2(rect.left() + 46.0, rect.top() + 19.0),
                        egui::Align2::LEFT_CENTER,
                        resident_label(slot, language),
                        egui::FontId::proportional(11.5),
                        palette.text_title,
                    );
                    ui.painter().text(
                        egui::pos2(rect.left() + 46.0, rect.bottom() - 17.0),
                        egui::Align2::LEFT_CENTER,
                        resident_state(self, slot, language),
                        egui::FontId::proportional(9.5),
                        palette.muted,
                    );
                    lucide::paint(
                        ui.painter(),
                        Rect::from_center_size(
                            egui::pos2(rect.right() - 17.0, rect.center().y),
                            egui::vec2(13.0, 13.0),
                        ),
                        Icon::GripVertical,
                        palette.subtle,
                    );
                    let response = response
                        .on_hover_cursor(egui::CursorIcon::Grab)
                        .on_hover_text(language.text("拖动调整位置", "Drag to reorder"));
                    response.dnd_set_drag_payload(ResidentDrag(slot));
                    if let Some(payload) = response.dnd_hover_payload::<ResidentDrag>() {
                        if payload.0 != slot {
                            ui.painter().rect_stroke(
                                rect,
                                8.0,
                                Stroke::new(1.8_f32, palette.accent),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                    if let Some(payload) = response.dnd_release_payload::<ResidentDrag>() {
                        movement = Some((payload.0, slot));
                    }
                    if (index + 1) % 2 == 0 && index + 1 < order.len() {
                        ui.end_row();
                    }
                }
            });
        if let Some((dragged, target)) = movement {
            let from = order.iter().position(|slot| *slot == dragged);
            let to = order.iter().position(|slot| *slot == target);
            if let (Some(from), Some(to)) = (from, to) {
                move_item(&mut order, from, to);
                self.settings.resident_order = order;
            }
        }

        ui.add_space(5.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button(language.text("恢复默认顺序", "Reset order"))
                .clicked()
            {
                self.settings.resident_order.clear();
            }
        });
    }

    fn page_order_settings(&mut self, ui: &mut egui::Ui, language: Language, palette: Palette) {
        #[derive(Clone)]
        struct PageDrag(usize);

        let mut order = self.settings.page_order;
        let mut movement = None;
        let mut visibility_change = None;
        let visible_count = self
            .settings
            .visible_pages
            .iter()
            .filter(|enabled| **enabled)
            .count();
        let gap = 8.0;
        let tile_w = ((ui.available_width() - gap) * 0.5).max(170.0);
        egui::Grid::new("page-layout-grid")
            .num_columns(2)
            .spacing(egui::vec2(gap, gap))
            .show(ui, |ui| {
                for (index, page) in order.into_iter().enumerate() {
                    let visible = self.settings.visible_pages[page];
                    let can_toggle = !visible || visible_count > 1;
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(tile_w, 56.0), Sense::click_and_drag());
                    let hovered = response.hovered();
                    let dragged = response.dragged();
                    ui.painter().rect_filled(
                        rect,
                        8.0,
                        if dragged {
                            palette.surface_selected
                        } else if hovered {
                            palette.surface_hover
                        } else {
                            palette.surface_subtle
                        },
                    );
                    ui.painter().rect_stroke(
                        rect,
                        8.0,
                        Stroke::new(1.0_f32, palette.line_soft),
                        egui::StrokeKind::Inside,
                    );
                    let icon_center = egui::pos2(rect.left() + 23.0, rect.center().y);
                    ui.painter()
                        .circle_filled(icon_center, 14.0, palette.surface);
                    let icon_color = if visible {
                        palette.text_soft
                    } else {
                        palette.subtle
                    };
                    lucide::paint(
                        ui.painter(),
                        Rect::from_center_size(icon_center, egui::vec2(14.0, 14.0)),
                        page_icon(page),
                        icon_color,
                    );
                    ui.painter().text(
                        egui::pos2(rect.left() + 46.0, rect.top() + 19.0),
                        egui::Align2::LEFT_CENTER,
                        page_label(page, language),
                        egui::FontId::proportional(11.5),
                        if visible {
                            palette.text_title
                        } else {
                            palette.muted
                        },
                    );
                    ui.painter().text(
                        egui::pos2(rect.left() + 46.0, rect.bottom() - 17.0),
                        egui::Align2::LEFT_CENTER,
                        if visible {
                            language.text("已显示", "Visible")
                        } else {
                            language.text("已隐藏", "Hidden")
                        },
                        egui::FontId::proportional(9.5),
                        if visible {
                            palette.accent
                        } else {
                            palette.muted
                        },
                    );
                    let track = Rect::from_center_size(
                        egui::pos2(rect.right() - 22.0, rect.center().y),
                        egui::vec2(25.0, 14.0),
                    );
                    ui.painter().rect_filled(
                        track,
                        7.0,
                        if visible {
                            palette.accent
                        } else {
                            palette.toggle_off
                        },
                    );
                    let knob_x = if visible {
                        track.right() - 7.0
                    } else {
                        track.left() + 7.0
                    };
                    ui.painter().circle_filled(
                        egui::pos2(knob_x, track.center().y),
                        5.0,
                        palette.surface,
                    );

                    response.dnd_set_drag_payload(PageDrag(page));
                    if let Some(payload) = response.dnd_hover_payload::<PageDrag>() {
                        if payload.0 != page {
                            ui.painter().rect_stroke(
                                rect,
                                8.0,
                                Stroke::new(1.8_f32, palette.accent),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                    if let Some(payload) = response.dnd_release_payload::<PageDrag>() {
                        movement = Some((payload.0, page));
                    }
                    if response.clicked() && can_toggle {
                        visibility_change = Some((page, !visible));
                    }
                    if (index + 1) % 2 == 0 && index + 1 < order.len() {
                        ui.end_row();
                    }
                }
            });
        if let Some((dragged, target)) = movement {
            let from = order.iter().position(|page| *page == dragged);
            let to = order.iter().position(|page| *page == target);
            if let (Some(from), Some(to)) = (from, to) {
                move_item(&mut order, from, to);
                self.settings.page_order = order;
            }
        }
        if let Some((page, visible)) = visibility_change {
            self.set_page_visible(page, visible);
        }
    }
}

fn resident_icon(slot: usize) -> Icon {
    match slot {
        BATTERY => Icon::Smartphone,
        PET => Icon::Sparkles,
        CLOCK => Icon::Sun,
        MUSIC => Icon::Music,
        BELL => Icon::Bell,
        _ => Icon::LayoutGrid,
    }
}

fn page_icon(page: usize) -> Icon {
    match page {
        page::MUSIC => Icon::Music,
        page::VIDEO => Icon::PlayCircle,
        page::MONITOR => Icon::Activity,
        page::SHELF => Icon::Folder,
        page::IMAGES => Icon::Image,
        page::CLIPBOARD => Icon::Clipboard,
        page::MAIL => Icon::Bell,
        _ => Icon::LayoutGrid,
    }
}

fn page_label(page: usize, language: Language) -> &'static str {
    match page {
        page::MUSIC => language.text("音乐", "Music"),
        page::VIDEO => language.text("视频", "Video"),
        page::MONITOR => language.text("系统监控", "Monitor"),
        page::SHELF => language.text("文件口袋", "Pocket"),
        page::IMAGES => language.text("图片", "Images"),
        page::CLIPBOARD => language.text("剪贴板", "Clipboard"),
        page::MAIL => language.text("邮箱", "Mail"),
        _ => "",
    }
}

fn resident_label(slot: usize, language: Language) -> &'static str {
    match slot {
        BATTERY => language.text("电量", "Battery"),
        PET => language.text("岛屿表情", "Island face"),
        CLOCK => language.text("时间", "Clock"),
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
