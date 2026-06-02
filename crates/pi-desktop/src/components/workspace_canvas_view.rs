use std::collections::HashMap;

use gpui::{Context, Entity, FocusHandle, IntoElement, Render, Window};
use gpui_component::input::InputState;
use gpui_component::slider::SliderState;

use crate::app::PiDesktop;
use crate::components::{chat_node, workspace_canvas};
use crate::workspace::canvas::{CanvasDrawingTool, WorldPoint, WorldSize};
use crate::workspace::state::WorkspaceTab;

#[derive(Clone)]
pub struct WorkspaceCanvasProps {
    pub tab: WorkspaceTab,
    pub workspace_id: usize,
    pub can_fork: bool,
    pub can_resume: bool,
    pub canvas_size: WorldSize,
    pub text_box_inputs: HashMap<usize, Entity<InputState>>,
    pub chat_node_views: HashMap<usize, Entity<chat_node::ChatNodeView>>,
    pub chat_node_render_revision: u64,
    pub snap_to_grid: bool,
    pub drawing_tools_visible: bool,
    pub active_drawing_tool: CanvasDrawingTool,
    pub drawing_stroke_width: f32,
    pub can_undo_drawing: bool,
    pub can_redo_drawing: bool,
    pub drawing_stroke_slider: Entity<SliderState>,
    pub focus_handle: FocusHandle,
}

pub struct WorkspaceCanvasView {
    app: Entity<PiDesktop>,
    props: WorkspaceCanvasProps,
}

impl WorkspaceCanvasView {
    pub fn new(app: Entity<PiDesktop>, props: WorkspaceCanvasProps) -> Self {
        Self { app, props }
    }

    pub fn sync(&mut self, props: WorkspaceCanvasProps, cx: &mut Context<Self>) -> bool {
        if !self.props_changed(&props) {
            return false;
        }
        self.props = props;
        cx.notify();
        true
    }

    fn props_changed(&self, props: &WorkspaceCanvasProps) -> bool {
        self.props.tab != props.tab
            || self.props.workspace_id != props.workspace_id
            || self.props.can_fork != props.can_fork
            || self.props.can_resume != props.can_resume
            || self.props.canvas_size != props.canvas_size
            || self.props.text_box_inputs != props.text_box_inputs
            || self.props.chat_node_views != props.chat_node_views
            || self.props.chat_node_render_revision != props.chat_node_render_revision
            || self.props.snap_to_grid != props.snap_to_grid
            || self.props.drawing_tools_visible != props.drawing_tools_visible
            || self.props.active_drawing_tool != props.active_drawing_tool
            || self.props.drawing_stroke_width != props.drawing_stroke_width
            || self.props.can_undo_drawing != props.can_undo_drawing
            || self.props.can_redo_drawing != props.can_redo_drawing
            || self.props.drawing_stroke_slider != props.drawing_stroke_slider
    }

    pub fn commit_current_text_box_edit_and_focus_canvas(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.commit_current_text_box_edit_and_focus_canvas(window, cx);
        });
    }

    pub fn start_canvas_drawing(
        &mut self,
        tool: CanvasDrawingTool,
        screen_position: WorldPoint,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.start_canvas_drawing(tool, screen_position, cx);
        });
    }

    pub fn start_canvas_drawing_drag(
        &mut self,
        screen_position: WorldPoint,
        cx: &mut Context<Self>,
    ) -> bool {
        self.update_app(cx, |view, cx| {
            view.start_canvas_drawing_drag(screen_position, cx)
        })
    }

    pub fn start_canvas_drawing_drag_by_index(
        &mut self,
        drawing_index: usize,
        screen_position: WorldPoint,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.start_canvas_drawing_drag_by_index(drawing_index, screen_position, cx);
        });
    }

    pub fn clear_canvas_drawing_selection(&mut self, cx: &mut Context<Self>) -> bool {
        self.update_app(cx, |view, cx| view.clear_canvas_drawing_selection(cx))
    }

    pub fn select_canvas_drawing(
        &mut self,
        workspace_id: usize,
        drawing_index: usize,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.select_canvas_drawing(workspace_id, drawing_index, cx);
        });
    }

    pub fn start_canvas_pan(&mut self, screen_position: WorldPoint, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| {
            view.start_canvas_pan(screen_position, cx);
        });
    }

    pub fn note_canvas_context_position(
        &mut self,
        event: &gpui::MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.note_canvas_context_position(event, cx);
        });
    }

    pub fn end_canvas_drawing(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.end_canvas_drawing(cx));
    }

    pub fn end_canvas_pan(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.end_canvas_pan(cx));
    }

    pub fn update_canvas_drawing(
        &mut self,
        screen_position: WorldPoint,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.update_canvas_drawing(screen_position, window, cx);
        });
    }

    pub fn update_canvas_pan(
        &mut self,
        screen_position: WorldPoint,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.update_canvas_pan(screen_position, window, cx);
        });
    }

    pub fn start_minimap_pan(
        &mut self,
        local_position: WorldPoint,
        viewport_size: WorldSize,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.start_minimap_pan(local_position, viewport_size, cx);
        });
    }

    pub fn update_minimap_pan(
        &mut self,
        local_position: WorldPoint,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.update_app(cx, |view, cx| {
            view.update_minimap_pan(local_position, window, cx)
        })
    }

    pub fn end_minimap_pan(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.end_minimap_pan(cx));
    }

    pub fn zoom_active_canvas_at(
        &mut self,
        screen_position: WorldPoint,
        zoom_factor: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.zoom_active_canvas_at(screen_position, zoom_factor, window, cx);
        });
    }

    pub fn zoom_active_canvas_in(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.zoom_active_canvas_in(cx));
    }

    pub fn zoom_active_canvas_out(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.zoom_active_canvas_out(cx));
    }

    pub fn create_new_session_node(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.create_new_session_node(cx));
    }

    pub fn create_fork_session_node(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.create_fork_session_node(cx));
    }

    pub fn create_resume_session_node(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.create_resume_session_node(cx));
    }

    pub fn focus_pinned_node(
        &mut self,
        workspace_id: usize,
        node_id: usize,
        cx: &mut Context<Self>,
    ) {
        self.update_app(cx, |view, cx| {
            view.focus_pinned_node(workspace_id, node_id, cx);
        });
    }

    pub fn select_drawing_tool(&mut self, tool: CanvasDrawingTool, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.select_drawing_tool(tool, cx));
    }

    pub fn undo_canvas_drawing(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.undo_canvas_drawing(cx));
    }

    pub fn redo_canvas_drawing(&mut self, cx: &mut Context<Self>) {
        self.update_app(cx, |view, cx| view.redo_canvas_drawing(cx));
    }

    fn update_app<R>(
        &self,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut PiDesktop, &mut Context<PiDesktop>) -> R,
    ) -> R {
        self.app.update(cx, update)
    }
}

impl Render for WorkspaceCanvasView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("WorkspaceCanvasView");
        workspace_canvas::workspace_canvas(
            &self.props.tab,
            self.props.workspace_id,
            self.props.can_fork,
            self.props.can_resume,
            self.props.canvas_size,
            &self.props.text_box_inputs,
            &self.props.chat_node_views,
            self.props.snap_to_grid,
            self.props.drawing_tools_visible,
            self.props.active_drawing_tool,
            self.props.drawing_stroke_width,
            self.props.can_undo_drawing,
            self.props.can_redo_drawing,
            self.props.drawing_stroke_slider.clone(),
            self.props.focus_handle.clone(),
            cx,
        )
    }
}
