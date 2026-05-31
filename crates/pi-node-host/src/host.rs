use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, Weak};

use parking_lot::Mutex;
use pi_bridge_types::{
    BridgeCommand, BridgeCommandEnvelope, BridgeError, BridgeErrorCode, BridgeEventEnvelope,
    BridgeResponse, ReadyEvent, RequestId,
};
use tokio::sync::{broadcast, oneshot, watch};
use tokio_stream::wrappers::BroadcastStream;

use crate::native::{NativeBridgeState, js_string_arg};
use crate::{NodeHostConfig, NodeHostError, Result};

const NATIVE_MODULE_NAME: &str = "pi_gpui_bridge";
static BOOTSTRAP_COUNTER: AtomicUsize = AtomicUsize::new(0);
static NATIVE_TARGET: OnceLock<Arc<Mutex<Option<Weak<NativeBridgeState>>>>> = OnceLock::new();
static NATIVE_MODULE_REGISTERED: OnceLock<()> = OnceLock::new();

fn current_native(
    target: &Arc<Mutex<Option<Weak<NativeBridgeState>>>>,
) -> Option<Arc<NativeBridgeState>> {
    target.lock().as_ref().and_then(Weak::upgrade)
}

pub struct NodeHost {
    node: Arc<Mutex<pi_edon::EmbeddedNode>>,
    native: Arc<NativeBridgeState>,
    events: broadcast::Sender<BridgeEventEnvelope>,
    ready: watch::Receiver<Option<ReadyEvent>>,
    request_timeout: std::time::Duration,
}

impl NodeHost {
    pub async fn start(config: NodeHostConfig) -> Result<Self> {
        if !config.bootstrap_path.is_file() {
            return Err(NodeHostError::MissingBootstrap(
                config.bootstrap_path.display().to_string(),
            ));
        }

        let node = pi_edon::EmbeddedNode::load(pi_edon::EmbeddedNodeConfig::new(
            config.libnode_path.clone(),
        ))?;
        let (events, _events_rx) = broadcast::channel(512);
        let (ready_tx, ready_rx) = watch::channel(None);
        let native = Arc::new(NativeBridgeState::new(events.clone(), ready_tx));
        Self::register_native_module(&node, Arc::clone(&native))?;

        let host = Self {
            node: Arc::new(Mutex::new(node)),
            native,
            events,
            ready: ready_rx,
            request_timeout: config.request_timeout,
        };
        host.load_bootstrap(&config.bootstrap_path).await?;
        Ok(host)
    }

    pub fn subscribe(&self) -> BroadcastStream<BridgeEventEnvelope> {
        BroadcastStream::new(self.events.subscribe())
    }

    pub async fn wait_until_ready(&self) -> Result<ReadyEvent> {
        let mut ready = self.ready.clone();
        if let Some(value) = ready.borrow().clone() {
            return Ok(value);
        }
        tokio::time::timeout(self.request_timeout, async move {
            loop {
                ready
                    .changed()
                    .await
                    .map_err(|_closed| NodeHostError::RequestCancelled)?;
                if let Some(value) = ready.borrow().clone() {
                    return Ok(value);
                }
            }
        })
        .await
        .map_err(|_elapsed| NodeHostError::RequestTimedOut)?
    }

