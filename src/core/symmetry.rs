use super::document::Object;
use super::path::AnchorPoint;
use std::f64::consts::TAU;

pub fn create_radial_symmetry(
    obj: &Object,
    cx: f64,
    cy: f64,
    folds: usize,
    mirror: bool,
) -> Vec<Object> {
    let folds = folds.max(2);
    let angle_step = TAU / folds as f64;
    let mut clones = Vec::with_capacity(folds * if mirror { 2 } else { 1 });

    for i in 0..folds {
        let angle = i as f64 * angle_step;
        let mut cloned = obj.clone();
        cloned.id = uuid::Uuid::new_v4().to_string();
        cloned.name = format!("{} (Sym {})", obj.name, i + 1);

        let mut path = cloned.to_path_data();
        // Transform around (cx, cy)
        path.elements.iter_mut().for_each(|elem| match elem {
            crate::core::path::PathElement::MoveTo(p)
            | crate::core::path::PathElement::LineTo(p) => {
                *p = rotate_point(*p, cx, cy, angle);
            }
            crate::core::path::PathElement::CurveTo(seg) => {
                seg.start = rotate_point(seg.start, cx, cy, angle);
                seg.control1 = rotate_point(seg.control1, cx, cy, angle);
                seg.control2 = rotate_point(seg.control2, cx, cy, angle);
                seg.end = rotate_point(seg.end, cx, cy, angle);
            }
            _ => {}
        });

        cloned.object_type = crate::core::document::ObjectType::Path(path);
        clones.push(cloned);

        if mirror {
            let mut mirrored = obj.clone();
            mirrored.id = uuid::Uuid::new_v4().to_string();
            mirrored.name = format!("{} (Sym Mir {})", obj.name, i + 1);

            let mut mpath = mirrored.to_path_data();
            mpath.elements.iter_mut().for_each(|elem| match elem {
                crate::core::path::PathElement::MoveTo(p)
                | crate::core::path::PathElement::LineTo(p) => {
                    let mp = AnchorPoint::new(cx - (p.x - cx), p.y);
                    *p = rotate_point(mp, cx, cy, angle);
                }
                crate::core::path::PathElement::CurveTo(seg) => {
                    let ms = AnchorPoint::new(cx - (seg.start.x - cx), seg.start.y);
                    let mc1 = AnchorPoint::new(cx - (seg.control1.x - cx), seg.control1.y);
                    let mc2 = AnchorPoint::new(cx - (seg.control2.x - cx), seg.control2.y);
                    let me = AnchorPoint::new(cx - (seg.end.x - cx), seg.end.y);
                    seg.start = rotate_point(ms, cx, cy, angle);
                    seg.control1 = rotate_point(mc1, cx, cy, angle);
                    seg.control2 = rotate_point(mc2, cx, cy, angle);
                    seg.end = rotate_point(me, cx, cy, angle);
                }
                _ => {}
            });

            mirrored.object_type = crate::core::document::ObjectType::Path(mpath);
            clones.push(mirrored);
        }
    }

    clones
}

fn rotate_point(p: AnchorPoint, cx: f64, cy: f64, angle: f64) -> AnchorPoint {
    let cos = angle.cos();
    let sin = angle.sin();
    let dx = p.x - cx;
    let dy = p.y - cy;
    AnchorPoint::new(cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
}
