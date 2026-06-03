use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

struct RenderTraceState {
    last_flush: Instant,
    counts: BTreeMap<&'static str, u64>,
}

struct FrameTraceState {
    last_flush: Instant,
    counts: BTreeMap<&'static str, FrameTraceCounter>,
}

#[derive(Clone, Copy, Debug, Default)]
struct FrameTraceCounter {
    count: u64,
    total_micros: u128,
    max_micros: u128,
}

static ENABLED: OnceLock<bool> = OnceLock::new();
static FRAME_ENABLED: OnceLock<bool> = OnceLock::new();
static FLUSH_INTERVAL: OnceLock<Duration> = OnceLock::new();
static STATE: OnceLock<Mutex<RenderTraceState>> = OnceLock::new();
static FRAME_STATE: OnceLock<Mutex<FrameTraceState>> = OnceLock::new();

pub fn record_render(label: &'static str) {
    if !render_trace_enabled() {
        return;
    }

    let Ok(mut state) = STATE
        .get_or_init(|| {
            Mutex::new(RenderTraceState {
                last_flush: Instant::now(),
                counts: BTreeMap::new(),
            })
        })
        .lock()
    else {
        return;
    };
    *state.counts.entry(label).or_default() += 1;

    let now = Instant::now();
    if now.duration_since(state.last_flush) < *flush_interval() {
        return;
    }

    let summary = state
        .counts
        .iter()
        .map(|(label, count)| format!("{label}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    eprintln!("[pi-workspaces render-trace] {summary}");
    state.counts.clear();
    state.last_flush = now;
}

pub fn record_frame_latency(label: &'static str, latency: Duration) {
    if !frame_trace_enabled() {
        return;
    }

    let Ok(mut state) = FRAME_STATE
        .get_or_init(|| {
            Mutex::new(FrameTraceState {
                last_flush: Instant::now(),
                counts: BTreeMap::new(),
            })
        })
        .lock()
    else {
        return;
    };
    let counter = state.counts.entry(label).or_default();
    counter.count = counter.count.saturating_add(1);
    let micros = latency.as_micros();
    counter.total_micros = counter.total_micros.saturating_add(micros);
    counter.max_micros = counter.max_micros.max(micros);

    let now = Instant::now();
    if now.duration_since(state.last_flush) < *flush_interval() {
        return;
    }

    let summary = state
        .counts
        .iter()
        .map(|(label, counter)| {
            let avg = if counter.count == 0 {
                0
            } else {
                counter.total_micros / u128::from(counter.count)
            };
            format!(
                "{label}:count={} avg_ms={:.2} max_ms={:.2}",
                counter.count,
                avg as f64 / 1000.0,
                counter.max_micros as f64 / 1000.0
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    eprintln!("[pi-workspaces frame-trace] {summary}");
    state.counts.clear();
    state.last_flush = now;
}

fn render_trace_enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var("PI_WORKSPACES_RENDER_TRACE")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
    })
}

fn frame_trace_enabled() -> bool {
    *FRAME_ENABLED.get_or_init(|| {
        std::env::var("PI_WORKSPACES_FRAME_TRACE")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
    })
}

fn flush_interval() -> &'static Duration {
    FLUSH_INTERVAL.get_or_init(|| {
        std::env::var("PI_WORKSPACES_RENDER_TRACE_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|millis| *millis > 0)
            .map(Duration::from_millis)
            .unwrap_or_else(|| Duration::from_secs(1))
    })
}
