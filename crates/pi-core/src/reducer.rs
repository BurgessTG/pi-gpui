use pi_bridge_types::{BridgeEvent, BridgeEventEnvelope, ExtensionUiRequest};

use crate::{BackendState, TranscriptItem};

#[derive(Debug, thiserror::Error)]
pub enum ReducerError {
    #[error("unsupported protocol version {0}")]
    UnsupportedProtocol(u16),
}

pub trait ApplyEvent {
    fn apply_event(&mut self, envelope: BridgeEventEnvelope) -> Result<(), ReducerError>;
}

impl ApplyEvent for BackendState {
    fn apply_event(&mut self, envelope: BridgeEventEnvelope) -> Result<(), ReducerError> {
        if envelope.version != pi_bridge_types::PROTOCOL_VERSION {
            return Err(ReducerError::UnsupportedProtocol(envelope.version));
        }
        match envelope.event {
            BridgeEvent::Ready(_ready) => self.ready = true,
            BridgeEvent::Log(log) => self.logs.push(log.message),
            BridgeEvent::FatalError { error } => self.fatal_error = Some(error),
            BridgeEvent::Diagnostics { diagnostics } => self.snapshot.diagnostics = diagnostics,
            BridgeEvent::StateSnapshot { state } => self.snapshot = state,
            BridgeEvent::PiSessionEvent { event, .. } => self.apply_session_event(event),
            BridgeEvent::SessionRunStarted { .. } => {
                self.transcript
                    .push(TranscriptItem::SessionEvent(serde_json::json!({
                        "type": "agent_start"
                    })))
            }
            BridgeEvent::SessionRunFinished { .. } => {
                self.transcript
                    .push(TranscriptItem::SessionEvent(serde_json::json!({
                        "type": "agent_end"
                    })))
            }
            BridgeEvent::SessionRunError { message, .. } => {
                self.transcript
                    .push(TranscriptItem::SessionEvent(serde_json::json!({
                        "type": "agent_error",
                        "message": message,
                    })))
            }
            BridgeEvent::SessionTextDelta { delta, .. } => {
                self.transcript.push(TranscriptItem::TextDelta(delta));
            }
            BridgeEvent::SessionToolStarted {
                tool_call_id,
                tool_name,
                args,
                ..
            } => self
                .transcript
                .push(TranscriptItem::ToolUpdate(serde_json::json!({
                    "type": "tool_execution_start",
                    "toolCallId": tool_call_id,
                    "toolName": tool_name,
                    "args": args,
                }))),
            BridgeEvent::SessionToolUpdated {
                tool_call_id,
                tool_name,
                args,
                partial_result,
                ..
            } => self
                .transcript
                .push(TranscriptItem::ToolUpdate(serde_json::json!({
                    "type": "tool_execution_update",
                    "toolCallId": tool_call_id,
                    "toolName": tool_name,
                    "args": args,
                    "partialResult": partial_result,
                }))),
            BridgeEvent::SessionToolFinished {
                tool_call_id,
                tool_name,
                result,
                is_error,
                ..
            } => self
                .transcript
                .push(TranscriptItem::ToolUpdate(serde_json::json!({
                    "type": "tool_execution_end",
                    "toolCallId": tool_call_id,
                    "toolName": tool_name,
                    "result": result,
                    "isError": is_error,
                }))),
            BridgeEvent::QueueUpdate { queue, .. } => self.snapshot.queue = queue,
            BridgeEvent::BashChunk { chunk } => self.bash_chunks.push(chunk),
            BridgeEvent::ExtensionUiRequest { request } => self.store_ui_request(request),
            BridgeEvent::ExtensionUiUpdate { update } => self.ui_updates.push(update),
            BridgeEvent::AuthFlowUpdate { .. } => {}
            BridgeEvent::ComponentRenderRequest { request } => {
                self.component_lines.entry(request.handle).or_default();
            }
            BridgeEvent::Shutdown => self.ready = false,
        }
        Ok(())
    }
}

impl BackendState {
    fn apply_session_event(&mut self, event: serde_json::Value) {
        if event.get("type").and_then(serde_json::Value::as_str) == Some("message_update")
            && let Some(delta) = event
                .get("assistantMessageEvent")
                .and_then(|value| value.get("delta"))
                .and_then(serde_json::Value::as_str)
        {
            self.transcript
                .push(TranscriptItem::TextDelta(delta.to_owned()));
            return;
        }
        if event.get("type").and_then(serde_json::Value::as_str) == Some("tool_execution_update") {
            self.transcript.push(TranscriptItem::ToolUpdate(event));
            return;
        }
        self.transcript.push(TranscriptItem::SessionEvent(event));
    }

    fn store_ui_request(&mut self, request: ExtensionUiRequest) {
        let key = match &request {
            ExtensionUiRequest::Select(payload) => payload.id.0.clone(),
            ExtensionUiRequest::Confirm(payload) => payload.id.0.clone(),
            ExtensionUiRequest::Input(payload) => payload.id.0.clone(),
            ExtensionUiRequest::Editor(payload) => payload.id.0.clone(),
            ExtensionUiRequest::CustomComponent(payload) => payload.id.0.clone(),
            ExtensionUiRequest::Autocomplete(payload) => payload.id.0.clone(),
        };
        self.pending_ui.insert(key, request);
    }
}
