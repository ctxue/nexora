//! 设置、通知队列与NEXORA ISLE表情。
use crate::app::{App, Cmd, HUD_TRACK_W, HUD_TRACK_X, HUD_W, PAGE_NAMES};
use crate::lucide::{self, Icon};
use crate::paint::{Card, H};
use crate::settings_theme::{
    palette as settings_palette, palette_for as settings_palette_for, Palette,
};
use crate::theme::{self, W};
use eframe::egui::{self, Color32, Rect, Vec2};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AlertSource {
    Internal,
    Wechat,
    External(String),
}

#[derive(Clone, Debug)]
pub(crate) struct Alert {
    pub title: String,
    pub source: AlertSource,
    pub body: String,
    pub page: usize,
    pub at: f64,
    pub duration: f64,
    pub bluetooth: Option<(crate::bt::DeviceKind, bool)>,
}
impl Alert {
    pub(crate) fn messenger(&self) -> Option<crate::app::Messenger> {
        match &self.source {
            AlertSource::Wechat => Some(crate::app::Messenger::Wechat),
            AlertSource::External(id) if id == "QQ" => Some(crate::app::Messenger::Qq),
            _ => None,
        }
    }
    pub(crate) fn is_hud(&self) -> bool {
        self.source == AlertSource::Internal
            && matches!(
                self.title.as_str(),
                "系统音量"
                    | "System volume"
                    | "大写锁定"
                    | "Caps Lock"
                    | "键盘输入语言"
                    | "Keyboard input language"
                    | "网络连接"
                    | "Network"
                    | "电源已连接"
                    | "Power connected"
                    | "已切换电池供电"
                    | "Running on battery"
                    | "蓝牙设备"
            )
    }
}
#[cfg(test)]
fn bluetooth_notice_content(body: &str) -> (&str, &str, Option<u8>) {
    bluetooth_notice_content_for(body, crate::i18n::Language::Zh)
}
fn bluetooth_notice_content_for<'a>(
    body: &'a str,
    language: crate::i18n::Language,
) -> (&'a str, &'static str, Option<u8>) {
    if let Some(name) = body.strip_suffix(" · 已断开") {
        return (name, language.text("已断开", "Disconnected"), None);
    }
    if body == "已断开连接" {
        return (
            language.text("蓝牙设备", "Bluetooth device"),
            language.text("已断开", "Disconnected"),
            None,
        );
    }
    if let Some((name, level)) = body.rsplit_once(" · 电量 ") {
        if let Some(level) = level
            .strip_suffix('%')
            .and_then(|s| s.parse::<u8>().ok())
            .filter(|v| *v <= 100)
        {
            return (name, language.text("已连接", "Connected"), Some(level));
        }
    }
    (body, language.text("已连接", "Connected"), None)
}

pub(crate) fn device_icon(c: &Card, kind: crate::bt::DeviceKind, x: f32, y: f32, size: f32) {
    let icon = match kind {
        crate::bt::DeviceKind::Bone
        | crate::bt::DeviceKind::Earbuds
        | crate::bt::DeviceKind::Headphones => Icon::Headphones,
        crate::bt::DeviceKind::Speaker => Icon::Speaker,
        crate::bt::DeviceKind::Keyboard => Icon::Keyboard,
        crate::bt::DeviceKind::Mouse => Icon::Mouse,
        crate::bt::DeviceKind::Gamepad => Icon::Gamepad,
        crate::bt::DeviceKind::Phone => Icon::Smartphone,
        crate::bt::DeviceKind::Other => Icon::Bluetooth,
    };
    lucide::paint(&c.painter, c.rect(x, y, size, size), icon, Color32::WHITE);
}

pub(crate) use crate::pet::PetState;

