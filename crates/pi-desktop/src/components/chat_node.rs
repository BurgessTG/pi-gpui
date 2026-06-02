use gpui::{
    AnyElement, AnyView, App, AppContext as _, Context, Entity, Hsla, InteractiveElement as _,
    IntoElement, ListAlignment, ListOffset, ListState, MouseButton, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled as _, Subscription,
    Window, div, list, prelude::FluentBuilder as _, px, svg,
};
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::tag::Tag;
use gpui_component::text::TextView;
use gpui_component::{Sizable as _, Size, StyledExt as _, h_flex, v_flex};

use crate::app::PiDesktop;
use crate::chat::transcript::{
    AssistantStatus, ChatEntry, ChatToolRun, ChatTranscript, ToolStatus,
};
use crate::components::chat_node_indicators::{LoadingBarView, WorkingInputOverlayView};
use crate::design::theme;
use crate::ui;

fn same_scale(left: f32, right: f32) -> bool {
    (left - right).abs() <= f32::EPSILON
}

pub struct ChatMessageView {
    workspace_id: usize,
    node_id: usize,
    index: usize,
    entry: ChatEntry,
    scale: f32,
}

impl ChatMessageView {
    fn new(
        workspace_id: usize,
        node_id: usize,
        index: usize,
        entry: ChatEntry,
        scale: f32,
    ) -> Self {
        Self {
            workspace_id,
            node_id,
            index,
            entry,
            scale,
        }
    }

    fn sync(&mut self, index: usize, entry: ChatEntry, scale: f32, cx: &mut Context<Self>) -> bool {
        if self.index == index && self.entry == entry && same_scale(self.scale, scale) {
            return false;
        }
        self.index = index;
        self.entry = entry;
        self.scale = scale;
        cx.notify();
        true
    }

    fn sync_scale(&mut self, scale: f32, cx: &mut Context<Self>) -> bool {
        if same_scale(self.scale, scale) {
            return false;
        }
        self.scale = scale;
        cx.notify();
        true
    }
}

impl Render for ChatMessageView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("ChatMessageView");
        render_transcript_entry(
            self.workspace_id,
            self.node_id,
            self.index,
            &self.entry,
            self.scale,
            window,
            &mut *cx,
        )
    }
}

pub struct ChatBodyView {
    workspace_id: usize,
    node_id: usize,
    transcript: Entity<ChatTranscript>,
    list_state: ListState,
    scroll_revision: u64,
    scale: f32,
    message_views: Vec<Entity<ChatMessageView>>,
    _transcript_subscription: Subscription,
}

impl ChatBodyView {
    pub fn new(
        workspace_id: usize,
        node_id: usize,
        transcript: Entity<ChatTranscript>,
        cx: &mut Context<Self>,
    ) -> Self {
        let entries = transcript.read(cx).entries().to_vec();
        let list_state = ListState::new(entries.len(), ListAlignment::Bottom, px(200.0));
        let message_views = entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                cx.new(|_| ChatMessageView::new(workspace_id, node_id, index, entry, 1.0))
            })
            .collect();
        let transcript_subscription = cx.observe(&transcript, |this, transcript, cx| {
            let entries = transcript.read(cx).entries().to_vec();
            this.sync_message_views(&entries, cx);
            cx.notify();
        });
        Self {
            workspace_id,
            node_id,
            transcript,
            list_state,
            scroll_revision: 0,
            scale: 1.0,
            message_views,
            _transcript_subscription: transcript_subscription,
        }
    }

    fn sync_message_views(&mut self, entries: &[ChatEntry], cx: &mut Context<Self>) {
        let old_len = self.message_views.len();
        let shared_len = old_len.min(entries.len());
        for (index, entry) in entries.iter().take(shared_len).cloned().enumerate() {
            if let Some(view) = self.message_views.get(index).cloned() {
                let scale = self.scale;
                let changed = view.update(cx, |view, cx| view.sync(index, entry, scale, cx));
                if changed {
                    self.list_state.splice(index..index + 1, 1);
                }
            }
        }
        if entries.len() < old_len {
            self.message_views.truncate(entries.len());
            self.list_state.splice(entries.len()..old_len, 0);
            return;
        }
        if entries.len() > old_len {
            for (index, entry) in entries.iter().cloned().enumerate().skip(old_len) {
                self.message_views.push(cx.new(|_| {
                    ChatMessageView::new(self.workspace_id, self.node_id, index, entry, self.scale)
                }));
            }
            self.list_state
                .splice(old_len..old_len, entries.len() - old_len);
        }
    }

    pub fn sync_scale(&mut self, scale: f32, cx: &mut Context<Self>) -> bool {
        if same_scale(self.scale, scale) {
            return false;
        }
        self.scale = scale;
        let item_count = self.message_views.len();
        for view in self.message_views.iter() {
            view.update(cx, |view, cx| view.sync_scale(scale, cx));
        }
        self.list_state.reset(item_count);
        cx.notify();
        true
    }
}

