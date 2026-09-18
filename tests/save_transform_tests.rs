use irasu_illustrator::cli::handlers::common::save_any_document;
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};

#[test]
fn test_rotated_rect_export_keeps_transform() {
    let mut doc = Document::default();
    let mut obj = Object::new_rect("R", 10.0, 20.0, 100.0, 50.0, 0.0);
    obj.transform.rotation = std::f64::consts::FRAC_PI_4;
    obj.transform.scale_x = 2.0;
    doc.add_object(obj);

    let svg = export_svg(&doc);
    assert!(
        svg.contains("transform=\"matrix("),
        "rotated/scaled rect must export a transform, got:\n{svg}"
    );

    let doc2 = parse_svg_document(&svg);
    let imported = doc2
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| matches!(o.object_type, irasu_illustrator::core::document::ObjectType::Rectangle { .. }))
        .expect("rect should round-trip");
    assert!(
        (imported.transform.rotation - std::f64::consts::FRAC_PI_4).abs() < 1e-6,
        "rotation lost: {}",
        imported.transform.rotation
    );
    assert!((imported.transform.scale_x - 2.0).abs() < 1e-6);
    assert!((imported.transform.x - 10.0).abs() < 1e-6);
    assert!((imported.transform.y - 20.0).abs() < 1e-6);
}

#[test]
fn test_translated_rect_stays_readable_without_matrix() {
    let mut doc = Document::default();
    doc.add_object(Object::new_rect("R", 5.0, 7.0, 10.0, 20.0, 0.0));
    let svg = export_svg(&doc);
    assert!(!svg.contains("transform=\"matrix("));
    assert!(svg.contains("x=\"5\""));
}

#[test]
fn test_save_any_document_respects_extension() {
    let mut doc = Document::default();
    doc.name = "ExtCheck".to_string();
    doc.add_object(Object::new_rect("R", 1.0, 2.0, 3.0, 4.0, 0.0));

    let dir = std::env::temp_dir().join("amata_save_ext_check");
    std::fs::create_dir_all(&dir).unwrap();
    let proj_path = dir.join("doc.amatATEST");
    let _ = std::fs::remove_file(&proj_path);
    let proj_path = dir.join("doc.amata");
    let svg_path = dir.join("doc.svg");

    save_any_document(&doc, &proj_path).expect("project save");
    save_any_document(&doc, &svg_path).expect("svg save");

    let proj_content = std::fs::read_to_string(&proj_path).unwrap();
    let svg_content = std::fs::read_to_string(&svg_path).unwrap();
    assert!(
        proj_content.trim_start().starts_with('{'),
        "project file must stay JSON, got: {}",
        &proj_content[..proj_content.len().min(120)]
    );
    assert!(svg_content.contains("<svg"));
}
