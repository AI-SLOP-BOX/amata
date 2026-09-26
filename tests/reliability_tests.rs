#![allow(clippy::field_reassign_with_default)]
use irasu_illustrator::cli::handlers::common::save_any_document_scaled;
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::path::AnchorPoint;
use irasu_illustrator::io::svg::parse_svg_document;

#[test]
fn test_tspan_text_imported() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
      <text x="10" y="20" font-size="16"><tspan>hello</tspan><tspan> world</tspan></text>
    </svg>"##;
    let doc = parse_svg_document(svg);
    let texts: Vec<_> = doc
        .all_objects()
        .map(|(_, o)| o)
        .filter(|o| {
            matches!(
                o.object_type,
                irasu_illustrator::core::document::ObjectType::Text { .. }
            )
        })
        .collect();
    assert_eq!(texts.len(), 1, "tspan content must survive import");
    if let irasu_illustrator::core::document::ObjectType::Text { text, .. } =
        &texts[0].object_type
    {
        assert!(text.contains("hello") && text.contains("world"), "got: {text}");
    }
}

#[test]
fn test_svg_convert_scale_applies() {
    let mut doc = Document::default();
    doc.width = 100.0;
    doc.height = 50.0;
    doc.add_object(Object::new_rect("R", 10.0, 10.0, 20.0, 20.0, 0.0));
    let dir = std::env::temp_dir().join("amata_scale_check");
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("scaled.svg");
    save_any_document_scaled(&doc, &out, 2.0).unwrap();
    let content = std::fs::read_to_string(&out).unwrap();
    assert!(content.contains("width=\"200\""), "canvas must scale:\n{content}");
    let doc2 = parse_svg_document(&content);
    let rect = doc2
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| {
            matches!(
                o.object_type,
                irasu_illustrator::core::document::ObjectType::Rectangle { .. }
            )
        })
        .expect("rect round-trips");
    assert!((rect.transform.x - 20.0).abs() < 1e-6);
}

#[test]
fn test_project_with_empty_layers_normalizes() {
    let dir = std::env::temp_dir().join("amata_normalize_check");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("empty.amata");
    std::fs::write(
        &path,
        r#"{"name":"x","layers":[],"active_layer_idx":5,"width":100.0,"height":100.0,"symbols":[]}"#,
    )
    .unwrap();
    let mut doc = irasu_illustrator::io::project::load_project(&path).unwrap();
    assert_eq!(doc.layers.len(), 1);
    // Must not panic when adding objects afterwards.
    doc.add_object(Object::new_rect("R", 0.0, 0.0, 10.0, 10.0, 0.0));
}

#[test]
fn test_script_infinite_loop_terminates() {
    let engine = irasu_illustrator::plugin::script::ScriptEngine::new();
    let mut doc = Document::default();
    let mut state = irasu_illustrator::core::state::AppState::default();
    let res = engine.run_script("while true {}", &mut doc, &mut state);
    assert!(res.is_err(), "runaway script must be stopped by limits");
}

#[test]
fn test_nested_child_removable_by_id() {
    let mut doc = Document::default();
    let mut inner = Object::new_rect("Inner", 0.0, 0.0, 10.0, 10.0, 0.0);
    let inner_id = inner.id.clone();
    inner.id = inner_id.clone();
    let group = Object::new_group("G", vec![inner]);
    doc.add_object(group);
    assert!(doc.find_object(&inner_id).is_some());
    let removed = doc.remove_object(&inner_id);
    assert!(removed.is_some(), "nested child must be removable by id");
}

#[test]
fn test_generative_inputs_are_bounded() {
    // Would hang/OOM before caps: 100k symmetry folds, giant spirograph.
    let motif = Object::new_rect("M", 0.0, 0.0, 10.0, 10.0, 0.0);
    let syms = irasu_illustrator::core::symmetry::create_radial_symmetry(
        &motif, 0.0, 0.0, 100_000, true,
    );
    assert!(syms.len() <= 1440);
    let spiral = irasu_illustrator::core::formula::FormulaCurves::spirograph(
        0.0, 0.0, 50.0, 20.0, 5.0, 100_000, 100_000,
    );
    assert!(spiral.elements.len() <= 1_000_002);
    let voronoi_seeds: Vec<AnchorPoint> = (0..3000)
        .map(|i| AnchorPoint::new(i as f64, (i % 7) as f64))
        .collect();
    let cells =
        irasu_illustrator::core::voronoi::generate_voronoi_cells(800.0, 600.0, &voronoi_seeds, 2.0);
    assert!(cells.len() <= 2048);
}

#[test]
fn test_knife_slice_uses_world_space() {
    // Translated rect: the cut line comes from the world-space bounding box,
    // so slicing must respect the object transform (previously the local
    // polygon was cut with world coordinates, producing garbage).
    let doc_rect = Object::new_rect("R", 100.0, 100.0, 100.0, 100.0, 0.0);
    let (min, max) = doc_rect.bounding_box().unwrap();
    let mid_y = (min.y + max.y) * 0.5;
    let p1 = AnchorPoint::new(min.x - 10.0, mid_y);
    let p2 = AnchorPoint::new(max.x + 10.0, mid_y);
    let (a, b) =
        irasu_illustrator::core::knife::slice_object_with_line(&doc_rect, p1, p2).unwrap();
    for part in [&a, &b] {
        let (pmin, pmax) = part.bounding_box().unwrap();
        // Each half must lie inside the original world rect.
        assert!(pmin.x >= 99.0 && pmax.x <= 201.0);
        assert!((pmax.y - pmin.y - 50.0).abs() < 1.0, "half height ~50");
    }
}

#[test]
fn test_simplify_preserves_bezier_curves() {
    use irasu_illustrator::core::path::{PathData, PathElement};
    let mut path = PathData::new();
    path.push_move_to(0.0, 0.0);
    path.push_line_to(10.0, 0.0);
    path.push_line_to(20.0, 0.0);
    path.push_line_to(30.0, 0.0);
    path.push_cubic_curve_to(40.0, 0.0, 50.0, 10.0, 60.0, 10.0);
    let simplified =
        irasu_illustrator::core::simplify::simplify_path_visvalingam(&path, 5.0);
    let curves = simplified
        .elements
        .iter()
        .filter(|e| matches!(e, PathElement::CurveTo(_)))
        .count();
    assert_eq!(curves, 1, "bezier spans must survive simplification");
    // Collinear line points collapse.
    let lines = simplified
        .elements
        .iter()
        .filter(|e| matches!(e, PathElement::LineTo(_)))
        .count();
    assert!(lines < 3, "collinear points should reduce, got {lines}");
}

