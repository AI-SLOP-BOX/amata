use irasu_illustrator::cli::{run_cli, Cli, CliCurveType, Commands};
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::formula::FormulaCurves;
use irasu_illustrator::core::path::PathData;
use irasu_illustrator::core::text_path::place_text_along_path;
use irasu_illustrator::core::vfx_particles::generate_particle_trail;

#[test]
fn test_text_on_path_placements() {
    let mut path = PathData::new();
    path.push_move_to(0.0, 100.0);
    path.push_line_to(200.0, 100.0);

    let text = "VECTOR";
    let placements = place_text_along_path(&path, text, 20.0, 0.0);

    assert_eq!(placements.len(), text.len());
    for p in &placements {
        assert!((p.position.y - 100.0).abs() < 1.0);
        assert!((p.rotation_rad - 0.0).abs() < 1e-4);
    }
}

#[test]
fn test_formula_curves_generation() {
    // 1. Spiral
    let spiral = FormulaCurves::spiral(200.0, 200.0, 3.0, 5.0, 2.0, 100);
    assert!(!spiral.is_empty());
    assert!(!spiral.closed);

    // 2. Lissajous
    let liss = FormulaCurves::lissajous(200.0, 200.0, 3.0, 2.0, 0.5, 100.0, 80.0, 120);
    assert!(!liss.is_empty());
    assert!(liss.closed);

    // 3. Spirograph
    let spiro = FormulaCurves::spirograph(200.0, 200.0, 80.0, 30.0, 50.0, 6, 32);
    assert!(!spiro.is_empty());
    assert!(spiro.closed);

    // 4. Rose
    let rose = FormulaCurves::rose_curve(200.0, 200.0, 4.0, 80.0, 100);
    assert!(!rose.is_empty());
    assert!(rose.closed);
}

#[test]
fn test_vfx_particle_trail_generation() {
    let mut path = PathData::new();
    path.push_move_to(0.0, 0.0);
    path.push_line_to(100.0, 100.0);

    let particles = generate_particle_trail(&path, 50, 40.0, 5.0);
    assert_eq!(particles.len(), 50);
    for p in &particles {
        assert!(p.lifetime > 0.0);
        assert!(p.size > 0.0);
    }
}

#[test]
fn test_cli_phase4_subcommands() {
    let temp_dir = std::env::temp_dir();
    let out_spiral = temp_dir.join("p4_spiral.svg");
    let out_liss = temp_dir.join("p4_liss.svg");
    let in_svg = temp_dir.join("p4_input.svg");
    let out_particles = temp_dir.join("p4_particles.json");

    // 1. Test CLI Formula (Spiral)
    let cli_spiral = Cli {
        command: Some(Commands::Formula {
            curve_type: CliCurveType::Spiral,
            output: out_spiral.clone(),
        }),
    };
    assert!(run_cli(cli_spiral).is_ok());
    assert!(out_spiral.exists());

    // 2. Test CLI Formula (Lissajous)
    let cli_liss = Cli {
        command: Some(Commands::Formula {
            curve_type: CliCurveType::Lissajous,
            output: out_liss.clone(),
        }),
    };
    assert!(run_cli(cli_liss).is_ok());
    assert!(out_liss.exists());

    // 3. Test CLI VFX Particle Trail
    let mut doc = Document::default();
    doc.add_object(Object::new_rect("Box", 0.0, 0.0, 100.0, 100.0, 0.0));
    let svg_str = irasu_illustrator::io::svg::export_svg(&doc);
    std::fs::write(&in_svg, svg_str).unwrap();

    let cli_trail = Cli {
        command: Some(Commands::VfxTrail {
            input: in_svg.clone(),
            output: out_particles.clone(),
            count: 100,
        }),
    };
    assert!(run_cli(cli_trail).is_ok());
    assert!(out_particles.exists());

    // Cleanup
    let _ = std::fs::remove_file(out_spiral);
    let _ = std::fs::remove_file(out_liss);
    let _ = std::fs::remove_file(in_svg);
    let _ = std::fs::remove_file(out_particles);
}
