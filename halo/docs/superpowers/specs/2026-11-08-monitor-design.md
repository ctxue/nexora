# HALO System Monitor Design

## Goal

Add a dedicated monitoring page to NEXORA HALO with real-time CPU, memory,
disk, GPU and process metrics, 1-minute and 5-minute history charts, top
application rows, threshold alerts, and a local event log.

## Scope

Included:

- One-second Windows sampling.
- CPU, memory, disk, GPU and process count.
- CPU and memory usage from process snapshots.
- GPU usage grouped by process from Windows GPU Engine counters.
- 300-sample in-memory history for the 1-minute and 5-minute views.
- Top applications sorted by CPU or memory.
- Sustained threshold alerts for CPU, memory and GPU.
- A local, privacy-minimal log of the latest 200 threshold events.
- A dedicated `Monitor` page and a settings tab for thresholds.

Excluded from this version:

- CPU or GPU temperature.
- Multi-day persisted resource history.
- Process termination, app blocking, publisher rules or privacy access history.
- Exact per-adapter GPU normalization across mixed engines.

## Data Flow

`monitor::Service` runs one background thread while monitoring or alerts are
enabled. It samples Windows once per second and sends a `Snapshot` containing:

- timestamp;
- overall CPU, memory, disk and optional GPU percentages;
- process count;
- top process groups by CPU and by memory.

The Halo shell forwards each snapshot to `App`. `App` appends it to a bounded
history, evaluates threshold gates, and enqueues island alerts when a metric
has stayed above its threshold for five seconds. Each fired alert is also
written to `monitor-events.json` in the Halo data directory.

The UI never blocks on Windows APIs. Missing GPU or disk counters are shown as
unavailable and do not disable the remaining metrics.

## Architecture

- `src/monitor.rs`: platform-neutral data types, history buffer, process
  grouping, threshold gates and event persistence; Windows-only sampler.
- `src/monitor_ui.rs`: the monitor page, metric cards, chart, Top Apps and
  event list. It consumes only data already collected by `App`.
- `src/app.rs`: holds monitor history, selection state, event queue and alert
  gates; exposes `page_monitor`.
- `src/model.rs`: monitor settings and page configuration.
- `src/main.rs`: starts/polls the monitor service and forwards snapshots.
- `src/extras.rs`: monitor settings tab and threshold controls.

## Visual Design

The page follows the compact, neutral shadcn visual language:

- five small rounded metric cells for CPU, memory, disk, GPU and processes;
- one violet area/line chart for the selected metric;
- a `1m / 5m` segmented control and a `Top CPU / Top memory` toggle;
- three compact application rows with icon-like initials, name and value;
- an event view reachable from the page header.

No decorative axes, gauges or dense legends are shown. The selected metric is
the only strong color; other cards use neutral surfaces and muted borders.

## Alert Rules

- CPU default threshold: 85%.
- Memory default threshold: 85%.
- GPU default threshold: 90%.
- A threshold must stay exceeded for five consecutive samples.
- The alert fires once, then enters a 60-second cooldown.
- Dropping 5 percentage points below the threshold re-arms the alert.
- Alerts use the existing Halo notification pipeline and point to the monitor
  page when clicked.

## Failure Handling

- Missing PDH GPU counters produce `None`, not fabricated values.
- Protected processes that cannot be opened are skipped.
- Counter query failures recreate the PDH query on the next sampling loop.
- The UI renders a short unavailable state until the first sample arrives.
- Event-log write failures do not interrupt sampling.

## Verification

- Unit tests cover history windows, process grouping, CPU normalization,
  GPU instance parsing, threshold sustain/cooldown, event persistence and page
  migration.
- Windows integration tests verify that one live sample contains a valid CPU
  percentage, non-zero memory information and a non-empty process list.
- `cargo test --all-targets -- --test-threads=1` and
  `cargo build --bin nexora-halo` must pass before the executable is replaced.
