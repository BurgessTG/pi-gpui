use std::time::Duration;

use gpui::{
    div, prelude::FluentBuilder as _, px, svg, Animation, AnimationExt as _, AnyElement, AnyView,
    App, AppContext as _, Context, Entity, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Render, ScrollHandle, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled as _, Subscription, Window,
};
use gpui_component::animation::cubic_bezier;
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::tag::Tag;
use gpui_component::text::TextView;
use gpui_component::{h_flex, v_flex, Sizable as _, Size, StyledExt as _};

use crate::app::PiDesktop;
use crate::chat::transcript::{
    AssistantStatus, ChatEntry, ChatToolRun, ChatTranscript, ToolStatus,
};
use crate::design::theme;
use crate::ui;
use crate::workspace::canvas::{SessionNode, WorldPoint};

pub struct ChatMessageView {
    workspace_id: usize,
    node_id: usize,
    index: usize,
    entry: ChatEntry,
}

impl ChatMessageView {
    fn new(workspace_id: usize, node_id: usize, index: usize, entry: ChatEntry) -> Self {
        Self {
            workspace_id,
            node_id,
            index,
            entry,
        }
    }

    fn sync(&mut self, index: usize, entry: ChatEntry, cx: &mut Context<Self>) {
        if self.index == index && self.entry == entry {
            return;
        }
        self.index = index;
        self.entry = entry;
        cx.notify();
    }
}

impl Render for ChatMessageView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_transcript_entry(
            self.workspace_id,
            self.node_id,
            self.index,
            &self.entry,
            1.0,
            window,
            &mut *cx,
        )
    }
}

pub struct ChatBodyView {
    workspace_id: usize,
    node_id: usize,
    transcript: Entity<ChatTranscript>,
    scroll_handle: ScrollHandle,
    scroll_revision: u64,
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
        let message_views = entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                cx.new(|_| ChatMessageView::new(workspace_id, node_id, index, entry))
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
            scroll_handle: ScrollHandle::default(),
            scroll_revision: 0,
            message_views,
            _transcript_subscription: transcript_subscription,
        }
    }

    fn sync_message_views(&mut self, entries: &[ChatEntry], cx: &mut Context<Self>) {
        self.message_views.truncate(entries.len());
        for (index, entry) in entries.iter().cloned().enumerate() {
            if let Some(view) = self.message_views.get(index).cloned() {
                view.update(cx, |view, cx| view.sync(index, entry, cx));
            } else {
                self.message_views.push(
                    cx.new(|_| ChatMessageView::new(self.workspace_id, self.node_id, index, entry)),
                );
            }
        }
    }
}

impl Render for ChatBodyView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            },
            &self.scroll_handle,
            &mut self.scroll_revision,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn chat_node(
    workspace_id: usize,
    node: &SessionNode,
    screen_position: WorldPoint,
    pi_working: bool,
    input: Entity<InputState>,
    title_input: Entity<InputState>,
    body_view: Entity<ChatBodyView>,
    editing_title: bool,
    _window: &mut Window,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    let node_id = node.id();
    let scale = 1.0;
    let node_size = node.size();

    div()
        .id(SharedString::from(format!(
            "chat-node-{workspace_id}-{node_id}"
        )))
        .absolute()
        .left(px(screen_position.x))
        .top(px(screen_position.y))
        .w(px(node_size.width))
        .h(px(node_size.height))
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::surface())
        .text_color(theme::text())
        .text_size(px(14.0))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view, event: &gpui::MouseDownEvent, _window, cx| {
                view.start_node_drag(
                    node_id,
                    super::workspace_canvas::screen_point_from_event(event),
                    cx,
                );
                cx.stop_propagation();
            }),
        )
        .flex()
        .flex_col()
        .child(render_header(
            workspace_id,
            node,
            title_input,
            editing_title,
            false,
            scale,
            cx,
        ))
        .child(render_loading_bar(pi_working, scale))
        .child(render_body_view(workspace_id, node_id, body_view, cx))
        .child(render_composer(
            workspace_id,
            node_id,
            input,
            pi_working,
            scale,
            cx,
        ))
        .child(render_resize_handle(node_id, scale, cx))
        .into_any_element()
}

