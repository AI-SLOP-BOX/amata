use irasu_illustrator::core::diff::{compute_semantic_diff, ObjectDiffStatus};
use irasu_illustrator::core::document::{Document, Object, Symbol};
use irasu_illustrator::core::path::{FillStyle, FillType, GradientStop, LinearGradient, PathData};
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};

fn create_test_doc(name: &str, width: f64, height: f64) -> Document {
    let mut doc = Document::default();
    doc.name = name.to_string();
    doc.width = width;
    doc.height = height;
    doc
}

#[test]
fn test_semantic_diff_text_only() {
    let mut doc_a = create_test_doc("A", 800.0, 600.0);
    let mut doc_b = create_test_doc("B", 800.0, 600.0);

    let mut t_a = Object::new_text("Title", "HIRARI SOUND", 100.0, 100.0, 48.0);
    t_a.id = "headline".to_string();

    let mut t_b = Object::new_text("Title", "HIRARI SOUND 2026", 100.0, 100.0, 56.0);
    t_b.id = "headline".to_string();

    doc_a.add_object(t_a);
    doc_b.add_object(t_b);

    let diff = compute_semantic_diff(&doc_a, &doc_b);
    assert_eq!(diff.summary.modified_count, 1);
    assert_eq!(diff.summary.added_count, 0);
    assert_eq!(diff.summary.removed_count, 0);

    let obj_diff = &diff.objects[0];
    assert_eq!(obj_diff.id, "headline");
    if let ObjectDiffStatus::Modified { changes } = &obj_diff.status {
        let text_change = changes.iter().find(|c| c.field == "text");
        assert!(text_change.is_some());
        assert_eq!(text_change.unwrap().new_value, "\"HIRARI SOUND 2026\"");

        let font_size_change = changes.iter().find(|c| c.field == "font-size");
        assert!(font_size_change.is_some());
    } else {
        panic!("Expected Modified status");
    }
}

#[test]
fn test_semantic_diff_fill_and_transform() {
    let mut doc_a = create_test_doc("A", 800.0, 600.0);
    let mut doc_b = create_test_doc("B", 800.0, 600.0);

    let mut rect_a = Object::new_rect("Box", 50.0, 50.0, 200.0, 100.0, 0.0);
    rect_a.id = "box-1".to_string();
    rect_a.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));

    let mut rect_b = Object::new_rect("Box", 80.0, 50.0, 200.0, 100.0, 0.0);
    rect_b.id = "box-1".to_string();
    rect_b.fill = Some(FillStyle::solid([0.0, 0.0, 1.0, 1.0]));

    doc_a.add_object(rect_a);
    doc_b.add_object(rect_b);

    let diff = compute_semantic_diff(&doc_a, &doc_b);
    assert_eq!(diff.summary.modified_count, 1);

    if let ObjectDiffStatus::Modified { changes } = &diff.objects[0].status {
        assert!(changes.iter().any(|c| c.field == "position"));
        assert!(changes.iter().any(|c| c.field == "fill"));
    } else {
        panic!("Expected Modified");
    }
}

#[test]
fn test_semantic_diff_add_and_remove() {
    let mut doc_a = create_test_doc("A", 800.0, 600.0);
    let mut doc_b = create_test_doc("B", 800.0, 600.0);

    let mut logo = Object::new_rect("Logo", 10.0, 10.0, 50.0, 50.0, 0.0);
    logo.id = "old-logo".to_string();
    doc_a.add_object(logo);

    let mut artist1 = Object::new_text("Artist 1", "Artist 09", 100.0, 200.0, 18.0);
    artist1.id = "artist-09".to_string();
    let mut artist2 = Object::new_text("Artist 2", "Artist 10", 100.0, 230.0, 18.0);
    artist2.id = "artist-10".to_string();
    doc_b.add_object(artist1);
    doc_b.add_object(artist2);

    let diff = compute_semantic_diff(&doc_a, &doc_b);
    assert_eq!(diff.summary.removed_count, 1);
    assert_eq!(diff.summary.added_count, 2);
}

#[test]
fn test_semantic_diff_path_geometry() {
    let mut doc_a = create_test_doc("A", 800.0, 600.0);
    let mut doc_b = create_test_doc("B", 800.0, 600.0);

    let mut p_a = PathData::new();
    p_a.push_move_to(0.0, 0.0);
    p_a.push_line_to(100.0, 0.0);
    let mut wave_a = Object::new_path("Wave", p_a);
    wave_a.id = "main-wave".to_string();

    let mut p_b = PathData::new();
    p_b.push_move_to(0.0, 0.0);
    p_b.push_line_to(50.0, 50.0);
    p_b.push_line_to(100.0, 0.0);
    let mut wave_b = Object::new_path("Wave", p_b);
    wave_b.id = "main-wave".to_string();

    doc_a.add_object(wave_a);
    doc_b.add_object(wave_b);

    let diff = compute_semantic_diff(&doc_a, &doc_b);
    assert_eq!(diff.summary.modified_count, 1);
    if let ObjectDiffStatus::Modified { changes } = &diff.objects[0].status {
        assert!(changes.iter().any(|c| c.field == "node-count"));
    } else {
        panic!("Expected Modified");
    }
}