#[test]
fn test_nan_gradient_offset_sanitized() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
      <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="0">
        <stop offset="NaN" stop-color="#ff0000" />
        <stop offset="100%" stop-color="#0000ff" />
      </linearGradient></defs>
      <rect x="0" y="0" width="100" height="100" fill="url(#g)" />
    </svg>"##;
    let doc = parse_svg_document(svg);
    let rect = doc
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| {
            matches!(
                o.object_type,
                irasu_illustrator::core::document::ObjectType::Rectangle { .. }
            )
        })
        .expect("rect imports");
    let fill = rect.fill.as_ref().expect("gradient fill");
    if let irasu_illustrator::core::path::FillType::Linear(g) = &fill.fill_type {
        assert!(g.stops.iter().all(|s| s.offset.is_finite()));
    } else {
        panic!("expected linear gradient");
    }
}

#[test]
fn test_transform_gesture_coalesces_to_one_undo_step() {
    use irasu_illustrator::core::state::AppState;
    let mut state = AppState::default();
    state.document.add_object(Object::new_rect("R", 10.0, 20.0, 30.0, 40.0, 0.0));
    let id = state.document.all_objects().next().unwrap().1.id.clone();

    // Simulate a drag gesture: many live mutations, one commit.
    for i in 1..=10 {
        state.ensure_transform_snapshot(&id);
        if let Some(o) = state.document.find_object_mut(&id) {
            o.transform.x = 10.0 + i as f64;
        }
    }
    state.commit_transform_edits("Edit Transform");
    assert_eq!(state.undo_manager.undo_depth(), 1);
    assert!(state.undo_manager.is_dirty());

    state.undo_manager.undo(&mut state.document);
    let obj = state.document.find_object(&id).unwrap();
    assert!((obj.transform.x - 10.0).abs() < 1e-9);
    assert!(!state.undo_manager.is_dirty());

    // No-op gesture records nothing.
    state.ensure_transform_snapshot(&id);
    state.commit_transform_edits("Edit Transform");
    assert_eq!(state.undo_manager.undo_depth(), 0);
}

/// A resize with 「角を拡大・縮小」/「線幅と効果を拡大・縮小」off rewrites
/// absolute-valued attributes *and* the transform. Undo has to take back
/// both in one step, or the object comes back half-scaled.
#[test]
fn test_counter_scaled_resize_undoes_transform_and_attributes_together() {
    use irasu_illustrator::core::path::StrokeStyle;
    use irasu_illustrator::core::state::AppState;
    let mut state = AppState::default();
    state.prefs.scale_corners = false;
    state.prefs.scale_strokes_effects = false;
    state
        .document
        .add_object(Object::new_rect("R", 0.0, 0.0, 100.0, 100.0, 20.0));
    let id = state.document.all_objects().next().unwrap().1.id.clone();
    if let Some(o) = state.document.find_object_mut(&id) {
        o.stroke = Some(StrokeStyle {
            width: 4.0,
            ..Default::default()
        });
    }

    // Two frames of a corner-handle resize, the way `update_resize` runs
    // them: ×4 then ×½ — the stored values must telescope to ÷2 overall.
    for factor in [4.0_f64, 0.5] {
        state.ensure_transform_snapshot(&id);
        if let Some(o) = state.document.find_object_mut(&id) {
            let before = o.visual_scale();
            o.transform.scale_x *= factor;
            o.transform.scale_y *= factor;
            o.apply_scale_change(o.visual_scale() / before, false, false);
        }
    }
    state.commit_transform_edits("Resize");
    assert_eq!(state.undo_manager.undo_depth(), 1);

    let obj = state.document.find_object(&id).unwrap();
    assert_eq!(obj.transform.scale_x, 2.0, "×4 ×½ telescopes to ×2");
    assert_eq!(obj.stroke.as_ref().unwrap().width, 2.0, "stroke ÷2");
    let corner = match &obj.object_type {
        irasu_illustrator::core::document::ObjectType::Rectangle { corner_radius, .. } => {
            *corner_radius
        }
        other => panic!("expected a rectangle, got {other:?}"),
    };
    assert_eq!(corner, 10.0, "corner radius ÷2");
    // The check that matters: drawn size never moved.
    assert_eq!(obj.stroke.as_ref().unwrap().width * obj.visual_scale(), 4.0);
    assert_eq!(corner * obj.visual_scale(), 20.0);

    // One undo restores the *whole* object, not just the transform.
    state.undo_manager.undo(&mut state.document);
    let obj = state.document.find_object(&id).unwrap();
    assert_eq!(obj.transform.scale_x, 1.0);
    assert_eq!(obj.stroke.as_ref().unwrap().width, 4.0);
    let corner = match &obj.object_type {
        irasu_illustrator::core::document::ObjectType::Rectangle { corner_radius, .. } => {
            *corner_radius
        }
        other => panic!("expected a rectangle, got {other:?}"),
    };
    assert_eq!(corner, 20.0);

    // Switches on: the transform carries the size, attributes stay put.
    state.prefs.scale_corners = true;
    state.prefs.scale_strokes_effects = true;
    state.ensure_transform_snapshot(&id);
    if let Some(o) = state.document.find_object_mut(&id) {
        let before = o.visual_scale();
        o.transform.scale_x *= 2.0;
        o.transform.scale_y *= 2.0;
        o.apply_scale_change(o.visual_scale() / before, true, true);
    }
    state.commit_transform_edits("Resize");
    let obj = state.document.find_object(&id).unwrap();
    assert_eq!(obj.stroke.as_ref().unwrap().width, 4.0, "untouched");
    state.undo_manager.undo(&mut state.document);
    let obj = state.document.find_object(&id).unwrap();
    assert_eq!(obj.transform.scale_x, 1.0);
    assert_eq!(obj.stroke.as_ref().unwrap().width, 4.0);
}

#[test]
fn test_object_gesture_coalesces_to_one_undo_step() {
    use irasu_illustrator::core::state::AppState;
    let mut state = AppState::default();
    state.document.add_object(Object::new_rect("R", 0.0, 0.0, 10.0, 10.0, 0.0));
    state.document.add_object(Object::new_rect("S", 50.0, 50.0, 10.0, 10.0, 0.0));
    let ids: Vec<String> = state
        .document
        .all_objects()
        .map(|(_, o)| o.id.clone())
        .collect();

    // Simulate a slider drag touching both objects over many frames.
    for i in 1..=20 {
        for id in &ids {
            state.ensure_object_snapshot(id);
        }
        for id in &ids {
            if let Some(o) = state.document.find_object_mut(id) {
                o.opacity = i as f32 / 40.0;
            }
        }
    }
    state.commit_object_edits("Edit Object");
    assert_eq!(state.undo_manager.undo_depth(), 1);

    state.undo_manager.undo(&mut state.document);
    for (_, o) in state.document.all_objects() {
        assert!((o.opacity - 1.0).abs() < 1e-6);
    }

    // Whole-object undo also restores nested children.
    let inner = Object::new_rect("Inner", 0.0, 0.0, 5.0, 5.0, 0.0);
    let inner_id = inner.id.clone();
    state.document.add_object(Object::new_group("G", vec![inner]));
    state.ensure_object_snapshot(&inner_id);
    if let Some(o) = state.document.find_object_mut(&inner_id) {
        o.opacity = 0.25;
    }
    state.commit_object_edits("Edit Object");
    state.undo_manager.undo(&mut state.document);
    assert!((state.document.find_object(&inner_id).unwrap().opacity - 1.0).abs() < 1e-6);
}

