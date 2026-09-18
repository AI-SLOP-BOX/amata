use super::document::Object;
use super::path::{AnchorPoint, PathData};

fn apply_matrix_pt(p: AnchorPoint, m: &[f64; 6]) -> AnchorPoint {
    AnchorPoint::new(
        m[0] * p.x + m[2] * p.y + m[4],
        m[1] * p.x + m[3] * p.y + m[5],
    )
}

/// Slice an Object's polygon into two distinct vector parts using a straight cutting line.
///
/// Both the polygon and the cutting line live in world coordinates (callers
/// derive the line from `bounding_box()`, which is world-space), so the local
/// path is lifted through the object transform first. Results carry world
/// coordinates with an identity transform.
pub fn slice_object_with_line(
    obj: &Object,
    p1: AnchorPoint,
    p2: AnchorPoint,
) -> Option<(Object, Object)> {
    let m = obj.transform.matrix();
    let poly: Vec<AnchorPoint> = obj
        .to_path_data()
        .to_polygon(24)
        .iter()
        .map(|p| apply_matrix_pt(*p, &m))
        .collect();
    if poly.len() < 3 {
        return None;
    }

    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return None;
    }

    let nx = -dy / len;
    let ny = dx / len;

    // Split into Left and Right polygons using halfplane clipping
    let mut left_pts = Vec::new();
    let mut right_pts = Vec::new();

    let n = poly.len();
    for i in 0..n {
        let pt1 = poly[i];
        let pt2 = poly[(i + 1) % n];

        let d1 = (pt1.x - p1.x) * nx + (pt1.y - p1.y) * ny;
        let d2 = (pt2.x - p1.x) * nx + (pt2.y - p1.y) * ny;

        if d1 >= 0.0 {
            left_pts.push(pt1);
        } else {
            right_pts.push(pt1);
        }

        if (d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0) {
            // Intersection point
            let t = d1 / (d1 - d2);
            let ix = pt1.x + t * (pt2.x - pt1.x);
            let iy = pt1.y + t * (pt2.y - pt1.y);
            let ip = AnchorPoint::new(ix, iy);

            left_pts.push(ip);
            right_pts.push(ip);
        }
    }

    if left_pts.len() < 3 || right_pts.len() < 3 {
        return None;
    }

    let mut path_left = PathData::from_polygon_points(&left_pts, true);
    path_left.fill = obj.to_path_data().fill.clone();
    path_left.stroke = obj.to_path_data().stroke.clone();

    let mut path_right = PathData::from_polygon_points(&right_pts, true);
    path_right.fill = obj.to_path_data().fill.clone();
    path_right.stroke = obj.to_path_data().stroke.clone();

    let mut obj_left = Object::new_path(&format!("{} (Slice A)", obj.name), path_left);
    let mut obj_right = Object::new_path(&format!("{} (Slice B)", obj.name), path_right);

    // Paths above are world-space; keep identity transforms so no double
    // transform applies. Preserve visual attributes of the source.
    for part in [&mut obj_left, &mut obj_right] {
        part.opacity = obj.opacity;
        part.blend_mode = obj.blend_mode;
        part.visible = obj.visible;
    }

    Some((obj_left, obj_right))
}
