use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Context as _, Result};
use pi_bridge_types::{
    AuthFlowUpdate, BridgeEvent, BridgeEventEnvelope, CoreStateSnapshot, ForkPosition, InitCommand,
    InstalledPackage, OAuthLoginMethod, PackageSearchResponse, ProviderAuthStatus, SessionTarget,
};
use pi_sdk_bridge::{BridgeClient, NodeHostTransport};
use tokio::sync::broadcast;
use tokio_stream::StreamExt as _;

pub struct BackendSession {
    runtime: tokio::runtime::Runtime,
    client: Arc<BridgeClient<NodeHostTransport>>,
    auth_updates: broadcast::Sender<AuthFlowUpdate>,
    event_updates: broadcast::Sender<BridgeEventEnvelope>,
    agent_ready: AtomicBool,
    session_command_lock: Mutex<()>,
    cwd: PathBuf,
}

#[derive(Clone)]
pub struct BackendData {
    pub auth: Vec<ProviderAuthStatus>,
    pub packages: Vec<InstalledPackage>,
    pub agent_ready: bool,
    pub state: Option<CoreStateSnapshot>,
}

pub struct BackendSnapshot {
    pub session: Arc<BackendSession>,
    pub data: BackendData,
}

pub struct SessionCommandResult {
    pub data: BackendData,
    pub cancelled: bool,
}

