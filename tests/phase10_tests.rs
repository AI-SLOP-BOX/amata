use irasu_illustrator::cli::{run_cli, Cli, Commands};
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::envelope::apply_envelope_distort;
use irasu_illustrator::core::knife::slice_object_with_line;
use irasu_illustrator::core::path::AnchorPoint;
use irasu_illustrator::core::polar::{apply_polar_transform, PolarMode};
use irasu_illustrator::core::revolve::generate_3d_revolve_obj;

#[test]
fn test_3d_revolve_obj() {
    let profile = Object::new_rect("VaseProfile", 20.0, 10.0, 30.0, 80.0, 0.0);
    let obj_str = generate_3d_revolve_obj(&profile, 0.0, 360.0, 16);
    assert!(obj_str.contains("v "));
    assert!(obj_str.contains("f "));
}

#[test]
fn test_envelope_distort() {
    let art = Object::new_rect("TextBanner", 0.0, 0.0, 100.0, 50.0, 0.0);
    let frame = Object::new_ellipse("OvalFrame", 100.0, 100.0, 60.0, 40.0);

    let warped = apply_envelope_distort(&art, &frame);
    assert!(!warped.to_path_data().is_empty());
}

#[test]
fn test_polar_coordinates() {
    let obj = Object::new_rect("CityBlock", 0.0, 0.0, 200.0, 100.0, 0.0);

    // 1. Rect to Polar
    let polar = apply_polar_transform(&obj, 200.0, 200.0, 400.0, 400.0, PolarMode::RectToPolar);
    assert!(!polar.to_path_data().is_empty());

    // 2. Polar to Rect
    let rect = apply_polar_transform(&polar, 200.0, 200.0, 400.0, 400.0, PolarMode::PolarToRect);
    assert!(!rect.to_path_data().is_empty());
}

#[test]
fn test_knife_polygon_slice() {
    let obj = Object::new_rect("Cake", 0.0, 0.0, 100.0, 100.0, 0.0);
    let p1 = AnchorPoint::new(-10.0, 50.0);
    let p2 = AnchorPoint::new(110.0, 50.0);

    let slices = slice_object_with_line(&obj, p1, p2);
    assert!(slices.is_some());
    let (part_a, part_b) = slices.unwrap();
    assert!(!part_a.to_path_data().is_empty());
    assert!(!part_b.to_path_data().is_empty());
}

#[test]
fn test_cli_phase10_subcommands() {
    let temp_dir = std::env::temp_dir();
    let in_profile = temp_dir.join("p10_profile.svg");
    let in_frame = temp_dir.join("p10_frame.svg");
    let out_obj = temp_dir.join("p10_vase.obj");
    let out_env = temp_dir.join("p10_env.svg");
    let out_polar = temp_dir.join("p10_polar.svg");
    let out_slice = temp_dir.join("p10_slice.svg");

    let mut doc_prof = Document::default();
    doc_prof.add_object(Object::new_rect("Prof", 10.0, 0.0, 20.0, 60.0, 0.0));
    let prof_svg = irasu_illustrator::io::svg::export_svg(&doc_prof);
    std::fs::write(&in_profile, &prof_svg).unwrap();

    let mut doc_frame = Document::default();
    doc_frame.add_object(Object::new_ellipse("Frame", 100.0, 100.0, 50.0, 30.0));
    let frame_svg = irasu_illustrator::io::svg::export_svg(&doc_frame);
    std::fs::write(&in_frame, &frame_svg).unwrap();

    // 1. Revolve CLI
    let cli_rev = Cli {
        command: Some(Commands::Revolve {
            input: in_profile.clone(),
            angle: 360.0,
            segments: 16,
            output: out_obj.clone(),
        }),
    };
    assert!(run_cli(cli_rev).is_ok());
    assert!(out_obj.exists());

    // 2. Envelope CLI
    let cli_env = Cli {
        command: Some(Commands::Envelope {
            art: in_profile.clone(),
            envelope: in_frame.clone(),
            output: out_env.clone(),
        }),
    };
    assert!(run_cli(cli_env).is_ok());
    assert!(out_env.exists());

    // 3. Polar CLI
    let cli_pol = Cli {
        command: Some(Commands::Polar {
            input: in_profile.clone(),
            output: out_polar.clone(),
        }),
    };
    assert!(run_cli(cli_pol).is_ok());
    assert!(out_polar.exists());

    // 4. Slice CLI
    let cli_sli = Cli {
        command: Some(Commands::Slice {
            input: in_profile.clone(),
            output: out_slice.clone(),
        }),
    };
    assert!(run_cli(cli_sli).is_ok());
    assert!(out_slice.exists());

    // Cleanup
    let _ = std::fs::remove_file(in_profile);
    let _ = std::fs::remove_file(in_frame);
    let _ = std::fs::remove_file(out_obj);
    let _ = std::fs::remove_file(out_env);
    let _ = std::fs::remove_file(out_polar);
    let _ = std::fs::remove_file(out_slice);
}
