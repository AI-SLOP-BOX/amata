use irasu_illustrator::cli::{run_cli, Cli, Commands};
use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::path::PathData;
use irasu_illustrator::core::text_path::{
    create_text_outlines, get_glyph_outline_path, text_on_path_to_outlines, text_to_outline_path,
};

#[test]
fn test_get_glyph_outline_path() {
    let glyph_a = get_glyph_outline_path('A');
    assert!(!glyph_a.is_empty());
    assert!(glyph_a.closed);

    let glyph_b = get_glyph_outline_path('B');
    assert!(!glyph_b.is_empty());

    let glyph_o = get_glyph_outline_path('O');
    assert!(!glyph_o.is_empty());

    let glyph_unknown = get_glyph_outline_path('?');
    assert!(!glyph_unknown.is_empty());
}

#[test]
fn test_text_to_outline_path() {
    let text = "VECTOR STUDIO";
    let outlined = text_to_outline_path(text, 24.0);
    assert!(!outlined.is_empty());

    // Outlined path should have valid bounding box
    let (min_pt, max_pt) = outlined.bounding_box().expect("Should have bounds");
    assert!(max_pt.x > min_pt.x);
    assert!(max_pt.y > min_pt.y || (max_pt.y - min_pt.y).abs() > 10.0);
}

#[test]
fn test_create_text_outlines_object() {
    let text_obj = Object::new_text("Headline", "IRASU", 100.0, 100.0, 36.0);
    let outlined = create_text_outlines(&text_obj);
    assert!(outlined.is_some());

    let out = outlined.unwrap();
    assert_eq!(out.name, "Headline_Outlines");
    if let ObjectType::Path(ref path) = out.object_type {
        assert!(!path.is_empty());
    } else {
        panic!("Expected ObjectType::Path for outlined text");
    }
}

#[test]
fn test_text_on_path_to_outlines() {
    let mut curve = PathData::new();
    curve.push_move_to(0.0, 100.0);
    curve.push_cubic_curve_to(100.0, 0.0, 200.0, 200.0, 300.0, 100.0);

    let text_outlines = text_on_path_to_outlines(&curve, "DESIGN", 20.0, 10.0);
    assert!(!text_outlines.is_empty());
    let bounds = text_outlines.bounding_box();
    assert!(bounds.is_some());
}

#[test]
fn test_cli_phase11_outline_and_text_path() {
    let temp_dir = std::env::temp_dir();
    let text_svg = temp_dir.join("p11_text.svg");
    let curve_svg = temp_dir.join("p11_curve.svg");
    let out_outline_svg = temp_dir.join("p11_outlined.svg");
    let out_text_path_svg = temp_dir.join("p11_text_path.svg");

    // 1. Text SVG document
    let mut doc_text = Document::default();
    doc_text.add_object(Object::new_text(
        "TestHeading",
        "ILLUSTRATOR",
        50.0,
        100.0,
        32.0,
    ));
    let text_content = irasu_illustrator::io::svg::export_svg(&doc_text);
    std::fs::write(&text_svg, &text_content).unwrap();

    // 2. Trajectory Curve SVG document
    let mut doc_curve = Document::default();
    let mut path = PathData::new();
    path.push_move_to(0.0, 150.0);
    path.push_cubic_curve_to(100.0, 50.0, 200.0, 250.0, 350.0, 150.0);
    doc_curve.add_object(Object::new_path("CurveTraj", path));
    let curve_content = irasu_illustrator::io::svg::export_svg(&doc_curve);
    std::fs::write(&curve_svg, &curve_content).unwrap();

    // Test CLI: Outline
    let cli_outline = Cli {
        command: Some(Commands::Outline {
            input: text_svg.clone(),
            output: out_outline_svg.clone(),
        }),
    };
    assert!(run_cli(cli_outline).is_ok());
    assert!(out_outline_svg.exists());
    let outline_res = std::fs::read_to_string(&out_outline_svg).unwrap();
    assert!(outline_res.contains("<path") && outline_res.contains("d=\""));

    // Test CLI: TextPath
    let cli_text_path = Cli {
        command: Some(Commands::TextPath {
            path: curve_svg.clone(),
            text: "CREATIVE FLOW".to_string(),
            font_size: 24.0,
            offset: 15.0,
            output: out_text_path_svg.clone(),
        }),
    };
    assert!(run_cli(cli_text_path).is_ok());
    assert!(out_text_path_svg.exists());
    let text_path_res = std::fs::read_to_string(&out_text_path_svg).unwrap();
    assert!(text_path_res.contains("<path") && text_path_res.contains("d=\""));

    // Cleanup
    let _ = std::fs::remove_file(text_svg);
    let _ = std::fs::remove_file(curve_svg);
    let _ = std::fs::remove_file(out_outline_svg);
    let _ = std::fs::remove_file(out_text_path_svg);
}