impl Render for ChatBodyView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("ChatBodyView");
        let transcript = self.transcript.read(cx);
        let entries_empty = transcript.entries().is_empty();
        let streaming = transcript.is_streaming();
        let revision = transcript.revision();
        let _ = transcript;
        render_body_contents(
            self.workspace_id,
            self.node_id,
            BodyRenderState {
                message_views: self.message_views.clone(),
                entries_empty,
                streaming,
                revision,
                scale: self.scale,
            },
            &self.list_state,
            &mut self.scroll_revision,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatNodePlacement {
    Canvas,
    Pinned { focused: bool },
}

#[derive(Clone)]
pub struct ChatNodeProps {
    pub workspace_id: usize,
    pub node_id: usize,
    pub title: String,
    pub pi_working: bool,
    pub input: Entity<InputState>,
    pub title_input: Entity<InputState>,
    pub body_view: Entity<ChatBodyView>,
    pub editing_title: bool,
    pub placement: ChatNodePlacement,
    pub scale: f32,
}

pub struct ChatNodeView {
    app: Entity<PiDesktop>,
    props: ChatNodeProps,
    loading_bar_view: Entity<LoadingBarView>,
    working_overlay_view: Entity<WorkingInputOverlayView>,
}

impl ChatNodeView {
    pub fn new(app: Entity<PiDesktop>, props: ChatNodeProps, cx: &mut Context<Self>) -> Self {
        let loading_bar_view = cx.new(|_| LoadingBarView::new(props.pi_working, props.scale));
        let working_overlay_view = cx.new(|_| WorkingInputOverlayView::new(props.scale));
        Self {
            app,
            props,
            loading_bar_view,
            working_overlay_view,
        }
    }

    pub fn sync(&mut self, props: ChatNodeProps, cx: &mut Context<Self>) -> bool {
        let changed = self.props.workspace_id != props.workspace_id
            || self.props.node_id != props.node_id
            || self.props.title != props.title
            || self.props.pi_working != props.pi_working
            || self.props.input != props.input
            || self.props.title_input != props.title_input
            || self.props.body_view != props.body_view
            || self.props.editing_title != props.editing_title
            || self.props.placement != props.placement
            || !same_scale(self.props.scale, props.scale);
        if !changed {
            return false;
        }
        self.props = props;
        cx.notify();
        true
    }
}

impl Render for ChatNodeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("ChatNodeView");
        let node_id = self.props.node_id;
        let workspace_id = self.props.workspace_id;
        let scale = self.props.scale;
        let pinned = matches!(self.props.placement, ChatNodePlacement::Pinned { .. });
        let focused = matches!(
            self.props.placement,
            ChatNodePlacement::Pinned { focused: true }
        );
        let app = self.app.clone();
        let show_working_text =
            self.props.pi_working && self.props.input.read(cx).value().trim().is_empty();
        self.loading_bar_view
            .update(cx, |view, cx| view.sync(self.props.pi_working, scale, cx));
        self.working_overlay_view
            .update(cx, |view, cx| view.sync_scale(scale, cx));
        self.props
            .body_view
            .update(cx, |view, cx| view.sync_scale(scale, cx));

        div()
            .id(SharedString::from(if pinned {
                format!("pinned-chat-node-{workspace_id}-{node_id}")
            } else {
                format!("chat-node-{workspace_id}-{node_id}")
            }))
            .size_full()
            .min_w_0()
            .min_h_0()
            .border_1()
            .border_color(if focused {
                theme::accent()
            } else {
                theme::hairline()
            })
            .bg(theme::surface())
            .text_color(theme::text())
            .text_size(scaled_px(14.0, scale))
            .flex()
            .flex_col()
            .on_mouse_down(
                MouseButton::Left,
                app_listener(
                    app.clone(),
                    move |view, event: &gpui::MouseDownEvent, _window, cx| {
                        if pinned {
                            view.focus_pinned_node(workspace_id, node_id, cx);
                        } else {
                            view.start_node_drag(
                                node_id,
                                super::workspace_canvas::screen_point_from_event(event),
                                cx,
                            );
                            cx.stop_propagation();
                        }
                    },
                ),
            )
            .on_mouse_move(app_listener(
                app.clone(),
                move |view, event: &gpui::MouseMoveEvent, window, cx| {
                    if !pinned && event.dragging() {
                        view.update_canvas_pan(
                            super::workspace_canvas::screen_point_from_mouse_move(event),
                            window,
                            cx,
                        );
                        cx.stop_propagation();
                    }
                },
            ))
            .on_mouse_up(
                MouseButton::Left,
                app_listener(
                    app.clone(),
                    move |view, _event: &gpui::MouseUpEvent, _window, cx| {
                        if !pinned {
                            view.end_canvas_pan(cx);
                        }
                    },
                ),
            )
            .child(render_header(
                app.clone(),
                workspace_id,
                node_id,
                &self.props.title,
                self.props.title_input.clone(),
                self.props.editing_title,
                pinned,
                scale,
            ))
            .child(self.loading_bar_view.clone())
            .child(render_body_view(
                app.clone(),
                workspace_id,
                node_id,
                self.props.body_view.clone(),
            ))
            .child(render_composer(
                app.clone(),
                workspace_id,
                node_id,
                self.props.input.clone(),
                show_working_text.then_some(self.working_overlay_view.clone()),
                scale,
            ))
            .when(!pinned, |this| {
                this.child(render_resize_handle(app, node_id, scale))
            })
    }
}

