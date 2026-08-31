use irasu_illustrator::cli::{run_cli, Cli, CliHalftonePattern, CliIsoPlane, Commands};
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::halftone::{generate_halftone_from_path, HalftonePattern};
use irasu_illustrator::core::isometric::{apply_isometric_transform, IsometricPlane};
use irasu_illustrator::core::path::{AnchorPoint, PathData};
use irasu_illustrator::core::simplify::simplify_path_visvalingam;
use irasu_illustrator::core::symmetry::create_radial_symmetry;

#[test]
fn test_halftone_generation() {
    let mut sq = PathData::new();
    sq.push_move_to(0.0, 0.0);
    sq.push_line_to(100.0, 0.0);
    sq.push_line_to(100.0, 100.0);
    sq.push_line_to(0.0, 100.0);
    sq.close();

    let ht_circ = generate_halftone_from_path(&sq, 10.0, 4.0, HalftonePattern::CircularGrid);
    assert!(!ht_circ.is_empty());

    let ht_hex = generate_halftone_from_path(&sq, 10.0, 4.0, HalftonePattern::HexagonalGrid);
    assert!(!ht_hex.is_empty());
}

#[test]
fn test_visvalingam_simplification() {
    // 100-point line with slight jitter
    let mut noisy_pts = Vec::new();
    for i in 0..100 {
        let x = i as f64 * 2.0;
        let y = 50.0 + if i % 2 == 0 { 0.5 } else { -0.5 };
        noisy_pts.push(AnchorPoint::new(x, y));
    }
    let noisy_path = PathData::from_polygon_points(&noisy_pts, false);
    assert_eq!(noisy_path.elements.len(), 100);

    let simplified = simplify_path_visvalingam(&noisy_path, 5.0);
    // Should reduce significantly
    assert!(simplified.elements.len() < 20);
}

#[test]
fn test_radial_symmetry() {
    let obj = Object::new_rect("Petal", 50.0, 50.0, 30.0, 10.0, 0.0);
    let clones = create_radial_symmetry(&obj, 100.0, 100.0, 6, false);
    assert_eq!(clones.len(), 6);

    let clones_mirror = create_radial_symmetry(&obj, 100.0, 100.0, 4, true);
    assert_eq!(clones_mirror.len(), 8);
}

#[test]
fn test_isometric_projection() {
    let rect = Object::new_rect("Top Box", 0.0, 0.0, 100.0, 100.0, 0.0);
    let top_iso = apply_isometric_transform(&rect, IsometricPlane::Top);
    let left_iso = apply_isometric_transform(&rect, IsometricPlane::Left);
    let right_iso = apply_isometric_transform(&rect, IsometricPlane::Right);

    assert!(!top_iso.to_path_data().is_empty());
    assert!(!left_iso.to_path_data().is_empty());
    assert!(!right_iso.to_path_data().is_empty());
}

#[test]
fn test_cli_phase5_subcommands() {
    let temp_dir = std::env::temp_dir();
    let in_svg = temp_dir.join("p5_in.svg");
    let out_ht = temp_dir.join("p5_ht.svg");
    let out_simp = temp_dir.join("p5_simp.svg");
    let out_iso = temp_dir.join("p5_iso.svg");

    let mut doc = Document::default();
    doc.add_object(Object::new_rect("Box", 0.0, 0.0, 80.0, 80.0, 0.0));
    let svg_str = irasu_illustrator::io::svg::export_svg(&doc);
    std::fs::write(&in_svg, svg_str).unwrap();

    // 1. Halftone CLI
    let cli_ht = Cli {
        command: Some(Commands::Halftone {
            input: in_svg.clone(),
            output: out_ht.clone(),
            spacing: 12.0,
            radius: 5.0,
            pattern: CliHalftonePattern::Circular,
        }),
    };
    assert!(run_cli(cli_ht).is_ok());
    assert!(out_ht.exists());

    // 2. Simplify CLI
    let cli_simp = Cli {
        command: Some(Commands::Simplify {
            input: in_svg.clone(),
            output: out_simp.clone(),
            tolerance: 3.0,
        }),
    };
    assert!(run_cli(cli_simp).is_ok());
    assert!(out_simp.exists());

    // 3. Isometric CLI
    let cli_iso = Cli {
        command: Some(Commands::Isometric {
            input: in_svg.clone(),
            output: out_iso.clone(),
            plane: CliIsoPlane::Top,
        }),
    };
    assert!(run_cli(cli_iso).is_ok());
    assert!(out_iso.exists());

    // Cleanup
    let _ = std::fs::remove_file(in_svg);
    let _ = std::fs::remove_file(out_ht);
    let _ = std::fs::remove_file(out_simp);
    let _ = std::fs::remove_file(out_iso);
}
