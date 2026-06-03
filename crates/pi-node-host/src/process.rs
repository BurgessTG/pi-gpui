use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use pi_bridge_types::{
    BridgeCommand, BridgeCommandEnvelope, BridgeError, BridgeErrorCode, BridgeEvent,
    BridgeEventEnvelope, BridgeResponse, LogEvent, LogLevel, ReadyEvent, RequestId,
};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{broadcast, watch};
use tokio_stream::wrappers::BroadcastStream;

use crate::native::NativeBridgeState;
use crate::process_metrics::ProcessBridgeMetrics;
use crate::{NodeHostError, Result};

#[derive(Clone, Debug)]
pub struct NodeProcessHostConfig {
    pub node_path: PathBuf,
    pub process_host_path: PathBuf,
    pub request_timeout: std::time::Duration,
    pub max_pending_requests: usize,
}

impl NodeProcessHostConfig {
    pub fn new(node_path: impl Into<PathBuf>, process_host_path: impl Into<PathBuf>) -> Self {
        Self {
            node_path: node_path.into(),
            process_host_path: process_host_path.into(),
            request_timeout: std::time::Duration::from_secs(20 * 60),
            max_pending_requests: 256,
        }
    }
}

pub struct NodeProcessHost {
    child: Arc<Mutex<Child>>,
    stdin: Arc<tokio::sync::Mutex<ChildStdin>>,
    native: Arc<NativeBridgeState>,
    metrics: Arc<ProcessBridgeMetrics>,
    events: broadcast::Sender<BridgeEventEnvelope>,
    ready: watch::Receiver<Option<ReadyEvent>>,
    request_timeout: std::time::Duration,
    max_pending_requests: usize,
}

