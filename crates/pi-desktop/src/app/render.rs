use super::*;
use crate::components::package_settings::{PackageSettingsState, package_settings_content};
use crate::components::theme_settings::{ThemeSettingsState, theme_settings_content};
use gpui_component::PixelsExt as _;

impl Render for PiDesktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("PiDesktop");
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
            .child(self.render_bottom_dock(cx))
    }
}

impl PiDesktop {
    fn render_bottom_dock(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let props_changed = self.bottom_dock_view.update(cx, |view, cx| {
            view.sync(
                bottom_dock::BottomDockProps {
                    settings_selected: self.settings_drawer_open,
                    snap_to_grid: self.snap_to_grid,
                    drawing_tools_visible: self.drawing_tools_visible,
                },
                cx,
            )
        });
        let bottom_dock = AnyView::from(self.bottom_dock_view.clone());

        div().h(px(BOTTOM_DOCK_HEIGHT)).child(if props_changed {
            bottom_dock.into_any_element()
        } else {
            bottom_dock
                .cached(StyleRefinement::default().size_full())
                .into_any_element()
        })
    }

    fn render_status_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let props = status_bar::StatusBarProps {
            tabs: self
                .workspace_state
                .tabs()
                .iter()
                .map(|tab| workspace_tabs::WorkspaceTabInfo {
                    title: tab.title().to_owned(),
                })
                .collect(),
            active_index: self.workspace_state.active_index(),
            previous_index: self.previous_workspace_index,
        };
        let props_changed = self
            .status_bar_view
            .update(cx, |view, cx| view.sync(props, cx));
        let status_bar = AnyView::from(self.status_bar_view.clone());
        div().h(px(STATUS_BAR_HEIGHT)).child(if props_changed {
            status_bar.into_any_element()
        } else {
            status_bar
                .cached(StyleRefinement::default().size_full())
                .into_any_element()
        })
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
            .map(|tab| tab.canvas().nodes().to_vec())
            .unwrap_or_default();
        let active_canvas_zoom = self
            .workspace_state
            .active_tab()
            .map(|tab| tab.canvas().viewport().zoom)
            .unwrap_or(1.0);
        let (pinned_node_ids, focused_pinned_node_id) = self
            .workspace_state
            .active_tab()
            .map(|tab| {
                (
                    tab.pinned_layout()
                        .panels()
                        .iter()
                        .map(|panel| panel.node_id())
                        .collect::<HashSet<_>>(),
                    tab.pinned_layout().focused_node_id(),
                )
            })
            .unwrap_or_default();
        let live_node_ids = node_entries
            .iter()
            .map(|node| node.id())
            .collect::<HashSet<_>>();
        self.retain_workspace_node_ui_state(workspace_id, &live_node_ids);
        let mut chat_node_props_changed = false;
        for node in node_entries {
            let node_id = node.id();
            let title = node.title();
            let key = (workspace_id, node_id);
            let transcript = self
                .chat_transcripts
                .entry(key)
                .or_insert_with(|| cx.new(|_| ChatTranscript::default()))
                .clone();
            let body_view = self
                .chat_body_views
                .entry(key)
                .or_insert_with(|| {
                    cx.new(|cx| chat_node::ChatBodyView::new(workspace_id, node_id, transcript, cx))
                })
                .clone();

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
                        .default_value(title.clone())
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

            let Some(input) = self.chat_inputs.get(&key).cloned() else {
                continue;
            };
            let Some(title_input) = self.title_inputs.get(&key).cloned() else {
                continue;
            };
            let placement = if pinned_node_ids.contains(&node_id) {
                chat_node::ChatNodePlacement::Pinned {
                    focused: focused_pinned_node_id == Some(node_id),
                }
            } else {
                chat_node::ChatNodePlacement::Canvas
            };
            let props = chat_node::ChatNodeProps {
                workspace_id,
                node_id,
                title,
                pi_working: self.chat_node_working(key),
                input,
                title_input,
                body_view,
                editing_title: self.editing_title == Some(key),
                placement,
                scale: if matches!(placement, chat_node::ChatNodePlacement::Pinned { .. }) {
                    1.0
                } else {
                    active_canvas_zoom
                },
            };
            if let Some(view) = self.chat_node_views.get(&key).cloned() {
                chat_node_props_changed |= view.update(cx, |view, cx| view.sync(props, cx));
            } else {
                let app = cx.entity().clone();
                self.chat_node_views.insert(
                    key,
                    cx.new(|cx| chat_node::ChatNodeView::new(app, props, cx)),
                );
                chat_node_props_changed = true;
            }
        }
        if chat_node_props_changed {
            self.chat_node_render_revision = self.chat_node_render_revision.wrapping_add(1);
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

        let Some(tab) = self.workspace_state.active_tab().cloned() else {
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
        let active_text_box_inputs = self
            .text_box_inputs
            .iter()
            .filter_map(|(&(input_workspace_id, drawing_index), input)| {
                (input_workspace_id == workspace_id).then_some((drawing_index, input.clone()))
            })
            .collect::<HashMap<_, _>>();
        let active_chat_node_views = self
            .chat_node_views
            .iter()
            .filter_map(|(&(node_workspace_id, node_id), node_view)| {
                (node_workspace_id == workspace_id).then_some((node_id, node_view.clone()))
            })
            .collect::<HashMap<_, _>>();
        let canvas_props = workspace_canvas_view::WorkspaceCanvasProps {
            tab: tab.clone(),
            workspace_id,
            can_fork: self.can_fork_session(),
            can_resume: self.can_resume_session(),
            canvas_size: canvas_panel_size,
            text_box_inputs: active_text_box_inputs,
            chat_node_views: active_chat_node_views.clone(),
            chat_node_render_revision: self.chat_node_render_revision,
            snap_to_grid: self.snap_to_grid,
            drawing_tools_visible: self.drawing_tools_visible,
            active_drawing_tool: self.active_drawing_tool,
            drawing_stroke_width: self.drawing_stroke_width,
            can_undo_drawing: self.workspace_state.active_canvas_can_undo_drawing(),
            can_redo_drawing: self.workspace_state.active_canvas_can_redo_drawing(),
            drawing_stroke_slider: self.drawing_stroke_slider.clone(),
            focus_handle: self.focus_handle.clone(),
        };
        let (canvas_view, canvas_props_changed) =
            if let Some(view) = self.workspace_canvas_views.get(&workspace_id).cloned() {
                let changed = view.update(cx, |view, cx| view.sync(canvas_props, cx));
                (view, changed)
            } else {
                let app = cx.entity().clone();
                let view =
                    cx.new(|_| workspace_canvas_view::WorkspaceCanvasView::new(app, canvas_props));
                self.workspace_canvas_views
                    .insert(workspace_id, view.clone());
                (view, true)
            };
        let canvas_view = AnyView::from(canvas_view);
        let canvas = if canvas_props_changed {
            canvas_view.into_any_element()
        } else {
            canvas_view
                .cached(StyleRefinement::default().size_full())
                .into_any_element()
        };

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
                            &tab,
                            self.pin_panel_state.clone(),
                            &active_chat_node_views,
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