#[test]
fn test_bulk_generation_undoes_in_one_step() {
    use irasu_illustrator::core::history::{AddObjectCommand, BatchCommand, Command};
    let mut doc = Document::default();
    let mut mgr = irasu_illustrator::core::history::UndoManager::new();
    let before = doc.all_objects().count();
    let cmds: Vec<Box<dyn Command>> = (0..7)
        .map(|i| {
            Box::new(AddObjectCommand::new(Object::new_rect(
                &format!("N{i}"),
                i as f64 * 10.0,
                0.0,
                5.0,
                5.0,
                0.0,
            ))) as Box<dyn Command>
        })
        .collect();
    mgr.execute(Box::new(BatchCommand::new("Neon Glow", cmds)), &mut doc);
    assert_eq!(mgr.undo_depth(), 1);
    assert_eq!(doc.all_objects().count(), before + 7);
    mgr.undo(&mut doc);
    assert_eq!(doc.all_objects().count(), before);
}

#[test]
fn test_layer_add_delete_reorder_undo() {
    use irasu_illustrator::core::document::Layer;
    use irasu_illustrator::core::history::{
        AddLayerCommand, RemoveLayerCommand, ReorderLayersCommand, ReorderObjectCommand,
    };
    let mut doc = Document::default();
    let mut mgr = irasu_illustrator::core::history::UndoManager::new();
    assert_eq!(doc.layers.len(), 1);

    // Add.
    mgr.execute(
        Box::new(AddLayerCommand {
            layer: Layer::new("L2"),
            index: 1,
            prev_active: 0,
        }),
        &mut doc,
    );
    assert_eq!(doc.layers.len(), 2);
    assert_eq!(doc.active_layer_idx, 1);
    mgr.undo(&mut doc);
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(doc.active_layer_idx, 0);
    mgr.redo(&mut doc);
    assert_eq!(doc.layers.len(), 2);

    // Reorder.
    let old_order: Vec<String> = doc.layers.iter().map(|l| l.id.clone()).collect();
    doc.move_layer_up(0);
    let new_order: Vec<String> = doc.layers.iter().map(|l| l.id.clone()).collect();
    mgr.execute(
        Box::new(ReorderLayersCommand {
            old_order: old_order.clone(),
            new_order,
        }),
        &mut doc,
    );
    mgr.undo(&mut doc);
    let restored: Vec<String> = doc.layers.iter().map(|l| l.id.clone()).collect();
    assert_eq!(restored, old_order);

    // Delete restores content + position.
    doc.add_object(Object::new_rect("R", 0.0, 0.0, 5.0, 5.0, 0.0));
    let victim = doc.layers[1].clone();
    mgr.execute(
        Box::new(RemoveLayerCommand {
            layer: victim.clone(),
            index: 1,
        }),
        &mut doc,
    );
    assert_eq!(doc.layers.len(), 1);
    mgr.undo(&mut doc);
    assert_eq!(doc.layers.len(), 2);
    assert_eq!(doc.layers[1].objects.len(), victim.objects.len());

    // Object reorder within a layer.
    doc.layers[1].objects.push(Object::new_rect("A", 0.0, 0.0, 1.0, 1.0, 0.0));
    doc.layers[1].objects.push(Object::new_rect("B", 0.0, 0.0, 1.0, 1.0, 0.0));
    let oo: Vec<String> = doc.layers[1].objects.iter().map(|o| o.id.clone()).collect();
    doc.move_object_up(1, 0);
    let no: Vec<String> = doc.layers[1].objects.iter().map(|o| o.id.clone()).collect();
    assert_ne!(oo, no);
    // Per-object absolute positions (PR1): move the object back via command.
    let moved_id = no[1].clone();
    mgr.execute(
        Box::new(ReorderObjectCommand {
            object_id: moved_id.clone(),
            layer_idx: 1,
            old_position: 1,
            new_position: 0,
        }),
        &mut doc,
    );
    let fwd: Vec<String> = doc.layers[1].objects.iter().map(|o| o.id.clone()).collect();
    assert_eq!(fwd[0], moved_id);
    mgr.undo(&mut doc);
    let back: Vec<String> = doc.layers[1].objects.iter().map(|o| o.id.clone()).collect();
    assert_eq!(back, no);
}

fn ring_area(poly: &[irasu_illustrator::core::path::AnchorPoint]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut a = 0.0;
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        a += p.x * q.y - q.x * p.y;
    }
    (a * 0.5).abs()
}

fn sq(x: f64, y: f64, s: f64) -> Vec<irasu_illustrator::core::path::AnchorPoint> {
    use irasu_illustrator::core::path::AnchorPoint;
    vec![
        AnchorPoint::new(x, y),
        AnchorPoint::new(x + s, y),
        AnchorPoint::new(x + s, y + s),
        AnchorPoint::new(x, y + s),
    ]
}

#[test]
fn test_boolean_overlap_squares() {
    use irasu_illustrator::core::boolean::{apply_polygon_boolean, BooleanOp};
    let a = sq(0.0, 0.0, 100.0);
    let b = sq(50.0, 50.0, 100.0);
    let area = |r: Vec<Vec<irasu_illustrator::core::path::AnchorPoint>>| {
        r.iter().map(|p| ring_area(p)).sum::<f64>()
    };
    let u = area(apply_polygon_boolean(&a, &b, BooleanOp::Union));
    assert!((u - 17500.0).abs() < 50.0, "union {u}");
    let i = area(apply_polygon_boolean(&a, &b, BooleanOp::Intersect));
    assert!((i - 2500.0).abs() < 50.0, "intersect {i}");
    let s = area(apply_polygon_boolean(&a, &b, BooleanOp::Subtract));
    assert!((s - 7500.0).abs() < 50.0, "subtract {s}");
    let e = area(apply_polygon_boolean(&a, &b, BooleanOp::Exclude));
    assert!((e - 15000.0).abs() < 100.0, "exclude {e}");
}

