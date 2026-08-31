use super::document::Object;
use super::path::{AnchorPoint, PathData};

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

                let hash = ((i as f64 * 37.891 + 13.77).sin() * 43758.5453).fract().abs();
                let scale_mod = 1.0 + (hash - 0.5) * jitter_scale;

                let mut clone = motif.clone();
                clone.id = uuid::Uuid::new_v4().to_string();
                clone.name = format!("{} (Brush {})", motif.name, i + 1);

                // Transform motif path to point (px, py)
                let mut path = clone.to_path_data();
                let cos = angle.cos() * scale_mod;
                let sin = angle.sin() * scale_mod;

                let transform_pt = |p: AnchorPoint| -> AnchorPoint {
                    AnchorPoint::new(
                        px + p.x * cos - p.y * sin,
                        py + p.x * sin + p.y * cos,
                    )
                };

                path.elements.iter_mut().for_each(|elem| match elem {
                    crate::core::path::PathElement::MoveTo(p) | crate::core::path::PathElement::LineTo(p) => {
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
