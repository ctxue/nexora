# HALO System Monitor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the A+B monitor experience: live system metrics, 1/5-minute charts, Top Apps, threshold alerts and local events.

**Architecture:** A single Windows sampler thread publishes immutable snapshots. `App` owns short history and alert gates; `monitor_ui` renders the monitor page from that state.

**Tech Stack:** Rust 2021, egui/eframe 0.31, Win32 PDH and ToolHelp APIs, serde_json.

**Spec:** `docs/superpowers/specs/2026-11-08-monitor-design.md`

## Global Constraints

- Windows 10/11 x64 only for the live sampler.
- One-second sampling and 300 in-memory samples.
- No temperature metric, long-term history or process control in this plan.
- No blocking Windows API call on the render thread.
- No fabricated values when a counter or process is unreadable.
- Monitor and event data stay local.

---

### Task 1: Monitor Data Model and Alert Gates

**Files:**
- Create: `src/monitor.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces: `Sample`, `ProcessUsage`, `Snapshot`, `AlertMetric`,
  `ThresholdGate::observe(value, threshold, now_secs) -> bool`,
  `group_processes(...)`, `process_cpu_percent(...)`.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn thresholds_require_sustained_load_and_respect_cooldown() {
    let mut gate = ThresholdGate::default();
    assert!(!gate.observe(90.0, 80.0, 0.0));
    assert!(!gate.observe(91.0, 80.0, 4.9));
    assert!(gate.observe(92.0, 80.0, 5.0));
    assert!(!gate.observe(93.0, 80.0, 20.0));
    assert!(!gate.observe(70.0, 80.0, 70.0));
    assert!(!gate.observe(90.0, 80.0, 75.0));
    assert!(gate.observe(90.0, 80.0, 80.0));
}

#[test]
fn history_keeps_exactly_the_requested_window() {
    let mut history = History::default();
    for second in 0..301 {
        history.push(Sample { at_secs: second as f64, ..Default::default() });
    }
    assert_eq!(history.window(60.0).len(), 60);
    assert_eq!(history.window(300.0).len(), 300);
}
```

- [ ] **Step 2: Run tests and verify failure**

Run: `cargo test --lib monitor::tests -- --test-threads=1`

- [ ] **Step 3: Implement the data types, history and threshold state machine**

`ThresholdGate` tracks `above_since`, `armed`, and `cooldown_until`. It fires
only after five seconds above threshold, re-arms after dropping five points
below threshold, and never emits within the 60-second cooldown.

- [ ] **Step 4: Run tests**

Run: `cargo test --lib monitor::tests -- --test-threads=1`

---

### Task 2: Windows Sampler