fn app_listener<E: ?Sized>(
    app: Entity<PiDesktop>,
    f: impl Fn(&mut PiDesktop, &E, &mut Window, &mut Context<PiDesktop>) + 'static,
) -> impl Fn(&E, &mut Window, &mut App) + 'static {
    move |event, window, cx| {
        app.update(cx, |view, cx| f(view, event, window, cx));
    }
}

#[allow(clippy::too_many_arguments)]
fn render_header(
    app: Entity<PiDesktop>,
    workspace_id: usize,
    node_id: usize,
    title: &str,
    title_input: Entity<InputState>,
    editing_title: bool,
    pinned: bool,
    scale: f32,
) -> AnyElement {
    h_flex()
        .h(scaled_px(46.0, scale))
        .px(scaled_px(16.0, scale))
        .border_b_1()
        .border_color(theme::hairline())
        .items_center()
        .justify_between()
        .child(
            h_flex()
                .min_w_0()
                .flex_1()
                .items_center()
                .gap(scaled_px(10.0, scale))
                .child(render_title(
                    app.clone(),
                    workspace_id,
                    node_id,
                    title.to_owned(),
                    title_input,
                    editing_title,
                    scale,
                )),
        )
        .child(
            h_flex()
                .flex_none()
                .items_center()
                .gap(scaled_px(4.0, scale))
                .child(pin_toggle(
                    app.clone(),
                    workspace_id,
                    node_id,
                    pinned,
                    scale,
                ))
                .child(close_button(app, workspace_id, node_id, scale)),
        )
        .into_any_element()
}

fn render_title(
    app: Entity<PiDesktop>,
    workspace_id: usize,
    node_id: usize,
    title: String,
    input: Entity<InputState>,
    editing: bool,
    scale: f32,
) -> AnyElement {
    if editing {
        return div()
            .id(SharedString::from(format!(
                "chat-node-title-edit-{workspace_id}-{node_id}"
            )))
            .h(scaled_px(28.0, scale))
            .flex_1()
            .min_w_0()
            .border_1()
            .border_color(theme::hairline())
            .bg(theme::app_bg())
            .text_size(scaled_px(14.0, scale))
            .on_mouse_down(
                MouseButton::Left,
                app_listener(
                    app.clone(),
                    move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                        view.focus_pinned_node(workspace_id, node_id, cx);
                        cx.stop_propagation();
                    },
                ),
            )
            .child(
                Input::new(&input)
                    .appearance(false)
                    .h(scaled_px(28.0, scale)),
            )
            .into_any_element();
    }

    div()
        .id(SharedString::from(format!(
            "chat-node-title-{workspace_id}-{node_id}"
        )))
        .flex_1()
        .min_w_0()
        .truncate()
        .cursor_pointer()
        .text_size(scaled_px(14.0, scale))
        .font_semibold()
        .text_color(theme::text())
        .on_mouse_down(
            MouseButton::Left,
            app_listener(
                app.clone(),
                move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                    view.focus_pinned_node(workspace_id, node_id, cx);
                    cx.stop_propagation();
                },
            ),
        )
        .on_click(app_listener(app, move |view, _, window, cx| {
            view.start_session_title_edit(workspace_id, node_id, window, cx);
        }))
        .child(title)
        .into_any_element()
}

