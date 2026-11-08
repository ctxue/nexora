use crate::app::{App, Cmd, Env, CONTENT_LEFT, CONTENT_WIDTH};
use crate::lucide::{self, Icon};
use crate::monitor::{GroupUsage, Metric};
use crate::paint::{Card, H};
use crate::theme::{self, W};
use chrono::TimeZone;
use eframe::egui::{self, Color32, Pos2, Rect};

const SURFACE: Color32 = Color32::from_rgb(24, 24, 28);
const ACTIVE: Color32 = Color32::from_rgb(151, 162, 255);
const ACTIVE_SOFT: Color32 = Color32::from_rgba_premultiplied(40, 45, 92, 180);
const MONITOR_GAP: f32 = 8.0;
const PROCESS_PANEL_WIDTH: f32 = 108.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct MonitorPageLayout {
    main: Rect,
    processes: Rect,
}

fn monitor_page_layout() -> MonitorPageLayout {
    let main_width = CONTENT_WIDTH - PROCESS_PANEL_WIDTH - MONITOR_GAP;
    MonitorPageLayout {
        main: Rect::from_min_size(Pos2::new(CONTENT_LEFT, 40.0), egui::vec2(main_width, 184.0)),
        processes: Rect::from_min_size(
            Pos2::new(CONTENT_LEFT + main_width + MONITOR_GAP, 40.0),
            egui::vec2(PROCESS_PANEL_WIDTH, 184.0),
        ),
    }
}

impl App {
    pub(crate) fn monitor_metric_tabs(&mut self, c: &Card) {
        let tabs = [
            (Metric::Cpu, Icon::Cpu),
            (Metric::Memory, Icon::MemoryStick),
            (Metric::Disk, Icon::HardDrive),
            (Metric::Gpu, Icon::Gauge),
        ];
        for (index, (metric, icon)) in tabs.into_iter().enumerate() {
            let x = 46.0 + index as f32 * 26.0;
            let selected = self.monitor_metric == metric;
            let (zh, en) = metric.label();
            let response = c
                .hit(x - 11.0, 8.0, 25.0, 27.0, ("monitor-icon-tab", index))
                .on_hover_text(self.settings.language.text(zh, en));
            if selected {
                c.fill(x - 11.0, 8.0, 25.0, 27.0, 7.0, ACTIVE_SOFT);
            }
            lucide::card(
                c,
                icon,
                x + 1.0,
                21.5,
                14.0,
                if selected { theme::WHITE } else { theme::GRAY },
            );
            if response.clicked() {
                self.monitor_metric = metric;
            }
        }
    }

