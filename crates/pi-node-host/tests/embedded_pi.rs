use std::path::Path;
use std::time::Duration;

use pi_bridge_types::{
    AutocompleteRequest, BridgeCommand, BridgeErrorCode, BridgeEvent, BridgeEventEnvelope,
    BridgeResponse, CompactCommand, ComponentContent, ComponentInputCommand,
    ComponentRenderCommand, CycleDirection, CycleModelCommand, EditorTextCommand,
    ExecuteBashCommand, ExportCommand, ExtensionUiRequest, ExtensionUiResponse, ForkPosition,
    GetAuthStatusCommand, GetThemeCommand, ImportJsonlCommand, InitCommand, MessageCommand,
    ModelSelection, NavigateTreeCommand, NewSessionCommand, PromptCommand, QueueMode,
    RemoveAuthCommand, SessionTarget, SetApiKeyCommand, SetEnabledCommand, SetModelCommand,
    SetQueueModeCommand, SetSessionNameCommand, SetThemeCommand, SetThinkingLevelCommand,
    SwitchSessionCommand, TestModeConfig, ThinkingLevel, UiRequestId, UiResponseCommand,
};
use serial_test::serial;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

fn maybe_config() -> Option<pi_node_host::NodeHostConfig> {
    let bootstrap = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)?
        .join("node/dist/bootstrap.js");
    if !bootstrap.is_file() {
        return None;
    }
    pi_node_host::NodeHostConfig::from_env(bootstrap)
        .ok()
        .map(|mut config| {
            config.request_timeout = Duration::from_secs(60);
            config
        })
}

fn test_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::other(message.into()))
}

fn init_command(temp: &tempfile::TempDir, enable_extensions: bool) -> InitCommand {
    InitCommand {
        cwd: temp.path().display().to_string(),
        agent_dir: Some(temp.path().join("agent").display().to_string()),
        session: Some(SessionTarget::New),
        model: None,
        tools: None,
        enable_extensions,
        test_mode: Some(TestModeConfig {
            faux_response: "hello from embedded pi".to_owned(),
            tokens_per_second: Some(0),
        }),
    }
}

async fn init_host(
    host: &pi_node_host::NodeHost,
    temp: &tempfile::TempDir,
    enable_extensions: bool,
) -> Result<pi_bridge_types::CoreStateSnapshot, Box<dyn std::error::Error>> {
    let BridgeResponse::State { state } = host
        .request(BridgeCommand::Init(init_command(temp, enable_extensions)))
        .await?
    else {
        return Err(test_error("expected init state response"));
    };
    Ok(state)
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

fn first_json_id(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(object) => object
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| object.values().find_map(first_json_id)),
        serde_json::Value::Array(values) => values.iter().find_map(first_json_id),
        _other => None,
    }
}