fn render_body_view(
    app: Entity<PiDesktop>,
    workspace_id: usize,
    node_id: usize,
    body_view: Entity<ChatBodyView>,
) -> AnyElement {
    div()
        .relative()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .bg(theme::surface())
        .on_mouse_down(
            MouseButton::Left,
            app_listener(
                app,
                move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                    view.focus_pinned_node(workspace_id, node_id, cx);
                    cx.stop_propagation();
                },
            ),
        )
        .child(AnyView::from(body_view).cached(StyleRefinement::default().size_full()))
        .into_any_element()
}

struct BodyRenderState {
    message_views: Vec<Entity<ChatMessageView>>,
    entries_empty: bool,
    streaming: bool,
    revision: u64,
    scale: f32,
}

fn render_body_contents(
    workspace_id: usize,
    node_id: usize,
    body: BodyRenderState,
    list_state: &ListState,
    scroll_revision: &mut u64,
) -> AnyElement {
    if body.streaming && *scroll_revision != body.revision {
        list_state.scroll_to(ListOffset {
            item_ix: body.message_views.len(),
            offset_in_item: px(0.0),
        });
        *scroll_revision = body.revision;
    }

    let message_views = body.message_views;
    let _item_count = message_views.len();

    div()
        .id(SharedString::from(format!(
            "chat-node-scroll-area-{workspace_id}-{node_id}"
        )))
        .size_full()
        .overflow_hidden()
        .when(body.entries_empty, |this| {
            this.child(
                div()
                    .size_full()
                    .px(scaled_px(16.0, body.scale))
                    .py(scaled_px(14.0, body.scale))
                    .child(render_empty_body(body.scale)),
            )
        })
        .when(!body.entries_empty, |this| {
            this.child(
                list(list_state.clone(), move |index, _window, _cx| {
                    let view = message_views[index].clone();
                    div()
                        .px(scaled_px(16.0, body.scale))
                        .pt(if index == 0 {
                            scaled_px(14.0, body.scale)
                        } else {
                            px(0.0)
                        })
                        .pb(scaled_px(14.0, body.scale))
                        .child(view)
                        .into_any_element()
                })
                .size_full(),
            )
            .vertical_scrollbar(list_state)
        })
        .into_any_element()
}

fn render_empty_body(scale: f32) -> AnyElement {
    v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(scaled_px(8.0, scale))
        .text_center()
        .text_size(scaled_px(13.0, scale))
        .text_color(theme::text_muted())
        .child("No Pi messages yet")
        .child(
            div()
                .max_w(scaled_px(280.0, scale))
                .child("Type below to stream a Pi response with tools shown inline."),
        )
        .into_any_element()
}

fn render_transcript_entry(
    workspace_id: usize,
    node_id: usize,
    index: usize,
    entry: &ChatEntry,
    scale: f32,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    match entry {
        ChatEntry::User(text) => render_user_message(text, scale),
        ChatEntry::Assistant { text, status } => render_assistant_message(
            workspace_id,
            node_id,
            index,
            text,
            status,
            scale,
            window,
            cx,
        ),
        ChatEntry::Tool(tool) => render_tool_run(tool, scale),
    }
}

