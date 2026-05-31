use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result};
use pi_bridge_types::{InitCommand, ProviderAuthStatus, SessionTarget};
use pi_sdk_bridge::{BridgeClient, NodeHostTransport};

pub struct BackendSession {
    runtime: tokio::runtime::Runtime,
    client: Arc<BridgeClient<NodeHostTransport>>,
}

#[derive(Clone)]
pub struct BackendData {
    pub auth: Vec<ProviderAuthStatus>,
}

pub struct BackendSnapshot {
    pub session: Arc<BackendSession>,
    pub data: BackendData,
}

impl BackendSession {
    pub fn connect(cwd: PathBuf) -> Result<BackendSnapshot> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("create frontend backend runtime")?;
        let bootstrap_path = bootstrap_path();
        let libnode_path = libnode_path();
        let cwd_text = cwd.display().to_string();
        let client = runtime.block_on(async move {
            let config = pi_node_host::NodeHostConfig::new(libnode_path, bootstrap_path);
            let host = Arc::new(pi_node_host::NodeHost::start(config).await?);
            host.wait_until_ready().await?;
            let client = Arc::new(BridgeClient::new(NodeHostTransport::new(host)));
            client
                .init(InitCommand {
                    cwd: cwd_text,
                    agent_dir: None,
                    session: Some(SessionTarget::New),
                    model: None,
                    tools: None,
                    enable_extensions: true,
                    test_mode: None,
                })
                .await?;
            Result::<_>::Ok(client)
        })?;
        let session = Arc::new(Self { runtime, client });
        let data = session.collect_data()?;
        Ok(BackendSnapshot { session, data })
    }

    pub fn refresh(&self) -> Result<BackendData> {
        self.runtime.block_on(self.client.state())?;
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

    pub fn remove_auth(&self, provider: String) -> Result<BackendData> {
        self.runtime.block_on(self.client.remove_auth(provider))?;
        self.refresh()
    }

    pub fn prompt(&self, text: String) -> Result<BackendData> {
        self.runtime.block_on(self.client.prompt(text))?;
        self.refresh()
    }

    fn collect_data(&self) -> Result<BackendData> {
        let auth = self.runtime.block_on(self.client.auth_status(None))?;
        Ok(BackendData { auth })
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
