use super::document::Object;
use super::path::{AnchorPoint, PathElement};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolarMode {
    RectToPolar,
    PolarToRect,
}

pub fn apply_polar_transform(
    obj: &Object,
    cx: f64,
    cy: f64,
    width: f64,
    height: f64,
    mode: PolarMode,
) -> Object {
    let max_radius = (width.min(height) * 0.45).max(10.0);

    let transform_pt = |p: AnchorPoint| -> AnchorPoint {
        match mode {
            PolarMode::RectToPolar => {
                let u = (p.x / width).clamp(0.0, 1.0);
                let v = (p.y / height).clamp(0.0, 1.0);

                let theta = u * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
                let r = (1.0 - v) * max_radius;

                AnchorPoint::new(cx + theta.cos() * r, cy + theta.sin() * r)
            }
            PolarMode::PolarToRect => {
                let dx = p.x - cx;
                let dy = p.y - cy;
                let r = (dx * dx + dy * dy).sqrt().min(max_radius);
                let theta = (dy.atan2(dx) + std::f64::consts::FRAC_PI_2).rem_euclid(std::f64::consts::TAU);

                let u = theta / std::f64::consts::TAU;
                let v = 1.0 - (r / max_radius);

                AnchorPoint::new(u * width, v * height)
            }
        }
    };

    let mut clone = obj.clone();
    clone.id = uuid::Uuid::new_v4().to_string();
    clone.name = format!("{} (Polar)", obj.name);

    let mut path = clone.to_path_data();
    path.elements.iter_mut().for_each(|elem| match elem {
        PathElement::MoveTo(p) | PathElement::LineTo(p) => {
            *p = transform_pt(*p);
        }
        PathElement::CurveTo(seg) => {
            seg.start = transform_pt(seg.start);
            seg.control1 = transform_pt(seg.control1);
            seg.control2 = transform_pt(seg.control2);
            seg.end = transform_pt(seg.end);
        }
        _ => {}
    });

    clone.object_type = crate::core::document::ObjectType::Path(path);
    clone
}
