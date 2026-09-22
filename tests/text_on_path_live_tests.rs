use irasu_illustrator::core::document::{Object, ObjectType, TextPathSide, TextStyle};
use irasu_illustrator::core::path::{AnchorPoint, PathData};
use irasu_illustrator::core::text_path::{
    arc_point_at, path_total_length, project_to_arc_length, text_on_path_outlines,
};

fn line_path(x2: f64, y2: f64) -> PathData {
    PathData::from_line(0.0, 0.0, x2, y2)
}

#[test]
fn outlines_non_empty_for_simple_text() {
    let style = TextStyle::new("Inter, sans-serif", 24.0);
    let out = text_on_path_outlines(&line_path(200.0, 0.0), "AB", &style, 0.0, TextPathSide::Top);
    assert!(!out.elements.is_empty());
}

#[test]
fn bottom_side_differs_from_top() {
    let style = TextStyle::new("Inter, sans-serif", 24.0);
    let path = line_path(200.0, 0.0);
    let top = text_on_path_outlines(&path, "A", &style, 0.0, TextPathSide::Top);
    let bottom = text_on_path_outlines(&path, "A", &style, 0.0, TextPathSide::Bottom);
    assert_ne!(format!("{top:?}"), format!("{bottom:?}"));
}

#[test]
fn start_offset_moves_first_glyph() {
    let style = TextStyle::new("Inter, sans-serif", 24.0);
    let path = line_path(300.0, 0.0);
    let at0 = text_on_path_outlines(&path, "A", &style, 0.0, TextPathSide::Top);
    let at50 = text_on_path_outlines(&path, "A", &style, 50.0, TextPathSide::Top);
    assert_ne!(format!("{at0:?}"), format!("{at50:?}"));
}

#[test]
fn total_length_of_straight_line() {
    assert!((path_total_length(&line_path(100.0, 0.0)) - 100.0).abs() < 1e-6);
}

#[test]
fn arc_point_at_zero_is_path_start() {
    let p = arc_point_at(&line_path(100.0, 0.0), 0.0).expect("point");
    assert!(p.distance(AnchorPoint::new(0.0, 0.0)) < 1e-6);
}

#[test]
fn arc_point_at_clamps_to_total() {
    let p = arc_point_at(&line_path(100.0, 0.0), 999.0).expect("point");
    assert!((p.x - 100.0).abs() < 1e-6);
}

#[test]
fn project_returns_nearest_arc_length() {
    let s = project_to_arc_length(&line_path(100.0, 0.0), 30.0, 5.0).expect("proj");
    assert!((s - 30.0).abs() < 1e-6);
}

#[test]
fn constructor_makes_live_text_on_path() {
    let obj = Object::new_text_on_path("T", "Hi", line_path(150.0, 0.0));
    match &obj.object_type {
        ObjectType::TextOnPath { text, .. } => assert_eq!(text, "Hi"),
        other => panic!("wrong type: {other:?}"),
    }
    assert!(!obj.to_path_data().elements.is_empty());
    assert!(obj.bounding_box().is_some());
}

#[test]
fn hit_test_covers_outlined_glyphs() {
    // 'D' is a single solid contour: its glyph centre must hit.
    let obj = Object::new_text_on_path("T", "D", line_path(200.0, 0.0));
    // font 24 → char width 14.4; glyph box y ∈ [-12, 12]; centre = (7.2, 0).
    assert!(obj.hit_test(7.2, 0.0));
    // Far away from both path and glyphs.
    assert!(!obj.hit_test(150.0, 80.0));
}
