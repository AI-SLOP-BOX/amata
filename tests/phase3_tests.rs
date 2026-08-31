use irasu_illustrator::cli::{run_cli, Cli, Commands};
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::presets::PresetLibrary;
use irasu_illustrator::core::timeline::{AnimProperty, EaseType, Timeline, Track};
use irasu_illustrator::core::trace::{trace_bitmap_to_path, trace_bitmap_to_polygons};

#[test]
fn test_timeline_keyframe_interpolation() {
    let mut track = Track::new("obj1", AnimProperty::PositionX);
    track.add_keyframe(0, 0.0, EaseType::Linear);
    track.add_keyframe(60, 100.0, EaseType::EaseInOut);
    track.add_keyframe(120, 200.0, EaseType::Linear);

    // Frame 0
    assert_eq!(track.eval_at(0), Some(0.0));
    // Frame 30 (midpoint)
    let mid = track.eval_at(30).unwrap();
    assert!((mid - 50.0).abs() < 5.0);
    // Frame 60
    assert_eq!(track.eval_at(60), Some(100.0));
    // Frame 90
    assert_eq!(track.eval_at(90), Some(150.0));
    // Frame 120
    assert_eq!(track.eval_at(120), Some(200.0));
    // Clamping beyond range
    assert_eq!(track.eval_at(200), Some(200.0));
}

#[test]
fn test_timeline_apply_to_document() {
    let mut doc = Document::default();
    let rect = Object::new_rect("Animated Rect", 0.0, 0.0, 50.0, 50.0, 0.0);
    let rect_id = rect.id.clone();
    doc.add_object(rect);

    let mut tl = Timeline::new(60.0, 2.0);
    let tr_x = tl.add_or_get_track_mut(&rect_id, AnimProperty::PositionX);
    tr_x.add_keyframe(0, 10.0, EaseType::Linear);
    tr_x.add_keyframe(60, 110.0, EaseType::Linear);

    let tr_rot = tl.add_or_get_track_mut(&rect_id, AnimProperty::Rotation);
    tr_rot.add_keyframe(0, 0.0, EaseType::Linear);
    tr_rot.add_keyframe(60, 90.0, EaseType::Linear);

    tl.current_frame = 30;
    tl.apply_to_document(&mut doc);

    let (_layer_idx, obj) = doc.object_by_id(&rect_id).unwrap();
    assert!((obj.transform.x - 60.0).abs() < 1e-4);
    assert!((obj.transform.rotation.to_degrees() - 45.0).abs() < 1e-4);
}

#[test]
fn test_marching_squares_image_trace() {
    // 8x8 bitmap with a 4x4 filled white square in the center
    let w = 8;
    let h = 8;
    let mut pixels = vec![0u8; w * h];
    for y in 2..6 {
        for x in 2..6 {
            pixels[y * w + x] = 255;
        }
    }

    let polys = trace_bitmap_to_polygons(w, h, &pixels, 128);
    assert!(!polys.is_empty());
    let contour = &polys[0];
    assert!(contour.len() >= 3);

    let path = trace_bitmap_to_path(w, h, &pixels, 128);
    assert!(!path.is_empty());
    assert!(path.closed);
}

#[test]
fn test_preset_library_shapes() {
    let heart = PresetLibrary::heart("Love", 100.0, 100.0, 80.0);
    assert!(heart.bounding_box().is_some());

    let arrow = PresetLibrary::arrow("Right Arrow", 200.0, 200.0, 100.0, 30.0);
    assert!(arrow.bounding_box().is_some());

    let gear = PresetLibrary::gear("Machine Gear", 300.0, 300.0, 8, 30.0, 50.0);
    assert!(gear.bounding_box().is_some());

    let bubble = PresetLibrary::speech_bubble("Chat", 400.0, 400.0, 120.0, 80.0);
    assert!(bubble.bounding_box().is_some());

    let portal = PresetLibrary::vfx_portal("Cyber Portal", 500.0, 500.0, 60.0);
    assert!(portal.bounding_box().is_some());
}

#[test]
fn test_cli_phase3_subcommands() {
    let temp_dir = std::env::temp_dir();
    let img_path = temp_dir.join("p3_input.png");
    let out_svg = temp_dir.join("p3_traced.svg");
    let doc_json = temp_dir.join("p3_doc.json");
    let out_aevfx = temp_dir.join("p3_anim.aevfx");

    // 1. Create a dummy image
    let img_buf = image::RgbImage::from_fn(16, 16, |x, y| {
        if (4..12).contains(&x) && (4..12).contains(&y) {
            image::Rgb([255, 255, 255])
        } else {
            image::Rgb([0, 0, 0])
        }
    });
    img_buf.save(&img_path).unwrap();

    // 2. Test CLI Trace
    let cli_trace = Cli {
        command: Some(Commands::Trace {
            input: img_path.clone(),
            threshold: 128,
            output: out_svg.clone(),
        }),
    };
    assert!(run_cli(cli_trace).is_ok());
    assert!(out_svg.exists());

    // 3. Test CLI Animate
    let doc = Document::default();
    irasu_illustrator::io::project::save_project(&doc, &doc_json).unwrap();

    let cli_anim = Cli {
        command: Some(Commands::Animate {
            input: doc_json.clone(),
            output: out_aevfx.clone(),
            fps: 60.0,
            duration: 2.0,
        }),
    };
    assert!(run_cli(cli_anim).is_ok());
    assert!(out_aevfx.exists());

    // Cleanup
    let _ = std::fs::remove_file(img_path);
    let _ = std::fs::remove_file(out_svg);
    let _ = std::fs::remove_file(doc_json);
    let _ = std::fs::remove_file(out_aevfx);
}
