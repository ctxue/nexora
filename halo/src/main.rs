#![cfg_attr(windows, windows_subsystem = "windows")]
//! Windows 外壳：无边框透明窗口 + 托盘 + 系统数据，界面全部交给 `nexora_halo::app::App`。

use eframe::egui::{self, Color32, Pos2};
use nexora_halo::app::{self, App, Cmd, Env, Options};
use nexora_halo::poll_clock::PollClock;
use nexora_halo::smtc;
use std::time::{Duration, Instant};

mod diagnostics;
mod graphics;

struct Shell {
    app: App,
    t0: Instant,
    #[cfg(windows)]
    frame_at: Instant,
    media: smtc::Shared,
    seek_rx: Option<std::sync::mpsc::Receiver<bool>>,
    media_control: Option<smtc::ControlWorker>,
    monitor: nexora_halo::monitor::Service,
    _resource_monitor: Option<nexora_halo::resources::Monitor>,
    resource_at: PollClock,
    guard_notified: bool,
    media_enabled: smtc::Poller,
    placed: bool,
    placement_at: PollClock,
    placement_scale: f32,
    /// 上次定位时的目标屏幕与水平位置；变化后立即重新定位，不等下一次轮询。
    placement_key: (String, i32, nexora_halo::model::Dock),
    win_x: f32,
    sys: (Option<u8>, bool, PollClock),
    #[cfg(windows)]
    system_state: nexora_halo::system::Snapshot,
    #[cfg(windows)]
    system_ready: bool,
    #[cfg(windows)]
    system_rx: std::sync::mpsc::Receiver<nexora_halo::system::Observation>,
    fullscreen: Option<nexora_halo::display_layout::Bounds>,
    input_state: nexora_halo::input::Monitor,
    #[cfg(windows)]
    input_keys: Option<nexora_halo::input::Shortcuts>,
    #[cfg(windows)]
    incoming: Option<nexora_halo::system::Notifications>,
    #[cfg(windows)]
    wechat: Option<nexora_halo::wechat::Listener>,
    #[cfg(windows)]
    wechat_retry: Option<Instant>,
    #[cfg(windows)]
    opening_messenger: Option<(app::Unread, Instant)>,
    /// 后台线程正在通过托盘按钮唤起微信。
    #[cfg(windows)]
    tray_opening: Option<(app::Unread, std::sync::mpsc::Receiver<Result<(), String>>)>,
    #[cfg(windows)]
    hidden: bool,
    #[cfg(windows)]
    native: Option<nexora_halo::native::Window>,
    #[cfg(windows)]
    tray: Option<nexora_halo::tray::Tray>,
    #[cfg(windows)]
    autostart: bool,
    /// 蓝牙音频设备（后台线程推送）与音量回声的静默窗口。
    #[cfg(windows)]
    bt_rx: Option<nexora_halo::bt::Service>,
    bt: Vec<nexora_halo::bt::Device>,
    bt_ready: bool,
    volume_quiet: PollClock,
    /// 邮件收信服务（工作线程），以及上次同步给它的账号与开关快照。
    mail: nexora_halo::mail::Service,
    mail_sig: (u32, Vec<nexora_halo::mail::Account>),
}

