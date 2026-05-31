use crate::error::BridgeError;
use crate::extension_ui::{ComponentRenderRequest, ExtensionUiRequest, ExtensionUiUpdate};
use crate::state::{CoreStateSnapshot, Diagnostic, QueueSnapshot};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum BridgeEvent {
    Ready(ReadyEvent),
    Log(LogEvent),
    FatalError { error: BridgeError },
    Diagnostics { diagnostics: Vec<Diagnostic> },
    StateSnapshot { state: CoreStateSnapshot },
    PiSessionEvent { event: serde_json::Value },
    QueueUpdate { queue: QueueSnapshot },
    BashChunk { chunk: String },
    ExtensionUiRequest { request: ExtensionUiRequest },
    ExtensionUiUpdate { update: ExtensionUiUpdate },
    ComponentRenderRequest { request: ComponentRenderRequest },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ReadyEvent {
    pub node_version: String,
    pub pi_version: Option<String>,
    pub protocol_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
