use irasu_illustrator::cli::{run_cli, Cli, Commands, CliBooleanOp};
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::path::{FillStyle, PathData, StrokeStyle};
use irasu_illustrator::io::vfx::{doc_to_aevfx_comp, object_to_motion_path_keyframes, object_to_vfx_spline};

#[test]
fn test_doc_to_aevfx_comp() {
    let mut doc = Document::default();
    doc.name = "Hero Intro Comp".to_string();
    doc.width = 1920.0;
    doc.height = 1080.0;

    let mut star = Object::new_star("Gold Star", 960.0, 540.0, 5, 50.0, 120.0);
    star.fill = Some(FillStyle::solid([1.0, 0.85, 0.2, 1.0]));
    star.stroke = Some(StrokeStyle {
        color: [1.0, 1.0, 1.0, 1.0],
        width: 4.0,
        dash_pattern: None,
        ..StrokeStyle::default()
    });
    doc.add_object(star);

    let text = Object::new_text("Title Text", "CYBER VFX", 960.0, 700.0, 48.0);
    doc.add_object(text);

    let comp = doc_to_aevfx_comp(&doc, 60.0, 4.0);

    assert_eq!(comp.comp_name, "Hero Intro Comp");
    assert_eq!(comp.width, 1920);
    assert_eq!(comp.height, 1080);
    assert_eq!(comp.fps, 60.0);
    assert_eq!(comp.total_frames, 240);
    assert_eq!(comp.layers.len(), 2);

    let star_layer = &comp.layers[0];
    assert_eq!(star_layer.layer_type, "Extrusion3D");
    assert_eq!(star_layer.extrusion_depth, 25.0);
    assert!(!star_layer.splines.is_empty());

    let text_layer = &comp.layers[1];
    assert_eq!(text_layer.layer_type, "Text");
    assert_eq!(text_layer.text_content.as_deref(), Some("CYBER VFX"));
}

#[test]
fn test_motion_path_keyframes() {
    let mut path = PathData::new();
    path.push_move_to(0.0, 0.0);
    path.push_line_to(100.0, 0.0);
    path.push_line_to(100.0, 100.0);
    path.push_line_to(200.0, 100.0);

    let obj = Object::new_path("Trajectory", path);
    let keyframes = object_to_motion_path_keyframes(&obj, 10, 2.0, 60.0);

    assert_eq!(keyframes.len(), 10);
    assert_eq!(keyframes[0].frame, 0);
    assert_eq!(keyframes[0].position, [0.0, 0.0, 0.0]);
    assert_eq!(keyframes.last().unwrap().frame, 120);
    assert!((keyframes.last().unwrap().position[0] - 200.0).abs() < 1.0);
}

#[test]
fn test_object_to_vfx_spline_tangents() {
    let mut star = Object::new_star("Star Spline", 0.0, 0.0, 5, 20.0, 50.0);
    star.transform.x = 100.0;
    star.transform.y = 100.0;

    let spline = object_to_vfx_spline(&star);
    assert!(spline.closed);
    assert_eq!(spline.vertices.len(), 10);
    assert_eq!(spline.in_tangents.len(), 10);
    assert_eq!(spline.out_tangents.len(), 10);
}

#[test]
fn test_cli_convert_and_info() {
    let temp_dir = std::env::temp_dir();
    let sample_svg = temp_dir.join("test_irasu_sample.svg");
    let sample_json = temp_dir.join("test_irasu_sample.json");
    let sample_vfx = temp_dir.join("test_irasu_comp.aevfx");

    // 1. Create a sample SVG
    let svg_data = r##"<svg width="800" height="600">
        <rect x="50" y="50" width="200" height="150" fill="#ff0000" rx="10" />
        <circle cx="400" cy="300" r="50" fill="#00ff00" />
    </svg>"##;
    std::fs::write(&sample_svg, svg_data).unwrap();

    // 2. Test CLI Convert SVG -> JSON
    let cli_convert = Cli {
        command: Some(Commands::Convert {
            input: sample_svg.clone(),
            output: sample_json.clone(),
        }),
    };
    assert!(run_cli(cli_convert).is_ok());
    assert!(sample_json.exists());

    // 3. Test CLI Export VFX
    let cli_vfx = Cli {
        command: Some(Commands::ExportVfx {
            input: sample_json.clone(),
            output: sample_vfx.clone(),
            fps: 30.0,
            duration: 3.0,
        }),
    };
    assert!(run_cli(cli_vfx).is_ok());
    assert!(sample_vfx.exists());

    // 4. Test CLI Info
    let cli_info = Cli {
        command: Some(Commands::Info {
            input: sample_json.clone(),
        }),
    };
    assert!(run_cli(cli_info).is_ok());

    // Clean up
    let _ = std::fs::remove_file(sample_svg);
    let _ = std::fs::remove_file(sample_json);
    let _ = std::fs::remove_file(sample_vfx);
}

#[test]
fn test_cli_boolean_operations() {
    let temp_dir = std::env::temp_dir();
    let svg1 = temp_dir.join("test_bool_in1.svg");
    let svg2 = temp_dir.join("test_bool_in2.svg");
    let out_svg = temp_dir.join("test_bool_out.svg");

    std::fs::write(&svg1, r##"<svg width="400" height="400"><rect x="0" y="0" width="100" height="100" fill="#ff0000" /></svg>"##).unwrap();
    std::fs::write(&svg2, r##"<svg width="400" height="400"><rect x="50" y="50" width="100" height="100" fill="#0000ff" /></svg>"##).unwrap();

    let cli_bool = Cli {
        command: Some(Commands::Boolean {
            input1: svg1.clone(),
            input2: svg2.clone(),
            op: CliBooleanOp::Union,
            output: out_svg.clone(),
        }),
    };
    assert!(run_cli(cli_bool).is_ok());
    assert!(out_svg.exists());

    // Clean up
    let _ = std::fs::remove_file(svg1);
    let _ = std::fs::remove_file(svg2);
    let _ = std::fs::remove_file(out_svg);
}