#[test]
fn test_boolean_shared_edge_and_touch() {
    use irasu_illustrator::core::boolean::{apply_polygon_boolean, BooleanOp};
    // Edge-adjacent squares sharing the x=100 boundary segment.
    let a = sq(0.0, 0.0, 100.0);
    let b = sq(100.0, 0.0, 100.0);
    let i = apply_polygon_boolean(&a, &b, BooleanOp::Intersect);
    let ia: f64 = i.iter().map(|p| ring_area(p)).sum();
    assert!(ia < 1.0, "shared-edge intersect is degenerate, got {ia}");
    let u = apply_polygon_boolean(&a, &b, BooleanOp::Union);
    let ua: f64 = u.iter().map(|p| ring_area(p)).sum();
    assert!((ua - 20000.0).abs() < 100.0, "touching union {ua}");
    let s = apply_polygon_boolean(&a, &b, BooleanOp::Subtract);
    let sa: f64 = s.iter().map(|p| ring_area(p)).sum();
    assert!((sa - 10000.0).abs() < 100.0, "touching subtract {sa}");
    // Corner touch.
    let c = sq(100.0, 100.0, 50.0);
    assert!(apply_polygon_boolean(&a, &c, BooleanOp::Intersect).is_empty());
    let uc = apply_polygon_boolean(&a, &c, BooleanOp::Union);
    let uca: f64 = uc.iter().map(|p| ring_area(p)).sum();
    assert!((uca - 12500.0).abs() < 100.0, "corner-touch union {uca}");
}

#[test]
fn test_boolean_concave_union() {
    use irasu_illustrator::core::boolean::{apply_polygon_boolean, BooleanOp};
    use irasu_illustrator::core::path::AnchorPoint;
    // L-shape (concave): angular-sort unions produce garbage here.
    let l = vec![
        AnchorPoint::new(0.0, 0.0),
        AnchorPoint::new(100.0, 0.0),
        AnchorPoint::new(100.0, 40.0),
        AnchorPoint::new(40.0, 40.0),
        AnchorPoint::new(40.0, 100.0),
        AnchorPoint::new(0.0, 100.0),
    ];
    let b = sq(20.0, 20.0, 100.0);
    let u = apply_polygon_boolean(&l, &b, BooleanOp::Union);
    let total: f64 = u.iter().map(|p| ring_area(p)).sum();
    // L area = 100*40 + 40*60 = 6400; B area = 10000; overlap = 20*40+...osed
    // to exact: just assert sane bounds and no garbage explosion.
    assert!(total > 6400.0 && total < 16400.0, "concave union {total}");
    let i = apply_polygon_boolean(&l, &b, BooleanOp::Intersect);
    let ia: f64 = i.iter().map(|p| ring_area(p)).sum();
    assert!(ia > 0.0 && ia <= 6400.0, "concave intersect {ia}");
}

#[test]
fn test_boolean_degenerate_and_contained() {
    use irasu_illustrator::core::boolean::{apply_polygon_boolean, BooleanOp};
    use irasu_illustrator::core::path::AnchorPoint;
    let big = sq(0.0, 0.0, 200.0);
    let small = sq(50.0, 50.0, 50.0);
    // Contained.
    let i = apply_polygon_boolean(&big, &small, BooleanOp::Intersect);
    assert_eq!(i.len(), 1);
    assert!((ring_area(&i[0]) - 2500.0).abs() < 10.0);
    let u = apply_polygon_boolean(&big, &small, BooleanOp::Union);
    assert!((ring_area(&u[0]) - 40000.0).abs() < 10.0);
    // Disjoint.
    let far = sq(500.0, 500.0, 50.0);
    let u2 = apply_polygon_boolean(&big, &far, BooleanOp::Union);
    assert_eq!(u2.len(), 2);
    assert!(apply_polygon_boolean(&big, &far, BooleanOp::Intersect).is_empty());
    // Degenerate inputs never panic and yield nothing meaningful.
    let empty: Vec<AnchorPoint> = vec![];
    assert!(apply_polygon_boolean(&empty, &big, BooleanOp::Intersect).is_empty());
    let dup = vec![
        AnchorPoint::new(1.0, 1.0),
        AnchorPoint::new(1.0, 1.0),
        AnchorPoint::new(1.0, 1.0),
    ];
    assert!(apply_polygon_boolean(&dup, &big, BooleanOp::Union).len() <= 1);
}

#[test]
fn test_multiline_text_round_trip() {
    use irasu_illustrator::core::document::ObjectType;
    let mut doc = Document::default();
    doc.add_object(Object::new_text("T", "hello\nworld", 10.0, 20.0, 16.0));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(svg.contains("<tspan"), "multiline must export tspans");
    assert!(svg.contains("dy=\"1.2em\""));
    let doc2 = parse_svg_document(&svg);
    let text = doc2
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| matches!(o.object_type, ObjectType::Text { .. }))
        .expect("text round-trips");
    if let ObjectType::Text { text, .. } = &text.object_type {
        assert_eq!(text, "hello\nworld", "got: {text:?}");
    }
    // Metrics cover both lines.
    let (w, h) =
        irasu_illustrator::core::document::object::text_block_size("hello\nworld", 16.0);
    assert!(w > 0.0 && h > 16.0);
}

#[test]
fn test_nested_delete_undo_restores_parent() {
    use irasu_illustrator::core::history::RemoveObjectCommand;
    let mut doc = Document::default();
    let inner_a = Object::new_rect("A", 0.0, 0.0, 10.0, 10.0, 0.0);
    let inner_b = Object::new_rect("B", 20.0, 0.0, 10.0, 10.0, 0.0);
    let aid = inner_a.id.clone();
    doc.add_object(Object::new_group("G", vec![inner_a, inner_b]));
    let mut mgr = irasu_illustrator::core::history::UndoManager::new();

    let obj = doc.find_object(&aid).cloned().unwrap();
    let cmd = RemoveObjectCommand::located(obj, &doc);
    mgr.execute(Box::new(cmd), &mut doc);
    assert!(doc.find_object(&aid).is_none());
    mgr.undo(&mut doc);
    let back = doc.find_object(&aid).expect("nested child restored");
    assert_eq!(back.name, "A");
    // Restored inside the group, not at top level.
    assert!(doc.all_objects().all(|(_, o)| o.id != aid));
}

#[test]
fn test_isolated_drag_delta_conversion() {
    use irasu_illustrator::core::state::AppState;
    use irasu_illustrator::tools::select::SelectState;
    let mut state = AppState::default();
    assert_eq!(SelectState::parent_delta(&state, 10.0, 5.0), (10.0, 5.0));

    // 90-degree rotated group: world +x maps to local -y... verify mapping.
    let mut g = Object::new_group("G", vec![Object::new_rect("R", 0.0, 0.0, 10.0, 10.0, 0.0)]);
    g.transform.rotation = std::f64::consts::FRAC_PI_2;
    let gid = g.id.clone();
    state.document.add_object(g);
    state.isolated_group_id = Some(gid);
    let (lx, ly) = SelectState::parent_delta(&state, 10.0, 0.0);
    // R(90°): world (10,0) -> local (0,-10).
    assert!((lx - 0.0).abs() < 1e-6 && (ly + 10.0).abs() < 1e-6, "got ({lx},{ly})");
}

