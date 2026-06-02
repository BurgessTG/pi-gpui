use std::collections::HashMap;

use gpui::{
    AnyElement, Bounds, Context, Entity, FocusHandle, Hsla, InteractiveElement as _,
    IntoElement as _, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement as _, PathBuilder,
    PathStyle, Pixels, Point, ScrollWheelEvent, SharedString, StrokeOptions, Styled as _, Window,
    actions, canvas, div, fill, point, px, size,
};
use gpui_component::{
    IconName,
    input::{Input, InputState},
    menu::ContextMenuExt as _,
    menu::PopupMenu,
    slider::SliderState,
};
use lyon::path::{LineCap, LineJoin};

use crate::components::chat_node;
use crate::components::workspace_canvas_markers::{render_number_markers, render_pinned_markers};
use crate::components::workspace_canvas_minimap::render_minimap;
use crate::components::workspace_canvas_toolbar::render_drawing_tool_overlay;
use crate::components::workspace_canvas_view::WorkspaceCanvasView;
use crate::design::theme;
use crate::workspace::canvas::{
    CanvasDrawing, CanvasDrawingBounds, CanvasDrawingDraft, CanvasDrawingTool, CanvasState,
    CanvasViewport, WorldPoint, WorldSize,
};
use crate::workspace::state::WorkspaceTab;

actions!(
    pi_workspace_canvas,
    [CreateNewSessionNode, ForkSessionNode, ResumeSessionNode]
);

const BASE_GRID_SPACING: f32 = 28.0;
pub(super) const MINIMAP_WIDTH: f32 = 148.0;
pub(super) const MINIMAP_HEIGHT: f32 = 108.0;
pub(super) const MINIMAP_LEFT: f32 = 16.0;
pub(super) const MINIMAP_BOTTOM: f32 = 16.0;
pub(super) const BOTTOM_DOCK_HEIGHT: f32 = 28.0;
const SCROLL_ZOOM_DENOMINATOR: f32 = 240.0;

