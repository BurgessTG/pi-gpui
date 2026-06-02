use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, Context, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Window, div, px,
};
use gpui_component::{animation::cubic_bezier, h_flex};

use crate::design::theme;

pub struct LoadingBarView {
    visible: bool,
}

impl LoadingBarView {
    pub fn new(visible: bool) -> Self {
        Self { visible }
    }

    pub fn sync(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible == visible {
            return;
        }
        self.visible = visible;
        cx.notify();
    }
}

impl Render for LoadingBarView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("LoadingBarView");
        if !self.visible {
            return div().h(px(0.0)).into_any_element();
        }

        div()
            .relative()
            .h(px(3.0))
            .overflow_hidden()
            .bg(theme::complement().opacity(0.18))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(-96.0))
                    .h_full()
                    .w(px(96.0))
                    .bg(theme::complement())
                    .with_animation(
                        "chat-node-loading-bar",
                        Animation::new(Duration::from_millis(1150))
                            .repeat()
                            .with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0)),
                        move |this, delta| this.left(px(-96.0 + (740.0 + 96.0) * delta)),
                    ),
            )
            .into_any_element()
    }
}

pub struct WorkingInputOverlayView;

impl Render for WorkingInputOverlayView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("WorkingInputOverlayView");
        h_flex()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .left_0()
            .items_center()
            .gap(px(1.0))
            .bg(theme::app_bg())
            .text_size(px(14.0))
            .text_color(theme::text_muted())
            .child("Working")
            .child(working_dot(0))
            .child(working_dot(1))
            .child(working_dot(2))
            .into_any_element()
    }
}

fn working_dot(index: usize) -> AnyElement {
    let threshold = match index {
        0 => 0.20,
        1 => 0.45,
        _ => 0.70,
    };
    div()
        .child(".")
        .text_size(px(14.0))
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
