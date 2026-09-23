#![allow(clippy::field_reassign_with_default)]
use irasu_illustrator::cli::handlers::svg_pipeline::{
    handle_inspect, handle_optimize, handle_render, handle_validate,
};
use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::path::FillType;
use irasu_illustrator::core::state::AppState;
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};
use irasu_illustrator::plugin::script::ScriptEngine;

const SAMPLE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 300" width="400" height="300">
  <defs>
    <linearGradient id="grad1" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#ff0000" />
      <stop offset="100%" stop-color="#0000ff" />
    </linearGradient>
    <radialGradient id="grad2" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0%" stop-color="#ffffff" />
      <stop offset="100%" stop-color="#00ff00" />
    </radialGradient>
    <linearGradient id="unused_grad" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#000000" />
      <stop offset="100%" stop-color="#ffffff" />
    </linearGradient>
  </defs>
  <g id="bg_layer">
    <rect x="0" y="0" width="400" height="300" fill="url(#grad1)" />
  </g>
  <g id="shapes_layer">
    <circle cx="200" cy="150" r="80" fill="url(#grad2)" stroke="#111111" stroke-width="3" stroke-dasharray="8 4" />
    <path d="M 50 50 C 100 20 150 80 200 50 S 300 20 350 50" fill="none" stroke="#222222" stroke-width="4" />
    <text x="50" y="250" font-size="24" fill="#ffffff">Amata SVG First Class</text>
  </g>
  <g id="empty_group">
  </g>
</svg>"##;

#[test]
fn test_svg_render_png_and_scale() {
    let temp_dir = std::env::temp_dir().join("amata_test_render");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let svg_path = temp_dir.join("test.svg");
    let out_png = temp_dir.join("output_4x.png");

    std::fs::write(&svg_path, SAMPLE_SVG).unwrap();

    let res = handle_render(&svg_path, &out_png, Some(4.0), None, None, None);

    assert!(res.is_ok(), "Render 4x should succeed: {:?}", res.err());
    assert!(out_png.exists(), "PNG file should be created");

    let png_bytes = std::fs::read(&out_png).unwrap();
    // PNG magic header: 137 80 78 71 13 10 26 10
    assert!(png_bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]));
}

#[test]
fn test_svg_render_huge_canvas_guard() {
    let temp_dir = std::env::temp_dir().join("amata_test_guard");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let svg_path = temp_dir.join("test.svg");
    let out_png = temp_dir.join("output_huge.png");

    std::fs::write(&svg_path, SAMPLE_SVG).unwrap();

    // Scale 100 on 400x300 canvas = 40000x30000 > 16384 max limit
    let res = handle_render(&svg_path, &out_png, Some(100.0), None, None, None);

    assert!(res.is_err(), "Huge canvas should be rejected");
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("exceed safe limit"),
        "Expected safe limit error, got: {}",
        err_msg
    );
}

#[test]
fn test_svg_inspect_text_and_json() {
    let temp_dir = std::env::temp_dir().join("amata_test_inspect");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let svg_path = temp_dir.join("test.svg");
    std::fs::write(&svg_path, SAMPLE_SVG).unwrap();

    // Plain text inspect
    let res_text = handle_inspect(&svg_path, false);
    assert!(res_text.is_ok());

    // JSON inspect
    let res_json = handle_inspect(&svg_path, true);
    assert!(res_json.is_ok());
}

