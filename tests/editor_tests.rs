use irasu_illustrator::core::boolean::{apply_polygon_boolean, execute_pathfinder, BooleanOp};
use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::geometry::{
    line_segment_intersection, point_in_polygon, polygon_centroid,
};
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
        ..StrokeStyle::default()
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

#[test]
fn test_multi_select_move_and_undo() {
    use irasu_illustrator::core::history::MoveObjectCommand;
    let mut doc = Document::default();
    let o1 = Object::new_rect("Rect1", 10.0, 20.0, 100.0, 50.0, 0.0);
    let o2 = Object::new_rect("Rect2", 200.0, 300.0, 80.0, 80.0, 0.0);
    let id1 = o1.id.clone();
    let id2 = o2.id.clone();
    doc.add_object(o1);
    doc.add_object(o2);

    let mut undo_mgr = UndoManager::new();

    // Simulate multi-select move by (dx=30, dy=40)
    let dx = 30.0;
    let dy = 40.0;
    for (id, orig_x, orig_y) in [(&id1, 0.0, 0.0), (&id2, 0.0, 0.0)] {
        for (_, obj) in doc.all_objects_mut() {
            if &obj.id == id {
                obj.transform.x = orig_x + dx;
                obj.transform.y = orig_y + dy;
            }
        }
        let cmd = Box::new(MoveObjectCommand {
            object_id: id.clone(),
            old_x: orig_x,
            old_y: orig_y,
            new_x: orig_x + dx,
            new_y: orig_y + dy,
        });
        undo_mgr.execute(cmd, &mut doc);
    }

    // Verify positions
    let obj1 = doc.all_objects().find(|(_, o)| o.id == id1).unwrap().1;
    assert_eq!(obj1.transform.x, 30.0);
    assert_eq!(obj1.transform.y, 40.0);
    let obj2 = doc.all_objects().find(|(_, o)| o.id == id2).unwrap().1;
    assert_eq!(obj2.transform.x, 30.0);
    assert_eq!(obj2.transform.y, 40.0);

    // Undo both moves
    undo_mgr.undo(&mut doc);
    undo_mgr.undo(&mut doc);

    let obj1_restored = doc.all_objects().find(|(_, o)| o.id == id1).unwrap().1;
    assert_eq!(obj1_restored.transform.x, 0.0);
    assert_eq!(obj1_restored.transform.y, 0.0);
    let obj2_restored = doc.all_objects().find(|(_, o)| o.id == id2).unwrap().1;
    assert_eq!(obj2_restored.transform.x, 0.0);
    assert_eq!(obj2_restored.transform.y, 0.0);
}

#[test]
fn test_delete_layer_preservation() {
    use irasu_illustrator::core::document::Layer;
    let mut doc = Document::default();
    doc.layers.push(Layer::new("Layer 2"));
    assert_eq!(doc.layers.len(), 2);

    let o1 = Object::new_rect("Obj1", 0.0, 0.0, 50.0, 50.0, 0.0);
    let o2 = Object::new_rect("Obj2", 100.0, 100.0, 50.0, 50.0, 0.0);
    let o3 = Object::new_rect("Obj3", 200.0, 200.0, 50.0, 50.0, 0.0);
    let id2 = o2.id.clone();

    // Insert into layer 1
    doc.layers[1].objects.push(o1);
    doc.layers[1].objects.push(o2);
    doc.layers[1].objects.push(o3);

    let mut undo_mgr = UndoManager::new();

    // Find layer and position before deletion
    let mut found = None;
    for (l_idx, layer) in doc.layers.iter().enumerate() {
        if let Some(pos) = layer.objects.iter().position(|o| o.id == id2) {
            found = Some((layer.objects[pos].clone(), l_idx, pos));
            break;
        }
    }
    assert!(found.is_some());
    let (target_obj, l_idx, pos) = found.unwrap();
    assert_eq!(l_idx, 1);
    assert_eq!(pos, 1);

    // Delete
    let cmd = Box::new(RemoveObjectCommand::new(target_obj, l_idx, pos));
    undo_mgr.execute(cmd, &mut doc);
    assert_eq!(doc.layers[1].objects.len(), 2);
    assert!(doc.layers[1].objects.iter().all(|o| o.id != id2));

    // Undo -> restores to exact layer 1 and pos 1
    undo_mgr.undo(&mut doc);
    assert_eq!(doc.layers[1].objects.len(), 3);
    assert_eq!(doc.layers[1].objects[1].id, id2);
}

