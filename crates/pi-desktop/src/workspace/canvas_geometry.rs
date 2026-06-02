use super::canvas_model::{CanvasDrawing, WorldPoint};

#[derive(Clone, Debug, PartialEq)]
pub struct DrawingPathGeometry {
    pub commands: Vec<DrawingPathCommand>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawingPathCommand {
    Move(WorldPoint),
    Line(WorldPoint),
    Cubic {
        target: WorldPoint,
        control_a: WorldPoint,
        control_b: WorldPoint,
    },
}

impl DrawingPathGeometry {
    pub fn from_drawing(drawing: &CanvasDrawing) -> Option<Self> {
        match drawing {
            CanvasDrawing::Pen { points } => Self::from_smooth_points(points),
            CanvasDrawing::Line { start, end } | CanvasDrawing::Arrow { start, end } => {
                Some(Self {
                    commands: vec![
                        DrawingPathCommand::Move(*start),
                        DrawingPathCommand::Line(*end),
                    ],
                })
            }
            CanvasDrawing::Rectangle { start, end } | CanvasDrawing::TextBox { start, end, .. } => {
                Some(Self::from_rectangle(*start, *end))
            }
            CanvasDrawing::Circle { .. } | CanvasDrawing::NumberMarker { .. } => None,
        }
    }

    fn from_smooth_points(points: &[WorldPoint]) -> Option<Self> {
        let len = points.len();
        let first = *points.first()?;
        let mut commands = Vec::with_capacity(len.saturating_mul(2));
        commands.push(DrawingPathCommand::Move(first));
        if len == 1 {
            return Some(Self { commands });
        }
        if len == 2 {
            commands.push(DrawingPathCommand::Line(points[1]));
            return Some(Self { commands });
        }

        for index in 0..len - 1 {
            let p0 = if index == 0 {
                points[0]
            } else {
                points[index - 1]
            };
            let p1 = points[index];
            let p2 = points[index + 1];
            let p3 = if index + 2 < len {
                points[index + 2]
            } else {
                points[len - 1]
            };
            commands.push(DrawingPathCommand::Cubic {
                target: p2,
                control_a: WorldPoint::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0),
                control_b: WorldPoint::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0),
            });
        }
        Some(Self { commands })
    }

    fn from_rectangle(start: WorldPoint, end: WorldPoint) -> Self {
        Self {
            commands: vec![
                DrawingPathCommand::Move(start),
                DrawingPathCommand::Line(WorldPoint::new(end.x, start.y)),
                DrawingPathCommand::Line(end),
                DrawingPathCommand::Line(WorldPoint::new(start.x, end.y)),
                DrawingPathCommand::Line(start),
            ],
        }
    }
}