pub(crate) fn notice_badge(c: &Card, x: f32, y: f32, a: &Alert) {
    if a.source == AlertSource::Internal && a.title == "蓝牙设备" {
        c.circle(
            x,
            y,
            11.0,
            if a.body.contains("已断开") {
                Color32::from_gray(95)
            } else {
                theme::BLUE
            },
        );
        device_icon(
            c,
            a.bluetooth.map_or(crate::bt::DeviceKind::Other, |v| v.0),
            x - 7.5,
            y - 7.5,
            15.0,
        );
    } else {
        notice_icon(&c.scaled(x, y, 0.85), x, y, a);
    }
}
pub(crate) fn messenger_icon(c: &Card, x: f32, y: f32, source: crate::app::Messenger) {
    let ink = Color32::from_gray(24);
    if source == crate::app::Messenger::Wechat {
        let back = Color32::from_rgb(70, 203, 123);
        let front = Color32::from_rgb(145, 235, 165);
        c.fill(x - 10., y - 7., 14., 11., 5.5, back);
        c.poly(
            &[(x - 8., y + 1.), (x - 8., y + 6.), (x - 3., y + 2.)],
            back,
        );
        for dx in [-6., -1.] {
            c.circle(x + dx, y - 3., 0.9, ink);
        }
        c.fill(x - 2., y - 2., 13., 10., 5., front);
        c.poly(
            &[(x + 5., y + 6.), (x + 9., y + 10.), (x + 8., y + 5.)],
            front,
        );
        for dx in [2., 7.] {
            c.circle(x + dx, y + 1., 0.8, ink);
        }
    } else {
        let rect = c.rect(x - 9.5, y - 9.5, 19.0, 19.0);
        let icon = egui::Image::new(egui::include_image!("../assets/qq-client.png"));
        if let Ok(egui::load::TexturePoll::Ready { texture }) =
            icon.load_for_size(c.ui.ctx(), rect.size())
        {
            c.painter.image(
                texture.id,
                rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }
}
fn notice_icon(c: &Card, x: f32, y: f32, a: &Alert) {
    if a.source != AlertSource::Internal {
        crate::paint::bell(c, x, y, theme::WHITE, 0.0);
        return;
    }
    match a.title.as_str() {
        "系统音量" | "System volume" => {
            c.poly(
                &[
                    (x - 9.0, y - 3.0),
                    (x - 5.0, y - 3.0),
                    (x, y - 7.0),
                    (x, y + 7.0),
                    (x - 5.0, y + 3.0),
                    (x - 9.0, y + 3.0),
                ],
                theme::WHITE,
            );
            c.arc(x, y, 5.0, -0.8, 0.8, 1.5, theme::WHITE);
            c.arc(x, y, 8.0, -0.7, 0.7, 1.3, theme::WHITE);
        }
        "大写锁定" | "Caps Lock" => {
            let ink = if a.body == "已开启" || a.body == "On" {
                theme::GREEN
            } else {
                theme::GRAY
            };
            c.poly(&[(x - 8.0, y - 1.0), (x, y - 9.0), (x + 8.0, y - 1.0)], ink);
            c.fill(x - 3.0, y - 1.5, 6.0, 6.5, 0.5, ink);
            c.fill(x - 4.0, y + 7.0, 8.0, 2.0, 0.8, ink);
        }
        "键盘输入语言" | "Keyboard input language" => {
            c.stroke(x - 10.0, y - 8.5, 20.0, 17.0, 4.0, 1.2, theme::GRAY);
            let label = match a.body.as_str() {
                "中文" => "中",
                "英语" => "EN",
                _ => "Aa",
            };
            c.centered_text(
                x,
                y,
                label,
                if label == "EN" { 9.0 } else { 11.0 },
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
        }
        "网络连接" | "Network" => {
            let connected = a.body.contains("已连接") || a.body.contains("connected");
            let ink = if connected { theme::GREEN } else { theme::GRAY };
            if a.body.starts_with("Wi-Fi") {
                for r in [4.0, 8.0, 12.0] {
                    c.arc(x, y + 6.0, r, -2.35, -0.79, 1.7, ink);
                }
                c.circle(x, y + 6.0, 1.6, ink);
            } else if a.body.starts_with("以太网") || a.body.starts_with("Ethernet") {
                c.stroke(x - 8.0, y - 8.0, 16.0, 14.0, 2.5, 1.6, ink);
                for dx in [-4.0, -1.3, 1.3, 4.0] {
                    c.seg((x + dx, y - 5.0), (x + dx, y - 1.0), 1.2, ink);
                }
                c.path(
                    &[
                        (x - 4.0, y + 6.0),
                        (x - 4.0, y + 9.0),
                        (x + 4.0, y + 9.0),
                        (x + 4.0, y + 6.0),
                    ],
                    1.6,
                    ink,
                );
            } else {
                c.ring(x, y, 8.0, 1.4, ink);
                c.seg((x - 8.0, y), (x + 8.0, y), 1.0, ink);
                c.stroke(x - 3.5, y - 8.0, 7.0, 16.0, 3.5, 1.0, ink);
            }
            if !connected {
                c.seg((x - 8.0, y - 8.0), (x + 9.0, y + 9.0), 2.0, theme::WHITE);
            }
        }
        "电源已连接"
        | "Power connected"
        | "已切换电池供电"
        | "Running on battery"
        | "电量偏低"
        | "Low battery" => {
            let low = a.title == "电量偏低" || a.title == "Low battery";
            let ink = if low {
                theme::RED_TOP
            } else if a.title == "电源已连接" || a.title == "Power connected" {
                theme::GREEN
            } else {
                theme::WHITE
            };
            c.stroke(x - 11.0, y - 6.0, 20.0, 12.0, 3.0, 1.5, ink);
            c.fill(x + 10.0, y - 2.5, 2.0, 5.0, 0.8, ink);
            if low {
                c.fill(x - 8.0, y - 3.5, 3.5, 7.0, 1.0, ink);
            } else if a.title == "电源已连接" || a.title == "Power connected" {
                c.poly(&[(x, y - 4.5), (x - 4.0, y + 0.8), (x + 0.5, y + 0.8)], ink);
                c.poly(
                    &[(x + 0.5, y - 0.8), (x + 4.0, y - 0.8), (x - 1.0, y + 4.5)],
                    ink,
                );
            } else {
                c.fill(x - 8.0, y - 3.5, 12.0, 7.0, 1.0, ink);
            }
        }
        _ => crate::paint::bell(&c.scaled(x, y, 1.7), x, y, theme::GRAY, 0.0),
    }
}

impl App {
    pub(crate) fn pet_state(&self) -> PetState {
        self.pet.reaction.map_or(PetState::Idle, |(state, _)| state)
    }
    pub fn alert(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        page: usize,
        system: bool,
    ) {
        let title = title.into();
        let transient = matches!(
            title.as_str(),
            "系统音量"
                | "System volume"
                | "大写锁定"
                | "Caps Lock"
                | "键盘输入语言"
                | "Keyboard input language"
                | "网络连接"
                | "Network"
                | "电源已连接"
                | "Power connected"
                | "已切换电池供电"
                | "Running on battery"
                | "蓝牙设备"
        );
        let a = Alert {
            source: AlertSource::Internal,
            title,
            body: body.into(),
            page,
            at: self.t,
            duration: if transient { 2.5 } else { 8.0 },
            bluetooth: None,
        };
        self.enqueue_alert(a, system);
    }
    pub(crate) fn bluetooth_alert(&mut self, device: &crate::bt::Device, connected: bool) {
        if !self.settings.bluetooth_notifications {
            return;
        }
        if connected
            && matches!(
                device.kind,
                crate::bt::DeviceKind::Bone
                    | crate::bt::DeviceKind::Earbuds
                    | crate::bt::DeviceKind::Headphones
                    | crate::bt::DeviceKind::Speaker
            )
        {
            self.alerts
                .retain(|a| a.title != "系统音量" && a.title != "System volume");
            if self
                .notice
                .as_ref()
                .is_some_and(|a| a.title == "系统音量" || a.title == "System volume")
            {
                self.notice = None;
            }
        }
        let a = Alert {
            source: AlertSource::Internal,
            title: "蓝牙设备".into(),
            body: if connected {
                device.summary()
            } else {
                format!("{} · 已断开", device.name)
            },
            page: 5,
            at: self.t,
            duration: 2.5,
            bluetooth: Some((device.kind, connected)),
        };
        self.enqueue_alert(a, false);
    }
    fn enqueue_alert(&mut self, a: Alert, system: bool) {
        // 通知仅保存在本次运行的内存中。
        if a.source == AlertSource::Internal {
            self.history.push_front(a.clone());
        }
        self.history.truncate(50);
        if self.settings.notifications {
            if system && a.source == AlertSource::Internal {
                self.pending
                    .push(Cmd::Notify(a.title.clone(), a.body.clone()));
            }
            let same_event = |old: &Alert| {
                old.source == a.source && old.title == a.title && (a.is_hud() || old.body == a.body)
            };
            if a.source != AlertSource::Internal || a.title != "蓝牙设备" {
                self.alerts.retain(|old| !same_event(old));
            }
            if (a.source != AlertSource::Internal || a.title != "蓝牙设备")
                && self.notice.as_ref().is_some_and(same_event)
            {
                self.notice = Some(a);
                return;
            }
            self.alerts.push_back(a);
            while self.alerts.len() > 8 {
                self.alerts.pop_front();
            }
        }
    }
    /// 该来源的所有账号都已查看。
    pub fn viewed_messenger(&mut self, source: crate::app::Messenger) {
        self.pending_messengers.retain(|u| u.source != source);
        self.clear_messenger_notices(source);
    }
    /// 某个账号（进程）的窗口被打开：只清它自己；分不出账号的条目（pid 0）一并清掉。
    pub fn viewed_client(&mut self, source: crate::app::Messenger, pid: u32) {
        self.pending_messengers
            .retain(|u| u.source != source || pid != 0 && u.pid != 0 && u.pid != pid);
        if !self.pending_messengers.iter().any(|u| u.source == source) {
            self.clear_messenger_notices(source);
        }
    }
    fn clear_messenger_notices(&mut self, source: crate::app::Messenger) {
        self.alerts.retain(|a| a.messenger() != Some(source));
        if self
            .notice
            .as_ref()
            .is_some_and(|a| a.messenger() == Some(source))
        {
            self.notice = None;
            self.notification_bell_until = 0.0;
        }
    }
    fn mark_messenger(&mut self, source: crate::app::Messenger, pid: u32, window: isize) {
        if let Some(u) = self
            .pending_messengers
            .iter_mut()
            .find(|u| u.source == source && u.pid == pid)
        {
            if window != 0 {
                u.window = window;
            }
            return;
        }
        self.pending_messengers.push(crate::app::Unread {
            source,
            pid,
            window,
        });
    }
    /// 横幅「查看」：跳到该来源最近一条未读的窗口。
    pub(crate) fn latest_unread(&self, source: crate::app::Messenger) -> crate::app::Unread {
        self.pending_messengers
            .iter()
            .rev()
            .find(|u| u.source == source)
            .copied()
            .unwrap_or(crate::app::Unread::of(source))
    }
    pub fn receive_wechat_reminder(&mut self) {
        self.receive_wechat_reminder_from(0, 0);
    }
    /// 微信窗口闪动：`pid` 是该账号的进程，`window` 是闪动的窗口。
    pub fn receive_wechat_reminder_from(&mut self, pid: u32, window: isize) {
        if !self.settings.wechat_notifications || !self.settings.notifications {
            return;
        }
        self.mark_messenger(crate::app::Messenger::Wechat, pid, window);
        self.notification_bell_at = self.t;
        self.notification_bell_until = self.t + 4.5;
        let language = self.settings.language;
        self.enqueue_alert(
            Alert {
                source: AlertSource::Wechat,
                title: language.text("微信", "WeChat").into(),
                body: language
                    .text(
                        "有新消息 · 请到微信查看",
                        "New message · Open WeChat to view",
                    )
                    .into(),
                page: usize::MAX,
                at: self.t,
                duration: 4.5,
                bluetooth: None,
            },
            false,
        );
    }
    pub fn receive_notification(&mut self, app: String, id: String, body: String) {
        if !self.settings.read_notifications || id.is_empty() {
            return;
        }
        // NEXORA ISLE 自己发出的系统提醒会被通知中心回传；已在历史里的同一内容不再入队，避免重复显示。
        if self
            .history
            .iter()
            .any(|a| format!("{} · {}", a.title, a.body) == body)
        {
            return;
        }
        if !self.notification_apps.observe(&id, &app) || !self.settings.notifications {
            return;
        }
        if id == "QQ" {
            self.mark_messenger(crate::app::Messenger::Qq, 0, 0);
        }
        self.notification_bell_at = self.t;
        self.notification_bell_until = self.t + 4.5;
        let title = app.chars().filter(|c| !c.is_control()).take(80).collect();
        let body = if self.settings.notification_preview {
            let mut text = body
                .chars()
                .filter(|c| !c.is_control())
                .take(300)
                .collect::<String>();
            if let Some(code) = crate::otp::extract("", &text, &[]) {
                text = crate::mail_privacy::mask(&text, &code.text);
            }
            text
        } else {
            self.settings
                .language
                .text(
                    "收到新通知 · 内容已隐藏",
                    "New notification · Preview hidden",
                )
                .into()
        };
        self.enqueue_alert(
            Alert {
                source: AlertSource::External(id),
                title,
                body,
                page: usize::MAX,
                at: self.t,
                duration: 4.5,
                bluetooth: None,
            },
            false,
        );
    }
    pub(crate) fn tick_alerts(&mut self) {
        self.pending_messengers.retain(|u| {
            self.settings.notifications
                && match u.source {
                    crate::app::Messenger::Wechat => self.settings.wechat_notifications,
                    crate::app::Messenger::Qq => {
                        self.settings.read_notifications
                            && self
                                .notification_apps
                                .items
                                .iter()
                                .any(|(key, _, on)| key == "QQ" && *on)
                    }
                }
        });
        let allowed = |a: &Alert| match &a.source {
            AlertSource::Internal => true,
            AlertSource::Wechat => {
                self.settings.wechat_notifications && self.settings.notifications
            }
            AlertSource::External(id) => {
                self.settings.read_notifications
                    && self.settings.notifications
                    && self
                        .notification_apps
                        .items
                        .iter()
                        .any(|(key, _, on)| key == id && *on)
            }
        };
        self.alerts.retain(&allowed);
        if self.notice.as_ref().is_some_and(|a| !allowed(a)) {
            self.notice = None;
        }
        if self.hud_notice.as_ref().is_some_and(|a| !allowed(a)) {
            self.hud_notice = None;
        }
        if !self.settings.notification_preview {
            for a in self
                .alerts
                .iter_mut()
                .chain(self.notice.iter_mut())
                .chain(self.hud_notice.iter_mut())
            {
                if matches!(&a.source, AlertSource::External(_)) {
                    a.body = self
                        .settings
                        .language
                        .text(
                            "收到新通知 · 内容已隐藏",
                            "New notification · Preview hidden",
                        )
                        .into();
                }
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|a| self.t - a.at > a.duration)
        {
            self.notice = None;
        }
        if self.notice.is_none() {
            if let Some(mut a) = self.alerts.pop_front() {
                a.at = self.t;
                self.notice = Some(a);
            }
        }
    }
    pub(crate) fn draw_notice(&mut self, c: &Card, panel: Rect, a: &Alert, _cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        let nc = c.clipped(0.0, 0.0, panel.width(), panel.height());
        if let Some(source) = a.messenger() {
            messenger_icon(&nc, 22.0, 22.0, source);
            nc.circle(31.0, 14.0, 2.6, Color32::from_rgb(255, 72, 72));
            nc.centered_text(
                44.0,
                14.0,
                &a.title,
                11.0,
                W::Semibold,
                theme::WHITE,
                H::Left,
            );
            nc.centered_text(
                44.0,
                30.0,
                &nc.fit(&a.body, 10.5, W::Regular, 154.0),
                10.5,
                W::Regular,
                theme::GRAY,
                H::Left,
            );
            if notice_button(
                &nc,
                206.0,
                12.0,
                52.0,
                language.text("查看", "View"),
                "messenger-view",
            ) {
                _cmds.push(Cmd::ViewMessenger(self.latest_unread(source)));
            }
            if notice_button(
                &nc,
                265.0,
                12.0,
                52.0,
                language.text("关闭", "Close"),
                "messenger-dismiss",
            ) {
                self.notice = None;
                self.notification_bell_until = 0.0;
            }
            return;
        }
        if a.source != AlertSource::Internal {
            nc.centered_text(
                16.0,
                14.0,
                &nc.fit(&a.title, 11.0, W::Semibold, 248.0),
                11.0,
                W::Semibold,
                theme::WHITE,
                H::Left,
            );
            nc.centered_text(
                16.0,
                30.0,
                &nc.fit(&a.body, 10.5, W::Regular, 248.0),
                10.5,
                W::Regular,
                theme::GRAY,
                H::Left,
            );
            if notice_button(
                &nc,
                272.0,
                12.0,
                46.0,
                language.text("关闭", "Close"),
                "external-dismiss",
            ) {
                self.notice = None;
                self.notification_bell_until = 0.0;
            }
            return;
        }
        if a.is_hud() {
            if a.title == "蓝牙设备" {
                let (name, status, battery) = bluetooth_notice_content_for(&a.body, language);
                nc.circle(
                    26.0,
                    24.0,
                    14.0,
                    if a.bluetooth.is_none_or(|(_, connected)| connected) {
                        Color32::from_rgb(10, 132, 255)
                    } else {
                        Color32::from_gray(95)
                    },
                );
                let kind = a
                    .bluetooth
                    .map(|(kind, _)| kind)
                    .unwrap_or(crate::bt::DeviceKind::Other);
                device_icon(&nc, kind, 17.0, 15.0, 18.0);
                let right = HUD_W - 14.0;
                let name_end = if let Some(level) = battery {
                    let pct = format!("{level}%");
                    let pct_width = nc.text_w(&pct, 10.5, W::Semibold);
                    let x = right - pct_width - 22.0;
                    nc.stroke(x, 20.0, 15.0, 8.0, 2.0, 1.0, Color32::from_gray(175));
                    nc.fill(x + 15.5, 22.5, 2.0, 3.0, 0.8, Color32::from_gray(175));
                    let fill = 11.0 * level as f32 / 100.0;
                    if fill > 0.0 {
                        nc.fill(
                            x + 2.0,
                            22.0,
                            fill,
                            4.0,
                            1.0,
                            if level <= 20 {
                                theme::RED
                            } else {
                                theme::GREEN
                            },
                        );
                    }
                    nc.centered_text(right, 24.0, &pct, 10.5, W::Semibold, theme::WHITE, H::Right);
                    x - 8.0
                } else {
                    right
                };
                nc.centered_text(
                    49.0,
                    16.0,
                    &nc.fit(name, 11.0, W::Semibold, (name_end - 49.0).max(20.0)),
                    11.0,
                    W::Semibold,
                    theme::WHITE,
                    H::Left,
                );
                nc.centered_text(49.0, 32.0, status, 10.5, W::Regular, theme::GRAY, H::Left);
                return;
            }
            match a.title.as_str() {
                "系统音量" | "System volume" => {
                    nc.poly(
                        &[
                            (17.0, 16.0),
                            (21.0, 16.0),
                            (26.0, 12.0),
                            (26.0, 26.0),
                            (21.0, 22.0),
                            (17.0, 22.0),
                        ],
                        theme::WHITE,
                    );
                    if self.volume_ui.is_some_and(|(_, muted)| muted) {
                        nc.seg((29.0, 16.0), (34.0, 22.0), 1.4, theme::WHITE);
                        nc.seg((34.0, 16.0), (29.0, 22.0), 1.4, theme::WHITE);
                    } else {
                        nc.arc(26.0, 19.0, 5.0, -0.8, 0.8, 1.5, theme::WHITE);
                        nc.arc(26.0, 19.0, 8.0, -0.7, 0.7, 1.3, theme::WHITE);
                    }
                }
                _ => notice_icon(&nc, 24.0, 19.0, a),
            }
            if a.title == "系统音量" || a.title == "System volume" {
                self.draw_volume_hud(&nc);
            } else {
                let (title, body) = if a.title == "键盘输入语言"
                    || a.title == "Keyboard input language"
                {
                    (language.text("输入语言", "Input language"), a.body.as_str())
                } else if a.title == "已切换电池供电" || a.title == "Running on battery" {
                    (language.text("电池供电", "Battery"), a.body.as_str())
                } else if a.title == "网络连接" || a.title == "Network" {
                    a.body
                        .split_once(" · ")
                        .unwrap_or((language.text("网络", "Network"), a.body.as_str()))
                } else {
                    (a.title.as_str(), a.body.as_str())
                };
                nc.centered_text(
                    48.0,
                    19.0,
                    &nc.fit(title, 11.0, W::Semibold, 90.0),
                    11.0,
                    W::Semibold,
                    theme::WHITE,
                    H::Left,
                );
                nc.centered_text(
                    HUD_W - 16.0,
                    19.0,
                    &nc.fit(body, 10.5, W::Regular, 92.0),
                    10.5,
                    W::Regular,
                    if (a.title == "大写锁定" || a.title == "Caps Lock")
                        && (a.body == "已开启" || a.body == "On")
                        || (a.title == "网络连接" || a.title == "Network")
                            && (a.body.contains("已连接") || a.body.contains("connected"))
                    {
                        theme::GREEN
                    } else {
                        theme::GRAY
                    },
                    H::Right,
                );
            }
        } else {
            let is_power = a.title == "电量偏低" || a.title == "Low battery";
            notice_icon(&nc, 22.0, 22.0, a);
            let max = 195.0;
            nc.centered_text(
                43.0,
                14.0,
                &nc.fit(&a.title, 11.0, W::Semibold, max),
                11.0,
                W::Semibold,
                theme::WHITE,
                H::Left,
            );
            nc.centered_text(
                43.0,
                30.0,
                &nc.fit(&a.body, 10.5, W::Regular, max),
                10.5,
                W::Regular,
                theme::GRAY,
                H::Left,
            );
            let label = if is_power {
                language.text("知道了", "Got it")
            } else {
                language.text("查看", "View")
            };
            if notice_button(&nc, 257.0, 12.0, 60.0, label, "notice-action") {
                self.notice = None;
                if is_power {
                    return;
                }
                if a.page < crate::model::page::COUNT {
                    self.show_page(a.page);
                } else {
                    self.settings_tab = 7;
                    self.open_settings();
                }
            }
        }
    }
    /// 可交互音量条：拖动或滚轮改音量，点击喇叭静音。
    pub(crate) fn draw_volume_hud(&mut self, nc: &Card) {
        let (v, muted) = self.volume_ui.unwrap_or((0, false));
        let v = v.min(100);
        // 喇叭：点击静音 / 恢复
        let icon = nc.hit(12.0, 9.0, 28.0, 22.0, "volume-mute");
        if icon.clicked() && nc.live {
            self.volume_ui = Some((v, !muted));
            self.volume_quiet_until = self.t + 0.9;
            self.pending.push(Cmd::ToggleMute);
        }
        // 滑条：拖动 / 点击定位；滚轮微调
        let track = nc.hit_drag(HUD_TRACK_X, 8.0, HUD_TRACK_W, 24.0, "volume-track");
        let mut want: Option<u32> = None;
        if track.dragged() || track.clicked() {
            if let Some(p) = track.interact_pointer_pos() {
                let x = nc.to_local(p).x;
                want = Some(
                    (((x - HUD_TRACK_X) / HUD_TRACK_W) * 100.0)
                        .clamp(0.0, 100.0)
                        .round() as u32,
                );
            }
        } else if nc.live && track.hovered() {
            let dy = nc.ui.ctx().input(|i| i.raw_scroll_delta.y);
            if dy.abs() > 0.5 {
                want = Some(((v as f32 + (dy / 120.0) * 4.0).clamp(0.0, 100.0)).round() as u32);
            }
        }
        if let Some(nv) = want {
            if self.volume_ui.map(|(o, _)| o) != Some(nv) || muted {
                self.volume_ui = Some((nv, false));
                self.volume_quiet_until = self.t + 0.9;
                self.pending.push(Cmd::SetVolume(nv));
            }
        }
        let (v, muted) = self.volume_ui.unwrap_or((v, muted));
        let pressed = track.dragged()
            || track.is_pointer_button_down_on()
            || icon.is_pointer_button_down_on();
        let target = if muted { 0.0 } else { v as f32 / 100.0 };
        self.volume_a = crate::anim::approach(
            self.volume_a,
            target,
            if pressed { 34.0 } else { 13.0 },
            self.dt,
        );
        let h = if pressed { 7.0 } else { 5.0 };
        let y = 19.0 - h / 2.0;
        nc.fill(
            HUD_TRACK_X,
            y,
            HUD_TRACK_W,
            h,
            h / 2.0,
            Color32::from_white_alpha(if pressed { 46 } else { 34 }),
        );
        if self.volume_a > 0.0 {
            let w = (HUD_TRACK_W * self.volume_a).max(h);
            let col = if muted {
                Color32::from_gray(150)
            } else {
                theme::WHITE
            };
            nc.fill(HUD_TRACK_X, y, w, h, h / 2.0, col);
            nc.fill(
                HUD_TRACK_X,
                y + 0.8,
                (w - 1.2).max(0.0),
                h * 0.4,
                h * 0.2,
                Color32::from_white_alpha(64),
            );
            if pressed {
                nc.circle(HUD_TRACK_X + w - h / 2.0, y + h / 2.0, h * 0.95, col);
            }
        }
        let label = if muted {
            "静音".to_string()
        } else {
            format!("{v}%")
        };
        nc.centered_text(
            HUD_W - 26.0,
            19.0,
            &label,
            10.5,
            if pressed { W::Semibold } else { W::Regular },
            if muted { theme::GRAY } else { theme::WHITE },
            H::Center,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn island_size_drag_commits_only_on_release() {
        let ctx = egui::Context::default();
        let mut value = 100;
        let pos = egui::pos2(30.0, 30.0);
        let end = egui::pos2(110.0, 30.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let frames = [
            vec![],
            vec![egui::Event::PointerMoved(pos)],
            vec![button(pos, true)],
            vec![egui::Event::PointerMoved(end)],
            vec![button(end, false)],
        ];
        for (i, events) in frames.into_iter().enumerate() {
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(320.0, 80.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().inner_margin(20))
                        .show(ctx, |ui| {
                            ui.spacing_mut().slider_width = 180.0;
                            island_size_slider(ui, &mut value);
                        });
                },
            );
            if i < 4 {
                assert_eq!(value, 100);
            } else {
                assert_ne!(value, 100);
            }
        }
    }
    #[test]
    fn number_stepper_clicks_follow_steps_and_limits() {
        for (initial, step, x, expected) in [
            (5u32, 1, 127.0, 6),
            (5, 1, 13.0, 4),
            (0, 1, 13.0, 0),
            (10, 1, 127.0, 10),
            (9, 3, 127.0, 10),
            (1, 3, 13.0, 0),
        ] {
            let ctx = egui::Context::default();
            let mut value = initial;
            let pos = egui::pos2(x + 20.0, 34.0);
            let events = [
                vec![],
                vec![egui::Event::PointerMoved(pos)],
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                }],
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                }],
            ];
            for events in events {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(200.0, 80.0),
                    )),
                    events,
                    ..Default::default()
                };
                let _ = ctx.run(raw, |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().inner_margin(20))
                        .show(ctx, |ui| {
                            ui.horizontal(|ui| {
                                number_stepper(ui, &mut value, 0, 10, "", step, 76.0);
                            });
                        });
                });
            }
            assert_eq!(value, expected, "初值 {initial}，步长 {step}，点击位置 {x}");
        }
    }

    #[test]
    fn number_stepper_fits_with_settings_button_padding() {
        let ctx = egui::Context::default();
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(340.0, 240.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new())
                .show(ctx, |ui| {
                    ui.spacing_mut().button_padding = egui::vec2(10.0, 6.0);
                    for mut value in [0u32, 25, 180] {
                        let row = ui.horizontal(|ui| {
                            number_stepper(ui, &mut value, 0, 180, " 分钟", 1, 76.0);
                        });
                        assert!(
                            row.response.rect.width() <= 140.1,
                            "数值控件超出预留宽度：{}",
                            row.response.rect.width()
                        );
                    }
                    for mut value in [25u32, 5, 15] {
                        setting_number(ui, "计时时长", &mut value, 1, 180, " 分钟", 1.0);
                    }
                    assert!(ui.min_rect().right() <= 340.1, "连续数值行不应撑宽卡片");
                });
        });
    }

    #[test]
    fn settings_groups_keep_every_page_reachable() {
        let groups = [
            "常规",
            "音乐与歌词",
            "",
            "",
            "",
            "文件口袋",
            "剪贴板",
            "通知与邮箱",
            "通知与邮箱",
            "关于",
            "页面管理",
            "NOMI",
            "系统监控",
        ];
        for (tab, expected) in groups.iter().enumerate() {
            if expected.is_empty() {
                continue;
            }
            let matches: Vec<_> = SETTINGS_GROUPS
                .iter()
                .filter(|(_, start, end)| (*start..=*end).contains(&tab))
                .collect();
            assert_eq!(matches.len(), 1, "页面 {tab} 必须且只能属于一个分组");
            assert_eq!(matches[0].0, *expected);
        }
    }

    #[test]
    fn mailbox_form_rejects_mismatched_provider_and_blank_password() {
        use crate::app::MailForm;

        let mut form = MailForm {
            email: "person@gmail.com".into(),
            password: " app password ".into(),
            provider: crate::mail::Provider::Gmail,
            ..Default::default()
        };
        let (_, password) =
            mail_form_account(&form, crate::i18n::Language::En).expect("valid mailbox form");
        assert_eq!(password, "app password");

        form.provider = crate::mail::Provider::Qq;
        assert!(mail_form_account(&form, crate::i18n::Language::En)
            .expect_err("provider mismatch must be rejected")
            .contains("Gmail"));

        form.provider = crate::mail::Provider::Gmail;
        form.password = "   ".into();
        assert!(mail_form_account(&form, crate::i18n::Language::En).is_err());

        form.password = "app password".into();
        form.email = "https://mail.163.com/".into();
        form.provider = crate::mail::Provider::Mail163;
        let error = mail_form_account(&form, crate::i18n::Language::Zh)
            .expect_err("a webmail URL is not a mailbox address");
        assert!(error.contains("name@163.com"), "{error}");
    }

    #[test]
    fn bluetooth_notice_uses_real_connection_and_battery_data() {
        assert_eq!(
            bluetooth_notice_content("OpenMove by Shokz · 电量 80%"),
            ("OpenMove by Shokz", "已连接", Some(80))
        );
        assert_eq!(bluetooth_notice_content("耳机"), ("耳机", "已连接", None));
        assert_eq!(
            bluetooth_notice_content("已断开连接"),
            ("蓝牙设备", "已断开", None)
        );
        assert_eq!(
            bluetooth_notice_content("青龙键盘 · 已断开"),
            ("青龙键盘", "已断开", None)
        );
        assert_eq!(
            bluetooth_notice_content("耳机 · 电量 101%"),
            ("耳机 · 电量 101%", "已连接", None)
        );
    }

    #[test]
    fn device_icons_render_as_visible_svg() {
        let icons: &[&[u8]] = &[
            include_bytes!("../assets/lucide/headphones.svg"),
            include_bytes!("../assets/lucide/speaker.svg"),
            include_bytes!("../assets/lucide/keyboard.svg"),
            include_bytes!("../assets/lucide/mouse.svg"),
            include_bytes!("../assets/lucide/gamepad-2.svg"),
            include_bytes!("../assets/lucide/smartphone.svg"),
            include_bytes!("../assets/lucide/bluetooth.svg"),
        ];
        for bytes in icons {
            let image = egui_extras::image::load_svg_bytes(bytes).unwrap();
            assert!(image.pixels.iter().any(|p| p.a() > 0));
        }
    }
}

