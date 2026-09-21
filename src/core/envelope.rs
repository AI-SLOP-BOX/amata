use super::document::Object;
use super::path::{AnchorPoint, PathElement};
use serde::{Deserialize, Serialize};

/// Live (non-destructive) envelope kinds. The point math mirrors the
/// destructive lattice presets so both agree; `amount` is -1..=1-ish
/// (clamped at apply time, not stored).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvelopeKind {
    Bulge,
    Pinch,
    Twist,
    Wave,
}

impl EnvelopeKind {
    pub fn name(&self) -> &'static str {
        match self {
            EnvelopeKind::Bulge => "Bulge",
            EnvelopeKind::Pinch => "Pinch",
            EnvelopeKind::Twist => "Twist",
            EnvelopeKind::Wave => "Wave",
        }
    }
}

/// Map one local point through the deform, normalized by `bb`
/// (the source's local bbox). Twist/Wave follow the lattice presets;
/// Bulge/Pinch use a cushion field so silhouettes bow (a pure radial
/// falloff pins borders and would leave filled shapes unchanged).
pub fn envelope_point(
    p: AnchorPoint,
    bb: (AnchorPoint, AnchorPoint),
    kind: EnvelopeKind,
    amount: f64,
) -> AnchorPoint {
    let (mn, mx) = bb;
    let w = (mx.x - mn.x).max(1e-6);
    let h = (mx.y - mn.y).max(1e-6);
    let cx = (mn.x + mx.x) / 2.0;
    let cy = (mn.y + mx.y) / 2.0;
    let dx = (p.x - cx) / (w * 0.5);
    let dy = (p.y - cy) / (h * 0.5);
    let dist = (dx * dx + dy * dy).sqrt().min(1.0);
    match kind {
        // Cushion bulge/pinch: edges bow (max at edge midpoints, corners
        // anchored). Pinch is the same field with negated amount. Unlike
        // the radial lattice falloff, this moves silhouettes, not just
        // interiors.
        EnvelopeKind::Bulge | EnvelopeKind::Pinch => {
            let signed = if kind == EnvelopeKind::Bulge { amount } else { -amount };
            let k = signed * 0.5;
            let nx = dx * (1.0 + k * (1.0 - dy * dy));
            let ny = dy * (1.0 + k * (1.0 - dx * dx));
            AnchorPoint::new(cx + nx * w * 0.5, cy + ny * h * 0.5)
        }
        EnvelopeKind::Twist => {
            let a = (1.0 - dist) * amount * 1.5;
            let nx = dx * a.cos() - dy * a.sin();
            let ny = dx * a.sin() + dy * a.cos();
            AnchorPoint::new(cx + nx * w * 0.5, cy + ny * h * 0.5)
        }
        EnvelopeKind::Wave => {
            let off = (dx * std::f64::consts::PI).sin() * h * 0.2 * amount;
            AnchorPoint::new(p.x, p.y + off)
        }
    }
}

