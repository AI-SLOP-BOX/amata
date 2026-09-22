//! Pure geometry for smart-guide distribution detection and axis snapping.

/// Find the best equidistant pair across two candidate lists.
///
/// Each side is a list of `(gap, marker_coord)`: for a horizontal search the
/// left side holds objects whose right edge faces the selection
/// (`gap = sel_min - other_max`) and the right side holds objects whose left
/// edge faces it (`gap = other_min - sel_max`). Returns `(left_idx,
/// right_idx, average_gap)` for the pair whose gaps differ the least, within
/// `tol`; `None` when no such pair exists.
pub fn best_distribution(
    left: &[(f64, f64)],
    right: &[(f64, f64)],
    tol: f64,
) -> Option<(usize, usize, f64)> {
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let mut l: Vec<usize> = (0..left.len()).collect();
    l.sort_by(|&a, &b| {
        left[a]
            .0
            .partial_cmp(&left[b].0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut r: Vec<usize> = (0..right.len()).collect();
    r.sort_by(|&a, &b| {
        right[a]
            .0
            .partial_cmp(&right[b].0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Two-pointer sweep over the sorted gaps: at each step advance the side
    // with the smaller gap, recording the closest pair within `tol`.
    let (mut i, mut j) = (0usize, 0usize);
    let mut best: Option<(usize, usize, f64, f64)> = None; // (l, r, avg, diff)
    while i < l.len() && j < r.len() {
        let gl = left[l[i]].0;
        let gr = right[r[j]].0;
        let diff = (gl - gr).abs();
        if diff <= tol {
            let better = best.map(|(_, _, _, d)| diff < d).unwrap_or(true);
            if better {
                best = Some((l[i], r[j], (gl + gr) * 0.5, diff));
            }
            if gl < gr {
                i += 1;
            } else {
                j += 1;
            }
        } else if gl < gr {
            i += 1;
        } else {
            j += 1;
        }
    }
    best.map(|(li, ri, avg, _)| (li, ri, avg))
}

/// Smallest correction that brings any of the moving object's key edges
/// (`moving`: [min, center, max]) onto one of `targets` within `tol`.
/// Returns `0.0` when nothing is close enough.
pub fn snap_axis(moving: [f64; 3], targets: &[f64], tol: f64) -> f64 {
    let mut best: Option<(f64, f64)> = None; // (|corr|, corr)
    for m in moving {
        for &t in targets {
            let corr = t - m;
            let abs = corr.abs();
            if abs <= tol && best.map(|(a, _)| abs < a).unwrap_or(true) {
                best = Some((abs, corr));
            }
        }
    }
    best.map(|(_, c)| c).unwrap_or(0.0)
}