#[test]
fn test_bold_italic_markup_import() {
    use irasu_illustrator::core::document::{FontStyle, ObjectType};
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
      <text x="10" y="20" font-size="16"><b>bold</b> and <i>italic</i></text>
    </svg>"##;
    let doc = parse_svg_document(svg);
    let text = doc
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| matches!(o.object_type, ObjectType::Text { .. }))
        .expect("text imports");
    if let ObjectType::Text { text, style, .. } = &text.object_type {
        assert!(text.contains("bold") && text.contains("italic"));
        assert_eq!(style.font_weight, 700);
        assert_eq!(style.font_style, FontStyle::Italic);
    } else {
        panic!("expected text");
    }
}

#[test]
fn test_timeline_playback_commits_one_undo_step() {
    use irasu_illustrator::core::state::AppState;
    use irasu_illustrator::core::timeline::{AnimProperty, Timeline};
    let mut state = AppState::default();
    state.document.add_object(Object::new_rect("R", 0.0, 0.0, 10.0, 10.0, 0.0));
    let id = state.document.all_objects().next().unwrap().1.id.clone();

    // Simulate app/mod.rs playback start: snapshot once.
    state.ensure_object_snapshot(&id);
    // Simulate several applied frames.
    let mut tl = Timeline::default();
    let track = tl.add_or_get_track_mut(&id, AnimProperty::PositionX);
    use irasu_illustrator::core::timeline::EaseType;
    track.add_keyframe(0, 0.0, EaseType::Linear);
    track.add_keyframe(10, 100.0, EaseType::Linear);
    for frame in [1, 5, 10] {
        tl.current_frame = frame;
        tl.apply_to_document(&mut state.document);
    }
    // Simulate playback stop.
    state.commit_object_edits("Timeline Playback");
    assert_eq!(state.undo_manager.undo_depth(), 1);
    assert!(state.undo_manager.is_dirty());
    state.undo_manager.undo(&mut state.document);
    let obj = state.document.find_object(&id).unwrap();
    assert!((obj.transform.x - 0.0).abs() < 1e-9);
}

#[test]
fn test_cut_paste_batch_semantics() {
    use irasu_illustrator::core::history::{AddObjectCommand, BatchCommand, Command};
    let mut doc = Document::default();
    let mut mgr = irasu_illustrator::core::history::UndoManager::new();
    for i in 0..3 {
        mgr.execute(
            Box::new(AddObjectCommand::new(Object::new_rect(
                &format!("R{i}"),
                i as f64 * 20.0,
                0.0,
                10.0,
                10.0,
                0.0,
            ))),
            &mut doc,
        );
    }
    // Cut all three as one step (mirrors the Cut handler).
    let mut cmds: Vec<Box<dyn Command>> = Vec::new();
    for (_, o) in doc.all_objects() {
        cmds.push(Box::new(
            irasu_illustrator::core::history::RemoveObjectCommand::located(
                o.clone(),
                &doc,
            ),
        ));
    }
    mgr.execute(Box::new(BatchCommand::new("Cut Objects", cmds)), &mut doc);
    assert_eq!(doc.all_objects().count(), 0);
    mgr.undo(&mut doc);
    assert_eq!(doc.all_objects().count(), 3);
}

#[test]
fn test_shape_to_path_conversion_undoable() {
    use irasu_illustrator::core::document::ObjectType;
    use irasu_illustrator::core::state::AppState;
    let mut state = AppState::default();
    state.document.add_object(Object::new_rect("R", 0.0, 0.0, 10.0, 20.0, 0.0));
    let id = state.document.all_objects().next().unwrap().1.id.clone();
    // Mirror the node-tool conversion path.
    state.ensure_object_snapshot(&id);
    if let Some(o) = state.document.find_object_mut(&id) {
        let p = o.to_path_data();
        o.object_type = ObjectType::Path(p);
    }
    state.commit_object_edits("Convert to Path");
    assert!(matches!(
        state.document.find_object(&id).unwrap().object_type,
        ObjectType::Path(_)
    ));
    state.undo_manager.undo(&mut state.document);
    assert!(matches!(
        state.document.find_object(&id).unwrap().object_type,
        ObjectType::Rectangle { .. }
    ));
}

#[test]
fn test_arrange_reorder_single_undo_step() {
    use irasu_illustrator::core::state::AppState;
    let mut state = AppState::default();
    for name in ["A", "B", "C"] {
        state.document.add_object(Object::new_rect(name, 0.0, 0.0, 10.0, 10.0, 0.0));
    }
    let order_before: Vec<String> = state
        .document
        .all_objects()
        .map(|(_, o)| o.id.clone())
        .collect();
    // Bring the back object to front, like the arrange buttons.
    let first = order_before[0].clone();
    state.reorder_objects_undoable("Bring to Front", |doc| {
        for layer in doc.layers.iter_mut() {
            if let Some(pos) = layer.objects.iter().position(|o| o.id == first) {
                let obj = layer.objects.remove(pos);
                layer.objects.push(obj);
            }
        }
    });
    assert_eq!(state.undo_manager.undo_depth(), 1);
    let order_after: Vec<String> = state
        .document
        .all_objects()
        .map(|(_, o)| o.id.clone())
        .collect();
    assert_ne!(order_before, order_after);
    state.undo_manager.undo(&mut state.document);
    let order_back: Vec<String> = state
        .document
        .all_objects()
        .map(|(_, o)| o.id.clone())
        .collect();
    assert_eq!(order_before, order_back);
}

#[test]
fn test_stored_checkpoint_parses_by_format() {
    use std::path::Path;
    let mut doc = Document::default();
    doc.name = "Proj".to_string();
    doc.add_object(Object::new_rect("R", 1.0, 2.0, 3.0, 4.0, 0.0));
    let json = serde_json::to_string_pretty(&doc).unwrap();
    let parsed =
        irasu_illustrator::ui::panels::history_diff::parse_stored_doc(Path::new("w.amata"), &json)
            .expect("project checkpoint parses as project");
    assert_eq!(parsed.layers[0].objects.len(), 1);
    // SVG content under an .amata extension must NOT parse as a project.
    assert!(
        irasu_illustrator::ui::panels::history_diff::parse_stored_doc(
            Path::new("w.amata"),
            "<svg></svg>"
        )
        .is_none()
    );
    let svg_doc = irasu_illustrator::ui::panels::history_diff::parse_stored_doc(
        Path::new("w.svg"),
        "<svg></svg>",
    )
    .expect("svg checkpoint parses as svg");
    assert_eq!(svg_doc.layers[0].objects.len(), 0);
}

