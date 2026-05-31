use std::collections::BTreeMap;

use pi_bridge_types::{
    BridgeError, ComponentHandleId, CoreStateSnapshot, ExtensionUiRequest, ExtensionUiUpdate,
    QueueSnapshot,
};

#[derive(Debug, Clone, PartialEq)]
pub struct BackendState {
    pub ready: bool,
    pub fatal_error: Option<BridgeError>,
    pub snapshot: CoreStateSnapshot,
    pub transcript: Vec<TranscriptItem>,
    pub pending_ui: BTreeMap<String, ExtensionUiRequest>,
    pub ui_updates: Vec<ExtensionUiUpdate>,
    pub component_lines: BTreeMap<ComponentHandleId, Vec<String>>,
    pub bash_chunks: Vec<String>,
    pub logs: Vec<String>,
}

impl BackendState {
    pub fn new() -> Self {
        Self {
            ready: false,
            fatal_error: None,
            snapshot: empty_snapshot(),
            transcript: Vec::new(),
            pending_ui: BTreeMap::new(),
            ui_updates: Vec::new(),
            component_lines: BTreeMap::new(),
            bash_chunks: Vec::new(),
            logs: Vec::new(),
        }
    }
}

impl Default for BackendState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TranscriptItem {
    SessionEvent(serde_json::Value),
    TextDelta(String),
    ToolUpdate(serde_json::Value),
}

fn empty_snapshot() -> CoreStateSnapshot {
    CoreStateSnapshot {
        initialized: false,
        cwd: None,
        session_id: None,
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
