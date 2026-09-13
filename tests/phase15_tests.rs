use irasu_illustrator::core::document::Object;
use irasu_illustrator::core::path::AnchorPoint;

#[test]
fn test_compound_path_creation_and_holes() {
    // Outer rectangle: 0,0 to 200,200
    let outer = Object::new_rect("Outer", 0.0, 0.0, 200.0, 200.0, 0.0);
    // Inner rectangle (hole): 50,50 to 150,150
    let inner = Object::new_rect("Inner", 50.0, 50.0, 100.0, 100.0, 0.0);

    // Make compound path
    let compound =
        Object::make_compound_path(&[outer, inner]).expect("Should create compound path");
    assert_eq!(compound.name, "Compound Path");

    // Hit test:
    // (20, 20) is inside outer, outside inner -> hit should be true
    assert!(
        compound.hit_test(20.0, 20.0),
        "Point outside hole should be hit"
    );

    // (100, 100) is inside inner hole -> hit should be false (hole is hollow!)
    assert!(
        !compound.hit_test(100.0, 100.0),
        "Center of hole should NOT be hit (hollow)"
    );

    // (250, 250) is outside everything -> hit should be false
    assert!(
        !compound.hit_test(250.0, 250.0),
        "Point outside everything should NOT be hit"
    );
}

#[test]
fn test_compound_path_triangulation_and_subpaths() {
    let outer = Object::new_rect("Outer", 0.0, 0.0, 100.0, 100.0, 0.0);
    let inner = Object::new_rect("Inner", 25.0, 25.0, 50.0, 50.0, 0.0);

    let compound = Object::make_compound_path(&[outer, inner]).unwrap();
    let path = compound.to_path_data();

    // Verify subpaths extraction
    let subpaths = path.to_subpaths(4);
    assert_eq!(
        subpaths.len(),
        2,
        "Should contain 2 separate subpaths (outer + hole)"
    );

    // Verify triangulation
    let triangles = path.to_triangles(4);
    assert!(
        !triangles.is_empty(),
        "Should generate triangles for rendering"
    );

    // Check that triangles do not cover the center hole (50, 50)
    for [a, b, c] in &triangles {
        let inside = is_point_in_tri(AnchorPoint::new(50.0, 50.0), *a, *b, *c);
        assert!(!inside, "Triangles should not fill the inner hole");
    }
}

#[test]
fn test_compound_path_release() {
    let outer = Object::new_rect("Outer", 0.0, 0.0, 100.0, 100.0, 0.0);
    let inner = Object::new_rect("Inner", 25.0, 25.0, 50.0, 50.0, 0.0);

    let compound = Object::make_compound_path(&[outer, inner]).unwrap();
    let released = compound.release_compound_path();

    assert_eq!(
        released.len(),
        2,
        "Release should return 2 independent objects"
    );
    assert!(released[0].name.contains("part_1"));
    assert!(released[1].name.contains("part_2"));
}

#[test]
fn test_cli_compound_path_subcommand() {
    let temp_dir = std::env::temp_dir();
    let in_svg = temp_dir.join("test_phase15_in.svg");
    let out_svg = temp_dir.join("test_phase15_out.svg");

    let mut doc = irasu_illustrator::core::document::Document::default();
    doc.add_object(Object::new_rect("Outer", 0.0, 0.0, 200.0, 200.0, 0.0));
    doc.add_object(Object::new_rect("Inner", 50.0, 50.0, 100.0, 100.0, 0.0));
    let svg_content = irasu_illustrator::io::svg::export_svg(&doc);
    std::fs::write(&in_svg, svg_content).unwrap();

    // Run CLI Compound
    let cli = irasu_illustrator::cli::Cli {
        command: Some(irasu_illustrator::cli::Commands::Compound {
            input: in_svg.clone(),
            output: out_svg.clone(),
            release: false,
        }),
    };
    let res = irasu_illustrator::cli::run_cli(cli);
    assert!(res.is_ok(), "CLI compound execution should succeed");
    assert!(out_svg.exists(), "Output SVG should be generated");

    // Clean up
    let _ = std::fs::remove_file(in_svg);
    let _ = std::fs::remove_file(out_svg);
}

fn is_point_in_tri(p: AnchorPoint, a: AnchorPoint, b: AnchorPoint, c: AnchorPoint) -> bool {
    let c1 = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let c2 = (c.x - b.x) * (p.y - b.y) - (c.y - b.y) * (p.x - b.x);
    let c3 = (a.x - c.x) * (p.y - c.y) - (a.y - c.y) * (p.x - c.x);

    let has_neg = (c1 < -1e-5) || (c2 < -1e-5) || (c3 < -1e-5);
    let has_pos = (c1 > 1e-5) || (c2 > 1e-5) || (c3 > 1e-5);

    !(has_neg && has_pos)
}
