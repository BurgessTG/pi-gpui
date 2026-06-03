use std::collections::HashMap;

pub const SESSION_NODE_DEFAULT_WIDTH: f32 = 560.0;
pub const SESSION_NODE_DEFAULT_HEIGHT: f32 = 420.0;
pub const SESSION_NODE_MIN_WIDTH: f32 = 320.0;
pub const SESSION_NODE_MIN_HEIGHT: f32 = 260.0;
const SESSION_NODE_SHELL_MAX_SCREEN_WIDTH: f32 = 240.0;
const SESSION_NODE_SHELL_MAX_SCREEN_HEIGHT: f32 = 190.0;

pub use super::canvas_geometry::DrawingPathGeometry;
pub use super::canvas_model::{
    CanvasDrawing, CanvasDrawingBounds, CanvasDrawingDraft, CanvasDrawingTool,
    CanvasNodeMaterialization, CanvasNodeRenderLevel, CanvasViewport, MinimapRect, WorldPoint,
    WorldSize, snap_world_point,
};
use super::canvas_model::{
    CanvasPanDrag, DRAWING_ERASE_RADIUS, DRAWING_HIT_SCREEN_RADIUS, DrawingDrag,
    DrawingHistoryAction, MINIMAP_WORLD_PADDING, MinimapPanDrag, MinimapWorldBounds, NodeDrag,
    NodeResizeDrag, PEN_SAMPLE_MIN_SCREEN_DISTANCE, distance, drawing_hit_test, drawing_near_point,
    minimap_to_world, world_to_minimap_x, world_to_minimap_y,
};
pub use super::canvas_session::{SessionNode, SessionNodeMetadata, SessionNodePrimitive};
use super::canvas_spatial::CanvasSpatialIndex;

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasState {
    viewport: CanvasViewport,
    nodes: Vec<SessionNode>,
    node_index: HashMap<usize, usize>,
    node_bounds: Vec<Option<CanvasDrawingBounds>>,
    node_spatial_index: CanvasSpatialIndex,
    drawings: Vec<CanvasDrawing>,
    drawing_bounds: Vec<Option<CanvasDrawingBounds>>,
    drawing_path_geometry: Vec<Option<DrawingPathGeometry>>,
    drawing_spatial_index: CanvasSpatialIndex,
    drawing_draft: Option<CanvasDrawingDraft>,
    undo_stack: Vec<DrawingHistoryAction>,
    redo_stack: Vec<DrawingHistoryAction>,
    next_marker_number: usize,
    next_node_id: usize,
    context_position: Option<WorldPoint>,
    pan_drag: Option<CanvasPanDrag>,
    minimap_drag: Option<MinimapPanDrag>,
    node_drag: Option<NodeDrag>,
    node_resize_drag: Option<NodeResizeDrag>,
    selected_drawing_index: Option<usize>,
    drawing_drag: Option<DrawingDrag>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            viewport: CanvasViewport::default(),
            nodes: Vec::new(),
            node_index: HashMap::new(),
            node_bounds: Vec::new(),
            node_spatial_index: CanvasSpatialIndex::default(),
            drawings: Vec::new(),
            drawing_bounds: Vec::new(),
            drawing_path_geometry: Vec::new(),
            drawing_spatial_index: CanvasSpatialIndex::default(),
            drawing_draft: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            next_marker_number: 1,
            next_node_id: 1,
            context_position: None,
            pan_drag: None,
            minimap_drag: None,
            node_drag: None,
            node_resize_drag: None,
            selected_drawing_index: None,
            drawing_drag: None,
        }
    }
}

fn node_bounds(node: &SessionNode) -> CanvasDrawingBounds {
    CanvasDrawingBounds {
        left: node.position().x,
        top: node.position().y,
        right: node.position().x + node.size().width,
        bottom: node.position().y + node.size().height,
    }
}

