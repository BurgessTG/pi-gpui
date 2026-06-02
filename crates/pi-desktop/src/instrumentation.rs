use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

struct RenderTraceState {
    last_flush: Instant,
    counts: BTreeMap<&'static str, u64>,
}

static ENABLED: OnceLock<bool> = OnceLock::new();
static FLUSH_INTERVAL: OnceLock<Duration> = OnceLock::new();
static STATE: OnceLock<Mutex<RenderTraceState>> = OnceLock::new();

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

fn render_trace_enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var("PI_WORKSPACES_RENDER_TRACE")
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