#[allow(clippy::too_many_arguments, dead_code)]
pub fn pinned_chat_node_panel(
    workspace_id: usize,
    node: &SessionNode,
    pi_working: bool,
    input: Entity<InputState>,
    title_input: Entity<InputState>,
    body_view: Entity<ChatBodyView>,
    editing_title: bool,
    focused: bool,
    _window: &mut Window,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    let node_id = node.id();
    let scale = 1.0;

    div()
        .id(SharedString::from(format!(
            "pinned-chat-node-{workspace_id}-{node_id}"
        )))
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
        .text_size(px(14.0))
        .flex()
        .flex_col()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                view.focus_pinned_node(workspace_id, node_id, cx);
            }),
        )
        .child(render_header(
            workspace_id,
            node,
            title_input,
            editing_title,
            true,
            scale,
            cx,
        ))
        .child(render_loading_bar(pi_working, scale))
        .child(render_body_view(workspace_id, node_id, body_view, cx))
        .child(render_composer(
            workspace_id,
            node_id,
            input,
            pi_working,
            scale,
            cx,
        ))
        .into_any_element()
}

fn render_header(
    workspace_id: usize,
    node: &SessionNode,
    title_input: Entity<InputState>,
    editing_title: bool,
    pinned: bool,
    scale: f32,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    let node_id = node.id();
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
                    workspace_id,
                    node_id,
                    node.title(),
                    title_input,
                    editing_title,
                    scale,
                    cx,
                )),
        )
        .child(
            h_flex()
                .flex_none()
                .items_center()
                .gap(scaled_px(4.0, scale))
                .child(pin_toggle(workspace_id, node_id, pinned, scale, cx))
                .child(close_button(workspace_id, node_id, scale, cx)),
        )
        .into_any_element()
}

fn render_title(
    workspace_id: usize,
    node_id: usize,
    title: String,
    input: Entity<InputState>,
    editing: bool,
    scale: f32,
    cx: &mut Context<PiDesktop>,
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
                cx.listener(move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                    view.focus_pinned_node(workspace_id, node_id, cx);
                    cx.stop_propagation();
                }),
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
            cx.listener(move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                view.focus_pinned_node(workspace_id, node_id, cx);
                cx.stop_propagation();
            }),
        )
        .on_click(cx.listener(move |view, _, window, cx| {
            view.start_session_title_edit(workspace_id, node_id, window, cx);
        }))
        .child(title)
        .into_any_element()
}

fn render_loading_bar(visible: bool, scale: f32) -> AnyElement {
    if !visible {
        return div().h(px(0.0)).into_any_element();
    }

    div()
        .relative()
        .h(scaled_px(3.0, scale))
        .overflow_hidden()
        .bg(theme::complement().opacity(0.18))
        .child(
            div()
                .absolute()
                .top_0()
                .left(scaled_px(-96.0, scale))
                .h_full()
                .w(scaled_px(96.0, scale))
                .bg(theme::complement())
                .with_animation(
                    "chat-node-loading-bar",
                    Animation::new(Duration::from_millis(1150))
                        .repeat()
                        .with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0)),
                    move |this, delta| this.left(scaled_px(-96.0 + (740.0 + 96.0) * delta, scale)),
                ),
        )
        .into_any_element()
}

fn render_body_view(
    workspace_id: usize,
    node_id: usize,
    body_view: Entity<ChatBodyView>,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    div()
        .relative()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .bg(theme::surface())
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                view.focus_pinned_node(workspace_id, node_id, cx);
                cx.stop_propagation();
            }),
        )
        .child(AnyView::from(body_view).cached(StyleRefinement::default().size_full()))
        .into_any_element()
}

