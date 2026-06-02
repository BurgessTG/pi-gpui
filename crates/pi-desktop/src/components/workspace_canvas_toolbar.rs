use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, Context, Entity, InteractiveElement as _,
    IntoElement as _, MouseButton, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, div, px, svg,
};
use gpui_component::{
    StyledExt as _,
    animation::cubic_bezier,
    h_flex,
    slider::{Slider, SliderState},
    tooltip::Tooltip,
};

use crate::components::workspace_canvas_view::WorkspaceCanvasView;
use crate::design::theme;
use crate::ui;
use crate::workspace::canvas::CanvasDrawingTool;

const DRAWING_TOOL_ANIMATION: Duration = Duration::from_millis(160);

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_drawing_tool_overlay(
    visible: bool,
    active_tool: CanvasDrawingTool,
    _snap_to_grid: bool,
    stroke_width: f32,
    can_undo: bool,
    can_redo: bool,
    stroke_slider: Entity<SliderState>,
    cx: &mut Context<WorkspaceCanvasView>,
) -> AnyElement {
    if !visible {
        return div().into_any_element();
    }

    div()
        .id("drawing-tool-overlay")
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .left_0()
        .opacity(1.0)
        .child(render_drawing_tool_palette(
            active_tool,
            stroke_width,
            can_undo,
            can_redo,
            stroke_slider,
            cx,
        ))
        .with_animation(
            "drawing-tool-overlay-fade-in",
            Animation::new(DRAWING_TOOL_ANIMATION).with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0)),
            |this, delta| this.opacity(delta),
        )
        .into_any_element()
}

fn render_drawing_tool_palette(
    active_tool: CanvasDrawingTool,
    stroke_width: f32,
    can_undo: bool,
    can_redo: bool,
    stroke_slider: Entity<SliderState>,
    cx: &mut Context<WorkspaceCanvasView>,
) -> AnyElement {
    h_flex()
        .id("drawing-tool-palette")
        .absolute()
        .top(px(12.0))
        .left_0()
        .right_0()
        .justify_center()
        .child(
            h_flex()
                .items_center()
                .gap_1()
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::surface().opacity(0.94))
                .p_1()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(drawing_tool_button(
                    CanvasDrawingTool::Select,
                    active_tool,
                    cx,
                ))
                .child(drawing_tool_button(CanvasDrawingTool::Pen, active_tool, cx))
                .child(drawing_tool_button(
                    CanvasDrawingTool::Line,
                    active_tool,
                    cx,
                ))
                .child(drawing_tool_button(
                    CanvasDrawingTool::Arrow,
                    active_tool,
                    cx,
                ))
                .child(drawing_tool_button(
                    CanvasDrawingTool::Rectangle,
                    active_tool,
                    cx,
                ))
                .child(drawing_tool_button(
                    CanvasDrawingTool::Circle,
                    active_tool,
                    cx,
                ))
                .child(drawing_tool_button(
                    CanvasDrawingTool::TextBox,
                    active_tool,
                    cx,
                ))
                .child(drawing_tool_button(
                    CanvasDrawingTool::NumberMarker,
                    active_tool,
                    cx,
                ))
                .child(drawing_tool_button(
                    CanvasDrawingTool::Eraser,
                    active_tool,
                    cx,
                ))
                .child(div().w(px(1.0)).h(px(18.0)).bg(theme::hairline()))
                .child(history_button(
                    "drawing-undo",
                    "↶",
                    "Undo",
                    can_undo,
                    true,
                    cx,
                ))
                .child(history_button(
                    "drawing-redo",
                    "↷",
                    "Redo",
                    can_redo,
                    false,
                    cx,
                ))
                .child(div().w(px(1.0)).h(px(18.0)).bg(theme::hairline()))
                .child(render_stroke_width_control(stroke_width, stroke_slider)),
        )
        .into_any_element()
}

