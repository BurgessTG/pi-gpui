use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(super) struct ProcessBridgeMetrics {
    enabled: bool,
    flush_interval: Duration,
    state: Mutex<ProcessBridgeMetricsState>,
}

#[derive(Debug)]
struct ProcessBridgeMetricsState {
    last_flush: Instant,
    counters: ProcessBridgeCounters,
}

#[derive(Clone, Copy, Debug, Default)]
struct ProcessBridgeCounters {
    requests: u64,
    request_bytes: u64,
    responses: u64,
    response_bytes: u64,
    events: u64,
    event_bytes: u64,
    stderr_lines: u64,
    stdout_invalid: u64,
    stdout_closed: u64,
    max_pending: usize,
}

impl ProcessBridgeMetrics {
    pub(super) fn from_env() -> Self {
        Self {
            enabled: trace_enabled(),
            flush_interval: flush_interval(),
            state: Mutex::new(ProcessBridgeMetricsState {
                last_flush: Instant::now(),
                counters: ProcessBridgeCounters::default(),
            }),
        }
    }

    pub(super) fn record_request(&self, bytes: usize, pending: usize) {
        self.record(|counters| {
            counters.requests = counters.requests.saturating_add(1);
            counters.request_bytes = counters.request_bytes.saturating_add(bytes as u64);
            counters.max_pending = counters.max_pending.max(pending);
        });
    }

    pub(super) fn record_response(&self, bytes: usize) {
        self.record(|counters| {
            counters.responses = counters.responses.saturating_add(1);
            counters.response_bytes = counters.response_bytes.saturating_add(bytes as u64);
        });
    }

    pub(super) fn record_event(&self, bytes: usize) {
        self.record(|counters| {
            counters.events = counters.events.saturating_add(1);
            counters.event_bytes = counters.event_bytes.saturating_add(bytes as u64);
        });
    }

    pub(super) fn record_stderr_line(&self) {
        self.record(|counters| {
            counters.stderr_lines = counters.stderr_lines.saturating_add(1);
        });
    }

    pub(super) fn record_invalid_stdout(&self) {
        self.record(|counters| {
            counters.stdout_invalid = counters.stdout_invalid.saturating_add(1);
        });
    }

    pub(super) fn record_stdout_closed(&self) {
        self.record(|counters| {
            counters.stdout_closed = counters.stdout_closed.saturating_add(1);
        });
    }

    fn record(&self, update: impl FnOnce(&mut ProcessBridgeCounters)) {
        if !self.enabled {
            return;
        }
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        update(&mut state.counters);
        let now = Instant::now();
        if now.duration_since(state.last_flush) < self.flush_interval {
            return;
        }
        eprintln!(
            "[pi-workspaces bridge-trace] requests={} request_bytes={} responses={} response_bytes={} events={} event_bytes={} stderr_lines={} stdout_invalid={} stdout_closed={} max_pending={}",
            state.counters.requests,
            state.counters.request_bytes,
            state.counters.responses,
            state.counters.response_bytes,
            state.counters.events,
            state.counters.event_bytes,
            state.counters.stderr_lines,
            state.counters.stdout_invalid,
            state.counters.stdout_closed,
            state.counters.max_pending,
        );
        state.counters = ProcessBridgeCounters::default();
        state.last_flush = now;
    }
}

fn trace_enabled() -> bool {
    std::env::var("PI_WORKSPACES_BRIDGE_TRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
}

fn flush_interval() -> Duration {
    std::env::var("PI_WORKSPACES_BRIDGE_TRACE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_secs(1))
}
