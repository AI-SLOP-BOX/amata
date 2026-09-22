use super::document::Object;
use super::path::{AnchorPoint, PathData};
use serde::{Deserialize, Serialize};

/// Replicate a motif object along a trajectory path with spacing and optional rotation follow
pub fn scatter_brush_along_path(
    trajectory: &PathData,
    motif: &Object,
    spacing: f64,
    jitter_scale: f64,
    follow_tangent: bool,
) -> Vec<Object> {
    let poly = trajectory.to_polygon(24);
    if poly.len() < 2 {
        return Vec::new();
    }

    let mut lengths = vec![0.0];
    let mut total_len = 0.0;
    for i in 0..poly.len() - 1 {
        let seg_len = poly[i].distance(poly[i + 1]);
        total_len += seg_len;
        lengths.push(total_len);
    }

    if total_len <= 1e-6 {
        return Vec::new();
    }

    let count = ((total_len / spacing.max(5.0)).floor() as usize).max(1);
    let mut clones = Vec::with_capacity(count);

    for i in 0..=count {
        let target_dist = (i as f64 * spacing).min(total_len);

        for j in 0..poly.len() - 1 {
            let d0 = lengths[j];
            let d1 = lengths[j + 1];
            if target_dist >= d0 && target_dist <= d1 {
                let seg_len = (d1 - d0).max(1e-6);
                let seg_t = (target_dist - d0) / seg_len;
                let p0 = poly[j];
                let p1 = poly[j + 1];

                let px = p0.x + seg_t * (p1.x - p0.x);
                let py = p0.y + seg_t * (p1.y - p0.y);

                let dx = p1.x - p0.x;
                let dy = p1.y - p0.y;
                let angle = if follow_tangent { dy.atan2(dx) } else { 0.0 };

                let hash = ((i as f64 * 37.891 + 13.77).sin() * 43758.5453)
                    .fract()
                    .abs();
                let scale_mod = 1.0 + (hash - 0.5) * jitter_scale;

                let mut clone = motif.clone();
                clone.id = uuid::Uuid::new_v4().to_string();
                clone.name = format!("{} (Brush {})", motif.name, i + 1);

                // Transform motif path to point (px, py)
                let mut path = clone.to_path_data();
                let cos = angle.cos() * scale_mod;
                let sin = angle.sin() * scale_mod;

                let transform_pt = |p: AnchorPoint| -> AnchorPoint {
                    AnchorPoint::new(px + p.x * cos - p.y * sin, py + p.x * sin + p.y * cos)
                };

                path.elements.iter_mut().for_each(|elem| match elem {
                    crate::core::path::PathElement::MoveTo(p)
                    | crate::core::path::PathElement::LineTo(p) => {
                        *p = transform_pt(*p);
                    }
                    crate::core::path::PathElement::CurveTo(seg) => {
                        seg.start = transform_pt(seg.start);
                        seg.control1 = transform_pt(seg.control1);
                        seg.control2 = transform_pt(seg.control2);
                        seg.end = transform_pt(seg.end);
                    }
                    _ => {}
                });

                clone.object_type = crate::core::document::ObjectType::Path(path);
                clones.push(clone);
                break;
            }
        }
    }

    clones
}

/// A brush definition. Applied destructively by [`apply_brush`]: the spine
/// path is replaced with outlined brush geometry (same precedent as Outline
/// Stroke — one undo step restores the spine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrushDefinition {
    pub name: String,
    pub kind: BrushKind,
}

/// Brush behavior variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BrushKind {
    /// Angled elliptical nib: width follows travel direction against the
    /// nib angle (thin along the major axis, full across it).
    Calligraphy {
        /// Nib angle in degrees (0 = flat horizontal nib).
        angle_deg: f64,
        /// Minor/major axis ratio 0..1 (1 = round nib = constant width).
        roundness: f64,
        /// Major-axis diameter in document units.
        size: f64,
    },
    /// Stretch vector artwork along the whole spine, scaled so the artwork
    /// height matches the path's stroke width.
    Art {
        artwork: PathData,
    },
    /// Tile vector artwork along the spine every `spacing` units.
    Pattern {
        artwork: PathData,
        spacing: f64,
        /// Uniform artwork scale.
        scale: f64,
    },
    /// Bristle brush: many dry streaks fanning around the spine (sumi /
    /// marker feel). Applied as a group of translucent ribbons.
    Bristle {
        /// Bristle count (1..=64).
        count: usize,
        /// Perpendicular scatter as a fraction of `size` (0..1).
        scatter: f64,
        /// Overall diameter in document units.
        size: f64,
        /// Peak streak opacity (0..1); individual streaks vary below it.
        opacity: f64,
    },
}

