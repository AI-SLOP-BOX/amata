use irasu_illustrator::cli::{run_cli, Cli, CliAxonometricMode, CliGradientMeshPreset, Commands};
use irasu_illustrator::core::axonometric::{apply_axonometric_projection, AxonometricMode};
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::evolutionary::evolve_vector_composition;
use irasu_illustrator::core::gradient_mesh::{generate_gradient_mesh, GradientMeshPreset};
use irasu_illustrator::core::neon_glow::generate_neon_glow;

#[test]
fn test_gradient_mesh_generation() {
    let patches = generate_gradient_mesh(GradientMeshPreset::Sunset, 500.0, 400.0, 3, 3);
    assert!(!patches.is_empty());
    for p in &patches {
        assert!(!p.to_path_data().is_empty());
        assert!(p.fill.is_some());
    }
}

#[test]
fn test_axonometric_projections() {
    let obj = Object::new_rect("CubeFace", 10.0, 10.0, 50.0, 50.0, 0.0);

    // 1. Dimetric
    let dim = apply_axonometric_projection(&obj, AxonometricMode::Dimetric);
    assert!(!dim.to_path_data().is_empty());

    // 2. Cabinet
    let cab = apply_axonometric_projection(&obj, AxonometricMode::Cabinet);
    assert!(!cab.to_path_data().is_empty());

    // 3. Cavalier
    let cav = apply_axonometric_projection(&obj, AxonometricMode::Cavalier);
    assert!(!cav.to_path_data().is_empty());
}

#[test]
fn test_evolutionary_art() {
    let evolved = evolve_vector_composition(400.0, 400.0, 15, 20);
    assert_eq!(evolved.len(), 15);
    for poly in &evolved {
        assert!(!poly.to_path_data().is_empty());
        assert!(poly.fill.is_some());
    }
}

#[test]
fn test_neon_glow_generation() {
    let obj = Object::new_ellipse("NeonBulb", 100.0, 100.0, 40.0, 40.0);
    let path = obj.to_path_data();

    let neon_layers = generate_neon_glow(&path, [0.0, 1.0, 0.9, 1.0], 20.0, 5);
    assert_eq!(neon_layers.len(), 6); // 5 halos + 1 white core
}

#[test]
fn test_cli_phase9_subcommands() {
    let temp_dir = std::env::temp_dir();
    let in_svg = temp_dir.join("p9_in.svg");
    let out_mesh = temp_dir.join("p9_mesh.svg");
    let out_axon = temp_dir.join("p9_axon.svg");
    let out_evolve = temp_dir.join("p9_evolve.svg");
    let out_neon = temp_dir.join("p9_neon.svg");

    let mut doc = Document::default();
    doc.add_object(Object::new_rect("Box", 30.0, 30.0, 80.0, 80.0, 0.0));
    let svg_str = irasu_illustrator::io::svg::export_svg(&doc);
    std::fs::write(&in_svg, &svg_str).unwrap();

    // 1. Gradient Mesh CLI
    let cli_mesh = Cli {
        command: Some(Commands::GradientMesh {
            preset: CliGradientMeshPreset::Cyberpunk,
            rows: 2,
            cols: 2,
            output: out_mesh.clone(),
        }),
    };
    assert!(run_cli(cli_mesh).is_ok());
    assert!(out_mesh.exists());

    // 2. Axonometric CLI
    let cli_axon = Cli {
        command: Some(Commands::Axonometric {
            input: in_svg.clone(),
            mode: CliAxonometricMode::Cabinet,
            output: out_axon.clone(),
        }),
    };
    assert!(run_cli(cli_axon).is_ok());
    assert!(out_axon.exists());

    // 3. Evolve CLI
    let cli_evolve = Cli {
        command: Some(Commands::Evolve {
            polygons: 10,
            generations: 10,
            output: out_evolve.clone(),
        }),
    };
    assert!(run_cli(cli_evolve).is_ok());
    assert!(out_evolve.exists());

    // 4. Neon CLI
    let cli_neon = Cli {
        command: Some(Commands::Neon {
            input: in_svg.clone(),
            radius: 15.0,
            layers: 4,
            output: out_neon.clone(),
        }),
    };
    assert!(run_cli(cli_neon).is_ok());
    assert!(out_neon.exists());

    // Cleanup
    let _ = std::fs::remove_file(in_svg);
    let _ = std::fs::remove_file(out_mesh);
    let _ = std::fs::remove_file(out_axon);
    let _ = std::fs::remove_file(out_evolve);
    let _ = std::fs::remove_file(out_neon);
}