#[tokio::test]
#[serial]
async fn embedded_node_runs_pi_sdk_with_faux_provider() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let Some(config) = maybe_config() else {
        return Ok(());
    };
    let host = pi_node_host::NodeHost::start(config).await?;
    let ready = host.wait_until_ready().await?;
    assert_eq!(ready.protocol_version, pi_bridge_types::PROTOCOL_VERSION);

    let state = init_host(&host, &temp, false).await?;
    assert!(state.initialized);
    assert!(matches!(
        host.request(BridgeCommand::Prompt(PromptCommand {
            text: "Say hello".to_owned(),
            images: Vec::new(),
            streaming_behavior: None,
        }))
        .await?,
        BridgeResponse::Ack
    ));
    let BridgeResponse::State { state } = host.request(BridgeCommand::GetState).await? else {
        return Err(test_error("expected state response"));
    };
    let serialized = serde_json::to_string(&state.messages)?;
    assert!(serialized.contains("hello from embedded pi"));
    host.shutdown().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn embedded_backend_handles_core_commands() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let Some(config) = maybe_config() else {
        return Ok(());
    };
    let host = pi_node_host::NodeHost::start(config).await?;
    host.wait_until_ready().await?;
    let state = init_host(&host, &temp, false).await?;
    let model = state
        .model
        .ok_or_else(|| test_error("missing faux model"))?;
    let BridgeResponse::AuthStatus { statuses } = host
        .request(BridgeCommand::GetAuthStatus(GetAuthStatusCommand {
            provider: Some(model.provider.clone()),
        }))
        .await?
    else {
        return Err(test_error("expected auth status response"));
    };
    assert!(statuses.iter().any(|status| status.configured));

    assert!(matches!(
        host.request(BridgeCommand::SetApiKey(SetApiKeyCommand {
            provider: "openai".to_owned(),
            api_key: "test-key".to_owned(),
            persist: true,
        }))
        .await?,
        BridgeResponse::Ack
    ));
    let BridgeResponse::AuthStatus { statuses } = host
        .request(BridgeCommand::GetAuthStatus(GetAuthStatusCommand {
            provider: Some("openai".to_owned()),
        }))
        .await?
    else {
        return Err(test_error("expected auth status response"));
    };
    assert!(statuses.iter().any(|status| status.configured));
    assert!(matches!(
        host.request(BridgeCommand::RemoveAuth(RemoveAuthCommand {
            provider: "openai".to_owned(),
        }))
        .await?,
        BridgeResponse::Ack
    ));

    assert!(matches!(
        host.request(BridgeCommand::SetModel(SetModelCommand {
            model: ModelSelection {
                provider: model.provider.clone(),
                model_id: model.id.clone(),
            },
        }))
        .await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::SetThinkingLevel(SetThinkingLevelCommand {
            level: ThinkingLevel::Low,
        },))
            .await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::CycleThinkingLevel).await?,
        BridgeResponse::Json { .. }
    ));
    assert!(matches!(
        host.request(BridgeCommand::CycleModel(CycleModelCommand {
            direction: CycleDirection::Forward,
        }))
        .await?,
        BridgeResponse::Json { .. }
    ));
    let BridgeResponse::Models { models } = host.request(BridgeCommand::GetAvailableModels).await?
    else {
        return Err(test_error("expected models response"));
    };
    assert!(models.iter().any(|item| item.provider == model.provider));

    let BridgeResponse::Tools { tools } = host.request(BridgeCommand::GetTools).await? else {
        return Err(test_error("expected tools response"));
    };
    let active_tools = tools.iter().take(2).map(|tool| tool.name.clone()).collect();
    assert!(matches!(
        host.request(BridgeCommand::SetActiveTools(
            pi_bridge_types::SetActiveToolsCommand {
                tool_names: active_tools,
            },
        ))
        .await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::SetSteeringMode(SetQueueModeCommand {
            mode: QueueMode::OneAtATime,
        }))
        .await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::SetFollowUpMode(SetQueueModeCommand {
            mode: QueueMode::All,
        }))
        .await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::SetAutoCompaction(SetEnabledCommand {
            enabled: false,
        }))
        .await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::SetAutoRetry(SetEnabledCommand {
            enabled: false,
        }))
        .await?,
        BridgeResponse::Ack
    ));

    assert!(matches!(
        host.request(BridgeCommand::SetSessionName(SetSessionNameCommand {
            name: "backend-test".to_owned(),
        }))
        .await?,
        BridgeResponse::Ack
    ));
    let BridgeResponse::State { state } = host.request(BridgeCommand::GetState).await? else {
        return Err(test_error("expected state response"));
    };
    assert_eq!(state.session_name.as_deref(), Some("backend-test"));

    assert!(matches!(
        host.request(BridgeCommand::SetEditorText(EditorTextCommand {
            text: "hello".to_owned(),
        }))
        .await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::PasteToEditor(EditorTextCommand {
            text: " world".to_owned(),
        }))
        .await?,
        BridgeResponse::Ack
    ));
    let BridgeResponse::Text { text } = host.request(BridgeCommand::GetEditorText).await? else {
        return Err(test_error("expected editor text response"));
    };
    assert_eq!(text, "hello world");

    assert!(matches!(
        host.request(BridgeCommand::SetTheme(SetThemeCommand {
            theme: serde_json::json!({ "name": "test-theme" }),
        }))
        .await?,
        BridgeResponse::Json { .. }
    ));
    assert!(matches!(
        host.request(BridgeCommand::GetTheme(GetThemeCommand {
            name: "native-gpui-placeholder".to_owned(),
        }))
        .await?,
        BridgeResponse::Json { .. }
    ));
    assert!(matches!(
        host.request(BridgeCommand::SetToolsExpanded(SetEnabledCommand {
            enabled: false,
        }))
        .await?,
        BridgeResponse::Ack
    ));

    assert!(matches!(
        host.request(BridgeCommand::SendUserMessage(MessageCommand {
            text: "queued user note".to_owned(),
            images: Vec::new(),
        }))
        .await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::ClearQueue).await?,
        BridgeResponse::Json { .. }
    ));

    let BridgeResponse::Json { value } = host
        .request(BridgeCommand::ExecuteBash(ExecuteBashCommand {
            command: "printf pi-gpui-bash".to_owned(),
            exclude_from_context: true,
        }))
        .await?
    else {
        return Err(test_error("expected bash json response"));
    };
    assert!(serde_json::to_string(&value)?.contains("pi-gpui-bash"));
    assert!(matches!(
        host.request(BridgeCommand::AbortBash).await?,
        BridgeResponse::Ack
    ));

    assert!(matches!(
        host.request(BridgeCommand::Prompt(PromptCommand {
            text: "Create exportable content".to_owned(),
            images: Vec::new(),
            streaming_behavior: None,
        }))
        .await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::Compact(CompactCommand {
            custom_instructions: Some("keep this very short".to_owned()),
        }))
        .await?,
        BridgeResponse::Json { .. }
    ));
    assert!(matches!(
        host.request(BridgeCommand::AbortCompaction).await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::AbortRetry).await?,
        BridgeResponse::Ack
    ));
    assert!(matches!(
        host.request(BridgeCommand::Abort).await?,
        BridgeResponse::Ack
    ));

    let jsonl = temp.path().join("session.jsonl");
    let html = temp.path().join("session.html");
    let BridgeResponse::Path { path } = host
        .request(BridgeCommand::ExportJsonl(ExportCommand {
            output_path: Some(jsonl.display().to_string()),
        }))
        .await?
    else {
        return Err(test_error("expected jsonl path response"));
    };
    assert!(Path::new(&path).is_file());
    let BridgeResponse::Path { path } = host
        .request(BridgeCommand::ExportHtml(ExportCommand {
            output_path: Some(html.display().to_string()),
        }))
        .await?
    else {
        return Err(test_error("expected html path response"));
    };
    assert!(Path::new(&path).is_file());
    assert!(matches!(
        host.request(BridgeCommand::GetMessages).await?,
        BridgeResponse::Messages { .. }
    ));
    assert!(matches!(
        host.request(BridgeCommand::GetSessionStats).await?,
        BridgeResponse::SessionStats { .. }
    ));

    host.shutdown().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn embedded_backend_handles_session_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let Some(config) = maybe_config() else {
        return Ok(());
    };
    let host = pi_node_host::NodeHost::start(config).await?;
    host.wait_until_ready().await?;
    init_host(&host, &temp, false).await?;
    host.request(BridgeCommand::Prompt(PromptCommand {
        text: "Create a session tree".to_owned(),
        images: Vec::new(),
        streaming_behavior: None,
    }))
    .await?;

    let BridgeResponse::State { state } = host.request(BridgeCommand::GetState).await? else {
        return Err(test_error("expected state response"));
    };
    let session_file = state
        .session_file
        .ok_or_else(|| test_error("missing session file"))?;
    let entry_id = first_json_id(&serde_json::Value::Array(state.messages));

    let jsonl = temp.path().join("roundtrip.jsonl");
    host.request(BridgeCommand::ExportJsonl(ExportCommand {
        output_path: Some(jsonl.display().to_string()),
    }))
    .await?;
    assert!(jsonl.is_file());

    assert!(matches!(
        host.request(BridgeCommand::NewSession(NewSessionCommand {
            parent_session: None,
        }))
        .await?,
        BridgeResponse::Cancelled { .. }
    ));
    assert!(matches!(
        host.request(BridgeCommand::SwitchSession(SwitchSessionCommand {
            session_path: session_file,
            cwd_override: None,
        }))
        .await?,
        BridgeResponse::Cancelled { .. }
    ));
    assert!(matches!(
        host.request(BridgeCommand::ImportJsonl(ImportJsonlCommand {
            input_path: jsonl.display().to_string(),
            cwd_override: None,
        }))
        .await?,
        BridgeResponse::Cancelled { .. }
    ));

    if let Some(entry_id) = entry_id {
        assert!(matches!(
            host.request(BridgeCommand::NavigateTree(NavigateTreeCommand {
                target_id: entry_id.clone(),
                summarize: false,
                custom_instructions: None,
                replace_instructions: false,
                label: Some("test navigation".to_owned()),
            }))
            .await?,
            BridgeResponse::Json { .. }
        ));
        assert!(matches!(
            host.request(BridgeCommand::Fork(pi_bridge_types::ForkCommand {
                entry_id,
                position: ForkPosition::At,
            }))
            .await?,
            BridgeResponse::Json { .. }
        ));
    }

    host.shutdown().await?;
    host.shutdown().await?;
    let BridgeResponse::State { state } = host.request(BridgeCommand::GetState).await? else {
        return Err(test_error("expected empty state response"));
    };
    assert!(!state.initialized);
    let error = host.request(BridgeCommand::GetMessages).await.err();
    let Some(pi_node_host::NodeHostError::Bridge(error)) = error else {
        return Err(test_error("expected bridge error after shutdown"));
    };
    assert_eq!(error.code, BridgeErrorCode::NotInitialized);
    Ok(())
}