#[test]
fn test_sample_gradient_stops_robustness() {
    use irasu_illustrator::core::path::GradientStop;
    use irasu_illustrator::ui::canvas::rendering::sample_gradient_stops;

    // Empty stops
    let col = sample_gradient_stops(&[], 0.5);
    assert_eq!(col, [0.0, 0.0, 0.0, 1.0]);

    // Single stop
    let single = vec![GradientStop {
        offset: 0.5,
        color: [1.0, 0.0, 0.0, 1.0],
    }];
    let col = sample_gradient_stops(&single, 0.2);
    assert_eq!(col, [1.0, 0.0, 0.0, 1.0]);

    // Normal multi-stops with clamp
    let stops = vec![
        GradientStop {
            offset: 0.0,
            color: [0.0, 0.0, 0.0, 1.0],
        },
        GradientStop {
            offset: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
        },
    ];
    let mid = sample_gradient_stops(&stops, 0.5);
    assert!((mid[0] - 0.5).abs() < 1e-3);

    // Negative and > 1.0 t
    let under = sample_gradient_stops(&stops, -10.0);
    assert_eq!(under[0], 0.0);
    let over = sample_gradient_stops(&stops, 10.0);
    assert_eq!(over[0], 1.0);
}

#[test]
fn test_batch_command_undo_redo() {
    use irasu_illustrator::core::history::{BatchCommand, RemoveObjectCommand};
    let mut doc = Document::default();
    let o1 = Object::new_rect("R1", 0.0, 0.0, 50.0, 50.0, 0.0);
    let o2 = Object::new_rect("R2", 60.0, 0.0, 50.0, 50.0, 0.0);
    let o3 = Object::new_rect("R3", 120.0, 0.0, 50.0, 50.0, 0.0);
    doc.add_object(o1.clone());
    doc.add_object(o2.clone());
    doc.add_object(o3.clone());
    assert_eq!(doc.all_objects().count(), 3);

    let mut undo_mgr = UndoManager::new();

    // Delete all 3 in a single batch
    let cmds: Vec<Box<dyn irasu_illustrator::core::history::Command>> = vec![
        Box::new(RemoveObjectCommand::new(o1, 0, 0)),
        Box::new(RemoveObjectCommand::new(o2, 0, 0)),
        Box::new(RemoveObjectCommand::new(o3, 0, 0)),
    ];
    let batch = Box::new(BatchCommand::new("Delete 3 Objects", cmds));
    undo_mgr.execute(batch, &mut doc);
    assert_eq!(doc.all_objects().count(), 0);

    // Single undo restores all 3
    undo_mgr.undo(&mut doc);
    assert_eq!(doc.all_objects().count(), 3);

    // Single redo deletes all 3 again
    undo_mgr.redo(&mut doc);
    assert_eq!(doc.all_objects().count(), 0);
}

#[test]
fn test_raster_export_png_and_jpeg() {
    let mut doc = Document::default();
    doc.width = 400.0;
    doc.height = 300.0;
    let mut r = Object::new_rect("Box", 50.0, 50.0, 200.0, 150.0, 10.0);
    r.fill = Some(FillStyle::solid([0.1, 0.5, 0.9, 1.0]));
    doc.add_object(r);

    // PNG Export
    let png_res = irasu_illustrator::io::raster::export_png(&doc, 1.0, true);
    assert!(png_res.is_ok(), "PNG export failed: {:?}", png_res.err());
    let png_bytes = png_res.unwrap();
    assert!(png_bytes.len() > 100);
    // Standard PNG magic signature: 0x89 'P' 'N' 'G'
    assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);

    // JPEG Export
    let jpeg_res = irasu_illustrator::io::raster::export_jpeg(&doc, 1.0);
    assert!(jpeg_res.is_ok(), "JPEG export failed: {:?}", jpeg_res.err());
    let jpeg_bytes = jpeg_res.unwrap();
    assert!(jpeg_bytes.len() > 100);
    // Standard JPEG magic signature: 0xFF 0xD8 0xFF
    assert_eq!(&jpeg_bytes[0..3], &[0xFF, 0xD8, 0xFF]);
}

