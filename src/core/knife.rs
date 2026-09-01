use super::document::Object;
use super::path::{AnchorPoint, PathData};

/// Slice an Object's polygon into two distinct vector parts using a straight cutting line
pub fn slice_object_with_line(
    obj: &Object,
    p1: AnchorPoint,
    p2: AnchorPoint,
) -> Option<(Object, Object)> {
    let poly = obj.to_path_data().to_polygon(24);
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

    obj_left.transform = obj.transform.clone();
    obj_right.transform = obj.transform.clone();

    Some((obj_left, obj_right))
}