fn screen_rect_visible(
    screen_position: WorldPoint,
    screen_size: WorldSize,
    canvas_size: WorldSize,
) -> bool {
    screen_position.x + screen_size.width >= 0.0
        && screen_position.y + screen_size.height >= 0.0
        && screen_position.x <= canvas_size.width
        && screen_position.y <= canvas_size.height
}

impl CanvasState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn viewport(&self) -> CanvasViewport {
        self.viewport
    }

    pub fn nodes(&self) -> &[SessionNode] {
        &self.nodes
    }

    pub fn node(&self, node_id: usize) -> Option<&SessionNode> {
        self.node_index
            .get(&node_id)
            .and_then(|index| self.nodes.get(*index))
    }

    fn node_mut(&mut self, node_id: usize) -> Option<&mut SessionNode> {
        let index = *self.node_index.get(&node_id)?;
        self.nodes.get_mut(index)
    }

    fn rebuild_node_index(&mut self) {
        self.node_index.clear();
        self.node_index.extend(
            self.nodes
                .iter()
                .enumerate()
                .map(|(index, node)| (node.id(), index)),
        );
        self.rebuild_node_spatial_index();
    }

    fn rebuild_node_spatial_index(&mut self) {
        self.node_bounds = self.nodes.iter().map(node_bounds).map(Some).collect();
        self.node_spatial_index = CanvasSpatialIndex::rebuild(&self.node_bounds);
    }

    fn refresh_node_spatial_index(&mut self, node_index: usize) {
        let bounds = self.nodes.get(node_index).map(node_bounds);
        self.node_bounds[node_index] = bounds.clone();
        self.node_spatial_index.set(node_index, bounds.as_ref());
    }

    fn rebuild_drawing_spatial_index(&mut self) {
        self.drawing_spatial_index = CanvasSpatialIndex::rebuild(&self.drawing_bounds);
    }

    pub fn drawings(&self) -> &[CanvasDrawing] {
        &self.drawings
    }

    pub fn drawing_bounds(&self, index: usize) -> Option<&CanvasDrawingBounds> {
        self.drawing_bounds.get(index).and_then(Option::as_ref)
    }

    pub fn drawing_path_geometry(&self, index: usize) -> Option<&DrawingPathGeometry> {
        self.drawing_path_geometry
            .get(index)
            .and_then(Option::as_ref)
    }

    pub fn node_indices_in_bounds(&self, bounds: &CanvasDrawingBounds) -> Vec<usize> {
        self.node_spatial_index
            .query(bounds)
            .into_iter()
            .filter(|index| {
                self.node_bounds
                    .get(*index)
                    .and_then(Option::as_ref)
                    .is_some_and(|node_bounds| node_bounds.intersects(bounds))
            })
            .collect()
    }

    pub fn node_materialization_plan(
        &self,
        canvas_size: WorldSize,
        screen_padding: f32,
    ) -> Vec<CanvasNodeMaterialization> {
        let visible_bounds = self
            .viewport
            .visible_world_bounds(canvas_size, screen_padding);
        self.node_indices_in_bounds(&visible_bounds)
            .into_iter()
            .filter_map(|node_index| {
                let node = self.nodes.get(node_index)?;
                let screen_position = self.viewport.world_to_screen(node.position());
                let node_size = node.size();
                let screen_size = WorldSize::new(
                    node_size.width * self.viewport.zoom,
                    node_size.height * self.viewport.zoom,
                );
                if !screen_rect_visible(screen_position, screen_size, canvas_size) {
                    return None;
                }
                let render_level = if screen_size.width <= SESSION_NODE_SHELL_MAX_SCREEN_WIDTH
                    || screen_size.height <= SESSION_NODE_SHELL_MAX_SCREEN_HEIGHT
                {
                    CanvasNodeRenderLevel::Shell
                } else {
                    CanvasNodeRenderLevel::Full
                };
                Some(CanvasNodeMaterialization {
                    node_index,
                    node_id: node.id(),
                    screen_position,
                    screen_size,
                    render_level,
                })
            })
            .collect()
    }

    pub fn drawing_indices_in_bounds(&self, bounds: &CanvasDrawingBounds) -> Vec<usize> {
        self.drawing_spatial_index
            .query(bounds)
            .into_iter()
            .filter(|index| {
                self.drawing_bounds(*index)
                    .is_some_and(|drawing_bounds| drawing_bounds.intersects(bounds))
            })
            .collect()
    }

    fn drawing_indices_near_point(&self, point: WorldPoint, radius: f32) -> Vec<usize> {
        self.drawing_indices_in_bounds(&CanvasDrawingBounds {
            left: point.x - radius,
            top: point.y - radius,
            right: point.x + radius,
            bottom: point.y + radius,
        })
    }

    pub fn can_undo_drawing(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo_drawing(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn drawing_draft(&self) -> Option<&CanvasDrawingDraft> {
        self.drawing_draft.as_ref()
    }

    pub fn selected_drawing_index(&self) -> Option<usize> {
        self.selected_drawing_index
    }

    pub fn note_context_position(&mut self, screen_position: WorldPoint) {
        self.context_position = Some(self.viewport.screen_to_world(screen_position));
    }

    pub fn start_pan(&mut self, screen_position: WorldPoint) {
        self.drawing_drag = None;
        self.pan_drag = Some(CanvasPanDrag {
            start_screen: screen_position,
            start_pan_x: self.viewport.pan_x,
            start_pan_y: self.viewport.pan_y,
        });
    }

    pub fn update_pan(&mut self, screen_position: WorldPoint) -> bool {
        let Some(drag) = self.pan_drag else {
            return false;
        };

        let next_pan_x = drag.start_pan_x + screen_position.x - drag.start_screen.x;
        let next_pan_y = drag.start_pan_y + screen_position.y - drag.start_screen.y;
        if self.viewport.pan_x == next_pan_x && self.viewport.pan_y == next_pan_y {
            return false;
        }

        self.viewport.pan_x = next_pan_x;
        self.viewport.pan_y = next_pan_y;
        true
    }

    pub fn end_pan(&mut self) -> bool {
        let was_panning = self.pan_drag.is_some();
        self.pan_drag = None;
        was_panning
    }

    fn minimap_world_bounds(&self, viewport_size: WorldSize) -> MinimapWorldBounds {
        let base = MinimapWorldBounds::default();
        let mut left = base.left;
        let mut top = base.top;
        let mut right = base.left + base.width;
        let mut bottom = base.top + base.height;

        let visible_start = self.viewport.screen_to_world(WorldPoint::new(0.0, 0.0));
        let visible_end = self
            .viewport
            .screen_to_world(WorldPoint::new(viewport_size.width, viewport_size.height));
        left = left.min(visible_start.x.min(visible_end.x));
        top = top.min(visible_start.y.min(visible_end.y));
        right = right.max(visible_start.x.max(visible_end.x));
        bottom = bottom.max(visible_start.y.max(visible_end.y));

        for node in &self.nodes {
            left = left.min(node.position().x);
            top = top.min(node.position().y);
            right = right.max(node.position().x + node.size().width);
            bottom = bottom.max(node.position().y + node.size().height);
        }
        for bounds in self.drawing_bounds.iter().flatten() {
            left = left.min(bounds.left);
            top = top.min(bounds.top);
            right = right.max(bounds.right);
            bottom = bottom.max(bounds.bottom);
        }

        MinimapWorldBounds::from_edges(
            left - MINIMAP_WORLD_PADDING,
            top - MINIMAP_WORLD_PADDING,
            right + MINIMAP_WORLD_PADDING,
            bottom + MINIMAP_WORLD_PADDING,
        )
    }

    pub fn minimap_viewport_rect(
        &self,
        minimap_size: WorldSize,
        viewport_size: WorldSize,
    ) -> MinimapRect {
        let bounds = self.minimap_world_bounds(viewport_size);
        let visible_width = viewport_size.width / self.viewport.zoom;
        let visible_height = viewport_size.height / self.viewport.zoom;
        let world_left = -self.viewport.pan_x / self.viewport.zoom;
        let world_top = -self.viewport.pan_y / self.viewport.zoom;

        let inset = 1.0;
        let available_width = (minimap_size.width - inset * 2.0).max(1.0);
        let available_height = (minimap_size.height - inset * 2.0).max(1.0);
        let min_width = 8.0_f32.min(available_width);
        let min_height = 8.0_f32.min(available_height);
        let width = (visible_width / bounds.width * minimap_size.width)
            .max(min_width)
            .min(available_width);
        let height = (visible_height / bounds.height * minimap_size.height)
            .max(min_height)
            .min(available_height);
        let max_left = inset + available_width - width;
        let max_top = inset + available_height - height;

        MinimapRect {
            left: world_to_minimap_x(world_left, minimap_size, bounds).clamp(inset, max_left),
            top: world_to_minimap_y(world_top, minimap_size, bounds).clamp(inset, max_top),
            width,
            height,
        }
    }

    pub fn node_minimap_position(
        &self,
        point: WorldPoint,
        minimap_size: WorldSize,
        viewport_size: WorldSize,
    ) -> WorldPoint {
        let bounds = self.minimap_world_bounds(viewport_size);
        WorldPoint::new(
            world_to_minimap_x(point.x, minimap_size, bounds),
            world_to_minimap_y(point.y, minimap_size, bounds),
        )
    }

    pub fn start_minimap_pan(
        &mut self,
        local_position: WorldPoint,
        minimap_size: WorldSize,
        viewport_size: WorldSize,
    ) -> bool {
        let world_bounds = self.minimap_world_bounds(viewport_size);
        if !self
            .minimap_viewport_rect(minimap_size, viewport_size)
            .contains(local_position)
        {
            let world_position = minimap_to_world(local_position, minimap_size, world_bounds);
            self.viewport.center_on_world(world_position, viewport_size);
        }

        self.minimap_drag = Some(MinimapPanDrag {
            start_local: local_position,
            start_pan_x: self.viewport.pan_x,
            start_pan_y: self.viewport.pan_y,
            world_bounds,
        });
        true
    }

    pub fn update_minimap_pan(
        &mut self,
        local_position: WorldPoint,
        minimap_size: WorldSize,
    ) -> bool {
        let Some(drag) = self.minimap_drag else {
            return false;
        };

        let world_delta_x =
            (local_position.x - drag.start_local.x) / minimap_size.width * drag.world_bounds.width;
        let world_delta_y = (local_position.y - drag.start_local.y) / minimap_size.height
            * drag.world_bounds.height;
        let next_pan_x = drag.start_pan_x - world_delta_x * self.viewport.zoom;
        let next_pan_y = drag.start_pan_y - world_delta_y * self.viewport.zoom;
        if self.viewport.pan_x == next_pan_x && self.viewport.pan_y == next_pan_y {
            return false;
        }

        self.viewport.pan_x = next_pan_x;
        self.viewport.pan_y = next_pan_y;
        true
    }

    pub fn end_minimap_pan(&mut self) -> bool {
        let was_dragging = self.minimap_drag.is_some();
        self.minimap_drag = None;
        was_dragging
    }

    pub fn start_node_drag(&mut self, node_id: usize, screen_position: WorldPoint) -> bool {
        let Some(start_position) = self.node(node_id).map(|node| node.position) else {
            return false;
        };
        self.pan_drag = None;
        self.minimap_drag = None;
        self.node_resize_drag = None;
        self.drawing_drag = None;
        self.node_drag = Some(NodeDrag {
            node_id,
            start_screen: screen_position,
            start_position,
        });
        true
    }

    pub fn update_node_drag(&mut self, screen_position: WorldPoint, snap_to_grid: bool) -> bool {
        let Some(drag) = self.node_drag else {
            return false;
        };
        let zoom = self.viewport.zoom;
        let next_position = WorldPoint::new(
            drag.start_position.x + (screen_position.x - drag.start_screen.x) / zoom,
            drag.start_position.y + (screen_position.y - drag.start_screen.y) / zoom,
        );
        let next_position = if snap_to_grid {
            snap_world_point(next_position)
        } else {
            next_position
        };
        let Some(node_index) = self.node_index.get(&drag.node_id).copied() else {
            self.node_drag = None;
            return false;
        };
        if self.nodes[node_index].position == next_position {
            return false;
        }
        self.nodes[node_index].position = next_position;
        self.refresh_node_spatial_index(node_index);
        true
    }

    pub fn end_node_drag(&mut self) -> bool {
        let was_dragging = self.node_drag.is_some();
        self.node_drag = None;
        was_dragging
    }

    pub fn start_node_resize(&mut self, node_id: usize, screen_position: WorldPoint) -> bool {
        let Some(start_size) = self.node(node_id).map(|node| node.size) else {
            return false;
        };
        self.pan_drag = None;
        self.minimap_drag = None;
        self.node_drag = None;
        self.drawing_drag = None;
        self.node_resize_drag = Some(NodeResizeDrag {
            node_id,
            start_screen: screen_position,
            start_size,
        });
        true
    }

    pub fn update_node_resize(&mut self, screen_position: WorldPoint) -> bool {
        let Some(drag) = self.node_resize_drag else {
            return false;
        };
        let zoom = self.viewport.zoom.max(0.1);
        let Some(node_index) = self.node_index.get(&drag.node_id).copied() else {
            self.node_resize_drag = None;
            return false;
        };

        let next_size = WorldSize::new(
            (drag.start_size.width + (screen_position.x - drag.start_screen.x) / zoom)
                .max(SESSION_NODE_MIN_WIDTH),
            (drag.start_size.height + (screen_position.y - drag.start_screen.y) / zoom)
                .max(SESSION_NODE_MIN_HEIGHT),
        );
        if self.nodes[node_index].size == next_size {
            return false;
        }
        self.nodes[node_index].size = next_size;
        self.refresh_node_spatial_index(node_index);
        true
    }

    pub fn end_node_resize(&mut self) -> bool {
        let was_dragging = self.node_resize_drag.is_some();
        self.node_resize_drag = None;
        was_dragging
    }

    pub fn select_drawing(&mut self, index: usize) -> bool {
        if index >= self.drawings.len() {
            return false;
        }
        if self.selected_drawing_index == Some(index) {
            return false;
        }
        self.selected_drawing_index = Some(index);
        true
    }

    pub fn clear_drawing_selection(&mut self) -> bool {
        let had_selection = self.selected_drawing_index.take().is_some();
        self.drawing_drag = None;
        had_selection
    }

    pub fn update_text_box_text(&mut self, index: usize, text: String) -> bool {
        let Some(CanvasDrawing::TextBox { text: value, .. }) = self.drawings.get_mut(index) else {
            return false;
        };
        if *value == text {
            return false;
        }
        *value = text;
        self.redo_stack.clear();
        true
    }

    pub fn start_drawing_drag(&mut self, index: usize, screen_position: WorldPoint) -> bool {
        let Some(drawing) = self.drawings.get(index).cloned() else {
            return false;
        };

        self.pan_drag = None;
        self.minimap_drag = None;
        self.node_drag = None;
        self.node_resize_drag = None;
        self.drawing_draft = None;
        self.selected_drawing_index = Some(index);
        self.drawing_drag = Some(DrawingDrag {
            index,
            start_screen: screen_position,
            start_drawing: drawing,
        });
        true
    }

    pub fn start_drawing_drag_at(&mut self, screen_position: WorldPoint) -> bool {
        let world_position = self.viewport.screen_to_world(screen_position);
        let Some(index) = self.drawing_index_at(world_position) else {
            return false;
        };
        self.start_drawing_drag(index, screen_position)
    }

    pub fn update_drawing_drag(&mut self, screen_position: WorldPoint, snap_to_grid: bool) -> bool {
        let Some(drag) = self.drawing_drag.clone() else {
            return false;
        };
        if drag.index >= self.drawings.len() {
            self.drawing_drag = None;
            self.selected_drawing_index = None;
            return false;
        }

        let mut delta = WorldPoint::new(
            (screen_position.x - drag.start_screen.x) / self.viewport.zoom,
            (screen_position.y - drag.start_screen.y) / self.viewport.zoom,
        );
        if snap_to_grid && let Some(anchor) = drag.start_drawing.primary_anchor() {
            let snapped_anchor =
                snap_world_point(WorldPoint::new(anchor.x + delta.x, anchor.y + delta.y));
            delta = WorldPoint::new(snapped_anchor.x - anchor.x, snapped_anchor.y - anchor.y);
        }

        let mut next_drawing = drag.start_drawing;
        next_drawing.translate(delta);
        if self.drawings[drag.index] == next_drawing {
            return false;
        }
        self.set_drawing_at(drag.index, next_drawing);
        self.redo_stack.clear();
        true
    }

    pub fn end_drawing_drag(&mut self) -> bool {
        let was_dragging = self.drawing_drag.is_some();
        self.drawing_drag = None;
        was_dragging
    }

    pub fn start_drawing(
        &mut self,
        tool: CanvasDrawingTool,
        screen_position: WorldPoint,
        snap_to_grid: bool,
    ) -> bool {
        if !tool.draws() {
            return false;
        }
        self.pan_drag = None;
        self.minimap_drag = None;
        self.node_drag = None;
        self.node_resize_drag = None;
        self.drawing_drag = None;

        let point = self.drawing_world_point(tool, screen_position, snap_to_grid);
        if tool == CanvasDrawingTool::Eraser {
            return self.erase_drawing_at(point);
        }
        if tool == CanvasDrawingTool::NumberMarker {
            let drawing = CanvasDrawing::NumberMarker {
                position: point,
                number: self.next_marker_number,
            };
            self.next_marker_number += 1;
            self.push_drawing(drawing);
            return true;
        }

        self.drawing_draft = Some(CanvasDrawingDraft {
            tool,
            start: point,
            current: point,
            points: vec![point],
        });
        true
    }

    pub fn update_drawing(&mut self, screen_position: WorldPoint, snap_to_grid: bool) -> bool {
        let Some(tool) = self.drawing_draft.as_ref().map(|draft| draft.tool) else {
            return false;
        };
        let point = self.drawing_world_point(tool, screen_position, snap_to_grid);
        let zoom = self.viewport.zoom;
        let Some(draft) = &mut self.drawing_draft else {
            return false;
        };
        if draft.current == point {
            return false;
        }
        if draft.tool == CanvasDrawingTool::Pen
            && distance(draft.current, point) * zoom < PEN_SAMPLE_MIN_SCREEN_DISTANCE
        {
            return false;
        }
        draft.current = point;
        if draft.tool == CanvasDrawingTool::Pen {
            draft.points.push(point);
        }
        true
    }

    pub fn end_drawing(&mut self) -> bool {
        let Some(draft) = self.drawing_draft.take() else {
            return false;
        };
        match draft.tool {
            CanvasDrawingTool::Pen if draft.points.len() > 1 => {
                self.push_drawing(CanvasDrawing::Pen {
                    points: draft.points,
                });
                true
            }
            CanvasDrawingTool::Line if draft.start != draft.current => {
                self.push_drawing(CanvasDrawing::Line {
                    start: draft.start,
                    end: draft.current,
                });
                true
            }
            CanvasDrawingTool::Arrow if draft.start != draft.current => {
                self.push_drawing(CanvasDrawing::Arrow {
                    start: draft.start,
                    end: draft.current,
                });
                true
            }
            CanvasDrawingTool::Rectangle if draft.start != draft.current => {
                self.push_drawing(CanvasDrawing::Rectangle {
                    start: draft.start,
                    end: draft.current,
                });
                true
            }
            CanvasDrawingTool::Circle if draft.start != draft.current => {
                self.push_drawing(CanvasDrawing::Circle {
                    start: draft.start,
                    end: draft.current,
                });
                true
            }
            CanvasDrawingTool::TextBox if draft.start != draft.current => {
                self.push_drawing(CanvasDrawing::TextBox {
                    start: draft.start,
                    end: draft.current,
                    text: String::new(),
                });
                true
            }
            _ => false,
        }
    }

    fn drawing_index_at(&self, point: WorldPoint) -> Option<usize> {
        let hit_radius = DRAWING_HIT_SCREEN_RADIUS / self.viewport.zoom.max(0.1);
        self.drawing_indices_near_point(point, hit_radius)
            .into_iter()
            .rev()
            .find(|index| {
                self.drawings
                    .get(*index)
                    .is_some_and(|drawing| drawing_hit_test(drawing, point, hit_radius))
            })
    }

    fn drawing_world_point(
        &self,
        tool: CanvasDrawingTool,
        screen_position: WorldPoint,
        snap_to_grid: bool,
    ) -> WorldPoint {
        let point = self.viewport.screen_to_world(screen_position);
        if snap_to_grid && tool.snaps_to_grid() {
            snap_world_point(point)
        } else {
            point
        }
    }

    fn erase_drawing_at(&mut self, point: WorldPoint) -> bool {
        let Some(index) = self
            .drawing_indices_near_point(point, DRAWING_ERASE_RADIUS)
            .into_iter()
            .rev()
            .find(|index| {
                self.drawings
                    .get(*index)
                    .is_some_and(|drawing| drawing_near_point(drawing, point))
            })
        else {
            return false;
        };
        let drawing = self.remove_drawing_at(index);
        self.undo_stack
            .push(DrawingHistoryAction::Removed { index, drawing });
        self.redo_stack.clear();
        true
    }

    fn push_drawing(&mut self, drawing: CanvasDrawing) {
        let index = self.drawings.len();
        let bounds = drawing.bounds();
        self.drawing_bounds.push(bounds.clone());
        self.drawing_path_geometry
            .push(DrawingPathGeometry::from_drawing(&drawing));
        self.drawing_spatial_index.push(index, bounds.as_ref());
        self.drawings.push(drawing.clone());
        self.undo_stack.push(DrawingHistoryAction::Added(drawing));
        self.redo_stack.clear();
    }

    fn set_drawing_at(&mut self, index: usize, drawing: CanvasDrawing) {
        let bounds = drawing.bounds();
        self.drawing_bounds[index] = bounds.clone();
        self.drawing_path_geometry[index] = DrawingPathGeometry::from_drawing(&drawing);
        self.drawing_spatial_index.set(index, bounds.as_ref());
        self.drawings[index] = drawing;
    }

    fn insert_drawing_at(&mut self, index: usize, drawing: CanvasDrawing) {
        self.drawing_bounds.insert(index, drawing.bounds());
        self.drawing_path_geometry
            .insert(index, DrawingPathGeometry::from_drawing(&drawing));
        self.drawings.insert(index, drawing);
        self.rebuild_drawing_spatial_index();
    }

    fn remove_drawing_at(&mut self, index: usize) -> CanvasDrawing {
        self.drawing_bounds.remove(index);
        self.drawing_path_geometry.remove(index);
        let drawing = self.drawings.remove(index);
        self.rebuild_drawing_spatial_index();
        self.drawing_drag = None;
        self.selected_drawing_index = match self.selected_drawing_index {
            Some(selected) if selected == index => None,
            Some(selected) if selected > index => Some(selected - 1),
            other => other,
        };
        drawing
    }

    pub fn undo_drawing(&mut self) -> bool {
        let Some(action) = self.undo_stack.pop() else {
            return false;
        };
        match &action {
            DrawingHistoryAction::Added(_) => {
                if !self.drawings.is_empty() {
                    self.remove_drawing_at(self.drawings.len() - 1);
                }
            }
            DrawingHistoryAction::Removed { index, drawing } => {
                let index = (*index).min(self.drawings.len());
                self.insert_drawing_at(index, drawing.clone());
            }
        }
        self.redo_stack.push(action);
        true
    }

    pub fn redo_drawing(&mut self) -> bool {
        let Some(action) = self.redo_stack.pop() else {
            return false;
        };
        match &action {
            DrawingHistoryAction::Added(drawing) => {
                let index = self.drawings.len();
                let bounds = drawing.bounds();
                self.drawing_bounds.push(bounds.clone());
                self.drawing_path_geometry
                    .push(DrawingPathGeometry::from_drawing(drawing));
                self.drawing_spatial_index.push(index, bounds.as_ref());
                self.drawings.push(drawing.clone());
            }
            DrawingHistoryAction::Removed { index, .. } => {
                if *index < self.drawings.len() {
                    self.remove_drawing_at(*index);
                }
            }
        }
        self.undo_stack.push(action);
        true
    }

    pub fn zoom_in(&mut self) {
        self.viewport.zoom_in();
    }

    pub fn zoom_out(&mut self) {
        self.viewport.zoom_out();
    }

    pub fn zoom_by_at(&mut self, factor: f32, screen_position: WorldPoint) {
        self.viewport.zoom_by_at(factor, screen_position);
    }

    pub fn add_session_node(
        &mut self,
        primitive: SessionNodePrimitive,
        metadata: SessionNodeMetadata,
        snap_to_grid: bool,
    ) -> usize {
        let id = self.next_node_id;
        self.next_node_id += 1;
        let position = self
            .context_position
            .take()
            .unwrap_or_else(|| WorldPoint::new(360.0, 220.0));
        let position = if snap_to_grid {
            snap_world_point(position)
        } else {
            position
        };
        let node_index = self.nodes.len();
        self.node_index.insert(id, node_index);
        self.nodes.push(SessionNode {
            id,
            primitive,
            position,
            size: WorldSize::new(SESSION_NODE_DEFAULT_WIDTH, SESSION_NODE_DEFAULT_HEIGHT),
            metadata,
        });
        let bounds = self.nodes.get(node_index).map(node_bounds);
        self.node_bounds.push(bounds.clone());
        self.node_spatial_index.push(node_index, bounds.as_ref());
        id
    }

    pub fn remove_session_node(&mut self, node_id: usize) -> bool {
        let Some(index) = self.node_index.get(&node_id).copied() else {
            return false;
        };
        self.nodes.remove(index);
        self.rebuild_node_index();
        if self.node_drag.is_some_and(|drag| drag.node_id == node_id) {
            self.node_drag = None;
        }
        if self
            .node_resize_drag
            .is_some_and(|drag| drag.node_id == node_id)
        {
            self.node_resize_drag = None;
        }
        true
    }

    pub fn update_session_node_metadata(
        &mut self,
        node_id: usize,
        metadata: SessionNodeMetadata,
    ) -> bool {
        let Some(node) = self.node_mut(node_id) else {
            return false;
        };
        node.metadata = metadata;
        true
    }

    pub fn sync_session_metadata(&mut self, metadata: &SessionNodeMetadata) -> bool {
        let mut changed = false;
        for node in &mut self.nodes {
            let matches_session_id = metadata.session_id.is_some()
                && node.metadata.session_id.as_ref() == metadata.session_id.as_ref();
            let matches_session_file = metadata.session_file.is_some()
                && node.metadata.session_file.as_ref() == metadata.session_file.as_ref();
            if matches_session_id || matches_session_file {
                node.metadata = metadata.clone();
                changed = true;
            }
        }
        changed
    }
}