impl BrushDefinition {
    pub fn calligraphy(angle_deg: f64, roundness: f64, size: f64) -> Self {
        Self {
            name: "Calligraphy".to_string(),
            kind: BrushKind::Calligraphy {
                angle_deg,
                roundness: roundness.clamp(0.05, 1.0),
                size: size.max(0.5),
            },
        }
    }
}

/// Resample a polyline by arclength (plus tangents). Returns
/// `(points, tangents)` with `samples + 1` entries.
fn resample_spine(poly: &[AnchorPoint], samples: usize) -> (Vec<AnchorPoint>, Vec<(f64, f64)>) {
    let mut lengths = vec![0.0];
    let mut total = 0.0;
    for i in 0..poly.len().saturating_sub(1) {
        total += poly[i].distance(poly[i + 1]);
        lengths.push(total);
    }
    let samples = samples.max(2);
    let mut pts = Vec::with_capacity(samples + 1);
    let mut idx = 0;
    for s in 0..=samples {
        let target = total * s as f64 / samples as f64;
        while idx + 1 < lengths.len() && lengths[idx + 1] < target {
            idx += 1;
        }
        let j = idx.min(poly.len().saturating_sub(2));
        let d0 = lengths[j];
        let d1 = lengths[j + 1].max(d0 + 1e-9);
        let t = ((target - d0) / (d1 - d0)).clamp(0.0, 1.0);
        pts.push(AnchorPoint::new(
            poly[j].x + t * (poly[j + 1].x - poly[j].x),
            poly[j].y + t * (poly[j + 1].y - poly[j].y),
        ));
    }
    let mut tangents = Vec::with_capacity(pts.len());
    for i in 0..pts.len() {
        let a = pts[i.saturating_sub(1)];
        let b = pts[(i + 1).min(pts.len() - 1)];
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = (dx * dx + dy * dy).sqrt().max(1e-9);
        tangents.push((dx / len, dy / len));
    }
    (pts, tangents)
}

/// Frame (point + normal) at arclength `s` along a resampled spine.
fn frame_at(
    pts: &[AnchorPoint],
    tangents: &[(f64, f64)],
    total: f64,
    s: f64,
) -> (AnchorPoint, (f64, f64)) {
    let n = pts.len();
    if n < 2 || total <= 1e-9 {
        let p = pts.first().copied().unwrap_or(AnchorPoint::new(0.0, 0.0));
        return (p, (1.0, 0.0));
    }
    let t = (s / total).clamp(0.0, 1.0) * (n - 1) as f64;
    let i = (t.floor() as usize).min(n - 2);
    let f = t - i as f64;
    let p = AnchorPoint::new(
        pts[i].x + f * (pts[i + 1].x - pts[i].x),
        pts[i].y + f * (pts[i + 1].y - pts[i].y),
    );
    let (tx, ty) = tangents[i];
    (p, (-ty, tx))
}

/// Map artwork-space point (x along length, y across) onto the spine.
/// `x0/x1` bound the artwork's length axis; `y_center` is its across-axis
/// origin; `y_scale` scales across (stroke-width fit for art brushes).
fn map_point(
    x: f64,
    y: f64,
    x0: f64,
    x1: f64,
    y_center: f64,
    y_scale: f64,
    pts: &[AnchorPoint],
    tangents: &[(f64, f64)],
    total: f64,
    s_origin: f64,
    s_span: f64,
) -> AnchorPoint {
    let span = (x1 - x0).max(1e-9);
    let s = s_origin + ((x - x0) / span).clamp(0.0, 1.0) * s_span;
    let (p, (nx, ny)) = frame_at(pts, tangents, total, s);
    let off = (y - y_center) * y_scale;
    AnchorPoint::new(p.x + nx * off, p.y + ny * off)
}

