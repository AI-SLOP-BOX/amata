//! Stroke tessellation: turn a stroked subpath into the *filled rings* that
//! reproduce it faithfully, plus the arrowhead geometry shared by every
//! renderer.
//!
//! egui's `Stroke` only carries a width and a colour — no dash array, no caps,
//! no joins and no miter limit — while the SVG and PDF writers honour all of
//! them.  Left to its own devices the canvas therefore showed dashed lines as
//! solid, round caps as butt caps, and cut sharp miters off at ~90°.  This
//! module closes that gap by building the stroke outline in document
//! coordinates; the canvas transforms the resulting rings to screen space and
//! fills them (see `crate::ui::canvas::stroke_paint`).
//!
//! Arrowheads live here too so the canvas and the SVG exporter agree on where
//! a head's tip sits and which way it points.

use crate::core::path::{AnchorPoint, ArrowHead, StrokeCap, StrokeJoin, StrokeStyle};

/// Arrowhead length as a multiple of the stroke width.  [`StrokeStyle`] has no
/// dedicated size field (Illustrator exposes one), so the head is derived from
/// the width: `4x` keeps it clearly readable without swallowing the line.
pub const ARROW_SIZE_FACTOR: f64 = 4.0;

const EPS: f64 = 1e-12;

/// Unit vector `(dx, dy)`, or `(0, 0)` for a degenerate segment.
fn unit(dx: f64, dy: f64) -> (f64, f64) {
    let l = (dx * dx + dy * dy).sqrt();
    if l < EPS {
        (0.0, 0.0)
    } else {
        (dx / l, dy / l)
    }
}

/// Left-of-travel normal of a unit direction (rotate by +90°).
fn normal(d: (f64, f64)) -> (f64, f64) {
    (-d.1, d.0)
}

/// A scalar usable as a size: finite and strictly positive.
///
/// Kept as a predicate so the rejection reads forwards — and so `NaN` (which a
/// negated comparison would also have caught) is explicitly excluded, since a
/// `NaN` width would otherwise poison every vertex of the outline.
fn positive(v: f64) -> bool {
    v.is_finite() && v > 0.0
}

/// Unit direction of every segment of `pts`.
///
/// Zero-length segments inherit a neighbour's direction so a doubled vertex
/// doesn't poison the outline of the whole subpath.
fn segment_dirs(pts: &[AnchorPoint], closed: bool) -> Vec<(f64, f64)> {
    let n = pts.len();
    let seg_count = if closed { n } else { n.saturating_sub(1) };
    if seg_count == 0 {
        return Vec::new();
    }
    let mut dirs: Vec<(f64, f64)> = (0..seg_count)
        .map(|i| {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            unit(b.x - a.x, b.y - a.y)
        })
        .collect();

    // Forward-fill from the left, then back-fill any leading gaps.
    let mut last = (0.0, 0.0);
    for d in dirs.iter_mut() {
        if *d == (0.0, 0.0) {
            *d = last;
        } else {
            last = *d;
        }
    }
    if dirs[0] == (0.0, 0.0) {
        let first_ok = dirs
            .iter()
            .copied()
            .find(|d| *d != (0.0, 0.0))
            .unwrap_or((1.0, 0.0));
        for d in dirs.iter_mut() {
            if *d == (0.0, 0.0) {
                *d = first_ok;
            } else {
                break;
            }
        }
    }
    dirs
}

fn push_offset(out: &mut Vec<AnchorPoint>, p: AnchorPoint, n: (f64, f64), delta: f64) {
    out.push(AnchorPoint::new(p.x + n.0 * delta, p.y + n.1 * delta));
}

/// Intersection of the two offset lines through `p`, or `None` when they are
/// (anti)parallel.  Works for the inner side of a turn as well as the outer.
fn miter_point(p: AnchorPoint, na: (f64, f64), nb: (f64, f64), delta: f64) -> Option<AnchorPoint> {
    let mx = na.0 + nb.0;
    let my = na.1 + nb.1;
    let ml = (mx * mx + my * my).sqrt();
    if ml < 1e-9 {
        return None;
    }
    let (ux, uy) = (mx / ml, my / ml);
    let denom = ux * na.0 + uy * na.1;
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = delta / denom;
    Some(AnchorPoint::new(p.x + ux * t, p.y + uy * t))
}

