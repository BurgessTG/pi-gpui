use super::*;

impl PiDesktop {
    pub(super) fn start_backend(&mut self, cx: &mut Context<Self>) {
        if self.backend.is_some() {
            self.run_backend(
                "Refreshing Pi worker backend…",
                "Pi worker backend ready.",
                false,
                |backend| backend.refresh(),
                cx,
            );
            return;
        }
        self.pending = true;
        self.status = "Starting Pi worker backend…".into();
        cx.notify();
        let cwd = self.cwd.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { BackendSession::connect(cwd) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.pending = false;
                match result {
                    Ok(snapshot) => view.apply_snapshot(snapshot, cx),
                    Err(error) => {
                        view.backend = None;
                        view.data = None;
                        view.status = format!("Backend unavailable: {error:#}").into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn apply_snapshot(&mut self, snapshot: BackendSnapshot, cx: &mut Context<Self>) {
        self.backend = Some(snapshot.session);
        self.subscribe_auth_updates(cx);
        self.subscribe_backend_events(cx);
        self.apply_data(
            snapshot.data,
            "Provider auth loaded. Starting Pi worker runtime…",
            cx,
        );
        self.start_agent_runtime(cx);
    }

    pub(super) fn start_agent_runtime(&mut self, cx: &mut Context<Self>) {
        if self.agent_ready() {
            return;
        }
        let Some(session) = self.backend.clone() else {
            return;
        };
        let cwd = self.cwd.clone();
        self.status = "Provider auth loaded. Starting Pi worker runtime…".into();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { session.init_runtime(cwd) })
                .await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(data) => view.apply_data(data, "Pi worker runtime ready.", cx),
                    Err(error) => {
                        view.status = format!(
                            "Provider auth loaded, but Pi runtime failed to start: {error:#}"
                        )
                        .into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn subscribe_auth_updates(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.backend.clone() else {
            return;
        };
        let mut receiver = session.subscribe_auth_updates();
        cx.spawn(async move |this, cx| {
            loop {
                let (next_receiver, result) = cx
                    .background_spawn(async move {
                        let result = receiver.recv().await;
                        (receiver, result)
                    })
                    .await;
                receiver = next_receiver;
                match result {
                    Ok(update) => {
                        let _ = this.update(cx, |view, cx| {
                            view.status = auth_update_status(&update).into();
                            cx.notify();
                        });
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        })
        .detach();
    }

    pub(super) fn subscribe_backend_events(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.backend.clone() else {
            return;
        };
        let mut receiver = session.subscribe_events();
        cx.spawn(async move |this, cx| {
            loop {
                let (next_receiver, result) = cx
                    .background_spawn(async move {
                        let result = match receiver.recv().await {
                            Ok(first) => {
                                let mut events = Vec::with_capacity(128);
                                events.push(first);
                                Timer::after(BACKEND_EVENT_BATCH_INTERVAL).await;
                                while events.len() < 512 {
                                    match receiver.try_recv() {
                                        Ok(event) => events.push(event),
                                        Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                                            break;
                                        }
                                        Err(
                                            tokio::sync::broadcast::error::TryRecvError::Lagged(_),
                                        ) => {
                                            continue;
                                        }
                                        Err(
                                            tokio::sync::broadcast::error::TryRecvError::Closed,
                                        ) => {
                                            break;
                                        }
                                    }
                                }
                                Ok(events)
                            }
                            Err(error) => Err(error),
                        };
                        (receiver, result)
                    })
                    .await;
                receiver = next_receiver;
                match result {
                    Ok(envelopes) => {
                        let _ = this.update(cx, |view, cx| {
                            view.apply_bridge_events(envelopes, cx);
                        });
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        })
        .detach();
    }

    pub(super) fn apply_bridge_events(
        &mut self,
        envelopes: Vec<BridgeEventEnvelope>,
        cx: &mut Context<Self>,
    ) {
        let mut session_events: HashMap<(Option<String>, Option<String>), Vec<serde_json::Value>> =
            HashMap::new();
        for envelope in envelopes {
            match envelope.event {
                BridgeEvent::PiSessionEvent {
                    session_id,
                    session_file,
                    event,
                } => {
                    session_events
                        .entry((session_id, session_file))
                        .or_default()
                        .push(event);
                }
                BridgeEvent::SessionRunStarted {
                    session_id,
                    session_file,
                } => {
                    session_events
                        .entry((session_id, session_file))
                        .or_default()
                        .push(serde_json::json!({ "type": "agent_start" }));
                }
                BridgeEvent::SessionRunFinished {
                    session_id,
                    session_file,
                } => {
                    session_events
                        .entry((session_id, session_file))
                        .or_default()
                        .push(serde_json::json!({ "type": "agent_end" }));
                }
                BridgeEvent::SessionRunError {
                    session_id,
                    session_file,
                    message,
                } => {
                    session_events
                        .entry((session_id, session_file))
                        .or_default()
                        .push(serde_json::json!({
                            "type": "agent_error",
                            "message": message,
                        }));
                }
                BridgeEvent::SessionTextDelta {
                    session_id,
                    session_file,
                    delta,
                } => {
                    session_events
                        .entry((session_id, session_file))
                        .or_default()
                        .push(serde_json::json!({
                            "type": "assistant_text_delta",
                            "delta": delta,
                        }));
                }
                BridgeEvent::SessionToolStarted {
                    session_id,
                    session_file,
                    tool_call_id,
                    tool_name,
                    args,
                } => {
                    session_events
                        .entry((session_id, session_file))
                        .or_default()
                        .push(serde_json::json!({
                            "type": "tool_execution_start",
                            "toolCallId": tool_call_id,
                            "toolName": tool_name,
                            "args": args,
                        }));
                }
                BridgeEvent::SessionToolUpdated {
                    session_id,
                    session_file,
                    tool_call_id,
                    tool_name,
                    args,
                    partial_result,
                } => {
                    session_events
                        .entry((session_id, session_file))
                        .or_default()
                        .push(serde_json::json!({
                            "type": "tool_execution_update",
                            "toolCallId": tool_call_id,
                            "toolName": tool_name,
                            "args": args,
                            "partialResult": partial_result,
                        }));
                }
                BridgeEvent::SessionToolFinished {
                    session_id,
                    session_file,
                    tool_call_id,
                    tool_name,
                    result,
                    is_error,
                } => {
                    session_events
                        .entry((session_id, session_file))
                        .or_default()
                        .push(serde_json::json!({
                            "type": "tool_execution_end",
                            "toolCallId": tool_call_id,
                            "toolName": tool_name,
                            "result": result,
                            "isError": is_error,
                        }));
                }
                event => self.apply_bridge_event(
                    BridgeEventEnvelope {
                        version: envelope.version,
                        event,
                    },
                    cx,
                ),
            }
        }
        for ((session_id, session_file), events) in session_events {
            self.apply_session_events(session_id.as_deref(), session_file.as_deref(), events, cx);
        }
    }

    fn apply_session_events(
        &mut self,
        session_id: Option<&str>,
        session_file: Option<&str>,
        events: Vec<serde_json::Value>,
        cx: &mut Context<Self>,
    ) {
        if events.is_empty() {
            return;
        }
        let Some(key) = self.node_key_for_session_event(session_id, session_file) else {
            return;
        };
        let status = events.last().map(chat_event_status);
        self.update_chat_transcript(key, cx, |transcript| {
            for event in &events {
                transcript.observe_session_event(event);
            }
        });
        if let Some(status) = status {
            self.status = status.into();
        }
        if events.iter().any(is_terminal_chat_event) && self.streaming_nodes.remove(&key) {
            self.start_next_queued_chat_prompt(cx);
        }
    }

    fn node_key_for_session_event(
        &self,
        session_id: Option<&str>,
        session_file: Option<&str>,
    ) -> Option<(usize, usize)> {
        self.workspace_state.tabs().iter().find_map(|tab| {
            tab.canvas().nodes().iter().find_map(|node| {
                let metadata = node.metadata();
                let matches_id =
                    session_id.is_some() && metadata.session_id.as_deref() == session_id;
                let matches_file =
                    session_file.is_some() && metadata.session_file.as_deref() == session_file;
                (matches_id || matches_file).then_some((tab.id(), node.id()))
            })
        })
    }

    pub(super) fn apply_bridge_event(
        &mut self,
        envelope: BridgeEventEnvelope,
        cx: &mut Context<Self>,
    ) {
        match envelope.event {
            BridgeEvent::PiSessionEvent {
                session_id,
                session_file,
                event,
            } => {
                self.apply_session_events(
                    session_id.as_deref(),
                    session_file.as_deref(),
                    vec![event],
                    cx,
                );
            }
            BridgeEvent::SessionRunStarted {
                session_id,
                session_file,
            } => {
                self.apply_session_events(
                    session_id.as_deref(),
                    session_file.as_deref(),
                    vec![serde_json::json!({ "type": "agent_start" })],
                    cx,
                );
            }
            BridgeEvent::SessionRunFinished {
                session_id,
                session_file,
            } => {
                self.apply_session_events(
                    session_id.as_deref(),
                    session_file.as_deref(),
                    vec![serde_json::json!({ "type": "agent_end" })],
                    cx,
                );
            }
            BridgeEvent::SessionRunError {
                session_id,
                session_file,
                message,
            } => {
                self.apply_session_events(
                    session_id.as_deref(),
                    session_file.as_deref(),
                    vec![serde_json::json!({
                        "type": "agent_error",
                        "message": message,
                    })],
                    cx,
                );
            }
            BridgeEvent::SessionTextDelta {
                session_id,
                session_file,
                delta,
            } => {
                self.apply_session_events(
                    session_id.as_deref(),
                    session_file.as_deref(),
                    vec![serde_json::json!({
                        "type": "assistant_text_delta",
                        "delta": delta,
                    })],
                    cx,
                );
            }
            BridgeEvent::SessionToolStarted {
                session_id,
                session_file,
                tool_call_id,
                tool_name,
                args,
            } => {
                self.apply_session_events(
                    session_id.as_deref(),
                    session_file.as_deref(),
                    vec![serde_json::json!({
                        "type": "tool_execution_start",
                        "toolCallId": tool_call_id,
                        "toolName": tool_name,
                        "args": args,
                    })],
                    cx,
                );
            }
            BridgeEvent::SessionToolUpdated {
                session_id,
                session_file,
                tool_call_id,
                tool_name,
                args,
                partial_result,
            } => {
                self.apply_session_events(
                    session_id.as_deref(),
                    session_file.as_deref(),
                    vec![serde_json::json!({
                        "type": "tool_execution_update",
                        "toolCallId": tool_call_id,
                        "toolName": tool_name,
                        "args": args,
                        "partialResult": partial_result,
                    })],
                    cx,
                );
            }
            BridgeEvent::SessionToolFinished {
                session_id,
                session_file,
                tool_call_id,
                tool_name,
                result,
                is_error,
            } => {
                self.apply_session_events(
                    session_id.as_deref(),
                    session_file.as_deref(),
                    vec![serde_json::json!({
                        "type": "tool_execution_end",
                        "toolCallId": tool_call_id,
                        "toolName": tool_name,
                        "result": result,
                        "isError": is_error,
                    })],
                    cx,
                );
            }
            BridgeEvent::StateSnapshot { state } => {
                if let Some(data) = &mut self.data {
                    data.state = Some(state);
                    self.request_event_render(cx);
                }
            }
            BridgeEvent::QueueUpdate { queue, .. } => {
                if let Some(state) = self.data.as_mut().and_then(|data| data.state.as_mut()) {
                    state.queue = queue;
                    self.request_event_render(cx);
                }
            }
            BridgeEvent::FatalError { error } => {
                self.status = format!("Pi backend error: {}", error.message).into();
                cx.notify();
            }
            _ => {}
        }
    }

    pub(super) fn apply_data(
        &mut self,
        data: BackendData,
        status: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = data.state.as_ref() {
            let metadata = session_node_metadata(state);
            self.workspace_state.sync_session_metadata(&metadata);
            self.hydrate_chat_transcripts_from_state(&metadata, &state.messages, cx);
        }
        self.canvas_node_registry = CanvasNodeRegistry::with_installed_packages(&data.packages);
        self.data = Some(data);
        self.status = status.into();
    }

    pub(super) fn run_backend(
        &mut self,
        pending_status: impl Into<SharedString>,
        success_status: impl Into<SharedString>,
        clear_selected_provider: bool,
        operation: impl FnOnce(Arc<BackendSession>) -> Result<BackendData> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.backend.clone() else {
            self.status = "Backend is not ready yet.".into();
            cx.notify();
            return;
        };
        let success_status = success_status.into();
        self.pending = true;
        self.status = pending_status.into();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { operation(session) }).await;
            let _ = this.update(cx, |view, cx| {
                view.pending = false;
                match result {
                    Ok(data) => {
                        if clear_selected_provider {
                            view.selected_provider = None;
                            view.auth_flow = AuthFlow::Choose;
                        }
                        view.apply_data(data, success_status, cx);
                    }
                    Err(error) => view.status = format!("Backend error: {error:#}").into(),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn request_event_render(&mut self, cx: &mut Context<Self>) {
        if self.event_render_scheduled {
            return;
        }
        self.event_render_scheduled = true;
        cx.spawn(async move |this, cx| {
            Timer::after(FRAME_RENDER_INTERVAL).await;
            let _ = this.update(cx, |view, cx| {
                view.event_render_scheduled = false;
                cx.notify();
            });
        })
        .detach();
    }
}