**Files:**
- Modify: `src/monitor.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Produces: `Service::start(ctx)`, `Service::try_recv()`,
  `Service::set_enabled(bool)`.
- Consumes: monitor data types from Task 1.

- [ ] **Step 1: Add failing Windows integration test**

```rust
#[cfg(windows)]
#[test]
fn live_sample_contains_real_system_data() {
    let sample = sampler::sample_once().expect("system sample");
    assert!((0.0..=100.0).contains(&sample.metrics.cpu));
    assert!(sample.metrics.memory_total > 0);
    assert!(sample.processes > 0);
}
```

- [ ] **Step 2: Run the test and verify failure**

Run: `cargo test --lib monitor::tests::live_sample_contains_real_system_data -- --test-threads=1`

- [ ] **Step 3: Implement sampling**

- Overall CPU: `GetSystemTimes` deltas.
- Memory: `GlobalMemoryStatusEx`.
- Process count and CPU: `CreateToolhelp32Snapshot`, `GetProcessTimes`.
- Process memory: `GetProcessMemoryInfo`.
- Disk and GPU: PDH English counters with graceful `None` results.
- Enable `Win32_System_Performance` and `Win32_System_ProcessStatus` features.

- [ ] **Step 4: Run the live sample test**

Run: `cargo test --lib monitor::tests::live_sample_contains_real_system_data -- --test-threads=1`

---

### Task 3: Page and Settings Integration

**Files:**
- Modify: `src/model/page.rs`
- Modify: `src/model.rs`
- Modify: `src/app.rs`
- Modify: `src/ui_tests.rs`

**Interfaces:**
- Produces: `page::MONITOR`, monitor settings fields and `App::page_available(MONITOR)`.
- Consumes: snapshot/history state from Tasks 1-2.

- [ ] **Step 1: Update failing migration tests**

```rust
assert_eq!(page::COUNT, 7);
assert_eq!(page::ALL[2], page::MONITOR);
assert_eq!(settings.monitor_cpu_threshold, 85);
```

- [ ] **Step 2: Run tests and verify failure**

Run: `cargo test --lib model::tests shell_uses_the_four_retained_pages_and_navigation -- --test-threads=1`

- [ ] **Step 3: Add the monitor page and settings**

Add `MONITOR=6`, settings for enablement and CPU/memory/GPU thresholds, and
versioned six-to-seven-page migration. Default the page to enabled.

- [ ] **Step 4: Run migration and page tests**

Run: `cargo test --lib model::tests ui_tests -- --test-threads=1`

---

### Task 4: Monitor UI

**Files:**
- Create: `src/monitor_ui.rs`
- Modify: `src/app.rs`
- Modify: `src/extras.rs`

**Interfaces:**
- Produces: `App::page_monitor`, metric cells, area chart, application list,
  event view and monitor settings.
- Consumes: `App::monitor_history`, selected metric/range/sort state and events.

- [ ] **Step 1: Write failing layout/presentation tests**

```rust
#[test]
fn monitor_chart_uses_only_samples_inside_the_selected_window() {
    let history = samples_at(0.0..=300.0);
    assert_eq!(chart_samples(&history, MonitorRange::OneMinute, 300.0).len(), 60);
}

#[test]
fn monitor_page_is_retained_and_named() {
    assert_eq!(page::retained_name(page::MONITOR), "监控");
}
```

- [ ] **Step 2: Run tests and verify failure**

Run: `cargo test --lib monitor_chart_uses_only_samples_inside_the_selected_window monitor_page_is_retained_and_named -- --test-threads=1`

- [ ] **Step 3: Implement the page**

Build five compact metric cells, one selected-metric chart, `1m/5m` and
`CPU/内存` controls, three application rows, and an event list. Use neutral
surfaces with one violet accent.

- [ ] **Step 4: Add settings UI**

Add a `Monitor` settings tab with master enable, alerts enable, and numeric
threshold controls for CPU, memory and GPU.

- [ ] **Step 5: Run UI tests and compile**

Run: `cargo test --lib ui_tests -- --test-threads=1`
Run: `cargo check --all-targets`

---

### Task 5: Alerts, Events and Runtime Wiring

**Files:**
- Modify: `src/main.rs`
- Modify: `src/app.rs`
- Modify: `src/monitor.rs`
- Modify: `README.md`

**Interfaces:**
- Produces: `App::monitor_snapshot`, persisted `MonitorEvent` queue and island alerts.
- Consumes: `monitor::Service`, settings and existing `App::alert`.

- [ ] **Step 1: Write failing event tests**

```rust
#[test]
fn event_log_keeps_newest_events_and_round_trips() {
    let events = vec![event_at(1), event_at(2), event_at(3)];
    let text = encode_events(&events);
    assert_eq!(decode_events(&text), events);
}
```

- [ ] **Step 2: Run tests and verify failure**

Run: `cargo test --lib monitor::tests::event_log_keeps_newest_events_and_round_trips -- --test-threads=1`

- [ ] **Step 3: Wire sampler and alerts**

Start the service in the shell, update its active state from monitor/alerts
settings, forward snapshots to `App`, persist fired events, and enqueue island
alerts with page target `page::MONITOR`.

- [ ] **Step 4: Run full verification**

Run: `cargo test --all-targets -- --test-threads=1`
Run: `cargo build --bin nexora-halo`

- [ ] **Step 5: Restart Halo**

Stop only the development executable at
`target/debug/nexora-halo.exe`, rebuild and launch it after tests pass.