#[test]
fn test_semantic_diff_reorder_z_order() {
    let mut doc_a = create_test_doc("A", 800.0, 600.0);
    let mut doc_b = create_test_doc("B", 800.0, 600.0);

    let mut obj1 = Object::new_rect("Obj1", 10.0, 10.0, 50.0, 50.0, 0.0);
    obj1.id = "id-1".to_string();
    let mut obj2 = Object::new_rect("Obj2", 20.0, 20.0, 50.0, 50.0, 0.0);
    obj2.id = "id-2".to_string();

    doc_a.add_object(obj1.clone());
    doc_a.add_object(obj2.clone());

    // In doc_b, reversed order
    doc_b.add_object(obj2);
    doc_b.add_object(obj1);

    let diff = compute_semantic_diff(&doc_a, &doc_b);
    assert_eq!(diff.summary.modified_count, 2);
    for obj in &diff.objects {
        if let ObjectDiffStatus::Modified { changes } = &obj.status {
            assert!(changes.iter().any(|c| c.field == "z-order"));
        }
    }
}

#[test]
fn test_semantic_diff_gradient_stop_change() {
    let mut doc_a = create_test_doc("A", 800.0, 600.0);
    let mut doc_b = create_test_doc("B", 800.0, 600.0);

    let mut grad_a = LinearGradient::default();
    grad_a.stops = vec![
        GradientStop {
            offset: 0.0,
            color: [1.0, 0.0, 0.0, 1.0],
        },
        GradientStop {
            offset: 1.0,
            color: [0.0, 0.0, 1.0, 1.0],
        },
    ];

    let mut grad_b = LinearGradient::default();
    grad_b.stops = vec![
        GradientStop {
            offset: 0.0,
            color: [1.0, 0.0, 0.0, 1.0],
        },
        GradientStop {
            offset: 0.5,
            color: [0.0, 1.0, 0.0, 1.0],
        },
        GradientStop {
            offset: 1.0,
            color: [0.0, 0.0, 1.0, 1.0],
        },
    ];

    let mut bg_a = Object::new_rect("BG", 0.0, 0.0, 800.0, 600.0, 0.0);
    bg_a.id = "bg".to_string();
    bg_a.fill = Some(FillStyle {
        color: [0.0, 0.0, 0.0, 1.0],
        fill_type: FillType::Linear(grad_a),
        rule: irasu_illustrator::core::path::FillRule::NonZero,
    });

    let mut bg_b = Object::new_rect("BG", 0.0, 0.0, 800.0, 600.0, 0.0);
    bg_b.id = "bg".to_string();
    bg_b.fill = Some(FillStyle {
        color: [0.0, 0.0, 0.0, 1.0],
        fill_type: FillType::Linear(grad_b),
        rule: irasu_illustrator::core::path::FillRule::NonZero,
    });

    doc_a.add_object(bg_a);
    doc_b.add_object(bg_b);

    let diff = compute_semantic_diff(&doc_a, &doc_b);
    assert_eq!(diff.summary.modified_count, 1);
    if let ObjectDiffStatus::Modified { changes } = &diff.objects[0].status {
        assert!(changes.iter().any(|c| c.field == "fill"));
    } else {
        panic!("Expected Modified for gradient");
    }
}

#[test]
fn test_semantic_diff_symbol_and_use_change() {
    let mut doc_a = create_test_doc("A", 800.0, 600.0);
    let mut doc_b = create_test_doc("B", 800.0, 600.0);

    let mut master = Object::new_rect("BtnMaster", 0.0, 0.0, 100.0, 40.0, 4.0);
    master.id = "btn-symbol".to_string();
    doc_a.add_symbol(Symbol {
        id: "btn-symbol".to_string(),
        name: "Button".to_string(),
        object: master.clone(),
        use_count: 1,
    });
    doc_b.add_symbol(Symbol {
        id: "btn-symbol".to_string(),
        name: "Button".to_string(),
        object: master,
        use_count: 1,
    });

    let mut u_a = Object::new_use("btn-1", "btn-symbol", 50.0, 50.0, Some(100.0), Some(40.0));
    u_a.id = "btn-inst-1".to_string();

    let mut u_b = Object::new_use("btn-1", "btn-symbol", 120.0, 50.0, Some(100.0), Some(40.0));
    u_b.id = "btn-inst-1".to_string();

    doc_a.add_object(u_a);
    doc_b.add_object(u_b);

    let diff = compute_semantic_diff(&doc_a, &doc_b);
    assert_eq!(diff.summary.modified_count, 1);
    if let ObjectDiffStatus::Modified { changes } = &diff.objects[0].status {
        assert!(changes.iter().any(|c| c.field == "position"));
    } else {
        panic!("Expected Modified for use instance");
    }
}

#[test]
fn test_round_trip_zero_semantic_diff_and_deterministic_output() {
    let svg_original = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 1000">
  <defs>
    <linearGradient id="g1" x1="0.00" y1="0.00" x2="1.00" y2="1.00">
      <stop offset="0.0%" stop-color="#ff0000" />
      <stop offset="100.0%" stop-color="#0000ff" />
    </linearGradient>
  </defs>
  <rect id="bg" x="0" y="0" width="1000" height="1000" fill="url(#g1)" />
  <text id="headline" x="100" y="200" font-size="64" font-family="system-ui, -apple-system, sans-serif" fill="#ffffff">HIRARI SOUND 2026</text>
</svg>
"##;

    let doc1 = parse_svg_document(svg_original);
    let exported1 = export_svg(&doc1);

    let doc2 = parse_svg_document(&exported1);
    let exported2 = export_svg(&doc2);

    // 1. Semantic diff between doc1 and doc2 must be completely empty
    let diff = compute_semantic_diff(&doc1, &doc2);
    assert!(diff.is_empty(), "Round-trip semantic diff must be empty");

    // 2. Deterministic output: saving twice produces zero text difference
    assert_eq!(
        exported1, exported2,
        "Exporting twice must be byte-for-byte identical"
    );
}
