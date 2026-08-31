use irasu_illustrator::cli::{run_cli, Cli, CliLSystemPreset, Commands};
use irasu_illustrator::core::barcode::generate_vector_qr;
use irasu_illustrator::core::lsystem::{generate_lsystem, LSystemPreset};
use irasu_illustrator::core::path::AnchorPoint;
use irasu_illustrator::core::voronoi::generate_voronoi_cells;

#[test]
fn test_voronoi_generation() {
    let seeds = vec![
        AnchorPoint::new(100.0, 100.0),
        AnchorPoint::new(300.0, 100.0),
        AnchorPoint::new(200.0, 300.0),
        AnchorPoint::new(100.0, 400.0),
    ];

    let cells = generate_voronoi_cells(500.0, 500.0, &seeds, 2.0);
    assert_eq!(cells.len(), 4);
    for cell in &cells {
        assert!(!cell.to_path_data().is_empty());
        assert!(cell.to_path_data().closed);
    }
}

#[test]
fn test_lsystem_fractals() {
    // 1. Tree
    let tree = generate_lsystem(LSystemPreset::Tree, 3, 200.0, 400.0, 8.0);
    assert!(!tree.is_empty());

    // 2. Dragon
    let dragon = generate_lsystem(LSystemPreset::Dragon, 6, 200.0, 200.0, 5.0);
    assert!(!dragon.is_empty());

    // 3. Snowflake
    let snowflake = generate_lsystem(LSystemPreset::Snowflake, 2, 200.0, 200.0, 6.0);
    assert!(!snowflake.is_empty());

    // 4. Hilbert
    let hilbert = generate_lsystem(LSystemPreset::Hilbert, 3, 200.0, 200.0, 10.0);
    assert!(!hilbert.is_empty());
}

#[test]
fn test_vector_qr_generation() {
    let qr = generate_vector_qr("https://github.com/AI-SLOP-BOX/amata", 200.0, 200.0, 300.0);
    assert!(qr.is_ok());
    let path = qr.unwrap();
    assert!(!path.is_empty());
    assert!(path.closed);
}

#[test]
fn test_cli_phase6_subcommands() {
    let temp_dir = std::env::temp_dir();
    let out_voronoi = temp_dir.join("p6_voronoi.svg");
    let out_tree = temp_dir.join("p6_tree.svg");
    let out_qr = temp_dir.join("p6_qr.svg");

    // 1. Voronoi CLI
    let cli_voronoi = Cli {
        command: Some(Commands::Voronoi {
            cells: 20,
            padding: 3.0,
            output: out_voronoi.clone(),
        }),
    };
    assert!(run_cli(cli_voronoi).is_ok());
    assert!(out_voronoi.exists());

    // 2. L-System CLI
    let cli_lsystem = Cli {
        command: Some(Commands::Lsystem {
            preset: CliLSystemPreset::Tree,
            iterations: 3,
            output: out_tree.clone(),
        }),
    };
    assert!(run_cli(cli_lsystem).is_ok());
    assert!(out_tree.exists());

    // 3. QR Code CLI
    let cli_qr = Cli {
        command: Some(Commands::Qr {
            text: "Hello IRASU".to_string(),
            output: out_qr.clone(),
        }),
    };
    assert!(run_cli(cli_qr).is_ok());
    assert!(out_qr.exists());

    // Cleanup
    let _ = std::fs::remove_file(out_voronoi);
    let _ = std::fs::remove_file(out_tree);
    let _ = std::fs::remove_file(out_qr);
}
