use std::collections::BTreeMap;

use serde_json::Value;

const SUMMARY_LIMIT: usize = 900;
const ARGUMENT_LIMIT: usize = 360;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatTranscript {
    entries: Vec<ChatEntry>,
    active_assistant: Option<usize>,
    tool_indices: BTreeMap<String, usize>,
    streaming: bool,
    revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatEntry {
    User(String),
    Assistant {
        text: String,
        status: AssistantStatus,
    },
    Tool(ChatToolRun),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssistantStatus {
    Streaming,
    Complete,
    Error(String),
    Aborted(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatToolRun {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub output: Option<String>,
    pub status: ToolStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolStatus {
    Pending,
    Running,
    Complete,
    Error,
}

impl ChatTranscript {
    pub fn entries(&self) -> &[ChatEntry] {
        &self.entries
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn push_user_message(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text.trim().is_empty() {
            return;
        }
        if self.last_user_message_is(&text) {
            return;
        }
        self.entries.push(ChatEntry::User(text));
        self.active_assistant = None;
        self.bump_revision();
    }

    pub fn observe_session_event(&mut self, event: &Value) {
        let Some(event_type) = event.get("type").and_then(Value::as_str) else {
            return;
        };

        match event_type {
            "agent_start" => {
                if !self.streaming || self.active_assistant.is_some() {
                    self.bump_revision();
                }
                self.streaming = true;
                self.active_assistant = None;
            }
            "agent_end" => self.mark_idle(),
            "message_start" => self.observe_message_start(event.get("message")),
            "assistant_text_delta" => self.observe_assistant_text_delta(event),
            "message_update" => self.observe_message_update(event.get("message")),
            "message_end" => self.observe_message_end(event.get("message")),
            "tool_execution_start" => self.observe_tool_start(event),
            "tool_execution_update" => self.observe_tool_update(event),
            "tool_execution_end" => self.observe_tool_end(event),
            _ => {}
        }
    }

    pub fn mark_idle(&mut self) {
        let mut changed = self.streaming;
        self.streaming = false;
        if let Some(index) = self.active_assistant
            && let Some(ChatEntry::Assistant { status, .. }) = self.entries.get_mut(index)
            && matches!(status, AssistantStatus::Streaming)
        {
            *status = AssistantStatus::Complete;
            changed = true;
        }
        for entry in &mut self.entries {
            if let ChatEntry::Tool(tool) = entry
                && matches!(tool.status, ToolStatus::Pending | ToolStatus::Running)
            {
                tool.status = ToolStatus::Complete;
                changed = true;
            }
        }
        if changed {
            self.bump_revision();
        }
    }

    pub fn mark_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.streaming = false;
        let index = self.ensure_assistant_index();
        if let Some(ChatEntry::Assistant { status, .. }) = self.entries.get_mut(index) {
            *status = AssistantStatus::Error(message);
            self.bump_revision();
        }
    }

    pub fn replace_from_snapshot_messages(&mut self, messages: &[Value]) {
        let mut next = Self::default();
        for message in messages {
            next.observe_snapshot_message(message);
        }
        next.streaming = false;
        next.active_assistant = None;

        if self.entries != next.entries || self.streaming != next.streaming {
            let revision = self.revision;
            *self = next;
            self.revision = revision;
            self.bump_revision();
        }
    }

    fn observe_snapshot_message(&mut self, message: &Value) {
        match message.get("role").and_then(Value::as_str) {
            Some("user") => self.push_user_message(message_text(message)),
            Some("assistant") => {
                self.update_assistant_from_message(message, assistant_status(message));
                self.observe_tool_calls_in_message(message);
            }
            Some("toolResult") => self.complete_tool_from_message(message),
            _ => {}
        }
    }

    fn observe_message_start(&mut self, message: Option<&Value>) {
        let Some(message) = message else {
            return;
        };
        match message.get("role").and_then(Value::as_str) {
            Some("user") => self.push_user_message(message_text(message)),
            Some("assistant") => {
                self.update_assistant_from_message(message, AssistantStatus::Streaming)
            }
            Some("toolResult") => self.complete_tool_from_message(message),
            _ => {}
        }
    }

    fn observe_message_update(&mut self, message: Option<&Value>) {
        let Some(message) = message else {
            return;
        };
        if message.get("role").and_then(Value::as_str) != Some("assistant") {
            return;
        }
        self.update_assistant_from_message(message, AssistantStatus::Streaming);
        self.observe_tool_calls_in_message(message);
    }

    fn observe_message_end(&mut self, message: Option<&Value>) {
        let Some(message) = message else {
            return;
        };
        match message.get("role").and_then(Value::as_str) {
            Some("assistant") => {
                let status = assistant_status(message);
                self.update_assistant_from_message(message, status);
                self.observe_tool_calls_in_message(message);
            }
            Some("toolResult") => self.complete_tool_from_message(message),
            _ => {}
        }
    }

    fn observe_tool_start(&mut self, event: &Value) {
        let id = value_string(event, "toolCallId").unwrap_or_else(|| "tool".to_owned());
        let name = value_string(event, "toolName").unwrap_or_else(|| "tool".to_owned());
        let arguments = compact_json(event.get("args"), ARGUMENT_LIMIT);
        self.upsert_tool(id, name, arguments, None, ToolStatus::Running);
    }

    fn observe_tool_update(&mut self, event: &Value) {
        let id = value_string(event, "toolCallId").unwrap_or_else(|| "tool".to_owned());
        let name = value_string(event, "toolName").unwrap_or_else(|| "tool".to_owned());
        let arguments = compact_json(event.get("args"), ARGUMENT_LIMIT);
        let output = tool_result_summary(event.get("partialResult"));
        self.upsert_tool(id, name, arguments, output, ToolStatus::Running);
    }

    fn observe_tool_end(&mut self, event: &Value) {
        let id = value_string(event, "toolCallId").unwrap_or_else(|| "tool".to_owned());
        let name = value_string(event, "toolName").unwrap_or_else(|| "tool".to_owned());
        let output = tool_result_summary(event.get("result"));
        let status = if event
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            ToolStatus::Error
        } else {
            ToolStatus::Complete
        };
        self.upsert_tool(id, name, String::new(), output, status);
    }

    fn observe_tool_calls_in_message(&mut self, message: &Value) {
        let Some(content) = message.get("content").and_then(Value::as_array) else {
            return;
        };
        for item in content {
            if item.get("type").and_then(Value::as_str) != Some("toolCall") {
                continue;
            }
            let id = value_string(item, "id").unwrap_or_else(|| "tool".to_owned());
            let name = value_string(item, "name").unwrap_or_else(|| "tool".to_owned());
            let arguments = compact_json(item.get("arguments"), ARGUMENT_LIMIT);
            self.upsert_tool(id, name, arguments, None, ToolStatus::Pending);
        }
    }

    fn complete_tool_from_message(&mut self, message: &Value) {
        let id = value_string(message, "toolCallId").unwrap_or_else(|| "tool".to_owned());
        let name = value_string(message, "toolName").unwrap_or_else(|| "tool".to_owned());
        let output = tool_result_summary(Some(message));
        let status = if message
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            ToolStatus::Error
        } else {
            ToolStatus::Complete
        };
        self.upsert_tool(id, name, String::new(), output, status);
    }

    fn observe_assistant_text_delta(&mut self, event: &Value) {
        let Some(delta) = value_string(event, "delta").filter(|delta| !delta.is_empty()) else {
            return;
        };
        self.streaming = true;
        let index = self.ensure_assistant_index();
        if let Some(ChatEntry::Assistant { text, status }) = self.entries.get_mut(index) {
            text.push_str(&delta);
            *status = AssistantStatus::Streaming;
            self.bump_revision();
        }
    }

    fn update_assistant_from_message(&mut self, message: &Value, status: AssistantStatus) {
        let text = message_text(message);
        let index = self.ensure_assistant_index();
        if let Some(ChatEntry::Assistant {
            text: current_text,
            status: current_status,
        }) = self.entries.get_mut(index)
            && (*current_text != text || *current_status != status)
        {
            *current_text = text;
            *current_status = status;
            self.bump_revision();
        }
    }

    fn ensure_assistant_index(&mut self) -> usize {
        if let Some(index) = self.active_assistant
            && matches!(self.entries.get(index), Some(ChatEntry::Assistant { .. }))
        {
            return index;
        }
        let index = self.entries.len();
        self.entries.push(ChatEntry::Assistant {
            text: String::new(),
            status: AssistantStatus::Streaming,
        });
        self.active_assistant = Some(index);
        self.bump_revision();
        index
    }

    fn upsert_tool(
        &mut self,
        id: String,
        name: String,
        arguments: String,
        output: Option<String>,
        status: ToolStatus,
    ) {
        if let Some(index) = self.tool_indices.get(&id).copied()
            && let Some(ChatEntry::Tool(tool)) = self.entries.get_mut(index)
        {
            let mut changed = false;
            if !name.trim().is_empty() && tool.name != name {
                tool.name = name;
                changed = true;
            }
            if !arguments.trim().is_empty() && tool.arguments != arguments {
                tool.arguments = arguments;
                changed = true;
            }
            if let Some(output) = output
                && tool.output.as_ref() != Some(&output)
            {
                tool.output = Some(output);
                changed = true;
            }
            if should_replace_tool_status(tool.status, status) {
                tool.status = status;
                changed = true;
            }
            if changed {
                self.bump_revision();
            }
            return;
        }

        let index = self.entries.len();
        self.entries.push(ChatEntry::Tool(ChatToolRun {
            id: id.clone(),
            name,
            arguments,
            output,
            status,
        }));
        self.tool_indices.insert(id, index);
        self.bump_revision();
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    fn last_user_message_is(&self, text: &str) -> bool {
        matches!(self.entries.last(), Some(ChatEntry::User(last)) if last == text)
    }
}

fn assistant_status(message: &Value) -> AssistantStatus {
    match message.get("stopReason").and_then(Value::as_str) {
        Some("error") => AssistantStatus::Error(
            value_string(message, "errorMessage")
                .unwrap_or_else(|| "Pi response failed".to_owned()),
        ),
        Some("aborted") => AssistantStatus::Aborted(
            value_string(message, "errorMessage")
                .unwrap_or_else(|| "Pi response aborted".to_owned()),
        ),
        _ => AssistantStatus::Complete,
    }
}

fn message_text(message: &Value) -> String {
    content_text(message.get("content")).unwrap_or_default()
}

fn tool_result_summary(value: Option<&Value>) -> Option<String> {
    let value = value?;
    content_text(value.get("content"))
        .filter(|text| !text.trim().is_empty())
        .or_else(|| {
            value
                .get("details")
                .map(|details| compact_json(Some(details), SUMMARY_LIMIT))
        })
        .or_else(|| Some(compact_json(Some(value), SUMMARY_LIMIT)))
}

fn content_text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let parts = items
                .iter()
                .filter_map(|item| match item.get("type").and_then(Value::as_str) {
                    Some("text") => value_string(item, "text"),
                    Some("thinking") => value_string(item, "thinking").map(|thinking| {
                        if thinking.trim().is_empty() {
                            String::new()
                        } else {
                            format!("Thinking…\n{thinking}")
                        }
                    }),
                    _ => None,
                })
                .filter(|text| !text.trim().is_empty())
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("\n\n"))
        }
        _ => None,
    }
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn compact_json(value: Option<&Value>, limit: usize) -> String {
    let text = value
        .map(|value| match value {
            Value::Null => String::new(),
            Value::String(text) => text.clone(),
            other => serde_json::to_string_pretty(other).unwrap_or_else(|_error| other.to_string()),
        })
        .unwrap_or_default();
    truncate_chars(text, limit)
}

fn truncate_chars(text: String, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text;
    }
    let mut truncated = text
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

fn should_replace_tool_status(current: ToolStatus, next: ToolStatus) -> bool {
    tool_status_rank(next) >= tool_status_rank(current)
}

fn tool_status_rank(status: ToolStatus) -> u8 {
    match status {
        ToolStatus::Pending => 0,
        ToolStatus::Running => 1,
        ToolStatus::Complete | ToolStatus::Error => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::{AssistantStatus, ChatEntry, ChatTranscript, ToolStatus};

    #[test]
    fn transcript_tracks_assistant_stream() {
        let mut transcript = ChatTranscript::default();
        transcript.push_user_message("hello");
        transcript.observe_session_event(&serde_json::json!({
            "type": "message_update",
            "message": {
                "role": "assistant",
                "content": [{ "type": "text", "text": "hi" }]
            }
        }));
        transcript.observe_session_event(&serde_json::json!({
            "type": "message_end",
            "message": {
                "role": "assistant",
                "content": [{ "type": "text", "text": "hi there" }],
                "stopReason": "stop"
            }
        }));

        assert_eq!(
            transcript.entries(),
            &[
                ChatEntry::User("hello".to_owned()),
                ChatEntry::Assistant {
                    text: "hi there".to_owned(),
                    status: AssistantStatus::Complete,
                },
            ]
        );
    }

    #[test]
    fn transcript_keeps_running_tools_from_downgrading_to_pending() {
        let mut transcript = ChatTranscript::default();
        transcript.observe_session_event(&serde_json::json!({
            "type": "tool_execution_start",
            "toolCallId": "tool-1",
            "toolName": "bash",
            "args": { "command": "pwd" }
        }));
        transcript.observe_session_event(&serde_json::json!({
            "type": "message_update",
            "message": {
                "role": "assistant",
                "content": [{
                    "type": "toolCall",
                    "id": "tool-1",
                    "name": "bash",
                    "arguments": { "command": "pwd" }
                }]
            }
        }));

        let tool = transcript
            .entries()
            .iter()
            .find(|entry| matches!(entry, ChatEntry::Tool(_)));
        assert!(matches!(tool, Some(ChatEntry::Tool(_))));
        if let Some(ChatEntry::Tool(tool)) = tool {
            assert_eq!(tool.status, ToolStatus::Running);
        }
    }

    #[test]
    fn transcript_appends_compact_assistant_text_deltas() {
        let mut transcript = ChatTranscript::default();
        transcript.observe_session_event(&serde_json::json!({
            "type": "agent_start"
        }));
        transcript.observe_session_event(&serde_json::json!({
            "type": "assistant_text_delta",
            "delta": "hi"
        }));
        transcript.observe_session_event(&serde_json::json!({
            "type": "assistant_text_delta",
            "delta": " there"
        }));

        assert_eq!(
            transcript.entries(),
            &[ChatEntry::Assistant {
                text: "hi there".to_owned(),
                status: AssistantStatus::Streaming,
            }]
        );
        assert!(transcript.is_streaming());
    }

    #[test]
    fn transcript_tracks_tool_lifecycle() {
        let mut transcript = ChatTranscript::default();
        transcript.observe_session_event(&serde_json::json!({
            "type": "message_update",
            "message": {
                "role": "assistant",
                "content": [{
                    "type": "toolCall",
                    "id": "tool-1",
                    "name": "read",
                    "arguments": { "path": "README.md" }
                }]
            }
        }));
        transcript.observe_session_event(&serde_json::json!({
            "type": "tool_execution_start",
            "toolCallId": "tool-1",
            "toolName": "read",
            "args": { "path": "README.md" }
        }));
        transcript.observe_session_event(&serde_json::json!({
            "type": "tool_execution_end",
            "toolCallId": "tool-1",
            "toolName": "read",
            "result": {
                "content": [{ "type": "text", "text": "# Pi" }]
            },
            "isError": false
        }));

        let tool = transcript.entries().last();
        assert!(matches!(tool, Some(ChatEntry::Tool(_))));
        if let Some(ChatEntry::Tool(tool)) = tool {
            assert_eq!(tool.name, "read");
            assert_eq!(tool.status, ToolStatus::Complete);
            assert_eq!(tool.output.as_deref(), Some("# Pi"));
        }
    }

    #[test]
    fn transcript_hydrates_from_snapshot_messages() {
        let mut transcript = ChatTranscript::default();
        transcript.replace_from_snapshot_messages(&[
            serde_json::json!({
                "role": "user",
                "content": [{ "type": "text", "text": "hello" }]
            }),
            serde_json::json!({
                "role": "assistant",
                "content": [{ "type": "text", "text": "hi there" }]
            }),
        ]);

        assert_eq!(transcript.entries().len(), 2);
        assert!(matches!(transcript.entries()[0], ChatEntry::User(ref text) if text == "hello"));
        assert!(matches!(
            transcript.entries()[1],
            ChatEntry::Assistant {
                ref text,
                status: AssistantStatus::Complete,
            } if text == "hi there"
        ));
    }
}