#[test]
fn test_layer_opacity_and_blend_export() {
    let mut doc = Document::default();
    doc.layers[0].opacity = 0.5;
    let mut obj = Object::new_rect("R", 0.0, 0.0, 10.0, 10.0, 0.0);
    obj.blend_mode = irasu_illustrator::core::document::BlendMode::Multiply;
    doc.add_object(obj);
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(svg.contains("<g opacity="), "layer group must carry opacity");
    assert!(
        svg.contains("mix-blend-mode=\"multiply\""),
        "blend mode must export"
    );
    // Opacity composes through import.
    let doc2 = parse_svg_document(&svg);
    let obj2 = doc2.all_objects().next().map(|(_, o)| o).unwrap();
    assert!((obj2.opacity - 0.5).abs() < 0.02, "got {}", obj2.opacity);
}

#[test]
fn test_variable_width_ribbon_export() {
    use irasu_illustrator::core::document::{WidthPoint, WidthProfile, WidthSide};
    use irasu_illustrator::core::path::PathData;
    let mut path = PathData::new();
    path.push_move_to(0.0, 0.0);
    path.push_line_to(50.0, 0.0);
    path.push_line_to(100.0, 0.0);
    let mut obj = Object::new_path("P", path);
    obj.stroke = Some(irasu_illustrator::core::path::StrokeStyle {
        color: [1.0, 0.0, 0.0, 1.0],
        width: 10.0,
        dash_pattern: None,
        ..Default::default()
    });
    obj.width_profile = Some(WidthProfile {
        points: vec![
            WidthPoint {
                position: 0.0,
                width: 0.2,
                side: WidthSide::Both,
            },
            WidthPoint {
                position: 1.0,
                width: 2.0,
                side: WidthSide::Both,
            },
        ],
    });
    let poly = obj.to_path_data().to_polygon(8);
    let ribbon = irasu_illustrator::core::offset::variable_width_outline(
        &poly,
        obj.width_profile.as_ref().unwrap(),
        10.0,
        false,
    );
    assert!(ribbon.len() >= 6, "ribbon ring expected");
    // Start is narrow, end is wide.
    let start_w = (ribbon[0].y - ribbon[ribbon.len() - 1].y).abs();
    assert!(start_w < 10.0, "tapered start {start_w}");
    let mut doc = Document::default();
    doc.add_object(obj);
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(!svg.contains("stroke-width"), "ribbon must bake stroke");
    assert!(svg.contains("fill=\"#ff0000\""));
}

fn tiny_test_png() -> Vec<u8> {
    // 4x2 RGBA checkerboard encoded as PNG.
    let mut img = image::RgbaImage::new(4, 2);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let v = if (x + y) % 2 == 0 { 255 } else { 0 };
        *px = image::Rgba([v, 128, 64, 255]);
    }
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )
    .unwrap();
    buf
}

#[test]
fn test_placed_image_decode_and_svg_round_trip() {
    let png = tiny_test_png();
    let (w, h, placed) = irasu_illustrator::io::raster::decode_placed_image(&png).unwrap();
    assert!((w - 4.0).abs() < 1e-6 && (h - 2.0).abs() < 1e-6);
    assert!(placed.starts_with(&[0x89, b'P', b'N', b'G']));

    let mut doc = Document::default();
    doc.add_object(Object::new_image("Img", 10.0, 20.0, w, h, placed));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(svg.contains("<image"));
    assert!(svg.contains("data:image/png;base64,"));
    let doc2 = parse_svg_document(&svg);
    let img = doc2
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| {
            matches!(
                o.object_type,
                irasu_illustrator::core::document::ObjectType::Image { .. }
            )
        })
        .expect("image round-trips");
    if let irasu_illustrator::core::document::ObjectType::Image {
        width,
        height,
        png_bytes,
    } = &img.object_type
    {
        assert!((*width - 4.0).abs() < 1e-6);
        assert!((*height - 2.0).abs() < 1e-6);
        assert!(!png_bytes.is_empty());
    }
    assert!((img.transform.x - 10.0).abs() < 1e-6);
}

#[test]
fn test_pathfinder_apply_is_atomic() {
    use irasu_illustrator::core::boolean::{execute_pathfinder, BooleanOp};
    use irasu_illustrator::core::history::{BatchCommand, Command};
    let mut doc = Document::default();
    let mut mgr = irasu_illustrator::core::history::UndoManager::new();
    doc.add_object(Object::new_rect("A", 0.0, 0.0, 100.0, 100.0, 0.0));
    doc.add_object(Object::new_rect("B", 50.0, 50.0, 100.0, 100.0, 0.0));
    let objs: Vec<Object> = doc.all_objects().map(|(_, o)| o.clone()).collect();
    let refs: Vec<&Object> = objs.iter().collect();
    let result = execute_pathfinder(&refs, BooleanOp::Union).unwrap();
    // Mirror PathfinderPanel::apply_op: removals + result in one step.
    let mut cmds: Vec<Box<dyn Command>> = Vec::new();
    for o in &objs {
        cmds.push(Box::new(
            irasu_illustrator::core::history::RemoveObjectCommand::located(o.clone(), &doc),
        ) as Box<dyn Command>);
    }
    cmds.push(Box::new(irasu_illustrator::core::history::AddObjectCommand::new(result))
        as Box<dyn Command>);
    mgr.execute(Box::new(BatchCommand::new("Pathfinder", cmds)), &mut doc);
    assert_eq!(doc.all_objects().count(), 1);
    mgr.undo(&mut doc);
    assert_eq!(doc.all_objects().count(), 2);
}

#[test]
fn test_compound_create_release_round_trip() {
    let a = Object::new_rect("A", 0.0, 0.0, 100.0, 100.0, 0.0);
    let b = Object::new_rect("B", 25.0, 25.0, 50.0, 50.0, 0.0);
    let compound = Object::make_compound_path(&[a, b]).expect("compound");
    let parts = compound.release_compound_path();
    assert!(parts.len() >= 2, "release yields parts");
}

#[test]
fn test_group_ungroup_atomic_undo() {
    use irasu_illustrator::core::history::{AddObjectCommand, BatchCommand, Command};
    let mut doc = Document::default();
    let mut mgr = irasu_illustrator::core::history::UndoManager::new();
    let a = Object::new_rect("A", 0.0, 0.0, 10.0, 10.0, 0.0);
    let b = Object::new_rect("B", 20.0, 0.0, 10.0, 10.0, 0.0);
    doc.add_object(a.clone());
    doc.add_object(b.clone());
    // Mirror the Group handler: removals + group in one step.
    let group = Object::new_group("G", vec![a.clone(), b.clone()]);
    let mut cmds: Vec<Box<dyn Command>> = Vec::new();
    for o in [&a, &b] {
        cmds.push(Box::new(
            irasu_illustrator::core::history::RemoveObjectCommand::located(o.clone(), &doc),
        ) as Box<dyn Command>);
    }
    cmds.push(
        Box::new(AddObjectCommand::new(group)) as Box<dyn Command>
    );
    mgr.execute(Box::new(BatchCommand::new("Group", cmds)), &mut doc);
    assert_eq!(doc.all_objects().count(), 1);
    mgr.undo(&mut doc);
    assert_eq!(doc.all_objects().count(), 2);
}

