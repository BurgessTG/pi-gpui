use std::collections::HashMap;

use parking_lot::Mutex;
use pi_bridge_types::{
    BridgeError, BridgeEventEnvelope, BridgeResponseEnvelope, BridgeResponsePayload, ReadyEvent,
    RequestId,
};
use pi_edon::napi::JsString;
use tokio::sync::{broadcast, oneshot, watch};

pub type PendingMap = Mutex<
    HashMap<RequestId, oneshot::Sender<Result<pi_bridge_types::BridgeResponse, BridgeError>>>,
>;

pub struct NativeBridgeState {
    pub pending: PendingMap,
    pub events: broadcast::Sender<BridgeEventEnvelope>,
    pub ready: watch::Sender<Option<ReadyEvent>>,
}

impl NativeBridgeState {
    pub fn new(
        events: broadcast::Sender<BridgeEventEnvelope>,
        ready: watch::Sender<Option<ReadyEvent>>,
    ) -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            events,
            ready,
        }
    }

    pub fn complete_response(&self, response: BridgeResponseEnvelope) {
        let sender = self.pending.lock().remove(&response.request_id);
        if let Some(sender) = sender {
            let payload = match response.response {
                BridgeResponsePayload::Ok { value } => Ok(value),
                BridgeResponsePayload::Error { error } => Err(error),
            };
            let _send_result = sender.send(payload);
        }
    }

    pub fn emit_event(&self, event: BridgeEventEnvelope) {
        if let pi_bridge_types::BridgeEvent::Ready(ready) = &event.event {
            let _ready_result = self.ready.send(Some(ready.clone()));
        }
        let _event_result = self.events.send(event);
    }
}

pub fn js_string_arg(ctx: &pi_edon::napi::CallContext<'_>) -> pi_edon::napi::Result<String> {
    Ok(ctx.get::<JsString>(0)?.into_utf8()?.as_str()?.to_owned())
}
