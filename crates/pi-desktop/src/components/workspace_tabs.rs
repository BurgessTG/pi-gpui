use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use gpui_component::h_flex;

use crate::app::PiDesktop;
use crate::design::theme;

const TEXT_LEADING_PADDING: f32 = 2.0;
const CLOSE_BUTTON_WIDTH: f32 = 14.0;
const ADD_BUTTON_WIDTH: f32 = 24.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceTabInfo {
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceTabsProps {
    pub tabs: Vec<WorkspaceTabInfo>,
    pub active_index: Option<usize>,
    pub previous_index: Option<usize>,
}

pub struct WorkspaceTabsView {
    app: Entity<PiDesktop>,
    props: WorkspaceTabsProps,
}

impl WorkspaceTabsView {
    pub fn new(app: Entity<PiDesktop>, props: WorkspaceTabsProps) -> Self {
        Self { app, props }
    }

    pub fn sync(&mut self, props: WorkspaceTabsProps, cx: &mut Context<Self>) -> bool {
        if self.props == props {
            return false;
        }
        self.props = props;
        cx.notify();
        true
    }
}

impl Render for WorkspaceTabsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("WorkspaceTabsView");
        let selected_index = self.props.active_index.unwrap_or(0);
        let _previous_index = self.props.previous_index;
        let app = self.app.clone();

        h_flex()
            .id("workspace-tabs")
            .relative()
            .size_full()
            .items_center()
            .gap(px(2.0))
            .children(
                self.props
                    .tabs
                    .iter()
                    .enumerate()
                    .map(|(index, workspace)| {
                        let selected = index == selected_index;
                        workspace_tab(app.clone(), index, &workspace.title, selected)
                    }),
            )
            .child(add_workspace_button(app))
    }
}

fn workspace_tab(app: Entity<PiDesktop>, index: usize, title: &str, selected: bool) -> AnyElement {
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
        .on_click(app_listener(app.clone(), move |view, _, _, cx| {
            view.select_workspace_tab(index, cx);
        }))
        .child(title.to_owned())
        .child(close_tab_button(app, index))
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

fn close_tab_button(app: Entity<PiDesktop>, index: usize) -> AnyElement {
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
        .on_click(app_listener(app, move |view, _, _, cx| {
            cx.stop_propagation();
            view.close_workspace_tab(index, cx);
        }))
        .child("×")
        .into_any_element()
}

fn add_workspace_button(app: Entity<PiDesktop>) -> AnyElement {
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
        .on_click(app_listener(app, |view, _, _, cx| {
            cx.stop_propagation();
            view.start_open_workspace_flow(cx);
        }))
        .child("+")
        .into_any_element()
}

fn app_listener<E: ?Sized>(
    app: Entity<PiDesktop>,
    f: impl Fn(&mut PiDesktop, &E, &mut Window, &mut Context<PiDesktop>) + 'static,
) -> impl Fn(&E, &mut Window, &mut App) + 'static {
    move |event, window, cx| {
        app.update(cx, |view, cx| f(view, event, window, cx));
    }
}