fn notice_button(c: &Card, x: f32, y: f32, w: f32, text: &str, id: &str) -> bool {
    let (r, b) = crate::paint::capsule(
        c,
        x,
        y,
        w,
        20.0,
        id,
        Color32::from_gray(35),
        theme::BTN_HOVER,
    );
    b.centered_text(
        x + w / 2.0,
        y + 10.0,
        text,
        10.5,
        W::Semibold,
        theme::WHITE,
        H::Center,
    );
    r.clicked()
}
fn switch(ui: &mut egui::Ui, value: &mut bool) -> bool {
    let ctx = &ui.ctx().clone();
    let palette = settings_palette(ctx);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 20.0), egui::Sense::click());
    if response.clicked() {
        *value = !*value;
    }
    let fill = if *value {
        palette.accent
    } else {
        palette.toggle_off
    };
    ui.painter().rect_filled(rect, 10.0, fill);
    let x = if *value {
        rect.right() - 10.0
    } else {
        rect.left() + 10.0
    };
    ui.painter().circle_filled(
        egui::pos2(x, rect.center().y + 0.5),
        7.5,
        Color32::from_black_alpha(35),
    );
    ui.painter().circle_filled(
        egui::pos2(x, rect.center().y),
        7.0,
        if *value {
            palette.accent_text
        } else {
            palette.knob_off
        },
    );
    response.clicked()
}
/// 设置页版式：标签列宽、开关尺寸、控件与卡片右边缘之间的固定留白。
pub(crate) const SETTINGS_LABEL_W: f32 = 80.0;
const SETTINGS_SWITCH_W: f32 = 34.0;
const SETTINGS_ROW_GAP: f32 = 16.0;

/// 开关行：左侧标题 + 说明（宽度固定、自动换行），右侧开关，长说明也不会把开关挤出卡片。
pub(crate) fn setting_row(ui: &mut egui::Ui, title: &str, description: &str, value: &mut bool) {
    let palette = settings_palette(ui.ctx());
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        let text_w = (ui.available_width() - SETTINGS_SWITCH_W - SETTINGS_ROW_GAP).max(120.0);
        ui.vertical(|ui| {
            ui.set_width(text_w);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.add(
                egui::Label::new(
                    egui::RichText::new(title)
                        .size(11.5)
                        .color(palette.text_medium),
                )
                .wrap(),
            );
            if !description.is_empty() {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(description)
                            .size(10.0)
                            .color(palette.subtle),
                    )
                    .wrap(),
                );
            }
        });
        // add_space 之后还会再插入一次 item_spacing.x，这里一并扣掉，开关才不会每行往外挤一点。
        ui.add_space(
            (ui.available_width() - SETTINGS_SWITCH_W - ui.spacing().item_spacing.x).max(0.0),
        );
        switch(ui, value);
    });
    ui.add_space(5.0);
}

