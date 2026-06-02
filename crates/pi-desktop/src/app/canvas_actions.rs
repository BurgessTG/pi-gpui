use super::*;

impl PiDesktop {
    pub(crate) fn note_canvas_context_position(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        let screen_position = workspace_canvas::screen_point_from_event(event);
        self.workspace_state
            .note_context_position(canvas_local_point(screen_position));
        cx.notify();
    }

    pub(crate) fn start_canvas_pan(&mut self, screen_position: WorldPoint, cx: &mut Context<Self>) {
        if self.drawing_tools_visible && self.active_drawing_tool.draws() {
            return;
        }
        if self
            .workspace_state
            .start_active_canvas_pan(canvas_local_point(screen_position))
        {
            cx.notify();
        }
    }

    pub(crate) fn start_canvas_drawing_drag(
        &mut self,
        screen_position: WorldPoint,
        cx: &mut Context<Self>,
    ) -> bool {
        self.commit_current_text_box_edit(cx);
        if self
            .workspace_state
            .start_active_drawing_drag_at(canvas_local_point(screen_position))
        {
            cx.notify();
            return true;
        }
        false
    }

    pub(crate) fn start_canvas_drawing_drag_by_index(
        &mut self,
        drawing_index: usize,
        screen_position: WorldPoint,
        cx: &mut Context<Self>,
    ) {
        self.commit_current_text_box_edit(cx);
        if self
            .workspace_state
            .start_active_drawing_drag(drawing_index, canvas_local_point(screen_position))
        {
            cx.notify();
        }
    }

    pub(crate) fn clear_canvas_drawing_selection(&mut self, cx: &mut Context<Self>) -> bool {
        if self.workspace_state.clear_active_drawing_selection() {
            cx.notify();
            return true;
        }
        false
    }