/// Append an arc of radius `r` around `p`, from angle `a0` through `sweep`
/// radians.  Endpoints are left to the caller so the arc can be spliced
/// between two already-emitted corner points.
fn push_arc(out: &mut Vec<AnchorPoint>, p: AnchorPoint, a0: f64, sweep: f64, r: f64) {
    if r <= 0.0 || sweep.abs() < 1e-9 {
        return;
    }
    let steps = ((sweep.abs() / (std::f64::consts::PI / 12.0)).ceil() as usize).clamp(1, 96);
    for k in 1..steps {
        let a = a0 + sweep * (k as f64) / (steps as f64);
        out.push(AnchorPoint::new(p.x + r * a.cos(), p.y + r * a.sin()));
    }
}

/// Emit the offset point(s) for an interior vertex where the segment
/// directions change from `na` to `nb`.
///
/// Only the *outer* side of the turn gets join geometry; the inner side is
/// always the exact miter intersection, which by construction can never
/// exceed the miter limit.
#[allow(clippy::too_many_arguments)]
fn push_join(
    out: &mut Vec<AnchorPoint>,
    p: AnchorPoint,
    na: (f64, f64),
    nb: (f64, f64),
    delta: f64,
    join: StrokeJoin,
    miter_limit: f64,
) {
    let cross = na.0 * nb.1 - na.1 * nb.0;
    let dot = (na.0 * nb.0 + na.1 * nb.1).clamp(-1.0, 1.0);

    // Collinear (or doubled) vertices need no join at all.
    if cross.abs() < 1e-10 && dot > 0.0 {
        push_offset(out, p, na, delta);
        return;
    }

    // A positive cross product means the path bends towards the +normal side,
    // making that the *inner* side of the turn.
    let outer = if cross > 0.0 {
        delta < 0.0
    } else if cross < 0.0 {
        delta > 0.0
    } else {
        false
    };

    if !outer {
        match miter_point(p, na, nb, delta) {
            Some(m) => out.push(m),
            None => {
                push_offset(out, p, na, delta);
                push_offset(out, p, nb, delta);
            }
        }
        return;
    }

    match join {
        StrokeJoin::Bevel => {
            push_offset(out, p, na, delta);
            push_offset(out, p, nb, delta);
        }
        StrokeJoin::Round => {
            push_offset(out, p, na, delta);
            let sgn = if delta < 0.0 { -1.0 } else { 1.0 };
            let a0 = (na.1 * sgn).atan2(na.0 * sgn);
            // The signed angle from `na` to `nb` is invariant when both are
            // mirrored, so the same sweep serves either offset side.
            push_arc(out, p, a0, cross.atan2(dot), delta.abs());
            push_offset(out, p, nb, delta);
        }
        StrokeJoin::Miter => {
            // Miter length / stroke width = 1 / cos(theta / 2).
            let miter_ratio = if 1.0 + dot > 1e-12 {
                (2.0 / (1.0 + dot)).sqrt()
            } else {
                f64::INFINITY
            };
            match miter_point(p, na, nb, delta) {
                Some(m) if miter_ratio <= miter_limit.max(1.0) => out.push(m),
                _ => {
                    push_offset(out, p, na, delta);
                    push_offset(out, p, nb, delta);
                }
            }
        }
    }
}

/// Offset `pts` by `delta` along the left-of-travel normal, honouring
/// [`StrokeJoin`] and the miter limit.  Open polylines get plain endpoint
/// offsets; closed ones join the last vertex back to the first.
fn offset_polyline(
    pts: &[AnchorPoint],
    closed: bool,
    delta: f64,
    join: StrokeJoin,
    miter_limit: f64,
) -> Vec<AnchorPoint> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let dirs = segment_dirs(pts, closed);
    if dirs.is_empty() {
        return pts.to_vec();
    }
    let normals: Vec<(f64, f64)> = dirs.iter().copied().map(normal).collect();
    let seg_count = normals.len();

    let mut out = Vec::with_capacity(n + 8);
    for i in 0..n {
        let p = pts[i];
        let has_prev = closed || i > 0;
        let has_next = closed || i + 1 < n;
        if !has_prev {
            push_offset(&mut out, p, normals[0], delta);
            continue;
        }
        if !has_next {
            push_offset(&mut out, p, normals[seg_count - 1], delta);
            continue;
        }
        let na = if i == 0 {
            normals[seg_count - 1]
        } else {
            normals[i - 1]
        };
        let nb = normals[i];
        push_join(&mut out, p, na, nb, delta, join, miter_limit);
    }
    out
}

