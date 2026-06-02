use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, MouseButton, MouseDownEvent,
    ParentElement as _, Styled as _, div, px,
};

use crate::components::workspace_canvas::visible_world_bounds;
use crate::components::workspace_canvas_view::WorkspaceCanvasView;
use crate::design::theme;
use crate::workspace::canvas::{CanvasDrawing, CanvasDrawingTool, CanvasState, WorldSize};
use crate::workspace::pinning::PinnedLayout;

pub(super) fn render_pinned_markers(
    canvas: &CanvasState,
    workspace_id: usize,
    pinned_layout: &PinnedLayout,
    canvas_size: WorldSize,
    cx: &mut Context<WorkspaceCanvasView>,
) -> Vec<AnyElement> {
    if pinned_layout.is_empty() {
        return Vec::new();
    }

    let viewport = canvas.viewport();
    let focused = pinned_layout.focused_node_id();
    canvas
        .nodes()
        .iter()
        .filter(|node| pinned_layout.is_pinned(node.id()))
        .filter_map(|node| {
            let screen = viewport.world_to_screen(node.position());
            let size = node.size();
            let width = size.width;
            let height = size.height;
            if !screen_rect_visible(screen.x, screen.y, width, height, canvas_size) {
                return None;
            }
            let node_id = node.id();
            let active = focused == Some(node_id);
            Some(
                div()
                    .id(("pinned-node-marker", node_id))
                    .absolute()
                    .left(px(screen.x))
                    .top(px(screen.y))
                    .w(px(width.max(120.0)))
                    .h(px(height.max(72.0)))
                    .border_2()
                    .border_color(if active {
                        theme::accent()
                    } else {
                        theme::complement()
                    })
                    .bg(theme::complement().opacity(0.06))
                    .text_color(theme::text())
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, _event: &MouseDownEvent, _window, cx| {
                            view.focus_pinned_node(workspace_id, node_id, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(10.0))
                            .left(px(10.0))
                            .right(px(10.0))
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .border_1()
                            .border_color(theme::complement().opacity(0.65))
                            .bg(theme::surface().opacity(0.86))
                            .text_sm()
                            .child("📌")
                            .child(div().truncate().child(node.title()))
                            .child(
                                div()
                                    .ml_auto()
                                    .text_xs()
                                    .text_color(theme::text_muted())
                                    .child("Pinned"),
                            ),
                    )
                    .into_any_element(),
            )
        })
        .collect()
}

pub(super) fn render_number_markers(
    canvas: &CanvasState,
    active_tool: CanvasDrawingTool,
    canvas_size: WorldSize,
    cx: &mut Context<WorkspaceCanvasView>,
) -> Vec<AnyElement> {
    let viewport = canvas.viewport();
    let selected_index = canvas.selected_drawing_index();
    let visible_bounds = visible_world_bounds(viewport, canvas_size, 24.0);
    canvas
        .drawing_indices_in_bounds(&visible_bounds)
        .into_iter()
        .filter_map(|index| {
            let drawing = canvas.drawings().get(index)?;
            let CanvasDrawing::NumberMarker { position, number } = drawing else {
                return None;
            };
            let screen = viewport.world_to_screen(*position);
            if !screen_rect_visible(screen.x - 11.0, screen.y - 11.0, 22.0, 22.0, canvas_size) {
                return None;
            }
            let selected = selected_index == Some(index);
            Some(
                div()
                    .id(("number-marker", *number))
                    .absolute()
                    .left(px(screen.x - 11.0))
                    .top(px(screen.y - 11.0))
                    .size(px(22.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(if selected {
                        theme::accent()
                    } else {
                        theme::text()
                    })
                    .bg(theme::surface())
                    .text_color(theme::text())
                    .text_xs()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, event: &MouseDownEvent, _window, cx| {
                            if active_tool == CanvasDrawingTool::Select {
                                view.start_canvas_drawing_drag_by_index(
                                    index,
                                    screen_point_from_mouse_down(event),
                                    cx,
                                );
                                cx.stop_propagation();
                            }
                        }),
                    )
                    .child(number.to_string())
                    .into_any_element(),
            )
        })
        .collect()
}

fn screen_rect_visible(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    canvas_size: WorldSize,
) -> bool {
    left + width >= 0.0
        && top + height >= 0.0
        && left <= canvas_size.width
        && top <= canvas_size.height
}

fn screen_point_from_mouse_down(event: &MouseDownEvent) -> crate::workspace::canvas::WorldPoint {
    crate::workspace::canvas::WorldPoint::new(
        f32::from(event.position.x),
        f32::from(event.position.y),
    )
}