impl Shell {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        diagnostics::graphics(cc);
        diagnostics::startup("加载本地配置");
        let mut app = App::new(&cc.egui_ctx, Options::default());
        diagnostics::startup("启动剪贴板记录");
        {
            let ctx = cc.egui_ctx.clone();
            nexora_halo::clipboard::watch(app.clipboard.clone(), move || ctx.request_repaint());
        }
        let monitor = nexora_halo::monitor::Service::start(cc.egui_ctx.clone());
        monitor.set_enabled(app.settings.monitor_enabled || app.settings.monitor_alerts);
        if std::env::args().any(|arg| arg == "--settings") {
            app.open_settings();
        }
        diagnostics::startup("初始化媒体和托盘");
        let media = smtc::shared();
        let media_enabled = smtc::spawn_poller(media.clone(), cc.egui_ctx.clone());
        media_enabled.store(
            app.settings.visible_pages[nexora_halo::model::page::MUSIC]
                || app.settings.visible_pages[nexora_halo::model::page::VIDEO],
            std::sync::atomic::Ordering::Relaxed,
        );
        #[cfg(windows)]
        let autostart = nexora_halo::tray::autostart_enabled();
        #[cfg(windows)]
        let tray = nexora_halo::tray::Tray::start(nexora_halo::tray::State {
            pet: app.settings.pet,
            minimal: app.settings.minimal,
            autostart,
            language: app.settings.language,
        });
        diagnostics::startup("初始化通知监听");
        #[cfg(windows)]
        let incoming = if app.settings.read_notifications {
            match nexora_halo::system::connect(cc.egui_ctx.clone()) {
                Ok(rx) => Some(rx),
                Err(e) => {
                    app.listener_status = if app.settings.language.is_en() {
                        format!("Windows could not read notifications: {}", e.code())
                    } else {
                        format!("Windows 无法读取通知：{}", e.code())
                    };
                    None
                }
            }
        } else {
            None
        };
        // 音频、网络与输入法查询由后台完成，避免首帧等待系统服务。
        diagnostics::startup("创建后台采样与窗口");
        let initial_input = None;
        #[cfg(windows)]
        let system_state = nexora_halo::system::Snapshot::default();
        #[cfg(windows)]
        let system_rx =
            nexora_halo::system::observe(cc.egui_ctx.clone(), system_state.clone(), initial_input);
        // 岛体大小只作用于主窗口，在首帧之前生效，避免启动时先按 100% 闪一下。
        #[cfg(windows)]
        eframe::set_root_viewport_scale(app.settings.island_scale());
        Self {
            app,
            t0: Instant::now(),
            #[cfg(windows)]
            frame_at: Instant::now(),
            media,
            seek_rx: None,
            media_control: None,
            monitor,
            _resource_monitor: nexora_halo::resources::Monitor::start(
                nexora_halo::model::config_dir().join("diagnostics"),
            ),
            resource_at: PollClock::default(),
            guard_notified: false,
            media_enabled,
            placed: false,
            placement_at: PollClock::default(),
            placement_scale: 0.0,
            placement_key: (String::new(), 0, nexora_halo::model::Dock::Top),
            win_x: 0.0,
            sys: (None, false, PollClock::default()),
            #[cfg(windows)]
            system_state,
            #[cfg(windows)]
            system_ready: false,
            #[cfg(windows)]
            system_rx,
            fullscreen: None,
            input_state: nexora_halo::input::Monitor::new(initial_input),
            #[cfg(windows)]
            input_keys: nexora_halo::input::Shortcuts::start(),
            #[cfg(windows)]
            incoming,
            #[cfg(windows)]
            wechat: None,
            #[cfg(windows)]
            wechat_retry: None,
            #[cfg(windows)]
            opening_messenger: None,
            #[cfg(windows)]
            tray_opening: None,
            #[cfg(windows)]
            hidden: false,
            #[cfg(windows)]
            native: nexora_halo::native::Window::new(cc),
            #[cfg(windows)]
            tray,
            #[cfg(windows)]
            autostart,
            #[cfg(windows)]
            bt_rx: None,
            bt: Vec::new(),
            bt_ready: false,
            volume_quiet: PollClock::default(),
            mail: nexora_halo::mail::Service::start(),
            mail_sig: (0, Vec::new()),
        }
    }

    #[cfg(windows)]
    fn poll_tray(&mut self) {
        use nexora_halo::tray::{self, Event};
        let language = self.app.settings.language;
        let events = self.tray.as_ref().map(|t| t.poll()).unwrap_or_default();
        for e in events {
            match e {
                Event::OpenSettings => self.app.open_settings(),
                Event::TogglePet => {
                    self.app.set_pet(!self.app.settings.pet);
                    self.app.say(if self.app.settings.pet {
                        language.text("岛屿已打开", "Island enabled")
                    } else {
                        language.text("岛屿已关闭", "Island disabled")
                    });
                }
                Event::ToggleMinimal => {
                    self.app.set_minimal(!self.app.settings.minimal);
                    self.app.say(if self.app.settings.minimal {
                        language.text("极简刘海已打开", "Minimal island enabled")
                    } else {
                        language.text("极简刘海已关闭", "Minimal island disabled")
                    });
                }
                Event::ToggleAutostart => {
                    let want = !self.autostart;
                    match tray::set_autostart(want) {
                        Ok(()) => {
                            self.autostart = want;
                            self.app.say(if want {
                                language.text("已设为开机启动", "Launch at startup enabled")
                            } else {
                                language.text("已取消开机启动", "Launch at startup disabled")
                            });
                        }
                        Err(e) => self.app.say(e),
                    }
                }
            }
        }
        if let Some(t) = &self.tray {
            t.sync(tray::State {
                pet: self.app.settings.pet,
                minimal: self.app.settings.minimal,
                autostart: self.autostart,
                language: self.app.settings.language,
            });
        }
    }

    fn env(&mut self, ctx: &egui::Context) -> Env {
        let language = self.app.settings.language;
        let clock = local_now();
        // 电量变化很慢，每 5 秒读一次就够
        if self.sys.2.ready(Duration::from_secs(5)) {
            let (b, c) = read_power();
            if self.app.settings.show_battery
                && self.sys.0.is_some()
                && self.sys.1 != c
                && self.app.settings.system_hud
            {
                self.app.alert(
                    if c {
                        language.text("电源已连接", "Power connected")
                    } else {
                        language.text("已切换电池供电", "Running on battery")
                    },
                    b.map(|v| {
                        if language.is_en() {
                            format!("Battery level {v}%")
                        } else {
                            format!("当前电量 {v}%")
                        }
                    })
                    .unwrap_or_else(|| {
                        language
                            .text("桌面电源状态已更新", "Desktop power status changed")
                            .into()
                    }),
                    5,
                    false,
                );
            }
            if self.app.settings.show_battery
                && b.is_some_and(|v| v <= 20)
                && !c
                && self.sys.0.is_some_and(|v| v > 20)
            {
                self.app.alert(
                    language.text("电量偏低", "Low battery"),
                    if language.is_en() {
                        format!("{}% remaining · Connect a charger", b.unwrap())
                    } else {
                        format!("剩余 {}% · 请连接电源", b.unwrap())
                    },
                    5,
                    true,
                );
            }
            self.sys = (b, c, PollClock::now());
        }
        let ppp = ctx.pixels_per_point() * eframe::root_frame_paint_scale();
        let origin = ctx
            .input(|i| i.viewport().outer_rect)
            .map(|r| r.min)
            .unwrap_or(Pos2::new(self.win_x, 0.0));
        #[cfg(windows)]
        let origin = self
            .native
            .as_ref()
            .and_then(|w| w.logical_origin(ppp))
            .unwrap_or(origin);
        let cursor = cursor_logical(ppp).map(|(x, y)| Pos2::new(x - origin.x, y - origin.y));
        // 所在屏幕换算到窗口逻辑坐标，供通知详情等浮层避开屏幕边缘。
        #[cfg(windows)]
        let screen = nexora_halo::display_layout::selected(
            &self.app.displays,
            &self.app.settings.display_id,
        )
        .map(|d| {
            let b = d.bounds;
            egui::Rect::from_min_max(
                Pos2::new(
                    b.left as f32 / ppp - origin.x,
                    b.top as f32 / ppp - origin.y,
                ),
                Pos2::new(
                    b.right as f32 / ppp - origin.x,
                    b.bottom as f32 / ppp - origin.y,
                ),
            )
        });
        #[cfg(not(windows))]
        let screen = None;
        Env {
            t: self.t0.elapsed().as_secs_f64(),
            clock,
            battery: self.sys.0,
            charging: self.sys.1,
            network: Some(self.system_state.network),
            cursor,
            browser_at_top: {
                #[cfg(windows)]
                {
                    if self.app.settings.browser_minimal {
                        self.native
                            .as_mut()
                            .and_then(|native| native.browser_at_top(&self.app.settings))
                    } else {
                        None
                    }
                }
                #[cfg(not(windows))]
                {
                    None
                }
            },
            media: self.media.lock().map(|m| m.clone()).unwrap_or_default(),
            volume: self.system_state.volume,
            bt: self.bt.clone(),
            bt_ready: self.bt_ready,
            screen,
            opaque_surface: {
                #[cfg(windows)]
                {
                    self.native.as_ref().is_some_and(|w| w.opaque_surface())
                }
                #[cfg(not(windows))]
                {
                    false
                }
            },
        }
    }
}