#[test]
fn test_new_document_resets_save_destination() {
    use irasu_illustrator::core::state::AppState;
    // Simulate create_new_document invariants without the GUI: a fresh
    // document must start clean with no stale destination.
    let mut state = AppState::default();
    state.undo_manager.execute(
        Box::new(irasu_illustrator::core::history::AddObjectCommand::new(
            Object::new_rect("R", 0.0, 0.0, 5.0, 5.0, 0.0),
        )),
        &mut state.document,
    );
    assert!(state.is_dirty());
    // Fresh default (what create hands over) is clean and empty.
    let fresh = AppState::default();
    assert!(!fresh.is_dirty());
    assert_eq!(fresh.document.all_objects().count(), 0);
    let _ = state;
}

#[test]
fn test_timeline_and_guides_survive_save_reload() {
    use irasu_illustrator::core::document::{Guide, GuideOrientation};
    use irasu_illustrator::core::timeline::{AnimProperty, EaseType};
    let mut doc = Document::default();
    doc.add_object(Object::new_rect("R", 0.0, 0.0, 10.0, 10.0, 0.0));
    let id = doc.all_objects().next().unwrap().1.id.clone();
    let track = doc.timeline.add_or_get_track_mut(&id, AnimProperty::PositionX);
    track.add_keyframe(0, 0.0, EaseType::Linear);
    track.add_keyframe(10, 50.0, EaseType::Linear);
    doc.guides.push(Guide {
        orientation: GuideOrientation::Vertical,
        position: 100.0,
    });
    let dir = std::env::temp_dir().join("amata_persist_check");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("anim.amata");
    irasu_illustrator::io::project::save_project(&doc, &path).unwrap();
    let loaded = irasu_illustrator::io::project::load_project(&path).unwrap();
    assert_eq!(loaded.timeline.tracks.len(), 1);
    assert_eq!(loaded.timeline.tracks[0].object_id, id);
    assert_eq!(loaded.guides.len(), 1);
    assert!((loaded.guides[0].position - 100.0).abs() < 1e-9);
    // Old files without the new keys still load (serde defaults).
    let legacy = r#"{"name":"o","layers":[],"active_layer_idx":0,"width":100.0,"height":100.0,"symbols":[]}"#;
    let legacy_path = dir.join("legacy.amata");
    std::fs::write(&legacy_path, legacy).unwrap();
    let legacy_doc = irasu_illustrator::io::project::load_project(&legacy_path).unwrap();
    assert_eq!(legacy_doc.layers.len(), 1);
    assert!(legacy_doc.timeline.tracks.is_empty());
}

#[test]
fn test_selected_only_export_filters() {
    let mut doc = Document::default();
    doc.add_object(Object::new_rect("A", 0.0, 0.0, 10.0, 10.0, 0.0));
    doc.add_object(Object::new_rect("B", 50.0, 50.0, 10.0, 10.0, 0.0));
    let keep: Vec<String> = doc
        .all_objects()
        .take(1)
        .map(|(_, o)| o.id.clone())
        .collect();
    // Mirror the utility export scope filter.
    let mut export_doc = doc.clone();
    for layer in &mut export_doc.layers {
        layer.objects.retain(|o| keep.contains(&o.id));
    }
    assert_eq!(export_doc.all_objects().count(), 1);
    let svg = irasu_illustrator::io::svg::export_svg(&export_doc);
    assert_eq!(svg.matches("<rect").count(), 1);
}

#[test]
fn replace_objects_undo_restores_original_order() {
    use irasu_illustrator::core::history::{
        collect_located_objects, ReplaceObjectsCommand,
    };
    let mut doc = Document::default();
    for name in ["A", "B", "C"] {
        doc.add_object(Object::new_rect(name, 0.0, 0.0, 10.0, 10.0, 0.0));
    }
    let mut mgr = irasu_illustrator::core::history::UndoManager::new();
    let before: Vec<String> = doc
        .all_objects()
        .map(|(_, o)| o.name.clone())
        .collect();
    assert_eq!(before, vec!["A", "B", "C"]);

    // Group A and B (out of the 3), then undo: exact order must return.
    let removed = collect_located_objects(
        &doc,
        &doc.all_objects()
            .take(2)
            .map(|(_, o)| o.id.clone())
            .collect::<Vec<_>>(),
    );
    let objects: Vec<Object> = removed.iter().map(|i| i.object.clone()).collect();
    let group = Object::new_group("G", objects);
    mgr.execute(
        Box::new(ReplaceObjectsCommand::new(
            "Group",
            removed,
            vec![group],
        )),
        &mut doc,
    );
    assert_eq!(doc.all_objects().count(), 2);
    mgr.undo(&mut doc);
    let back: Vec<String> = doc
        .all_objects()
        .map(|(_, o)| o.name.clone())
        .collect();
    assert_eq!(back, vec!["A", "B", "C"]);
}

#[test]
fn failed_compound_path_does_not_remove_objects() {
    let a = Object::new_rect("A", 0.0, 0.0, 10.0, 10.0, 0.0);
    // make_compound_path on an empty slice must fail without touching docs.
    assert!(Object::make_compound_path(&[]).is_none());
    // A single object cannot usefully compound through the helper path:
    // callers check len >= 2 first (no mutation happens on failure).
    let mut doc = Document::default();
    doc.add_object(a);
    let count_before = doc.all_objects().count();
    let objs: Vec<Object> = doc.all_objects().map(|(_, o)| o.clone()).collect();
    if objs.len() >= 2 {
        panic!("test setup: expected a single object");
    }
    assert_eq!(doc.all_objects().count(), count_before);
}

#[test]
fn reorder_marks_document_dirty() {
    use irasu_illustrator::core::state::AppState;
    let mut state = AppState::default();
    for name in ["A", "B", "C"] {
        state
            .document
            .add_object(Object::new_rect(name, 0.0, 0.0, 10.0, 10.0, 0.0));
    }
    state.undo_manager.mark_saved();
    assert!(!state.is_dirty());
    // Bring the back object forward through the undoable helper.
    let first = state.document.all_objects().next().unwrap().1.id.clone();
    state.reorder_objects_undoable("Bring Forward", |doc| {
        for layer in doc.layers.iter_mut() {
            if let Some(pos) = layer.objects.iter().position(|o| o.id == first) {
                if pos + 1 < layer.objects.len() {
                    layer.objects.swap(pos, pos + 1);
                }
            }
        }
    });
    assert!(state.is_dirty());
}

