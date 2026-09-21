//! Gradient mesh: model math, document integration, export paths.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::gradient_mesh::MeshGradient;

fn sample_mesh() -> MeshGradient {
    MeshGradient::new_rect(
        0.0, 0.0, 100.0, 50.0, 3, 4,
        [
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
            [1.0, 1.0, 0.0, 1.0],
        ],
    )
}

#[test]
fn test_mesh_object_basics() {
    let mut mesh = sample_mesh();
    // Drag a node: geometry follows.
    mesh.node_mut(1, 1).x = 42.0;
    mesh.node_mut(1, 1).color = [0.5, 0.5, 0.5, 1.0];
    let obj = Object::new_mesh("M", 10.0, 20.0, mesh);
    assert!(obj.hit_test(15.0, 25.0), "inside node bbox hits");
    assert!(!obj.hit_test(500.0, 500.0), "outside misses");
    let (mn, mx) = obj.bounding_box().expect("bbox");
    assert!((mx.x - mn.x - 100.0).abs() < 1e-6);
    assert!((mx.y - mn.y - 50.0).abs() < 1e-6);
}

#[test]
fn test_mesh_serialization_round_trip() {
    let mut doc = Document::default();
    let mut mesh = sample_mesh();
    mesh.node_mut(0, 0).color = [0.1, 0.2, 0.3, 0.9];
    doc.add_object(Object::new_mesh("M", 0.0, 0.0, mesh));
    let json = serde_json::to_string(&doc).unwrap();
    let back: Document = serde_json::from_str(&json).unwrap();
    let found = back
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| matches!(o.object_type, ObjectType::GradientMesh(_)))
        .expect("mesh survives JSON");
    if let ObjectType::GradientMesh(m) = &found.object_type {
        assert_eq!((m.rows, m.cols), (3, 4));
        assert_eq!(m.node(0, 0).color, [0.1, 0.2, 0.3, 0.9]);
    }
}

#[test]
fn test_mesh_svg_export_bakes_quads() {
    let mut doc = Document::default();
    doc.width = 200.0;
    doc.height = 100.0;
    doc.add_object(Object::new_mesh("M", 10.0, 10.0, sample_mesh()));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    // 2x3 patches at subdiv 6 = 18*36 flat quads.
    let paths = svg.matches("<path d=\"M").count();
    assert_eq!(paths, 2 * 3 * 36, "one path per quad, got {paths}");
    assert!(svg.contains("fill=\"#"), "solid quad fills");
    // Reimport keeps the artwork visible (as baked paths, documented).
    let back = irasu_illustrator::io::svg::parse_svg_document(&svg);
    assert!(back.all_objects().count() >= 2 * 3 * 36);
}

#[test]
fn test_mesh_pdf_exports_parse() {
    let mut doc = Document::default();
    doc.width = 200.0;
    doc.height = 100.0;
    doc.add_object(Object::new_mesh("M", 10.0, 10.0, sample_mesh()));
    let pdf = irasu_illustrator::io::pdf::export_pdf(&doc);
    assert!(pdf.starts_with(b"%PDF"));
    let imported = lopdf::Document::load_mem(&pdf).expect("mesh PDF parses");
    assert_eq!(imported.get_pages().len(), 1);
}
