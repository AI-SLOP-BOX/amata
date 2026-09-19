#![allow(clippy::field_reassign_with_default)]
use irasu_illustrator::cli::{run_cli, Cli, Commands};
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::mesh3d::{extrude_polygon_3d, triangulate_polygon};
use irasu_illustrator::core::morph::{morph_paths, morph_polygons, resample_polygon};
use irasu_illustrator::core::offset::{offset_path, offset_polygon, outline_stroke};
use irasu_illustrator::core::path::{AnchorPoint, PathData};
use irasu_illustrator::io::vfx::export_doc_to_obj;

#[test]
fn test_morph_engine() {
    let square = vec![
        AnchorPoint::new(0.0, 0.0),
        AnchorPoint::new(100.0, 0.0),
        AnchorPoint::new(100.0, 100.0),
        AnchorPoint::new(0.0, 100.0),
    ];

    let triangle = vec![
        AnchorPoint::new(50.0, 0.0),
        AnchorPoint::new(100.0, 100.0),
        AnchorPoint::new(0.0, 100.0),
    ];

    let resampled_sq = resample_polygon(&square, 16);
    assert_eq!(resampled_sq.len(), 16);

    let morphed_half = morph_polygons(&square, &triangle, 0.5);
    assert_eq!(morphed_half.len(), 32);

    let path_sq = PathData::from_polygon_points(&square, true);
    let path_tri = PathData::from_polygon_points(&triangle, true);
    let morphed_path = morph_paths(&path_sq, &path_tri, 0.5);
    assert!(!morphed_path.is_empty());
}

#[test]
fn test_offset_and_outline_stroke() {
    let square = vec![
        AnchorPoint::new(0.0, 0.0),
        AnchorPoint::new(100.0, 0.0),
        AnchorPoint::new(100.0, 100.0),
        AnchorPoint::new(0.0, 100.0),
    ];

    // Outward offset
    let expanded = offset_polygon(&square, 10.0);
    assert_eq!(expanded.len(), 4);
    assert!(expanded.iter().any(|p| p.x > 100.0));
    assert!(expanded.iter().any(|p| p.y > 100.0));

    // Path offset
    let sq_path = PathData::from_polygon_points(&square, true);
    let off_path = offset_path(&sq_path, 15.0);
    assert!(!off_path.is_empty());
    assert!(off_path.closed);

    // Outline stroke on open line
    let mut line_path = PathData::new();
    line_path.push_move_to(0.0, 50.0);
    line_path.push_line_to(100.0, 50.0);
    let ribbon = outline_stroke(&line_path, 10.0);
    assert!(!ribbon.is_empty());
    assert!(ribbon.closed);
}

#[test]
fn test_triangulation_and_3d_mesh_extrusion() {
    let square = vec![
        AnchorPoint::new(0.0, 0.0),
        AnchorPoint::new(100.0, 0.0),
        AnchorPoint::new(100.0, 100.0),
        AnchorPoint::new(0.0, 100.0),
    ];

    let tris = triangulate_polygon(&square);
    assert_eq!(tris.len(), 2);

    let mesh = extrude_polygon_3d(&square, 25.0, 2.0);
    // 4 front + 4 back + 16 wall vertices = 24 vertices
    assert_eq!(mesh.vertices.len(), 24);
    // 2 front + 2 back + 8 wall tris = 12 faces
    assert_eq!(mesh.faces.len(), 12);

    let obj_str = mesh.to_obj("Extruded_Square");
    assert!(obj_str.contains("o Extruded_Square"));
    assert!(obj_str.contains("v "));
    assert!(obj_str.contains("vn "));
    assert!(obj_str.contains("f "));
}

#[test]
fn test_doc_to_3d_obj_export() {
    let mut doc = Document::default();
    doc.name = "Logo_3D".to_string();

    let star = Object::new_star("Gold_Star", 500.0, 500.0, 5, 40.0, 100.0);
    doc.add_object(star);

    let obj_export = export_doc_to_obj(&doc, 30.0, 1.5);
    assert!(obj_export.contains("# IRASU Illustrator 3D Mesh Export"));
    assert!(obj_export.contains("v "));
    assert!(obj_export.contains("vn "));
    assert!(obj_export.contains("f "));
}

#[test]
fn test_cli_phase2_subcommands() {
    let temp_dir = std::env::temp_dir();
    let in_svg1 = temp_dir.join("p2_in1.svg");
    let in_svg2 = temp_dir.join("p2_in2.svg");
    let out_obj = temp_dir.join("p2_out.obj");
    let out_morph = temp_dir.join("p2_morph.svg");
    let out_offset = temp_dir.join("p2_offset.svg");
    let out_outline = temp_dir.join("p2_outline.svg");

    std::fs::write(
        &in_svg1,
        r##"<svg width="200" height="200"><rect x="10" y="10" width="80" height="80" /></svg>"##,
    )
    .unwrap();
    std::fs::write(
        &in_svg2,
        r##"<svg width="200" height="200"><circle cx="100" cy="100" r="40" /></svg>"##,
    )
    .unwrap();

    // 1. Export 3D
    let cli_3d = Cli {
        command: Some(Commands::Export3d {
            input: in_svg1.clone(),
            output: out_obj.clone(),
            depth: 25.0,
            bevel: 2.0,
        }),
    };
    assert!(run_cli(cli_3d).is_ok());
    assert!(out_obj.exists());

    // 2. Morph
    let cli_morph = Cli {
        command: Some(Commands::Morph {
            input1: in_svg1.clone(),
            input2: in_svg2.clone(),
            t: 0.5,
            output: out_morph.clone(),
        }),
    };
    assert!(run_cli(cli_morph).is_ok());
    assert!(out_morph.exists());

    // 3. Offset
    let cli_offset = Cli {
        command: Some(Commands::Offset {
            input: in_svg1.clone(),
            delta: 8.0,
            output: out_offset.clone(),
        }),
    };
    assert!(run_cli(cli_offset).is_ok());
    assert!(out_offset.exists());

    // 4. Outline Stroke
    let cli_outline = Cli {
        command: Some(Commands::OutlineStroke {
            input: in_svg1.clone(),
            width: 6.0,
            output: out_outline.clone(),
        }),
    };
    assert!(run_cli(cli_outline).is_ok());
    assert!(out_outline.exists());

    // Cleanup
    let _ = std::fs::remove_file(in_svg1);
    let _ = std::fs::remove_file(in_svg2);
    let _ = std::fs::remove_file(out_obj);
    let _ = std::fs::remove_file(out_morph);
    let _ = std::fs::remove_file(out_offset);
    let _ = std::fs::remove_file(out_outline);
}