struct BodyRenderState {
    message_views: Vec<Entity<ChatMessageView>>,
    entries_empty: bool,
    streaming: bool,
    revision: u64,
}

fn render_body_contents(
    workspace_id: usize,
    node_id: usize,
    body: BodyRenderState,
    scroll_handle: &ScrollHandle,
    scroll_revision: &mut u64,
) -> AnyElement {
    if body.streaming && *scroll_revision != body.revision {
        scroll_handle.scroll_to_bottom();
        *scroll_revision = body.revision;
    }

    div()
        .id(SharedString::from(format!(
            "chat-node-scroll-area-{workspace_id}-{node_id}"
        )))
        .size_full()
        .track_scroll(scroll_handle)
        .overflow_y_scroll()
        .vertical_scrollbar(scroll_handle)
        .child(
            v_flex()
                .min_h_full()
                .justify_end()
                .gap(px(14.0))
                .px(px(16.0))
                .py(px(14.0))
                .when(body.entries_empty, |this| {
                    this.child(render_empty_body(1.0))
                })
                .children(body.message_views),
        )
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
    workspace_id: usize,
    node_id: usize,
    input: Entity<InputState>,
    pi_working: bool,
    scale: f32,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    let show_working_text = pi_working && input.read(cx).value().trim().is_empty();
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
            cx.listener(move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                view.focus_pinned_node(workspace_id, node_id, cx);
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .relative()
                .flex_1()
                .min_w_0()
                .child(Input::new(&input).appearance(false).h_full())
                .when(show_working_text, |this| {
                    this.child(working_input_overlay(scale))
                }),
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
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.submit_chat_node(workspace_id, node_id, window, cx);
                }))
                .child("→"),
        )
        .into_any_element()
}

fn working_input_overlay(scale: f32) -> AnyElement {
    h_flex()
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .left_0()
        .items_center()
        .gap(scaled_px(1.0, scale))
        .bg(theme::app_bg())
        .text_size(scaled_px(14.0, scale))
        .text_color(theme::text_muted())
        .child("Working")
        .child(working_dot(0, scale))
        .child(working_dot(1, scale))
        .child(working_dot(2, scale))
        .into_any_element()
}

fn working_dot(index: usize, scale: f32) -> AnyElement {
    let threshold = match index {
        0 => 0.20,
        1 => 0.45,
        _ => 0.70,
    };
    div()
        .child(".")
        .text_size(scaled_px(14.0, scale))
        .with_animation(
            SharedString::from(format!("chat-working-dot-{index}")),
            Animation::new(Duration::from_millis(900)).repeat(),
            move |this, delta| {
                if delta >= threshold {
                    this.opacity(1.0)
                } else {
                    this.opacity(0.0)
                }
            },
        )
        .into_any_element()
}

fn pin_toggle(
    workspace_id: usize,
    node_id: usize,
    pinned: bool,
    scale: f32,
    cx: &mut Context<PiDesktop>,
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
            cx.listener(move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                view.focus_pinned_node(workspace_id, node_id, cx);
                cx.stop_propagation();
            }),
        )
        .on_click(cx.listener(move |view, _, _window, cx| {
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
    workspace_id: usize,
    node_id: usize,
    scale: f32,
    cx: &mut Context<PiDesktop>,
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
            cx.listener(move |view, _event: &gpui::MouseDownEvent, _window, cx| {
                view.focus_pinned_node(workspace_id, node_id, cx);
                cx.stop_propagation();
            }),
        )
        .on_click(cx.listener(move |view, _, _window, cx| {
            view.close_session_node(workspace_id, node_id, cx);
        }))
        .child("×")
        .into_any_element()
}

fn render_resize_handle(node_id: usize, scale: f32, cx: &mut Context<PiDesktop>) -> AnyElement {
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
            cx.listener(move |view, event: &gpui::MouseDownEvent, _window, cx| {
                view.start_node_resize(
                    node_id,
                    super::workspace_canvas::screen_point_from_event(event),
                    cx,
                );
                cx.stop_propagation();
            }),
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