fn render_user_message(text: &str, scale: f32) -> AnyElement {
    div()
        .w_full()
        .flex()
        .justify_end()
        .child(
            v_flex()
                .gap(scaled_px(6.0, scale))
                .max_w(scaled_px(420.0, scale))
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::app_bg())
                .px(scaled_px(12.0, scale))
                .py(scaled_px(10.0, scale))
                .child(
                    div()
                        .text_size(scaled_px(14.0, scale))
                        .text_color(theme::text())
                        .child(text.to_owned()),
                ),
        )
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_assistant_message(
    workspace_id: usize,
    node_id: usize,
    index: usize,
    text: &str,
    status: &AssistantStatus,
    scale: f32,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let detail = assistant_error_detail(status);
    let content = if text.trim().is_empty() { "" } else { text };

    v_flex()
        .w_full()
        .gap(scaled_px(8.0, scale))
        .child(
            TextView::markdown(
                SharedString::from(format!(
                    "chat-node-{workspace_id}-{node_id}-assistant-{index}"
                )),
                content.to_owned(),
                window,
                cx,
            )
            .selectable(true)
            .text_size(scaled_px(14.0, scale))
            .text_color(theme::text()),
        )
        .when_some(detail, |this, detail| {
            this.child(
                div()
                    .border_1()
                    .border_color(theme::danger())
                    .bg(theme::danger_soft())
                    .px(scaled_px(10.0, scale))
                    .py(scaled_px(8.0, scale))
                    .text_size(scaled_px(12.0, scale))
                    .text_color(theme::text())
                    .child(detail),
            )
        })
        .into_any_element()
}

fn render_tool_run(tool: &ChatToolRun, scale: f32) -> AnyElement {
    let (label, color) = tool_status_parts(tool.status);
    v_flex()
        .gap(scaled_px(8.0, scale))
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::app_bg())
        .px(scaled_px(10.0, scale))
        .py(scaled_px(9.0, scale))
        .child(
            h_flex()
                .items_center()
                .gap(scaled_px(6.0, scale))
                .child(role_tag("TOOL", theme::surface(), theme::text(), scale))
                .child(role_tag(label, color, theme::app_bg(), scale))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .font_semibold()
                        .text_size(scaled_px(13.0, scale))
                        .text_color(theme::text())
                        .child(tool.name.clone()),
                ),
        )
        .when(!tool.arguments.trim().is_empty(), |this| {
            this.child(tool_detail_block("args", &tool.arguments, scale))
        })
        .when_some(tool.output.as_deref(), |this, output| {
            this.child(tool_detail_block("output", output, scale))
        })
        .into_any_element()
}

fn tool_detail_block(label: &'static str, text: &str, scale: f32) -> AnyElement {
    v_flex()
        .gap(scaled_px(4.0, scale))
        .child(
            div()
                .text_size(scaled_px(10.0, scale))
                .text_color(theme::text_muted())
                .child(label),
        )
        .child(
            div()
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::surface())
                .px(scaled_px(8.0, scale))
                .py(scaled_px(6.0, scale))
                .text_size(scaled_px(12.0, scale))
                .text_color(theme::text_muted())
                .child(text.to_owned()),
        )
        .into_any_element()
}

fn role_tag(label: &'static str, color: Hsla, foreground: Hsla, scale: f32) -> AnyElement {
    Tag::custom(color, foreground, theme::hairline())
        .rounded(px(0.0))
        .with_size(Size::Small)
        .text_size(scaled_px(10.0, scale))
        .child(label)
        .into_any_element()
}

fn assistant_error_detail(status: &AssistantStatus) -> Option<String> {
    match status {
        AssistantStatus::Error(message) | AssistantStatus::Aborted(message) => {
            Some(message.clone())
        }
        AssistantStatus::Streaming | AssistantStatus::Complete => None,
    }
}

fn tool_status_parts(status: ToolStatus) -> (&'static str, Hsla) {
    match status {
        ToolStatus::Pending => ("QUEUED", theme::complement()),
        ToolStatus::Running => ("RUN", theme::accent()),
        ToolStatus::Complete => ("DONE", theme::success()),
        ToolStatus::Error => ("ERROR", theme::danger()),
    }
}

fn render_composer(
    app: Entity<PiDesktop>,
    workspace_id: usize,
    node_id: usize,
    input: Entity<InputState>,
    working_overlay_view: Option<Entity<WorkingInputOverlayView>>,
    scale: f32,
) -> AnyElement {
    h_flex()
        .mx(scaled_px(16.0, scale))
        .mb(scaled_px(16.0, scale))
        .h(scaled_px(52.0, scale))
        .items_center()
        .gap(scaled_px(8.0, scale))
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::app_bg())
        .text_size(scaled_px(14.0, scale))
        .on_mouse_down(
            MouseButton::Left,
            app_listener(
                app.clone(),
                move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                    view.focus_pinned_node(workspace_id, node_id, cx);
                    cx.stop_propagation();
                },
            ),
        )
        .child(
            div()
                .relative()
                .flex_1()
                .min_w_0()
                .child(Input::new(&input).appearance(false).h_full())
                .when_some(working_overlay_view, |this, view| this.child(view)),
        )
        .child(
            div()
                .id(SharedString::from(format!(
                    "chat-node-send-{workspace_id}-{node_id}"
                )))
                .flex_none()
                .w(scaled_px(40.0, scale))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_size(scaled_px(18.0, scale))
                .text_color(theme::text())
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(app_listener(app, move |view, _, window, cx| {
                    view.submit_chat_node(workspace_id, node_id, window, cx);
                }))
                .child("→"),
        )
        .into_any_element()
}

