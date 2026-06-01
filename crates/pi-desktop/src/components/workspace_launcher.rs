use gpui::{Context, IntoElement, ParentElement as _, Styled as _, div, px, svg};

use crate::app::PiDesktop;
use crate::components::button::{PiButtonKind, pi_button};
use crate::design::theme;
use crate::ui;

pub fn workspace_launcher(cx: &mut Context<PiDesktop>) -> impl IntoElement {
    div()
        .relative()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .px_8()
        .child(
            div()
                .mb(px(64.0))
                .flex()
                .flex_col()
                .items_center()
                .gap_4()
                .child(
                    svg()
                        .path(ui::logo_path())
                        .size(px(72.0))
                        .text_color(theme::text()),
                )
                .child(
                    pi_button(
                        "open-workspace",
                        "Open Workspace",
                        PiButtonKind::Secondary,
                        cx,
                    )
                    .w(px(180.0))
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.start_open_workspace_flow(cx);
                    })),
                ),
        )
}