fn map_artwork_onto_spine(
    artwork: &PathData,
    pts: &[AnchorPoint],
    tangents: &[(f64, f64)],
    total: f64,
    y_scale: f64,
    s_origin: f64,
    s_span: f64,
) -> PathData {
    use super::path::PathElement;
    let (bb_min, bb_max) = match artwork.bounding_box() {
        Some(v) => v,
        None => return PathData::new(),
    };
    let (x0, x1) = (bb_min.x, bb_max.x);
    let y_center = (bb_min.y + bb_max.y) / 2.0;
    let map = |x: f64, y: f64| {
        map_point(x, y, x0, x1, y_center, y_scale, pts, tangents, total, s_origin, s_span)
    };
    let mut out = PathData::new();
    out.fill = artwork.fill.clone();
    for el in &artwork.elements {
        match el {
            PathElement::MoveTo(p) => {
                let q = map(p.x, p.y);
                out.push_move_to(q.x, q.y);
            }
            PathElement::LineTo(p) => {
                let q = map(p.x, p.y);
                out.push_line_to(q.x, q.y);
            }
            PathElement::CurveTo(seg) => {
                // Control points ride the same frame field evaluated at
                // their own stations: keeps curves curved (approximate
                // under strong bending, exact on straight runs).
                let c1 = map(seg.control1.x, seg.control1.y);
                let c2 = map(seg.control2.x, seg.control2.y);
                let s = map(seg.start.x, seg.start.y);
                let e = map(seg.end.x, seg.end.y);
                out.elements.push(PathElement::CurveTo(
                    super::path::BezierSegment {
                        start: s,
                        control1: c1,
                        control2: c2,
                        end: e,
                    },
                ));
            }
            PathElement::ClosePath => out.elements.push(PathElement::ClosePath),
        }
    }
    out
}

/// Apply a brush to one spine polyline, returning outlined geometry.
/// `stroke_width` sizes art brushes (artwork height := stroke width).
/// Returns `None` when the spine is degenerate.
pub fn apply_brush_to_polyline(
    poly: &[AnchorPoint],
    closed: bool,
    def: &BrushDefinition,
    stroke_width: f64,
) -> Option<PathData> {
    if poly.len() < 2 {
        return None;
    }
    match &def.kind {
        BrushKind::Bristle { count, scatter, size, opacity } => {
            // Single-path fallback (per-streak alpha needs the multi
            // entry point `apply_bristle`): merge all streaks.
            let mut spine_only = PathData::new();
            for p in poly {
                if spine_only.elements.is_empty() {
                    spine_only.push_move_to(p.x, p.y);
                } else {
                    spine_only.push_line_to(p.x, p.y);
                }
            }
            let mut merged = PathData::new();
            for (band, _) in apply_bristle(&spine_only, *count, *scatter, *size, *opacity as f64) {
                merged.elements.extend(band.elements);
            }
            if merged.elements.is_empty() {
                return None;
            }
            merged.fill = Some(super::path::FillStyle::default());
            Some(merged)
        }
        BrushKind::Calligraphy {
            angle_deg,
            roundness,
            size,
        } => {
            // Direction-dependent nib width sampled into a WidthProfile,
            // reusing the tested variable-width ribbon machinery.
            let phi = angle_deg.to_radians();
            let (pts, tangents) = resample_spine(poly, 64);
            let mut profile = crate::core::document::WidthProfile { points: Vec::new() };
            for (i, (tx, ty)) in tangents.iter().enumerate() {
                let theta = ty.atan2(*tx);
                let d = theta - phi;
                // Full nib width across the travel direction (the ribbon
                // builder halves it internally).
                let w = size * (roundness * roundness * d.cos() * d.cos() + d.sin() * d.sin()).sqrt();
                profile.points.push(crate::core::document::WidthPoint {
                    position: i as f64 / 64.0,
                    width: w.max(0.05),
                    side: crate::core::document::WidthSide::Both,
                });
            }
            let band = crate::core::offset::variable_width_outline(&pts, &profile, 1.0, closed);
            if band.len() < 3 {
                return None;
            }
            let mut path = PathData::from_polygon_points(&band, true);
            path.fill = Some(super::path::FillStyle::default());
            Some(path)
        }
        BrushKind::Art { artwork } => {
            let (pts, tangents) = resample_spine(poly, 128);
            let total: f64 = {
                let mut t = 0.0;
                for i in 0..poly.len().saturating_sub(1) {
                    t += poly[i].distance(poly[i + 1]);
                }
                t
            };
            if total <= 1e-6 {
                return None;
            }
            let ah = artwork
                .bounding_box()
                .map(|(mn, mx)| (mx.y - mn.y).max(0.5))
                .unwrap_or(1.0);
            let mapped = map_artwork_onto_spine(
                artwork,
                &pts,
                &tangents,
                total,
                stroke_width.max(0.5) / ah,
                0.0,
                total,
            );
            if mapped.elements.is_empty() {
                return None;
            }
            Some(mapped)
        }
        BrushKind::Pattern {
            artwork,
            spacing,
            scale,
        } => {
            let (pts, tangents) = resample_spine(poly, 256);
            let total: f64 = {
                let mut t = 0.0;
                for i in 0..poly.len().saturating_sub(1) {
                    t += poly[i].distance(poly[i + 1]);
                }
                t
            };
            if total <= 1e-6 {
                return None;
            }
            let aw = artwork
                .bounding_box()
                .map(|(mn, mx)| (mx.x - mn.x).max(1.0))
                .unwrap_or(1.0)
                * scale.max(0.1);
            let step = spacing.max(aw);
            let mut out = PathData::new();
            out.fill = artwork.fill.clone();
            let mut s = 0.0;
            while s < total {
                // The artwork's own bbox maps onto [s, s + aw].
                let tile = map_artwork_onto_spine(
                    artwork, &pts, &tangents, total, scale.max(0.1), s, aw,
                );
                out.elements.extend(tile.elements);
                s += step;
            }
            if out.elements.is_empty() {
                return None;
            }
            Some(out)
        }
    }
}

