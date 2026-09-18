use super::path::{AnchorPoint, PathData, PathElement};

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
            let area =
                (p0.x * (p1.y - p2.y) + p1.x * (p2.y - p0.y) + p2.x * (p0.y - p1.y)).abs() * 0.5;

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

/// Simplify a PathData using Visvalingam-Whyatt effective area reduction.
///
/// Only runs of straight LineTo segments are reduced; bezier CurveTo spans
/// are passed through untouched. (The previous implementation flattened the
/// whole path with `to_polygon` and rebuilt it as polylines, silently turning
/// every curve into straight segments.)
pub fn simplify_path_visvalingam(path: &PathData, tolerance_area: f64) -> PathData {
    let mut new_path = PathData::new();
    // Current LineTo run, including its start anchor.
    let mut run: Vec<AnchorPoint> = Vec::new();

    let flush_run = |run: &mut Vec<AnchorPoint>, out: &mut PathData| {
        if run.is_empty() {
            return;
        }
        if run.len() <= 2 {
            for p in run.iter().skip(1) {
                out.push_line_to(p.x, p.y);
            }
        } else {
            let simplified = simplify_polygon_visvalingam(run, tolerance_area);
            // The run start anchor is already the cursor; emit the rest.
            for p in simplified.iter().skip(1) {
                out.push_line_to(p.x, p.y);
            }
        }
        run.clear();
    };

    for elem in &path.elements {
        match elem {
            PathElement::MoveTo(p) => {
                flush_run(&mut run, &mut new_path);
                new_path.push_move_to(p.x, p.y);
                run.push(*p);
            }
            PathElement::LineTo(p) => {
                if run.is_empty() {
                    // No preceding MoveTo in this subpath; treat as implicit start.
                    run.push(*p);
                } else {
                    run.push(*p);
                }
            }
            PathElement::CurveTo(seg) => {
                flush_run(&mut run, &mut new_path);
                new_path.push_cubic_curve_to(
                    seg.control1.x,
                    seg.control1.y,
                    seg.control2.x,
                    seg.control2.y,
                    seg.end.x,
                    seg.end.y,
                );
                run.push(seg.end);
            }
            PathElement::ClosePath => {
                flush_run(&mut run, &mut new_path);
                new_path.close();
                run.clear();
            }
        }
    }
    flush_run(&mut run, &mut new_path);

    new_path.closed = path.closed;
    new_path.fill = path.fill.clone();
    new_path.stroke = path.stroke.clone();
    new_path
}
