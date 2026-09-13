use super::geometry::signed_polygon_area;
use super::path::{AnchorPoint, FillStyle, PathData};

/// Offset a closed polygon outward (delta > 0) or inward (delta < 0)
pub fn offset_polygon(poly: &[AnchorPoint], delta: f64) -> Vec<AnchorPoint> {
    let n = poly.len();
    if n < 3 || delta.abs() < 1e-4 {
        return poly.to_vec();
    }

    let is_ccw = signed_polygon_area(poly) >= 0.0;
    let sign = if is_ccw { 1.0 } else { -1.0 };

    let mut normals = Vec::with_capacity(n);
    for i in 0..n {
        let p1 = poly[i];
        let p2 = poly[(i + 1) % n];
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        // Outward normal: (dy, -dx) for CCW
        normals.push(AnchorPoint::new(sign * dy / len, sign * -dx / len));
    }

    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let prev_idx = if i == 0 { n - 1 } else { i - 1 };
        let n1 = normals[prev_idx];
        let n2 = normals[i];

        let p = poly[i];
        // Bisector vector
        let bx = (n1.x + n2.x) * 0.5;
        let by = (n1.y + n2.y) * 0.5;
        let blen = (bx * bx + by * by).sqrt().max(1e-6);

        // Miter factor
        let cos_half = (n1.x * n2.x + n1.y * n2.y).clamp(-1.0, 1.0);
        let miter_len = if cos_half > -0.99 {
            (delta / (1.0_f64 + cos_half).sqrt().max(0.1))
                .clamp(-5.0 * delta.abs(), 5.0 * delta.abs())
        } else {
            delta
        };

        result.push(AnchorPoint::new(
            p.x + (bx / blen) * miter_len,
            p.y + (by / blen) * miter_len,
        ));
    }

    result
}

/// Apply offset path to a PathData
pub fn offset_path(path: &PathData, delta: f64) -> PathData {
    let poly = path.to_polygon(16);
    let offset_pts = offset_polygon(&poly, delta);
    let mut new_path = PathData::from_polygon_points(&offset_pts, true);
    new_path.fill = path.fill.clone();
    new_path.stroke = path.stroke.clone();
    new_path
}

/// Outline stroke: Converts an open or closed stroked path into a filled ribbon polygon
pub fn outline_stroke(path: &PathData, stroke_width: f64) -> PathData {
    let poly = path.to_polygon(16);
    let n = poly.len();
    if n < 2 {
        return path.clone();
    }

    let half_w = (stroke_width * 0.5).max(0.5);

    if path.closed && n >= 3 {
        let outer = offset_polygon(&poly, half_w);
        let inner = offset_polygon(&poly, -half_w);
        let mut combined = PathData::new();

        let mut outer_path = PathData::from_polygon_points(&outer, true);
        let mut inner_path = PathData::from_polygon_points(&inner, true);
        combined.elements.append(&mut outer_path.elements);
        combined.elements.append(&mut inner_path.elements);
        combined.fill = path
            .stroke
            .as_ref()
            .map(|s| FillStyle::solid(s.color))
            .or_else(|| path.fill.clone());
        combined.stroke = None;
        combined
    } else {
        // Open stroke ribbon
        let mut left_side = Vec::new();
        let mut right_side = Vec::new();

        for i in 0..n {
            let (dx, dy) = if i == 0 {
                (poly[1].x - poly[0].x, poly[1].y - poly[0].y)
            } else if i == n - 1 {
                (poly[n - 1].x - poly[n - 2].x, poly[n - 1].y - poly[n - 2].y)
            } else {
                (poly[i + 1].x - poly[i - 1].x, poly[i + 1].y - poly[i - 1].y)
            };

            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let nx = -dy / len;
            let ny = dx / len;

            let p = poly[i];
            left_side.push(AnchorPoint::new(p.x + nx * half_w, p.y + ny * half_w));
            right_side.push(AnchorPoint::new(p.x - nx * half_w, p.y - ny * half_w));
        }

        right_side.reverse();
        left_side.extend(right_side);

        let mut out_path = PathData::from_polygon_points(&left_side, true);
        out_path.fill = path
            .stroke
            .as_ref()
            .map(|s| FillStyle::solid(s.color))
            .or_else(|| path.fill.clone());
        out_path.stroke = None;
        out_path
    }
}