/// Deterministic 0..1 hash (splitmix64 half) — stable bristles, no RNG dep.
fn hash01(n: u64) -> f64 {
    let mut x = n.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    ((x ^ (x >> 31)) as f64) / (u64::MAX as f64)
}

/// Apply a bristle brush: `count` dry streaks fanning around the spine.
/// Returns outlined ribbons with per-streak alpha (caller assigns color).
pub fn apply_bristle(
    spine: &PathData,
    count: usize,
    scatter: f64,
    size: f64,
    opacity: f64,
) -> Vec<(PathData, f32)> {
    let count = count.clamp(1, 64);
    let size = size.max(0.5);
    let scatter = scatter.clamp(0.0, 1.0);
    let opacity = (opacity as f64).clamp(0.0, 1.0);
    let mut out = Vec::new();
    for (run_idx, (flat, closed)) in spine_runs(spine).iter().enumerate() {
        if flat.len() < 2 {
            continue;
        }
        let (pts, tangents) = resample_spine(flat, 64);
        for k in 0..count {
            let seed = (run_idx as u64) * 131 + (k as u64) * 17 + 7;
            let off = (hash01(seed) - 0.5) * 2.0 * scatter * size;
            let w = size * (0.2 + 0.5 * hash01(seed ^ 0x5DEECE66D));
            let alpha = (opacity * (0.3 + 0.7 * hash01(seed ^ 0xB5297A4D))) as f32;
            let shifted: Vec<AnchorPoint> = pts
                .iter()
                .zip(tangents.iter())
                .map(|(p, (tx, ty))| AnchorPoint::new(p.x - ty * off, p.y + tx * off))
                .collect();
            let profile = crate::core::document::WidthProfile {
                points: vec![
                    crate::core::document::WidthPoint {
                        position: 0.0,
                        width: w.max(0.05),
                        side: crate::core::document::WidthSide::Both,
                    },
                    crate::core::document::WidthPoint {
                        position: 1.0,
                        width: w.max(0.05),
                        side: crate::core::document::WidthSide::Both,
                    },
                ],
            };
            let band = crate::core::offset::variable_width_outline(&shifted, &profile, 1.0, *closed);
            if band.len() < 3 {
                continue;
            }
            let mut path = PathData::from_polygon_points(&band, true);
            path.fill = Some(super::path::FillStyle::default());
            out.push((path, alpha));
        }
    }
    out
}

