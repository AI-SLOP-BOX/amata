use irasu_illustrator::cli::{
    run_cli, Cli, CliBlendMode, CliWarpPreset, CliWaveformType, Commands,
};
use irasu_illustrator::core::audio_curve::{generate_audio_waveform, WaveformType};
use irasu_illustrator::core::blend::{blend_colors, BlendMode};
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::mesh_warp::{apply_lattice_warp, WarpPreset};
use irasu_illustrator::io::pdf::export_pdf;

#[test]
fn test_photoshop_blend_modes() {
    let red = [1.0, 0.0, 0.0, 1.0];
    let blue = [0.0, 0.0, 1.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];

    // 1. Multiply
    let mult = blend_colors(red, white, BlendMode::Multiply);
    assert!((mult[0] - 1.0).abs() < 1e-3);
    assert!((mult[1] - 0.0).abs() < 1e-3);

    // 2. Screen
    let scr = blend_colors(red, blue, BlendMode::Screen);
    assert!((scr[0] - 1.0).abs() < 1e-3);
    assert!((scr[2] - 1.0).abs() < 1e-3);

    // 3. Difference
    let diff = blend_colors(red, red, BlendMode::Difference);
    assert!((diff[0] - 0.0).abs() < 1e-3);
}

#[test]
fn test_audio_waveforms() {
    // 1. Sine
    let sine = generate_audio_waveform(WaveformType::Sine, 4.0, 1, 400.0, 200.0, 100);
    assert!(!sine.is_empty());

    // 2. Harmonics
    let harm = generate_audio_waveform(WaveformType::Harmonics, 3.0, 5, 400.0, 200.0, 100);
    assert!(!harm.is_empty());

    // 3. FM
    let fm = generate_audio_waveform(WaveformType::FM, 2.0, 1, 400.0, 200.0, 100);
    assert!(!fm.is_empty());
}

#[test]
fn test_mesh_lattice_warp() {
    let obj = Object::new_rect("LatticeBox", 50.0, 50.0, 100.0, 100.0, 0.0);

    // 1. Bulge
    let bulge = apply_lattice_warp(&obj, 4, 4, WarpPreset::Bulge, 1.0);
    assert!(!bulge.to_path_data().is_empty());

    // 2. Twist
    let twist = apply_lattice_warp(&obj, 4, 4, WarpPreset::TwistS, 1.0);
    assert!(!twist.to_path_data().is_empty());
}

#[test]
fn test_vector_pdf_export() {
    let mut doc = Document::default();
    doc.add_object(Object::new_rect("PDFBox", 20.0, 20.0, 150.0, 100.0, 0.0));
    doc.add_object(Object::new_ellipse("PDFCircle", 200.0, 200.0, 50.0, 50.0));

    let pdf_bytes = export_pdf(&doc);
    assert!(pdf_bytes.starts_with(b"%PDF-1.4"));
    assert!(pdf_bytes.ends_with(b"%%EOF\n"));
}

#[test]
fn test_cli_phase8_subcommands() {
    let temp_dir = std::env::temp_dir();
    let in_svg1 = temp_dir.join("p8_in1.svg");
    let in_svg2 = temp_dir.join("p8_in2.svg");
    let out_blend = temp_dir.join("p8_blend.svg");
    let out_audio = temp_dir.join("p8_audio.svg");
    let out_warp = temp_dir.join("p8_warp.svg");
    let out_pdf = temp_dir.join("p8_out.pdf");

    let mut doc1 = Document::default();
    doc1.add_object(Object::new_rect("Box1", 50.0, 50.0, 100.0, 100.0, 0.0));
    let svg1 = irasu_illustrator::io::svg::export_svg(&doc1);
    std::fs::write(&in_svg1, &svg1).unwrap();

    let mut doc2 = Document::default();
    doc2.add_object(Object::new_ellipse("Circle2", 100.0, 100.0, 60.0, 60.0));
    let svg2 = irasu_illustrator::io::svg::export_svg(&doc2);
    std::fs::write(&in_svg2, &svg2).unwrap();

    // 1. Blend CLI
    let cli_blend = Cli {
        command: Some(Commands::Blend {
            base: in_svg1.clone(),
            blend: in_svg2.clone(),
            mode: CliBlendMode::Multiply,
            output: out_blend.clone(),
        }),
    };
    assert!(run_cli(cli_blend).is_ok());
    assert!(out_blend.exists());

    // 2. Audio Wave CLI
    let cli_audio = Cli {
        command: Some(Commands::AudioWave {
            wave_type: CliWaveformType::Harmonics,
            freq: 3.0,
            harmonics: 4,
            output: out_audio.clone(),
        }),
    };
    assert!(run_cli(cli_audio).is_ok());
    assert!(out_audio.exists());

    // 3. Warp CLI
    let cli_warp = Cli {
        command: Some(Commands::Warp {
            input: in_svg1.clone(),
            preset: CliWarpPreset::Bulge,
            grid: 4,
            output: out_warp.clone(),
        }),
    };
    assert!(run_cli(cli_warp).is_ok());
    assert!(out_warp.exists());

    // 4. Export PDF CLI
    let cli_pdf = Cli {
        command: Some(Commands::ExportPdf {
            input: in_svg1.clone(),
            output: out_pdf.clone(),
        }),
    };
    assert!(run_cli(cli_pdf).is_ok());
    assert!(out_pdf.exists());

    // Cleanup
    let _ = std::fs::remove_file(in_svg1);
    let _ = std::fs::remove_file(in_svg2);
    let _ = std::fs::remove_file(out_blend);
    let _ = std::fs::remove_file(out_audio);
    let _ = std::fs::remove_file(out_warp);
    let _ = std::fs::remove_file(out_pdf);
}