/// Geometry for the cap at one end of an open polyline, in the order it has
/// to be spliced between the two offset sides.
///
/// `n` is the endpoint's segment normal, `d` its direction.  `at_end == false`
/// describes the start cap, which is emitted after the reversed second side.
fn cap_points(
    center: AnchorPoint,
    n: (f64, f64),
    d: (f64, f64),
    hw: f64,
    cap: StrokeCap,
    at_end: bool,
) -> Vec<AnchorPoint> {
    match cap {
        StrokeCap::Butt => Vec::new(),
        StrokeCap::Square => {
            let ext = if at_end { hw } else { -hw };
            let a = AnchorPoint::new(
                center.x + n.0 * hw + d.0 * ext,
                center.y + n.1 * hw + d.1 * ext,
            );
            let b = AnchorPoint::new(
                center.x - n.0 * hw + d.0 * ext,
                center.y - n.1 * hw + d.1 * ext,
            );
            if at_end {
                vec![a, b]
            } else {
                vec![b, a]
            }
        }
        StrokeCap::Round => {
            // Sweep from `+n` through `+d` to `-n` (end cap), or from `-n`
            // through `-d` to `+n` (start cap).  Both go the same way round
            // because `n` is `d` rotated by +90°.
            let a0 = if at_end {
                n.1.atan2(n.0)
            } else {
                (-n.1).atan2(-n.0)
            };
            let mut v = Vec::new();
            push_arc(&mut v, center, a0, -std::f64::consts::PI, hw);
            v
        }
    }
}

/// Single ring for an open stroked polyline: side A forward, the end cap,
/// side B backwards, the start cap.
fn outline_open(pts: &[AnchorPoint], hw: f64, style: &StrokeStyle) -> Option<Vec<AnchorPoint>> {
    let n = pts.len();
    if n < 2 || hw <= 0.0 {
        return None;
    }
    let dirs = segment_dirs(pts, false);
    if dirs.is_empty() {
        return None;
    }
    let front = offset_polyline(pts, false, hw, style.join, style.miter_limit);
    let back = offset_polyline(pts, false, -hw, style.join, style.miter_limit);
    if front.len() < 2 || back.len() < 2 {
        return None;
    }

    let mut ring = Vec::with_capacity(front.len() + back.len() + 8);
    ring.extend(front.iter().copied());
    ring.extend(cap_points(
        pts[n - 1],
        normal(*dirs.last().unwrap()),
        *dirs.last().unwrap(),
        hw,
        style.cap,
        true,
    ));
    ring.extend(back.iter().rev().copied());
    ring.extend(cap_points(
        pts[0],
        normal(dirs[0]),
        dirs[0],
        hw,
        style.cap,
        false,
    ));
    Some(ring)
}

