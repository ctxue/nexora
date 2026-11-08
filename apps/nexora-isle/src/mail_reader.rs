use crate::app::{App, Cmd};
use eframe::egui::{self, Color32, Vec2};

impl App {
    pub fn mail_reader_ui(&mut self, ctx: &egui::Context) {
        let Some(raw) = self.mail.reading.clone() else {
            return;
        };
        let hidden = self.mail_code_hidden(&raw);
        let item = self.display_mail(&raw);
        let language = self.settings.language;
        let mut reveal = false;
        let focus = std::mem::take(&mut self.mail.reader_focus);
        let (mut close, mut link) = (false, None);
        let window_size = Vec2::new(700.0, 620.0);
        let position = crate::app::centered_decorated_viewport_position(ctx, window_size);
        let mut viewport = egui::ViewportBuilder::default()
            .with_title(language.text("邮件全文 · NEXORA ISLE", "Mail · NEXORA ISLE"))
            .with_inner_size(window_size)
            .with_min_inner_size([500.0, 380.0]);
        if let Some(position) = position {
            viewport = viewport.with_position(position);
        }
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("nexora-isle-mail-reader"),
            viewport,
            |ctx, _| {
                if focus {
                    if let Some(position) = position {
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                close = ctx.input(|i| i.viewport().close_requested());
                if raw.code.is_some() {
                    ctx.request_repaint_after(std::time::Duration::from_secs(1));
                }
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(Color32::from_rgb(25, 25, 27))
                            .inner_margin(22),
                    )
                    .show(ctx, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&item.subject)
                                    .size(20.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .wrap(),
                        );
                        egui::Grid::new("mail-reader-meta")
                            .spacing([12.0, 5.0])
                            .show(ui, |ui| {
                                for (label, text) in [
                                    (language.text("发件人", "From"), &item.from),
                                    (language.text("收件邮箱", "Mailbox"), &item.account),
                                    (language.text("时间", "Time"), &item.time),
                                ] {
                                    ui.label(
                                        egui::RichText::new(label).color(Color32::from_gray(160)),
                                    );
                                    ui.label(
                                        egui::RichText::new(text).color(Color32::from_gray(220)),
                                    );
                                    ui.end_row();
                                }
                            });
                        if hidden {
                            ui.label(language.text(
                                "验证码已隐藏，不代表验证码失效；相关链接也暂时隐藏。",
                                "The verification code is hidden and may still be valid. Related links are hidden too.",
                            ));
                            reveal = ui
                                .button(language.text(
                                    "主动查看验证码与正文 15 秒",
                                    "Show the code and message for 15 seconds",
                                ))
                                .clicked();
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new(language.text(
                                "鼠标滚轮或拖动右侧滚动条查看全文",
                                "Use the mouse wheel or scrollbar to read the message",
                            ))
                                .size(11.5)
                                .color(Color32::from_gray(160)),
                        );
                        ui.style_mut().spacing.scroll.floating = false;
                        egui::ScrollArea::vertical()
                            .id_salt(("mail-reader-body", &item.account, item.uid))
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(if item.summary.is_empty() {
                                            language.text(
                                                "（这封邮件没有正文）",
                                                "(This message has no body)",
                                            )
                                        } else {
                                            item.summary.as_ref()
                                        })
                                        .size(14.0)
                                        .color(Color32::from_gray(235)),
                                    )
                                    .selectable(true)
                                    .wrap(),
                                );
                                if !item.links.is_empty() {
                                    ui.add_space(12.0);
                                    ui.separator();
                                    ui.label(
                                        egui::RichText::new(language.text(
                                            "邮件中的链接",
                                            "Links in this message",
                                        ))
                                        .strong(),
                                    );
                                    for target in &item.links {
                                        if ui
                                            .link(&target.label)
                                            .on_hover_text(&target.url)
                                            .clicked()
                                        {
                                            link = Some(target.url.clone());
                                        }
                                    }
                                }
                            });
                    });
            },
        );
        if reveal {
            self.reveal_mail_code(&raw);
            ctx.request_repaint();
        }
        if close {
            self.mail.reading = None;
        }
        if let Some(url) = link {
            self.pending.push(Cmd::OpenUrl(url));
            ctx.request_repaint();
        }
    }
}
