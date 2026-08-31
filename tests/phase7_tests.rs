use irasu_illustrator::cli::{run_cli, Cli, CliDeformType, CliFlowFieldPreset, Commands};
use irasu_illustrator::core::brush::scatter_brush_along_path;
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::flowfield::{generate_flowfield_streamlines, FlowFieldPreset};
use irasu_illustrator::core::noise::{deform_path, DeformType};
use irasu_illustrator::core::path::PathData;

#[test]
fn test_noise_deformers() {
    let mut square = PathData::new();
    square.push_move_to(0.0, 0.0);
    square.push_line_to(100.0, 0.0);
    square.push_line_to(100.0, 100.0);
    square.push_line_to(0.0, 100.0);
    square.close();

    // 1. Wave
    let wave = deform_path(&square, DeformType::SineWave, 10.0, 0.1, 0.0);
    assert!(!wave.is_empty());

    // 2. Turbulent Noise
    let noise = deform_path(&square, DeformType::TurbulentNoise, 15.0, 0.05, 1.23);
    assert!(!noise.is_empty());

    // 3. Glitch
    let glitch = deform_path(&square, DeformType::JitterGlitch, 8.0, 0.2, 3.45);
    assert!(!glitch.is_empty());
}

#[test]
fn test_flowfield_generation() {
    // 1. Vortex
    let vortex = generate_flowfield_streamlines(FlowFieldPreset::Vortex, 400.0, 400.0, 20, 50, 4.0);
    assert_eq!(vortex.len(), 20);

    // 2. Magnetic
    let magnetic = generate_flowfield_streamlines(FlowFieldPreset::MagneticDipole, 400.0, 400.0, 15, 50, 4.0);
    assert_eq!(magnetic.len(), 15);

    // 3. Cyber
    let cyber = generate_flowfield_streamlines(FlowFieldPreset::CyberChaos, 400.0, 400.0, 15, 50, 4.0);
    assert_eq!(cyber.len(), 15);
}

#[test]
fn test_scatter_brush_replication() {
    let mut curve = PathData::new();
    curve.push_move_to(0.0, 0.0);
    curve.push_line_to(300.0, 0.0);

    let motif = Object::new_rect("Dot", -5.0, -5.0, 10.0, 10.0, 0.0);
    let scattered = scatter_brush_along_path(&curve, &motif, 30.0, 0.0, true);

    assert!(scattered.len() >= 9);
    for item in &scattered {
        assert!(!item.to_path_data().is_empty());
    }
}

#[test]
fn test_cli_phase7_subcommands() {
    let temp_dir = std::env::temp_dir();
    let in_svg = temp_dir.join("p7_in.svg");
    let in_motif = temp_dir.join("p7_motif.svg");
    let out_deform = temp_dir.join("p7_deform.svg");
    let out_flow = temp_dir.join("p7_flow.svg");
    let out_brush = temp_dir.join("p7_brush.svg");

    let mut doc_in = Document::default();
    doc_in.add_object(Object::new_rect("Box", 50.0, 50.0, 100.0, 100.0, 0.0));
    let svg_str = irasu_illustrator::io::svg::export_svg(&doc_in);
    std::fs::write(&in_svg, &svg_str).unwrap();

    let mut doc_motif = Document::default();
    doc_motif.add_object(Object::new_rect("Dot", 0.0, 0.0, 20.0, 20.0, 0.0));
    let motif_svg = irasu_illustrator::io::svg::export_svg(&doc_motif);
    std::fs::write(&in_motif, &motif_svg).unwrap();

    // 1. Deform CLI
    let cli_def = Cli {
        command: Some(Commands::Deform {
            input: in_svg.clone(),
            output: out_deform.clone(),
            deform_type: CliDeformType::Noise,
            amplitude: 10.0,
            frequency: 0.05,
        }),
    };
    assert!(run_cli(cli_def).is_ok());
    assert!(out_deform.exists());

    // 2. Flowfield CLI
    let cli_flow = Cli {
        command: Some(Commands::Flowfield {
            preset: CliFlowFieldPreset::Vortex,
            lines: 30,
            output: out_flow.clone(),
        }),
    };
    assert!(run_cli(cli_flow).is_ok());
    assert!(out_flow.exists());

    // 3. Brush-Stroke CLI
    let cli_brush = Cli {
        command: Some(Commands::BrushStroke {
            path: in_svg.clone(),
            motif: in_motif.clone(),
            spacing: 25.0,
            output: out_brush.clone(),
        }),
    };
    assert!(run_cli(cli_brush).is_ok());
    assert!(out_brush.exists());

    // Cleanup
    let _ = std::fs::remove_file(in_svg);
    let _ = std::fs::remove_file(in_motif);
    let _ = std::fs::remove_file(out_deform);
    let _ = std::fs::remove_file(out_flow);
    let _ = std::fs::remove_file(out_brush);
}