    pub async fn request(&self, command: BridgeCommand) -> Result<BridgeResponse> {
        let request_id = RequestId::new();
        let envelope = BridgeCommandEnvelope::new(request_id.clone(), command);
        let (tx, rx) = oneshot::channel();
        self.native.pending.lock().insert(request_id.clone(), tx);

        if let Err(error) = self.dispatch_to_node(&envelope).await {
            let _removed = self.native.pending.lock().remove(&request_id);
            return Err(error);
        }

        let result = match tokio::time::timeout(self.request_timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_closed)) => {
                let _removed = self.native.pending.lock().remove(&request_id);
                return Err(NodeHostError::RequestCancelled);
            }
            Err(_elapsed) => {
                let _removed = self.native.pending.lock().remove(&request_id);
                return Err(NodeHostError::RequestTimedOut);
            }
        };
        result.map_err(Into::into)
    }

    pub async fn shutdown(&self) -> Result<()> {
        match self.request(BridgeCommand::Shutdown).await {
            Ok(_response) => Ok(()),
            Err(NodeHostError::Bridge(_bridge_error)) => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn register_native_module(
        node: &pi_edon::EmbeddedNode,
        native: Arc<NativeBridgeState>,
    ) -> Result<()> {
        let target = Arc::clone(NATIVE_TARGET.get_or_init(|| Arc::new(Mutex::new(None))));
        *target.lock() = Some(Arc::downgrade(&native));
        if NATIVE_MODULE_REGISTERED.get().is_some() {
            return Ok(());
        }

        node.register_module(NATIVE_MODULE_NAME, move |env, mut exports| {
            let events_target = Arc::clone(&target);
            let emit_event = env.create_function_from_closure("emitEvent", move |ctx| {
                let json = js_string_arg(&ctx)?;
                if let Some(events) = current_native(&events_target) {
                    match serde_json::from_str::<BridgeEventEnvelope>(&json) {
                        Ok(event) => events.emit_event(event),
                        Err(error) => events.emit_event(BridgeEventEnvelope::new(
                            pi_bridge_types::BridgeEvent::FatalError {
                                error: BridgeError::new(
                                    BridgeErrorCode::InvalidPayload,
                                    "invalid event envelope from JavaScript",
                                )
                                .with_details(error.to_string()),
                            },
                        )),
                    }
                }
                ctx.env.get_undefined()
            })?;

            let responses_target = Arc::clone(&target);
            let emit_response = env.create_function_from_closure("emitResponse", move |ctx| {
                let json = js_string_arg(&ctx)?;
                if let Some(responses) = current_native(&responses_target) {
                    match serde_json::from_str::<pi_bridge_types::BridgeResponseEnvelope>(&json) {
                        Ok(response) => responses.complete_response(response),
                        Err(error) => responses.emit_event(BridgeEventEnvelope::new(
                            pi_bridge_types::BridgeEvent::FatalError {
                                error: BridgeError::new(
                                    BridgeErrorCode::InvalidPayload,
                                    "invalid response envelope from JavaScript",
                                )
                                .with_details(error.to_string()),
                            },
                        )),
                    }
                }
                ctx.env.get_undefined()
            })?;

            exports.set_named_property("emitEvent", emit_event)?;
            exports.set_named_property("emitResponse", emit_response)?;
            Ok(exports)
        })?;
        let _registered = NATIVE_MODULE_REGISTERED.set(());
        Ok(())
    }

    async fn load_bootstrap(&self, bootstrap_path: &std::path::Path) -> Result<()> {
        let path = bootstrap_path.display().to_string();
        let path_literal = serde_json::to_string(&path)?;
        let instance_id = BOOTSTRAP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let module_literal = serde_json::to_string(NATIVE_MODULE_NAME)?;
        let code = format!(
            r#"
            globalThis.__PI_GPUI_NATIVE = process._linkedBinding({module_literal});
            const {{ pathToFileURL }} = require('node:url');
            void import(`${{pathToFileURL({path_literal}).href}}?instance={instance_id}`).catch((error) => {{
              globalThis.__PI_GPUI_NATIVE.emitEvent(JSON.stringify({{
                version: 1,
                event: {{
                  type: 'fatalError',
                  payload: {{
                    error: {{
                      code: 'nodeRuntimeError',
                      message: error && error.stack ? String(error.stack) : String(error),
                      details: null,
                      retryable: false
                    }}
                  }}
                }}
              }}));
            }});
            "#,
        );
        self.eval_node(code).await
    }

    async fn dispatch_to_node(&self, envelope: &BridgeCommandEnvelope) -> Result<()> {
        let command_json = serde_json::to_string(envelope)?;
        let command_literal = serde_json::to_string(&command_json)?;
        let code = format!(
            r#"
            void Promise.resolve()
              .then(() => globalThis.__piBridge.dispatch(JSON.parse({command_literal})))
              .then(
                (response) => globalThis.__PI_GPUI_NATIVE.emitResponse(JSON.stringify(response)),
                (error) => globalThis.__PI_GPUI_NATIVE.emitResponse(JSON.stringify({{
                  version: 1,
                  requestId: JSON.parse({command_literal}).requestId,
                  response: {{
                    status: 'error',
                    error: {{
                      code: 'nodeRuntimeError',
                      message: error && error.stack ? String(error.stack) : String(error),
                      details: null,
                      retryable: false
                    }}
                  }}
                }}))
              );
            "#,
        );
        self.eval_node(code).await
    }

    async fn eval_node(&self, code: String) -> Result<()> {
        let node = Arc::clone(&self.node);
        tokio::task::spawn_blocking(move || node.lock().eval(&code))
            .await
            .map_err(|error| NodeHostError::Join(error.to_string()))??;
        Ok(())
    }
}
