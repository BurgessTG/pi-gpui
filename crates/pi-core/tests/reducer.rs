use pi_bridge_types::{BridgeEvent, BridgeEventEnvelope, QueueSnapshot};
use pi_core::{ApplyEvent, BackendState, TranscriptItem};

#[test]
fn reducer_tracks_ready_and_queue() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = BackendState::new();
    state.apply_event(BridgeEventEnvelope::new(BridgeEvent::Ready(
        pi_bridge_types::ReadyEvent {
            node_version: "24".to_owned(),
            pi_version: Some("0.78.0".to_owned()),
            protocol_version: pi_bridge_types::PROTOCOL_VERSION,
        },
    )))?;
    state.apply_event(BridgeEventEnvelope::new(BridgeEvent::QueueUpdate {
        session_id: None,
        session_file: None,
        queue: QueueSnapshot {
            steering: vec!["a".to_owned()],
            follow_up: vec!["b".to_owned()],
        },
    }))?;
    assert!(state.ready);
    assert_eq!(state.snapshot.queue.steering, ["a"]);
    assert_eq!(state.snapshot.queue.follow_up, ["b"]);
    Ok(())
}

#[test]
fn reducer_extracts_text_deltas() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = BackendState::new();
    state.apply_event(BridgeEventEnvelope::new(BridgeEvent::PiSessionEvent {
        session_id: None,
        session_file: None,
        event: serde_json::json!({
            "type": "message_update",
            "assistantMessageEvent": { "type": "text_delta", "delta": "hello" }
        }),
    }))?;
    state.apply_event(BridgeEventEnvelope::new(BridgeEvent::SessionTextDelta {
        session_id: None,
        session_file: None,
        delta: " world".to_owned(),
    }))?;
    assert_eq!(
        state.transcript,
        vec![
            TranscriptItem::TextDelta("hello".to_owned()),
            TranscriptItem::TextDelta(" world".to_owned()),
        ]
    );
    Ok(())
}

#[test]
fn reducer_tracks_typed_tool_events() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = BackendState::new();
    state.apply_event(BridgeEventEnvelope::new(BridgeEvent::SessionToolUpdated {
        session_id: None,
        session_file: None,
        tool_call_id: "tool-1".to_owned(),
        tool_name: "bash".to_owned(),
        args: serde_json::json!({ "command": "pwd" }),
        partial_result: Some(serde_json::json!({ "stdout": "/tmp" })),
    }))?;
    assert_eq!(state.transcript.len(), 1);
    assert!(matches!(state.transcript[0], TranscriptItem::ToolUpdate(_)));
    Ok(())
}
