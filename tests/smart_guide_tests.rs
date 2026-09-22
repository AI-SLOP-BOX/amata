use irasu_illustrator::core::smart_guides::{best_distribution, snap_axis};

#[test]
fn distribution_picks_matching_gaps() {
    // Only the 24 / 24.5 pair is within tol 1.0.
    let left = [(24.0, 86.0), (50.0, 60.0)];
    let right = [(24.5, 184.0), (80.0, 240.0)];
    let (li, ri, avg) = best_distribution(&left, &right, 1.0).expect("pair");
    assert_eq!(li, 0);
    assert_eq!(ri, 0);
    assert!((avg - 24.25).abs() < 1e-9);
}

#[test]
fn distribution_none_when_gaps_differ() {
    let left = [(10.0, 0.0)];
    let right = [(30.0, 0.0)];
    assert!(best_distribution(&left, &right, 1.0).is_none());
}

#[test]
fn distribution_none_on_empty_side() {
    let right = [(10.0, 0.0)];
    assert!(best_distribution(&[], &right, 1.0).is_none());
    assert!(best_distribution(&[(10.0, 0.0)], &[], 1.0).is_none());
}

#[test]
fn distribution_finds_best_across_unsorted_input() {
    // Input order is irrelevant: the sweep runs over sorted gaps.
    let left = [(40.0, 0.0), (12.0, 0.0), (25.0, 0.0)];
    let right = [(12.4, 0.0), (60.0, 0.0), (39.0, 0.0)];
    let (li, ri, avg) = best_distribution(&left, &right, 1.0).expect("pair");
    assert_eq!(left[li].0, 12.0);
    assert_eq!(right[ri].0, 12.4);
    assert!((avg - 12.2).abs() < 1e-9);
}

#[test]
fn snap_axis_nearest_edge_within_tolerance() {
    let targets = [50.0, 101.5, 300.0];
    let corr = snap_axis([100.0, 110.0, 120.0], &targets, 4.0);
    assert!((corr - 1.5).abs() < 1e-9);
}

#[test]
fn snap_axis_out_of_tolerance_is_zero() {
    assert_eq!(snap_axis([100.0, 110.0, 120.0], &[130.0, 200.0], 4.0), 0.0);
}

#[test]
fn snap_axis_prefers_smallest_correction() {
    // min edge 100 can reach 103 (corr 3) or 101 (corr 1) → picks 1.
    let targets = [103.0, 101.0];
    let corr = snap_axis([100.0, 110.0, 120.0], &targets, 4.0);
    assert!((corr - 1.0).abs() < 1e-9);
}

#[test]
fn snap_axis_uses_center_edge_too() {
    // Only the center edge (110) is near target 112.
    let targets = [112.0];
    let corr = snap_axis([100.0, 110.0, 120.0], &targets, 4.0);
    assert!((corr - 2.0).abs() < 1e-9);
}