fn write_ui_probe(agent_dir: &Path) -> std::io::Result<()> {
    let extensions = agent_dir.join("extensions");
    std::fs::create_dir_all(&extensions)?;
    std::fs::write(
        extensions.join("ui-probe.ts"),
        r#"
class ProbeComponent {
  constructor(prefix, done) {
    this.prefix = prefix;
    this.done = done;
  }
  render(width) {
    return [`${this.prefix}:${width}`];
  }
  handleInput(data) {
    if (this.done) this.done(`input:${data}`);
  }
}

export default function(pi) {
  pi.on("session_start", (_event, ctx) => {
    ctx.ui.notify("ui-probe-start", "info");
    ctx.ui.setStatus("ui-probe", "ready");
    ctx.ui.setWorkingMessage("working-message");
    ctx.ui.setWorkingVisible(true);
    ctx.ui.setWorkingIndicator({ frames: ["one", "two"] });
    ctx.ui.setHiddenThinkingLabel("hidden-label");
    ctx.ui.setWidget("widget-lines", ["widget-line"]);
    ctx.ui.setWidget("widget-handle", () => new ProbeComponent("widget", null));
    ctx.ui.setHeader(() => new ProbeComponent("header", null));
    ctx.ui.setFooter(() => new ProbeComponent("footer", null));
    ctx.ui.setTitle("ui-probe-title");
    ctx.ui.setEditorText("seed");
    ctx.ui.pasteToEditor("-paste");
    ctx.ui.setToolsExpanded(false);
    ctx.ui.onTerminalInput((data) => ({ consume: true, data: `handled:${data}` }));
    ctx.ui.addAutocompleteProvider(() => ({
      async getSuggestions() {
        return { items: [{ label: "probe", value: "probe-value", description: "Probe item" }] };
      },
      applyCompletion(lines, cursorLine, cursorCol) {
        return { lines, cursorLine, cursorCol };
      }
    }));
    void (async () => {
      const selected = await ctx.ui.select("Probe select", ["a", "b"]);
      ctx.ui.notify(`selected:${selected}`, "info");
      const confirmed = await ctx.ui.confirm("Probe confirm", "Continue?");
      ctx.ui.notify(`confirmed:${confirmed}`, "info");
      const input = await ctx.ui.input("Probe input", "type here");
      ctx.ui.notify(`input:${input}`, "info");
      const edited = await ctx.ui.editor("Probe editor", "prefill");
      ctx.ui.notify(`editor:${edited}`, "info");
      const custom = await ctx.ui.custom((_tui, _theme, _kb, done) => new ProbeComponent("custom", done));
      ctx.ui.notify(`custom:${custom}`, "info");
    })();
  });
}
"#,
    )
}

