use super::*;
use crate::components::package_settings::{package_settings_content, PackageSettingsState};
use crate::components::theme_settings::{theme_settings_content, ThemeSettingsState};
use gpui_component::PixelsExt as _;

impl Render for PiDesktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let canvas_size = workspace_canvas_size(window);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::app_bg())
            .text_color(theme::text())
            .track_focus(&self.focus_handle)
            .child(self.render_status_bar(cx))
            .child(
                div()
                    .flex_1()
                    .relative()
                    .overflow_hidden()
                    .child(self.render_background_grid())
                    .child(self.render_workspace_stage(canvas_size, window, cx))
                    .when(self.settings_drawer_visible, |this| {
                        this.child(self.render_settings_backdrop(cx))
                            .child(self.render_settings_drawer(cx))
                    })
                    .when_some(self.workspace_dialog, |this, dialog| {
                        this.child(file_picker::workspace_dialog_backdrop(cx))
                            .child(self.render_workspace_dialog(dialog, cx))
                    }),
            )
            .child(bottom_dock::bottom_dock(
                self.settings_drawer_open,
                self.snap_to_grid,
                self.drawing_tools_visible,
                cx,
            ))
    }
}

impl PiDesktop {
    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(28.0))
            .px_2()
            .child(
                div()
                    .id("home-logo")
                    .p_1()
                    .cursor_pointer()
                    .hover(|style| style.bg(theme::surface_hover()))
                    .on_click(cx.listener(|view, _, _, cx| view.open_landing(cx)))
                    .child(
                        svg()
                            .path(ui::logo_path())
                            .size_4()
                            .text_color(theme::text()),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(workspace_tabs::workspace_tabs(
                        &self.workspace_state,
                        self.previous_workspace_index,
                        cx,
                    )),
            )
            .child(div().w(px(24.0)))
    }

    fn render_workspace_stage(
        &mut self,
        canvas_size: WorldSize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.showing_landing || self.workspace_state.is_empty() {
            return self.render_landing(cx).into_any_element();
        }

        self.render_workspace_content(canvas_size, window, cx)
    }

    fn render_landing(&self, cx: &mut Context<Self>) -> impl IntoElement {
        workspace_launcher::workspace_launcher(cx)
    }

    fn render_workspace_content(
        &mut self,
        canvas_size: WorldSize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(workspace_index) = self.workspace_state.active_index() else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme::text())
                .child("No workspace selected")
                .into_any_element();
        };
        let Some(workspace_id) = self.workspace_id_for_index(workspace_index) else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme::text())
                .child("No workspace selected")
                .into_any_element();
        };
        let node_entries = self
            .workspace_state
            .active_tab()
            .map(|tab| {
                tab.canvas()
                    .nodes()
                    .iter()
                    .map(|node| (node.id(), node.title()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let live_node_ids = node_entries
            .iter()
            .map(|(node_id, _)| *node_id)
            .collect::<HashSet<_>>();
        self.retain_workspace_node_ui_state(workspace_id, &live_node_ids);
        for (node_id, title) in node_entries {
            let key = (workspace_id, node_id);
            let transcript = self
                .chat_transcripts
                .entry(key)
                .or_insert_with(|| cx.new(|_| ChatTranscript::default()))
                .clone();
            self.chat_body_views.entry(key).or_insert_with(|| {
                cx.new(|cx| chat_node::ChatBodyView::new(workspace_id, node_id, transcript, cx))
            });

            if let std::collections::hash_map::Entry::Vacant(e) = self.chat_inputs.entry(key) {
                let chat_input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder("Ask Pi…")
                        .auto_grow(1, 3)
                });
                let chat_subscription = cx.subscribe_in(
                    &chat_input,
                    window,
                    move |view, input, event, window, cx| {
                        if matches!(event, InputEvent::PressEnter { secondary: false }) {
                            view.submit_chat_node_from_enter(
                                workspace_id,
                                node_id,
                                input,
                                window,
                                cx,
                            );
                        }
                    },
                );
                e.insert(chat_input);
                self.chat_input_subscriptions.insert(key, chat_subscription);
            }

            if let Some(input) = self.title_inputs.get(&key).cloned() {
                if self.editing_title != Some(key) && input.read(cx).value() != title {
                    input.update(cx, |input, cx| input.set_value(title.clone(), window, cx));
                }
            } else {
                let title_input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder("Session name")
                        .default_value(title)
                });
                let title_subscription = cx.subscribe_in(
                    &title_input,
                    window,
                    move |view, _input, event, window, cx| {
                        if matches!(
                            event,
                            InputEvent::PressEnter { secondary: false } | InputEvent::Blur
                        ) {
                            view.commit_session_title_edit(workspace_id, node_id, window, cx);
                        }
                    },
                );
                self.title_inputs.insert(key, title_input);
                self.title_input_subscriptions
                    .insert(key, title_subscription);
            }
        }

        let text_box_entries = self
            .workspace_state
            .active_tab()
            .map(|tab| {
                tab.canvas()
                    .drawings()
                    .iter()
                    .enumerate()
                    .filter_map(|(drawing_index, drawing)| {
                        let CanvasDrawing::TextBox { text, .. } = drawing else {
                            return None;
                        };
                        Some((drawing_index, text.clone()))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let live_text_box_indices = text_box_entries
            .iter()
            .map(|(drawing_index, _)| *drawing_index)
            .collect::<HashSet<_>>();
        self.retain_workspace_text_box_ui_state(workspace_id, &live_text_box_indices);
        for (drawing_index, text) in text_box_entries {
            let key = (workspace_id, drawing_index);
            if let Some(input) = self.text_box_inputs.get(&key).cloned() {
                if self.editing_text_box != Some(key) && input.read(cx).value() != text {
                    input.update(cx, |input, cx| input.set_value(text.clone(), window, cx));
                }
            } else {
                let text_input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder("")
                        .default_value(text)
                        .auto_grow(1, 12)
                });
                let text_subscription = cx.subscribe_in(
                    &text_input,
                    window,
                    move |view, input, event, window, cx| match event {
                        InputEvent::Focus => {
                            view.start_text_box_edit(workspace_id, drawing_index, cx);
                        }
                        InputEvent::Blur => {
                            view.commit_text_box_edit(workspace_id, drawing_index, input, cx);
                        }
                        InputEvent::PressEnter { secondary: true } => {
                            view.commit_text_box_edit_from_secondary_enter(
                                workspace_id,
                                drawing_index,
                                input,
                                window,
                                cx,
                            );
                        }
                        InputEvent::Change | InputEvent::PressEnter { secondary: false } => {}
                    },
                );
                self.text_box_inputs.insert(key, text_input);
                self.text_box_input_subscriptions
                    .insert(key, text_subscription);
            }
        }

        let Some(tab) = self.workspace_state.active_tab() else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme::text())
                .child("No workspace selected")
                .into_any_element();
        };

        let pinned = !tab.pinned_layout().is_empty();
        let canvas_panel_size = if pinned {
            self.pinned_canvas_size(canvas_size, cx)
        } else {
            canvas_size
        };
        let canvas = workspace_canvas::workspace_canvas(
            tab,
            workspace_id,
            self.can_fork_session(),
            self.can_resume_session(),
            self.streaming_node,
            canvas_panel_size,
            &self.chat_inputs,
            &self.title_inputs,
            &self.text_box_inputs,
            &self.chat_body_views,
            self.editing_title,
            self.snap_to_grid,
            self.drawing_tools_visible,
            self.active_drawing_tool,
            self.drawing_stroke_width,
            self.workspace_state.active_canvas_can_undo_drawing(),
            self.workspace_state.active_canvas_can_redo_drawing(),
            self.drawing_stroke_slider.clone(),
            self.focus_handle.clone(),
            window,
            cx,
        );

        let content = if pinned {
            let pinned_canvas = div()
                .size_full()
                .border_2()
                .border_color(theme::complement())
                .bg(theme::app_bg())
                .child(canvas);
            h_resizable("workspace-pin-shell")
                .with_state(&self.pin_shell_state)
                .child(
                    resizable_panel()
                        .size(px((canvas_size.width * 0.5).max(320.0)))
                        .child(pinned_canvas),
                )
                .child(
                    resizable_panel()
                        .size(px((canvas_size.width * 0.5).max(320.0)))
                        .child(pinned_panels::pinned_panel_region(
                            tab,
                            workspace_id,
                            self.streaming_node,
                            self.pin_panel_state.clone(),
                            &self.chat_inputs,
                            &self.title_inputs,
                            &self.chat_body_views,
                            self.editing_title,
                            window,
                            cx,
                        )),
                )
                .into_any_element()
        } else {
            canvas
        };

        div()
            .id("workspace-stage-content")
            .key_context(pinned_actions::WORKSPACE_KEY_CONTEXT)
            .on_action(
                cx.listener(|view, _: &pinned_actions::SwapPinnedPanel, _window, cx| {
                    view.swap_focused_pinned_panel(cx);
                }),
            )
            .on_action(cx.listener(
                |view, _: &pinned_actions::TogglePinnedPanelAxis, _window, cx| {
                    view.toggle_focused_pinned_panel_axis(cx);
                },
            ))
            .size_full()
            .child(content)
            .into_any_element()
    }

    fn pinned_canvas_size(&self, canvas_size: WorldSize, cx: &mut Context<Self>) -> WorldSize {
        let shell_width = self
            .pin_shell_state
            .read(cx)
            .sizes()
            .first()
            .map(|size| size.as_f32())
            .filter(|width| *width >= 240.0)
            .unwrap_or(canvas_size.width * 0.5);
        let max_width = canvas_size.width.max(1.0);
        let min_width = 240.0_f32.min(max_width);
        WorldSize::new(shell_width.clamp(min_width, max_width), canvas_size.height)
    }

    fn render_workspace_dialog(
        &self,
        dialog: WorkspaceDialog,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match dialog {
            WorkspaceDialog::OpenWorkspace => {
                let selected_path = self.selected_workspace_path(cx);
                file_picker::open_workspace_dialog(
                    &self.workspace_tree,
                    &self.workspace_picker_root,
                    &selected_path,
                    &self.new_folder_name_input,
                    self.new_folder_input_visible,
                    self.showing_new_folder_input,
                    self.pending_delete_folder.as_deref(),
                    self.showing_delete_folder_confirmation,
                    cx,
                )
            }
        }
    }

    fn render_background_grid(&self) -> impl IntoElement {
        canvas(
            |_, _, _| (),
            |bounds, _, window, _| {
                paint_background_grid_axis(bounds, true, window);
                paint_background_grid_axis(bounds, false, window);
            },
        )
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .left_0()
    }

    fn render_settings_backdrop(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let opening = self.settings_drawer_open;
        let animation_id = if opening {
            "settings-backdrop-open"
        } else {
            "settings-backdrop-close"
        };
        let close_view = cx.entity().clone();

        div()
            .id("settings-backdrop")
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .left_0()
            .bg(gpui::black())
            .opacity(if opening { 0.52 } else { 0.0 })
            .on_click(move |_, _, cx| {
                close_view.update(cx, |view, cx| view.close_settings_drawer(cx));
            })
            .with_animation(animation_id, drawer_animation(), move |this, delta| {
                let opacity = if opening { delta } else { 1.0 - delta } * 0.52;
                this.opacity(opacity)
            })
    }

    fn render_settings_drawer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let opening = self.settings_drawer_open;
        let animation_id = if opening {
            "settings-drawer-open"
        } else {
            "settings-drawer-close"
        };

        div()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(DRAWER_WIDTH))
            .occlude()
            .border_l_1()
            .border_color(theme::hairline())
            .bg(theme::surface())
            .child(self.render_settings_component(cx))
            .with_animation(animation_id, drawer_animation(), move |this, delta| {
                let closed_offset = px(-DRAWER_WIDTH);
                let offset = if opening {
                    closed_offset * (1.0 - delta)
                } else {
                    closed_offset * delta
                };
                this.right(offset)
            })
    }

    fn render_settings_component(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let providers = self.providers();
        let selected_provider = self.selected_provider.clone();
        let hover_card_provider = self.hover_card_provider.clone();
        let auth_flow = self.auth_flow;
        let api_key_input = self.api_key_input.clone();
        let pending = self.pending;
        let appearance = self.appearance;
        let package_results = self.package_results.clone();
        let installed_packages = self.installed_packages();
        let package_search_input = self.package_search_input.clone();
        let installed_packages_table = self.installed_packages_table.clone();
        let package_pending = self.package_pending;
        let installing_package = self.installing_package.clone();
        let removing_package = self.removing_package.clone();
        let new_installed_package = self.new_installed_package.clone();
        let view = cx.entity().clone();
        let package_view = view.clone();
        let theme_view = view.clone();

        Settings::new("pi-settings")
            .sidebar_width(px(148.0))
            .search_visible(false)
            .with_size(Size::Small)
            .with_group_variant(gpui_component::group_box::GroupBoxVariant::Normal)
            .pages(vec![
                SettingPage::new("Auth")
                    .default_open(true)
                    .resettable(false)
                    .group(
                        SettingGroup::new().item(SettingItem::render(move |_, _, cx| {
                            auth_settings_content(
                                AuthSettingsState {
                                    providers: providers.clone(),
                                    selected_provider: selected_provider.clone(),
                                    hover_card_provider: hover_card_provider.clone(),
                                    auth_flow,
                                    pending,
                                },
                                api_key_input.clone(),
                                view.clone(),
                                cx,
                            )
                        })),
                    ),
                SettingPage::new("Packages").resettable(false).group(
                    SettingGroup::new()
                        .title("Packages")
                        .item(SettingItem::render(move |_, _, cx| {
                            package_settings_content(
                                PackageSettingsState {
                                    results: package_results.clone(),
                                    installed: installed_packages.clone(),
                                    installing_source: installing_package.clone(),
                                    removing_source: removing_package.clone(),
                                    new_installed_source: new_installed_package.clone(),
                                    pending: package_pending,
                                },
                                package_search_input.clone(),
                                installed_packages_table.clone(),
                                package_view.clone(),
                                cx,
                            )
                        })),
                ),
                SettingPage::new("Theme").resettable(false).group(
                    SettingGroup::new().title("Theme").item(SettingItem::render(
                        move |_, _, cx| {
                            theme_settings_content(
                                ThemeSettingsState { appearance },
                                theme_view.clone(),
                                cx,
                            )
                        },
                    )),
                ),
                SettingPage::new("Advanced").resettable(false).group(
                    SettingGroup::new()
                        .title("Advanced")
                        .item(SettingItem::render(|_, _, _| {
                            settings_placeholder(
                                "Runtime, workspace, SDK, and debugging controls will live here.",
                            )
                        })),
                ),
            ])
    }
}
