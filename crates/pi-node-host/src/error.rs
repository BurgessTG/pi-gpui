pub type Result<T> = std::result::Result<T, NodeHostError>;

#[derive(Debug, thiserror::Error)]
pub enum NodeHostError {
    #[error("libnode path is not configured")]
    MissingLibnode,
    #[error("bootstrap module does not exist: {0}")]
    MissingBootstrap(String),
    #[error("embedded Node error: {0}")]
    EmbeddedNode(#[from] pi_edon::EdonBoundaryError),
    #[error("protocol error: {0}")]
    Protocol(#[from] pi_bridge_types::ProtocolError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("request timed out")]
    RequestTimedOut,
    #[error("request was cancelled before Node responded")]
    RequestCancelled,
    #[error("Node host task failed: {0}")]
    Join(String),
    #[error("Pi bridge error: {0}")]
    Bridge(#[from] pi_bridge_types::BridgeError),
}
