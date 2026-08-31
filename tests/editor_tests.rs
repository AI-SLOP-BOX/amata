use irasu_illustrator::core::boolean::{apply_polygon_boolean, execute_pathfinder, BooleanOp};
use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::geometry::{line_segment_intersection, point_in_polygon, polygon_centroid};
use irasu_illustrator::core::history::{AddObjectCommand, RemoveObjectCommand, UndoManager};
use irasu_illustrator::core::path::{AnchorPoint, FillStyle, PathData, StrokeStyle};
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};

#[test]
fn test_geometry_point_in_polygon() {
    let square = vec![
        AnchorPoint::new(0.0, 0.0),
        AnchorPoint::new(100.0, 0.0),
        AnchorPoint::new(100.0, 100.0),
        AnchorPoint::new(0.0, 100.0),
    ];

    assert!(point_in_polygon(50.0, 50.0, &square));
    assert!(point_in_polygon(10.0, 90.0, &square));
    assert!(!point_in_polygon(150.0, 50.0, &square));
    assert!(!point_in_polygon(-10.0, -10.0, &square));
}

#[test]
fn test_line_segment_intersection() {
    let a1 = AnchorPoint::new(0.0, 50.0);
    let a2 = AnchorPoint::new(100.0, 50.0);
    let b1 = AnchorPoint::new(50.0, 0.0);
    let b2 = AnchorPoint::new(50.0, 100.0);

    let inter = line_segment_intersection(a1, a2, b1, b2);
    assert!(inter.is_some());
    let pt = inter.unwrap();
    assert!((pt.x - 50.0).abs() < 1e-4);
    assert!((pt.y - 50.0).abs() < 1e-4);
}

#[test]
fn test_polygon_centroid() {
    let square = vec![
        AnchorPoint::new(0.0, 0.0),
        AnchorPoint::new(100.0, 0.0),
        AnchorPoint::new(100.0, 100.0),
        AnchorPoint::new(0.0, 100.0),
    ];
    let c = polygon_centroid(&square);
    assert!((c.x - 50.0).abs() < 1e-4);
    assert!((c.y - 50.0).abs() < 1e-4);
}

#[test]
fn test_path_generators() {
    // Regular Polygon
    let poly_path = PathData::from_polygon(6, 50.0, 100.0, 100.0);
    assert_eq!(poly_path.elements.len(), 7); // 1 MoveTo + 5 LineTo + 1 ClosePath

    // Star
    let star_path = PathData::from_star(5, 20.0, 50.0, 0.0, 0.0);
    assert_eq!(star_path.elements.len(), 11); // 1 MoveTo + 9 LineTo + 1 ClosePath

    // Rounded Rect
    let rrect = PathData::from_rect(0.0, 0.0, 200.0, 100.0, 10.0);
    assert!(!rrect.is_empty());
    let bb = rrect.bounding_box();
    assert!(bb.is_some());

    // Freehand Smooth Spline
    let freehand_pts = vec![
        AnchorPoint::new(0.0, 0.0),
        AnchorPoint::new(50.0, 20.0),
        AnchorPoint::new(100.0, 80.0),
        AnchorPoint::new(150.0, 10.0),
    ];
    let smooth_path = PathData::from_smooth_points(&freehand_pts);
    assert_eq!(smooth_path.elements.len(), 4); // 1 MoveTo + 3 CurveTo
}

