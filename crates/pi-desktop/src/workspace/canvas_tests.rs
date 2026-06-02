use super::canvas::{
    CanvasDrawing, CanvasDrawingTool, CanvasState, CanvasViewport, SESSION_NODE_DEFAULT_HEIGHT,
    SESSION_NODE_DEFAULT_WIDTH, SESSION_NODE_MIN_HEIGHT, SESSION_NODE_MIN_WIDTH,
    SessionNodeMetadata, SessionNodePrimitive, WorldPoint, WorldSize,
};

fn empty_metadata() -> SessionNodeMetadata {
    SessionNodeMetadata {
        session_id: None,
        session_name: None,
        session_file: None,
        cwd: None,
        message_count: 0,
    }
}

#[test]
fn viewport_round_trips_world_and_screen_points() {
    let viewport = CanvasViewport {
        pan_x: 24.0,
        pan_y: -12.0,
        zoom: 1.5,
    };
    let world = WorldPoint::new(40.0, 80.0);
    let screen = viewport.world_to_screen(world);

    assert_eq!(viewport.screen_to_world(screen), world);
}

#[test]
fn zoom_is_clamped_to_canvas_bounds() {
    let mut viewport = CanvasViewport::default();
    for _ in 0..100 {
        viewport.zoom_out();
    }
    assert_eq!(viewport.zoom, CanvasViewport::MIN_ZOOM);

    for _ in 0..100 {
        viewport.zoom_in();
    }
    assert_eq!(viewport.zoom, CanvasViewport::MAX_ZOOM);
}

#[test]
fn zoom_at_screen_position_preserves_world_point_under_cursor() {
    let mut viewport = CanvasViewport {
        pan_x: 20.0,
        pan_y: 10.0,
        zoom: 1.0,
    };
    let cursor = WorldPoint::new(140.0, 90.0);
    let before = viewport.screen_to_world(cursor);

    viewport.zoom_by_at(1.4, cursor);

    assert_eq!(viewport.screen_to_world(cursor), before);
}

#[test]
fn panning_uses_left_drag_delta() {
    let mut canvas = CanvasState::new();
    canvas.start_pan(WorldPoint::new(100.0, 120.0));

    assert!(canvas.update_pan(WorldPoint::new(130.0, 80.0)));
    assert_eq!(canvas.viewport().pan_x, 30.0);
    assert_eq!(canvas.viewport().pan_y, -40.0);

    canvas.end_pan();
    assert!(!canvas.update_pan(WorldPoint::new(200.0, 200.0)));
}

#[test]
fn snap_grid_snaps_session_node_creation_and_drag() {
    let mut canvas = CanvasState::new();
    canvas.note_context_position(WorldPoint::new(43.0, 57.0));
    let node_id = canvas.add_session_node(SessionNodePrimitive::NewSession, empty_metadata(), true);

    assert_eq!(canvas.nodes()[0].position(), WorldPoint::new(56.0, 56.0));

    assert!(canvas.start_node_drag(node_id, WorldPoint::new(0.0, 0.0)));
    assert!(canvas.update_node_drag(WorldPoint::new(31.0, 17.0), true));
    assert_eq!(canvas.nodes()[0].position(), WorldPoint::new(84.0, 84.0));
}

#[test]
fn minimap_viewport_rect_stays_nested_inside_outer_box() {
    let canvas = CanvasState::new();
    let minimap_size = WorldSize::new(100.0, 60.0);
    let oversized_viewport = WorldSize::new(10_000.0, 8_000.0);

    let rect = canvas.minimap_viewport_rect(minimap_size, oversized_viewport);

    assert!(rect.left >= 1.0);
    assert!(rect.top >= 1.0);
    assert!(rect.left + rect.width <= minimap_size.width - 1.0);
    assert!(rect.top + rect.height <= minimap_size.height - 1.0);
}

#[test]
fn minimap_click_jumps_and_drag_controls_viewport() {
    let mut canvas = CanvasState::new();
    let minimap_size = WorldSize::new(100.0, 100.0);
    let viewport_size = WorldSize::new(1_000.0, 500.0);

    assert!(canvas.start_minimap_pan(WorldPoint::new(25.0, 25.0), minimap_size, viewport_size,));
    assert_eq!(canvas.viewport().pan_x, 1_500.0);
    assert_eq!(canvas.viewport().pan_y, 900.0);

    assert!(canvas.update_minimap_pan(WorldPoint::new(35.0, 25.0), minimap_size));
    assert_eq!(canvas.viewport().pan_x, 1_100.0);
    assert_eq!(canvas.viewport().pan_y, 900.0);

    assert!(canvas.end_minimap_pan());
    assert!(!canvas.update_minimap_pan(WorldPoint::new(40.0, 40.0), minimap_size));
}