impl eframe::App for Shell {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let language = self.app.settings.language;
        if self.resource_at.ready(Duration::from_secs(30)) {
            let (count, bytes) = {
                let textures = ctx.tex_manager();
                let textures = textures.read();
                (
                    textures.num_allocated(),
                    textures
                        .allocated()
                        .map(|(_, info)| info.bytes_used())
                        .sum(),
                )
            };
            nexora_halo::resources::TEXTURES.store(count, std::sync::atomic::Ordering::Relaxed);
            nexora_halo::resources::TEXTURE_BYTES
                .store(bytes, std::sync::atomic::Ordering::Relaxed);
            nexora_halo::resources::GALLEYS.store(
                ctx.fonts(|fonts| fonts.num_galleys_in_cache()),
                std::sync::atomic::Ordering::Relaxed,
            );
            self.resource_at = PollClock::now();
        }
        let guarded = nexora_halo::resources::GUARD.tripped();
        if guarded && !self.guard_notified {
            // 资源保护熔断：停止音乐会话轮询（含封面、网易云日志）与歌词查询，保住用户电脑，重启后恢复。
            self.guard_notified = true;
            self.app.opts.fetch_lyrics = false;
            self.app.alert(
                language.text("HALO 已暂停音乐功能", "HALO paused music features"),
                language.text(
                    "检测到资源占用异常，已暂停读取音乐信息和歌词。重启 NEXORA HALO 即可恢复；诊断记录在数据目录的 diagnostics 文件夹。",
                    "A resource issue paused music metadata and lyrics. Restart NEXORA HALO to recover. Diagnostics are stored in the diagnostics folder under your data directory.",
                ),
                0,
                false,
            );
        }
        self.media_enabled.store(
            (self.app.settings.visible_pages[nexora_halo::model::page::MUSIC]
                || self.app.settings.visible_pages[nexora_halo::model::page::VIDEO])
                && !guarded,
            std::sync::atomic::Ordering::Relaxed,
        );
        // 设置里改了岛体大小：下一帧起主窗口按新倍率栅格化，定位逻辑随像素比变化同步调整画布。
        #[cfg(windows)]
        eframe::share_root_font_scale(self.app.settings_open);
        #[cfg(windows)]
        eframe::set_root_viewport_scale(self.app.settings.island_scale());
        #[cfg(windows)]
        {
            // 用帧间隔限制刷新，绕过部分 Intel 驱动在垂直同步中的忙等待。
            let delay = Duration::from_secs_f64(1.0 / 60.0).saturating_sub(self.frame_at.elapsed());
            if !delay.is_zero() {
                std::thread::sleep(delay);
            }
            self.frame_at = Instant::now();
        }
        #[cfg(windows)]
        self.poll_tray();
        #[cfg(windows)]
        let foreground_changed = nexora_halo::native::take_foreground_change();
        #[cfg(windows)]
        if foreground_changed {
            // 切到别的程序：收起，并在光标离开前不再自动展开
            self.app.foreground_changed();
        }
        #[cfg(windows)]
        let placement_key = (
            self.app.settings.display_id.clone(),
            self.app.settings.island_offset,
            self.app.settings.dock,
        );
        #[cfg(windows)]
        if !self.placed
            || self.placement_scale != (ctx.pixels_per_point() * eframe::root_frame_paint_scale())
            || self.placement_key != placement_key
            || self.placement_at.ready(Duration::from_secs(1))
        {
            self.placement_at = PollClock::now();
            self.placement_scale = ctx.pixels_per_point() * eframe::root_frame_paint_scale();
            self.placement_key = placement_key;
            self.app.displays = nexora_halo::display_layout::connected();
            let display = nexora_halo::display_layout::selected(
                &self.app.displays,
                &self.app.settings.display_id,
            );
            let offset = self.app.settings.island_offset_ratio();
            if display.is_some_and(|d| {
                self.native.as_ref().is_some_and(|w| {
                    w.place_on_display(
                        self.placement_scale,
                        d.bounds,
                        offset,
                        self.app.settings.dock,
                    )
                })
            }) {
                if !self.placed {
                    diagnostics::startup("目标屏幕物理坐标定位完成，已请求显示");
                }
                self.placed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            } else if self.t0.elapsed() >= Duration::from_secs(2) {
                // Do not leave a hidden startup window forever when monitor discovery fails.
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            }
        }
        #[cfg(not(windows))]
        if !self.placed {
            if let Some(mon) = ctx.input(|i| i.viewport().monitor_size) {
                self.placed = true;
                self.win_x = ((mon.x - app::WIN_W) / 2.0).round();
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    self.win_x, 0.0,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            }
        }
        #[cfg(windows)]
        {
            while let Ok(observation) = self.system_rx.try_recv() {
                let next = observation.input;
                let manual = next.is_some_and(|state| {
                    self.input_keys
                        .as_ref()
                        .is_some_and(|keys| keys.allows(state.context))
                });
                if let Some(label) = self.input_state.observe(next, manual) {
                    if let Some(keys) = &self.input_keys {
                        keys.clear();
                    }
                    if self.app.settings.system_hud {
                        self.app.alert(
                            language.text("键盘输入语言", "Keyboard input language"),
                            label,
                            5,
                            false,
                        );
                    }
                }
                let new = observation.system;
                // 切换输出设备时，系统会恢复该设备的音量；等待它稳定后再提示调节。
                if new.audio_endpoint != self.system_state.audio_endpoint {
                    self.volume_quiet = PollClock::now();
                }
                if self.system_ready && self.app.settings.system_hud {
                    for (title, body) in new.changes_for(&self.system_state, language) {
                        // 拖动音量条刚改过音量时，系统回声不再单独弹一条。
                        if title == language.text("系统音量", "System volume")
                            && !self.volume_quiet.ready(Duration::from_millis(900))
                        {
                            continue;
                        }
                        self.app.alert(title, body, 5, false);
                    }
                }
                self.system_state = new;
                self.system_ready = true;
                self.fullscreen = observation.fullscreen;
            }
            self.hidden = nexora_halo::display_layout::hide_for_fullscreen(
                self.app.settings.hide_fullscreen,
                self.fullscreen,
                nexora_halo::display_layout::selected(
                    &self.app.displays,
                    &self.app.settings.display_id,
                )
                .map(|d| d.bounds),
            );
            if self.app.settings.bluetooth_notifications {
                if self.bt_rx.is_none() {
                    self.bt_rx = Some(nexora_halo::bt::spawn());
                }
            } else {
                self.bt_rx = None;
                self.bt.clear();
                self.bt_ready = false;
            }
            if let Some(service) = &self.bt_rx {
                while let Ok(found) = service.rx.try_recv() {
                    self.bt = found;
                    self.bt_ready = true;
                }
            }
            let wechat_enabled =
                self.app.settings.wechat_notifications && self.app.settings.notifications;
            if self
                .wechat
                .as_ref()
                .is_some_and(|listener| !listener.registered())
            {
                self.wechat = None;
            }
            if !wechat_enabled {
                self.wechat = None;
                self.wechat_retry = None;
                self.app.wechat_status.clear();
            } else if self.wechat.is_none()
                && self
                    .wechat_retry
                    .is_none_or(|at| at.elapsed() >= Duration::from_secs(10))
            {
                match nexora_halo::wechat::Listener::new(ctx.clone()) {
                    Ok(listener) => {
                        self.wechat = Some(listener);
                        self.wechat_retry = None;
                        self.app.wechat_status = language
                            .text(
                                "正在监听 · 微信任务栏闪动时提醒",
                                "Listening · Alerts when WeChat flashes on the taskbar",
                            )
                            .into();
                    }
                    Err(error) => {
                        self.wechat_retry = Some(Instant::now());
                        self.app.wechat_status = if language.is_en() {
                            format!("{error} · Retrying in 10 seconds")
                        } else {
                            format!("{error} · 10 秒后重试")
                        };
                    }
                }
            }
            let previous_pending = self.app.pending_messengers.len();
            for (pid, window) in self
                .wechat
                .as_ref()
                .map(|listener| listener.take())
                .unwrap_or_default()
            {
                nexora_halo::wechat::trace(format!("岛上显示微信未读 账号 {pid}"));
                self.app.receive_wechat_reminder_from(pid, window);
            }
            if let Some(rx) = &self.incoming {
                while let Ok(event) = rx.try_recv() {
                    match event {
                        nexora_halo::system::Incoming::Revoked => {
                            self.app.settings.read_notifications = false;
                            self.app.settings.save();
                        }
                        nexora_halo::system::Incoming::Status(s) => self.app.listener_status = s,
                        nexora_halo::system::Incoming::Message(a, id, b) => {
                            self.app.receive_notification(a, id, b)
                        }
                    }
                }
            }
            if foreground_changed || self.app.pending_messengers.len() != previous_pending {
                // 在微信 / QQ 里打开了某个账号的窗口：清掉这个账号的未读。
                let mut sources: Vec<app::Messenger> = self
                    .app
                    .pending_messengers
                    .iter()
                    .map(|u| u.source)
                    .collect();
                sources.dedup();
                match nexora_halo::wechat::foreground_client(&sources) {
                    Some((source, pid)) => {
                        if self
                            .app
                            .pending_messengers
                            .iter()
                            .any(|u| u.source == source)
                        {
                            nexora_halo::wechat::trace(format!(
                                "岛上清除 {} 账号 {pid}（前台切换={foreground_changed}）",
                                source.name()
                            ));
                        }
                        self.app.viewed_client(source, pid);
                    }
                    None => {
                        if foreground_changed && !sources.is_empty() {
                            nexora_halo::wechat::trace(format!(
                                "前台切换但不是消息客户端主窗口：{}；岛上未读 {:?}",
                                nexora_halo::wechat::describe_foreground(),
                                self.app
                                    .pending_messengers
                                    .iter()
                                    .map(|u| (u.source.name(), u.pid))
                                    .collect::<Vec<_>>()
                            ));
                        }
                    }
                }
            }
            if let Some((unread, rx)) = &self.tray_opening {
                let unread = *unread;
                match rx.try_recv() {
                    Ok(Ok(())) => {
                        self.tray_opening = None;
                        self.opening_messenger = Some((unread, Instant::now()));
                        ctx.request_repaint_after(Duration::from_secs(4));
                    }
                    Ok(Err(error)) => {
                        self.tray_opening = None;
                        self.app.say(error);
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.tray_opening = None;
                        self.app.say(language.text(
                            "未能唤起微信，请从托盘打开",
                            "Could not open WeChat. Open it from the tray.",
                        ));
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
            }
            if let Some((unread, started)) = self.opening_messenger {
                if !self.app.pending_messengers.contains(&unread) {
                    self.opening_messenger = None;
                } else if started.elapsed() >= Duration::from_secs(4) {
                    self.opening_messenger = None;
                    match nexora_halo::wechat::foreground_client(&[unread.source]) {
                        Some((source, pid)) => self.app.viewed_client(source, pid),
                        None => self.app.say(if language.is_en() {
                            format!(
                                "{} did not come to the foreground. Click the icon to retry.",
                                unread.source.name_for(language)
                            )
                        } else {
                            format!(
                                "{}尚未切到前台，可点图标重试",
                                unread.source.name_for(language)
                            )
                        }),
                    }
                }
            }
        }
        if let Some(rx) = &self.seek_rx {
            if let Some(success) = match rx.try_recv() {
                Ok(success) => Some(success),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(false),
                Err(_) => None,
            } {
                self.seek_rx = None;
                if !success {
                    self.app.say(language.text(
                        "播放器未完成跳转，请打开客户端后重试",
                        "The player did not seek. Open the player and try again.",
                    ));
                }
            }
        }
        let env = self.env(ctx);
        self.monitor
            .set_enabled(self.app.settings.monitor_enabled || self.app.settings.monitor_alerts);
        while let Some(snapshot) = self.monitor.try_recv() {
            self.app.monitor_snapshot(snapshot);
        }
        // 开关、间隔或账号清单变化后重新对齐收信线程，再把事件交给界面。
        let sig = (
            self.app.settings.mail_interval,
            self.app.mail_accounts().to_vec(),
        );
        if sig != self.mail_sig {
            self.mail.sync(&sig.1, sig.0);
            self.mail_sig = sig;
        }
        while let Some(ev) = self.mail.try_recv() {
            self.app.mail_event(ev);
        }
        let mut out = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                // 左右停靠时画布是竖向的 SIDE_W × SIDE_H。
                let size = if self.app.settings.dock.is_side() {
                    egui::vec2(
                        nexora_halo::side_dock::SIDE_W,
                        nexora_halo::side_dock::SIDE_H,
                    )
                } else {
                    egui::vec2(app::WIN_W, app::WIN_H)
                };
                let win = egui::Rect::from_min_size(ui.max_rect().min, size);
                out = Some(self.app.frame(ui, win, &env));
            });
        let Some(out) = out else { return };

