use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use gpui_component::h_flex;

use crate::app::PiDesktop;
use crate::design::theme;
use crate::workspace::state::WorkspaceState;

const TEXT_LEADING_PADDING: f32 = 2.0;
const CLOSE_BUTTON_WIDTH: f32 = 14.0;
const ADD_BUTTON_WIDTH: f32 = 24.0;

pub fn workspace_tabs(
    state: &WorkspaceState,
    previous_index: Option<usize>,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    let selected_index = state.active_index().unwrap_or(0);
    let _previous_index = previous_index;

    h_flex()
        .id("workspace-tabs")
        .relative()
        .h_full()
        .items_center()
        .gap(px(2.0))
        .children(state.tabs().iter().enumerate().map(|(index, workspace)| {
            let selected = index == selected_index;
            workspace_tab(index, workspace.title(), selected, cx)
        }))
        .child(add_workspace_button(cx))
        .into_any_element()
}

fn workspace_tab(
    index: usize,
    title: &str,
    selected: bool,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    div()
        .id(("workspace-tab", index))
        .relative()
        .h_full()
        .flex()
        .items_center()
        .pl(px(TEXT_LEADING_PADDING))
        .cursor_pointer()
        .text_sm()
        .text_color(if selected {
            theme::text()
        } else {
            theme::text_muted()
        })
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(cx.listener(move |view, _, _, cx| {
            view.select_workspace_tab(index, cx);
        }))
        .child(title.to_owned())
        .child(close_tab_button(index, cx))
        .when(selected, |this| {
            this.child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .h(px(2.0))
                    .bg(theme::text()),
            )
        })
        .into_any_element()
}

fn close_tab_button(index: usize, cx: &mut Context<PiDesktop>) -> AnyElement {
    div()
        .id(("close-workspace-tab", index))
        .w(px(CLOSE_BUTTON_WIDTH))
        .h(px(18.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_xs()
        .text_color(theme::text())
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(cx.listener(move |view, _, _, cx| {
            cx.stop_propagation();
            view.close_workspace_tab(index, cx);
        }))
        .child("×")
        .into_any_element()
}

fn add_workspace_button(cx: &mut Context<PiDesktop>) -> AnyElement {
    div()
        .id("add-workspace-tab")
        .h_full()
        .w(px(ADD_BUTTON_WIDTH))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_sm()
        .text_color(theme::text())
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(cx.listener(|view, _, _, cx| {
            cx.stop_propagation();
            view.start_open_workspace_flow(cx);
        }))
        .child("+")
        .into_any_element()
}