    pub(crate) fn select_canvas_drawing(
        &mut self,
        workspace_id: usize,
        drawing_index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_index) = self.workspace_index_for_id(workspace_id) else {
            return;
        };
        if self
            .workspace_state
            .select_drawing(workspace_index, drawing_index)
        {
            cx.notify();
        }
    }

    pub(crate) fn start_text_box_edit(
        &mut self,
        workspace_id: usize,
        drawing_index: usize,
        cx: &mut Context<Self>,
    ) {
        let key = (workspace_id, drawing_index);
        if self.editing_text_box != Some(key) {
            self.commit_current_text_box_edit(cx);
        }
        self.editing_text_box = Some(key);
        self.select_canvas_drawing(workspace_id, drawing_index, cx);
        cx.notify();
    }

    pub(crate) fn commit_text_box_edit(
        &mut self,
        workspace_id: usize,
        drawing_index: usize,
        input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_index) = self.workspace_index_for_id(workspace_id) else {
            return;
        };
        let key = (workspace_id, drawing_index);
        let text = input.read(cx).value().to_string();
        let updated =
            self.workspace_state
                .update_text_box_text(workspace_index, drawing_index, text);
        let was_editing = self.editing_text_box == Some(key);
        if was_editing {
            self.editing_text_box = None;
        }
        if updated || was_editing {
            cx.notify();
        }
    }

    pub(crate) fn commit_text_box_edit_from_secondary_enter(
        &mut self,
        workspace_id: usize,
        drawing_index: usize,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        remove_auto_inserted_enter_newline(input, window, cx);
        self.commit_text_box_edit(workspace_id, drawing_index, input, cx);
        self.focus_handle.focus(window);
    }

    pub(crate) fn commit_current_text_box_edit(&mut self, cx: &mut Context<Self>) -> bool {
        let Some((workspace_id, drawing_index)) = self.editing_text_box else {
            return false;
        };
        let Some(input) = self
            .text_box_inputs
            .get(&(workspace_id, drawing_index))
            .cloned()
        else {
            self.editing_text_box = None;
            cx.notify();
            return true;
        };
        self.commit_text_box_edit(workspace_id, drawing_index, &input, cx);
        true
    }

    pub(crate) fn commit_current_text_box_edit_and_focus_canvas(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.commit_current_text_box_edit(cx) {
            self.focus_handle.focus(window);
        }
    }

    pub(crate) fn start_canvas_drawing(
        &mut self,
        tool: CanvasDrawingTool,
        screen_position: WorldPoint,
        cx: &mut Context<Self>,
    ) {
        if self.workspace_state.start_active_drawing(
            tool,
            canvas_local_point(screen_position),
            self.snap_to_grid,
        ) {
            cx.notify();
        }
    }

    pub(crate) fn update_canvas_drawing(
        &mut self,
        screen_position: WorldPoint,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .workspace_state
            .update_active_drawing(canvas_local_point(screen_position), self.snap_to_grid)
        {
            cx.stop_propagation();
            self.request_canvas_frame_render(window, cx);
        }
    }

    pub(crate) fn end_canvas_drawing(&mut self, cx: &mut Context<Self>) {
        if self.workspace_state.end_active_drawing() {
            cx.notify();
        }
    }

    pub(crate) fn update_canvas_pan(
        &mut self,
        screen_position: WorldPoint,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .workspace_state
            .update_active_drawing_drag(canvas_local_point(screen_position), self.snap_to_grid)
        {
            cx.stop_propagation();
            self.request_canvas_frame_render(window, cx);
            return;
        }
        if self
            .workspace_state
            .update_active_node_resize(canvas_local_point(screen_position))
        {
            cx.stop_propagation();
            self.request_canvas_frame_render(window, cx);
            return;
        }
        if self
            .workspace_state
            .update_active_node_drag(canvas_local_point(screen_position), self.snap_to_grid)
        {
            cx.stop_propagation();
            self.request_canvas_frame_render(window, cx);
            return;
        }
        if self
            .workspace_state
            .update_active_canvas_pan(canvas_local_point(screen_position))
        {
            cx.stop_propagation();
            self.request_canvas_frame_render(window, cx);
        }
    }

    pub(crate) fn end_canvas_pan(&mut self, cx: &mut Context<Self>) {
        let drawing_drag_ended = self.workspace_state.end_active_drawing_drag();
        let node_resize_ended = self.workspace_state.end_active_node_resize();
        let node_drag_ended = self.workspace_state.end_active_node_drag();
        let pan_ended = self.workspace_state.end_active_canvas_pan();
        if drawing_drag_ended || node_resize_ended || node_drag_ended || pan_ended {
            cx.notify();
        }
    }

    pub(crate) fn start_node_drag(
        &mut self,
        node_id: usize,
        screen_position: WorldPoint,
        cx: &mut Context<Self>,
    ) {
        self.commit_current_text_box_edit(cx);
        if self
            .workspace_state
            .start_active_node_drag(node_id, canvas_local_point(screen_position))
        {
            cx.notify();
        }
    }

    pub(crate) fn start_node_resize(
        &mut self,
        node_id: usize,
        screen_position: WorldPoint,
        cx: &mut Context<Self>,
    ) {
        self.commit_current_text_box_edit(cx);
        if self
            .workspace_state
            .start_active_node_resize(node_id, canvas_local_point(screen_position))
        {
            cx.notify();
        }
    }

    pub(crate) fn close_session_node(
        &mut self,
        workspace_id: usize,
        node_id: usize,
        cx: &mut Context<Self>,
    ) {
        if self.remove_session_node_locally(workspace_id, node_id) {
            self.status = "Closed session node.".into();
            cx.notify();
        }
    }

    pub(super) fn remove_session_node_locally(
        &mut self,
        workspace_id: usize,
        node_id: usize,
    ) -> bool {
        let Some(workspace_index) = self.workspace_index_for_id(workspace_id) else {
            self.remove_session_node_ui_state(workspace_id, node_id);
            return false;
        };
        if !self
            .workspace_state
            .remove_session_node(workspace_index, node_id)
        {
            self.remove_session_node_ui_state(workspace_id, node_id);
            return false;
        }

        self.remove_session_node_ui_state(workspace_id, node_id);
        true
    }

    pub(crate) fn start_minimap_pan(
        &mut self,
        local_position: WorldPoint,
        viewport_size: WorldSize,
        cx: &mut Context<Self>,
    ) {
        self.workspace_state.end_active_canvas_pan();
        if self.workspace_state.start_active_minimap_pan(
            local_position,
            minimap_size(),
            viewport_size,
        ) {
            cx.notify();
        }
    }

    pub(crate) fn update_minimap_pan(
        &mut self,
        local_position: WorldPoint,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self
            .workspace_state
            .update_active_minimap_pan(local_position, minimap_size())
        {
            self.request_canvas_frame_render(window, cx);
            return true;
        }

        false
    }

    pub(crate) fn end_minimap_pan(&mut self, cx: &mut Context<Self>) {
        if self.workspace_state.end_active_minimap_pan() {
            cx.notify();
        }
    }

    pub(crate) fn zoom_active_canvas_at(
        &mut self,
        screen_position: WorldPoint,
        zoom_factor: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .workspace_state
            .zoom_active_canvas_by_at(zoom_factor, canvas_local_point(screen_position))
        {
            self.request_canvas_frame_render(window, cx);
        }
    }

    pub(crate) fn zoom_active_canvas_in(&mut self, cx: &mut Context<Self>) {
        if self.workspace_state.zoom_active_canvas_in() {
            cx.notify();
        }
    }

    pub(crate) fn zoom_active_canvas_out(&mut self, cx: &mut Context<Self>) {
        if self.workspace_state.zoom_active_canvas_out() {
            cx.notify();
        }
    }

    fn request_canvas_frame_render(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.canvas_render_scheduled {
            return;
        }
        self.canvas_render_scheduled = true;
        let view = cx.entity().clone();
        window.on_next_frame(move |_, cx| {
            view.update(cx, |view, cx| {
                view.canvas_render_scheduled = false;
                cx.notify();
            });
        });
    }
}