        #[cfg(windows)]
        if let Some(native) = &mut self.native {
            let r = |r: egui::Rect| (r.left(), r.top(), r.width(), r.height());
            // 保留主窗口的事件循环，避免全屏隐藏后设置窗口无法关闭或重新打开。
            let region = if self.hidden {
                (0.0, 0.0, 0.0, 0.0)
            } else {
                r(out.region)
            };
            let hit = if self.hidden {
                (0.0, 0.0, 0.0, 0.0)
            } else {
                r(out.hit)
            };
            native.update(
                region,
                hit,
                if self.hidden {
                    None
                } else {
                    out.notice_hit.map(r)
                },
                if self.hidden {
                    None
                } else {
                    out.file_drop_zone.map(r)
                },
                if self.hidden {
                    None
                } else {
                    out.window_shape.as_deref()
                },
                ctx.pixels_per_point() * eframe::root_frame_paint_scale(),
                out.editing,
                !self.hidden && out.expanded,
            );
            if out.raise_window {
                native.raise_topmost();
            }
        }
        for cmd in out.cmds {
            match cmd {
                Cmd::Media(t) => {
                    let source = self
                        .media
                        .lock()
                        .map(|media| media.source.clone())
                        .unwrap_or_default();
                    if self.media_control.is_none() {
                        self.media_control = smtc::ControlWorker::start(ctx.clone());
                    }
                    match self
                        .media_control
                        .as_ref()
                        .ok_or(())
                        .and_then(|worker| worker.submit(t, source))
                    {
                        Ok(Some(rx)) => self.seek_rx = Some(rx),
                        Ok(None) => {}
                        Err(()) => self.app.say(language.text(
                            "播放器正在响应，请稍后重试",
                            "The player is still responding. Try again shortly.",
                        )),
                    }
                }
                Cmd::FocusMedia(source) => {
                    if let Err(error) = smtc::activate_source(&source) {
                        self.app.say(if language.is_en() {
                            format!("Could not return to the browser: {error}")
                        } else {
                            format!("暂时无法切回浏览器：{error}")
                        });
                    }
                }
                Cmd::ViewMessenger(unread) => {
                    #[cfg(windows)]
                    match nexora_halo::wechat::open_unread(unread) {
                        Ok(()) => match nexora_halo::wechat::foreground_client(&[unread.source]) {
                            Some((source, pid)) => self.app.viewed_client(source, pid),
                            None => {
                                self.opening_messenger = Some((unread, Instant::now()));
                                ctx.request_repaint_after(Duration::from_secs(4));
                            }
                        },
                        Err(_)
                            if self.app.settings.wechat_tray_open
                                && unread.source == app::Messenger::Wechat
                                && self.tray_opening.is_none() =>
                        {
                            match nexora_halo::wechat::wechat_in_tray(unread) {
                                Ok(()) => {
                                    // 缩在托盘：后台线程按下该账号的托盘按钮，不阻塞界面。
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    let repaint = ctx.clone();
                                    std::thread::spawn(move || {
                                        let _ = tx.send(
                                            nexora_halo::wechat::open_wechat_from_tray(unread),
                                        );
                                        repaint.request_repaint();
                                    });
                                    self.tray_opening = Some((unread, rx));
                                }
                                Err(error) => self.app.say(error),
                            }
                        }
                        Err(error) => self.app.say(error),
                    }
                }
                Cmd::Open(p) => {
                    if let Err(e) = open_path(&p) {
                        self.app.say(e);
                    }
                }
                Cmd::OpenUrl(url) => {
                    if let Some(url) = nexora_halo::mail::web_url(&url) {
                        if open_path(std::path::Path::new(&url)).is_err() {
                            self.app.say(language.text(
                                "无法打开默认浏览器，请检查 Windows 默认应用设置",
                                "Could not open the default browser. Check your Windows default apps.",
                            ));
                        }
                    }
                }
                Cmd::AddFiles => {
                    #[cfg(windows)]
                    match nexora_halo::system::pick_files(
                        self.native.as_ref().map(nexora_halo::native::Window::hwnd),
                    ) {
                        Ok(files) => {
                            if !files.is_empty() {
                                self.app.on_drop(files);
                                self.app.show_page(nexora_halo::model::page::SHELF);
                            }
                        }
                        Err(e) => self.app.say(if language.is_en() {
                            format!("Could not choose files: {}", e.code())
                        } else {
                            format!("无法选择文件：{}", e.code())
                        }),
                    }
                }
                Cmd::DragFile(p) => {
                    #[cfg(windows)]
                    if let Err(e) = nexora_halo::system::drag_file(&p) {
                        self.app.say(if language.is_en() {
                            format!("Could not drag the file: {}", e.code())
                        } else {
                            format!("无法拖出文件：{}", e.code())
                        });
                    }
                    ctx.input_mut(|i| i.pointer = Default::default());
                }
                Cmd::DisconnectNotifications => {
                    #[cfg(windows)]
                    {
                        self.incoming = None;
                        self.app.settings.read_notifications = false;
                        self.app.settings.save();
                        self.app.listener_status = language
                            .text(
                                "已断开，不再读取 Windows 通知",
                                "Disconnected. Windows notifications are no longer read.",
                            )
                            .into();
                    }
                }
                Cmd::Reveal(p) => {
                    let _ = std::process::Command::new("explorer.exe")
                        .arg(format!("/select,{}", p.display()))
                        .spawn();
                }
                Cmd::ToggleAutostart => {
                    #[cfg(windows)]
                    {
                        let want = !self.autostart;
                        match nexora_halo::tray::set_autostart(want) {
                            Ok(()) => self.autostart = want,
                            Err(e) => self.app.say(e),
                        }
                    }
                }
                Cmd::SetVolume(v) => {
                    #[cfg(windows)]
                    nexora_halo::system::set_volume(v);
                    self.volume_quiet = PollClock::now();
                }
                Cmd::ToggleMute => {
                    #[cfg(windows)]
                    nexora_halo::system::toggle_mute();
                    self.volume_quiet = PollClock::now();
                }
                Cmd::Copy(text) => {
                    ctx.copy_text(text);
                    self.app.say(language.text("已复制", "Copied"));
                }
                Cmd::Notify(title, body) =>
                {
                    #[cfg(windows)]
                    if let Some(tray) = &self.tray {
                        tray.notify(&title, &body, self.app.settings.sound);
                    }
                }
                Cmd::RequestNotifications => {
                    #[cfg(windows)]
                    {
                        self.app.settings.read_notifications = true;
                        self.app.settings.save();
                        self.incoming = None;
                        match nexora_halo::system::connect(ctx.clone()) {
                            Ok(rx) => {
                                self.incoming = Some(rx);
                                self.app.listener_status = language
                                    .text(
                                        "正在请求 Windows 通知权限…",
                                        "Requesting Windows notification access…",
                                    )
                                    .into();
                            }
                            Err(e) => {
                                self.app.listener_status = if language.is_en() {
                                    format!(
                                        "Windows did not allow notification access: {}. The required system permission or app identity is missing.",
                                        e.code()
                                    )
                                } else {
                                    format!(
                                        "Windows 未允许读取通知：{}。需要相应的系统权限或应用身份。",
                                        e.code()
                                    )
                                }
                            }
                        }
                    }
                }
                Cmd::EnableWechatAlerts => {
                    self.app.settings.wechat_notifications = true;
                    self.app.settings.notifications = true;
                    self.app.settings.save();
                    self.app
                        .say(language.text("微信提醒已开启", "WeChat alerts enabled"));
                }
                Cmd::MailSync => {
                    let accounts = self.app.mail_accounts().to_vec();
                    self.mail.sync(&accounts, self.app.settings.mail_interval);
                    self.mail_sig = (
                        self.app.settings.mail_interval,
                        self.app.mail_accounts().to_vec(),
                    );
                }
                Cmd::MailTest(account, password) => self.mail.test(account, password),
            }
        }