#[test]
fn test_modify_path_command_undo_redo() {
    use irasu_illustrator::core::history::ModifyPathCommand;
    use irasu_illustrator::core::path::PathElement;

    let mut doc = Document::default();
    let mut path = PathData::new();
    path.push_move_to(0.0, 0.0);
    path.push_line_to(100.0, 100.0);
    let orig_elements = path.elements.clone();

    let obj = Object::new_path("TestPath", path);
    let obj_id = obj.id.clone();
    doc.add_object(obj);

    let mut new_elements = orig_elements.clone();
    new_elements[1] = PathElement::LineTo(AnchorPoint::new(250.0, 300.0));

    let mut undo_mgr = UndoManager::new();
    let cmd = Box::new(ModifyPathCommand::new(
        obj_id.clone(),
        orig_elements.clone(),
        new_elements.clone(),
    ));
    undo_mgr.execute(cmd, &mut doc);

    // Verify modified
    let elem = doc.all_objects().find(|(_, o)| o.id == obj_id).unwrap().1;
    if let ObjectType::Path(ref p) = elem.object_type {
        assert_eq!(
            p.elements[1],
            PathElement::LineTo(AnchorPoint::new(250.0, 300.0))
        );
    } else {
        panic!("Expected Path object");
    }

    // Undo -> restores original
    undo_mgr.undo(&mut doc);
    let elem = doc.all_objects().find(|(_, o)| o.id == obj_id).unwrap().1;
    if let ObjectType::Path(ref p) = elem.object_type {
        assert_eq!(
            p.elements[1],
            PathElement::LineTo(AnchorPoint::new(100.0, 100.0))
        );
    } else {
        panic!("Expected Path object");
    }
}

#[test]
fn test_pre_release_multi_select_move_and_alt_duplicate() {
    use irasu_illustrator::core::history::{AddObjectCommand, BatchCommand, MoveObjectCommand};
    let mut doc = Document::default();
    let o1 = Object::new_rect("R1", 10.0, 20.0, 100.0, 50.0, 0.0);
    let o2 = Object::new_rect("R2", 200.0, 300.0, 80.0, 80.0, 0.0);
    let id1 = o1.id.clone();
    let id2 = o2.id.clone();
    doc.add_object(o1);
    doc.add_object(o2);

    let mut undo_mgr = UndoManager::new();

    // 1. Move both objects together in a BatchCommand
    let dx = 40.0;
    let dy = 50.0;
    for (id, orig_x, orig_y) in [(&id1, 0.0, 0.0), (&id2, 0.0, 0.0)] {
        for (_, obj) in doc.all_objects_mut() {
            if &obj.id == id {
                obj.transform.x = orig_x + dx;
                obj.transform.y = orig_y + dy;
            }
        }
    }
    let move_batch = Box::new(BatchCommand::new(
        "Move Objects",
        vec![
            Box::new(MoveObjectCommand {
                object_id: id1.clone(),
                old_x: 0.0,
                old_y: 0.0,
                new_x: 40.0,
                new_y: 50.0,
            }),
            Box::new(MoveObjectCommand {
                object_id: id2.clone(),
                old_x: 0.0,
                old_y: 0.0,
                new_x: 40.0,
                new_y: 50.0,
            }),
        ],
    ));
    undo_mgr.execute(move_batch, &mut doc);

    // Verify moved
    assert_eq!(
        doc.all_objects()
            .find(|(_, o)| o.id == id1)
            .unwrap()
            .1
            .transform
            .x,
        40.0
    );
    assert_eq!(
        doc.all_objects()
            .find(|(_, o)| o.id == id2)
            .unwrap()
            .1
            .transform
            .x,
        40.0
    );

    // Undo move in 1 single step
    undo_mgr.undo(&mut doc);
    assert_eq!(
        doc.all_objects()
            .find(|(_, o)| o.id == id1)
            .unwrap()
            .1
            .transform
            .x,
        0.0
    );
    assert_eq!(
        doc.all_objects()
            .find(|(_, o)| o.id == id2)
            .unwrap()
            .1
            .transform
            .x,
        0.0
    );

    // Redo move in 1 step
    undo_mgr.redo(&mut doc);
    assert_eq!(
        doc.all_objects()
            .find(|(_, o)| o.id == id1)
            .unwrap()
            .1
            .transform
            .x,
        40.0
    );
    assert_eq!(
        doc.all_objects()
            .find(|(_, o)| o.id == id2)
            .unwrap()
            .1
            .transform
            .x,
        40.0
    );

    // 2. Alt+Drag Duplicate in a single BatchCommand
    let d1 = Object::new_rect("R1 Copy", 50.0, 60.0, 100.0, 50.0, 0.0);
    let d2 = Object::new_rect("R2 Copy", 250.0, 350.0, 80.0, 80.0, 0.0);
    let dup_batch = Box::new(BatchCommand::new(
        "Duplicate Objects",
        vec![
            Box::new(AddObjectCommand::new(d1)),
            Box::new(AddObjectCommand::new(d2)),
        ],
    ));
    undo_mgr.execute(dup_batch, &mut doc);
    assert_eq!(doc.all_objects().count(), 4);

    // Undo duplication in 1 step
    undo_mgr.undo(&mut doc);
    assert_eq!(doc.all_objects().count(), 2);
}