/// Apply a brush to a whole spine path (each subpath brushed, results
/// concatenated). `stroke_width` sizes art brushes.
pub fn apply_brush(spine: &PathData, def: &BrushDefinition, stroke_width: f64) -> Option<PathData> {
    let mut out = PathData::new();
    let mut any = false;
    for (flat, closed) in spine_runs(spine) {
        if let Some(mut band) = apply_brush_to_polyline(&flat, closed, def, stroke_width) {
            if out.fill.is_none() {
                out.fill = band.fill.clone();
            }
            out.elements.append(&mut band.elements);
            any = true;
        }
    }
    if any { Some(out) } else { None }
}

/// Split a spine into flattened runs, keeping 2-point runs (which
/// `to_subpaths` drops) and tracking closedness via `ClosePath`, the
/// path flag, or coincident end points.
fn spine_runs(spine: &PathData) -> Vec<(Vec<AnchorPoint>, bool)> {
    use super::path::PathElement;
    let mut runs: Vec<(Vec<AnchorPoint>, bool)> = Vec::new();
    let mut cur: Vec<AnchorPoint> = Vec::new();
    let mut cur_closed = false;
    let mut flush = |cur: &mut Vec<AnchorPoint>, closed: &mut bool| {
        if cur.len() >= 2 {
            if cur.len() > 2 && cur[0] == cur[cur.len() - 1] {
                cur.pop();
                *closed = true;
            }
            runs.push((std::mem::take(cur), *closed));
        }
        *closed = false;
    };
    for el in &spine.elements {
        match el {
            PathElement::MoveTo(p) => {
                flush(&mut cur, &mut cur_closed);
                cur.push(*p);
            }
            PathElement::LineTo(p) => cur.push(*p),
            PathElement::CurveTo(seg) => {
                for i in 1..=16 {
                    cur.push(seg.eval(i as f64 / 16.0));
                }
            }
            PathElement::ClosePath => cur_closed = true,
        }
    }
    if spine.closed {
        cur_closed = true;
    }
    flush(&mut cur, &mut cur_closed);
    runs
}

