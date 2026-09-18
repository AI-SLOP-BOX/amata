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