#[test]
fn node_resize_changes_screen_size_and_clamps_minimums() {
    let mut canvas = CanvasState::new();
    canvas.zoom_by_at(2.0, WorldPoint::new(0.0, 0.0));
    let node_id =
        canvas.add_session_node(SessionNodePrimitive::NewSession, empty_metadata(), false);

    assert!(canvas.start_node_resize(node_id, WorldPoint::new(100.0, 100.0)));
    assert!(canvas.update_node_resize(WorldPoint::new(140.0, 160.0)));
    assert_eq!(
        canvas.nodes()[0].size(),
        WorldSize::new(
            SESSION_NODE_DEFAULT_WIDTH + 40.0,
            SESSION_NODE_DEFAULT_HEIGHT + 60.0
        )
    );

    assert!(canvas.update_node_resize(WorldPoint::new(-1_000.0, -1_000.0)));
    assert_eq!(
        canvas.nodes()[0].size(),
        WorldSize::new(SESSION_NODE_MIN_WIDTH, SESSION_NODE_MIN_HEIGHT)
    );
    assert!(canvas.end_node_resize());
}

#[test]
fn removing_session_node_clears_it_from_canvas() {
    let mut canvas = CanvasState::new();
    let node_id =
        canvas.add_session_node(SessionNodePrimitive::NewSession, empty_metadata(), false);

    assert!(canvas.remove_session_node(node_id));
    assert!(canvas.nodes().is_empty());
    assert!(!canvas.remove_session_node(node_id));
}

#[test]
fn drawing_drag_moves_selected_shape() {
    let mut canvas = CanvasState::new();
    assert!(canvas.start_drawing(
        CanvasDrawingTool::Rectangle,
        WorldPoint::new(10.0, 10.0),
        false,
    ));
    assert!(canvas.update_drawing(WorldPoint::new(60.0, 40.0), false));
    assert!(canvas.end_drawing());

    assert_eq!(canvas.selected_drawing_index(), None);
    assert!(canvas.start_drawing_drag_at(WorldPoint::new(20.0, 20.0)));
    assert!(canvas.update_drawing_drag(WorldPoint::new(35.0, 45.0), false));

    assert_eq!(canvas.selected_drawing_index(), Some(0));
    assert_eq!(
        canvas.drawings().first(),
        Some(&CanvasDrawing::Rectangle {
            start: WorldPoint::new(25.0, 35.0),
            end: WorldPoint::new(75.0, 65.0),
        })
    );
    assert!(canvas.end_drawing_drag());
}

#[test]
fn text_box_drag_preserves_text() {
    let mut canvas = CanvasState::new();
    assert!(canvas.start_drawing(
        CanvasDrawingTool::TextBox,
        WorldPoint::new(100.0, 100.0),
        false,
    ));
    assert!(canvas.update_drawing(WorldPoint::new(200.0, 160.0), false));
    assert!(canvas.end_drawing());
    assert!(canvas.update_text_box_text(0, "hello canvas".to_owned()));

    assert!(canvas.start_drawing_drag(0, WorldPoint::new(120.0, 120.0)));
    assert!(canvas.update_drawing_drag(WorldPoint::new(150.0, 150.0), false));

    assert_eq!(
        canvas.drawings().first(),
        Some(&CanvasDrawing::TextBox {
            start: WorldPoint::new(130.0, 130.0),
            end: WorldPoint::new(230.0, 190.0),
            text: "hello canvas".to_owned(),
        })
    );
}

#[test]
fn session_nodes_use_last_context_position_then_fallback() {
    let mut canvas = CanvasState::new();
    canvas.note_context_position(WorldPoint::new(90.0, 45.0));

    let first = canvas.add_session_node(
        SessionNodePrimitive::NewSession,
        SessionNodeMetadata {
            session_id: Some("abc".to_owned()),
            session_name: Some("Saved Chat".to_owned()),
            session_file: None,
            cwd: None,
            message_count: 0,
        },
        false,
    );
    let second = canvas.add_session_node(
        SessionNodePrimitive::ResumeSession,
        SessionNodeMetadata {
            session_id: None,
            session_name: None,
            session_file: None,
            cwd: None,
            message_count: 0,
        },
        false,
    );

    assert_eq!(first, 1);
    assert_eq!(second, 2);
    assert_eq!(canvas.nodes()[0].position(), WorldPoint::new(90.0, 45.0));
    assert_eq!(canvas.nodes()[1].position(), WorldPoint::new(360.0, 220.0));
}
