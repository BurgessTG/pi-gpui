use crate::RequestId;

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
pub struct UiRequestId(pub String);

impl UiRequestId {
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7().to_string())
    }
}

impl Default for UiRequestId {
    fn default() -> Self {
        Self::new()
    }
}

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
pub struct ComponentHandleId(pub String);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum ExtensionUiRequest {
    Select(SelectRequest),
    Confirm(ConfirmRequest),
    Input(InputRequest),
    Editor(EditorRequest),
    CustomComponent(CustomComponentRequest),
    Autocomplete(AutocompleteRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SelectRequest {
    pub id: UiRequestId,
    pub title: String,
    pub options: Vec<String>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmRequest {
    pub id: UiRequestId,
    pub title: String,
    pub message: String,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InputRequest {
    pub id: UiRequestId,
    pub title: String,
    pub placeholder: Option<String>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct EditorRequest {
    pub id: UiRequestId,
    pub title: String,
    pub prefill: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CustomComponentRequest {
    pub id: UiRequestId,
    pub handle: ComponentHandleId,
    pub overlay: bool,
    pub overlay_options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteRequest {
    pub id: UiRequestId,
    pub text: String,
    pub cursor: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct UiResponseCommand {
    pub request_id: UiRequestId,
    pub response: ExtensionUiResponse,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum ExtensionUiResponse {
    Selected { value: Option<String> },
    Confirmed { value: bool },
    Text { value: Option<String> },
    Custom { value: serde_json::Value },
    Autocomplete { items: Vec<AutocompleteItem> },
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteItem {
    pub label: String,
    pub detail: Option<String>,
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInputCommand {
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ComponentInputCommand {
    pub handle: ComponentHandleId,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ComponentRenderCommand {
    pub handle: ComponentHandleId,
    pub width: u16,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum ExtensionUiUpdate {
    Notify(NotifyUpdate),
    Status(StatusUpdate),
    Working(WorkingUpdate),
    Widget(WidgetUpdate),
    Footer(ComponentSlotUpdate),
    Header(ComponentSlotUpdate),
    Title { title: String },
    EditorText { text: String },
    Theme { theme: serde_json::Value },
    ToolsExpanded { expanded: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NotifyUpdate {
    pub message: String,
    pub level: NotifyLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum NotifyLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StatusUpdate {
    pub key: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WorkingUpdate {
    pub message: Option<String>,
    pub visible: Option<bool>,
    pub indicator_frames: Option<Vec<String>>,
    pub hidden_thinking_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WidgetUpdate {
    pub key: String,
    pub placement: WidgetPlacement,
    pub content: Option<ComponentContent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum WidgetPlacement {
    AboveEditor,
    BelowEditor,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum ComponentContent {
    Lines { lines: Vec<String> },
    Handle { handle: ComponentHandleId },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ComponentSlotUpdate {
    pub content: Option<ComponentContent>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ComponentRenderRequest {
    pub request_id: RequestId,
    pub handle: ComponentHandleId,
    pub width: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ComponentRenderResult {
    pub handle: ComponentHandleId,
    pub lines: Vec<String>,
}