/// Built-in art/pattern motifs (drawn in a 0..100 box, y-down).
pub fn builtin_motif(name: &str) -> Option<PathData> {
    let mut path = PathData::new();
    match name {
        "arrow" => {
            path.push_move_to(0.0, 35.0);
            path.push_line_to(65.0, 35.0);
            path.push_line_to(65.0, 15.0);
            path.push_line_to(100.0, 50.0);
            path.push_line_to(65.0, 85.0);
            path.push_line_to(65.0, 65.0);
            path.push_line_to(0.0, 65.0);
            path.elements.push(super::path::PathElement::ClosePath);
        }
        "leaf" => {
            path.push_move_to(0.0, 50.0);
            path.push_line_to(50.0, 10.0);
            path.push_line_to(100.0, 50.0);
            path.push_line_to(50.0, 90.0);
            path.elements.push(super::path::PathElement::ClosePath);
            path.push_move_to(10.0, 50.0);
            path.push_line_to(90.0, 50.0);
        }
        "wave" => {
            path.push_move_to(0.0, 50.0);
            for i in 1..=8 {
                let x = i as f64 * 12.5;
                let y = 50.0 + if i % 2 == 1 { -30.0 } else { 30.0 };
                path.push_line_to(x, y);
            }
        }
        _ => return None,
    }
    path.fill = Some(super::path::FillStyle::default());
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hline() -> PathData {
        let mut p = PathData::new();
        p.push_move_to(0.0, 0.0);
        p.push_line_to(100.0, 0.0);
        p
    }

    fn vline() -> PathData {
        let mut p = PathData::new();
        p.push_move_to(0.0, 0.0);
        p.push_line_to(0.0, 100.0);
        p
    }

    fn band_span(path: &PathData) -> (f64, f64) {
        let (mn, mx) = path.bounding_box().expect("brushed band has bbox");
        (mx.x - mn.x, mx.y - mn.y)
    }

    #[test]
    fn calligraphy_thin_along_nib_thick_across() {
        // Flat nib (angle 0): horizontal travel rides the major axis
        // (thin), vertical travel crosses it (full size).
        let thin = BrushDefinition::calligraphy(0.0, 0.1, 20.0);
        let h = apply_brush(&hline(), &thin, 2.0).expect("band");
        let v = apply_brush(&vline(), &thin, 2.0).expect("band");
        let (_, hh) = band_span(&h);
        let (vw, _) = band_span(&v);
        assert!(hh < 4.0, "along-nib stays thin: {hh}");
        assert!((vw - 20.0).abs() < 3.0, "across-nib reaches size: {vw}");
    }

    #[test]
    fn calligraphy_round_nib_is_uniform() {
        let round = BrushDefinition::calligraphy(0.0, 1.0, 20.0);
        let h = apply_brush(&hline(), &round, 2.0).unwrap();
        let v = apply_brush(&vline(), &round, 2.0).unwrap();
        let (hmn, hmx) = h.bounding_box().unwrap();
        let (vmn, vmx) = v.bounding_box().unwrap();
        assert!((hmx.y - hmn.y - 20.0).abs() < 3.0, "round nib uniform H");
        assert!((vmx.x - vmn.x - 20.0).abs() < 3.0, "round nib uniform V");
    }

    #[test]
    fn art_brush_stretches_to_spine() {
        let art = BrushDefinition {
            name: "Arrow".into(),
            kind: BrushKind::Art {
                artwork: builtin_motif("arrow").unwrap(),
            },
        };
        let out = apply_brush(&hline(), &art, 10.0).expect("mapped");
        let (mn, mx) = out.bounding_box().expect("bbox");
        assert!((mx.x - mn.x - 100.0).abs() < 5.0, "full spine span");
        assert!((mx.y - mn.y - 10.0).abs() < 3.0, "height := stroke width");
    }

    #[test]
    fn pattern_brush_tiles() {
        let pat = BrushDefinition {
            name: "Leaf".into(),
            kind: BrushKind::Pattern {
                artwork: builtin_motif("leaf").unwrap(),
                spacing: 100.0,
                scale: 1.0,
            },
        };
        let out = apply_brush(&hline(), &pat, 2.0).expect("tiled");
        let moves = out
            .elements
            .iter()
            .filter(|e| matches!(e, super::super::path::PathElement::MoveTo(_)))
            .count();
        assert!(moves >= 2, "at least two tiles placed: {moves}");
    }

    #[test]
    fn degenerate_spine_returns_none() {
        let mut p = PathData::new();
        p.push_move_to(5.0, 5.0);
        assert!(apply_brush(&p, &BrushDefinition::calligraphy(0.0, 0.5, 10.0), 2.0).is_none());
    }

    #[test]
    fn bristle_yields_counted_streaks() {
        let mut spine = PathData::new();
        spine.push_move_to(0.0, 0.0);
        spine.push_line_to(100.0, 0.0);
        let streaks = apply_bristle(&spine, 8, 0.5, 10.0, 0.8);
        assert_eq!(streaks.len(), 8);
        for (band, alpha) in &streaks {
            assert!(!band.elements.is_empty());
            assert!(*alpha > 0.0 && *alpha <= 0.8, "alpha varies under peak: {alpha}");
        }
        // Deterministic: same input, same output.
        let again = apply_bristle(&spine, 8, 0.5, 10.0, 0.8);
        assert_eq!(streaks, again);
        // Scatter spreads ribbons around the spine.
        let ys: Vec<f64> = streaks
            .iter()
            .flat_map(|(b, _)| b.bounding_box())
            .flat_map(|(mn, mx)| [mn.y, mx.y])
            .collect();
        let span = ys.iter().cloned().fold(f64::MIN, f64::max)
            - ys.iter().cloned().fold(f64::MAX, f64::min);
        assert!(span > 2.0, "bristles fan out: {span}");
    }
}