#[test]
fn test_pre_release_bezier_rapid_undo_redo() {
    use irasu_illustrator::core::history::ModifyPathCommand;

    let mut doc = Document::default();
    let mut path = PathData::new();
    path.push_move_to(0.0, 0.0);
    path.push_curve_to(
        AnchorPoint::new(30.0, 100.0),
        AnchorPoint::new(70.0, 100.0),
        AnchorPoint::new(100.0, 0.0),
    );
    let orig = path.elements.clone();

    let obj = Object::new_path("CurveObj", path);
    let obj_id = obj.id.clone();
    doc.add_object(obj);

    let mut undo_mgr = UndoManager::new();

    // Perform 10 consecutive Bézier handle mutations
    let mut last_elems = orig.clone();
    for step in 1..=10 {
        let mut next_elems = last_elems.clone();
        if let irasu_illustrator::core::path::PathElement::CurveTo(ref mut seg) = next_elems[1] {
            seg.control1.y += step as f64 * 5.0;
            seg.control2.y += step as f64 * 7.0;
        }
        let cmd = Box::new(ModifyPathCommand::new(
            obj_id.clone(),
            last_elems.clone(),
            next_elems.clone(),
        ));
        undo_mgr.execute(cmd, &mut doc);
        last_elems = next_elems;
    }

    // Rapid Undo 10 times in a row
    for _ in 0..10 {
        assert!(undo_mgr.can_undo());
        undo_mgr.undo(&mut doc);
    }
    assert!(!undo_mgr.can_undo());

    // Verify completely restored to initial geometry
    let restored = doc.all_objects().find(|(_, o)| o.id == obj_id).unwrap().1;
    if let ObjectType::Path(ref p) = restored.object_type {
        assert_eq!(p.elements, orig);
    }

    // Rapid Redo 10 times in a row
    for _ in 0..10 {
        assert!(undo_mgr.can_redo());
        undo_mgr.redo(&mut doc);
    }
    assert!(!undo_mgr.can_redo());
    let redo_res = doc.all_objects().find(|(_, o)| o.id == obj_id).unwrap().1;
    if let ObjectType::Path(ref p) = redo_res.object_type {
        assert_eq!(p.elements, last_elems);
    }
}

