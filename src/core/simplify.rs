use super::path::{AnchorPoint, PathData};

/// Visvalingam-Whyatt line simplification based on minimum effective triangle area
pub fn simplify_polygon_visvalingam(pts: &[AnchorPoint], tolerance_area: f64) -> Vec<AnchorPoint> {
    if pts.len() <= 3 {
        return pts.to_vec();
    }

    let mut current = pts.to_vec();

    loop {
        if current.len() <= 3 {
            break;
        }

        let mut min_area = f64::INFINITY;
        let mut min_idx = None;

        for i in 1..current.len() - 1 {
            let p0 = current[i - 1];
            let p1 = current[i];
            let p2 = current[i + 1];

            // Triangle area = 0.5 * |x0(y1 - y2) + x1(y2 - y0) + x2(y0 - y1)|
            let area = (p0.x * (p1.y - p2.y) + p1.x * (p2.y - p0.y) + p2.x * (p0.y - p1.y)).abs() * 0.5;

            if area < min_area {
                min_area = area;
                min_idx = Some(i);
            }
        }

        if min_area < tolerance_area {
            if let Some(idx) = min_idx {
                current.remove(idx);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    current
}

/// Simplify a PathData using Visvalingam-Whyatt effective area reduction
pub fn simplify_path_visvalingam(path: &PathData, tolerance_area: f64) -> PathData {
    let poly = path.to_polygon(1);
    let simplified_pts = simplify_polygon_visvalingam(&poly, tolerance_area);
    let mut new_path = PathData::from_polygon_points(&simplified_pts, path.closed);
    new_path.fill = path.fill.clone();
    new_path.stroke = path.stroke.clone();
    new_path
}
