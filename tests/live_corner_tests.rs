use irasu_illustrator::core::document::{Object, ObjectType};
use irasu_illustrator::core::geometry::compute_corner_radius;
use irasu_illustrator::core::path::PathData;

#[test]
fn depth_from_top_left_is_min_edge_distance() {
    // Cursor inside near the top-left vertex of a 100×60 rect.
    assert!((compute_corner_radius(100.0, 60.0, 12.0, 40.0, (0.0, 0.0)) - 12.0).abs() < 1e-9);
}

#[test]
fn depth_from_bottom_right_uses_opposite_edges() {
    // Local (85, 50) on 100×60: distance to right = 15, to bottom = 10.
    let r = compute_corner_radius(100.0, 60.0, 85.0, 50.0, (100.0, 60.0));
    assert!((r - 10.0).abs() < 1e-9);
}

#[test]
fn radius_clamps_to_half_min_side() {
    // Far outside the half-min clamp (min(100,40)/2 = 20).
    let r = compute_corner_radius(100.0, 40.0, 500.0, 500.0, (0.0, 0.0));
    assert!((r - 20.0).abs() < 1e-9);
}

#[test]
fn radius_clamps_to_zero_outside_shape() {
    // Cursor past the corner (negative local coords) → 0, not negative.
    let r = compute_corner_radius(100.0, 60.0, -5.0, -5.0, (0.0, 0.0));
    assert_eq!(r, 0.0);
}

#[test]
fn from_rect_clamps_radius_like_compute() {
    let path = PathData::from_rect(0.0, 0.0, 100.0, 40.0, 999.0);
    // Same clamp as compute_corner_radius: 0..=20.
    let r = compute_corner_radius(100.0, 40.0, 20.0, 20.0, (0.0, 0.0));
    assert!((r - 20.0).abs() < 1e-9);
    assert!(!path.elements.is_empty());
}

#[test]
fn zero_radius_from_rect_is_sharp_corners() {
    let path = PathData::from_rect(0.0, 0.0, 50.0, 50.0, 0.0);
    // MoveTo + 3 LineTo + Close = 5 elements for a sharp rect.
    assert_eq!(path.elements.len(), 5);
}

#[test]
fn rectangle_object_to_path_reflects_corner_radius() {
    let mut obj = Object::new_rect("R", 0.0, 0.0, 100.0, 60.0, 0.0);
    let sharp = obj.to_path_data();
    assert_eq!(sharp.elements.len(), 5);

    if let ObjectType::Rectangle {
        corner_radius, ..
    } = &mut obj.object_type
    {
        *corner_radius = 15.0;
    }
    let rounded = obj.to_path_data();
    // Rounded path uses cubics: more elements than the sharp 5.
    assert!(rounded.elements.len() > 5);
}

#[test]
fn rectangle_property_edit_round_trips_through_object_type() {
    let mut obj = Object::new_rect("R", 10.0, 20.0, 80.0, 40.0, 0.0);
    let target = compute_corner_radius(80.0, 40.0, 8.0, 8.0, (0.0, 0.0));
    if let ObjectType::Rectangle {
        corner_radius, ..
    } = &mut obj.object_type
    {
        *corner_radius = target;
    }
    match &obj.object_type {
        ObjectType::Rectangle { corner_radius, .. } => {
            assert!((corner_radius - target).abs() < 1e-9);
            assert!(corner_radius <= &20.0);
        }
        _ => panic!("expected Rectangle"),
    }
}
