use super::*;

impl PiDesktop {
    pub(crate) fn submit_chat_node_from_enter(
        &mut self,
        workspace_id: usize,
        node_id: usize,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        remove_auto_inserted_enter_newline(input, window, cx);
        self.submit_chat_node(workspace_id, node_id, window, cx);
    }

    pub(crate) fn submit_chat_node(
        &mut self,
        workspace_id: usize,
        node_id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = (workspace_id, node_id);
        let Some(input) = self.chat_inputs.get(&key).cloned() else {
            self.status = "Chat node input is not ready yet.".into();
            cx.notify();
            return;
        };
        if !self.agent_ready() {
            self.status = "Embedded Pi runtime is still starting.".into();
            cx.notify();
            return;
        }
        if self.pending || self.streaming_node.is_some() {
            self.status = "Pi is busy; wait for the current stream to finish.".into();
            cx.notify();
            return;
        }
        let Some(session) = self.backend.clone() else {
            self.status = "Embedded Pi backend is not ready yet.".into();
            cx.notify();
            return;
        };
        let text = input.read(cx).value().trim().to_owned();
        if text.is_empty() {
            self.status = "Type a message before sending.".into();
            cx.notify();
            return;
        }
        let Some(workspace_index) = self.workspace_index_for_id(workspace_id) else {
            self.status = "Session node is no longer available.".into();
            cx.notify();
            return;
        };
        let Some(session_path) = self.session_node_session_path(workspace_index, node_id) else {
            self.status = "Wait for this Pi session node to finish syncing before sending.".into();
            cx.notify();
            return;
        };

        input.update(cx, |input, cx| input.set_value("", window, cx));
        self.update_chat_transcript(key, cx, |transcript| {
            transcript.push_user_message(text.clone());
        });
        self.streaming_node = Some(key);
        self.pending = true;
        self.status = format!("Sending chat node #{node_id} to Pi…").into();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { session.prompt(Some(session_path), text) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.pending = false;
                let Some(workspace_index) = view.workspace_index_for_id(workspace_id) else {
                    view.remove_session_node_ui_state(workspace_id, node_id);
                    view.status = "Pi chat response arrived after its workspace was closed.".into();
                    cx.notify();
                    return;
                };
                if view.session_node_title(workspace_index, node_id).is_none() {
                    view.remove_session_node_ui_state(workspace_id, node_id);
                    view.status = "Pi chat response arrived after its node was closed.".into();
                    cx.notify();
                    return;
                }
                match result {
                    Ok(data) => {
                        let metadata = data
                            .state
                            .as_ref()
                            .map(session_node_metadata)
                            .unwrap_or_else(empty_session_node_metadata);
                        view.workspace_state.update_session_node_metadata(
                            workspace_index,
                            node_id,
                            metadata,
                        );
                        view.update_chat_transcript(key, cx, ChatTranscript::mark_idle);
                        view.streaming_node = None;
                        view.apply_data(data, "Pi chat response complete.", cx);
                    }
                    Err(error) => {
                        let message = format!("Pi chat failed: {error:#}");
                        view.update_chat_transcript(key, cx, |transcript| {
                            transcript.mark_error(message.clone());
                        });
                        view.streaming_node = None;
                        view.status = message.into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn start_session_title_edit(
        &mut self,
        workspace_id: usize,
        node_id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = (workspace_id, node_id);
        let Some(workspace_index) = self.workspace_index_for_id(workspace_id) else {
            self.status = "Session node is no longer available.".into();
            cx.notify();
            return;
        };
        let Some(title) = self.session_node_title(workspace_index, node_id) else {
            self.status = "Session node is no longer available.".into();
            cx.notify();
            return;
        };
        let Some(input) = self.title_inputs.get(&key).cloned() else {
            self.status = "Session title editor is not ready yet.".into();
            cx.notify();
            return;
        };

        self.editing_title = Some(key);
        input.update(cx, |input, cx| {
            input.set_value(title, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub(crate) fn commit_session_title_edit(
        &mut self,
        workspace_id: usize,
        node_id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = (workspace_id, node_id);
        if self.editing_title != Some(key) {
            return;
        }
        let Some(input) = self.title_inputs.get(&key).cloned() else {
            self.editing_title = None;
            self.status = "Session title editor is not ready yet.".into();
            cx.notify();
            return;
        };
        let Some(workspace_index) = self.workspace_index_for_id(workspace_id) else {
            self.editing_title = None;
            self.status = "Session node is no longer available.".into();
            cx.notify();
            return;
        };
        let next_name = input.read(cx).value().trim().to_owned();
        let current_title = self.session_node_title(workspace_index, node_id);
        if next_name.is_empty() {
            if let Some(title) = current_title {
                input.update(cx, |input, cx| input.set_value(title, window, cx));
            }
            self.editing_title = None;
            self.status = "Session name was not changed.".into();
            cx.notify();
            return;
        }
        if current_title.as_deref() == Some(next_name.as_str()) {
            self.editing_title = None;
            cx.notify();
            return;
        }

        let Some(session) = self.backend.clone() else {
            self.editing_title = None;
            self.status = "Embedded Pi backend is not ready yet.".into();
            cx.notify();
            return;
        };
        let session_path = self.session_node_session_path(workspace_index, node_id);
        self.editing_title = None;
        self.pending = true;
        self.status = format!("Renaming Pi session to {next_name}…").into();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let requested_name = next_name.clone();
            let result = cx
                .background_spawn(async move { session.set_session_name(session_path, next_name) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.pending = false;
                let Some(workspace_index) = view.workspace_index_for_id(workspace_id) else {
                    view.remove_session_node_ui_state(workspace_id, node_id);
                    view.status =
                        "Pi session rename finished after its workspace was closed.".into();
                    cx.notify();
                    return;
                };
                if view.session_node_title(workspace_index, node_id).is_none() {
                    view.remove_session_node_ui_state(workspace_id, node_id);
                    view.status = "Pi session rename finished after its node was closed.".into();
                    cx.notify();
                    return;
                }
                match result {
                    Ok(data) => {
                        let metadata = data
                            .state
                            .as_ref()
                            .map(session_node_metadata)
                            .unwrap_or_else(|| SessionNodeMetadata {
                                session_id: None,
                                session_name: Some(requested_name.clone()),
                                session_file: None,
                                cwd: None,
                                message_count: 0,
                            });
                        view.workspace_state.update_session_node_metadata(
                            workspace_index,
                            node_id,
                            metadata,
                        );
                        view.apply_data(data, "Pi session renamed.", cx);
                    }
                    Err(error) => {
                        view.status = format!("Pi session rename failed: {error:#}").into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn session_node_title(
        &self,
        workspace_index: usize,
        node_id: usize,
    ) -> Option<String> {
        self.workspace_state
            .tab(workspace_index)?
            .canvas()
            .nodes()
            .iter()
            .find(|node| node.id() == node_id)
            .map(|node| node.title())
    }

    pub(super) fn session_node_session_path(
        &self,
        workspace_index: usize,
        node_id: usize,
    ) -> Option<String> {
        self.workspace_state
            .tab(workspace_index)?
            .canvas()
            .nodes()
            .iter()
            .find(|node| node.id() == node_id)
            .and_then(|node| node.metadata().session_file.clone())
    }

    pub(crate) fn create_new_session_node(&mut self, cx: &mut Context<Self>) {
        self.create_session_node(SessionNodePrimitive::NewSession, None, cx);
    }

    pub(crate) fn create_fork_session_node(&mut self, cx: &mut Context<Self>) {
        let entry_id = self
            .runtime_state()
            .and_then(|state| latest_json_id(&state.messages));
        let Some(entry_id) = entry_id else {
            self.status = "Fork session nodes need an existing Pi message entry.".into();
            cx.notify();
            return;
        };
        self.create_session_node(SessionNodePrimitive::ForkSession, Some(entry_id), cx);
    }

    pub(crate) fn create_resume_session_node(&mut self, cx: &mut Context<Self>) {
        let session_path = self
            .runtime_state()
            .and_then(|state| state.session_file.clone());
        let Some(session_path) = session_path else {
            self.status = "Resume session nodes need an existing Pi session file.".into();
            cx.notify();
            return;
        };
        self.create_session_node(SessionNodePrimitive::ResumeSession, Some(session_path), cx);
    }

    pub(super) fn create_session_node(
        &mut self,
        primitive: SessionNodePrimitive,
        argument: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.backend.clone() else {
            self.status = "Embedded Pi backend is not ready yet.".into();
            cx.notify();
            return;
        };
        if !self.agent_ready() {
            self.status = "Embedded Pi runtime is still starting.".into();
            cx.notify();
            return;
        }
        let Some(workspace_index) = self.workspace_state.active_index() else {
            self.status = "Open a workspace before creating a chat node.".into();
            cx.notify();
            return;
        };
        let Some(workspace_id) = self.workspace_id_for_index(workspace_index) else {
            self.status = "Open a workspace before creating a chat node.".into();
            cx.notify();
            return;
        };
        let Some(node_id) = self.workspace_state.add_session_node_to_active_canvas(
            primitive,
            pending_session_node_metadata(primitive),
            self.snap_to_grid,
        ) else {
            self.status = "No active workspace is available for the chat node.".into();
            cx.notify();
            return;
        };

        self.pending = true;
        self.status = format!(
            "Opened {} node instantly; syncing {} in the background…",
            primitive.label(),
            primitive.status_label()
        )
        .into();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    match primitive {
                        SessionNodePrimitive::NewSession => session.new_session(),
                        SessionNodePrimitive::ForkSession => session.fork_session(
                            argument.ok_or_else(|| anyhow::anyhow!("missing fork entry id"))?,
                        ),
                        SessionNodePrimitive::ResumeSession => session.switch_session(
                            argument.ok_or_else(|| anyhow::anyhow!("missing session path"))?,
                        ),
                    }
                })
                .await;

            let _ = this.update(cx, |view, cx| {
                view.pending = false;
                match result {
                    Ok(result) if result.cancelled => {
                        view.apply_data(result.data, "Pi session command cancelled.", cx);
                        view.remove_session_node_locally(workspace_id, node_id);
                        view.status = "Pi session command cancelled.".into();
                    }
                    Ok(result) => {
                        let data = result.data;
                        let metadata = data
                            .state
                            .as_ref()
                            .map(session_node_metadata)
                            .unwrap_or_else(empty_session_node_metadata);
                        let Some(workspace_index) = view.workspace_index_for_id(workspace_id)
                        else {
                            view.remove_session_node_ui_state(workspace_id, node_id);
                            view.status =
                                "Pi session command succeeded, but the opening workspace is gone."
                                    .into();
                            cx.notify();
                            return;
                        };
                        if view.workspace_state.update_session_node_metadata(
                            workspace_index,
                            node_id,
                            metadata,
                        ) {
                            view.apply_data(data, "Pi session node ready.", cx);
                        } else {
                            view.remove_session_node_ui_state(workspace_id, node_id);
                            view.status =
                                "Pi session command succeeded, but the opening node is gone."
                                    .into();
                        }
                    }
                    Err(error) => {
                        view.remove_session_node_locally(workspace_id, node_id);
                        view.status = format!("Pi session node failed: {error:#}").into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}