pub fn apply_envelope_distort(art_obj: &Object, envelope_obj: &Object) -> Object {
    let (min_pt, max_pt) = art_obj
        .bounding_box()
        .unwrap_or((AnchorPoint::new(0.0, 0.0), AnchorPoint::new(100.0, 100.0)));

    let art_w = (max_pt.x - min_pt.x).max(1.0);
    let art_h = (max_pt.y - min_pt.y).max(1.0);

    // bounding_box() is world-space, so both the artwork and the envelope
    // must be lifted through their transforms first. (Previously local art
    // coordinates were warped with world-space parameters and the result kept
    // the original transform, applying it twice.)
    let art_m = art_obj.transform.matrix();
    let lift = |p: AnchorPoint| -> AnchorPoint {
        AnchorPoint::new(
            art_m[0] * p.x + art_m[2] * p.y + art_m[4],
            art_m[1] * p.x + art_m[3] * p.y + art_m[5],
        )
    };
    let env_m = envelope_obj.transform.matrix();
    let env_poly: Vec<AnchorPoint> = envelope_obj
        .to_path_data()
        .to_polygon(24)
        .iter()
        .map(|p| {
            AnchorPoint::new(
                env_m[0] * p.x + env_m[2] * p.y + env_m[4],
                env_m[1] * p.x + env_m[3] * p.y + env_m[5],
            )
        })
        .collect();
    if env_poly.len() < 4 {
        return art_obj.clone();
    }

    // Pick 4 corner control points from envelope polygon
    let n = env_poly.len();
    let p00 = env_poly[0];
    let p10 = env_poly[n / 4];
    let p11 = env_poly[n / 2];
    let p01 = env_poly[(3 * n) / 4];

    let warp_point = |p: AnchorPoint| -> AnchorPoint {
        let u = ((p.x - min_pt.x) / art_w).clamp(0.0, 1.0);
        let v = ((p.y - min_pt.y) / art_h).clamp(0.0, 1.0);

        // Bilinear quadrilateral interpolation
        let top_x = p00.x + u * (p10.x - p00.x);
        let top_y = p00.y + u * (p10.y - p00.y);
        let bot_x = p01.x + u * (p11.x - p01.x);
        let bot_y = p01.y + u * (p11.y - p01.y);

        let out_x = top_x + v * (bot_x - top_x);
        let out_y = top_y + v * (bot_y - top_y);

        AnchorPoint::new(out_x, out_y)
    };

    let mut clone = art_obj.clone();
    clone.id = uuid::Uuid::new_v4().to_string();
    clone.name = format!("{} (Enveloped)", art_obj.name);

    let mut path = clone.to_path_data();
    path.elements.iter_mut().for_each(|elem| match elem {
        PathElement::MoveTo(p) | PathElement::LineTo(p) => {
            *p = warp_point(lift(*p));
        }
        PathElement::CurveTo(seg) => {
            seg.start = warp_point(lift(seg.start));
            seg.control1 = warp_point(lift(seg.control1));
            seg.control2 = warp_point(lift(seg.control2));
            seg.end = warp_point(lift(seg.end));
        }
        _ => {}
    });

    // Warped points are world-space; reset to identity so the old transform
    // is not applied a second time.
    clone.object_type = crate::core::document::ObjectType::Path(path);
    clone.transform = crate::core::document::Transform::default();
    clone
}

/// Subdivide a path for live envelopes: straight edges gain midpoints and
/// beziers split via de Casteljau (shape-exact, so release is lossless).
/// Without this, sparse geometry (e.g. a 4-corner rect) has no interior
/// points to bend and warps would be invisible.
pub fn subdivide_path(path: &super::path::PathData, iterations: usize) -> super::path::PathData {
    use super::path::{AnchorPoint, BezierSegment, PathElement};
    let mid = |a: AnchorPoint, b: AnchorPoint| AnchorPoint::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let mut current = path.clone();
    for _ in 0..iterations.min(4) {
        let mut out = super::path::PathData::new();
        out.fill = current.fill.clone();
        out.stroke = current.stroke.clone();
        let mut pen = AnchorPoint::new(0.0, 0.0);
        let mut have_pen = false;
        for el in &current.elements {
            match el {
                PathElement::MoveTo(p) => {
                    out.push_move_to(p.x, p.y);
                    pen = *p;
                    have_pen = true;
                }
                PathElement::LineTo(p) => {
                    if have_pen {
                        let m = mid(pen, *p);
                        out.push_line_to(m.x, m.y);
                    }
                    out.push_line_to(p.x, p.y);
                    pen = *p;
                    have_pen = true;
                }
                PathElement::CurveTo(seg) => {
                    // de Casteljau at t=0.5.
                    let a = mid(seg.start, seg.control1);
                    let b = mid(seg.control1, seg.control2);
                    let c = mid(seg.control2, seg.end);
                    let d = mid(a, b);
                    let e = mid(b, c);
                    let f = mid(d, e);
                    out.elements.push(PathElement::CurveTo(BezierSegment {
                        start: seg.start,
                        control1: a,
                        control2: d,
                        end: f,
                    }));
                    out.elements.push(PathElement::CurveTo(BezierSegment {
                        start: f,
                        control1: e,
                        control2: c,
                        end: seg.end,
                    }));
                    pen = seg.end;
                    have_pen = true;
                }
                PathElement::ClosePath => out.elements.push(PathElement::ClosePath),
            }
        }
        current = out;
    }
    current
}