/// Split `pts` into the "on" runs of `pattern`.
///
/// Returns one `(polyline, closed)` pair per run.  With no usable pattern the
/// input comes back untouched, `closed` preserved.  Dashes are always open —
/// each one gets both caps, exactly like SVG.
fn dash_runs(
    pts: &[AnchorPoint],
    closed: bool,
    pattern: Option<&Vec<f64>>,
) -> Vec<(Vec<AnchorPoint>, bool)> {
    let passthrough = || vec![(pts.to_vec(), closed)];
    let Some(raw) = pattern else {
        return passthrough();
    };
    if raw.is_empty() {
        return passthrough();
    }
    // SVG repeats an odd-length list before concatenating it.
    let mut pat = raw.clone();
    if pat.len() % 2 == 1 {
        let dup = pat.clone();
        pat.extend(dup);
    }
    if !positive(pat.iter().sum::<f64>()) {
        return passthrough();
    }

    // Walk the closing segment too so a dashed closed path keeps its rhythm
    // across the seam.
    let mut trav = pts.to_vec();
    if closed {
        trav.push(pts[0]);
    }
    if trav.len() < 2 {
        return passthrough();
    }
    let mut cum = Vec::with_capacity(trav.len());
    cum.push(0.0_f64);
    for w in trav.windows(2) {
        let dx = w[1].x - w[0].x;
        let dy = w[1].y - w[0].y;
        cum.push(cum.last().unwrap() + (dx * dx + dy * dy).sqrt());
    }
    let total = *cum.last().unwrap();
    if !positive(total) {
        return passthrough();
    }

    let point_at = |s: f64| -> AnchorPoint {
        let s = s.clamp(0.0, total);
        let mut i = 0usize;
        while i + 1 < cum.len() - 1 && cum[i + 1] <= s {
            i += 1;
        }
        let seg_len = cum[i + 1] - cum[i];
        let t = if seg_len <= EPS {
            0.0
        } else {
            ((s - cum[i]) / seg_len).clamp(0.0, 1.0)
        };
        let a = trav[i];
        let b = trav[i + 1];
        AnchorPoint::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    };

    let mut runs: Vec<Vec<AnchorPoint>> = Vec::new();
    let mut cur: Vec<AnchorPoint> = Vec::new();
    let mut s = 0.0_f64;
    let mut pi = 0usize;
    // Index of the next path vertex at or after `s`; dash runs walk the path
    // monotonically so this never needs rewinding.
    let mut vi = 1usize;
    let mut guard = 0usize;
    while s < total && guard < 1_000_000 {
        guard += 1;
        // Skip zero-length entries so they can't flip the on/off parity.
        let mut skipped = 0usize;
        while pat[pi % pat.len()] <= EPS && skipped < pat.len() {
            pi += 1;
            skipped += 1;
        }
        if skipped >= pat.len() {
            break; // every entry is zero — handled by the caller's sum check
        }
        let idx = pi % pat.len();
        let on = idx.is_multiple_of(2);
        let take = pat[idx].min(total - s);
        let ns = s + take;
        if on {
            if cur.is_empty() {
                cur.push(point_at(s));
            }
            // Keep every path vertex the run passes through, otherwise a dash
            // that spans a corner would cut straight across it.  The run's own
            // endpoints come from `point_at`, so they're not duplicated here.
            while vi < trav.len() && cum[vi] <= ns + 1e-9 {
                if cum[vi] > s + 1e-9 && cum[vi] < ns - 1e-9 {
                    cur.push(trav[vi]);
                }
                vi += 1;
            }
            cur.push(point_at(ns));
        } else {
            while vi < trav.len() && cum[vi] <= s + 1e-9 {
                vi += 1;
            }
            if cur.len() >= 2 {
                runs.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
        s = ns;
        pi += 1;
    }
    let ended_on = !cur.is_empty();
    if ended_on {
        runs.push(cur);
    }

    // A closed path that finishes mid-dash has two runs meeting at the seam;
    // stitch them so the join doesn't get two sets of caps.
    if closed && ended_on && runs.len() > 1 {
        let first = runs.remove(0);
        if let Some(last) = runs.last_mut() {
            last.extend(first.iter().skip(1).copied());
        } else {
            runs.push(first);
        }
    }

    runs.retain(|r| r.len() >= 2);
    if runs.is_empty() {
        return passthrough();
    }
    runs.into_iter().map(|r| (r, false)).collect()
}

/// Rings that visually reproduce the stroke of one subpath, in document
/// coordinates.
///
/// * open subpath → one ring per dash run (side A, caps, side B)
/// * closed subpath with no dash → an outer ring plus an inner ring; fill the
///   pair with the even-odd rule so the stroke stays an annulus
///
/// Rings may be concave or self-overlapping; callers triangulate with
/// [`crate::core::path::PathData::to_triangles`].  An empty vector means
/// nothing to paint (degenerate input or zero width).
pub fn stroke_rings(
    pts: &[AnchorPoint],
    closed: bool,
    style: &StrokeStyle,
) -> Vec<Vec<AnchorPoint>> {
    if pts.len() < 2 || !positive(style.width) {
        return Vec::new();
    }
    // A subpath of doubled vertices has nothing to stroke.
    let extent = pts.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |a, p| (a.0.min(p.x), a.1.min(p.y), a.2.max(p.x), a.3.max(p.y)),
    );
    if extent.2 - extent.0 <= EPS && extent.3 - extent.1 <= EPS {
        return Vec::new();
    }
    let hw = style.width * 0.5;
    if !hw.is_finite() {
        return Vec::new();
    }

    let mut rings = Vec::new();
    for (run, run_closed) in dash_runs(pts, closed, style.dash_pattern.as_ref()) {
        if run.len() < 2 {
            continue;
        }
        if run_closed {
            // Which side ends up enclosing the other depends on the winding;
            // the fill rule decides that from containment, not from order.
            let side_a = offset_polyline(&run, true, hw, style.join, style.miter_limit);
            let side_b = offset_polyline(&run, true, -hw, style.join, style.miter_limit);
            if side_a.len() >= 3 {
                rings.push(side_a);
            }
            if side_b.len() >= 3 {
                rings.push(side_b);
            }
        } else if let Some(ring) = outline_open(&run, hw, style) {
            if ring.len() >= 3 {
                rings.push(ring);
            }
        }
    }
    rings
}

fn circle_pts(cx: f64, cy: f64, r: f64, n: usize) -> Vec<AnchorPoint> {
    (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * (i as f64) / (n as f64);
            AnchorPoint::new(cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

/// One arrowhead ring in *head-local* coordinates: the tip sits at the origin
/// and the head points along +X.
///
/// `width` is only used by [`ArrowHead::Arrow`], whose open chevron is a
/// stroked band rather than a solid polygon.
fn local_head(kind: ArrowHead, size: f64, width: f64) -> Option<Vec<AnchorPoint>> {
    if !positive(size) {
        return None;
    }
    let p = AnchorPoint::new;
    let ring = match kind {
        ArrowHead::None => return None,
        ArrowHead::Triangle => vec![p(0.0, 0.0), p(-size, size * 0.4), p(-size, -size * 0.4)],
        ArrowHead::Barbed => vec![
            p(0.0, 0.0),
            p(-size, size * 0.4),
            p(-size + size * 0.45, 0.0),
            p(-size, -size * 0.4),
        ],
        ArrowHead::Diamond => vec![
            p(0.0, 0.0),
            p(-size * 0.5, size * 0.3),
            p(-size, 0.0),
            p(-size * 0.5, -size * 0.3),
        ],
        ArrowHead::Square => {
            let s = size * 0.6;
            vec![
                p(-s, s * 0.5),
                p(0.0, s * 0.5),
                p(0.0, -s * 0.5),
                p(-s, -s * 0.5),
            ]
        }
        ArrowHead::Circle => circle_pts(0.0, 0.0, size * 0.32, 24),
        ArrowHead::Arrow => {
            // Open chevron: stroke the centreline so the head reads as two
            // wings instead of a solid wedge.
            let wing = size * 0.4;
            let half = ((width * 0.5).max(size * 0.06)).min(size * 0.5);
            let spine = vec![p(-size, wing), p(0.0, 0.0), p(-size, -wing)];
            let front = offset_polyline(&spine, false, half, StrokeJoin::Miter, 1e6);
            let back = offset_polyline(&spine, false, -half, StrokeJoin::Miter, 1e6);
            if front.len() < 2 || back.len() < 2 {
                return None;
            }
            let mut r = front;
            r.extend(back.iter().rev().copied());
            r
        }
    };
    if ring.len() < 3 {
        return None;
    }
    Some(ring)
}

/// A single arrowhead ring positioned in document coordinates: `tip` is the
/// path endpoint, `dir` the unit vector pointing away from the path (i.e.
/// where the head points).
pub fn arrowhead_ring(
    kind: ArrowHead,
    tip: AnchorPoint,
    dir: (f64, f64),
    size: f64,
    width: f64,
) -> Option<Vec<AnchorPoint>> {
    if !positive(size) {
        return None;
    }
    let d = unit(dir.0, dir.1);
    if d == (0.0, 0.0) {
        return None;
    }
    let mut ring = local_head(kind, size, width)?;
    for v in ring.iter_mut() {
        let (lx, ly) = (v.x, v.y);
        v.x = tip.x + lx * d.0 - ly * d.1;
        v.y = tip.y + lx * d.1 + ly * d.0;
    }
    Some(ring)
}

/// Arrowhead rings for a subpath: none, one, or (for an open path with heads
/// at both ends) two.  Closed paths and paths without arrowheads get none —
/// the same rule the SVG exporter follows.
pub fn arrowhead_rings(
    pts: &[AnchorPoint],
    closed: bool,
    style: &StrokeStyle,
) -> Vec<Vec<AnchorPoint>> {
    let n = pts.len();
    if closed || n < 2 || !positive(style.width) {
        return Vec::new();
    }
    let size = style.width * ARROW_SIZE_FACTOR;
    let mut out = Vec::new();

    if style.arrow_start != ArrowHead::None {
        let dir = unit(pts[0].x - pts[1].x, pts[0].y - pts[1].y);
        if let Some(r) = arrowhead_ring(style.arrow_start, pts[0], dir, size, style.width) {
            out.push(r);
        }
    }
    if style.arrow_end != ArrowHead::None {
        let dir = unit(pts[n - 1].x - pts[n - 2].x, pts[n - 1].y - pts[n - 2].y);
        if let Some(r) = arrowhead_ring(style.arrow_end, pts[n - 1], dir, size, style.width) {
            out.push(r);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::geometry::signed_polygon_area;

    fn line(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<AnchorPoint> {
        vec![AnchorPoint::new(x0, y0), AnchorPoint::new(x1, y1)]
    }

    fn style() -> StrokeStyle {
        StrokeStyle::default()
    }

    #[test]
    fn butt_cap_ring_is_the_plain_ribbon() {
        let rings = stroke_rings(&line(0.0, 0.0, 10.0, 0.0), false, &style());
        assert_eq!(rings.len(), 1);
        let r = &rings[0];
        assert_eq!(r.len(), 4, "butt cap adds no corner points");
        // Half width 0.5 either side of y = 0.
        let ys: Vec<f64> = r.iter().map(|p| p.y).collect();
        assert!(ys.iter().all(|y| (y.abs() - 0.5).abs() < 1e-9), "{ys:?}");
        assert!(signed_polygon_area(r).abs() > 0.0);
    }

    #[test]
    fn square_cap_extends_by_half_width() {
        let mut s = style();
        s.cap = StrokeCap::Square;
        let rings = stroke_rings(&line(0.0, 0.0, 10.0, 0.0), false, &s);
        assert_eq!(rings.len(), 1);
        let xs: Vec<f64> = rings[0].iter().map(|p| p.x).collect();
        assert!(xs.iter().any(|x| (*x - 10.5).abs() < 1e-9), "no +10.5 tip");
        assert!(xs.iter().any(|x| (*x + 0.5).abs() < 1e-9), "no -0.5 tail");
        assert_eq!(rings[0].len(), 8, "square cap adds two corners per end");
    }

    #[test]
    fn round_cap_adds_an_arc() {
        let mut s = style();
        s.cap = StrokeCap::Round;
        s.width = 4.0;
        let rings = stroke_rings(&line(0.0, 0.0, 10.0, 0.0), false, &s);
        assert_eq!(rings.len(), 1);
        assert!(rings[0].len() > 6, "arc should be subdivided");
        // Every point is within half width of the segment or its ends.
        for p in &rings[0] {
            let d_end = ((p.x - 10.0).powi(2) + p.y.powi(2)).sqrt();
            let d_start = (p.x.powi(2) + p.y.powi(2)).sqrt();
            assert!(
                d_end <= 2.0 + 1e-6 || d_start <= 2.0 + 1e-6 || (0.0..=10.0).contains(&p.x),
                "point {:?} escaped the round cap",
                (p.x, p.y)
            );
        }
    }

    #[test]
    fn dash_pattern_splits_the_run() {
        let mut s = style();
        s.dash_pattern = Some(vec![2.0, 2.0]);
        let rings = stroke_rings(&line(0.0, 0.0, 10.0, 0.0), false, &s);
        assert_eq!(rings.len(), 3, "2 on / 2 off over 10 units == three dashes");
        for r in &rings {
            let (mut lo, mut hi) = (f64::MAX, f64::MIN);
            for p in r {
                lo = lo.min(p.x);
                hi = hi.max(p.x);
            }
            assert!(
                hi - lo <= 2.0 + 1e-6,
                "dash is {} wide, expected <= 2",
                hi - lo
            );
        }
    }

    #[test]
    fn odd_dash_list_is_repeated_like_svg() {
        let mut s = style();
        s.dash_pattern = Some(vec![4.0]);
        let rings = stroke_rings(&line(0.0, 0.0, 16.0, 0.0), false, &s);
        assert_eq!(rings.len(), 2, "[4] repeats to [4, 4] == 4 on / 4 off");
    }

    #[test]
    fn dashed_closed_path_has_no_seam_gap() {
        let mut s = style();
        s.dash_pattern = Some(vec![10.0, 5.0]);
        let square = vec![
            AnchorPoint::new(0.0, 0.0),
            AnchorPoint::new(10.0, 0.0),
            AnchorPoint::new(10.0, 10.0),
            AnchorPoint::new(0.0, 10.0),
        ];
        let rings = stroke_rings(&square, true, &s);
        // Perimeter 40: on 0-10, off 10-15, on 15-25, off 25-30, on 30-40 —
        // the first and last runs meet at the seam and must be stitched into
        // one, leaving two dashes rather than three.
        assert_eq!(rings.len(), 2);
        // The stitched run crosses the seam: it reaches around the (0, 10)
        // corner (negative x) *and* along the bottom edge to (10, 0)
        // (y > 9), which neither unstitched run can do on its own.
        let crosses_seam = rings.iter().any(|r| {
            let neg_x = r.iter().any(|p| p.x < -1e-9);
            let top = r.iter().any(|p| p.y > 9.0);
            neg_x && top
        });
        assert!(crosses_seam, "seam run was not stitched: {rings:?}");
    }

    #[test]
    fn closed_undashed_path_yields_outer_and_inner_ring() {
        let square = vec![
            AnchorPoint::new(0.0, 0.0),
            AnchorPoint::new(10.0, 0.0),
            AnchorPoint::new(10.0, 10.0),
            AnchorPoint::new(0.0, 10.0),
        ];
        let rings = stroke_rings(&square, true, &style());
        assert_eq!(rings.len(), 2, "annulus needs an outer and an inner ring");
        // Which side comes first depends on the winding; the fill rule
        // resolves it by containment, so only the set of areas matters.
        let mut areas: Vec<f64> = rings.iter().map(|r| signed_polygon_area(r).abs()).collect();
        areas.sort_by(f64::total_cmp);
        assert!(
            (areas[0] - 81.0).abs() < 1e-6,
            "inner 9x9 == 81, got {}",
            areas[0]
        );
        assert!(
            (areas[1] - 121.0).abs() < 1e-6,
            "outer 11x11 == 121, got {}",
            areas[1]
        );
    }

    #[test]
    fn miter_limit_clamps_sharp_corners() {
        let mut s = style();
        s.width = 10.0;
        // A 12° spike: miter ratio ~ 1/sin(6°) ≈ 9.5.
        let spike = vec![
            AnchorPoint::new(0.0, 0.0),
            AnchorPoint::new(100.0, 10.0),
            AnchorPoint::new(0.0, 20.0),
        ];

        s.miter_limit = 100.0;
        let long = stroke_rings(&spike, false, &s);
        s.miter_limit = 4.0;
        let clamped = stroke_rings(&spike, false, &s);

        let extent = |rings: &[Vec<AnchorPoint>]| {
            rings.iter().flatten().map(|p| p.x).fold(f64::MIN, f64::max)
        };
        assert!(
            extent(&long) > extent(&clamped),
            "miter limit must truncate the spike: {:?} vs {:?}",
            extent(&long),
            extent(&clamped)
        );
    }

    #[test]
    fn round_join_adds_points_bevel_adds_two() {
        let corner = vec![
            AnchorPoint::new(0.0, 0.0),
            AnchorPoint::new(10.0, 0.0),
            AnchorPoint::new(10.0, 10.0),
        ];
        let mut s = style();
        s.join = StrokeJoin::Bevel;
        let bevel = stroke_rings(&corner, false, &s);
        s.join = StrokeJoin::Miter;
        let miter = stroke_rings(&corner, false, &s);
        s.join = StrokeJoin::Round;
        let round = stroke_rings(&corner, false, &s);

        assert!(
            bevel[0].len() > miter[0].len(),
            "bevel splits the corner vertex"
        );
        assert!(
            round[0].len() > bevel[0].len(),
            "round join arcs the corner"
        );
    }

    #[test]
    fn arrowheads_point_outward_at_both_ends() {
        let mut s = style();
        s.arrow_start = ArrowHead::Triangle;
        s.arrow_end = ArrowHead::Triangle;
        s.width = 10.0;
        let pts = line(0.0, 0.0, 50.0, 0.0);
        let heads = arrowhead_rings(&pts, false, &s);
        assert_eq!(heads.len(), 2);

        // `heads[0]` is the start head (pointing back along -X), `heads[1]`
        // the end head (pointing along +X).
        let start_tip = heads[0].iter().fold(f64::MAX, |m, p| p.x.min(m));
        let start_back = heads[0].iter().fold(f64::MIN, |m, p| p.x.max(m));
        let end_tip = heads[1].iter().fold(f64::MIN, |m, p| p.x.max(m));
        let end_back = heads[1].iter().fold(f64::MAX, |m, p| p.x.min(m));
        assert!(
            start_tip.abs() < 1e-9,
            "start head tip must sit at 0, got {start_tip}"
        );
        assert!(
            (end_tip - 50.0).abs() < 1e-9,
            "end head tip must sit at 50, got {end_tip}"
        );
        // Head length = width * 4.
        assert!(
            (start_back - 40.0).abs() < 1e-6,
            "start head length {start_back}"
        );
        assert!(
            (end_back - 10.0).abs() < 1e-6,
            "end head length {}",
            50.0 - end_back
        );
    }

    #[test]
    fn closed_paths_get_no_arrowheads() {
        let mut s = style();
        s.arrow_end = ArrowHead::Arrow;
        let square = vec![
            AnchorPoint::new(0.0, 0.0),
            AnchorPoint::new(10.0, 0.0),
            AnchorPoint::new(10.0, 10.0),
            AnchorPoint::new(0.0, 10.0),
        ];
        assert!(arrowhead_rings(&square, true, &s).is_empty());
    }

    #[test]
    fn arrow_head_shapes_are_simple_and_sized() {
        for kind in [
            ArrowHead::Triangle,
            ArrowHead::Arrow,
            ArrowHead::Circle,
            ArrowHead::Diamond,
            ArrowHead::Square,
            ArrowHead::Barbed,
        ] {
            let ring = arrowhead_ring(kind, AnchorPoint::new(0.0, 0.0), (1.0, 0.0), 20.0, 5.0)
                .unwrap_or_else(|| panic!("{kind:?} produced no ring"));
            assert!(ring.len() >= 3, "{kind:?} ring too small");
            let area = signed_polygon_area(&ring).abs();
            assert!(area > 0.5, "{kind:?} area {area} is degenerate");
        }
    }

    #[test]
    fn zero_width_and_degenerate_input_produce_nothing() {
        let mut s = style();
        s.width = 0.0;
        assert!(stroke_rings(&line(0.0, 0.0, 1.0, 1.0), false, &s).is_empty());
        assert!(stroke_rings(&[AnchorPoint::new(1.0, 1.0)], false, &style()).is_empty());
        assert!(stroke_rings(&line(5.0, 5.0, 5.0, 5.0), false, &style()).is_empty());
    }

    #[test]
    fn doubled_vertices_do_not_poison_the_outline() {
        let pts = vec![
            AnchorPoint::new(0.0, 0.0),
            AnchorPoint::new(10.0, 0.0),
            AnchorPoint::new(10.0, 0.0),
            AnchorPoint::new(10.0, 10.0),
        ];
        let rings = stroke_rings(&pts, false, &style());
        assert_eq!(rings.len(), 1);
        for p in &rings[0] {
            assert!(p.x.is_finite() && p.y.is_finite(), "NaN in outline");
        }
    }
}