impl NodeProcessHost {
    pub async fn start(config: NodeProcessHostConfig) -> Result<Self> {
        if !config.process_host_path.is_file() {
            return Err(NodeHostError::MissingBootstrap(
                config.process_host_path.display().to_string(),
            ));
        }

        let mut child = Command::new(&config.node_path)
            .arg(&config.process_host_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or(NodeHostError::MissingProcessStdin)?;
        let stdout = child
            .stdout
            .take()
            .ok_or(NodeHostError::MissingProcessStdout)?;
        let stderr = child
            .stderr
            .take()
            .ok_or(NodeHostError::MissingProcessStderr)?;

        let (events, _events_rx) = broadcast::channel(1024);
        let (ready_tx, ready_rx) = watch::channel(None);
        let native = Arc::new(NativeBridgeState::new(events.clone(), ready_tx));
        let metrics = Arc::new(ProcessBridgeMetrics::from_env());
        spawn_stdout_reader(stdout, Arc::clone(&native), Arc::clone(&metrics));
        spawn_stderr_reader(stderr, events.clone(), Arc::clone(&metrics));

        let host = Self {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(tokio::sync::Mutex::new(stdin)),
            native,
            metrics,
            events,
            ready: ready_rx,
            request_timeout: config.request_timeout,
            max_pending_requests: config.max_pending_requests.max(1),
        };
        host.wait_until_ready().await?;
        Ok(host)
    }

    pub fn subscribe(&self) -> BroadcastStream<BridgeEventEnvelope> {
        BroadcastStream::new(self.events.subscribe())
    }

    pub async fn wait_until_ready(&self) -> Result<ReadyEvent> {
        let mut ready = self.ready.clone();
        if let Some(value) = ready.borrow().clone() {
            return Ok(value);
        }
        tokio::time::timeout(self.request_timeout, async move {
            loop {
                ready
                    .changed()
                    .await
                    .map_err(|_closed| NodeHostError::RequestCancelled)?;
                if let Some(value) = ready.borrow().clone() {
                    return Ok(value);
                }
            }
        })
        .await
        .map_err(|_elapsed| NodeHostError::RequestTimedOut)?
    }

    pub async fn request(&self, command: BridgeCommand) -> Result<BridgeResponse> {
        let request_id = RequestId::new();
        let envelope = BridgeCommandEnvelope::new(request_id.clone(), command);
        let (tx, rx) = tokio::sync::oneshot::channel();
        let pending_len = {
            let mut pending = self.native.pending.lock();
            if pending.len() >= self.max_pending_requests {
                return Err(NodeHostError::Bridge(
                    BridgeError::new(
                        BridgeErrorCode::RequestTimedOut,
                        format!(
                            "Node worker has too many pending requests ({})",
                            self.max_pending_requests
                        ),
                    )
                    .retryable(true),
                ));
            }
            pending.insert(request_id.clone(), tx);
            pending.len()
        };

        if let Err(error) = self.dispatch_to_node(&envelope, pending_len).await {
            let _removed = self.native.pending.lock().remove(&request_id);
            return Err(error);
        }

        let result = match tokio::time::timeout(self.request_timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_closed)) => {
                let _removed = self.native.pending.lock().remove(&request_id);
                return Err(NodeHostError::RequestCancelled);
            }
            Err(_elapsed) => {
                let _removed = self.native.pending.lock().remove(&request_id);
                return Err(NodeHostError::RequestTimedOut);
            }
        };
        result.map_err(Into::into)
    }

    pub async fn shutdown(&self) -> Result<()> {
        match self.request(BridgeCommand::Shutdown).await {
            Ok(_) | Err(NodeHostError::Bridge(_)) => {
                let _kill_result = self.child.lock().start_kill();
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    async fn dispatch_to_node(
        &self,
        envelope: &BridgeCommandEnvelope,
        pending_len: usize,
    ) -> Result<()> {
        self.ensure_child_running()?;
        let line = serde_json::to_string(envelope)?;
        self.metrics.record_request(line.len(), pending_len);
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(line.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    fn ensure_child_running(&self) -> Result<()> {
        if let Some(status) = self.child.lock().try_wait()? {
            return Err(NodeHostError::ProcessExited(status.to_string()));
        }
        Ok(())
    }
}

impl Drop for NodeProcessHost {
    fn drop(&mut self) {
        let _kill_result = self.child.lock().start_kill();
    }
}

fn spawn_stdout_reader(
    stdout: tokio::process::ChildStdout,
    native: Arc<NativeBridgeState>,
    metrics: Arc<ProcessBridgeMetrics>,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => handle_stdout_line(&native, &metrics, &line),
                Ok(None) => {
                    metrics.record_stdout_closed();
                    fail_worker_io(
                        &native,
                        BridgeError::new(
                            BridgeErrorCode::NodeRuntimeError,
                            "Node worker stdout closed",
                        ),
                    );
                    break;
                }
                Err(error) => {
                    fail_worker_io(
                        &native,
                        BridgeError::new(
                            BridgeErrorCode::NodeRuntimeError,
                            "failed to read Node worker stdout",
                        )
                        .with_details(error.to_string()),
                    );
                    break;
                }
            }
        }
    });
}

fn handle_stdout_line(native: &NativeBridgeState, metrics: &ProcessBridgeMetrics, line: &str) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        metrics.record_invalid_stdout();
        native.emit_event(BridgeEventEnvelope::new(BridgeEvent::Log(LogEvent {
            level: LogLevel::Warn,
            message: format!("Ignoring non-JSON Node worker stdout: {trimmed}"),
        })));
        return;
    };
    if value.get("response").is_some() {
        metrics.record_response(trimmed.len());
        if let Ok(response) = serde_json::from_value(value) {
            native.complete_response(response);
        }
    } else if value.get("event").is_some()
        && let Ok(event) = serde_json::from_value(value)
    {
        metrics.record_event(trimmed.len());
        native.emit_event(event);
    }
}

fn fail_worker_io(native: &NativeBridgeState, error: BridgeError) {
    native.fail_pending(error.clone());
    native.emit_event(BridgeEventEnvelope::new(BridgeEvent::FatalError { error }));
}

fn spawn_stderr_reader(
    stderr: tokio::process::ChildStderr,
    events: broadcast::Sender<BridgeEventEnvelope>,
    metrics: Arc<ProcessBridgeMetrics>,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let message = line.trim();
            if message.is_empty() {
                continue;
            }
            metrics.record_stderr_line();
            let _send_result = events.send(BridgeEventEnvelope::new(BridgeEvent::Log(LogEvent {
                level: LogLevel::Info,
                message: message.to_owned(),
            })));
        }
    });
}