impl BackendSession {
    pub fn connect(_cwd: PathBuf) -> Result<BackendSnapshot> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("create frontend backend runtime")?;
        let bootstrap_path = bootstrap_path();
        let libnode_path = libnode_path();
        let (auth_updates, _) = broadcast::channel(128);
        let (event_updates, _) = broadcast::channel(512);
        let auth_events = auth_updates.clone();
        let bridge_events = event_updates.clone();
        let cwd_for_packages = _cwd.clone();
        let (client, auth, packages) = runtime.block_on(async move {
            let mut config = pi_node_host::NodeHostConfig::new(libnode_path, bootstrap_path);
            config.request_timeout = Duration::from_secs(20 * 60);
            let host = Arc::new(pi_node_host::NodeHost::start(config).await?);
            let mut events = host.subscribe();
            tokio::spawn(async move {
                while let Some(Ok(event)) = events.next().await {
                    if let BridgeEvent::AuthFlowUpdate { update } = &event.event {
                        let _ = auth_events.send(update.clone());
                    }
                    let _ = bridge_events.send(event);
                }
            });
            host.wait_until_ready().await?;
            let client = Arc::new(BridgeClient::new(NodeHostTransport::new(host)));
            let auth = client.auth_status(None).await?;
            let packages = client
                .list_packages(cwd_for_packages.display().to_string())
                .await
                .unwrap_or_default();
            Result::<_>::Ok((client, auth, packages))
        })?;
        let session = Arc::new(Self {
            runtime,
            client,
            auth_updates,
            event_updates,
            agent_ready: AtomicBool::new(false),
            session_command_lock: Mutex::new(()),
            cwd: _cwd,
        });
        let data = BackendData {
            auth,
            packages,
            agent_ready: false,
            state: None,
        };
        Ok(BackendSnapshot { session, data })
    }

    pub fn subscribe_auth_updates(&self) -> broadcast::Receiver<AuthFlowUpdate> {
        self.auth_updates.subscribe()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<BridgeEventEnvelope> {
        self.event_updates.subscribe()
    }

    pub fn init_runtime(&self, cwd: PathBuf) -> Result<BackendData> {
        if !self.agent_ready.load(Ordering::Acquire) {
            self.runtime.block_on(self.client.init(InitCommand {
                cwd: cwd.display().to_string(),
                agent_dir: None,
                session: Some(SessionTarget::New),
                model: None,
                tools: None,
                enable_extensions: true,
                test_mode: None,
            }))?;
            self.agent_ready.store(true, Ordering::Release);
        }
        self.collect_data()
    }

    pub fn refresh(&self) -> Result<BackendData> {
        self.collect_data()
    }

    pub fn save_api_key(
        &self,
        provider: String,
        api_key: String,
        persist: bool,
    ) -> Result<BackendData> {
        if persist {
            self.runtime
                .block_on(self.client.set_persisted_api_key(provider, api_key))?;
        } else {
            self.runtime
                .block_on(self.client.set_runtime_api_key(provider, api_key))?;
        }
        self.refresh()
    }

    pub fn oauth_login(
        &self,
        provider: String,
        method: Option<OAuthLoginMethod>,
    ) -> Result<BackendData> {
        self.runtime
            .block_on(self.client.oauth_login(provider, method))?;
        self.refresh()
    }

    pub fn remove_auth(&self, provider: String) -> Result<BackendData> {
        self.runtime.block_on(self.client.remove_auth(provider))?;
        self.refresh()
    }

    pub fn search_packages(&self, query: String, limit: u32) -> Result<PackageSearchResponse> {
        Ok(self
            .runtime
            .block_on(self.client.search_packages(query, limit))?)
    }

    pub fn install_package(
        &self,
        source: String,
        project: bool,
        cwd: PathBuf,
    ) -> Result<BackendData> {
        self.runtime.block_on(self.client.install_package(
            source,
            project,
            cwd.display().to_string(),
        ))?;
        self.refresh()
    }

    pub fn remove_package(
        &self,
        source: String,
        project: bool,
        cwd: PathBuf,
    ) -> Result<BackendData> {
        self.runtime.block_on(self.client.remove_package(
            source,
            project,
            cwd.display().to_string(),
        ))?;
        self.refresh()
    }

    pub fn new_session(&self) -> Result<SessionCommandResult> {
        self.ensure_agent_ready()?;
        let _guard = self.lock_session_commands();
        let cancelled = self.runtime.block_on(self.client.new_session(None))?;
        Ok(SessionCommandResult {
            data: self.collect_data()?,
            cancelled,
        })
    }

    pub fn switch_session(&self, session_path: String) -> Result<SessionCommandResult> {
        self.ensure_agent_ready()?;
        let _guard = self.lock_session_commands();
        let cancelled = self
            .runtime
            .block_on(self.client.switch_session(session_path, None))?;
        Ok(SessionCommandResult {
            data: self.collect_data()?,
            cancelled,
        })
    }

    pub fn fork_session(&self, entry_id: String) -> Result<SessionCommandResult> {
        self.ensure_agent_ready()?;
        let _guard = self.lock_session_commands();
        self.runtime
            .block_on(self.client.fork_session(entry_id, ForkPosition::At))?;
        Ok(SessionCommandResult {
            data: self.collect_data()?,
            cancelled: false,
        })
    }

    pub fn prompt(&self, session_path: Option<String>, text: String) -> Result<BackendData> {
        self.ensure_agent_ready()?;
        let _guard = self.lock_session_commands();
        if let Some(session_path) = session_path {
            let current_path = self
                .runtime
                .block_on(self.client.state())?
                .session_file
                .unwrap_or_default();
            if current_path != session_path {
                let cancelled = self
                    .runtime
                    .block_on(self.client.switch_session(session_path, None))?;
                anyhow::ensure!(!cancelled, "Pi session switch was cancelled");
            }
        }
        self.runtime.block_on(self.client.prompt(text))?;
        self.collect_data()
    }

    #[allow(dead_code)]
    pub fn set_session_name(
        &self,
        session_path: Option<String>,
        name: String,
    ) -> Result<BackendData> {
        self.ensure_agent_ready()?;
        let _guard = self.lock_session_commands();
        if let Some(session_path) = session_path {
            let current_path = self
                .runtime
                .block_on(self.client.state())?
                .session_file
                .unwrap_or_default();
            if current_path != session_path {
                self.runtime
                    .block_on(self.client.switch_session(session_path, None))?;
            }
        }
        self.runtime.block_on(self.client.set_session_name(name))?;
        self.collect_data()
    }

    fn lock_session_commands(&self) -> MutexGuard<'_, ()> {
        self.session_command_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn ensure_agent_ready(&self) -> Result<()> {
        anyhow::ensure!(
            self.agent_ready.load(Ordering::Acquire),
            "Pi runtime is not ready yet"
        );
        Ok(())
    }

    fn collect_data(&self) -> Result<BackendData> {
        let auth = self.runtime.block_on(self.client.auth_status(None))?;
        let packages = self
            .runtime
            .block_on(self.client.list_packages(self.cwd_display()))
            .unwrap_or_default();
        let agent_ready = self.agent_ready.load(Ordering::Acquire);
        let state = if agent_ready {
            Some(self.runtime.block_on(self.client.state())?)
        } else {
            None
        };
        Ok(BackendData {
            auth,
            packages,
            agent_ready,
            state,
        })
    }

    fn cwd_display(&self) -> String {
        self.cwd.display().to_string()
    }
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or(manifest)
}

fn bootstrap_path() -> PathBuf {
    std::env::var_os("PI_GPUI_BOOTSTRAP")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root().join("node/dist/bootstrap.js"))
}

fn libnode_path() -> PathBuf {
    std::env::var_os("PI_GPUI_LIBNODE")
        .or_else(|| std::env::var_os("EDON_LIBNODE_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root().join(".libnode/v24.4.1/libnode.so"))
}