#[tokio::test]
#[serial]
async fn embedded_backend_roundtrips_extension_ui() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let agent_dir = temp.path().join("agent");
    write_ui_probe(&agent_dir)?;
    let Some(config) = maybe_config() else {
        return Ok(());
    };
    let host = pi_node_host::NodeHost::start(config).await?;
    let mut events = host.subscribe();
    host.wait_until_ready().await?;
    init_host(&host, &temp, true).await?;

    let handle = next_matching(&mut events, |event| match event.event {
        BridgeEvent::ExtensionUiUpdate { update } => match update {
            pi_bridge_types::ExtensionUiUpdate::Widget(widget) if widget.key == "widget-handle" => {
                match widget.content {
                    Some(ComponentContent::Handle { handle }) => Some(handle),
                    _other => None,
                }
            }
            _other => None,
        },
        _other => None,
    })
    .await?;
    let BridgeResponse::ComponentRender { render } = host
        .request(BridgeCommand::RenderComponent(ComponentRenderCommand {
            handle: handle.clone(),
            width: 7,
        }))
        .await?
    else {
        return Err(test_error("expected component render response"));
    };
    assert_eq!(render.lines, vec!["widget:7"]);

    let BridgeResponse::Text { text } = host.request(BridgeCommand::GetEditorText).await? else {
        return Err(test_error("expected editor text response"));
    };
    assert_eq!(text, "seed-paste");
    let BridgeResponse::Json { value } = host
        .request(BridgeCommand::TerminalInput(
            pi_bridge_types::TerminalInputCommand {
                data: "abc".to_owned(),
            },
        ))
        .await?
    else {
        return Err(test_error("expected terminal input json response"));
    };
    assert_eq!(value["consume"], true);
    assert_eq!(value["data"], "handled:abc");
    let BridgeResponse::Autocomplete { items } = host
        .request(BridgeCommand::Autocomplete(AutocompleteRequest {
            id: UiRequestId("autocomplete-test".to_owned()),
            text: "pro".to_owned(),
            cursor: 3,
        }))
        .await?
    else {
        return Err(test_error("expected autocomplete response"));
    };
    assert_eq!(items.first().map(|item| item.label.as_str()), Some("probe"));

    let select_id = next_matching(&mut events, |event| match event.event {
        BridgeEvent::ExtensionUiRequest {
            request: ExtensionUiRequest::Select(request),
        } if request.title == "Probe select" => Some(request.id),
        _other => None,
    })
    .await?;
    host.request(BridgeCommand::UiResponse(UiResponseCommand {
        request_id: select_id,
        response: ExtensionUiResponse::Selected {
            value: Some("b".to_owned()),
        },
    }))
    .await?;

    let confirm_id = next_matching(&mut events, |event| match event.event {
        BridgeEvent::ExtensionUiRequest {
            request: ExtensionUiRequest::Confirm(request),
        } if request.title == "Probe confirm" => Some(request.id),
        _other => None,
    })
    .await?;
    host.request(BridgeCommand::UiResponse(UiResponseCommand {
        request_id: confirm_id,
        response: ExtensionUiResponse::Confirmed { value: true },
    }))
    .await?;

    let input_id = next_matching(&mut events, |event| match event.event {
        BridgeEvent::ExtensionUiRequest {
            request: ExtensionUiRequest::Input(request),
        } if request.title == "Probe input" => Some(request.id),
        _other => None,
    })
    .await?;
    host.request(BridgeCommand::UiResponse(UiResponseCommand {
        request_id: input_id,
        response: ExtensionUiResponse::Text {
            value: Some("typed".to_owned()),
        },
    }))
    .await?;

    let editor_id = next_matching(&mut events, |event| match event.event {
        BridgeEvent::ExtensionUiRequest {
            request: ExtensionUiRequest::Editor(request),
        } if request.title == "Probe editor" => Some(request.id),
        _other => None,
    })
    .await?;
    host.request(BridgeCommand::UiResponse(UiResponseCommand {
        request_id: editor_id,
        response: ExtensionUiResponse::Text {
            value: Some("edited".to_owned()),
        },
    }))
    .await?;

    let (custom_id, custom_handle) = next_matching(&mut events, |event| match event.event {
        BridgeEvent::ExtensionUiRequest {
            request: ExtensionUiRequest::CustomComponent(request),
        } => Some((request.id, request.handle)),
        _other => None,
    })
    .await?;
    let BridgeResponse::ComponentRender { render } = host
        .request(BridgeCommand::RenderComponent(ComponentRenderCommand {
            handle: custom_handle.clone(),
            width: 9,
        }))
        .await?
    else {
        return Err(test_error("expected custom render response"));
    };
    assert_eq!(render.lines, vec!["custom:9"]);
    assert!(matches!(
        host.request(BridgeCommand::ComponentInput(ComponentInputCommand {
            handle: custom_handle,
            data: "payload".to_owned(),
        }))
        .await?,
        BridgeResponse::Ack
    ));
    host.request(BridgeCommand::UiResponse(UiResponseCommand {
        request_id: custom_id,
        response: ExtensionUiResponse::Custom {
            value: serde_json::json!("frontend"),
        },
    }))
    .await?;

    next_matching(&mut events, |event| match event.event {
        BridgeEvent::ExtensionUiUpdate {
            update: pi_bridge_types::ExtensionUiUpdate::Notify(update),
        } if update.message == "custom:input:payload" => Some(()),
        _other => None,
    })
    .await?;

    host.shutdown().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn embedded_backend_can_use_real_provider_when_configured()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let Ok(provider) = std::env::var("PI_GPUI_REAL_PROVIDER") else {
        return Ok(());
    };
    let Ok(model_id) = std::env::var("PI_GPUI_REAL_MODEL") else {
        return Ok(());
    };
    let Some(config) = maybe_config() else {
        return Ok(());
    };
    let host = pi_node_host::NodeHost::start(config).await?;
    host.wait_until_ready().await?;
    let mut init = init_command(&temp, false);
    init.test_mode = None;
    init.model = Some(ModelSelection {
        provider: provider.clone(),
        model_id: model_id.clone(),
    });
    let BridgeResponse::State { state } = host.request(BridgeCommand::Init(init)).await? else {
        return Err(test_error("expected init state response"));
    };
    assert!(state.initialized);

    if let Ok(api_key) = std::env::var("PI_GPUI_REAL_API_KEY") {
        assert!(matches!(
            host.request(BridgeCommand::SetApiKey(SetApiKeyCommand {
                provider: provider.clone(),
                api_key,
                persist: false,
            }))
            .await?,
            BridgeResponse::Ack
        ));
    }
    let BridgeResponse::AuthStatus { statuses } = host
        .request(BridgeCommand::GetAuthStatus(GetAuthStatusCommand {
            provider: Some(provider),
        }))
        .await?
    else {
        return Err(test_error("expected auth status response"));
    };
    if !statuses.iter().any(|status| status.configured) {
        host.shutdown().await?;
        return Err(test_error(
            "PI_GPUI_REAL_PROVIDER/PI_GPUI_REAL_MODEL were set, but provider auth was not configured",
        ));
    }

    assert!(matches!(
        host.request(BridgeCommand::SetModel(SetModelCommand {
            model: ModelSelection {
                provider: statuses[0].provider.clone(),
                model_id
            },
        }))
        .await?,
        BridgeResponse::Ack
    ));
    let expected = std::env::var("PI_GPUI_REAL_EXPECT")
        .unwrap_or_else(|_missing| "pi-gpui-real-provider-ok".to_owned());
    host.request(BridgeCommand::Prompt(PromptCommand {
        text: std::env::var("PI_GPUI_REAL_PROMPT")
            .unwrap_or_else(|_missing| format!("Reply with exactly: {expected}")),
        images: Vec::new(),
        streaming_behavior: None,
    }))
    .await?;
    let BridgeResponse::State { state } = host.request(BridgeCommand::GetState).await? else {
        return Err(test_error("expected state response"));
    };
    assert!(serde_json::to_string(&state.messages)?.contains(&expected));
    host.shutdown().await?;
    Ok(())
}
