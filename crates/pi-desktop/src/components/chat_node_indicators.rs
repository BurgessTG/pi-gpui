use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, Context, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Window, div, px,
};
use gpui_component::{animation::cubic_bezier, h_flex};

use crate::design::theme;

pub struct LoadingBarView {
    visible: bool,
    scale: f32,
}

impl LoadingBarView {
    pub fn new(visible: bool, scale: f32) -> Self {
        Self { visible, scale }
    }

    pub fn sync(&mut self, visible: bool, scale: f32, cx: &mut Context<Self>) {
        if self.visible == visible && same_scale(self.scale, scale) {
            return;
        }
        self.visible = visible;
        self.scale = scale;
        cx.notify();
    }
}

impl Render for LoadingBarView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("LoadingBarView");
        if !self.visible {
            return div().h(px(0.0)).into_any_element();
        }

        let scale = self.scale;
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
                        move |this, delta| {
                            this.left(scaled_px(-96.0 + (740.0 + 96.0) * delta, scale))
                        },
                    ),
            )
            .into_any_element()
    }
}

pub struct WorkingInputOverlayView {
    scale: f32,
}

impl WorkingInputOverlayView {
    pub fn new(scale: f32) -> Self {
        Self { scale }
    }

    pub fn sync_scale(&mut self, scale: f32, cx: &mut Context<Self>) {
        if same_scale(self.scale, scale) {
            return;
        }
        self.scale = scale;
        cx.notify();
    }
}

impl Render for WorkingInputOverlayView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("WorkingInputOverlayView");
        let scale = self.scale;
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

fn scaled_px(value: f32, scale: f32) -> gpui::Pixels {
    px(value * scale.max(0.05))
}

fn same_scale(left: f32, right: f32) -> bool {
    (left - right).abs() <= f32::EPSILON
}