fn drawing_tool_button(
    tool: CanvasDrawingTool,
    active_tool: CanvasDrawingTool,
    cx: &mut Context<WorkspaceCanvasView>,
) -> AnyElement {
    let selected = tool == active_tool;
    let tool_id: usize = match tool {
        CanvasDrawingTool::Select => 0,
        CanvasDrawingTool::Pen => 1,
        CanvasDrawingTool::Line => 2,
        CanvasDrawingTool::Arrow => 3,
        CanvasDrawingTool::Rectangle => 4,
        CanvasDrawingTool::Circle => 5,
        CanvasDrawingTool::TextBox => 6,
        CanvasDrawingTool::NumberMarker => 7,
        CanvasDrawingTool::Eraser => 8,
    };
    let tooltip = drawing_tool_tooltip(tool);

    div()
        .id(("drawing-tool-button", tool_id))
        .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
        .h(px(30.0))
        .w(px(30.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_size(px(22.0))
        .text_color(if selected {
            theme::app_bg()
        } else {
            theme::text()
        })
        .bg(if selected {
            theme::accent()
        } else {
            gpui::transparent_black()
        })
        .hover(|style| style.bg(theme::surface_hover()).text_color(theme::text()))
        .on_click(cx.listener(move |view, _, _, cx| {
            cx.stop_propagation();
            view.select_drawing_tool(tool, cx);
        }))
        .child(drawing_tool_icon(tool, selected))
        .into_any_element()
}

fn drawing_tool_icon(tool: CanvasDrawingTool, selected: bool) -> AnyElement {
    let color = if selected {
        theme::app_bg()
    } else {
        theme::text()
    };
    match tool {
        CanvasDrawingTool::Select => svg()
            .path(ui::drawing_tool_icon_path("pointer"))
            .size(px(24.0))
            .text_color(color)
            .into_any_element(),
        CanvasDrawingTool::Pen => svg()
            .path(ui::drawing_tool_icon_path("pencil"))
            .size(px(24.0))
            .text_color(color)
            .into_any_element(),
        CanvasDrawingTool::Eraser => svg()
            .path(ui::drawing_tool_icon_path("eraser"))
            .size(px(24.0))
            .text_color(color)
            .into_any_element(),
        CanvasDrawingTool::TextBox => div()
            .font_bold()
            .text_size(px(22.0))
            .text_color(color)
            .child("T")
            .into_any_element(),
        CanvasDrawingTool::NumberMarker => div()
            .font_bold()
            .text_size(px(21.0))
            .text_color(color)
            .child("#")
            .into_any_element(),
        _ => div()
            .text_size(px(24.0))
            .text_color(color)
            .child(tool.icon())
            .into_any_element(),
    }
}

fn drawing_tool_tooltip(tool: CanvasDrawingTool) -> &'static str {
    match tool {
        CanvasDrawingTool::Select => "Select",
        CanvasDrawingTool::Pen => "Pen",
        CanvasDrawingTool::Line => "Line",
        CanvasDrawingTool::Arrow => "Arrow",
        CanvasDrawingTool::Rectangle => "Rectangle",
        CanvasDrawingTool::Circle => "Circle",
        CanvasDrawingTool::TextBox => "Text box",
        CanvasDrawingTool::NumberMarker => "Numbered marker",
        CanvasDrawingTool::Eraser => "Eraser",
    }
}

fn history_button(
    id: &'static str,
    label: &'static str,
    tooltip: &'static str,
    enabled: bool,
    undo: bool,
    cx: &mut Context<WorkspaceCanvasView>,
) -> AnyElement {
    div()
        .id(id)
        .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
        .h(px(30.0))
        .w(px(30.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_size(px(22.0))
        .text_color(if enabled {
            theme::text()
        } else {
            theme::text_muted()
        })
        .bg(gpui::transparent_black())
        .hover(move |style| {
            if enabled {
                style.bg(theme::surface_hover()).text_color(theme::text())
            } else {
                style
            }
        })
        .on_click(cx.listener(move |view, _, _, cx| {
            if !enabled {
                return;
            }
            cx.stop_propagation();
            if undo {
                view.undo_canvas_drawing(cx);
            } else {
                view.redo_canvas_drawing(cx);
            }
        }))
        .child(label)
        .into_any_element()
}

fn render_stroke_width_control(
    stroke_width: f32,
    stroke_slider: Entity<SliderState>,
) -> AnyElement {
    h_flex()
        .id("drawing-stroke-width")
        .tooltip(|window, cx| Tooltip::new("Stroke width").build(window, cx))
        .items_center()
        .gap_1()
        .h(px(24.0))
        .w(px(124.0))
        .px_1()
        .text_xs()
        .text_color(theme::text())
        .child(div().w(px(84.0)).child(Slider::new(&stroke_slider)))
        .child(
            div()
                .w(px(28.0))
                .text_right()
                .child(format!("{}px", stroke_width.round() as i32)),
        )
        .into_any_element()
}