pub(crate) fn settings_segmented<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    value: &mut T,
    options: &[(T, &str)],
    item_w: f32,
) -> bool {
    let options: Vec<_> = options
        .iter()
        .map(|(option, label)| (*option, *label, None))
        .collect();
    settings_segmented_inner(ui, value, &options, item_w)
}

pub(crate) fn settings_segmented_with_icons<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    value: &mut T,
    options: &[(T, &str, Icon)],
    item_w: f32,
) -> bool {
    let options: Vec<_> = options
        .iter()
        .map(|(option, label, icon)| (*option, *label, Some(*icon)))
        .collect();
    settings_segmented_inner(ui, value, &options, item_w)
}

fn settings_segmented_inner<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    value: &mut T,
    options: &[(T, &str, Option<Icon>)],
    item_w: f32,
) -> bool {
    let ctx = &ui.ctx().clone();
    let palette = settings_palette(ctx);
    let mut changed = false;
    egui::Frame::new()
        .fill(palette.surface_selected)
        .stroke(egui::Stroke::new(1.0_f32, palette.line_muted))
        .corner_radius(7.0)
        .inner_margin(2)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let right_to_left = ui.layout().main_dir() == egui::Direction::RightToLeft;
                let ordered: Vec<_> = if right_to_left {
                    options.iter().rev().collect()
                } else {
                    options.iter().collect()
                };
                for (option, label, icon) in ordered {
                    let selected = *value == *option;
                    let color = if selected {
                        palette.text_title
                    } else {
                        palette.muted
                    };
                    let text = egui::RichText::new(*label).size(10.5).color(color);
                    let button = match icon {
                        Some(icon) => egui::Button::image_and_text(
                            lucide::image(ctx, *icon, 13.0, color),
                            text,
                        ),
                        None => egui::Button::new(text),
                    }
                    .fill(if selected {
                        palette.surface
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(egui::Stroke::NONE)
                    .corner_radius(5.0);
                    let width = item_w.max(
                        ctx.fonts(|f| {
                            f.layout_no_wrap(
                                label.to_string(),
                                egui::FontId::proportional(10.5),
                                Color32::WHITE,
                            )
                            .size()
                            .x
                        }) + if icon.is_some() { 39.0 } else { 20.0 },
                    );
                    if ui.add_sized([width, 23.0], button).clicked() && !selected {
                        *value = *option;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// 选项行：标签列之后紧跟控件，同一卡片里所有控件左缘对齐成一条竖线。
pub(crate) fn settings_control_row_right(
    ui: &mut egui::Ui,
    title: &str,
    controls: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    settings_control_row(ui, title, SETTINGS_LABEL_W, |ui| {
        ui.horizontal(|ui| controls(ui)).response
    })
}

fn settings_slider(
    ui: &mut egui::Ui,
    value: &mut i32,
    min: i32,
    max: i32,
    step: i32,
    width: f32,
) -> egui::Response {
    let palette = settings_palette(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, 16.0), egui::Sense::click_and_drag());
    if (response.dragged() || response.clicked()) && ui.input(|input| input.pointer.primary_down())
    {
        if let Some(pointer) = response.interact_pointer_pos() {
            let track = rect.shrink2(egui::vec2(6.5, 6.0));
            let ratio = ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let half = step as f32 / 2.0;
            let snapped = (((min as f32 + (max - min) as f32 * ratio) + half) / step as f32)
                .floor()
                * step as f32;
            *value = (snapped as i32).clamp(min, max);
        }
    }
    let ratio = ((*value - min) as f32 / (max - min) as f32).clamp(0.0, 1.0);
    let track = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2((rect.width() - 13.0).max(1.0), 4.0),
    );
    let filled = egui::Rect::from_min_max(
        track.left_top(),
        egui::pos2(track.left() + track.width() * ratio, track.bottom()),
    );
    let thumb = egui::pos2(track.left() + track.width() * ratio, track.center().y);
    ui.painter().rect_filled(track, 999.0, palette.line_strong);
    ui.painter().rect_filled(filled, 999.0, palette.text_soft);
    ui.painter().circle_filled(
        thumb + egui::vec2(0.0, 1.0),
        6.5,
        Color32::from_black_alpha(28),
    );
    ui.painter().circle_filled(thumb, 6.5, palette.surface);
    ui.painter().circle_stroke(
        thumb,
        6.5,
        egui::Stroke::new(1.0_f32, palette.line_emphasis),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn island_size_slider(ui: &mut egui::Ui, value: &mut u32) {
    let id = ui.make_persistent_id("island-size-draft");
    let mut current = ui
        .data(|data| data.get_temp::<i32>(id))
        .unwrap_or(*value as i32);
    settings_slider(
        ui,
        &mut current,
        crate::model::ISLAND_SIZE_MIN as i32,
        crate::model::ISLAND_SIZE_MAX as i32,
        crate::model::ISLAND_SIZE_STEP as i32,
        148.0,
    );
    if ui.input(|input| input.pointer.primary_down()) {
        ui.data_mut(|data| data.insert_temp(id, current));
    } else {
        ui.data_mut(|data| data.remove::<i32>(id));
        *value = crate::model::island_size_value(current as u32);
    }
    let shown = if ui.input(|input| input.pointer.primary_down()) {
        crate::model::island_size_value(current as u32)
    } else {
        *value
    };
    ui.label(egui::RichText::new(format!("{shown}%")).size(10.0));
}

pub(crate) fn settings_hint(ui: &mut egui::Ui, text: &str) {
    let palette = settings_palette(ui.ctx());
    ui.add(egui::Label::new(egui::RichText::new(text).size(10.5).color(palette.muted)).wrap());
}
pub(crate) fn settings_card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let palette = settings_palette(ui.ctx());
    egui::Frame::new()
        .fill(palette.surface_subtle)
        .stroke(egui::Stroke::new(1.0_f32, palette.line_soft))
        .corner_radius(9.0)
        .inner_margin(egui::Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            add(ui);
        });
}
pub(crate) fn settings_label_with_icon(ui: &mut egui::Ui, icon: Icon, text: &str) {
    let palette = settings_palette(ui.ctx());
    ui.add_space(9.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.add(lucide::image(ui.ctx(), icon, 13.0, palette.muted));
        ui.label(egui::RichText::new(text).size(10.0).color(palette.muted));
    });
    ui.add_space(2.0);
}
pub(crate) fn settings_control_row(
    ui: &mut egui::Ui,
    title: &str,
    label_width: f32,
    controls: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    let palette = settings_palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(label_width.max(SETTINGS_LABEL_W), 30.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(
                    egui::RichText::new(title)
                        .size(11.5)
                        .color(palette.text_medium),
                );
            },
        );
        let content = ui
            .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                controls(ui)
            })
            .inner;
        content
    })
    .inner
}
fn setting_number<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    title: &str,
    value: &mut N,
    min: N,
    max: N,
    suffix: &str,
    speed: f64,
) -> bool {
    let mut changed = false;
    settings_control_row(ui, title, 80.0, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(140.0, 30.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    changed = number_stepper(ui, value, min, max, suffix, N::from_f64(speed), 76.0);
                },
            );
        })
        .response
    });
    changed
}
fn number_stepper<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut N,
    min: N,
    max: N,
    suffix: &str,
    step: N,
    field_width: f32,
) -> bool {
    let palette = settings_palette(ui.ctx());
    let mut changed = false;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.spacing_mut().button_padding = egui::vec2(4.0, 2.0);
        let button = |text| {
            egui::Button::new(
                egui::RichText::new(text)
                    .size(16.0)
                    .color(palette.text_medium),
            )
            .fill(palette.control_bg)
            .stroke(egui::Stroke::NONE)
            .corner_radius(6.0)
        };
        let decrease = ui
            .add_enabled(*value > min, button("−").min_size(egui::vec2(26.0, 26.0)))
            .on_hover_text("减少");
        if decrease.clicked() {
            *value = N::from_f64((value.to_f64() - step.to_f64()).max(min.to_f64()));
            changed = true;
        }
        let field = ui.add_sized(
            [field_width, 28.0],
            egui::DragValue::new(value)
                .range(min..=max)
                .speed(step.to_f64())
                .suffix(suffix)
                .fixed_decimals(if N::INTEGRAL { 0 } else { 2 }),
        );
        changed |= field.changed();
        if ui
            .add_enabled(*value < max, button("+").min_size(egui::vec2(26.0, 26.0)))
            .on_hover_text("增加")
            .clicked()
        {
            *value = N::from_f64((value.to_f64() + step.to_f64()).min(max.to_f64()));
            changed = true;
        }
    });
    changed
}
fn setting_choices(ui: &mut egui::Ui, title: &str, value: &mut u32, values: [u32; 3], unit: &str) {
    let labels = values.map(|v| format!("{v} {unit}"));
    let options: Vec<(u32, &str)> = values
        .iter()
        .zip(labels.iter())
        .map(|(v, l)| (*v, l.as_str()))
        .collect();
    settings_control_row_right(ui, title, |ui| {
        settings_segmented(ui, value, &options, 62.0);
    });
}
const SETTINGS_GROUPS: [(&str, usize, usize); 9] = [
    ("常规", 0, 0),
    ("页面管理", 10, 10),
    ("NOMI", 11, 11),
    ("系统监控", 12, 12),
    ("音乐与歌词", 1, 1),
    ("文件口袋", 5, 5),
    ("剪贴板", 6, 6),
    ("通知与邮箱", 7, 8),
    ("关于", 9, 9),
];
const SETTINGS_SIZE: [f32; 2] = [900.0, 680.0];
const SETTINGS_SIDEBAR_WIDTH: f32 = 150.0;

fn settings_group_name(start: usize, language: crate::i18n::Language) -> &'static str {
    match start {
        0 => language.text("常规", "General"),
        10 => language.text("页面管理", "Pages"),
        11 => language.text("NOMI", "NOMI"),
        12 => language.text("系统监控", "System Monitor"),
        1 => language.text("音乐与歌词", "Music & Lyrics"),
        5 => language.text("文件口袋", "Pocket"),
        6 => language.text("剪贴板", "Clipboard"),
        7 => language.text("通知与邮箱", "Alerts & Mail"),
        _ => language.text("关于", "About"),
    }
}

fn settings_page_title(index: usize, language: crate::i18n::Language) -> &'static str {
    match index {
        0 => language.text("常规", "General"),
        1 => language.text("音乐与歌词", "Music & Lyrics"),
        11 => language.text("NOMI", "NOMI"),
        12 => language.text("系统监控", "System Monitor"),
        5 => language.text("文件口袋", "Pocket"),
        6 => language.text("剪贴板", "Clipboard"),
        7 => language.text("通知", "Notifications"),
        8 => language.text("邮箱", "Mail"),
        9 => language.text("关于", "About"),
        10 => language.text("页面管理", "Pages"),
        _ => "",
    }
}

fn settings_viewport(title: &str, position: Option<egui::Pos2>) -> egui::ViewportBuilder {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(title)
        .with_icon(theme::app_icon())
        .with_decorations(true)
        .with_inner_size(SETTINGS_SIZE)
        .with_min_inner_size([720.0, 540.0])
        .with_visible(false)
        .with_active(false)
        .with_window_level(egui::WindowLevel::Normal);
    if let Some(position) = position {
        viewport = viewport.with_position(position);
    }
    viewport
}

#[cfg(windows)]
fn apply_settings_caption(title: &str, dark: bool, palette: Palette) {
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::HWND,
            Graphics::Dwm::{
                DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
            },
            UI::WindowsAndMessaging::FindWindowW,
        },
    };

    fn colorref(color: Color32) -> u32 {
        ((color.b() as u32) << 16) | ((color.g() as u32) << 8) | color.r() as u32
    }

    let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    let hwnd = unsafe { FindWindowW(None, PCWSTR(title.as_ptr())) }.unwrap_or(HWND::default());
    if hwnd.0.is_null() {
        return;
    }
    let set = |attribute, value: u32| unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            attribute,
            &value as *const u32 as *const core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
    };
    set(DWMWA_CAPTION_COLOR, colorref(palette.chrome));
    set(DWMWA_TEXT_COLOR, colorref(palette.text));
    set(DWMWA_USE_IMMERSIVE_DARK_MODE, u32::from(dark));
}

