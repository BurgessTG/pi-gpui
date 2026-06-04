use std::time::Duration;

use pi_bridge_types::{
    BridgeCommand, BridgeEvent, BridgeEventEnvelope, BridgeResponse, InitCommand, PromptCommand,
    SessionStateCommand, SessionTarget, TestModeConfig,
};
use serial_test::serial;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

fn maybe_process_config() -> Option<pi_node_host::NodeProcessHostConfig> {
    let process_host = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)?
        .join("node/dist/process_host.js");
    if !process_host.is_file() {
        return None;
    }
    let mut config = pi_node_host::NodeProcessHostConfig::new("node", process_host);
    config.request_timeout = Duration::from_secs(60);
    Some(config)
}

fn test_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::other(message.into()))
}

fn init_command(temp: &tempfile::TempDir) -> InitCommand {
    InitCommand {
        cwd: temp.path().display().to_string(),
        agent_dir: Some(temp.path().join("agent").display().to_string()),
        session: Some(SessionTarget::New),
        model: None,
        tools: None,
        enable_extensions: false,
        test_mode: Some(TestModeConfig {
            faux_response: "hello from session worker".to_owned(),
            tokens_per_second: Some(0),
        }),
    }
}

async fn next_matching<T, F>(
    events: &mut BroadcastStream<BridgeEventEnvelope>,
    mut filter: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnMut(BridgeEventEnvelope) -> Option<T>,
{
    tokio::time::timeout(Duration::from_secs(15), async move {
        loop {
            let Some(item) = events.next().await else {
                return Err(test_error("event stream ended"));
            };
            let event = item.map_err(|error| test_error(format!("event stream error: {error}")))?;
            if let Some(value) = filter(event) {
                return Ok(value);
            }
        }
    })
    .await
    .map_err(|_elapsed| test_error("timed out waiting for event"))?
}

#[tokio::test]
#[serial]
async fn worker_pool_routes_targeted_session_prompt_to_session_worker()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let Some(process_config) = maybe_process_config() else {
        return Ok(());
    };
    let mut pool_config = pi_node_host::NodeWorkerPoolConfig::new(process_config);
    pool_config.max_session_workers = 2;
    let pool = pi_node_host::NodeWorkerPool::start(pool_config).await?;
    let mut events = pool.subscribe();

    let BridgeResponse::State { state } = pool
        .request(BridgeCommand::Init(init_command(&temp)))
        .await?
    else {
        return Err(test_error("expected init state"));
    };
    let session_file = state
        .session_file
        .ok_or_else(|| test_error("missing session file"))?;

    assert!(matches!(
        pool.request(BridgeCommand::Prompt(PromptCommand {
            session_path: Some(session_file.clone()),
            text: "Say hello".to_owned(),
            images: Vec::new(),
            streaming_behavior: None,
        }))
        .await?,
        BridgeResponse::Ack
    ));

    next_matching(&mut events, |event| match event.event {
        BridgeEvent::SessionRunFinished {
            session_file: Some(path),
            ..
        } if path == session_file => Some(Ok(())),
        BridgeEvent::SessionRunError { message, .. } => Some(Err(test_error(message))),
        _other => None,
    })
    .await??;

    let BridgeResponse::State { state } = pool
        .request(BridgeCommand::GetSessionState(SessionStateCommand {
            session_path: session_file,
        }))
        .await?
    else {
        return Err(test_error("expected session state"));
    };
    assert!(serde_json::to_string(&state.messages)?.contains("hello from session worker"));

    pool.shutdown().await?;
    Ok(())
}
