pub(super) const MINIMAP_WORLD_WIDTH: f32 = 4_000.0;
pub(super) const MINIMAP_WORLD_HEIGHT: f32 = 2_600.0;
pub(super) const MINIMAP_WORLD_LEFT: f32 = -2_000.0;
pub(super) const MINIMAP_WORLD_TOP: f32 = -1_300.0;
pub(super) const MINIMAP_WORLD_PADDING: f32 = 320.0;
pub(super) const SNAP_GRID_SIZE: f32 = 28.0;
pub(super) const DRAWING_ERASE_RADIUS: f32 = 18.0;
pub(super) const DRAWING_HIT_SCREEN_RADIUS: f32 = 12.0;
pub(super) const PEN_SAMPLE_MIN_SCREEN_DISTANCE: f32 = 1.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasDrawingTool {
    Select,
    Pen,
    Line,
    Arrow,
    Rectangle,
    Circle,
    TextBox,
    NumberMarker,
    Eraser,
}

impl CanvasDrawingTool {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Select => "⌖",
            Self::Pen => "✎",
            Self::Line => "／",
            Self::Arrow => "↗",
            Self::Rectangle => "□",
            Self::Circle => "○",
            Self::TextBox => "T",
            Self::NumberMarker => "①",
            Self::Eraser => "⌫",
        }
    }

    pub fn draws(self) -> bool {
        !matches!(self, Self::Select)
    }

    pub(super) fn snaps_to_grid(self) -> bool {
        matches!(
            self,
            Self::Line | Self::Arrow | Self::Rectangle | Self::Circle | Self::TextBox
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldPoint {
    pub x: f32,
    pub y: f32,
}

impl WorldPoint {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

pub fn snap_world_point(point: WorldPoint) -> WorldPoint {
    WorldPoint::new(snap_scalar(point.x), snap_scalar(point.y))
}

fn snap_scalar(value: f32) -> f32 {
    (value / SNAP_GRID_SIZE).round() * SNAP_GRID_SIZE
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldSize {
    pub width: f32,
    pub height: f32,
}

impl WorldSize {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct MinimapWorldBounds {
    pub(super) left: f32,
    pub(super) top: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

impl Default for MinimapWorldBounds {
    fn default() -> Self {
        Self {
            left: MINIMAP_WORLD_LEFT,
            top: MINIMAP_WORLD_TOP,
            width: MINIMAP_WORLD_WIDTH,
            height: MINIMAP_WORLD_HEIGHT,
        }
    }
}

impl MinimapWorldBounds {
    pub(super) fn from_edges(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            width: (right - left).max(1.0),
            height: (bottom - top).max(1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinimapRect {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl MinimapRect {
    pub fn contains(&self, point: WorldPoint) -> bool {
        point.x >= self.left
            && point.x <= self.left + self.width
            && point.y >= self.top
            && point.y <= self.top + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct MinimapPanDrag {
    pub(super) start_local: WorldPoint,
    pub(super) start_pan_x: f32,
    pub(super) start_pan_y: f32,
    pub(super) world_bounds: MinimapWorldBounds,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasViewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

pub(super) fn world_to_minimap_x(
    world_x: f32,
    minimap_size: WorldSize,
    bounds: MinimapWorldBounds,
) -> f32 {
    (world_x - bounds.left) / bounds.width * minimap_size.width
}

pub(super) fn world_to_minimap_y(
    world_y: f32,
    minimap_size: WorldSize,
    bounds: MinimapWorldBounds,
) -> f32 {
    (world_y - bounds.top) / bounds.height * minimap_size.height
}

pub(super) fn minimap_to_world(
    local_position: WorldPoint,
    minimap_size: WorldSize,
    bounds: MinimapWorldBounds,
) -> WorldPoint {
    WorldPoint::new(
        bounds.left + local_position.x / minimap_size.width * bounds.width,
        bounds.top + local_position.y / minimap_size.height * bounds.height,
    )
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }
}

impl CanvasViewport {
    pub const MIN_ZOOM: f32 = 0.35;
    pub const MAX_ZOOM: f32 = 2.5;

    pub fn screen_to_world(&self, point: WorldPoint) -> WorldPoint {
        WorldPoint {
            x: (point.x - self.pan_x) / self.zoom,
            y: (point.y - self.pan_y) / self.zoom,
        }
    }

    pub fn world_to_screen(&self, point: WorldPoint) -> WorldPoint {
        WorldPoint {
            x: point.x * self.zoom + self.pan_x,
            y: point.y * self.zoom + self.pan_y,
        }
    }

    pub fn zoom_in(&mut self) {
        self.zoom_by_at(1.15, WorldPoint::new(0.0, 0.0));
    }

    pub fn zoom_out(&mut self) {
        self.zoom_by_at(1.0 / 1.15, WorldPoint::new(0.0, 0.0));
    }

    pub fn zoom_by_at(&mut self, factor: f32, screen_position: WorldPoint) {
        if factor <= 0.0 {
            return;
        }

        let world_position = self.screen_to_world(screen_position);
        let next_zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        self.zoom = next_zoom;
        self.pan_x = screen_position.x - world_position.x * next_zoom;
        self.pan_y = screen_position.y - world_position.y * next_zoom;
    }

    pub fn center_on_world(&mut self, world_position: WorldPoint, viewport_size: WorldSize) {
        self.pan_x = viewport_size.width / 2.0 - world_position.x * self.zoom;
        self.pan_y = viewport_size.height / 2.0 - world_position.y * self.zoom;
    }

    pub fn visible_world_bounds(
        self,
        canvas_size: WorldSize,
        screen_padding: f32,
    ) -> CanvasDrawingBounds {
        let top_left = self.screen_to_world(WorldPoint::new(0.0, 0.0));
        let bottom_right =
            self.screen_to_world(WorldPoint::new(canvas_size.width, canvas_size.height));
        let padding = screen_padding / self.zoom.max(0.1);
        CanvasDrawingBounds {
            left: top_left.x.min(bottom_right.x) - padding,
            top: top_left.y.min(bottom_right.y) - padding,
            right: top_left.x.max(bottom_right.x) + padding,
            bottom: top_left.y.max(bottom_right.y) + padding,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasNodeRenderLevel {
    Full,
    Shell,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasNodeMaterialization {
    pub node_index: usize,
    pub node_id: usize,
    pub screen_position: WorldPoint,
    pub screen_size: WorldSize,
    pub render_level: CanvasNodeRenderLevel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CanvasPanDrag {
    pub(super) start_screen: WorldPoint,
    pub(super) start_pan_x: f32,
    pub(super) start_pan_y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NodeDrag {
    pub(super) node_id: usize,
    pub(super) start_screen: WorldPoint,
    pub(super) start_position: WorldPoint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NodeResizeDrag {
    pub(super) node_id: usize,
    pub(super) start_screen: WorldPoint,
    pub(super) start_size: WorldSize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasDrawingBounds {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl CanvasDrawingBounds {
    fn from_points(points: impl IntoIterator<Item = WorldPoint>) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next()?;
        let mut left = first.x;
        let mut right = first.x;
        let mut top = first.y;
        let mut bottom = first.y;
        for point in points {
            left = left.min(point.x);
            right = right.max(point.x);
            top = top.min(point.y);
            bottom = bottom.max(point.y);
        }
        Some(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    fn from_corners(start: WorldPoint, end: WorldPoint) -> Self {
        Self {
            left: start.x.min(end.x),
            top: start.y.min(end.y),
            right: start.x.max(end.x),
            bottom: start.y.max(end.y),
        }
    }

    pub fn padded(&self, amount: f32) -> Self {
        Self {
            left: self.left - amount,
            top: self.top - amount,
            right: self.right + amount,
            bottom: self.bottom + amount,
        }
    }

    pub fn contains(&self, point: WorldPoint) -> bool {
        point.x >= self.left
            && point.x <= self.right
            && point.y >= self.top
            && point.y <= self.bottom
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.right >= other.left
            && self.left <= other.right
            && self.bottom >= other.top
            && self.top <= other.bottom
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CanvasDrawing {
    Pen {
        points: Vec<WorldPoint>,
    },
    Line {
        start: WorldPoint,
        end: WorldPoint,
    },
    Arrow {
        start: WorldPoint,
        end: WorldPoint,
    },
    Rectangle {
        start: WorldPoint,
        end: WorldPoint,
    },
    Circle {
        start: WorldPoint,
        end: WorldPoint,
    },
    TextBox {
        start: WorldPoint,
        end: WorldPoint,
        text: String,
    },
    NumberMarker {
        position: WorldPoint,
        number: usize,
    },
}

impl CanvasDrawing {
    pub fn bounds(&self) -> Option<CanvasDrawingBounds> {
        match self {
            Self::Pen { points } => CanvasDrawingBounds::from_points(points.iter().copied()),
            Self::Line { start, end }
            | Self::Arrow { start, end }
            | Self::Rectangle { start, end }
            | Self::Circle { start, end }
            | Self::TextBox { start, end, .. } => {
                Some(CanvasDrawingBounds::from_corners(*start, *end))
            }
            Self::NumberMarker { position, .. } => {
                let radius = DRAWING_ERASE_RADIUS;
                Some(CanvasDrawingBounds {
                    left: position.x - radius,
                    top: position.y - radius,
                    right: position.x + radius,
                    bottom: position.y + radius,
                })
            }
        }
    }

    pub(super) fn primary_anchor(&self) -> Option<WorldPoint> {
        match self {
            Self::Pen { points } => points.first().copied(),
            Self::Line { start, .. }
            | Self::Arrow { start, .. }
            | Self::Rectangle { start, .. }
            | Self::Circle { start, .. }
            | Self::TextBox { start, .. } => Some(*start),
            Self::NumberMarker { position, .. } => Some(*position),
        }
    }

    pub(super) fn translate(&mut self, delta: WorldPoint) {
        let translate_point = |point: &mut WorldPoint| {
            point.x += delta.x;
            point.y += delta.y;
        };
        match self {
            Self::Pen { points } => {
                for point in points {
                    translate_point(point);
                }
            }
            Self::Line { start, end }
            | Self::Arrow { start, end }
            | Self::Rectangle { start, end }
            | Self::Circle { start, end }
            | Self::TextBox { start, end, .. } => {
                translate_point(start);
                translate_point(end);
            }
            Self::NumberMarker { position, .. } => translate_point(position),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct DrawingDrag {
    pub(super) index: usize,
    pub(super) start_screen: WorldPoint,
    pub(super) start_drawing: CanvasDrawing,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum DrawingHistoryAction {
    Added(CanvasDrawing),
    Removed {
        index: usize,
        drawing: CanvasDrawing,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasDrawingDraft {
    pub tool: CanvasDrawingTool,
    pub start: WorldPoint,
    pub current: WorldPoint,
    pub points: Vec<WorldPoint>,
}

pub(super) fn drawing_near_point(drawing: &CanvasDrawing, point: WorldPoint) -> bool {
    drawing_hit_test(drawing, point, DRAWING_ERASE_RADIUS)
}

pub(super) fn drawing_hit_test(drawing: &CanvasDrawing, point: WorldPoint, radius: f32) -> bool {
    match drawing {
        CanvasDrawing::Pen { points } => {
            points
                .windows(2)
                .any(|segment| distance_to_segment(point, segment[0], segment[1]) <= radius)
                || points
                    .iter()
                    .any(|candidate| distance(*candidate, point) <= radius)
        }
        CanvasDrawing::Line { start, end } | CanvasDrawing::Arrow { start, end } => {
            distance_to_segment(point, *start, *end) <= radius
        }
        CanvasDrawing::Rectangle { start, end }
        | CanvasDrawing::Circle { start, end }
        | CanvasDrawing::TextBox { start, end, .. } => {
            CanvasDrawingBounds::from_corners(*start, *end)
                .padded(radius)
                .contains(point)
        }
        CanvasDrawing::NumberMarker { position, .. } => distance(point, *position) <= radius,
    }
}

pub(super) fn distance(a: WorldPoint, b: WorldPoint) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn distance_to_segment(point: WorldPoint, start: WorldPoint, end: WorldPoint) -> f32 {
    let segment_x = end.x - start.x;
    let segment_y = end.y - start.y;
    let length_squared = segment_x * segment_x + segment_y * segment_y;
    if length_squared <= f32::EPSILON {
        return distance(point, start);
    }

    let t = (((point.x - start.x) * segment_x + (point.y - start.y) * segment_y) / length_squared)
        .clamp(0.0, 1.0);
    distance(
        point,
        WorldPoint::new(start.x + t * segment_x, start.y + t * segment_y),
    )
}