#[test]
fn test_svg_validate_valid_and_broken_refs() {
    let temp_dir = std::env::temp_dir().join("amata_test_validate");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let valid_svg_path = temp_dir.join("valid.svg");
    std::fs::write(&valid_svg_path, SAMPLE_SVG).unwrap();

    let res_valid = handle_validate(&valid_svg_path, false);
    assert!(res_valid.is_ok(), "Valid SVG should pass validation");

    let broken_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
      <rect x="0" y="0" width="100" height="100" fill="url(#missing_gradient)" clip-path="url(#missing_clip)" />
    </svg>"##;
    let broken_svg_path = temp_dir.join("broken.svg");
    std::fs::write(&broken_svg_path, broken_svg).unwrap();

    let res_broken = handle_validate(&broken_svg_path, true);
    assert!(
        res_broken.is_err(),
        "Broken references should fail validation"
    );
    let err_msg = res_broken.unwrap_err().to_string();
    assert!(
        err_msg.contains("missing_gradient")
            || err_msg.contains("Reference error")
            || err_msg.contains("Validation failed")
    );
}

#[test]
fn test_svg_optimize_safe() {
    let temp_dir = std::env::temp_dir().join("amata_test_optimize");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let in_svg_path = temp_dir.join("unoptimized.svg");
    let out_svg_path = temp_dir.join("optimized.svg");
    std::fs::write(&in_svg_path, SAMPLE_SVG).unwrap();

    let res = handle_optimize(&in_svg_path, &out_svg_path, 2);
    assert!(res.is_ok());

    let optimized_content = std::fs::read_to_string(&out_svg_path).unwrap();
    // Empty group should have been removed
    assert!(!optimized_content.contains(r#"<g id="empty_group">"#));
    // Unused gradient should have been removed
    assert!(!optimized_content.contains("unused_grad"));
    // Used gradients should be preserved
    assert!(optimized_content.contains("grad1"));
    assert!(optimized_content.contains("grad2"));
    // Semantic elements preserved
    assert!(optimized_content.contains("Amata SVG First Class"));
}

#[test]
fn test_svg_roundtrip_fidelity() {
    let doc = parse_svg_document(SAMPLE_SVG);
    assert_eq!(doc.width, 400.0);
    assert_eq!(doc.height, 300.0);

    // Verify objects were imported (flatten recursively to check both top-level and grouped children)
    fn collect_recursive<'a>(objs: &'a [Object], out: &mut Vec<&'a Object>) {
        for o in objs {
            out.push(o);
            if let ObjectType::Group(children) = &o.object_type {
                collect_recursive(children, out);
            }
        }
    }

    let mut all_objects = Vec::new();
    for layer in &doc.layers {
        collect_recursive(&layer.objects, &mut all_objects);
    }
    assert!(
        !all_objects.is_empty(),
        "Document should contain imported objects"
    );

    // Check that we have a rect, circle, path, and text
    let has_rect = all_objects
        .iter()
        .any(|o| matches!(o.object_type, ObjectType::Rectangle { .. }));
    let has_circle = all_objects
        .iter()
        .any(|o| matches!(o.object_type, ObjectType::Ellipse { .. }));
    let has_path = all_objects
        .iter()
        .any(|o| matches!(o.object_type, ObjectType::Path { .. }));
    let has_text = all_objects
        .iter()
        .any(|o| matches!(o.object_type, ObjectType::Text { .. }));

    assert!(has_rect, "Should contain Rectangle object");
    assert!(has_circle, "Should contain Ellipse/Circle object");
    assert!(has_path, "Should contain Path object");
    assert!(has_text, "Should contain Text object");

    // Verify gradient fill import
    let has_gradient_fill = all_objects.iter().any(|o| {
        o.fill
            .as_ref()
            .map(|f| matches!(f.fill_type, FillType::Linear(_) | FillType::Radial(_)))
            .unwrap_or(false)
    });
    assert!(has_gradient_fill, "Should have imported gradient fill");

    // Verify stroke dasharray import
    let has_dash = all_objects.iter().any(|o| {
        o.stroke
            .as_ref()
            .and_then(|s| s.dash_pattern.as_ref())
            .is_some()
    });
    assert!(has_dash, "Should have imported stroke-dasharray");

    // Round-trip back to SVG
    let exported_svg = export_svg(&doc);
    assert!(exported_svg.contains("<svg"));
    assert!(exported_svg.contains("</svg>"));
    assert!(exported_svg.contains("stroke-dasharray="));
    assert!(exported_svg.contains("<linearGradient") || exported_svg.contains("<radialGradient"));
}

#[test]
fn test_rhai_procedural_helpers() {
    let engine = ScriptEngine::new();
    let mut doc = Document::default();
    doc.width = 1000.0;
    doc.height = 1000.0;
    let mut state = AppState::default();

    let script = r#"
        let wave = waveform(10.0, 200.0, 800.0, 100.0, 16, 2.0);
        let g_pts = grid(3, 2, 100.0, 80.0, 10.0, 10.0);
        let r_pts = radial_repeat(500.0, 500.0, 6, 200.0);

        #{
            width: 1000.0,
            height: 1000.0,
            objects: [wave]
        }
    "#;

    let res = engine.run_script(script, &mut doc, &mut state);
    assert!(
        res.is_ok(),
        "Rhai script with helpers should succeed: {:?}",
        res.err()
    );
    let objs: Vec<&Object> = doc.all_objects().map(|(_, o)| o).collect();
    assert_eq!(objs.len(), 1);
    assert!(matches!(objs[0].object_type, ObjectType::Path { .. }));
}