#[cfg(not(windows))]
fn apply_settings_caption(_title: &str, _dark: bool, _palette: Palette) {}

fn style_settings_content(ui: &mut egui::Ui, palette: Palette) {
    ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(12.5));
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(12.0));
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(10.5));
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(10.5));
    ui.visuals_mut().override_text_color = Some(palette.text);
    ui.visuals_mut().extreme_bg_color = palette.control_bg;
    ui.visuals_mut().faint_bg_color = palette.surface_subtle;
    ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
    ui.spacing_mut().interact_size.y = 24.0;
    {
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.noninteractive.bg_fill = palette.surface;
        widgets.noninteractive.weak_bg_fill = palette.surface_subtle;
        widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, palette.line_soft);
        widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, palette.text_soft);
        widgets.inactive.weak_bg_fill = palette.control_bg;
        widgets.inactive.bg_fill = palette.control_bg;
        widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, palette.line_strong);
        widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, palette.text_medium);
        widgets.hovered.weak_bg_fill = palette.control_bg_hover;
        widgets.hovered.bg_fill = palette.control_bg_hover;
        widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, palette.line_emphasis);
        widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, palette.text_title);
        widgets.active.weak_bg_fill = palette.surface_selected;
        widgets.active.bg_fill = palette.surface_selected;
        widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, palette.line_emphasis);
        widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, palette.text_title);
        for widget in [
            &mut widgets.noninteractive,
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
            &mut widgets.open,
        ] {
            widget.corner_radius = egui::CornerRadius::same(6);
        }
    }
    ui.visuals_mut().selection.bg_fill = palette.line_emphasis;
    ui.visuals_mut().selection.stroke.color = palette.text_soft;
    let divider = palette.line;
    ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, divider);
}

fn settings_group_icon(group: usize) -> Icon {
    match group {
        0 => Icon::Settings,
        10 => Icon::LayoutGrid,
        11 => Icon::Sparkles,
        1 => Icon::Music,
        5 => Icon::Folder,
        6 => Icon::Clipboard,
        7 => Icon::Bell,
        12 => Icon::Activity,
        _ => Icon::Info,
    }
}

fn nomi_expression_gallery(
    ui: &mut egui::Ui,
    language: crate::i18n::Language,
    palette: Palette,
    t: f64,
) {
    let total = crate::pet::NomiExpression::ALL.len();
    ui.label(
        egui::RichText::new(if language.is_en() {
            format!(
                "{total} NOMI expressions. Only the default eyes follow the pointer; the other expressions keep a fixed composition. Idle expressions rotate every 5–10 seconds."
            )
        } else {
            format!(
                "全部 {total} 个 NOMI 表情。只有默认双眼跟随鼠标，其他表情保持固定构图；闲置时每 5–10 秒自然轮换。"
            )
        })
        .size(10.5)
        .color(palette.muted),
    );
    ui.add_space(8.0);

    for category in crate::pet::NomiCategory::ALL {
        debug_assert!(category
            .expressions()
            .iter()
            .all(|expression| expression.category() == category));
        settings_label_with_icon(ui, Icon::Sparkles, category.label_for(language));
        settings_card(ui, |ui| {
            let count = category.expressions().len();
            ui.label(
                egui::RichText::new(if language.is_en() {
                    format!("{count} expressions in this scene")
                } else {
                    format!("此场景共 {count} 个表情")
                })
                .size(10.5)
                .color(palette.muted),
            );
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                for &expression in category.expressions() {
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(80.0, 82.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 9.0, palette.surface_subtle);
                    let card = Card::new(ui, rect.min, rect);
                    let center_x = rect.width() / 2.0;
                    draw_nomi_tile(&card, expression, center_x, t);
                    card.centered_text(
                        center_x,
                        68.0,
                        expression.label_for(language),
                        10.5,
                        W::Regular,
                        palette.text_soft,
                        H::Center,
                    );
                    response.on_hover_text(format!(
                        "{} · {}",
                        category.label_for(language),
                        expression.label_for(language)
                    ));
                }
            });
        });
        ui.add_space(7.0);
    }
}

fn draw_nomi_tile(card: &Card<'_>, expression: crate::pet::NomiExpression, cx: f32, t: f64) {
    const CY: f32 = 28.0;
    let face = card.scaled(cx, CY, 1.72);
    crate::pet::draw_expression(&face, expression, cx, CY, t, [0.0, 0.0]);
}

fn mail_form_account(
    form: &crate::app::MailForm,
    language: crate::i18n::Language,
) -> Result<(crate::mail::Account, String), String> {
    let email = form.email.trim().to_string();
    if let Some(provider) = crate::mail::Provider::for_web_url(&email) {
        return Err(if language.is_en() {
            format!(
                "This is the {} website, not a mailbox account. Enter an address such as {}.",
                provider.label_for(language),
                provider.login_example()
            )
        } else {
            format!(
                "这是{}网页地址，不是邮箱账号。请填写完整邮箱，例如 {}。",
                provider.label_for(language),
                provider.login_example()
            )
        });
    }
    let (local, domain) = email
        .split_once('@')
        .ok_or_else(|| language.text("请输入完整邮箱地址", "Enter a complete email address"))?;
    if local.is_empty()
        || domain.is_empty()
        || !domain.contains('.')
        || email.chars().any(char::is_whitespace)
    {
        return Err(language
            .text("邮箱地址格式不正确", "The email address is invalid")
            .into());
    }
    if form.provider != crate::mail::Provider::Custom {
        if let Some(detected) = crate::mail::Provider::for_email(&email) {
            if detected != form.provider {
                return Err(if language.is_en() {
                    format!(
                        "This address belongs to {}. Select that provider or Custom IMAP.",
                        detected.label_for(language)
                    )
                } else {
                    format!(
                        "这个邮箱地址应使用「{}」。请切换服务商，或选择自定义 IMAP。",
                        detected.label_for(language)
                    )
                });
            }
        }
    }
    let password = form.password.trim().to_string();
    if password.is_empty() {
        return Err(language
            .text(
                "请输入授权码或应用专用密码，不是邮箱登录密码",
                "Enter an app password or authorization code, not your normal login password",
            )
            .into());
    }
    let account = crate::mail::Account {
        custom: (form.provider == crate::mail::Provider::Custom).then(|| form.custom.clone()),
        email,
        provider: form.provider,
        enabled: true,
    };
    account.endpoint()?;
    Ok((account, password))
}