fn pin_toggle(
    app: Entity<PiDesktop>,
    workspace_id: usize,
    node_id: usize,
    pinned: bool,
    scale: f32,
) -> AnyElement {
    div()
        .id(SharedString::from(format!(
            "pin-node-toggle-{workspace_id}-{node_id}"
        )))
        .flex_none()
        .h(scaled_px(22.0, scale))
        .px(scaled_px(8.0, scale))
        .flex()
        .items_center()
        .border_1()
        .border_color(if pinned {
            theme::complement()
        } else {
            gpui::transparent_black()
        })
        .bg(if pinned {
            theme::complement().opacity(0.12)
        } else {
            gpui::transparent_black()
        })
        .cursor_pointer()
        .text_size(scaled_px(12.0, scale))
        .text_color(theme::text())
        .hover(|style| style.bg(theme::surface_hover()))
        .on_mouse_down(
            MouseButton::Left,
            app_listener(
                app.clone(),
                move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                    view.focus_pinned_node(workspace_id, node_id, cx);
                    cx.stop_propagation();
                },
            ),
        )
        .on_click(app_listener(app, move |view, _, _window, cx| {
            view.toggle_session_node_pin(workspace_id, node_id, cx);
        }))
        .child(
            svg()
                .path(ui::pin_icon_path())
                .size(scaled_px(14.0, scale))
                .text_color(if pinned {
                    theme::complement()
                } else {
                    theme::text()
                }),
        )
        .into_any_element()
}

fn close_button(
    app: Entity<PiDesktop>,
    workspace_id: usize,
    node_id: usize,
    scale: f32,
) -> AnyElement {
    div()
        .id(SharedString::from(format!(
            "close-chat-node-{workspace_id}-{node_id}"
        )))
        .flex_none()
        .h(scaled_px(22.0, scale))
        .w(scaled_px(22.0, scale))
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(gpui::transparent_black())
        .cursor_pointer()
        .text_size(scaled_px(15.0, scale))
        .text_color(theme::text())
        .hover(|style| style.bg(theme::surface_hover()))
        .on_mouse_down(
            MouseButton::Left,
            app_listener(
                app.clone(),
                move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                    view.focus_pinned_node(workspace_id, node_id, cx);
                    cx.stop_propagation();
                },
            ),
        )
        .on_click(app_listener(app, move |view, _, _window, cx| {
            view.close_session_node(workspace_id, node_id, cx);
        }))
        .child("×")
        .into_any_element()
}

fn render_resize_handle(app: Entity<PiDesktop>, node_id: usize, scale: f32) -> AnyElement {
    div()
        .id(SharedString::from(format!("resize-chat-node-{node_id}")))
        .absolute()
        .right_0()
        .bottom_0()
        .w(scaled_px(22.0, scale))
        .h(scaled_px(22.0, scale))
        .cursor_pointer()
        .opacity(0.0)
        .hover(|style| style.opacity(1.0))
        .flex()
        .items_end()
        .justify_end()
        .pr(scaled_px(3.0, scale))
        .pb(scaled_px(3.0, scale))
        .on_mouse_down(
            MouseButton::Left,
            app_listener(
                app,
                move |view, event: &gpui::MouseDownEvent, _window, cx| {
                    view.start_node_resize(
                        node_id,
                        super::workspace_canvas::screen_point_from_event(event),
                        cx,
                    );
                    cx.stop_propagation();
                },
            ),
        )
        .child(
            div()
                .w(scaled_px(10.0, scale))
                .h(scaled_px(10.0, scale))
                .border_r_1()
                .border_b_1()
                .border_color(theme::text()),
        )
        .into_any_element()
}

fn scaled_px(value: f32, scale: f32) -> gpui::Pixels {
    px(value * scale)
}
