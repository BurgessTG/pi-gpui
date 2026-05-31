#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BridgeError {
    pub code: BridgeErrorCode,
    pub message: String,
    pub details: Option<String>,
    pub retryable: bool,
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for BridgeError {}

impl BridgeError {
    pub fn new(code: BridgeErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            retryable: false,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
    strum::Display,
)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum BridgeErrorCode {
    ProtocolVersionMismatch,
    InvalidCommand,
    InvalidPayload,
    NotInitialized,
    AlreadyInitialized,
    PiSdkError,
    NodeRuntimeError,
    RequestCancelled,
    RequestTimedOut,
    HostShuttingDown,
    Unsupported,
    Internal,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u16, actual: u16 },
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}
