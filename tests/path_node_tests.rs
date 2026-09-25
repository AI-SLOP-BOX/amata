use irasu_illustrator::core::path::{AnchorPoint, PathData, PathElement};

fn line_path() -> PathData {
    let mut p = PathData::new();
    p.push_move_to(0.0, 0.0);
    p.push_line_to(100.0, 0.0);
    p.push_line_to(100.0, 100.0);
    p.push_line_to(0.0, 100.0);
    p.close();
    p
}

#[test]
fn split_line_segment_adds_anchor_at_midpoint() {
    let mut p = line_path();
    assert_eq!(p.anchor_count(), 4);
    assert!(p.split_segment(1, 0.5));
    assert_eq!(p.anchor_count(), 5);
    match &p.elements[1] {
        PathElement::LineTo(pt) => {
            assert!((pt.x - 50.0).abs() < 1e-9);
            assert!((pt.y - 0.0).abs() < 1e-9);
        }
        other => panic!("expected LineTo, got {other:?}"),
    }
    match &p.elements[2] {
        PathElement::LineTo(pt) => {
            assert!((pt.x - 100.0).abs() < 1e-9);
            assert!((pt.y - 0.0).abs() < 1e-9);
        }
        other => panic!("expected LineTo, got {other:?}"),
    }
    assert!(matches!(p.elements.last(), Some(PathElement::ClosePath)));
}

#[test]
fn split_curve_uses_de_casteljau() {
    let mut p = PathData::new();
    p.push_move_to(0.0, 0.0);
    p.push_curve_to(
        AnchorPoint::new(0.0, 100.0),
        AnchorPoint::new(100.0, 100.0),
        AnchorPoint::new(100.0, 0.0),
    );
    assert!(p.split_segment(1, 0.5));
    assert_eq!(p.anchor_count(), 3);

    let PathElement::CurveTo(left) = &p.elements[1] else {
        panic!("expected CurveTo left half");
    };
    let PathElement::CurveTo(right) = &p.elements[2] else {
        panic!("expected CurveTo right half");
    };
    // Halves join exactly at the split point.
    assert!((left.end.x - right.start.x).abs() < 1e-9);
    assert!((left.end.y - right.start.y).abs() < 1e-9);
    // Original endpoints preserved.
    assert!((left.start.x - 0.0).abs() < 1e-9);
    assert!((left.start.y - 0.0).abs() < 1e-9);
    assert!((right.end.x - 100.0).abs() < 1e-9);
    assert!((right.end.y - 0.0).abs() < 1e-9);
    // t=0.5 on the symmetric cubic is (50, 75).
    assert!((left.end.x - 50.0).abs() < 1e-6);
    assert!((left.end.y - 75.0).abs() < 1e-6);
}

#[test]
fn remove_middle_anchor_reconnects() {
    let mut p = line_path();
    assert!(p.remove_anchor(1));
    assert_eq!(p.anchor_count(), 3);
    match &p.elements[1] {
        PathElement::LineTo(pt) => {
            assert!((pt.x - 100.0).abs() < 1e-9);
            assert!((pt.y - 100.0).abs() < 1e-9);
        }
        other => panic!("expected LineTo, got {other:?}"),
    }
    assert!(matches!(p.elements.last(), Some(PathElement::ClosePath)));
}

#[test]
fn remove_first_anchor_of_closed_path_promotes_next() {
    let mut p = line_path();
    assert!(p.remove_anchor(0));
    assert_eq!(p.anchor_count(), 3);
    match &p.elements[0] {
        PathElement::MoveTo(pt) => {
            assert!((pt.x - 100.0).abs() < 1e-9);
            assert!((pt.y - 0.0).abs() < 1e-9);
        }
        other => panic!("expected MoveTo, got {other:?}"),
    }
    assert!(matches!(p.elements.last(), Some(PathElement::ClosePath)));
}

#[test]
fn remove_anchor_before_curve_shifts_start_and_handle() {
    let mut p = PathData::new();
    p.push_move_to(0.0, 0.0);
    p.push_line_to(50.0, 0.0);
    p.push_curve_to(
        AnchorPoint::new(60.0, 50.0),
        AnchorPoint::new(90.0, 50.0),
        AnchorPoint::new(100.0, 0.0),
    );
    assert_eq!(p.anchor_count(), 3);
    assert!(p.remove_anchor(1));
    assert_eq!(p.anchor_count(), 2);
    let PathElement::CurveTo(seg) = &p.elements[1] else {
        panic!("expected CurveTo");
    };
    assert!((seg.start.x - 0.0).abs() < 1e-9);
    assert!((seg.start.y - 0.0).abs() < 1e-9);
    // control1 shifted by (prev - deleted) = (-50, 0).
    assert!((seg.control1.x - 10.0).abs() < 1e-9);
    assert!((seg.control1.y - 50.0).abs() < 1e-9);
    assert!((seg.end.x - 100.0).abs() < 1e-9);
}

#[test]
fn remove_refuses_below_three_anchors() {
    let mut p = PathData::new();
    p.push_move_to(0.0, 0.0);
    p.push_line_to(10.0, 0.0);
    assert_eq!(p.anchor_count(), 2);
    assert!(!p.remove_anchor(1));
    assert!(!p.remove_anchor(0));
    assert_eq!(p.anchor_count(), 2);
}

#[test]
fn remove_refuses_closepath_and_out_of_range() {
    let mut p = line_path();
    assert!(!p.remove_anchor(4)); // ClosePath
    assert!(!p.remove_anchor(99)); // out of range
    assert_eq!(p.anchor_count(), 4);
}

#[test]
fn split_refuses_move_to_and_out_of_range() {
    let mut p = line_path();
    assert!(!p.split_segment(0, 0.5));
    assert!(!p.split_segment(99, 0.5));
    assert_eq!(p.anchor_count(), 4);
}

#[test]
fn stroke_subpaths_keep_the_two_point_runs_that_fills_drop() {
    // The most common thing a pen draws: an open path with two anchors.
    let mut open = PathData::new();
    open.push_move_to(0.0, 0.0);
    open.push_line_to(50.0, 0.0);

    // There is nothing to triangulate from two points…
    assert!(open.to_subpaths(4).is_empty());
    // …but a line still has to be drawn.
    let runs = open.to_stroke_subpaths(4);
    assert_eq!(runs.len(), 1, "the open run must survive");
    assert_eq!(runs[0].len(), 2);

    // Everything a fill can see, a stroke sees too.
    let closed = line_path();
    assert_eq!(closed.to_stroke_subpaths(4), closed.to_subpaths(4));
    assert_eq!(closed.to_stroke_subpaths(4).len(), 1);
}
