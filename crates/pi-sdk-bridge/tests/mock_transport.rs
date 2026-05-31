use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use pi_bridge_types::{
    AuthSource, BridgeCommand, BridgeResponse, CoreStateSnapshot, InitCommand, ProviderAuthStatus,
    QueueSnapshot,
};
use pi_sdk_bridge::{BridgeClient, BridgeClientError, BridgeTransport};

#[derive(Clone)]
struct MockTransport {
    inner: Arc<MockTransportInner>,
}

struct MockTransportInner {
    responses: Mutex<VecDeque<BridgeResponse>>,
    commands: Mutex<Vec<BridgeCommand>>,
}

impl MockTransport {
    fn new(responses: Vec<BridgeResponse>) -> Self {
        Self {
            inner: Arc::new(MockTransportInner {
                responses: Mutex::new(responses.into()),
                commands: Mutex::new(Vec::new()),
            }),
        }
    }

    fn command_count(&self) -> usize {
        self.inner.commands.lock().len()
    }
}

#[async_trait]
impl BridgeTransport for MockTransport {
    async fn request(&self, command: BridgeCommand) -> Result<BridgeResponse, BridgeClientError> {
        self.inner.commands.lock().push(command);
        self.inner
            .responses
            .lock()
            .pop_front()
            .ok_or_else(|| BridgeClientError::Transport("no response".to_owned()))
    }
}

fn empty_state() -> CoreStateSnapshot {
    CoreStateSnapshot {
        initialized: true,
        cwd: Some("/tmp".to_owned()),
        session_id: Some("s".to_owned()),
        session_file: None,
        session_name: None,
        is_streaming: false,
        is_compacting: false,
        is_retrying: false,
        is_bash_running: false,
        model: None,
        thinking_level: None,
        active_tools: Vec::new(),
        queue: QueueSnapshot {
            steering: Vec::new(),
            follow_up: Vec::new(),
        },
        messages: Vec::new(),
        diagnostics: Vec::new(),
    }
}

#[tokio::test]
async fn client_maps_auth_commands() -> Result<(), Box<dyn std::error::Error>> {
    let status = ProviderAuthStatus {
        provider: "openai".to_owned(),
        display_name: "OpenAI".to_owned(),
        configured: true,
        source: Some(AuthSource::Runtime),
        label: Some("runtime API key".to_owned()),
    };
    let transport = MockTransport::new(vec![
        BridgeResponse::AuthStatus {
            statuses: vec![status.clone()],
        },
        BridgeResponse::Ack,
        BridgeResponse::Ack,
        BridgeResponse::Ack,
    ]);
    let client = BridgeClient::new(transport.clone());
    assert_eq!(
        client.auth_status(Some("openai".to_owned())).await?,
        vec![status]
    );
    client.set_runtime_api_key("openai", "test-key").await?;
    client.set_persisted_api_key("openai", "test-key").await?;
    client.remove_auth("openai").await?;
    assert_eq!(transport.command_count(), 4);
    Ok(())
}

#[tokio::test]
async fn client_maps_typed_init_and_prompt() -> Result<(), Box<dyn std::error::Error>> {
    let state = empty_state();
    let transport = MockTransport::new(vec![
        BridgeResponse::State {
            state: state.clone(),
        },
        BridgeResponse::Ack,
    ]);
    let client = BridgeClient::new(transport.clone());
    let init = InitCommand {
        cwd: "/tmp".to_owned(),
        agent_dir: None,
        session: None,
        model: None,
        tools: None,
        enable_extensions: false,
        test_mode: None,
    };
    assert_eq!(client.init(init).await?, state);
    client.prompt("hello").await?;
    assert_eq!(transport.command_count(), 2);
    Ok(())
}