#[test]
fn test_svg_clip_path_import_and_roundtrip() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">
  <defs>
    <clipPath id="cp1">
      <rect x="20" y="20" width="80" height="80" />
    </clipPath>
  </defs>
  <g clip-path="url(#cp1)">
    <rect x="0" y="0" width="200" height="200" fill="#ff0000" />
    <circle cx="50" cy="50" r="20" fill="#00ff00" />
  </g>
  <rect x="100" y="100" width="50" height="50" clip-path="url(#cp1)" fill="#0000ff" />
</svg>"##;

    let doc = parse_svg_document(svg);
    let clipping: Vec<&Object> = doc
        .all_objects()
        .map(|(_, o)| o)
        .filter(|o| matches!(o.object_type, ObjectType::ClippingMask { .. }))
        .collect();
    assert!(
        !clipping.is_empty(),
        "clip-path group and element should import as ClippingMask"
    );
    assert_eq!(
        clipping.len(),
        2,
        "one group clip + one element clip expected"
    );
    for c in &clipping {
        if let ObjectType::ClippingMask { children } = &c.object_type {
            assert!(!children.is_empty(), "mask must have at least a mask shape");
        }
    }
    // Mask shapes must not appear as free top-level objects.
    let top_level = doc.all_objects().count();
    assert_eq!(top_level, 2, "only the two ClippingMask wrappers at top level");

    // Round-trip: export re-emits clipPath + clip-path, re-import keeps masks.
    let out = export_svg(&doc);
    assert!(out.contains("<clipPath"), "export must emit clipPath defs");
    assert!(
        out.contains("clip-path="),
        "export must emit clip-path references"
    );
    let back = parse_svg_document(&out);
    let clipping2 = back
        .all_objects()
        .filter(|(_, o)| matches!(o.object_type, ObjectType::ClippingMask { .. }))
        .count();
    assert_eq!(clipping2, 2, "round-trip must preserve both clipping masks");
}

#[test]
fn test_svg_missing_clip_ref_does_not_crash() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
      <rect x="0" y="0" width="100" height="100" fill="#ccc" clip-path="url(#nope)" />
      <g clip-path="url(#also_missing)">
        <rect x="10" y="10" width="20" height="20" fill="#000" />
      </g>
    </svg>"##;
    let doc = parse_svg_document(svg);
    let objs: Vec<&Object> = doc.all_objects().map(|(_, o)| o).collect();
    assert_eq!(objs.len(), 2, "missing clip refs keep content unclipped");
    assert!(
        objs.iter()
            .all(|o| !matches!(o.object_type, ObjectType::ClippingMask { .. }))
    );
}