#[test]
fn test_boolean_operations_pathfinder() {
    let square_a = vec![
        AnchorPoint::new(0.0, 0.0),
        AnchorPoint::new(100.0, 0.0),
        AnchorPoint::new(100.0, 100.0),
        AnchorPoint::new(0.0, 100.0),
    ];
    let square_b = vec![
        AnchorPoint::new(50.0, 0.0),
        AnchorPoint::new(150.0, 0.0),
        AnchorPoint::new(150.0, 100.0),
        AnchorPoint::new(50.0, 100.0),
    ];

    // Union
    let union_res = apply_polygon_boolean(&square_a, &square_b, BooleanOp::Union);
    assert!(!union_res.is_empty());

    // Intersect
    let inter_res = apply_polygon_boolean(&square_a, &square_b, BooleanOp::Intersect);
    assert_eq!(inter_res.len(), 1);
    let poly = &inter_res[0];
    assert!(poly.iter().any(|p| (p.x - 50.0).abs() < 1e-3));
    assert!(poly.iter().any(|p| (p.x - 100.0).abs() < 1e-3));

    // Subtract
    let sub_res = apply_polygon_boolean(&square_a, &square_b, BooleanOp::Subtract);
    assert!(!sub_res.is_empty());
}

#[test]
fn test_execute_pathfinder_with_objects() {
    let mut obj1 = Object::new_rect("Rect 1", 0.0, 0.0, 100.0, 100.0, 0.0);
    obj1.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));

    let mut obj2 = Object::new_rect("Rect 2", 50.0, 50.0, 100.0, 100.0, 0.0);
    obj2.fill = Some(FillStyle::solid([0.0, 1.0, 0.0, 1.0]));

    let result = execute_pathfinder(&[&obj1, &obj2], BooleanOp::Union);
    assert!(result.is_some());
    let united = result.unwrap();
    assert!(matches!(united.object_type, ObjectType::Path(_)));
    assert!(united.fill.is_some());
}

#[test]
fn test_document_history_undo_redo() {
    let mut doc = Document::default();
    let mut undo_mgr = UndoManager::new();

    let obj = Object::new_rect("Test Rect", 10.0, 10.0, 50.0, 50.0, 0.0);
    let obj_id = obj.id.clone();

    // Execute Add
    let cmd = Box::new(AddObjectCommand::new(obj.clone()));
    undo_mgr.execute(cmd, &mut doc);
    assert_eq!(doc.all_objects().count(), 1);
    assert!(doc.object_by_id(&obj_id).is_some());

    // Undo Add
    assert!(undo_mgr.can_undo());
    undo_mgr.undo(&mut doc);
    assert_eq!(doc.all_objects().count(), 0);

    // Redo Add
    assert!(undo_mgr.can_redo());
    undo_mgr.redo(&mut doc);
    assert_eq!(doc.all_objects().count(), 1);

    // Remove Object
    let removed = doc.remove_object(&obj_id).unwrap();
    let cmd_remove = Box::new(RemoveObjectCommand::new(removed, 0, 0));
    undo_mgr.execute(cmd_remove, &mut doc);
    assert_eq!(doc.all_objects().count(), 0);

    // Undo Remove
    undo_mgr.undo(&mut doc);
    assert_eq!(doc.all_objects().count(), 1);
}

#[test]
fn test_svg_export_and_parse() {
    let mut doc = Document::default();
    doc.width = 800.0;
    doc.height = 600.0;

    let mut rect = Object::new_rect("Main Rect", 20.0, 30.0, 200.0, 150.0, 8.0);
    rect.fill = Some(FillStyle::solid([0.8, 0.2, 0.3, 1.0]));
    doc.add_object(rect);

    let mut circle = Object::new_ellipse("Circle", 400.0, 300.0, 60.0, 60.0);
    circle.stroke = Some(StrokeStyle {
        color: [0.0, 0.0, 1.0, 1.0],
        width: 3.0,
        dash_pattern: None,
    });
    doc.add_object(circle);

    let star = Object::new_star("Star", 600.0, 200.0, 5, 25.0, 60.0);
    doc.add_object(star);

    let text = Object::new_text("Text", "Hello Vector World", 100.0, 500.0, 36.0);
    doc.add_object(text);

    let svg = export_svg(&doc);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("<ellipse"));
    assert!(svg.contains("<text"));
    assert!(svg.contains("Hello Vector World"));

    let imported_doc = parse_svg_document(&svg);
    assert_eq!(imported_doc.all_objects().count(), 4);
}