#[allow(clippy::too_many_arguments)]
pub fn workspace_canvas(
    tab: &WorkspaceTab,
    workspace_id: usize,
    can_fork: bool,
    can_resume: bool,
    canvas_size: WorldSize,
    text_box_inputs: &HashMap<usize, Entity<InputState>>,
    chat_node_views: &HashMap<usize, Entity<chat_node::ChatNodeView>>,
    snap_to_grid: bool,
    drawing_tools_visible: bool,
    active_drawing_tool: CanvasDrawingTool,
    drawing_stroke_width: f32,
    can_undo_drawing: bool,
    can_redo_drawing: bool,
    drawing_stroke_slider: Entity<SliderState>,
    focus_handle: FocusHandle,
    cx: &mut Context<WorkspaceCanvasView>,
) -> AnyElement {
    let canvas = tab.canvas();
    let pinned_layout = tab.pinned_layout();
    let viewport = canvas.viewport();
    let menu_focus_handle = focus_handle.clone();
    let drawing_active = drawing_tools_visible && active_drawing_tool.draws();

    div()
        .id("workspace-canvas")
        .relative()
        .size_full()
        .overflow_hidden()
        .bg(theme::app_bg())
        .track_focus(&focus_handle)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                view.commit_current_text_box_edit_and_focus_canvas(window, cx);
                if drawing_active {
                    view.start_canvas_drawing(
                        active_drawing_tool,
                        screen_point_from_mouse_down(event),
                        cx,
                    );
                    cx.stop_propagation();
                } else if active_drawing_tool == CanvasDrawingTool::Select
                    && view.start_canvas_drawing_drag(screen_point_from_mouse_down(event), cx)
                {
                    cx.stop_propagation();
                } else {
                    if active_drawing_tool == CanvasDrawingTool::Select {
                        view.clear_canvas_drawing_selection(cx);
                    }
                    view.start_canvas_pan(screen_point_from_mouse_down(event), cx);
                }
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|view, event: &MouseDownEvent, _window, cx| {
                view.note_canvas_context_position(event, cx);
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |view, _, _window, cx| {
                if drawing_active {
                    view.end_canvas_drawing(cx);
                    cx.stop_propagation();
                } else {
                    view.end_canvas_pan(cx);
                }
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(move |view, _, _window, cx| {
                if drawing_active {
                    view.end_canvas_drawing(cx);
                } else {
                    view.end_canvas_pan(cx);
                }
            }),
        )
        .on_mouse_move(
            cx.listener(move |view, event: &MouseMoveEvent, window, cx| {
                if event.dragging() {
                    if drawing_active {
                        view.update_canvas_drawing(screen_point_from_mouse_move(event), window, cx);
                        cx.stop_propagation();
                        return;
                    }
                    let minimap_local =
                        minimap_local_from_screen(screen_point_from_mouse_move(event), window);
                    if view.update_minimap_pan(minimap_local, window, cx) {
                        cx.stop_propagation();
                        return;
                    }
                    view.update_canvas_pan(screen_point_from_mouse_move(event), window, cx);
                } else {
                    view.end_minimap_pan(cx);
                    view.end_canvas_pan(cx);
                }
            }),
        )
        .on_scroll_wheel(cx.listener(|view, event: &ScrollWheelEvent, window, cx| {
            let zoom_factor = zoom_factor_from_scroll_event(event);
            if zoom_factor != 1.0 {
                view.zoom_active_canvas_at(
                    screen_point_from_scroll_event(event),
                    zoom_factor,
                    window,
                    cx,
                );
                cx.stop_propagation();
            }
        }))
        .on_action(cx.listener(|view, _: &CreateNewSessionNode, _window, cx| {
            view.create_new_session_node(cx);
        }))
        .on_action(cx.listener(|view, _: &ForkSessionNode, _window, cx| {
            view.create_fork_session_node(cx);
        }))
        .on_action(cx.listener(|view, _: &ResumeSessionNode, _window, cx| {
            view.create_resume_session_node(cx);
        }))
        .child(render_grid(canvas))
        .child(render_drawings(canvas, drawing_stroke_width, canvas_size))
        .children(render_text_boxes(
            canvas,
            workspace_id,
            active_drawing_tool,
            canvas_size,
            text_box_inputs,
            cx,
        ))
        .children(render_number_markers(
            canvas,
            active_drawing_tool,
            canvas_size,
            cx,
        ))
        .children(render_pinned_markers(
            canvas,
            workspace_id,
            pinned_layout,
            canvas_size,
            cx,
        ))
        .children(canvas.nodes().iter().filter_map(|node| {
            if pinned_layout.is_pinned(node.id()) {
                return None;
            }
            let screen_position = viewport.world_to_screen(node.position());
            let node_size = node.size();
            let screen_size = WorldSize::new(
                node_size.width * viewport.zoom,
                node_size.height * viewport.zoom,
            );
            if !node_visible(screen_position, screen_size, canvas_size) {
                return None;
            }
            let node_view = chat_node_views.get(&node.id())?.clone();
            Some(
                div()
                    .id(SharedString::from(format!(
                        "chat-node-frame-{workspace_id}-{}",
                        node.id()
                    )))
                    .absolute()
                    .left(px(screen_position.x))
                    .top(px(screen_position.y))
                    .w(px(screen_size.width))
                    .h(px(screen_size.height))
                    .child(node_view)
                    .into_any_element(),
            )
        }))
        .child(render_minimap(canvas, canvas_size, cx))
        .child(render_drawing_tool_overlay(
            drawing_tools_visible,
            active_drawing_tool,
            snap_to_grid,
            drawing_stroke_width,
            can_undo_drawing,
            can_redo_drawing,
            drawing_stroke_slider,
            cx,
        ))
        .context_menu(move |menu, _window, _cx| {
            session_node_menu(menu, menu_focus_handle.clone(), can_fork, can_resume)
        })
        .into_any_element()
}

fn render_drawings(
    canvas_state: &CanvasState,
    stroke_width: f32,
    canvas_size: WorldSize,
) -> AnyElement {
    let viewport = canvas_state.viewport();
    let visual_stroke_width = stroke_width * viewport.zoom;
    let visible_bounds = visible_world_bounds(viewport, canvas_size, visual_stroke_width + 24.0);
    let drawings = canvas_state
        .drawing_indices_in_bounds(&visible_bounds)
        .into_iter()
        .filter_map(|index| {
            canvas_state
                .drawings()
                .get(index)
                .map(|drawing| (index, drawing.clone()))
        })
        .collect::<Vec<_>>();
    let draft = canvas_state.drawing_draft().cloned();
    let color = theme::text();
    let selection_color = theme::accent();
    let selected_index = canvas_state.selected_drawing_index();

    canvas(
        move |_, _, _| (),
        move |bounds, _, window, _| {
            for (index, drawing) in drawings.iter() {
                paint_drawing(
                    drawing,
                    selected_index == Some(*index),
                    viewport,
                    bounds,
                    color,
                    selection_color,
                    visual_stroke_width,
                    window,
                );
            }
            if let Some(draft) = draft.as_ref() {
                paint_drawing_draft(draft, viewport, bounds, color, visual_stroke_width, window);
            }
        },
    )
    .absolute()
    .top_0()
    .right_0()
    .bottom_0()
    .left_0()
    .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn paint_drawing(
    drawing: &CanvasDrawing,
    selected: bool,
    viewport: CanvasViewport,
    bounds: Bounds<Pixels>,
    color: Hsla,
    selection_color: Hsla,
    stroke_width: f32,
    window: &mut Window,
) {
    match drawing {
        CanvasDrawing::Pen { points } => {
            paint_smooth_path(points, viewport, bounds, color, stroke_width, window)
        }
        CanvasDrawing::Line { start, end } => {
            paint_line(*start, *end, viewport, bounds, color, stroke_width, window);
        }
        CanvasDrawing::Arrow { start, end } => {
            paint_arrow(*start, *end, viewport, bounds, color, stroke_width, window);
        }
        CanvasDrawing::Rectangle { start, end } => {
            paint_rectangle(*start, *end, viewport, bounds, color, stroke_width, window);
        }
        CanvasDrawing::TextBox { start, end, .. } => {
            if selected {
                paint_rectangle(*start, *end, viewport, bounds, selection_color, 1.5, window);
            }
        }
        CanvasDrawing::Circle { start, end } => {
            paint_ellipse(*start, *end, viewport, bounds, color, stroke_width, window);
        }
        CanvasDrawing::NumberMarker { .. } => {}
    }

    if selected
        && !matches!(
            drawing,
            CanvasDrawing::TextBox { .. } | CanvasDrawing::NumberMarker { .. }
        )
    {
        paint_selection_bounds(drawing, viewport, bounds, selection_color, window);
    }
}

fn paint_drawing_draft(
    draft: &CanvasDrawingDraft,
    viewport: CanvasViewport,
    bounds: Bounds<Pixels>,
    color: Hsla,
    stroke_width: f32,
    window: &mut Window,
) {
    match draft.tool {
        CanvasDrawingTool::Pen => {
            paint_smooth_path(&draft.points, viewport, bounds, color, stroke_width, window)
        }
        CanvasDrawingTool::Line => {
            paint_line(
                draft.start,
                draft.current,
                viewport,
                bounds,
                color,
                stroke_width,
                window,
            );
        }
        CanvasDrawingTool::Arrow => {
            paint_arrow(
                draft.start,
                draft.current,
                viewport,
                bounds,
                color,
                stroke_width,
                window,
            );
        }
        CanvasDrawingTool::Rectangle | CanvasDrawingTool::TextBox => {
            paint_rectangle(
                draft.start,
                draft.current,
                viewport,
                bounds,
                color,
                stroke_width,
                window,
            );
        }
        CanvasDrawingTool::Circle => {
            paint_ellipse(
                draft.start,
                draft.current,
                viewport,
                bounds,
                color,
                stroke_width,
                window,
            );
        }
        CanvasDrawingTool::Select | CanvasDrawingTool::Eraser | CanvasDrawingTool::NumberMarker => {
        }
    }
}

fn paint_smooth_path(
    points: &[WorldPoint],
    viewport: CanvasViewport,
    bounds: Bounds<Pixels>,
    color: Hsla,
    stroke_width: f32,
    window: &mut Window,
) {
    let len = points.len();
    if len == 0 {
        return;
    }
    let screen_point = |index: usize| drawing_point(points[index], viewport, bounds);
    if len == 1 {
        paint_round_dot(screen_point(0), color, stroke_width, window);
        return;
    }

    let mut builder = rounded_stroke_builder(stroke_width);
    builder.move_to(screen_point(0));
    if len == 2 {
        builder.line_to(screen_point(1));
    } else {
        for index in 0..len - 1 {
            let p0 = if index == 0 {
                screen_point(0)
            } else {
                screen_point(index - 1)
            };
            let p1 = screen_point(index);
            let p2 = screen_point(index + 1);
            let p3 = if index + 2 < len {
                screen_point(index + 2)
            } else {
                screen_point(len - 1)
            };
            let control_a = point(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
            let control_b = point(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);
            builder.cubic_bezier_to(p2, control_a, control_b);
        }
    }

    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn paint_line(
    start: WorldPoint,
    end: WorldPoint,
    viewport: CanvasViewport,
    bounds: Bounds<Pixels>,
    color: Hsla,
    stroke_width: f32,
    window: &mut Window,
) {
    let start = drawing_point(start, viewport, bounds);
    let end = drawing_point(end, viewport, bounds);
    if start == end {
        paint_round_dot(start, color, stroke_width, window);
        return;
    }

    let mut builder = rounded_stroke_builder(stroke_width);
    builder.move_to(start);
    builder.line_to(end);
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn paint_rectangle(
    start: WorldPoint,
    end: WorldPoint,
    viewport: CanvasViewport,
    bounds: Bounds<Pixels>,
    color: Hsla,
    stroke_width: f32,
    window: &mut Window,
) {
    let original_start = start;
    let original_end = end;
    let start = drawing_point(start, viewport, bounds);
    let end = drawing_point(end, viewport, bounds);
    let left = start.x.min(end.x);
    let right = start.x.max(end.x);
    let top = start.y.min(end.y);
    let bottom = start.y.max(end.y);
    if right - left <= px(0.5) || bottom - top <= px(0.5) {
        paint_line(
            original_start,
            original_end,
            viewport,
            bounds,
            color,
            stroke_width,
            window,
        );
        return;
    }

    let mut builder = square_stroke_builder(stroke_width);
    builder.move_to(point(left, top));
    builder.line_to(point(right, top));
    builder.line_to(point(right, bottom));
    builder.line_to(point(left, bottom));
    builder.close();
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn paint_arrow(
    start: WorldPoint,
    end: WorldPoint,
    viewport: CanvasViewport,
    bounds: Bounds<Pixels>,
    color: Hsla,
    stroke_width: f32,
    window: &mut Window,
) {
    paint_line(start, end, viewport, bounds, color, stroke_width, window);
    let start = drawing_point(start, viewport, bounds);
    let end = drawing_point(end, viewport, bounds);
    let dx = f32::from(end.x - start.x);
    let dy = f32::from(end.y - start.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.1 {
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let head = 14.0 * viewport.zoom + stroke_width;
    let left = point(
        end.x - px(ux * head - uy * head * 0.55),
        end.y - px(uy * head + ux * head * 0.55),
    );
    let right = point(
        end.x - px(ux * head + uy * head * 0.55),
        end.y - px(uy * head - ux * head * 0.55),
    );
    let mut builder = rounded_stroke_builder(stroke_width);
    builder.move_to(left);
    builder.line_to(end);
    builder.line_to(right);
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn paint_ellipse(
    start: WorldPoint,
    end: WorldPoint,
    viewport: CanvasViewport,
    bounds: Bounds<Pixels>,
    color: Hsla,
    stroke_width: f32,
    window: &mut Window,
) {
    let start = drawing_point(start, viewport, bounds);
    let end = drawing_point(end, viewport, bounds);
    let left = start.x.min(end.x);
    let right = start.x.max(end.x);
    let top = start.y.min(end.y);
    let bottom = start.y.max(end.y);
    let rx = (right - left) / 2.0;
    let ry = (bottom - top) / 2.0;
    if rx <= px(0.5) || ry <= px(0.5) {
        return;
    }
    let center = point(left + rx, top + ry);
    let mut builder = rounded_stroke_builder(stroke_width);
    for step in 0..=48 {
        let angle = step as f32 / 48.0 * std::f32::consts::TAU;
        let p = point(center.x + rx * angle.cos(), center.y + ry * angle.sin());
        if step == 0 {
            builder.move_to(p);
        } else {
            builder.line_to(p);
        }
    }
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn paint_selection_bounds(
    drawing: &CanvasDrawing,
    viewport: CanvasViewport,
    bounds: Bounds<Pixels>,
    color: Hsla,
    window: &mut Window,
) {
    let Some(drawing_bounds) = drawing.bounds() else {
        return;
    };
    let padding = 6.0 / viewport.zoom.max(0.1);
    paint_rectangle(
        WorldPoint::new(drawing_bounds.left - padding, drawing_bounds.top - padding),
        WorldPoint::new(
            drawing_bounds.right + padding,
            drawing_bounds.bottom + padding,
        ),
        viewport,
        bounds,
        color,
        1.0,
        window,
    );
}

fn paint_round_dot(center: Point<Pixels>, color: Hsla, stroke_width: f32, window: &mut Window) {
    let diameter = px(stroke_width);
    window.paint_quad(
        fill(Bounds::centered_at(center, size(diameter, diameter)), color)
            .corner_radii(diameter / 2.0),
    );
}

fn drawing_point(
    world_point: WorldPoint,
    viewport: CanvasViewport,
    bounds: Bounds<Pixels>,
) -> Point<Pixels> {
    let screen_point = viewport.world_to_screen(world_point);
    point(
        bounds.origin.x + px(screen_point.x),
        bounds.origin.y + px(screen_point.y),
    )
}

pub(super) fn visible_world_bounds(
    viewport: CanvasViewport,
    canvas_size: WorldSize,
    screen_padding: f32,
) -> CanvasDrawingBounds {
    let top_left = viewport.screen_to_world(WorldPoint::new(0.0, 0.0));
    let bottom_right =
        viewport.screen_to_world(WorldPoint::new(canvas_size.width, canvas_size.height));
    let padding = screen_padding / viewport.zoom.max(0.1);
    CanvasDrawingBounds {
        left: top_left.x.min(bottom_right.x) - padding,
        top: top_left.y.min(bottom_right.y) - padding,
        right: top_left.x.max(bottom_right.x) + padding,
        bottom: top_left.y.max(bottom_right.y) + padding,
    }
}

fn node_visible(screen_position: WorldPoint, node_size: WorldSize, canvas_size: WorldSize) -> bool {
    screen_rect_visible(
        screen_position.x,
        screen_position.y,
        node_size.width,
        node_size.height,
        canvas_size,
    )
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

fn rounded_stroke_builder(stroke_width: f32) -> PathBuilder {
    stroke_builder(LineCap::Round, LineJoin::Round, stroke_width)
}

fn square_stroke_builder(stroke_width: f32) -> PathBuilder {
    stroke_builder(LineCap::Butt, LineJoin::Miter, stroke_width)
}

fn stroke_builder(line_cap: LineCap, line_join: LineJoin, stroke_width: f32) -> PathBuilder {
    PathBuilder::default().with_style(PathStyle::Stroke(
        StrokeOptions::DEFAULT
            .with_line_width(stroke_width)
            .with_line_cap(line_cap)
            .with_line_join(line_join)
            .with_tolerance(0.03),
    ))
}

#[allow(clippy::too_many_arguments)]
fn render_text_boxes(
    canvas: &CanvasState,
    workspace_id: usize,
    active_tool: CanvasDrawingTool,
    canvas_size: WorldSize,
    text_box_inputs: &HashMap<usize, Entity<InputState>>,
    cx: &mut Context<WorkspaceCanvasView>,
) -> Vec<AnyElement> {
    let viewport = canvas.viewport();
    let selected_index = canvas.selected_drawing_index();
    let visible_bounds = visible_world_bounds(viewport, canvas_size, 56.0 * viewport.zoom);
    canvas
        .drawing_indices_in_bounds(&visible_bounds)
        .into_iter()
        .filter_map(|index| {
            let drawing = canvas.drawings().get(index)?;
            let CanvasDrawing::TextBox { start, end, .. } = drawing else {
                return None;
            };
            let input = text_box_inputs.get(&index)?.clone();
            let input_for_focus = input.clone();
            let screen_start = viewport.world_to_screen(*start);
            let screen_end = viewport.world_to_screen(*end);
            let left = screen_start.x.min(screen_end.x);
            let top = screen_start.y.min(screen_end.y);
            let text_scale = viewport.zoom;
            let width = (screen_start.x - screen_end.x).abs().max(48.0 * text_scale);
            let height = (screen_start.y - screen_end.y).abs().max(32.0 * text_scale);
            if !screen_rect_visible(left, top, width, height, canvas_size) {
                return None;
            }
            let selected = selected_index == Some(index);
            Some(
                div()
                    .id(("text-box", index))
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(width))
                    .h(px(height))
                    .overflow_hidden()
                    .border_1()
                    .border_color(if selected {
                        theme::accent()
                    } else {
                        gpui::transparent_black()
                    })
                    .text_color(theme::text())
                    .text_size(px(14.0 * text_scale))
                    .cursor_text()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, _event: &MouseDownEvent, window, cx| {
                            view.select_canvas_drawing(workspace_id, index, cx);
                            input_for_focus.update(cx, |input, cx| input.focus(window, cx));
                            cx.stop_propagation();
                        }),
                    )
                    .child(
                        div()
                            .size_full()
                            .px(px(4.0 * text_scale))
                            .py(px(3.0 * text_scale))
                            .child(
                                Input::new(&input)
                                    .appearance(false)
                                    .focus_bordered(false)
                                    .h_full(),
                            ),
                    )
                    .children(
                        (selected && active_tool == CanvasDrawingTool::Select)
                            .then(|| render_text_box_drag_handle(index, text_scale, cx)),
                    )
                    .into_any_element(),
            )
        })
        .collect()
}

fn render_text_box_drag_handle(
    drawing_index: usize,
    scale: f32,
    cx: &mut Context<WorkspaceCanvasView>,
) -> AnyElement {
    div()
        .id(("text-box-drag-handle", drawing_index))
        .absolute()
        .right(px(2.0 * scale))
        .top(px(2.0 * scale))
        .w(px(18.0 * scale))
        .h(px(18.0 * scale))
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(theme::accent())
        .bg(theme::surface())
        .cursor_pointer()
        .text_color(theme::text())
        .text_size(px(12.0 * scale))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view, event: &MouseDownEvent, _window, cx| {
                view.start_canvas_drawing_drag_by_index(
                    drawing_index,
                    screen_point_from_mouse_down(event),
                    cx,
                );
                cx.stop_propagation();
            }),
        )
        .child("↕")
        .into_any_element()
}

fn render_grid(canvas_state: &CanvasState) -> AnyElement {
    let viewport = canvas_state.viewport();

    canvas(
        move |_, _, _| (),
        move |bounds, _, window, _| {
            let spacing = (BASE_GRID_SPACING * viewport.zoom).clamp(16.0, 72.0);
            paint_grid_axis(bounds, viewport.pan_x, spacing, true, window);
            paint_grid_axis(bounds, viewport.pan_y, spacing, false, window);
        },
    )
    .absolute()
    .top_0()
    .right_0()
    .bottom_0()
    .left_0()
    .into_any_element()
}

fn paint_grid_axis(
    bounds: Bounds<Pixels>,
    pan: f32,
    spacing: f32,
    vertical: bool,
    window: &mut Window,
) {
    let extent = if vertical {
        f32::from(bounds.size.width)
    } else {
        f32::from(bounds.size.height)
    };
    let start = pan.rem_euclid(spacing) - spacing;
    let line_count = (extent / spacing).ceil() as i32 + 3;

    for index in 0..line_count {
        let position = start + index as f32 * spacing;
        if position < -1.0 || position > extent + 1.0 {
            continue;
        }
        let grid_index = ((position - pan) / spacing).round() as i32;
        let color = if grid_index.rem_euclid(4) == 0 {
            theme::grid_major()
        } else {
            theme::grid_minor()
        };
        let line_bounds = if vertical {
            Bounds::new(
                point(bounds.origin.x + px(position), bounds.origin.y),
                size(px(1.0), bounds.size.height),
            )
        } else {
            Bounds::new(
                point(bounds.origin.x, bounds.origin.y + px(position)),
                size(bounds.size.width, px(1.0)),
            )
        };
        window.paint_quad(fill(line_bounds, color));
    }
}

fn session_node_menu(
    menu: PopupMenu,
    focus_handle: FocusHandle,
    can_fork: bool,
    can_resume: bool,
) -> PopupMenu {
    menu.action_context(focus_handle)
        .label("Create session node")
        .separator()
        .menu_with_icon(
            "New Session",
            IconName::Plus,
            Box::new(CreateNewSessionNode),
        )
        .menu_with_icon_and_disabled(
            "Fork Session",
            IconName::Redo2,
            Box::new(ForkSessionNode),
            !can_fork,
        )
        .menu_with_icon_and_disabled(
            "Resume Session",
            IconName::FolderOpen,
            Box::new(ResumeSessionNode),
            !can_resume,
        )
}

pub fn screen_point_from_event(event: &MouseDownEvent) -> WorldPoint {
    screen_point_from_mouse_down(event)
}

fn screen_point_from_mouse_down(event: &MouseDownEvent) -> WorldPoint {
    WorldPoint::new(f32::from(event.position.x), f32::from(event.position.y))
}

pub fn screen_point_from_mouse_move(event: &MouseMoveEvent) -> WorldPoint {
    WorldPoint::new(f32::from(event.position.x), f32::from(event.position.y))
}

fn screen_point_from_scroll_event(event: &ScrollWheelEvent) -> WorldPoint {
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

fn zoom_factor_from_scroll_event(event: &ScrollWheelEvent) -> f32 {
    let delta = event.delta.pixel_delta(px(16.0));
    let dominant_delta = if delta.y.abs() >= delta.x.abs() {
        delta.y
    } else {
        delta.x
    };
    let amount = f32::from(dominant_delta);
    if amount.abs() < f32::EPSILON {
        return 1.0;
    }

    (-amount / SCROLL_ZOOM_DENOMINATOR).exp().clamp(0.85, 1.18)
}
