use std::sync::Arc;

use async_trait::async_trait;
use pi_bridge_types::{BridgeCommand, BridgeResponse};

use crate::Result;

#[async_trait]
pub trait BridgeTransport: Send + Sync {
    async fn request(&self, command: BridgeCommand) -> Result<BridgeResponse>;
}

#[derive(Clone)]
pub struct NodeWorkerPoolTransport {
    pool: Arc<pi_node_host::NodeWorkerPool>,
}

impl NodeWorkerPoolTransport {
    pub fn new(pool: Arc<pi_node_host::NodeWorkerPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BridgeTransport for NodeWorkerPoolTransport {
    async fn request(&self, command: BridgeCommand) -> Result<BridgeResponse> {
        Ok(self.pool.request(command).await?)
    }
}

#[derive(Clone)]
pub struct NodeProcessTransport {
    host: Arc<pi_node_host::NodeProcessHost>,
}

impl NodeProcessTransport {
    pub fn new(host: Arc<pi_node_host::NodeProcessHost>) -> Self {
        Self { host }
    }
}

#[async_trait]
impl BridgeTransport for NodeProcessTransport {
    async fn request(&self, command: BridgeCommand) -> Result<BridgeResponse> {
        Ok(self.host.request(command).await?)
    }
}

#[derive(Clone)]
pub struct NodeHostTransport {
    host: Arc<pi_node_host::NodeHost>,
}

impl NodeHostTransport {
    pub fn new(host: Arc<pi_node_host::NodeHost>) -> Self {
        Self { host }
    }
}

#[async_trait]
impl BridgeTransport for NodeHostTransport {
    async fn request(&self, command: BridgeCommand) -> Result<BridgeResponse> {
        Ok(self.host.request(command).await?)
    }
}
