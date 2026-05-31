use crate::error::BridgeError;
use crate::extension_ui::{AutocompleteItem, ComponentRenderResult};
use crate::state::{CoreStateSnapshot, ModelDescriptor, ProviderAuthStatus, ToolDescriptor};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum BridgeResponsePayload {
    Ok { value: BridgeResponse },
    Error { error: BridgeError },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum BridgeResponse {
    Ack,
    State { state: CoreStateSnapshot },
    Messages { messages: Vec<serde_json::Value> },
    SessionStats { stats: serde_json::Value },
    AuthStatus { statuses: Vec<ProviderAuthStatus> },
    Models { models: Vec<ModelDescriptor> },
    Tools { tools: Vec<ToolDescriptor> },
    Path { path: String },
    Text { text: String },
    Json { value: serde_json::Value },
    Cancelled { cancelled: bool },
    Autocomplete { items: Vec<AutocompleteItem> },
    ComponentRender { render: ComponentRenderResult },
}
