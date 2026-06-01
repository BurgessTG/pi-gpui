use crate::extension_ui::{
    AutocompleteRequest, ComponentInputCommand, ComponentRenderCommand, TerminalInputCommand,
    UiResponseCommand,
};
use crate::state::{ImageAttachment, ModelSelection, SessionTarget, ThinkingLevel};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum BridgeCommand {
    Init(InitCommand),
    Shutdown,
    Reload,
    Prompt(PromptCommand),
    Steer(MessageCommand),
    FollowUp(MessageCommand),
    SendUserMessage(MessageCommand),
    SendCustomMessage(CustomMessageCommand),
    Abort,
    ClearQueue,
    GetState,
    GetMessages,
    GetSessionStats,
    GetAuthStatus(GetAuthStatusCommand),
    SetApiKey(SetApiKeyCommand),
    #[serde(rename = "oauthLogin")]
    OAuthLogin(OAuthLoginCommand),
    RemoveAuth(RemoveAuthCommand),
    SearchPackages(SearchPackagesCommand),
    ListPackages(PackageScopeCommand),
    InstallPackage(InstallPackageCommand),
    RemovePackage(RemovePackageCommand),
    NewSession(NewSessionCommand),
    SwitchSession(SwitchSessionCommand),
    Fork(ForkCommand),
    NavigateTree(NavigateTreeCommand),
    ImportJsonl(ImportJsonlCommand),
    ExportHtml(ExportCommand),
    ExportJsonl(ExportCommand),
    SetSessionName(SetSessionNameCommand),
    GetAvailableModels,
    SetModel(SetModelCommand),
    CycleModel(CycleModelCommand),
    SetThinkingLevel(SetThinkingLevelCommand),
    CycleThinkingLevel,
    GetTools,
    SetActiveTools(SetActiveToolsCommand),
    SetSteeringMode(SetQueueModeCommand),
    SetFollowUpMode(SetQueueModeCommand),
    SetAutoCompaction(SetEnabledCommand),
    SetAutoRetry(SetEnabledCommand),
    Compact(CompactCommand),
    AbortCompaction,
    AbortRetry,
    ExecuteBash(ExecuteBashCommand),
    AbortBash,
    UiResponse(UiResponseCommand),
    Autocomplete(AutocompleteRequest),
    TerminalInput(TerminalInputCommand),
    ComponentInput(ComponentInputCommand),
    RenderComponent(ComponentRenderCommand),
    SetEditorText(EditorTextCommand),
    GetEditorText,
    PasteToEditor(EditorTextCommand),
    SetTheme(SetThemeCommand),
    GetTheme(GetThemeCommand),
    GetAllThemes,
    SetToolsExpanded(SetEnabledCommand),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InitCommand {
    pub cwd: String,
    pub agent_dir: Option<String>,
    pub session: Option<SessionTarget>,
    pub model: Option<ModelSelection>,
    pub tools: Option<Vec<String>>,
    pub enable_extensions: bool,
    pub test_mode: Option<TestModeConfig>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TestModeConfig {
    pub faux_response: String,
    pub tokens_per_second: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct GetAuthStatusCommand {
    pub provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SetApiKeyCommand {
    pub provider: String,
    pub api_key: String,
    pub persist: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct OAuthLoginCommand {
    pub provider: String,
    pub method: Option<OAuthLoginMethod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum OAuthLoginMethod {
    Browser,
    DeviceCode,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAuthCommand {
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SearchPackagesCommand {
    pub query: String,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageScopeCommand {
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InstallPackageCommand {
    pub source: String,
    pub project: bool,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RemovePackageCommand {
    pub source: String,
    pub project: bool,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PromptCommand {
    pub text: String,
    pub images: Vec<ImageAttachment>,
    pub streaming_behavior: Option<StreamingBehavior>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MessageCommand {
    pub text: String,
    pub images: Vec<ImageAttachment>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CustomMessageCommand {
    pub custom_type: String,
    pub content: serde_json::Value,
    pub display: bool,
    pub details: Option<serde_json::Value>,
    pub trigger_turn: bool,
    pub deliver_as: Option<DeliveryMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StreamingBehavior {
    Steer,
    FollowUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum DeliveryMode {
    Steer,
    FollowUp,
    NextTurn,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionCommand {
    pub parent_session: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SwitchSessionCommand {
    pub session_path: String,
    pub cwd_override: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ForkCommand {
    pub entry_id: String,
    pub position: ForkPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ForkPosition {
    Before,
    At,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NavigateTreeCommand {
    pub target_id: String,
    pub summarize: bool,
    pub custom_instructions: Option<String>,
    pub replace_instructions: bool,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ImportJsonlCommand {
    pub input_path: String,
    pub cwd_override: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ExportCommand {
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SetSessionNameCommand {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SetModelCommand {
    pub model: ModelSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CycleModelCommand {
    pub direction: CycleDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum CycleDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SetThinkingLevelCommand {
    pub level: ThinkingLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SetActiveToolsCommand {
    pub tool_names: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SetQueueModeCommand {
    pub mode: QueueMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum QueueMode {
    All,
    OneAtATime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SetEnabledCommand {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CompactCommand {
    pub custom_instructions: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteBashCommand {
    pub command: String,
    pub exclude_from_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct EditorTextCommand {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SetThemeCommand {
    pub theme: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct GetThemeCommand {
    pub name: String,
}
