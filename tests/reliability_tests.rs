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
    let mut inner = Object::new_rect("Inner", 0.0, 0.0, 5.0, 5.0, 0.0);
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
        AddLayerCommand, RemoveLayerCommand, ReorderLayersCommand, ReorderObjectsCommand,
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
    let lid = doc.layers[1].id.clone();
    let oo: Vec<String> = doc.layers[1].objects.iter().map(|o| o.id.clone()).collect();
    doc.move_object_up(1, 0);
    let no: Vec<String> = doc.layers[1].objects.iter().map(|o| o.id.clone()).collect();
    assert_ne!(oo, no);
    mgr.execute(
        Box::new(ReorderObjectsCommand {
            layer_id: lid,
            old_order: oo.clone(),
            new_order: no,
        }),
        &mut doc,
    );
    mgr.undo(&mut doc);
    let back: Vec<String> = doc.layers[1].objects.iter().map(|o| o.id.clone()).collect();
    assert_eq!(back, oo);
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