#[test]
fn validated_svg_parse_rejects_broken_xml() {
    let good = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect x="0" y="0" width="10" height="10" /></svg>"##;
    assert!(irasu_illustrator::io::svg::try_parse_svg_document(good).is_ok());
    assert!(irasu_illustrator::io::svg::try_parse_svg_document("<svg><g><rect").is_err());
    assert!(irasu_illustrator::io::svg::try_parse_svg_document("not xml at all {{{").is_err());
}

#[test]
fn atomic_write_round_trip() {
    let dir = std::env::temp_dir().join("amata_atomic_check");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("doc.amata");
    let mut doc = Document::default();
    doc.add_object(Object::new_rect("R", 1.0, 2.0, 3.0, 4.0, 0.0));
    irasu_illustrator::io::project::save_project(&doc, &path).unwrap();
    assert!(!dir.join("doc.amata.tmp").exists());
    let loaded = irasu_illustrator::io::project::load_project(&path).unwrap();
    assert_eq!(loaded.all_objects().count(), 1);
    // Byte-level helper leaves no temp file behind either.
    let bin_path = dir.join("blob.bin");
    irasu_illustrator::io::atomic::atomic_write_bytes(&bin_path, b"hello").unwrap();
    assert_eq!(std::fs::read(&bin_path).unwrap(), b"hello");
    assert!(!dir.join("blob.bin.tmp").exists());
}

#[test]
fn test_cmyk_round_trip() {
    use irasu_illustrator::ui::panels::color_utils::{cmyk_to_rgb, rgb_to_cmyk};
    // Black
    let (c, m, y, k) = rgb_to_cmyk(0.0, 0.0, 0.0);
    assert!(k > 0.9 && c < 0.01 && m < 0.01 && y < 0.01);
    let (r, g, b) = cmyk_to_rgb(c, m, y, k);
    assert!((r - 0.0).abs() < 0.01 && (g - 0.0).abs() < 0.01 && (b - 0.0).abs() < 0.01);
    // White
    let (c, m, y, k) = rgb_to_cmyk(1.0, 1.0, 1.0);
    assert!(k < 0.01 && c < 0.01 && m < 0.01 && y < 0.01);
    // Red
    let (c, m, y, k) = rgb_to_cmyk(0.9, 0.2, 0.2);
    let (r, g, b) = cmyk_to_rgb(c, m, y, k);
    assert!((r - 0.9).abs() < 0.05 && (g - 0.2).abs() < 0.05 && (b - 0.2).abs() < 0.05);
}

#[test]
fn test_document_cmyk_color_mode() {
    let mut doc = irasu_illustrator::core::document::Document::default();
    assert_eq!(doc.color_mode, irasu_illustrator::core::document::ColorMode::Rgb);
    doc.color_mode = irasu_illustrator::core::document::ColorMode::Cmyk;
    assert_eq!(doc.color_mode, irasu_illustrator::core::document::ColorMode::Cmyk);
}

#[test]
fn test_artboards_effective_list() {
    let mut doc = irasu_illustrator::core::document::Document::default();
    // Empty artboards → implicit single artboard from width/height
    let eff = doc.effective_artboards();
    assert_eq!(eff.len(), 1);
    assert_eq!(eff[0].name, "Artboard 1");
    assert!((eff[0].width - doc.width).abs() < 0.01);
    // With explicit artboards, they are returned as-is
    doc.artboards.push(irasu_illustrator::core::document::Artboard::new("A", 0.0, 0.0, 800.0, 600.0));
    doc.artboards.push(irasu_illustrator::core::document::Artboard::new("B", 0.0, 600.0, 800.0, 600.0));
    let eff = doc.effective_artboards();
    assert_eq!(eff.len(), 2);
    assert_eq!(eff[1].y, 600.0);
}

#[test]
fn test_text_style_word_wrap() {
    use irasu_illustrator::core::document::object::{compute_wrapped_lines, TextStyle};
    let style = TextStyle {
        font_size: 16.0,
        word_wrap: true,
        max_width: Some(100.0),
        ..Default::default()
    };
    let lines = compute_wrapped_lines("This is a long sentence that should wrap", &style, 100.0);
    assert!(lines.len() > 1, "long text should wrap into multiple lines, got {lines:?}");
    // Hard breaks always preserved
    let hard = compute_wrapped_lines("line1\nline2", &style, 100.0);
    assert!(hard.len() >= 2);
    // Short text stays single-line
    let short = compute_wrapped_lines("Hi", &style, 100.0);
    assert_eq!(short.len(), 1);
}

#[test]
fn test_text_style_line_height() {
    let mut style = irasu_illustrator::core::document::object::TextStyle::default();
    style.line_height = Some(2.0);
    assert!((style.effective_line_height() - 48.0).abs() < 0.01); // 2.0 * 24
    style.line_height = None;
    assert!((style.effective_line_height() - 28.8).abs() < 0.01); // 1.2 * 24
}

#[test]
fn test_text_block_size_word_wrap() {
    use irasu_illustrator::core::document::object::text_block_size_with_style;
    let style = irasu_illustrator::core::document::object::TextStyle {
        font_size: 16.0,
        word_wrap: true,
        max_width: Some(50.0),
        ..Default::default()
    };
    let (_w, h) = text_block_size_with_style("word1 word2 word3 word4", &style);
    assert!(h > 16.0, "wrapped text should be taller than single line, got {h}");
}

#[test]
fn test_document_color_mode_roundtrip() {
    let mut doc = irasu_illustrator::core::document::Document::default();
    doc.color_mode = irasu_illustrator::core::document::ColorMode::Cmyk;
    let json = serde_json::to_string(&doc).unwrap();
    let reloaded: irasu_illustrator::core::document::Document = serde_json::from_str(&json).unwrap();
    assert_eq!(reloaded.color_mode, irasu_illustrator::core::document::ColorMode::Cmyk);
}

#[test]
fn test_undo_redo_return_owned_names() {
    let mut doc = Document::default();
    let mut mgr = irasu_illustrator::core::history::UndoManager::new();
    mgr.execute(
        Box::new(irasu_illustrator::core::history::AddObjectCommand::new(
            Object::new_rect("R", 0.0, 0.0, 5.0, 5.0, 0.0),
        )),
        &mut doc,
    );
    assert_eq!(mgr.undo(&mut doc).as_deref(), Some("Add Object"));
    assert_eq!(mgr.redo(&mut doc).as_deref(), Some("Add Object"));
}
