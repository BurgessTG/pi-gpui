use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, MouseButton, MouseDownEvent,
    MouseMoveEvent, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div,
    px,
};
use gpui_component::{StyledExt as _, h_flex};

use crate::app::PiDesktop;
use crate::design::theme;
use crate::workspace::canvas::{CanvasState, SessionNodePrimitive, WorldPoint, WorldSize};

use super::workspace_canvas::{
    BOTTOM_DOCK_HEIGHT, MINIMAP_BOTTOM, MINIMAP_HEIGHT, MINIMAP_LEFT, MINIMAP_WIDTH,
};

pub(crate) fn render_minimap(
    canvas: &CanvasState,
    canvas_size: WorldSize,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    div()
        .id("workspace-minimap")
        .absolute()
        .left(px(MINIMAP_LEFT))
        .bottom(px(MINIMAP_BOTTOM))
        .w(px(MINIMAP_WIDTH))
        .h(px(MINIMAP_HEIGHT))
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::app_bg())
        .overflow_hidden()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                view.start_minimap_pan(
                    minimap_local_from_screen(screen_point_from_mouse_down(event), window),
                    canvas_size,
                    cx,
                );
                cx.stop_propagation();
            }),
        )
        .on_mouse_move(cx.listener(|view, event: &MouseMoveEvent, window, cx| {
            if event.dragging() {
                view.update_minimap_pan(
                    minimap_local_from_screen(screen_point_from_mouse_move(event), window),
                    cx,
                );
                cx.stop_propagation();
            }
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|view, _, _window, cx| {
                view.end_minimap_pan(cx);
                cx.stop_propagation();
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|view, _, _window, cx| {
                view.end_minimap_pan(cx);
            }),
        )
        .child(render_minimap_grid())
        .children(render_minimap_node_symbols(canvas))
        .child(render_minimap_viewport(canvas, canvas_size))
        .child(render_minimap_controls(canvas.viewport(), cx))
        .into_any_element()
}

fn render_minimap_grid() -> AnyElement {
    let vertical = (1..6).map(|index| {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(index as f32 * MINIMAP_WIDTH / 6.0))
            .w(px(1.0))
            .bg(theme::grid_minor())
            .into_any_element()
    });
    let horizontal = (1..4).map(|index| {
        div()
            .absolute()
            .left_0()
            .right_0()
            .top(px(index as f32 * MINIMAP_HEIGHT / 4.0))
            .h(px(1.0))
            .bg(theme::grid_minor())
            .into_any_element()
    });

    div()
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .left_0()
        .children(vertical.chain(horizontal))
        .into_any_element()
}

fn render_minimap_node_symbols(canvas: &CanvasState) -> Vec<AnyElement> {
    let minimap_size = WorldSize::new(MINIMAP_WIDTH, MINIMAP_HEIGHT);
    canvas
        .nodes()
        .iter()
        .map(|node| {
            let position = canvas.node_minimap_position(node.position(), minimap_size);
            div()
                .absolute()
                .left(px(position.x - 5.0))
                .top(px(position.y - 5.0))
                .w(px(10.0))
                .h(px(10.0))
                .flex()
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(node_symbol_color(node.primitive()))
                .child(node_symbol(node.primitive()))
                .into_any_element()
        })
        .collect()
}

fn render_minimap_viewport(canvas: &CanvasState, canvas_size: WorldSize) -> AnyElement {
    let minimap_size = WorldSize::new(MINIMAP_WIDTH, MINIMAP_HEIGHT);
    let rect = canvas.minimap_viewport_rect(minimap_size, canvas_size);
    div()
        .absolute()
        .left(px(rect.left))
        .top(px(rect.top))
        .w(px(rect.width))
        .h(px(rect.height))
        .border_1()
        .border_color(theme::accent())
        .bg(theme::accent().opacity(0.10))
        .into_any_element()
}

fn render_minimap_controls(
    viewport: crate::workspace::canvas::CanvasViewport,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    h_flex()
        .id("workspace-zoom-controls")
        .absolute()
        .left(px(6.0))
        .right(px(4.0))
        .bottom(px(4.0))
        .items_center()
        .gap_1()
        .child(zoom_button(
            "zoom-in",
            "+",
            cx.listener(|view, _, _, cx| view.zoom_active_canvas_in(cx)),
        ))
        .child(zoom_button(
            "zoom-out",
            "−",
            cx.listener(|view, _, _, cx| view.zoom_active_canvas_out(cx)),
        ))
        .child(
            div()
                .flex_1()
                .text_right()
                .text_xs()
                .text_color(theme::text())
                .child(format!("{}%", (viewport.zoom * 100.0).round() as u32)),
        )
        .into_any_element()
}

fn zoom_button(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .w(px(18.0))
        .h(px(18.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::transparent_black())
        .cursor_pointer()
        .text_color(theme::text())
        .text_sm()
        .font_bold()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_click)
        .child(label)
        .into_any_element()
}

fn node_symbol(primitive: SessionNodePrimitive) -> &'static str {
    match primitive {
        SessionNodePrimitive::NewSession => "●",
        SessionNodePrimitive::ForkSession => "◆",
        SessionNodePrimitive::ResumeSession => "■",
    }
}

fn node_symbol_color(primitive: SessionNodePrimitive) -> gpui::Hsla {
    match primitive {
        SessionNodePrimitive::NewSession => theme::accent(),
        SessionNodePrimitive::ForkSession => theme::complement(),
        SessionNodePrimitive::ResumeSession => theme::success(),
    }
}

fn screen_point_from_mouse_down(event: &MouseDownEvent) -> WorldPoint {
    WorldPoint::new(f32::from(event.position.x), f32::from(event.position.y))
}

fn screen_point_from_mouse_move(event: &MouseMoveEvent) -> WorldPoint {
    WorldPoint::new(f32::from(event.position.x), f32::from(event.position.y))
}

fn minimap_local_from_screen(screen_position: WorldPoint, window: &Window) -> WorldPoint {
    let window_height = f32::from(window.bounds().size.height);
    let minimap_top = window_height - BOTTOM_DOCK_HEIGHT - MINIMAP_BOTTOM - MINIMAP_HEIGHT;
    WorldPoint::new(
        screen_position.x - MINIMAP_LEFT,
        screen_position.y - minimap_top,
    )
}