        #[cfg(windows)]
        let autostart = self.autostart;
        #[cfg(not(windows))]
        let autostart = false;
        self.app.settings_ui(ctx, autostart);
        self.app.mail_reader_ui(ctx);
        #[cfg(windows)]
        let repaint_after = if self.hidden {
            Duration::from_secs(1)
        } else {
            out.repaint_after
        };
        #[cfg(not(windows))]
        let repaint_after = out.repaint_after;
        schedule_repaint(ctx, repaint_after);
    }
}

// ---------------- 系统信息 ----------------

fn schedule_repaint(ctx: &egui::Context, interval: Duration) {
    // egui 会预减一帧；本程序自行调度刷新，补回后才能保持实际的等待间隔。
    let predicted = ctx.input(|i| Duration::from_secs_f32(i.predicted_dt));
    ctx.request_repaint_after(interval.saturating_add(predicted));
}

#[cfg(test)]
mod repaint_tests {
    use super::*;

    fn scheduled_delay(interval: Duration, immediate: bool) -> Duration {
        let ctx = egui::Context::default();
        let mut delay = Duration::ZERO;
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    time: Some(frame as f64),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    if immediate {
                        ctx.request_repaint();
                    }
                    schedule_repaint(ctx, interval);
                },
            );
            delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        }
        delay
    }

    #[test]
    fn background_repaint_keeps_the_requested_frame_interval() {
        for fps in [60.0, 30.0, 20.0, 10.0, 1.0] {
            let interval = Duration::from_secs_f64(1.0 / fps);
            let actual = scheduled_delay(interval, false);
            assert!(
                actual.abs_diff(interval) < Duration::from_micros(1),
                "目标 {fps} 帧不应被提前唤醒：{actual:?}"
            );
        }
    }

    #[test]
    fn interaction_and_service_events_still_repaint_immediately() {
        assert_eq!(
            scheduled_delay(Duration::from_secs(1), true),
            Duration::ZERO
        );
    }
}