#[test]
fn test_pre_release_svg_import_and_png_scales_and_jpeg() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">
        <rect x="20" y="20" width="160" height="160" rx="20" fill="#2073e6"/>
        <circle cx="100" cy="100" r="50" fill="#ffffff" opacity="0.85"/>
    </svg>"##;

    let doc = parse_svg_document(svg);
    assert_eq!(doc.all_objects().count(), 2);

    // PNG 1x
    let png1 = irasu_illustrator::io::raster::export_png(&doc, 1.0, true).expect("PNG 1x failed");
    assert_eq!(&png1[0..4], &[0x89, 0x50, 0x4E, 0x47]);

    // PNG 2x
    let png2 = irasu_illustrator::io::raster::export_png(&doc, 2.0, true).expect("PNG 2x failed");
    assert_eq!(&png2[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    assert!(
        png2.len() > png1.len(),
        "2x PNG should be larger in bytes than 1x PNG"
    );

    // PNG 4x
    let png4 = irasu_illustrator::io::raster::export_png(&doc, 4.0, true).expect("PNG 4x failed");
    assert_eq!(&png4[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    assert!(
        png4.len() > png2.len(),
        "4x PNG should be larger in bytes than 2x PNG"
    );

    // Transparent PNG vs Non-transparent PNG
    let png_trans =
        irasu_illustrator::io::raster::export_png(&doc, 1.0, true).expect("Transparent PNG failed");
    let png_opaque =
        irasu_illustrator::io::raster::export_png(&doc, 1.0, false).expect("Opaque PNG failed");
    assert_eq!(&png_trans[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    assert_eq!(&png_opaque[0..4], &[0x89, 0x50, 0x4E, 0x47]);

    // JPEG Export
    let jpeg = irasu_illustrator::io::raster::export_jpeg(&doc, 1.0).expect("JPEG export failed");
    assert_eq!(&jpeg[0..3], &[0xFF, 0xD8, 0xFF]);
}

#[test]
fn test_pre_release_empty_document_export() {
    let empty_doc = Document::default();
    assert_eq!(empty_doc.all_objects().count(), 0);

    // SVG Export on empty document
    let svg = export_svg(&empty_doc);
    assert!(!svg.is_empty());

    // PDF Export on empty document
    let pdf = irasu_illustrator::io::pdf::export_pdf(&empty_doc);
    assert!(!pdf.is_empty());

    // PNG Export on empty document
    let png = irasu_illustrator::io::raster::export_png(&empty_doc, 1.0, true);
    assert!(png.is_ok());
    let png_bytes = png.unwrap();
    assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);

    // JPEG Export on empty document
    let jpeg = irasu_illustrator::io::raster::export_jpeg(&empty_doc, 1.0);
    assert!(jpeg.is_ok());
    let jpeg_bytes = jpeg.unwrap();
    assert_eq!(&jpeg_bytes[0..3], &[0xFF, 0xD8, 0xFF]);
}

#[test]
fn test_pre_release_extreme_and_degenerate_export() {
    let mut doc = Document::default();

    // 1. Extreme coordinates (500,000 pt)
    let huge_obj = Object::new_rect("Huge", 500_000.0, 500_000.0, 100_000.0, 100_000.0, 0.0);
    doc.add_object(huge_obj);

    // 2. Zero-width / zero-height shape
    let zero_obj = Object::new_rect("Zero", 0.0, 0.0, 0.0, 0.0, 0.0);
    doc.add_object(zero_obj);

    // 3. Degenerate empty path
    let empty_path_obj = Object::new_path("EmptyPath", PathData::new());
    doc.add_object(empty_path_obj);

    // 4. Single-point path
    let mut single_pt = PathData::new();
    single_pt.push_move_to(50.0, 50.0);
    doc.add_object(Object::new_path("SinglePt", single_pt));

    // Must not panic, must produce valid SVG, PDF, PNG, JPEG
    let svg = export_svg(&doc);
    assert!(!svg.is_empty());

    let pdf = irasu_illustrator::io::pdf::export_pdf(&doc);
    assert!(!pdf.is_empty());

    let png = irasu_illustrator::io::raster::export_png(&doc, 1.0, true);
    assert!(png.is_ok());

    let jpeg = irasu_illustrator::io::raster::export_jpeg(&doc, 1.0);
    assert!(jpeg.is_ok());
}

#[test]
fn test_stress_complex_operation_chain() {
    let mut doc = Document::default();
    let mut undo_mgr = UndoManager::new();

    // Create 3 layers
    doc.layers
        .push(irasu_illustrator::core::document::Layer::new("Layer 2"));
    doc.layers
        .push(irasu_illustrator::core::document::Layer::new("Layer 3"));

    // Add objects across layers
    let obj1 = Object::new_rect("Rect1", 10.0, 10.0, 50.0, 50.0, 0.0);
    let obj2 = Object::new_rect("Rect2", 70.0, 10.0, 50.0, 50.0, 0.0);
    let _id1 = obj1.id.clone();
    let _id2 = obj2.id.clone();
    doc.layers[0].objects.push(obj1);
    doc.layers[0].objects.push(obj2);

    // Group obj1 and obj2 into a Group object
    let g_obj1 = doc.layers[0].objects.remove(0);
    let g_obj2 = doc.layers[0].objects.remove(0);
    let group = Object::new_group("Group1", vec![g_obj1, g_obj2]);
    let group_id = group.id.clone();
    doc.layers[0].objects.push(group);

    // Add obj3 on Layer 2
    let obj3 = Object::new_rect("Rect3", 200.0, 100.0, 80.0, 80.0, 0.0);
    let id3 = obj3.id.clone();
    doc.layers[1].objects.push(obj3);

    // Multi-selection: Select group on Layer 1 AND rect on Layer 2
    let selected_ids = vec![group_id.clone(), id3.clone()];

    // Move both by dx=30, dy=40 using BatchCommand
    let mut move_cmds: Vec<Box<dyn irasu_illustrator::core::history::Command>> = Vec::new();
    for id in &selected_ids {
        if let Some((_, obj)) = doc.object_by_id(id) {
            move_cmds.push(Box::new(
                irasu_illustrator::core::history::MoveObjectCommand {
                    object_id: id.clone(),
                    old_x: obj.transform.x,
                    old_y: obj.transform.y,
                    new_x: obj.transform.x + 30.0,
                    new_y: obj.transform.y + 40.0,
                },
            ));
        }
    }
    let batch_move = Box::new(irasu_illustrator::core::history::BatchCommand::new(
        "Move Multi",
        move_cmds,
    ));
    undo_mgr.execute(batch_move, &mut doc);

    // Assert positions moved
    let (_, g) = doc.object_by_id(&group_id).unwrap();
    assert_eq!(g.transform.x, 30.0);
    assert_eq!(g.transform.y, 40.0);
    let (_, r3) = doc.object_by_id(&id3).unwrap();
    assert_eq!(r3.transform.x, 230.0);
    assert_eq!(r3.transform.y, 140.0);

    // Reorder layers
    doc.move_layer_up(0);
    assert_eq!(doc.layers[0].name, "Layer 2");
    assert_eq!(doc.layers[1].name, "Layer 1");

    // Alt-duplicate selection
    let mut dup_cmds: Vec<Box<dyn irasu_illustrator::core::history::Command>> = Vec::new();
    let mut new_ids = Vec::new();
    for id in &selected_ids {
        if let Some((_layer_idx, obj)) = doc.object_by_id(id) {
            let mut clone = obj.clone();
            clone.id = uuid::Uuid::new_v4().to_string();
            clone.transform.x += 20.0;
            clone.transform.y += 20.0;
            new_ids.push(clone.id.clone());
            dup_cmds.push(Box::new(AddObjectCommand::new(clone)));
        }
    }
    let batch_dup = Box::new(irasu_illustrator::core::history::BatchCommand::new(
        "Alt Duplicate",
        dup_cmds,
    ));
    undo_mgr.execute(batch_dup, &mut doc);

    // Total objects is now 4 (Layer 2 has 2, Layer 1 has 2)
    assert_eq!(doc.all_objects().count(), 4);

    // Undo duplicate
    undo_mgr.undo(&mut doc);
    assert_eq!(doc.all_objects().count(), 2);

    // Undo move
    undo_mgr.undo(&mut doc);
    let (_, g) = doc.object_by_id(&group_id).unwrap();
    assert_eq!(g.transform.x, 0.0);
    assert_eq!(g.transform.y, 0.0);
    let (_, r3) = doc.object_by_id(&id3).unwrap();
    assert_eq!(r3.transform.x, 200.0);
    assert_eq!(r3.transform.y, 100.0);

    // Redo move
    undo_mgr.redo(&mut doc);
    let (_, g) = doc.object_by_id(&group_id).unwrap();
    assert_eq!(g.transform.x, 30.0);
    assert_eq!(g.transform.y, 40.0);

    // Redo duplicate
    undo_mgr.redo(&mut doc);
    assert_eq!(doc.all_objects().count(), 4);
}

#[test]
fn test_stress_external_svg_multiline_styles_and_nested_groups() {
    let external_svg = r##"<?xml version="1.0" encoding="UTF-8"?>
<!-- Created with Adobe Illustrator 28.0 / Figma / Inkscape -->
<svg xmlns="http://www.w3.org/2000/svg" width="800" height="600" viewBox="0 0 800 600">
  <!-- Multi-line path with inline style -->
  <path
    d="M 50 50
       C 100 0, 150 100, 200 50
       L 200 150
       Z"
    style="fill:#00ff88;stroke:#112233;stroke-width:3px;opacity:0.85"
  />

  <!-- Polygon and Polyline -->
  <polygon
    points="10,10 40,40 10,70"
    style="fill:#ff0000;stroke:#000000;stroke-width:1"
  />
  <polyline
    points="300,100 350,150 400,100"
    style="fill:none;stroke:#0000ff;stroke-width:2"
  />

  <!-- Nested groups with translations -->
  <g transform="translate(100, 50)">
    <g transform="translate(20, 10)">
      <circle cx="10" cy="10" r="15" fill="#ffa500" />
      <rect x="50" y="50" width="80" height="40" rx="5" fill="#333333" />
    </g>
  </g>
</svg>"##;

    let doc = parse_svg_document(external_svg);
    assert_eq!(doc.width, 800.0);
    assert_eq!(doc.height, 600.0);

    let objects: Vec<_> = doc.all_objects().map(|(_, o)| o).collect();
    assert_eq!(objects.len(), 5);

    // 1. Path
    let p_obj = objects[0];
    assert_eq!(p_obj.opacity, 0.85);
    if let ObjectType::Path(ref p) = p_obj.object_type {
        assert!(p.closed);
        assert_eq!(p.elements.len(), 4); // MoveTo, CurveTo, LineTo, ClosePath
        let fill = p.fill.as_ref().unwrap();
        assert_eq!(fill.color, [0.0, 1.0, 136.0 / 255.0, 1.0]);
        let stroke = p.stroke.as_ref().unwrap();
        assert_eq!(stroke.width, 3.0);
    } else {
        panic!("Object 0 should be a Path");
    }

    // 2. Polygon
    let poly_obj = objects[1];
    if let ObjectType::Path(ref p) = poly_obj.object_type {
        assert!(p.closed);
        assert_eq!(p.elements.len(), 4); // MoveTo + 2 LineTo + ClosePath
    } else {
        panic!("Object 1 should be a closed Path");
    }

    // 3. Polyline
    let line_obj = objects[2];
    if let ObjectType::Path(ref p) = line_obj.object_type {
        assert!(!p.closed);
        assert_eq!(p.elements.len(), 3); // MoveTo + 2 LineTo
    } else {
        panic!("Object 2 should be an open Path");
    }

    // 4. Circle in nested groups (tx = 100 + 20 = 120, ty = 50 + 10 = 60)
    let circle_obj = objects[3];
    if let ObjectType::Ellipse { rx, ry } = circle_obj.object_type {
        assert_eq!(rx, 15.0);
        assert_eq!(ry, 15.0);
        // Original cx=10, cy=10 -> with total_tx=120, total_ty=60:
        assert_eq!(circle_obj.transform.x, 130.0);
        assert_eq!(circle_obj.transform.y, 70.0);
    } else {
        panic!("Object 3 should be an Ellipse");
    }

    // 5. Rect in nested groups
    let rect_obj = objects[4];
    if let ObjectType::Rectangle {
        width,
        height,
        corner_radius,
    } = rect_obj.object_type
    {
        assert_eq!(width, 80.0);
        assert_eq!(height, 40.0);
        assert_eq!(corner_radius, 5.0);
        assert_eq!(rect_obj.transform.x, 170.0); // 50 + 120
        assert_eq!(rect_obj.transform.y, 110.0); // 50 + 60
    } else {
        panic!("Object 4 should be a Rectangle");
    }
}

#[test]
fn test_stress_save_reload_round_trip_fidelity() {
    let doc = Document {
        name: "Fidelity Test Doc".to_string(),
        width: 2560.0,
        height: 1440.0,
        active_layer_idx: 1,
        layers: vec![
            irasu_illustrator::core::document::Layer {
                id: "layer_base".to_string(),
                name: "Base Background".to_string(),
                objects: vec![Object::new_rect(
                    "Background",
                    0.0,
                    0.0,
                    2560.0,
                    1440.0,
                    0.0,
                )],
                visible: true,
                locked: false,
                opacity: 0.9,
            },
            irasu_illustrator::core::document::Layer {
                id: "layer_vectors".to_string(),
                name: "Vector Artwork".to_string(),
                objects: vec![
                    {
                        let mut p = PathData::new();
                        p.push_move_to(100.0, 200.0);
                        p.push_curve_to(
                            AnchorPoint::new(150.0, 100.0),
                            AnchorPoint::new(250.0, 300.0),
                            AnchorPoint::new(300.0, 200.0),
                        );
                        p.closed = false;
                        let mut obj = Object::new_path("Curve", p);
                        obj.blend_mode = irasu_illustrator::core::document::BlendMode::Overlay;
                        obj.opacity = 0.75;
                        obj.shadow = Some(irasu_illustrator::core::effects::DropShadow {
                            offset_x: 8.0,
                            offset_y: 12.0,
                            blur_radius: 16.0,
                            color: [0.1, 0.1, 0.2, 0.8],
                            opacity: 0.6,
                        });
                        obj
                    },
                    Object::new_ellipse("Planet", 600.0, 400.0, 120.0, 80.0),
                    Object::new_text("Title", "Amata Studio 0.1.0", 50.0, 80.0, 48.0),
                ],
                visible: true,
                locked: false,
                opacity: 1.0,
            },
        ],
        symbols: Vec::new(),
        timeline: Default::default(),
        guides: Vec::new(),
        color_mode: Default::default(),
        artboards: Vec::new(),
    };

    let temp_dir = std::env::temp_dir();
    let amata_path = temp_dir.join("test_fidelity.amata");
    let json_path = temp_dir.join("test_fidelity.json");

    // Save as .amata
    let res_save_amata = irasu_illustrator::io::project::save_project(&doc, &amata_path);
    assert!(res_save_amata.is_ok());

    // Load from .amata
    let loaded_amata = irasu_illustrator::io::project::load_project(&amata_path).unwrap();
    assert_eq!(loaded_amata, doc);

    // Save as .json
    let res_save_json = irasu_illustrator::io::project::save_project(&doc, &json_path);
    assert!(res_save_json.is_ok());

    // Load from .json
    let loaded_json = irasu_illustrator::io::project::load_project(&json_path).unwrap();
    assert_eq!(loaded_json, doc);

    // Clean up
    let _ = std::fs::remove_file(amata_path);
    let _ = std::fs::remove_file(json_path);
}

#[test]
fn test_stress_bezier_node_editing_boundary_conditions() {
    let mut doc = Document::default();
    let mut undo_mgr = UndoManager::new();

    // 1. Single-node path (degenerate MoveTo only)
    let mut single_node_path = PathData::new();
    single_node_path.push_move_to(100.0, 100.0);
    let single_obj = Object::new_path("SingleNode", single_node_path.clone());
    let s_id = single_obj.id.clone();
    doc.add_object(single_obj);

    // Move the single anchor point to (150, 160)
    let mut modified_single = single_node_path.clone();
    if let irasu_illustrator::core::path::PathElement::MoveTo(ref mut pt) =
        modified_single.elements[0]
    {
        pt.x = 150.0;
        pt.y = 160.0;
    }
    let cmd = Box::new(irasu_illustrator::core::history::ModifyPathCommand::new(
        s_id.clone(),
        single_node_path.elements.clone(),
        modified_single.elements.clone(),
    ));
    undo_mgr.execute(cmd, &mut doc);

    let (_, obj) = doc.object_by_id(&s_id).unwrap();
    if let ObjectType::Path(ref p) = obj.object_type {
        assert_eq!(p.elements.len(), 1);
        if let irasu_illustrator::core::path::PathElement::MoveTo(ref pt) = p.elements[0] {
            assert_eq!(pt.x, 150.0);
            assert_eq!(pt.y, 160.0);
        }
    }

    // Undo and verify restored
    undo_mgr.undo(&mut doc);
    let (_, obj) = doc.object_by_id(&s_id).unwrap();
    if let ObjectType::Path(ref p) = obj.object_type {
        if let irasu_illustrator::core::path::PathElement::MoveTo(ref pt) = p.elements[0] {
            assert_eq!(pt.x, 100.0);
            assert_eq!(pt.y, 100.0);
        }
    }

    // 2. Closed path with cubic curves
    let mut loop_path = PathData::new();
    loop_path.push_move_to(0.0, 0.0);
    loop_path.push_curve_to(
        AnchorPoint::new(10.0, 50.0),
        AnchorPoint::new(90.0, 50.0),
        AnchorPoint::new(100.0, 0.0),
    );
    loop_path.push_curve_to(
        AnchorPoint::new(90.0, -50.0),
        AnchorPoint::new(10.0, -50.0),
        AnchorPoint::new(0.0, 0.0),
    );
    loop_path
        .elements
        .push(irasu_illustrator::core::path::PathElement::ClosePath);
    loop_path.closed = true;

    let loop_obj = Object::new_path("Loop", loop_path.clone());
    let l_id = loop_obj.id.clone();
    doc.add_object(loop_obj);

    // Edit Control2 of the first curve
    let mut edited_loop = loop_path.clone();
    if let irasu_illustrator::core::path::PathElement::CurveTo(ref mut seg) =
        edited_loop.elements[1]
    {
        seg.control2 = AnchorPoint::new(95.0, 75.0);
    }
    let cmd2 = Box::new(irasu_illustrator::core::history::ModifyPathCommand::new(
        l_id.clone(),
        loop_path.elements.clone(),
        edited_loop.elements.clone(),
    ));
    undo_mgr.execute(cmd2, &mut doc);

    let (_, obj) = doc.object_by_id(&l_id).unwrap();
    if let ObjectType::Path(ref p) = obj.object_type {
        if let irasu_illustrator::core::path::PathElement::CurveTo(ref seg) = p.elements[1] {
            assert_eq!(seg.control2.x, 95.0);
            assert_eq!(seg.control2.y, 75.0);
        }
    }
}

#[test]
fn test_stress_toast_notification_feedback() {
    let mut state = irasu_illustrator::core::state::AppState::default();
    assert!(state.toast.is_none());

    state.notify_info("書き出し成功");
    assert!(state.toast.is_some());
    let toast = state.toast.as_ref().unwrap();
    assert_eq!(toast.message, "書き出し成功");
    assert!(!toast.is_error);

    state.notify_error("保存失敗: 書き込み権限がありません");
    let err_toast = state.toast.as_ref().unwrap();
    assert_eq!(err_toast.message, "保存失敗: 書き込み権限がありません");
    assert!(err_toast.is_error);
}
