#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ImageAttachment {
    pub media_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelection {
    pub provider: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAuthStatus {
    pub provider: String,
    pub display_name: String,
    pub configured: bool,
    pub source: Option<AuthSource>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageSearchResponse {
    pub query: String,
    pub limit: u32,
    pub total: u32,
    pub results: Vec<PackageSearchResult>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageSearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub publisher: Option<String>,
    pub monthly_downloads: Option<u32>,
    pub updated: Option<String>,
    pub resource_types: Vec<String>,
    pub install_command: String,
    pub npm_url: String,
    pub repository_url: Option<String>,
    pub homepage_url: Option<String>,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackage {
    pub source: String,
    pub display_name: String,
    pub scope: PackageScope,
    pub filtered: bool,
    pub installed_path: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum PackageScope {
    User,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum AuthSource {
    Stored,
    Runtime,
    Environment,
    Fallback,
    ModelsJsonKey,
    ModelsJsonCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    #[serde(rename = "xhigh")]
    #[ts(rename = "xhigh")]
    XHigh,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum SessionTarget {
    New,
    ContinueRecent,
    Open { path: String },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CoreStateSnapshot {
    pub initialized: bool,
    pub cwd: Option<String>,
    pub session_id: Option<String>,
    pub session_file: Option<String>,
    pub session_name: Option<String>,
    pub is_streaming: bool,
    pub is_compacting: bool,
    pub is_retrying: bool,
    pub is_bash_running: bool,
    pub model: Option<ModelDescriptor>,
    pub thinking_level: Option<ThinkingLevel>,
    pub active_tools: Vec<String>,
    pub queue: QueueSnapshot,
    pub messages: Vec<serde_json::Value>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct QueueSnapshot {
    pub steering: Vec<String>,
    pub follow_up: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ModelDescriptor {
    pub provider: String,
    pub id: String,
    pub name: String,
    pub reasoning: bool,
    pub context_window: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub active: bool,
    pub source: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
}
