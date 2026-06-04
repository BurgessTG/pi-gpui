use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use pi_bridge_types::{
    BridgeCommand, BridgeError, BridgeErrorCode, BridgeEventEnvelope, BridgeResponse, InitCommand,
    SessionTarget,
};
use tokio::sync::broadcast;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::{NodeProcessHost, NodeProcessHostConfig, Result};

#[derive(Clone, Debug)]
pub struct NodeWorkerPoolConfig {
    pub process: NodeProcessHostConfig,
    pub max_session_workers: usize,
}

impl NodeWorkerPoolConfig {
    pub fn new(process: NodeProcessHostConfig) -> Self {
        Self {
            process,
            max_session_workers: 16,
        }
    }
}

pub struct NodeWorkerPool {
    primary: Arc<NodeProcessHost>,
    config: NodeWorkerPoolConfig,
    init_template: Mutex<Option<InitCommand>>,
    session_workers: Mutex<HashMap<String, Arc<NodeProcessHost>>>,
    session_start_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    events: broadcast::Sender<BridgeEventEnvelope>,
}

impl NodeWorkerPool {
    pub async fn start(config: NodeWorkerPoolConfig) -> Result<Self> {
        let primary = Arc::new(NodeProcessHost::start(config.process.clone()).await?);
        let (events, _rx) = broadcast::channel(2048);
        forward_worker_events(Arc::clone(&primary), events.clone());
        Ok(Self {
            primary,
            config,
            init_template: Mutex::new(None),
            session_workers: Mutex::new(HashMap::new()),
            session_start_locks: Mutex::new(HashMap::new()),
            events,
        })
    }

    pub fn subscribe(&self) -> BroadcastStream<BridgeEventEnvelope> {
        BroadcastStream::new(self.events.subscribe())
    }

    pub async fn request(&self, command: BridgeCommand) -> Result<BridgeResponse> {
        match command {
            BridgeCommand::Init(command) => {
                *self.init_template.lock() = Some(command.clone());
                self.primary.request(BridgeCommand::Init(command)).await
            }
            BridgeCommand::Prompt(mut command) => {
                let Some(session_path) = command.session_path.take() else {
                    return self.primary.request(BridgeCommand::Prompt(command)).await;
                };
                self.session_worker(&session_path)
                    .await?
                    .request(BridgeCommand::Prompt(command))
                    .await
            }
            BridgeCommand::GetSessionState(command) => {
                self.session_worker(&command.session_path)
                    .await?
                    .request(BridgeCommand::GetState)
                    .await
            }
            BridgeCommand::SetSessionName(mut command) => {
                let Some(session_path) = command.session_path.take() else {
                    return self
                        .primary
                        .request(BridgeCommand::SetSessionName(command))
                        .await;
                };
                self.session_worker(&session_path)
                    .await?
                    .request(BridgeCommand::SetSessionName(command))
                    .await
            }
            BridgeCommand::Shutdown => {
                self.shutdown().await?;
                Ok(BridgeResponse::Ack)
            }
            command => self.primary.request(command).await,
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        let workers = {
            let mut workers = self.session_workers.lock();
            workers
                .drain()
                .map(|(_path, worker)| worker)
                .collect::<Vec<_>>()
        };
        for worker in workers {
            worker.shutdown().await?;
        }
        self.primary.shutdown().await
    }

    async fn session_worker(&self, session_path: &str) -> Result<Arc<NodeProcessHost>> {
        if let Some(worker) = self.session_workers.lock().get(session_path).cloned() {
            return Ok(worker);
        }
        let start_lock = self.session_start_lock(session_path);
        let _guard = start_lock.lock().await;
        if let Some(worker) = self.session_workers.lock().get(session_path).cloned() {
            return Ok(worker);
        }
        if self.session_workers.lock().len() >= self.config.max_session_workers.max(1) {
            return Err(crate::NodeHostError::Bridge(
                BridgeError::new(
                    BridgeErrorCode::RequestTimedOut,
                    format!(
                        "maximum Pi session worker count reached ({})",
                        self.config.max_session_workers.max(1)
                    ),
                )
                .retryable(true),
            ));
        }

        let init_command = self.init_command_for_session(session_path)?;
        let worker = Arc::new(NodeProcessHost::start(self.config.process.clone()).await?);
        forward_worker_events(Arc::clone(&worker), self.events.clone());
        worker.request(BridgeCommand::Init(init_command)).await?;
        self.session_workers
            .lock()
            .insert(session_path.to_owned(), Arc::clone(&worker));
        Ok(worker)
    }

    fn session_start_lock(&self, session_path: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.session_start_locks
            .lock()
            .entry(session_path.to_owned())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    fn init_command_for_session(&self, session_path: &str) -> Result<InitCommand> {
        let Some(mut command) = self.init_template.lock().clone() else {
            return Err(crate::NodeHostError::Bridge(BridgeError::new(
                BridgeErrorCode::NotInitialized,
                "Pi runtime has not been initialized",
            )));
        };
        command.session = Some(SessionTarget::Open {
            path: session_path.to_owned(),
        });
        Ok(command)
    }
}

fn forward_worker_events(
    worker: Arc<NodeProcessHost>,
    events: broadcast::Sender<BridgeEventEnvelope>,
) {
    tokio::spawn(async move {
        let mut stream = worker.subscribe();
        while let Some(Ok(event)) = stream.next().await {
            let _send_result = events.send(event);
        }
    });
}