    pub(crate) fn page_monitor(&mut self, c: &Card, _env: &Env, _cmds: &mut Vec<Cmd>) {
        let language = self.settings.language;
        if self.monitor_show_events {
            self.monitor_events_view(c);
            return;
        }
        if self.monitor_latest.sample.at_secs <= 0.0 {
            c.fill(CONTENT_LEFT, 40.0, CONTENT_WIDTH, 184.0, 11.0, SURFACE);
            c.text(
                209.5,
                138.0,
                language.text("正在读取系统数据…", "Reading system metrics…"),
                12.0,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            return;
        }
        self.monitor_selected_card(c);
        self.monitor_processes(c);
    }

    fn monitor_icon(&self, metric: Metric) -> Icon {
        match metric {
            Metric::Cpu => Icon::Cpu,
            Metric::Memory => Icon::MemoryStick,
            Metric::Disk => Icon::HardDrive,
            Metric::Gpu => Icon::Gauge,
        }
    }

    fn monitor_selected_card(&mut self, c: &Card) {
        let language = self.settings.language;
        let panel = monitor_page_layout().main;
        c.painter.rect_filled(
            c.rect(panel.left(), panel.top(), panel.width(), panel.height()),
            10.0,
            SURFACE,
        );
        let metric = self.monitor_metric;
        let (zh, en) = metric.label();
        c.circle(
            panel.left() + 17.0,
            panel.top() + 20.0,
            10.0,
            Color32::from_gray(38),
        );
        lucide::card(
            c,
            self.monitor_icon(metric),
            panel.left() + 17.0,
            panel.top() + 20.0,
            12.0,
            theme::WHITE,
        );
        c.text(
            panel.left() + 33.0,
            panel.top() + 16.0,
            language.text(zh, en),
            8.0,
            W::Regular,
            theme::DIM,
            H::Left,
        );
        let value = metric
            .value(&self.monitor_latest.sample)
            .map(|value| format!("{value:.0}%"))
            .unwrap_or_else(|| "--".into());
        c.text(
            panel.left() + 33.0,
            panel.top() + 33.0,
            &value,
            18.0,
            W::Bold,
            theme::WHITE,
            H::Left,
        );

        let spark = Rect::from_min_max(
            c.pos(panel.left() + 86.0, panel.top() + 12.0),
            c.pos(panel.right() - 8.0, panel.top() + 38.0),
        );
        let samples = self.monitor_history.window(300.0);
        let values: Vec<_> = samples
            .iter()
            .filter_map(|sample| metric.value(sample))
            .map(|value| value.clamp(0.0, 100.0))
            .collect();
        draw_line(c, spark, &values, ACTIVE, 1.25, false);

        let plot = Rect::from_min_max(
            c.pos(panel.left() + 8.0, panel.top() + 48.0),
            c.pos(panel.right() - 8.0, panel.top() + 121.0),
        );
        for fraction in [0.25f32, 0.5, 0.75] {
            let y = plot.bottom() - plot.height() * fraction;
            c.painter.line_segment(
                [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                egui::Stroke::new(0.7_f32, Color32::from_white_alpha(12)),
            );
        }
        draw_history_bars(c, plot, &values, ACTIVE);
        let memory_y = panel.top() + 132.0;
        let used = crate::model::human_size(self.monitor_latest.sample.memory_used_bytes);
        let total = crate::model::human_size(self.monitor_latest.sample.memory_total_bytes);
        c.text(
            panel.left() + 8.0,
            memory_y + 9.0,
            &format!("{used} / {total}"),
            8.5,
            W::Semibold,
            theme::WHITE,
            H::Left,
        );
        c.text(
            panel.right() - 8.0,
            memory_y + 9.0,
            language.text("内存", "Memory"),
            7.5,
            W::Regular,
            theme::DIM,
            H::Right,
        );
        let track = Rect::from_min_max(
            c.pos(panel.left() + 8.0, memory_y + 15.0),
            c.pos(panel.right() - 8.0, memory_y + 19.0),
        );
        c.painter
            .rect_filled(track, 2.0, Color32::from_white_alpha(20));
        c.painter.rect_filled(
            Rect::from_min_max(
                track.min,
                Pos2::new(
                    track.left()
                        + track.width() * self.monitor_latest.sample.memory_percent / 100.0,
                    track.bottom(),
                ),
            ),
            2.0,
            ACTIVE,
        );
        let disk = self
            .monitor_latest
            .sample
            .disk_percent
            .map(|value| format!("{value:.0}%"))
            .unwrap_or_else(|| "--".into());
        let gpu = self
            .monitor_latest
            .sample
            .gpu_percent
            .map(|value| format!("{value:.0}%"))
            .unwrap_or_else(|| "--".into());
        c.text(
            panel.left() + 8.0,
            panel.bottom() - 8.0,
            &format!("D  {disk}"),
            7.5,
            W::Regular,
            theme::DIM,
            H::Left,
        );
        c.text(
            panel.left() + 63.0,
            panel.bottom() - 8.0,
            &format!("G  {gpu}"),
            7.5,
            W::Regular,
            theme::DIM,
            H::Left,
        );
        c.text(
            panel.right() - 8.0,
            panel.bottom() - 8.0,
            &format!("{} 进程", self.monitor_latest.sample.process_count),
            7.5,
            W::Regular,
            theme::DIM,
            H::Right,
        );
    }

    fn monitor_processes(&mut self, c: &Card) {
        let language = self.settings.language;
        let panel = monitor_page_layout().processes;
        c.painter.rect_filled(
            c.rect(panel.left(), panel.top(), panel.width(), panel.height()),
            10.0,
            SURFACE,
        );
        c.text(
            panel.left() + 8.0,
            panel.top() + 17.0,
            language.text("当前进程", "Processes"),
            8.0,
            W::Semibold,
            theme::WHITE,
            H::Left,
        );
        let items: Vec<_> = self
            .monitor_latest
            .top_cpu
            .iter()
            .take(6)
            .cloned()
            .collect();
        let paths: Vec<_> = items
            .iter()
            .filter(|item| !item.path.as_os_str().is_empty())
            .map(|item| item.path.clone())
            .collect();
        self.icons.retain(&paths);
        for (index, item) in items.iter().enumerate() {
            let center_y = panel.top() + 35.0 + index as f32 * 25.0;
            if index > 0 {
                c.seg(
                    (panel.left() + 7.0, center_y - 12.5),
                    (panel.right() - 7.0, center_y - 12.5),
                    1.0,
                    Color32::from_white_alpha(12),
                );
            }
            self.monitor_process_icon(c, item, panel.left() + 14.0, center_y, 15.0);
            c.text(
                panel.left() + 26.0,
                center_y - 1.0,
                &c.fit(&item.name, 8.2, W::Regular, 40.0),
                8.2,
                W::Regular,
                theme::WHITE,
                H::Left,
            );
            c.text(
                panel.right() - 7.0,
                center_y - 1.0,
                &format!("{:.0}%", item.cpu_percent),
                8.2,
                W::Semibold,
                theme::WHITE,
                H::Right,
            );
            c.text(
                panel.left() + 26.0,
                center_y + 8.0,
                &crate::model::human_size(item.memory_bytes),
                6.8,
                W::Regular,
                theme::DIM,
                H::Left,
            );
        }
    }

    fn monitor_process_icon(&mut self, c: &Card, item: &GroupUsage, cx: f32, cy: f32, size: f32) {
        let texture = (!item.path.as_os_str().is_empty())
            .then(|| self.icons.get(c.ui.ctx(), &item.path))
            .flatten()
            .and_then(|preview| preview.image);
        if let Some(texture) = texture {
            let source = texture.size_vec2();
            let factor = size / source.x.max(source.y).max(1.0);
            let rect = Rect::from_center_size(c.pos(cx, cy), source * factor * c.s);
            c.painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            let initial: String = item
                .name
                .chars()
                .find(|value| value.is_alphanumeric())
                .unwrap_or('?')
                .to_uppercase()
                .collect();
            c.circle(cx, cy, size * 0.46, Color32::from_gray(38));
            c.centered_text(cx, cy, &initial, 7.5, W::Semibold, theme::WHITE, H::Center);
        }
    }

    fn monitor_events_view(&mut self, c: &Card) {
        let language = self.settings.language;
        c.fill(CONTENT_LEFT, 40.0, CONTENT_WIDTH, 184.0, 10.0, SURFACE);
        c.text(
            52.0,
            58.0,
            language.text("最近警报", "Recent alerts"),
            11.0,
            W::Semibold,
            theme::WHITE,
            H::Left,
        );
        if self.monitor_events.is_empty() {
            c.text(
                209.5,
                138.0,
                language.text(
                    "超过阈值并持续 5 秒后会记录在这里",
                    "Sustained threshold alerts will appear here",
                ),
                10.0,
                W::Regular,
                theme::DIM,
                H::Center,
            );
            return;
        }
        for (index, event) in self.monitor_events.iter().take(7).enumerate() {
            let y = 80.0 + index as f32 * 21.0;
            if index > 0 {
                c.seg(
                    (52.0, y - 15.0),
                    (367.0, y - 15.0),
                    1.0,
                    Color32::from_white_alpha(12),
                );
            }
            let time = chrono::Local
                .timestamp_millis_opt(event.at_ms)
                .single()
                .map(|time| time.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "--:--:--".into());
            let (zh, en) = event.metric.title();
            c.text(52.0, y, &time, 8.5, W::Regular, theme::DIM, H::Left);
            c.text(
                108.0,
                y,
                language.text(zh, en),
                8.5,
                W::Semibold,
                theme::WHITE,
                H::Left,
            );
            c.text(
                368.0,
                y,
                &format!("{:.0}%", event.value),
                8.5,
                W::Semibold,
                theme::WHITE,
                H::Right,
            );
        }
    }
}

fn draw_line(c: &Card, plot: Rect, values: &[f32], color: Color32, width: f32, fill: bool) {
    if values.len() < 2 {
        return;
    }
    let step = plot.width() / (values.len() - 1) as f32;
    let mut previous = None;
    for (index, value) in values.iter().enumerate() {
        let x = plot.left() + step * index as f32;
        let y = plot.bottom() - value / 100.0 * plot.height();
        if fill {
            c.painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(x, y),
                    Pos2::new((x + step).min(plot.right()), plot.bottom()),
                ),
                0.0,
                Color32::from_white_alpha(8),
            );
        }
        if let Some(old) = previous {
            c.painter
                .line_segment([old, Pos2::new(x, y)], egui::Stroke::new(width, color));
        }
        previous = Some(Pos2::new(x, y));
    }
}

