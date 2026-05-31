#![allow(clippy::large_enum_variant)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

mod command;
mod error;
mod event;
mod extension_ui;
mod response;
mod state;

pub use command::*;
pub use error::*;
pub use event::*;
pub use extension_ui::*;
pub use response::*;
pub use state::*;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
)]
#[ts(export)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7().to_string())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&str> for RequestId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BridgeCommandEnvelope {
    pub version: u16,
    pub request_id: RequestId,
    pub command: BridgeCommand,
}

impl BridgeCommandEnvelope {
    pub fn new(request_id: RequestId, command: BridgeCommand) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            request_id,
            command,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BridgeResponseEnvelope {
    pub version: u16,
    pub request_id: RequestId,
    pub response: BridgeResponsePayload,
}

impl BridgeResponseEnvelope {
    pub fn ok(request_id: RequestId, value: BridgeResponse) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            request_id,
            response: BridgeResponsePayload::Ok { value },
        }
    }

    pub fn error(request_id: RequestId, error: BridgeError) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            request_id,
            response: BridgeResponsePayload::Error { error },
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BridgeEventEnvelope {
    pub version: u16,
    pub event: BridgeEvent,
}

impl BridgeEventEnvelope {
    pub fn new(event: BridgeEvent) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            event,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_envelope_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
        let envelope = BridgeCommandEnvelope::new(
            RequestId::from("req-1"),
            BridgeCommand::Prompt(PromptCommand {
                text: "hello".to_owned(),
                images: Vec::new(),
                streaming_behavior: None,
            }),
        );
        let json = serde_json::to_string_pretty(&envelope)?;
        let restored: BridgeCommandEnvelope = serde_json::from_str(&json)?;
        assert_eq!(envelope, restored);
        insta::assert_json_snapshot!(restored);
        Ok(())
    }

    #[test]
    fn ui_request_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
        let event = BridgeEventEnvelope::new(BridgeEvent::ExtensionUiRequest {
            request: ExtensionUiRequest::Select(SelectRequest {
                id: UiRequestId("ui-1".to_owned()),
                title: "Pick".to_owned(),
                options: vec!["a".to_owned(), "b".to_owned()],
                timeout_ms: Some(1000),
            }),
        });
        let json = serde_json::to_value(&event)?;
        let restored: BridgeEventEnvelope = serde_json::from_value(json)?;
        assert_eq!(event, restored);
        Ok(())
    }
}