impl App {
    fn settings_sidebar(
        &mut self,
        ctx: &egui::Context,
        palette: Palette,
        language: crate::i18n::Language,
    ) {
        egui::SidePanel::left("settings-sidebar")
            .resizable(false)
            .show_separator_line(false)
            .exact_width(SETTINGS_SIDEBAR_WIDTH)
            .frame(
                egui::Frame::new()
                    .fill(palette.chrome)
                    .inner_margin(egui::Margin::symmetric(8, 10)),
            )
            .show(ctx, |ui| {
                for (_, start, end) in SETTINGS_GROUPS {
                    let name = settings_group_name(start, language);
                    let selected = (start..=end).contains(&self.settings_tab);
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 32.0),
                        egui::Sense::click(),
                    );
                    if response.clicked() && !selected {
                        self.settings_tab = start;
                    }
                    if selected || response.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            6.0,
                            if selected {
                                palette.surface
                            } else {
                                palette.surface_hover
                            },
                        );
                    }
                    let color = if selected {
                        palette.text
                    } else {
                        palette.muted
                    };
                    let icon_rect = Rect::from_center_size(
                        egui::pos2(rect.left() + 15.0, rect.center().y),
                        egui::vec2(14.0, 14.0),
                    );
                    lucide::paint(ui.painter(), icon_rect, settings_group_icon(start), color);
                    ui.painter().text(
                        egui::pos2(rect.left() + 30.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        name,
                        egui::FontId::proportional(11.5),
                        color,
                    );
                    ui.add_space(2.0);
                }
            });
    }

    pub fn settings_ui(&mut self, ctx: &egui::Context, autostart: bool) {
        self.updates.poll(ctx, self.settings.language);
        self.updates.dialog_ui(ctx, self.settings.language);
        if !self.settings_open {
            return;
        }
        let id = egui::ViewportId::from_hash_of("nexora-isle-settings");
        let focus_requested = std::mem::take(&mut self.settings_focus_requested);
        let language = self.settings.language;
        let settings_title = language.text(
            crate::product::identity::SETTINGS_TITLE,
            crate::product::identity::SETTINGS_TITLE_EN,
        );
        let settings_position = crate::app::centered_decorated_viewport_position(
            ctx,
            Vec2::new(SETTINGS_SIZE[0], SETTINGS_SIZE[1]),
        );
        ctx.show_viewport_immediate(id, settings_viewport(settings_title, settings_position), |ctx, _| {
            let dark=self.settings.settings_theme.dark(crate::settings_theme::system_dark(ctx));
            let original_style=ctx.style();
            let mut style=(*original_style).clone();
            style.visuals=if dark{egui::Visuals::dark()}else{egui::Visuals::light()};
            ctx.set_style(style);
            let palette=settings_palette_for(dark);
            ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(if dark{egui::SystemTheme::Dark}else{egui::SystemTheme::Light}));
            if self.settings.settings_theme==crate::settings_theme::SettingsTheme::System {
                ctx.request_repaint_after(std::time::Duration::from_secs(1));
            }
            if focus_requested {
                // Glow 在首帧交换缓冲后执行窗口命令，避免先显示白色空窗口。
                if let Some(position) = settings_position {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            if ctx.input(|i|i.viewport().close_requested()){self.settings_open=false;}
            let before=format!("{:?}",self.settings);
            apply_settings_caption(settings_title, dark, palette);
            self.settings_sidebar(ctx, palette, language);
            egui::CentralPanel::default().frame(egui::Frame::new().fill(palette.chrome).inner_margin(0)).show(ctx,|ui|{
                egui::Frame::new().fill(palette.surface).corner_radius(12.0).inner_margin(egui::Margin { left: 22, right: 22, top: 12, bottom: 18 }).outer_margin(egui::Margin { left: 8, right: 12, top: 12, bottom: 12 }).show(ui,|ui|{
                ui.set_min_width(ui.available_width());
                ui.set_min_height(ui.available_height());
                style_settings_content(ui, palette);
                ui.add_space(2.0);
                ui.label(egui::RichText::new(settings_page_title(self.settings_tab, language)).font(theme::font(14.0,W::Semibold)).color(palette.text));
                if let Some((_,start,end))=SETTINGS_GROUPS.iter().find(|(_,start,end)|(*start..=*end).contains(&self.settings_tab)) {
                    if start!=end {
                        ui.add_space(6.0);
                        egui::Frame::new().fill(palette.surface_selected).stroke(egui::Stroke::new(1.0_f32,palette.line_muted)).corner_radius(7.0).inner_margin(2).show(ui,|ui|{
                            ui.spacing_mut().item_spacing.x=2.0;
                            ui.horizontal(|ui|{
                                for index in *start..=*end {
                                    let selected=self.settings_tab==index;
                                    let button=egui::Button::new(egui::RichText::new(settings_page_title(index, language)).size(10.5).color(if selected{palette.text_title}else{palette.muted}))
                                            .fill(if selected{palette.surface}else{Color32::TRANSPARENT})
                                        .stroke(egui::Stroke::NONE).corner_radius(5.0);
                                    if ui.add_sized([68.0,23.0],button).clicked(){self.settings_tab=index;}
                                }
                            });
                        });
                    }
                }
                ui.add_space(6.0);
                egui::ScrollArea::vertical().id_salt(("settings-scroll",self.settings_tab)).auto_shrink([false,false]).show(ui,|ui|{
                    // 右侧给滚动条留出空隙，卡片和开关不会被它压住。
                    ui.set_width((ui.available_width()-12.0).max(240.0));
                    match self.settings_tab {
                    11=>{
                        nomi_expression_gallery(ui,language,palette,self.t);
                    }
                    12=>{
                        ui.label(egui::RichText::new(language.text("实时读取本机 CPU、内存、磁盘与 GPU；历史数据只保存在本次运行中。","Read CPU, memory, disk and GPU locally. History stays in memory for this session.")).size(10.5).color(palette.muted));
                        settings_label_with_icon(ui,Icon::Activity,language.text("显示与采样","Display & sampling"));
                        settings_card(ui,|ui|{
                            setting_row(ui,language.text("系统监控页","System monitor"),language.text("在岛屿中显示实时资源、1/5 分钟曲线和 Top 应用","Show live resources, 1/5-minute charts and Top Apps in the island"),&mut self.settings.monitor_enabled);
                            ui.separator();
                            setting_row(ui,language.text("阈值警报","Threshold alerts"),language.text("指标持续超过阈值 5 秒后提醒，冷却 60 秒","Alert after five seconds above threshold with a 60-second cooldown"),&mut self.settings.monitor_alerts);
                        });
                        settings_label_with_icon(ui,Icon::Bell,language.text("警报阈值","Alert thresholds"));
                        settings_card(ui,|ui|{
                            setting_number(ui,language.text("CPU","CPU"),&mut self.settings.monitor_cpu_threshold,50,100," %",1.0);
                            ui.separator();
                            setting_number(ui,language.text("内存","Memory"),&mut self.settings.monitor_memory_threshold,50,100," %",1.0);
                            ui.separator();
                            setting_number(ui,language.text("GPU","GPU"),&mut self.settings.monitor_gpu_threshold,50,100," %",1.0);
                            settings_hint(ui,language.text("触发后通过现有岛屿通知呈现，并写入本机最近 200 条监控事件。","Alerts use the island notification pipeline and keep the latest 200 monitor events locally."));
                        });
                        settings_label_with_icon(ui,Icon::Activity,language.text("当前状态","Current status"));
                        settings_card(ui,|ui|{
                            ui.horizontal(|ui|{
                                ui.label(egui::RichText::new(format!("CPU {:.0}%",self.monitor_latest.sample.cpu_percent)).size(10.5).color(palette.text_soft));
                                ui.label(egui::RichText::new(format!("· {} {:.0}%",language.text("内存","Memory"),self.monitor_latest.sample.memory_percent)).size(10.5).color(palette.text_soft));
                                if let Some(gpu)=self.monitor_latest.sample.gpu_percent{ui.label(egui::RichText::new(format!("· GPU {gpu:.0}%")).size(10.5).color(palette.text_soft));}
                            });
                            ui.add_space(5.0);
                            ui.horizontal(|ui|{
                                if ui.add_sized([120.0,28.0],egui::Button::new(language.text("打开监控","Open Monitor"))).clicked(){self.settings_open=false;self.show_page(crate::model::page::MONITOR);}
                                if ui.add_sized([120.0,28.0],egui::Button::new(language.text("清空事件","Clear events"))).clicked(){self.monitor_events.clear();crate::monitor::save_events(&[]);}
                            });
                        });
                    }
                    10=>{
                        ui.label(egui::RichText::new(language.text("选择需要的页面和状态组件。","Choose the pages and status items shown in ISLE.")).size(10.5).color(palette.muted));
                        settings_label_with_icon(ui,Icon::LayoutGrid,language.text("常驻排列","Permanent order"));
                        settings_card(ui,|ui|self.resident_order_settings(ui,language));
                        settings_label_with_icon(ui,Icon::Settings,language.text("状态组件","Status items"));
                        settings_card(ui,|ui|{
                            setting_row(ui,language.text("电量","Battery"),language.text("本机电量、充电状态与提醒；无电池设备自动隐藏","Battery level, charging state, and alerts; hidden automatically when unavailable"),&mut self.settings.show_battery);
                            ui.separator();
                            setting_row(ui,language.text("岛屿歌词","Island lyrics"),language.text("收起时显示正在唱的那一句","Show the current lyric line while collapsed"),&mut self.settings.island_lyrics);
                        });
                        settings_label_with_icon(ui,Icon::LayoutGrid,language.text("功能页面","Feature pages"));
                        settings_card(ui,|ui|{
                            let descriptions=if language.is_en(){["Playback controls and synced lyrics","Browser video sessions and return to the page","Live system metrics, history and alerts","Keep temporary file shortcuts","Copied images with hover previews","Recent copied text and pinned entries","Mail summaries and verification codes"]}else{["播放控制与同步歌词","浏览器视频会话与返回网页","实时资源、历史曲线与阈值警报","临时收纳文件位置","复制图片与悬浮预览","最近复制文字与固定条目","邮件摘要与验证码"]};
                            ui.label(egui::RichText::new(language.text("拖动左侧手柄调整顺序，松开后自动应用到岛屿。","Drag the handle to reorder. Changes apply when released.")).size(10.5).color(palette.muted));
                            #[derive(Clone)]
                            struct PageDrag(usize);
                            let mut movement=None;
                            let row_w=ui.available_width();
                            for (index,page) in self.settings.page_order.into_iter().enumerate() {
                                if index>0 {ui.separator();}
                                let mut visible=self.settings.visible_pages[page];
                                let can_change=!visible || self.settings.visible_pages.iter().filter(|enabled|**enabled).count()>1;
                                let row=ui.push_id(("page-order",page),|ui|ui.horizontal(|ui|{
                                    ui.set_max_width(row_w);
                                    let (rect,handle)=ui.allocate_exact_size(egui::vec2(20.0,40.0),egui::Sense::drag());
                                    lucide::paint(ui.painter(),Rect::from_center_size(rect.center(),egui::vec2(16.0,16.0)),Icon::GripVertical,palette.subtle);
                                    handle.on_hover_cursor(egui::CursorIcon::Grab).dnd_set_drag_payload(PageDrag(page));
                                    let page_name=if language.is_en(){match page {crate::model::page::MUSIC=>"Music",crate::model::page::VIDEO=>"Video",crate::model::page::MONITOR=>"Monitor",crate::model::page::SHELF=>"Pocket",crate::model::page::IMAGES=>"Images",crate::model::page::CLIPBOARD=>"Clipboard",_=>"Mail"}}else{PAGE_NAMES[page]};
                                    ui.add_enabled_ui(can_change,|ui|{setting_row(ui,page_name,descriptions[page],&mut visible);});
                                })).inner.response;
                                if let Some(payload)=row.dnd_hover_payload::<PageDrag>() {
                                    if payload.0!=page {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                        let y=if self.settings.page_order.iter().position(|p|*p==payload.0).unwrap()<index {row.rect.bottom()}else{row.rect.top()};
                                        ui.painter().line_segment([egui::pos2(row.rect.left(),y),egui::pos2(row.rect.right(),y)],egui::Stroke::new(2.0_f32,Color32::from_rgb(70,150,255)));
                                    }
                                }
                                if let Some(payload)=row.dnd_release_payload::<PageDrag>() {movement=Some((payload.0,index));}
                                if visible!=self.settings.visible_pages[page] {self.set_page_visible(page,visible);}
                            }
                            if let Some((page,to))=movement {
                                let from=self.settings.page_order.iter().position(|p|*p==page).unwrap();
                                if from<to {self.settings.page_order[from..=to].rotate_left(1);}
                                else {self.settings.page_order[to..=from].rotate_right(1);}
                            }
                            if self.settings.visible_pages.iter().filter(|enabled|**enabled).count()==1 {
                                ui.add_space(5.0);
                                ui.label(egui::RichText::new(language.text("至少保留一个页面。关闭仅隐藏页面，数据和提醒设置保留。","At least one page must remain enabled. Disabling a page only hides it; its data and settings are retained.")).size(10.5).color(palette.muted));
                            }
                        });
                    }
                    0=>{
                        use crate::i18n::Language;
                        settings_label_with_icon(ui,Icon::Languages,language.text("界面语言","Language"));
                        settings_card(ui,|ui|{
                            settings_control_row_right(ui,language.text("语言","Language"),|ui|{
                                settings_segmented(ui,&mut self.settings.language,&[(Language::Zh,"中文"),(Language::En,"English")],62.0);
                            });
                        });
                        settings_label_with_icon(ui,Icon::Sun,language.text("设置页外观","Settings appearance"));
                        settings_card(ui,|ui|{
                            use crate::settings_theme::SettingsTheme;
                            settings_control_row_right(ui,language.text("主题","Theme"),|ui|{
                                if settings_segmented_with_icons(ui,&mut self.settings.settings_theme,&[
                                    (SettingsTheme::System,language.text("跟随系统","System"),Icon::Monitor),
                                    (SettingsTheme::Light,language.text("浅色","Light"),Icon::Sun),
                                    (SettingsTheme::Dark,language.text("深色","Dark"),Icon::Moon),
                                ],52.0) {ctx.request_repaint();}
                            });
                            settings_hint(ui,language.text("仅影响设置页，岛的外观保持不变。","Changes the settings window only. The island remains unchanged."));
                        });
                        settings_label_with_icon(ui,Icon::Settings,language.text("启动","Startup"));
                        settings_card(ui,|ui|{
                            let mut a=autostart;
                            setting_row(ui,language.text("开机启动","Launch at startup"),language.text("登录 Windows 时自动打开 NEXORA ISLE","Open NEXORA ISLE when you sign in to Windows"),&mut a);
                            if a!=autostart{self.pending.push(Cmd::ToggleAutostart);}
                        });
                        settings_label_with_icon(ui,Icon::LayoutGrid,language.text("岛屿外观","Island appearance"));
                        settings_card(ui,|ui|{
                            use crate::model;
                            let shapes:Vec<(model::IslandShape,&str)>=model::IslandShape::ALL.iter().map(|s|(*s,s.label_for(language))).collect();
                            settings_control_row_right(ui,language.text("形状","Shape"),|ui|{settings_segmented(ui,&mut self.settings.island_shape,&shapes,64.0);});
                            settings_hint(ui,self.settings.island_shape.description_for(language));
                            ui.separator();
                            settings_control_row(ui,language.text("大小","Size"),56.0,|ui|{
                                ui.allocate_ui_with_layout(
                                    egui::vec2(270.0,30.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                    island_size_slider(ui,&mut self.settings.island_size);
                                    if ui.add_enabled(self.settings.island_size!=100,egui::Button::new(language.text("恢复默认","Reset"))).clicked(){self.settings.island_size=100;}
                                    },
                                ).response
                            });
                            settings_hint(ui,language.text("文字、图标和岛屿表情按实际像素重新绘制，放大后依旧清晰；不影响设置窗口。","Text, icons, and expressions are redrawn at the target pixel size. This only affects the island."));
                        });
                        settings_label_with_icon(ui,Icon::Monitor,language.text("岛屿显示","Island display"));
                        settings_card(ui,|ui|{
                            let selected=self.displays.iter().find(|d|d.id==self.settings.display_id);
                            let label=if self.settings.display_id.is_empty(){
                                language.text("跟随主屏","Primary display").to_string()
                            }else{
                                selected.map_or_else(
                                    || language.text("所选屏幕未连接","Selected display disconnected").to_string(),
                                    |display| display.label_for(language),
                                )
                            };
                            settings_control_row(ui,language.text("显示屏","Display"),56.0,|ui|{
                                egui::ComboBox::from_id_salt("island-display").selected_text(label).width(ui.available_width().min(300.0)).truncate().show_ui(ui,|ui|{
                                    ui.selectable_value(&mut self.settings.display_id,String::new(),language.text("跟随主屏","Primary display"));
                                    for display in &self.displays {ui.selectable_value(&mut self.settings.display_id,display.id.clone(),display.label_for(language));}
                                }).response
                            });
                            let edge=self.settings.dock.label_for(language);
                            ui.label(egui::RichText::new(if !self.settings.display_id.is_empty()&&selected.is_none(){language.text("暂时显示在主屏，重新连接所选屏幕后自动恢复。","Showing on the primary display until the selected display reconnects.").to_string()}else if language.is_en(){format!("Docked at the {} of the selected display. Falls back to the primary display when disconnected.",edge)}else{format!("显示在所选屏幕{edge}；断开时暂时回到主屏。")}).size(10.5).color(palette.muted));
                            ui.add_space(4.0);
                            // 停靠边缘：顶部为横向；左右为竖向排列，展开面板从边缘滑出。
                            let docks:Vec<(crate::model::Dock,&str)>=crate::model::Dock::ALL.iter().map(|d|(*d,d.label_for(language))).collect();
                            settings_control_row_right(ui,language.text("停靠","Dock"),|ui|{
                                // 换边后偏移的含义变了（水平 ↔ 上下），回到居中。
                                if settings_segmented(ui,&mut self.settings.dock,&docks,64.0) {self.settings.island_offset=0;}
                            });
                            if self.settings.dock.is_side() {
                                ui.label(egui::RichText::new(language.text("收起时竖向排列时间、电池、播放和提醒；展开面板从屏幕边缘滑出。岛屿表情和歌词只在顶部停靠时显示。","Time, battery, playback, and alerts are arranged vertically when collapsed. The island expression and lyrics are shown only when docked at the top.")).size(10.5).color(palette.muted));
                            }
                            ui.add_space(4.0);
                            let side=self.settings.dock.is_side();
                            settings_control_row(ui,if side{language.text("上下位置","Vertical position")}else{language.text("水平位置","Horizontal position")},56.0,|ui|{
                                ui.allocate_ui_with_layout(
                                    egui::vec2(330.0,30.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                    let limit=crate::model::ISLAND_OFFSET_MAX;
                                    let offset=&mut self.settings.island_offset;
                                    let (less,more,less_tip,more_tip)=if side{
                                        (Icon::ChevronUp,Icon::ChevronDown,
                                         language.text("向上微调","Move up"),
                                         language.text("向下微调","Move down"))
                                    }else{
                                        (Icon::ChevronLeft,Icon::ChevronRight,
                                         language.text("向左微调","Move left"),
                                         language.text("向右微调","Move right"))
                                    };
                                    if ui.add_enabled(*offset>-limit,egui::Button::image(lucide::image(ctx,less,14.0,palette.text_medium)).min_size(egui::vec2(26.0,22.0))).on_hover_text(less_tip).clicked(){*offset=(*offset-5).max(-limit);}
                                    settings_slider(ui,offset,-limit,limit,5,150.0);
                                    if ui.add_enabled(*offset<limit,egui::Button::image(lucide::image(ctx,more,14.0,palette.text_medium)).min_size(egui::vec2(26.0,22.0))).on_hover_text(more_tip).clicked(){*offset=(*offset+5).min(limit);}
                                    if ui.add_enabled(*offset!=0,egui::Button::new(language.text("恢复居中","Center"))).clicked(){*offset=0;}
                                    },
                                ).response
                            });
                            let offset=self.settings.island_offset;
                            let (less,more)=if side{(language.text("偏上","Above center"),language.text("偏下","Below center"))}else{(language.text("偏左","Left of center"),language.text("偏右","Right of center"))};
                            let position=match offset {0=>language.text("居中","Centered").to_string(),o if o<0=>format!("{less} {}%",-o),o=>format!("{more} {o}%")};
                            ui.label(egui::RichText::new(if language.is_en(){format!("Current: {position}. Panels and notifications remain on screen.")}else{format!("当前：{position}。展开面板和通知始终留在屏幕内；换屏或调整大小后保持相对位置。")}).size(10.5).color(palette.muted));
                            ui.separator();
                            settings_control_row_right(ui,language.text("时间制式","Time format"),|ui|{settings_segmented(ui,&mut self.settings.clock_12h,&[(false,language.text("24 小时","24-hour")),(true,language.text("12 小时","12-hour"))],72.0);});
                            settings_control_row_right(ui,language.text("时段文字","Period label"),|ui|{settings_segmented(ui,&mut self.settings.clock_period,&[(0,language.text("不显示","Off")),(1,language.text("上午 / 下午","AM / PM")),(2,language.text("详细时段","Detailed"))],72.0);});
                            settings_hint(ui,language.text("详细时段：凌晨、上午、中午、下午、傍晚、晚上","Detailed periods: night, morning, noon, afternoon, evening, and late night."));
                            ui.separator();
                            let mut minimal=self.settings.minimal;
                            setting_row(ui,language.text("极简刘海","Minimal island"),language.text("收起时显示一条细线","Show a thin line when collapsed"),&mut minimal);
                            if minimal!=self.settings.minimal {self.set_minimal(minimal);}
                            ui.separator();

                            let mut pet=self.settings.pet;
                            setting_row(ui,language.text("岛屿","Island"),language.text("收起时常驻，会看向鼠标并对操作作出表情","Stay visible when collapsed, follow the pointer, and react to actions"),&mut pet);
                            if pet!=self.settings.pet {self.set_pet(pet);}
                        });
                        self.avoidance.settings(ui,&mut self.settings,language);
                        settings_label_with_icon(ui,Icon::Monitor,language.text("声音与显示","Sound & display"));
                        settings_card(ui,|ui|{
                            setting_row(ui,language.text("提醒声音","Alert sounds"),language.text("系统与消息提示声音","Sounds for system and message alerts"),&mut self.settings.sound);
                            ui.separator();
                            setting_row(ui,language.text("全屏时隐藏","Hide in full screen"),language.text("游戏、视频和演示全屏时隐藏","Hide during full-screen games, videos, and presentations"),&mut self.settings.hide_fullscreen);
                        });
                    }
                    1=>{
                        settings_label_with_icon(ui,Icon::Music,language.text("歌词","Lyrics"));
                        settings_card(ui,|ui|{
                            setting_row(ui,language.text("展开时显示歌词","Show lyrics when expanded"),language.text("在播放器右侧同步滚动显示歌词","Show synchronized lyrics beside the player"),&mut self.settings.lyrics_enabled);
                            self.show_lyrics=self.settings.lyrics_enabled;
                            ui.separator();
                            setting_row(ui,language.text("岛屿歌词","Island lyrics"),language.text("收起时显示当前句；无同步歌词时恢复普通岛屿","Show the current line while collapsed; fall back to the standard island when synced lyrics are unavailable"),&mut self.settings.island_lyrics);
                        });
                        settings_label_with_icon(ui,Icon::Music,language.text("歌词显示时间","Lyric timing"));
                        settings_card(ui,|ui|{
                            let mut advance=-self.settings.lyric_delay_ms as f64/1000.0;
                            if setting_number(ui,language.text("时间偏移","Offset"),&mut advance,-5.0,5.0,language.text(" 秒"," s"),0.1){self.settings.lyric_delay_ms=(-advance*1000.0).round()as i32;}
                            ui.add_space(3.0);
                            ui.small(language.text("正数提前，负数延后；默认提前 0.35 秒。","Positive values advance lyrics; negative values delay them. Default: 0.35 s early."));
                        });
                    }
                    5=>{
                        settings_label_with_icon(ui,Icon::Folder,language.text("文件口袋","Pocket"));
                        settings_card(ui,|ui|{
                            ui.label(egui::RichText::new(language.text("保存文件位置，移除条目不会删除原文件。","Keep file shortcuts. Removing an item never deletes the source file.")).size(12.0));
                            ui.label(egui::RichText::new(if language.is_en(){format!("{} items",self.shelf.items.len())}else{format!("当前 {} 项",self.shelf.items.len())}).size(10.5).color(palette.muted));
                            ui.add_space(5.0);
                            if ui.add_sized([120.0,28.0],egui::Button::new(language.text("打开文件口袋","Open Pocket"))).clicked(){self.settings_open=false;self.show_page(crate::model::page::SHELF);}
                        });
                    }
                    6=>{
                        settings_label_with_icon(ui,Icon::Clipboard,language.text("剪贴板","Clipboard"));
                        settings_card(ui,|ui|{
                            setting_row(ui,language.text("记录剪贴板","Clipboard history"),language.text("查看最近复制的文字，点一下再次复制","Review recent copied text and click an item to copy it again"),&mut self.settings.clipboard_history);
                            ui.separator();
                            setting_row(ui,language.text("记录图片","Image history"),language.text("复制图片或截图后，在本机保存缩略图与原图","Keep copied images and screenshots locally with hover previews"),&mut self.settings.clipboard_images);
                            ui.label(egui::RichText::new(if language.is_en(){format!("The latest {} entries are stored locally. Pinned entries survive manual clears. Content marked private by password managers is ignored.",crate::clipboard::MAX_RECENT)}else{format!("本机保存最近 {} 条，重新打开后可继续使用。关闭记录会暂停收集，手动清空会保留固定条目。密码管理器标记为敏感的内容不会记录。",crate::clipboard::MAX_RECENT)}).size(10.5).color(palette.muted));
                            ui.add_space(5.0);
                            ui.horizontal(|ui|{
                                if ui.add_sized([120.0,28.0],egui::Button::new(language.text("打开剪贴板","Open Clipboard"))).clicked(){self.settings_open=false;self.show_page(crate::model::page::CLIPBOARD);}
                                if ui.add_sized([96.0,28.0],egui::Button::new(language.text("打开图片","Open Images"))).clicked(){self.settings_open=false;self.show_page(crate::model::page::IMAGES);}
                            });
                        });
                    }
                    7=>{
                        ui.label(egui::RichText::new(language.text("选择哪些事件需要出现在岛屿上。","Choose which events should appear on the island.")).size(10.5).color(palette.muted));
                        settings_label_with_icon(ui,Icon::Bell,language.text("系统状态","System status"));
                        settings_card(ui,|ui|{
                            setting_row(ui,language.text("状态变化提示","Status changes"),language.text("音量、电源、网络和输入状态的即时反馈","Immediate feedback for volume, power, network, and input changes"),&mut self.settings.system_hud);
                            ui.separator();
                            setting_row(ui,language.text("蓝牙设备","Bluetooth devices"),language.text("连接、断开和电量变化提示；关闭后停止扫描","Prompts for connection, disconnection, and battery changes; disables scanning when off"),&mut self.settings.bluetooth_notifications);
                            ui.separator();
                            setting_row(ui,language.text("岛屿通知","Island notifications"),language.text("允许消息与系统事件通过岛屿呈现","Allow messages and system events to appear through the island"),&mut self.settings.notifications);
                        });
                        settings_label_with_icon(ui,Icon::Bell,language.text("微信与系统通知","WeChat & system notifications"));
                        settings_card(ui,|ui|{
                            egui::Frame::new().fill(palette.surface_subtle).stroke(egui::Stroke::new(1.0_f32,palette.line_soft)).corner_radius(8.0).inner_margin(egui::Margin::symmetric(10,9)).show(ui,|ui|{
                                ui.horizontal(|ui|{
                                    let (dot,_)=ui.allocate_exact_size(egui::vec2(9.0,9.0),egui::Sense::hover());
                                    ui.painter().circle_filled(dot.center(),4.0,if self.settings.read_notifications{palette.accent}else{palette.subtle});
                                    ui.vertical(|ui|{
                                        ui.label(egui::RichText::new(language.text("通知连接","Notification access")).size(11.5).strong().color(palette.text_title));
                                        ui.add(egui::Label::new(egui::RichText::new(if self.settings.read_notifications{self.listener_status.as_str()}else{language.text("微信提醒可直接启用；系统通知需要 Windows 权限。","WeChat alerts work immediately; system alerts need Windows access.")}).size(10.0).color(palette.muted)).wrap());
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                                        let (label,command)=if self.settings.read_notifications{(language.text("断开","Disconnect"),Cmd::DisconnectNotifications)}else{(language.text("连接系统通知","Connect"),Cmd::RequestNotifications)};
                                        let button=egui::Button::new(egui::RichText::new(label).size(10.5).strong().color(if self.settings.read_notifications{palette.text_medium}else{palette.accent_text})).fill(if self.settings.read_notifications{palette.control_bg}else{palette.accent}).stroke(egui::Stroke::new(1.0_f32,palette.line_emphasis)).corner_radius(6.0);
                                        if ui.add_sized([100.0,28.0],button).clicked(){self.pending.push(command);}
                                    });
                                });
                            });
                            ui.add_space(5.0);
                            setting_row(ui,language.text("微信消息提醒","WeChat message alerts"),language.text("识别任务栏闪动；不读取联系人、聊天内容或通知正文","Detects taskbar activity without reading contacts, chats, or notification text"),&mut self.settings.wechat_notifications);
                            if self.settings.wechat_notifications {
                                ui.separator();
                                setting_row(ui,language.text("从提醒打开微信","Open WeChat from alerts"),language.text("微信缩在托盘时，点击岛屿提醒直接唤起窗口","Restore WeChat from its tray icon when the island alert is clicked"),&mut self.settings.wechat_tray_open);
                                if !self.wechat_status.is_empty(){ui.label(egui::RichText::new(&self.wechat_status).size(10.5).color(palette.muted));}
                            }else{
                                ui.add_space(2.0);
                                let enable=egui::Button::new(egui::RichText::new(language.text("启用微信提醒","Enable WeChat alerts")).size(10.5).color(palette.text_medium));
                                if ui.add_sized([124.0,26.0],enable).clicked(){self.pending.push(Cmd::EnableWechatAlerts);}
                            }
                            ui.separator();
                            setting_row(ui,language.text("显示系统通知正文","Show system notification preview"),language.text("可能包含私密内容；屏幕共享时建议关闭","May contain private content; keep it off while sharing your screen"),&mut self.settings.notification_preview);
                            ui.add(egui::Label::new(egui::RichText::new(language.text("仅读取 Windows 通知中心中的可见内容；微信始终通过任务栏闪动识别。应用首次通知后会出现在下方。","Only visible Windows notifications are read; WeChat always uses taskbar activity. Apps appear below after their first notification.")).size(10.5).color(palette.muted)).wrap());
                            let before_apps=self.notification_apps.items.clone();
                            for (_,name,enabled) in &mut self.notification_apps.items {setting_row(ui,name,"",enabled);}
                            if before_apps!=self.notification_apps.items{self.notification_apps.save();}
                        });
                    }
                    8=>{
                        ui.label(egui::RichText::new(language.text("连接邮箱后，可在岛屿上查看新邮件摘要和验证码。","Connect a mailbox to receive summaries and verification codes on the island.")).size(10.5).color(palette.muted));
                        settings_label_with_icon(ui,Icon::Bell,language.text("提醒规则","Alert rules"));
                        settings_card(ui,|ui|{
                            setting_row(ui,language.text("新邮件提醒","New mail alerts"),language.text("新邮件到达时在岛屿上显示；歌词模式改为角标","Show new mail on the island; use a badge while lyrics are visible"),&mut self.settings.mail_enabled);
                            ui.separator();
                            setting_row(ui,language.text("验证码高亮","Verification codes"),language.text("识别后突出显示，点击即可复制","Highlight detected codes and copy them with one click"),&mut self.settings.mail_code);
                            ui.separator();
                            setting_choices(ui,language.text("自动隐藏","Hide after"),&mut self.settings.mail_code_hide_seconds,[30,60,300],language.text("秒","s"));
                            settings_hint(ui,language.text("默认 60 秒；邮件注明更短有效期时会提前隐藏。手动查看后 15 秒重新隐藏。","Default: 60 seconds. Shorter stated lifetimes are respected. Manually revealed codes hide again after 15 seconds."));
                            setting_choices(ui,language.text("补查间隔","Fallback interval"),&mut self.settings.mail_interval,[60,120,300],language.text("秒","s"));
                            settings_hint(ui,language.text("支持推送的邮箱实时收信，并每 10 秒校准一次；此间隔用于不支持推送的邮箱。","Push-capable mailboxes receive mail in real time and reconcile every 10 seconds. This interval applies to fallback polling."));
                        });
                        settings_label_with_icon(ui,Icon::Bell,language.text("邮箱账号","Mailbox accounts"));
                        settings_card(ui,|ui|{
                            if self.mail.accounts.is_empty(){ui.label(egui::RichText::new(language.text("尚未添加邮箱。填写下方配置并测试连接后保存。","No mailbox configured yet. Complete the form below, verify the connection, then save.")).size(10.5).color(palette.muted));}
                            let total=self.mail.accounts.len();
                            let(mut changed,mut remove,mut test)=(false,None,None);
                            for i in 0..total{
                                let(email,provider)=(self.mail.accounts[i].email.clone(),self.mail.accounts[i].provider);
                                let status=self.mail.status_of(&email).label_for(language);
                                let mut on=self.mail.accounts[i].enabled;
                                if i>0{ui.separator();}
                                ui.label(egui::RichText::new(&email).size(12.5).color(palette.text_soft));
                                ui.label(egui::RichText::new(format!("{} · {}",provider.label_for(language),status)).size(10.5).color(palette.muted));
                                ui.horizontal(|ui|{
                                    if ui.add_sized([66.0,28.0],egui::Button::new(language.text("测试","Test"))).clicked(){test=Some(i);}
                                    if ui.add_sized([66.0,28.0],egui::Button::new(language.text("删除","Delete"))).clicked(){remove=Some(i);}
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(language.text("启用","Enabled")).size(10.5).color(palette.text_medium));
                                    if switch(ui,&mut on){self.mail.accounts[i].enabled=on;changed=true;}
                                });
                            }
                            if changed{
                                if let Err(error)=self.mail_accounts_changed(){self.mail.note=error;}
                            }
                            if let Some(i)=remove{
                                let email=self.mail.accounts[i].email.clone();
                                let credential=self.mail.accounts[i].credential_key();
                                self.mail.accounts.remove(i);
                                crate::mail::secrets::delete(&credential);
                                self.mail.status.remove(&email.to_ascii_lowercase());
                                self.mail.note=match self.mail_accounts_changed(){
                                    Ok(())=>if language.is_en(){format!("{email}: removed. Credentials were deleted from Windows Credential Manager.")}else{format!("{email}：已移除，凭据已从系统凭据管理器删除")},
                                    Err(error)=>error,
                                };
                            }
                            if let Some(i)=test{
                                let account=self.mail.accounts[i].clone();
                                match crate::mail::secrets::load(&account.credential_key()){
                                    Ok(pass)=>{self.mail.testing=Some(account.email.clone());self.mail.note=if language.is_en(){format!("{}: testing connection…",account.email)}else{format!("{}：正在测试连接…",account.email)};self.pending.push(Cmd::MailTest(account,pass));}
                                    Err(e)=>self.mail.note=format!("{}：{e}",account.email),
                                }
                            }
                        });
                        settings_label_with_icon(ui,Icon::Bell,language.text("接入邮箱","Mailbox setup"));
                        settings_card(ui,|ui|{
                            settings_control_row(ui,language.text("邮箱服务","Provider"),56.0,|ui|{
                                egui::ComboBox::from_id_salt("mail-provider").width(160.0).selected_text(self.mail.form.provider.label_for(language)).show_ui(ui,|ui|{
                                    for p in crate::mail::Provider::ALL{if ui.selectable_label(self.mail.form.provider==p,p.label_for(language)).clicked(){self.mail.form.provider=p;}}
                                }).response
                            });
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(language.text("准备应用专用密码","Prepare an app password")).size(11.5));
                            ui.add(egui::Label::new(egui::RichText::new(self.mail.form.provider.hint_for(language)).size(10.5).color(palette.muted)).wrap());
                            let (label,url)=self.mail.form.provider.setup_link_for(language);
                            if !url.is_empty() && ui.link(label).clicked(){self.pending.push(Cmd::OpenUrl(url.into()));}
                            ui.add_space(8.0);
                            if self.mail.form.provider==crate::mail::Provider::Custom {
                                settings_control_row(ui,language.text("IMAP 主机","IMAP host"),80.0,|ui|{ui.text_edit_singleline(&mut self.mail.form.custom.host)});
                                settings_control_row(ui,language.text("TLS 端口","TLS port"),80.0,|ui|{ui.add(egui::DragValue::new(&mut self.mail.form.custom.port).range(1..=65535))});
                                settings_control_row(ui,language.text("登录用户名","Username"),80.0,|ui|{ui.add(egui::TextEdit::singleline(&mut self.mail.form.custom.username).vertical_align(egui::Align::Center).hint_text(language.text("留空使用邮箱地址","Leave blank to use the email address")))});
                            }
                            let email_hint=if language.is_en(){format!("Enter an account such as {}",self.mail.form.provider.login_example())}else{format!("输入完整邮箱账号，例如 {}",self.mail.form.provider.login_example())};
                            settings_control_row(ui,language.text("邮箱账号","Email account"),56.0,|ui|{
                                let width=ui.available_width();
                                let response=ui.add_sized([width,30.0],egui::TextEdit::singleline(&mut self.mail.form.email).vertical_align(egui::Align::Center).hint_text(email_hint));
                                if response.changed()&&self.mail.form.provider!=crate::mail::Provider::Custom{
                                    if let Some(provider)=crate::mail::Provider::for_email(&self.mail.form.email).or_else(||crate::mail::Provider::for_web_url(&self.mail.form.email)){self.mail.form.provider=provider;}
                                }
                                response
                            });
                            if let Some(provider)=crate::mail::Provider::for_web_url(&self.mail.form.email) {
                                let guidance=if language.is_en(){format!("Recognized the {} website. Enter the full mailbox account, for example {}.",provider.label_for(language),provider.login_example())}else{format!("已识别为{}网页；请填写完整邮箱账号，例如 {}。",provider.label_for(language),provider.login_example())};
                                settings_hint(ui,&guidance);
                            }
                            settings_control_row(ui,language.text("授权码","App password"),56.0,|ui|{
                                let width=(ui.available_width()-48.0-ui.spacing().item_spacing.x).max(120.0);
                                let show=ui.add_sized([48.0,30.0],egui::Button::new(if self.mail.form.show{language.text("隐藏","Hide")}else{language.text("显示","Show")}));
                                let field=ui.add_sized([width,30.0],egui::TextEdit::singleline(&mut self.mail.form.password).vertical_align(egui::Align::Center).password(!self.mail.form.show).hint_text(language.text("输入邮箱授权码","Enter app password")));
                                if show.clicked(){self.mail.form.show=!self.mail.form.show;}
                                field
                            });
                            ui.add_space(7.0);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                                let save=ui.add_sized([164.0,32.0],egui::Button::new(egui::RichText::new(language.text("保存账号","Save account")).strong().color(palette.accent_text)).fill(palette.accent).stroke(egui::Stroke::new(1.0_f32,palette.line_emphasis)));
                                let test=ui.add_sized([112.0,32.0],egui::Button::new(language.text("检查连接","Check connection")));
                                if test.clicked(){
                                    match mail_form_account(&self.mail.form,language){
                                        Ok((account,password))=>{
                                            self.mail.testing=Some(account.email.clone());
                                            self.mail.note=if language.is_en(){format!("{}: checking connection…",account.email)}else{format!("{}：正在检查连接…",account.email)};
                                            self.pending.push(Cmd::MailTest(account,password));
                                        }
                                        Err(error)=>self.mail.note=error,
                                    }
                                }
                                if save.clicked(){
                                    match mail_form_account(&self.mail.form,language){
                                        Ok((account,password))=>{
                                            match crate::mail::secrets::save(&account.credential_key(),&password){
                                                Ok(())=>{
                                                    let position=self.mail.accounts.iter().position(|stored|stored.email.eq_ignore_ascii_case(&account.email));
                                                    let previous=if let Some(index)=position{
                                                        Some(std::mem::replace(&mut self.mail.accounts[index],account.clone()))
                                                    }else{
                                                        self.mail.accounts.push(account.clone());
                                                        None
                                                    };
                                                    match self.mail_accounts_changed(){
                                                        Ok(())=>{
                                                            self.mail.status.insert(account.email.to_ascii_lowercase(),crate::mail::Status::Connecting);
                                                            self.mail.testing=None;
                                                            self.mail.form.password.clear();
                                                            self.mail.note=if language.is_en(){format!("{}: account saved, checking connection…",account.email)}else{format!("{}：账号已保存，正在检查连接…",account.email)};
                                                        }
                                                        Err(error)=>{
                                                            if let Some(index)=position{
                                                                if let Some(previous)=previous{self.mail.accounts[index]=previous;}
                                                            }else{
                                                                self.mail.accounts.pop();
                                                            }
                                                            crate::mail::secrets::delete(&account.credential_key());
                                                            self.mail.note=error;
                                                        }
                                                    }
                                                }
                                                Err(error)=>self.mail.note=error,
                                            }
                                        }
                                        Err(error)=>self.mail.note=error,
                                    }
                                }
                            });
                            if !self.mail.note.is_empty(){
                                ui.add_space(5.0);
                                egui::Frame::new().fill(palette.surface_selected).corner_radius(6.0).inner_margin(egui::Margin::symmetric(8,6)).show(ui,|ui|{
                                    ui.add(egui::Label::new(egui::RichText::new(&self.mail.note).size(10.5).color(palette.text_soft)).wrap());
                                });
                            }
                            settings_hint(ui,language.text("密码只会写入 Windows 凭据管理器，不会保存到设置文件或账号清单。","The password is written only to Windows Credential Manager, never to settings or the account list."));
                            settings_hint(ui,language.text("图片里的验证码无法自动识别，请到邮箱原邮件查看。","Codes embedded in images cannot be detected automatically; open the source message to view them."));
                        });
                    }
                    _=>{
                        settings_label_with_icon(ui,Icon::Info,crate::product::identity::DISPLAY_NAME);
                        settings_card(ui,|ui|{
                            ui.horizontal(|ui|{
                                ui.label(egui::RichText::new(if language.is_en(){format!("Version {}",env!("CARGO_PKG_VERSION"))}else{format!("版本 {}",env!("CARGO_PKG_VERSION"))}).size(12.0));
                                ui.add_space(8.0);
                                ui.vertical(|ui|self.updates.ui(ui,language));
                            });
                            ui.label(egui::RichText::new(language.text("桌面边缘的音乐、消息与系统信息伴侣。","A music and messaging companion at the edge of your desktop.")).size(10.5).color(palette.muted));
                            ui.horizontal_wrapped(|ui|{
                                if ui.button(language.text("打开本地数据目录","Open local data folder")).clicked(){self.pending.push(Cmd::Open(crate::model::config_dir()));}
                                ui.label(egui::RichText::new(language.text("界面图标：Lucide Icons","Interface icons: Lucide")).size(10.0).color(palette.subtle));
                                ui.hyperlink_to("CC BY 4.0", "https://creativecommons.org/licenses/by/4.0/");
                            });
                        });
                    }
                    }
                });
            });
            self.avoidance.dialog(ctx,&mut self.settings,language);
            if before!=format!("{:?}",self.settings){self.settings.save();}
            ctx.set_style(original_style);
                });
        });
    }

    pub fn check_updates(&mut self, ctx: &egui::Context) {
        self.updates.start_check(ctx);
        self.settings_tab = 9;
        self.open_settings();
    }
}
