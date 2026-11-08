//! Lightweight Windows system monitoring and alert history.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

pub const MAX_SAMPLES: usize = 300;
pub const MAX_EVENTS: usize = 200;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sample {
    pub at_secs: f64,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_percent: Option<f32>,
    pub gpu_percent: Option<f32>,
    pub process_count: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessUsage {
    pub pid: u32,
    pub name: String,
    pub path: PathBuf,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub gpu_percent: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupUsage {
    pub name: String,
    pub path: PathBuf,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub gpu_percent: Option<f32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupedProcesses {
    pub by_cpu: Vec<GroupUsage>,
    pub by_memory: Vec<GroupUsage>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub sample: Sample,
    pub top_cpu: Vec<GroupUsage>,
    pub top_memory: Vec<GroupUsage>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Metric {
    #[default]
    Cpu,
    Memory,
    Disk,
    Gpu,
}

impl Metric {
    pub fn value(self, sample: &Sample) -> Option<f32> {
        match self {
            Self::Cpu => Some(sample.cpu_percent),
            Self::Memory => Some(sample.memory_percent),
            Self::Disk => sample.disk_percent,
            Self::Gpu => sample.gpu_percent,
        }
    }

    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            Self::Cpu => ("CPU", "CPU"),
            Self::Memory => ("内存", "Memory"),
            Self::Disk => ("磁盘", "Disk"),
            Self::Gpu => ("GPU", "GPU"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertMetric {
    Cpu,
    Memory,
    Gpu,
}

impl AlertMetric {
    pub fn title(self) -> (&'static str, &'static str) {
        match self {
            Self::Cpu => ("CPU 占用持续偏高", "Sustained high CPU usage"),
            Self::Memory => ("内存占用持续偏高", "Sustained high memory usage"),
            Self::Gpu => ("GPU 占用持续偏高", "Sustained high GPU usage"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonitorEvent {
    pub at_ms: i64,
    pub metric: AlertMetric,
    pub value: f32,
    pub threshold: f32,
    pub app: String,
}

#[derive(Debug, Default)]
pub struct History {
    items: VecDeque<Sample>,
}

impl History {
    pub fn push(&mut self, sample: Sample) {
        self.items.push_back(sample);
        while self.items.len() > MAX_SAMPLES {
            self.items.pop_front();
        }
    }

    pub fn window(&self, seconds: f64) -> Vec<Sample> {
        let Some(latest) = self.items.back() else {
            return Vec::new();
        };
        let cutoff = latest.at_secs - seconds.max(0.0);
        self.items
            .iter()
            .filter(|sample| sample.at_secs > cutoff)
            .cloned()
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct ThresholdGate {
    required_secs: f64,
    cooldown_secs: f64,
    above_since: Option<f64>,
    armed: bool,
    cooldown_until: f64,
}

impl ThresholdGate {
    pub fn new(required_secs: f64) -> Self {
        Self {
            required_secs,
            cooldown_secs: 60.0,
            above_since: None,
            armed: true,
            cooldown_until: 0.0,
        }
    }

    pub fn observe(&mut self, value: f32, threshold: f32, now_secs: f64) -> bool {
        if !self.armed {
            if value < threshold - 5.0 {
                self.armed = true;
                self.above_since = None;
            }
            return false;
        }
        if value < threshold {
            self.above_since = None;
            if value < threshold - 5.0 {
                self.armed = true;
            }
            return false;
        }
        let since = *self.above_since.get_or_insert(now_secs);
        if now_secs - since < self.required_secs || now_secs < self.cooldown_until {
            return false;
        }
        self.above_since = None;
        self.armed = false;
        self.cooldown_until = now_secs + self.cooldown_secs;
        true
    }
}

impl Default for ThresholdGate {
    fn default() -> Self {
        Self::new(5.0)
    }
}

#[derive(Default)]
pub struct AlertState {
    cpu: ThresholdGate,
    memory: ThresholdGate,
    gpu: ThresholdGate,
}

impl AlertState {
    pub fn observe(
        &mut self,
        sample: &Sample,
        cpu_threshold: u32,
        memory_threshold: u32,
        gpu_threshold: u32,
    ) -> Vec<AlertMetric> {
        let mut fired = Vec::new();
        if self
            .cpu
            .observe(sample.cpu_percent, cpu_threshold as f32, sample.at_secs)
        {
            fired.push(AlertMetric::Cpu);
        }
        if self.memory.observe(
            sample.memory_percent,
            memory_threshold as f32,
            sample.at_secs,
        ) {
            fired.push(AlertMetric::Memory);
        }
        if let Some(gpu) = sample.gpu_percent {
            if self.gpu.observe(gpu, gpu_threshold as f32, sample.at_secs) {
                fired.push(AlertMetric::Gpu);
            }
        }
        fired
    }
}

pub fn process_cpu_percent(delta_100ns: u64, elapsed_secs: f64, logical_cores: u32) -> f32 {
    if elapsed_secs <= 0.0 || logical_cores == 0 {
        return 0.0;
    }
    (delta_100ns as f64 / 10_000_000.0 / elapsed_secs / logical_cores as f64 * 100.0)
        .clamp(0.0, 100.0) as f32
}

pub fn group_processes(
    processes: Vec<ProcessUsage>,
    cpu_limit: usize,
    memory_limit: usize,
) -> GroupedProcesses {
    let mut grouped: HashMap<String, GroupUsage> = HashMap::new();
    for process in processes {
        let key = process.name.to_ascii_lowercase();
        let item = grouped.entry(key).or_insert_with(|| GroupUsage {
            name: process.name.clone(),
            path: process.path.clone(),
            ..Default::default()
        });
        if item.path.as_os_str().is_empty() && !process.path.as_os_str().is_empty() {
            item.path = process.path.clone();
        }
        item.cpu_percent = (item.cpu_percent + process.cpu_percent).min(100.0);
        item.memory_bytes = item.memory_bytes.saturating_add(process.memory_bytes);
        if let Some(gpu) = process.gpu_percent {
            item.gpu_percent = Some((item.gpu_percent.unwrap_or(0.0) + gpu).min(100.0));
        }
    }
    let mut all: Vec<_> = grouped.into_values().collect();
    all.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent));
    let by_cpu = all.iter().take(cpu_limit).cloned().collect();
    all.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    let by_memory = all.into_iter().take(memory_limit).collect();
    GroupedProcesses { by_cpu, by_memory }
}

fn events_path() -> std::path::PathBuf {
    crate::model::config_dir().join("monitor-events.json")
}

pub fn encode_events(events: &[MonitorEvent]) -> Vec<u8> {
    serde_json::to_vec(events).unwrap_or_default()
}

pub fn decode_events(raw: &[u8]) -> Vec<MonitorEvent> {
    let mut events: Vec<_> = serde_json::from_slice(raw).unwrap_or_default();
    if events.len() > MAX_EVENTS {
        events.drain(..events.len() - MAX_EVENTS);
    }
    events
}

pub fn load_events() -> Vec<MonitorEvent> {
    std::fs::read(events_path())
        .map(|raw| decode_events(&raw))
        .unwrap_or_default()
}

pub fn save_events(events: &[MonitorEvent]) {
    let mut events = events.to_vec();
    if events.len() > MAX_EVENTS {
        events.drain(..events.len() - MAX_EVENTS);
    }
    if let Some(parent) = events_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = crate::model::atomic_write(&events_path(), &encode_events(&events));
}

pub struct Service {
    incoming: mpsc::Receiver<Snapshot>,
    enabled: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Service {
    pub fn start(ctx: eframe::egui::Context) -> Self {
        let (tx, incoming) = mpsc::sync_channel(4);
        let enabled = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));
        let active = enabled.clone();
        let stopped = stop.clone();
        let worker = std::thread::Builder::new()
            .name("nexora-isle-monitor".into())
            .spawn(move || {
                #[cfg(windows)]
                let mut sampler = sampler::Sampler::default();
                loop {
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    if !active.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(250));
                        #[cfg(windows)]
                        sampler.reset();
                        continue;
                    }
                    #[cfg(windows)]
                    let snapshot = sampler.sample();
                    #[cfg(not(windows))]
                    let snapshot = Some(Snapshot::default());
                    if let Some(snapshot) = snapshot {
                        let _ = tx.try_send(snapshot);
                        ctx.request_repaint();
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            })
            .ok();
        Self {
            incoming,
            enabled,
            stop,
            worker,
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn try_recv(&self) -> Option<Snapshot> {
        self.incoming.try_recv().ok()
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = &self.worker {
            worker.thread().unpark();
        }
    }
}

#[cfg(windows)]
mod sampler {
    use super::*;
    use windows::core::{w, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhGetFormattedCounterValue, PdhOpenQueryW, PDH_FMT_COUNTERVALUE,
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MORE_DATA,
    };
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::Threading::{
        GetProcessTimes, GetSystemTimes, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    #[derive(Default)]
    pub struct Sampler {
        system: Option<(u64, u64)>,
        processes: HashMap<u32, (u64, u64)>,
        sampled_at: Option<Instant>,
        pdh: Option<Pdh>,
    }

    impl Sampler {
        pub fn reset(&mut self) {
            self.system = None;
            self.processes.clear();
            self.sampled_at = None;
            self.pdh = None;
        }

        pub fn sample(&mut self) -> Option<Snapshot> {
            let now = Instant::now();
            let elapsed = self
                .sampled_at
                .map(|at| now.duration_since(at).as_secs_f64())
                .unwrap_or(0.0);
            let (idle, kernel, user) = system_times()?;
            let total = kernel.saturating_add(user);
            let cpu_percent = self
                .system
                .map(|(old_idle, old_total)| {
                    let dt = total.saturating_sub(old_total);
                    if dt == 0 {
                        0.0
                    } else {
                        (dt.saturating_sub(idle.saturating_sub(old_idle)) as f64 / dt as f64
                            * 100.0)
                            .clamp(0.0, 100.0) as f32
                    }
                })
                .unwrap_or(0.0);
            self.system = Some((idle, total));
            self.sampled_at = Some(now);

            let mut memory = MEMORYSTATUSEX {
                dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                ..Default::default()
            };
            unsafe { GlobalMemoryStatusEx(&mut memory).ok()? };
            let used = memory.ullTotalPhys.saturating_sub(memory.ullAvailPhys);

            if self.pdh.is_none() {
                self.pdh = Pdh::new();
            }
            if let Some(pdh) = &mut self.pdh {
                if !pdh.collect() {
                    self.pdh = None;
                }
            }
            let (disk_percent, gpu_by_pid, gpu_percent) = self
                .pdh
                .as_mut()
                .map(|pdh| (pdh.disk(), pdh.gpu_by_pid(), pdh.gpu_total()))
                .unwrap_or((None, HashMap::new(), None));

            let (process_count, processes) =
                process_snapshot(&self.processes, elapsed, &gpu_by_pid);
            self.processes = processes
                .iter()
                .map(|process| {
                    let total = process.1 .0.saturating_add(process.1 .1);
                    (process.0, (process.1 .2, total))
                })
                .collect();
            let raw: Vec<_> = processes.into_iter().map(|(_, row)| row.3).collect();
            let grouped = group_processes(raw, 6, 6);
            Some(Snapshot {
                sample: Sample {
                    at_secs: unix_secs(),
                    cpu_percent,
                    memory_percent: memory.dwMemoryLoad as f32,
                    memory_used_bytes: used,
                    memory_total_bytes: memory.ullTotalPhys,
                    disk_percent,
                    gpu_percent,
                    process_count,
                },
                top_cpu: grouped.by_cpu,
                top_memory: grouped.by_memory,
            })
        }
    }

    fn system_times() -> Option<(u64, u64, u64)> {
        let (mut idle, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        unsafe {
            GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()?;
        }
        Some((filetime(idle), filetime(kernel), filetime(user)))
    }

    fn filetime(value: FILETIME) -> u64 {
        ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
    }

    fn process_snapshot(
        previous: &HashMap<u32, (u64, u64)>,
        elapsed: f64,
        gpu_by_pid: &HashMap<u32, f32>,
    ) -> (u32, Vec<(u32, (u64, u64, u64, ProcessUsage))>) {
        let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
            return (0, Vec::new());
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut count = 0;
        let mut rows = Vec::new();
        let mut ok = unsafe { Process32FirstW(snapshot, &mut entry).is_ok() };
        while ok {
            count += 1;
            let pid = entry.th32ProcessID;
            let name = wide_string(&entry.szExeFile);
            if pid != 0 {
                if let Some(usage) = process_usage(pid, &name, previous, elapsed, gpu_by_pid) {
                    rows.push(usage);
                }
            }
            ok = unsafe { Process32NextW(snapshot, &mut entry).is_ok() };
        }
        let _ = unsafe { CloseHandle(snapshot) };
        (count, rows)
    }

    fn process_usage(
        pid: u32,
        name: &str,
        previous: &HashMap<u32, (u64, u64)>,
        elapsed: f64,
        gpu_by_pid: &HashMap<u32, f32>,
    ) -> Option<(u32, (u64, u64, u64, ProcessUsage))> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let (mut created, mut exited, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        let times =
            unsafe { GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user) };
        let mut memory = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };
        let memory_ok = unsafe { GetProcessMemoryInfo(process, &mut memory, memory.cb).is_ok() };
        let mut path_buffer = vec![0u16; 32_768];
        let mut path_len = path_buffer.len() as u32;
        let path = unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(path_buffer.as_mut_ptr()),
                &mut path_len,
            )
        }
        .is_ok()
        .then(|| PathBuf::from(String::from_utf16_lossy(&path_buffer[..path_len as usize])))
        .unwrap_or_default();
        let _ = unsafe { CloseHandle(process) };
        times.ok()?;
        if !memory_ok {
            return None;
        }
        let creation = filetime(created);
        let total_cpu = filetime(kernel).saturating_add(filetime(user));
        let cpu_percent = previous
            .get(&pid)
            .filter(|(old_creation, _)| *old_creation == creation)
            .map(|(_, old_cpu)| {
                process_cpu_percent(total_cpu.saturating_sub(*old_cpu), elapsed, logical_cores())
            })
            .unwrap_or(0.0);
        Some((
            pid,
            (
                filetime(kernel),
                filetime(user),
                creation,
                ProcessUsage {
                    pid,
                    name: name.to_string(),
                    path,
                    cpu_percent,
                    memory_bytes: memory.WorkingSetSize as u64,
                    gpu_percent: gpu_by_pid.get(&pid).copied(),
                },
            ),
        ))
    }

    fn wide_string(raw: &[u16]) -> String {
        let len = raw
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(raw.len());
        String::from_utf16_lossy(&raw[..len])
    }

    fn logical_cores() -> u32 {
        std::thread::available_parallelism()
            .map(|value| value.get() as u32)
            .unwrap_or(1)
    }

    fn unix_secs() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }

    struct Pdh {
        query: isize,
        disk: isize,
        gpu: isize,
    }

    impl Pdh {
        fn new() -> Option<Self> {
            let mut query = 0;
            let mut disk = 0;
            let mut gpu = 0;
            unsafe {
                if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                    return None;
                }
                let disk_status = PdhAddEnglishCounterW(
                    query,
                    w!("\\PhysicalDisk(_Total)\\% Disk Time"),
                    0,
                    &mut disk,
                );
                let gpu_status = PdhAddEnglishCounterW(
                    query,
                    w!("\\GPU Engine(*)\\Utilization Percentage"),
                    0,
                    &mut gpu,
                );
                if disk_status != 0 || gpu_status != 0 {
                    let _ = PdhCloseQuery(query);
                    return None;
                }
                let _ = PdhCollectQueryData(query);
            }
            Some(Self { query, disk, gpu })
        }

        fn collect(&self) -> bool {
            unsafe { PdhCollectQueryData(self.query) == 0 }
        }

        fn disk(&self) -> Option<f32> {
            let mut value = PDH_FMT_COUNTERVALUE::default();
            let status =
                unsafe { PdhGetFormattedCounterValue(self.disk, PDH_FMT_DOUBLE, None, &mut value) };
            if status != 0 || value.CStatus != 0 {
                return None;
            }
            Some(unsafe { value.Anonymous.doubleValue }.clamp(0.0, 100.0) as f32)
        }

        fn gpu_items(&self) -> Vec<(String, f32)> {
            let mut size = 0u32;
            let mut count = 0u32;
            let status = unsafe {
                PdhGetFormattedCounterArrayW(self.gpu, PDH_FMT_DOUBLE, &mut size, &mut count, None)
            };
            if status != PDH_MORE_DATA || size == 0 || count == 0 {
                return Vec::new();
            }
            let mut bytes = vec![0u64; (size as usize).div_ceil(8)];
            let status = unsafe {
                PdhGetFormattedCounterArrayW(
                    self.gpu,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    Some(bytes.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>()),
                )
            };
            if status != 0 {
                return Vec::new();
            }
            let items = unsafe {
                std::slice::from_raw_parts(
                    bytes.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                    count as usize,
                )
            };
            items
                .iter()
                .filter_map(|item| {
                    if item.FmtValue.CStatus != 0 || item.szName.is_null() {
                        return None;
                    }
                    Some((
                        unsafe { item.szName.to_string().ok()? },
                        unsafe { item.FmtValue.Anonymous.doubleValue }.clamp(0.0, 100.0) as f32,
                    ))
                })
                .collect()
        }

        fn gpu_by_pid(&self) -> HashMap<u32, f32> {
            let mut map = HashMap::<u32, f32>::new();
            for (name, value) in self.gpu_items() {
                if let Some(pid) = gpu_instance_pid(&name) {
                    let total = map.entry(pid).or_default();
                    *total = (*total + value).min(100.0);
                }
            }
            map
        }

        fn gpu_total(&self) -> Option<f32> {
            let items = self.gpu_items();
            items.iter().map(|(_, value)| *value).max_by(f32::total_cmp)
        }
    }

    impl Drop for Pdh {
        fn drop(&mut self) {
            let _ = unsafe { PdhCloseQuery(self.query) };
        }
    }

    fn gpu_instance_pid(instance: &str) -> Option<u32> {
        let lower = instance.to_ascii_lowercase();
        let rest = lower.split("pid_").nth(1)?;
        rest.split('_').next()?.parse().ok()
    }

    #[cfg(test)]
    #[test]
    fn gpu_instances_parse_process_and_engine_names() {
        assert_eq!(
            gpu_instance_pid("pid_11704_luid_0x00000000_0x0000ed93_phys_0_eng_0_engtype_3d"),
            Some(11704)
        );
        assert_eq!(gpu_instance_pid("gpu engine"), None);
    }

    #[cfg(test)]
    #[test]
    fn live_sample_contains_real_system_data() {
        let mut sampler = Sampler::default();
        assert!(sampler.sample().is_some());
        std::thread::sleep(Duration::from_secs(1));
        let snapshot = sampler.sample().expect("second sample");
        assert!((0.0..=100.0).contains(&snapshot.sample.cpu_percent));
        assert!(snapshot.sample.memory_total_bytes > 0);
        assert!(snapshot.sample.process_count > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_keeps_exactly_the_requested_window() {
        let mut history = History::default();
        for second in 0..=300 {
            history.push(Sample {
                at_secs: second as f64,
                ..Default::default()
            });
        }
        assert_eq!(history.window(60.0).len(), 60);
        assert_eq!(history.window(300.0).len(), 300);
    }

    #[test]
    fn thresholds_require_sustained_load_and_respect_cooldown() {
        let mut gate = ThresholdGate::new(5.0);
        assert!(!gate.observe(90.0, 80.0, 0.0));
        assert!(!gate.observe(91.0, 80.0, 4.9));
        assert!(gate.observe(92.0, 80.0, 5.0));
        assert!(!gate.observe(93.0, 80.0, 20.0));
        assert!(!gate.observe(70.0, 80.0, 70.0));
        assert!(!gate.observe(90.0, 80.0, 75.0));
        assert!(gate.observe(90.0, 80.0, 80.0));
    }

    #[test]
    fn process_cpu_is_normalized_by_elapsed_time_and_logical_cores() {
        assert!((process_cpu_percent(100_000_000, 1.0, 16) - 62.5).abs() < 0.01);
        assert_eq!(process_cpu_percent(100, 0.0, 16), 0.0);
    }

    #[test]
    fn processes_are_grouped_by_executable_name_and_sorted() {
        let grouped = group_processes(
            vec![
                ProcessUsage {
                    pid: 1,
                    name: "msedge.exe".into(),
                    path: PathBuf::new(),
                    cpu_percent: 10.0,
                    memory_bytes: 100,
                    gpu_percent: Some(4.0),
                },
                ProcessUsage {
                    pid: 2,
                    name: "MSEDGE.EXE".into(),
                    path: PathBuf::new(),
                    cpu_percent: 5.0,
                    memory_bytes: 200,
                    gpu_percent: Some(3.0),
                },
                ProcessUsage {
                    pid: 3,
                    name: "game.exe".into(),
                    path: PathBuf::new(),
                    cpu_percent: 8.0,
                    memory_bytes: 50,
                    gpu_percent: Some(20.0),
                },
            ],
            4,
            4,
        );
        assert_eq!(grouped.by_cpu[0].name, "msedge.exe");
        assert_eq!(grouped.by_cpu[0].cpu_percent, 15.0);
        assert_eq!(grouped.by_memory[0].name, "msedge.exe");
        assert_eq!(grouped.by_memory[0].memory_bytes, 300);
    }

    #[test]
    fn event_log_keeps_newest_events_and_round_trips() {
        let events: Vec<_> = (0..205)
            .map(|index| MonitorEvent {
                at_ms: index,
                metric: AlertMetric::Cpu,
                value: 90.0,
                threshold: 85.0,
                app: "test.exe".into(),
            })
            .collect();
        let decoded = decode_events(&encode_events(&events));
        assert_eq!(decoded.len(), MAX_EVENTS);
        assert_eq!(decoded.first().map(|event| event.at_ms), Some(5));
        assert_eq!(decoded.last().map(|event| event.at_ms), Some(204));
    }
}