fn draw_history_bars(c: &Card, plot: Rect, values: &[f32], color: Color32) {
    if values.is_empty() {
        return;
    }
    const BARS: usize = 22;
    for bar in 0..BARS {
        let start = values.len() * bar / BARS;
        let end = (values.len() * (bar + 1) / BARS).max(start + 1);
        let sample = values[start.min(values.len() - 1)..end.min(values.len())]
            .iter()
            .copied()
            .fold(0.0, f32::max);
        let x = plot.left() + bar as f32 * plot.width() / BARS as f32 + 1.5;
        let width = plot.width() / BARS as f32 - 3.0;
        let height = (sample / 100.0 * plot.height()).max(2.0);
        c.painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x, plot.bottom() - height),
                Pos2::new(x + width, plot.bottom()),
            ),
            2.0,
            color,
        );
    }
}

#[test]
fn monitor_layout_gives_the_chart_more_width_than_processes() {
    let layout = monitor_page_layout();
    assert_eq!(layout.main.left(), CONTENT_LEFT);
    assert_eq!(layout.main.top(), 40.0);
    assert_eq!(layout.main.width(), 243.0);
    assert_eq!(layout.processes.left(), 281.0);
    assert_eq!(layout.processes.width(), 108.0);
    assert_eq!(layout.processes.right(), 389.0);
}