#[cfg(windows)]
fn local_now() -> String {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    let st = unsafe { GetLocalTime() };
    format!("{:02}:{:02}:{:02}", st.wHour, st.wMinute, st.wSecond)
}

#[cfg(not(windows))]
fn local_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

#[cfg(windows)]
fn read_power() -> (Option<u8>, bool) {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut st = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut st) }.is_ok() {
        let pct = if st.BatteryLifePercent == 255 {
            None
        } else {
            Some(st.BatteryLifePercent)
        };
        (pct, st.ACLineStatus == 1)
    } else {
        (None, false)
    }
}

#[cfg(not(windows))]
fn read_power() -> (Option<u8>, bool) {
    (None, true)
}

#[cfg(windows)]
fn cursor_logical(ppp: f32) -> Option<(f32, f32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p) }.ok()?;
    Some((p.x as f32 / ppp, p.y as f32 / ppp))
}

#[cfg(not(windows))]
fn cursor_logical(_ppp: f32) -> Option<(f32, f32)> {
    None
}

#[cfg(windows)]
fn open_path(p: &std::path::Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let mut wide: Vec<u16> = p.as_os_str().encode_wide().collect();
    wide.push(0);
    let op: Vec<u16> = "open\0".encode_utf16().collect();
    unsafe {
        let result = ShellExecuteW(
            None,
            PCWSTR(op.as_ptr()),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        if result.0 as usize <= 32 {
            return Err("无法打开，请检查文件位置和默认应用".into());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn open_path(_p: &std::path::Path) -> Result<(), String> {
    Ok(())
}

fn main() -> std::process::ExitCode {
    diagnostics::install();
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            diagnostics::report("NEXORA HALO 启动或运行失败。", &error.to_string());
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> eframe::Result<()> {
    // 单实例互斥体
    #[cfg(windows)]
    if !nexora_halo::tray::single_instance() {
        if !nexora_halo::tray::activate_existing() {
            diagnostics::report("NEXORA HALO 已在运行，但暂时无法打开窗口。\n请稍后重试；若仍无法打开，请在任务管理器的“详细信息”中结束 nexora-halo.exe 后重试。", "已有实例可能正在启动或已无响应。请稍后重试；若仍无法打开，请在任务管理器的“详细信息”中结束 nexora-halo.exe，再重新启动。");
        }
        return Ok(());
    }
    diagnostics::startup("初始化图形窗口");
    let viewport = egui::ViewportBuilder::default()
        .with_title(nexora_halo::product::identity::WINDOW_TITLE)
        .with_icon(nexora_halo::theme::app_icon())
        .with_inner_size([app::CANVAS_W, app::WIN_H])
        .with_decorations(false)
        .with_transparent(true)
        .with_active(false)
        .with_resizable(false)
        .with_taskbar(false)
        .with_always_on_top()
        .with_visible(false);
    let opts = eframe::NativeOptions {
        viewport,
        vsync: !cfg!(windows),
        ..Default::default()
    };
    graphics::run(opts, |opts| {
        eframe::run_native(
            nexora_halo::product::identity::WINDOW_TITLE,
            opts,
            Box::new(|cc| Ok(Box::new(Shell::new(cc)))),
        )
    })
}
