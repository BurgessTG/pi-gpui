use super::*;

impl PiDesktop {
    pub(super) fn open_landing(&mut self, cx: &mut Context<Self>) {
        self.workspace_dialog = None;
        self.showing_landing = true;
        self.close_settings_drawer(cx);
        cx.notify();
    }

    pub(super) fn toggle_settings_drawer(&mut self, cx: &mut Context<Self>) {
        if self.settings_drawer_open {
            self.close_settings_drawer(cx);
        } else {
            self.settings_drawer_visible = true;
            self.settings_drawer_open = true;
            cx.notify();
        }
    }

    pub(super) fn close_settings_drawer(&mut self, cx: &mut Context<Self>) {
        self.settings_drawer_open = false;
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(DRAWER_ANIMATION_DURATION).await;
            let _ = this.update(cx, |view, cx| {
                if !view.settings_drawer_open {
                    view.settings_drawer_visible = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn select_provider(&mut self, provider: String, cx: &mut Context<Self>) {
        self.selected_provider = Some(provider);
        self.hovered_provider = None;
        self.hover_card_provider = None;
        self.auth_flow = AuthFlow::Choose;
        cx.notify();
    }

    pub(crate) fn start_provider_hover(&mut self, provider: String, cx: &mut Context<Self>) {
        self.hovered_provider = Some(provider.clone());
        self.hover_card_provider = None;
        cx.spawn(async move |this, cx| {
            Timer::after(PROVIDER_HOVER_DELAY).await;
            let _ = this.update(cx, |view, cx| {
                if view.hovered_provider.as_deref() == Some(provider.as_str()) {
                    view.hover_card_provider = Some(provider);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn end_provider_hover(&mut self, provider: &str, cx: &mut Context<Self>) {
        if self.hovered_provider.as_deref() == Some(provider) {
            self.hovered_provider = None;
        }
        if self.hover_card_provider.as_deref() == Some(provider) {
            self.hover_card_provider = None;
        }
        cx.notify();
    }

    pub(crate) fn show_api_key_flow(&mut self, cx: &mut Context<Self>) {
        self.auth_flow = AuthFlow::ApiKey;
        cx.notify();
    }

    pub(crate) fn start_oauth_login(&mut self, method: OAuthLoginMethod, cx: &mut Context<Self>) {
        let Some(provider) = self.selected_provider.clone() else {
            self.status = "Pick a provider first.".into();
            cx.notify();
            return;
        };
        if !oauth_methods_for(&provider).contains(&method) {
            self.status = "That provider does not support the selected sign-in flow.".into();
            cx.notify();
            return;
        }
        let label = match method {
            OAuthLoginMethod::Browser => "Opening browser sign-in with Pi auth…",
            OAuthLoginMethod::DeviceCode => "Opening device-code sign-in with Pi auth…",
        };
        self.run_backend(
            label,
            "Provider authenticated successfully.",
            false,
            move |backend| backend.oauth_login(provider, Some(method)),
            cx,
        );
    }

    pub(crate) fn save_api_key(&mut self, persist: bool, cx: &mut Context<Self>) {
        let Some(provider) = self.selected_provider.clone() else {
            self.status = "Pick a provider first.".into();
            cx.notify();
            return;
        };
        let api_key = self.api_key_input.read(cx).value().to_string();
        if api_key.trim().is_empty() {
            self.status = "Paste an API key before saving.".into();
            cx.notify();
            return;
        }
        let label = if persist {
            "Saving API key to Pi auth storage…"
        } else {
            "Activating API key for this run…"
        };
        let success = if persist {
            "API key saved. Provider configured."
        } else {
            "API key active for this run. Provider configured."
        };
        self.run_backend(
            label,
            success,
            false,
            move |backend| backend.save_api_key(provider, api_key, persist),
            cx,
        );
    }

    pub(crate) fn remove_selected_auth(&mut self, cx: &mut Context<Self>) {
        let Some(provider) = self.selected_provider.clone() else {
            self.status = "Pick a provider first.".into();
            cx.notify();
            return;
        };
        self.run_backend(
            "Removing provider auth…",
            "Provider auth removed.",
            false,
            move |backend| backend.remove_auth(provider),
            cx,
        );
    }

    pub(crate) fn create_workspace_from_name(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = self.workspace_name_input.read(cx).value().to_string();
        if self.workspace_state.add_named_blank(name.clone()).is_none() {
            self.status = "Workspace name cannot be empty.".into();
            cx.notify();
            return;
        }

        self.workspace_name_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.workspace_dialog = None;
        self.showing_landing = false;
        self.status = format!("Created Pi Workspace: {}", name.trim()).into();
        cx.notify();
    }

    pub(crate) fn start_open_workspace_flow(&mut self, cx: &mut Context<Self>) {
        self.close_settings_drawer(cx);
        self.refresh_workspace_picker(cx);
        self.pending_delete_folder = None;
        self.showing_delete_folder_confirmation = false;
        self.workspace_dialog = Some(WorkspaceDialog::OpenWorkspace);
        self.status = "Choose a folder to open as a Pi Workspace.".into();
        cx.notify();
    }

    pub(crate) fn open_selected_workspace(&mut self, cx: &mut Context<Self>) {
        let selected_path = self.selected_workspace_path(cx);
        self.open_workspace_path(selected_path, cx);
    }

    pub(crate) fn navigate_workspace_picker_root(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path.is_dir() {
            self.status = format!("Not a folder: {}", path.display()).into();
            cx.notify();
            return;
        }

        self.workspace_picker_root = std::fs::canonicalize(&path).unwrap_or(path);
        self.refresh_workspace_picker(cx);
        self.status = format!(
            "Browsing folders from: {}",
            self.workspace_picker_root.display()
        )
        .into();
        cx.notify();
    }

    pub(crate) fn start_new_folder_flow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parent = self.selected_workspace_path(cx);
        if !parent.is_dir() {
            cx.notify();
            return;
        }

        let name = next_new_folder_name(&parent);
        self.new_folder_input_visible = true;
        self.showing_new_folder_input = true;
        self.new_folder_name_input.update(cx, |input, cx| {
            input.set_value(&name, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub(crate) fn create_folder_in_selected_workspace_path(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let parent = self.selected_workspace_path(cx);
        if !parent.is_dir() {
            cx.notify();
            return;
        }

        let name = self
            .new_folder_name_input
            .read(cx)
            .value()
            .trim()
            .to_owned();
        if !valid_new_folder_name(&name) {
            cx.notify();
            return;
        }

        let folder = parent.join(&name);
        if folder.exists() {
            cx.notify();
            return;
        }

        if std::fs::create_dir(&folder).is_ok() {
            self.showing_new_folder_input = false;
            self.refresh_workspace_picker_to(parent, cx);
            cx.spawn(async move |this, cx| {
                Timer::after(NEW_FOLDER_ROW_ANIMATION).await;
                let _ = this.update(cx, |view, cx| {
                    if !view.showing_new_folder_input {
                        view.new_folder_input_visible = false;
                    }
                    cx.notify();
                });
            })
            .detach();
        }
        cx.notify();
    }

    pub(crate) fn request_delete_workspace_folder(
        &mut self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        if !path.is_dir() {
            self.status = format!("Not a folder: {}", path.display()).into();
            cx.notify();
            return;
        }
        if path == self.workspace_picker_root {
            self.status = "The current picker root cannot be deleted here.".into();
            cx.notify();
            return;
        }

        self.pending_delete_folder = Some(path.clone());
        self.showing_delete_folder_confirmation = true;
        self.status = format!("Confirm deleting folder: {}", path.display()).into();
        cx.notify();
    }

    pub(crate) fn cancel_delete_workspace_folder(&mut self, cx: &mut Context<Self>) {
        self.hide_delete_folder_confirmation(cx);
        self.status = "Folder deletion cancelled.".into();
        cx.notify();
    }

    pub(crate) fn confirm_delete_workspace_folder(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.pending_delete_folder.clone() else {
            cx.notify();
            return;
        };
        if path == self.workspace_picker_root {
            self.status = "The current picker root cannot be deleted here.".into();
            cx.notify();
            return;
        }

        match std::fs::remove_dir_all(&path) {
            Ok(()) => {
                let parent = path.parent().map(Path::to_path_buf);
                self.status = format!("Deleted folder: {}", path.display()).into();
                self.showing_delete_folder_confirmation = false;
                self.refresh_workspace_picker_to(
                    parent.unwrap_or_else(|| self.workspace_picker_root.clone()),
                    cx,
                );
                self.schedule_delete_folder_confirmation_removal(cx);
            }
            Err(error) => {
                self.status = format!("Failed to delete {}: {error}", path.display()).into();
            }
        }
        cx.notify();
    }

    pub(crate) fn cancel_workspace_dialog(&mut self, cx: &mut Context<Self>) {
        self.workspace_dialog = None;
        self.pending_delete_folder = None;
        self.showing_delete_folder_confirmation = false;
        self.status = "Workspace action cancelled.".into();
        cx.notify();
    }

    pub(crate) fn select_workspace_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let previous_index = self.workspace_state.active_index();
        if self.workspace_state.select(index) {
            self.previous_workspace_index = previous_index;
            self.showing_landing = false;
            let title = self
                .workspace_state
                .active_tab()
                .map(|tab| tab.title().to_owned())
                .unwrap_or_else(|| "Workspace".to_owned());
            self.status = format!("Workspace active: {title}").into();
        }
        cx.notify();
    }

    pub(crate) fn close_workspace_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some((workspace_id, title)) = self
            .workspace_state
            .tabs()
            .get(index)
            .map(|workspace| (workspace.id(), workspace.title().to_owned()))
        else {
            self.status = "Workspace tab is no longer available.".into();
            cx.notify();
            return;
        };

        if self.workspace_state.close(index).is_some() {
            self.remove_workspace_ui_state(workspace_id);
            self.previous_workspace_index = self.workspace_state.active_index();
            self.showing_landing = self.workspace_state.is_empty();
            self.status = format!("Closed Pi Workspace: {title}").into();
        }
        cx.notify();
    }

    pub(crate) fn select_bottom_dock_item(&mut self, index: usize, cx: &mut Context<Self>) {
        if index == 0 {
            self.toggle_settings_drawer(cx);
        }
    }

    pub(crate) fn set_snap_to_grid(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.snap_to_grid = enabled;
        cx.notify();
    }

    pub(crate) fn set_drawing_tools_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        self.drawing_tools_visible = visible;
        cx.notify();
    }

    pub(crate) fn select_drawing_tool(&mut self, tool: CanvasDrawingTool, cx: &mut Context<Self>) {
        self.commit_current_text_box_edit(cx);
        self.active_drawing_tool = if self.active_drawing_tool == tool {
            CanvasDrawingTool::Select
        } else {
            tool
        };
        self.drawing_tools_visible = true;
        cx.notify();
    }

    pub(crate) fn set_drawing_stroke_width(&mut self, width: f32, cx: &mut Context<Self>) {
        self.drawing_stroke_width = width.clamp(1.0, 16.0);
        cx.notify();
    }

    pub(crate) fn undo_canvas_drawing(&mut self, cx: &mut Context<Self>) {
        if self.workspace_state.undo_active_drawing() {
            cx.notify();
        }
    }

    pub(crate) fn redo_canvas_drawing(&mut self, cx: &mut Context<Self>) {
        if self.workspace_state.redo_active_drawing() {
            cx.notify();
        }
    }

    pub(super) fn refresh_workspace_picker(&mut self, cx: &mut Context<Self>) {
        let selected_path = self.selected_workspace_path(cx);
        self.refresh_workspace_picker_to(selected_path, cx);
    }

    pub(super) fn refresh_workspace_picker_to(
        &mut self,
        selected_path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let selected_path = std::fs::canonicalize(&selected_path).unwrap_or(selected_path);
        let items = picker::build_directory_tree_with_expanded_path(
            &self.workspace_picker_root,
            DEFAULT_DIRECTORY_DEPTH,
            &selected_path,
        );
        let selected_id = selected_path.to_string_lossy().to_string();
        self.workspace_tree.update(cx, |tree, cx| {
            tree.set_items(items, cx);
            if !tree.set_selected_id(&selected_id, cx) {
                tree.set_selected_index(Some(0), cx);
            }
        });
    }

    pub(super) fn hide_delete_folder_confirmation(&mut self, cx: &mut Context<Self>) {
        self.showing_delete_folder_confirmation = false;
        self.schedule_delete_folder_confirmation_removal(cx);
    }

    pub(super) fn schedule_delete_folder_confirmation_removal(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            Timer::after(NEW_FOLDER_ROW_ANIMATION).await;
            let _ = this.update(cx, |view, cx| {
                if !view.showing_delete_folder_confirmation {
                    view.pending_delete_folder = None;
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn selected_workspace_path(&self, cx: &mut Context<Self>) -> PathBuf {
        self.workspace_tree
            .read(cx)
            .selected_entry()
            .map(|entry| PathBuf::from(entry.item().id.to_string()))
            .unwrap_or_else(|| self.workspace_picker_root.clone())
    }

    pub(super) fn open_workspace_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path.is_dir() {
            self.status = format!("Not a folder: {}", path.display()).into();
            cx.notify();
            return;
        }

        let root = std::fs::canonicalize(&path).unwrap_or(path);
        self.workspace_state.add_folder(root.clone());
        self.workspace_dialog = None;
        self.showing_landing = false;
        self.status = format!("Opened Pi Workspace: {}", root.display()).into();
        cx.notify();
    }

    pub(super) fn providers(&self) -> Vec<ProviderAuthStatus> {
        match &self.data {
            Some(data) if !data.auth.is_empty() => data.auth.clone(),
            _ => ui::builtin_provider_auth_statuses(),
        }
    }

    pub(super) fn agent_ready(&self) -> bool {
        self.data
            .as_ref()
            .map(|data| data.agent_ready)
            .unwrap_or(false)
    }

    pub(super) fn runtime_state(&self) -> Option<&CoreStateSnapshot> {
        self.data.as_ref().and_then(|data| data.state.as_ref())
    }

    pub(super) fn can_resume_session(&self) -> bool {
        self.runtime_state()
            .and_then(|state| state.session_file.as_deref())
            .is_some_and(|path| !path.trim().is_empty())
    }

    pub(super) fn can_fork_session(&self) -> bool {
        self.runtime_state()
            .is_some_and(|state| latest_json_id(&state.messages).is_some())
    }
}
