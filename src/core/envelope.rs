use super::document::Object;
use super::path::{AnchorPoint, PathElement};

pub fn apply_envelope_distort(art_obj: &Object, envelope_obj: &Object) -> Object {
    let (min_pt, max_pt) = art_obj
        .bounding_box()
        .unwrap_or((AnchorPoint::new(0.0, 0.0), AnchorPoint::new(100.0, 100.0)));

    let art_w = (max_pt.x - min_pt.x).max(1.0);
    let art_h = (max_pt.y - min_pt.y).max(1.0);

    let env_poly = envelope_obj.to_path_data().to_polygon(24);
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
            *p = warp_point(*p);
        }
        PathElement::CurveTo(seg) => {
            seg.start = warp_point(seg.start);
            seg.control1 = warp_point(seg.control1);
            seg.control2 = warp_point(seg.control2);
            seg.end = warp_point(seg.end);
        }
        _ => {}
    });

    clone.object_type = crate::core::document::ObjectType::Path(path);
    clone
}
