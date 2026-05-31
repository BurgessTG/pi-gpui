pub type Result<T> = std::result::Result<T, BridgeClientError>;

#[derive(Debug, thiserror::Error)]
pub enum BridgeClientError {
    #[error("Node host error: {0}")]
    NodeHost(#[from] pi_node_host::NodeHostError),
    #[error("unexpected response for command: {0}")]
    UnexpectedResponse(&'static str),
    #[error("transport error: {0}")]
    Transport(String),
}
