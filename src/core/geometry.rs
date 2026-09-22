use super::path::AnchorPoint;

pub fn point_in_polygon(px: f64, py: f64, verts: &[AnchorPoint]) -> bool {
    let n = verts.len();
    if n < 3 {
        return false;
    }

    for i in 0..n {
        let j = (i + 1) % n;
        let x1 = verts[i].x;
        let y1 = verts[i].y;
        let x2 = verts[j].x;
        let y2 = verts[j].y;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len_sq = dx * dx + dy * dy;
        if len_sq > 1e-6 {
            let t = (((px - x1) * dx + (py - y1) * dy) / len_sq).clamp(0.0, 1.0);
            let proj_x = x1 + t * dx;
            let proj_y = y1 + t * dy;
            let dist_sq = (px - proj_x).powi(2) + (py - proj_y).powi(2);
            if dist_sq < 1e-4 {
                return true;
            }
        }
    }

    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let xi = verts[i].x;
        let yi = verts[i].y;
        let xj = verts[j].x;
        let yj = verts[j].y;
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Check if point is inside multi-contour/hole-bearing subpaths using the specified fill rule (EvenOdd or NonZero)
pub fn point_in_subpaths(px: f64, py: f64, subpaths: &[Vec<AnchorPoint>], even_odd: bool) -> bool {
    if even_odd {
        let mut inside = false;
        for poly in subpaths {
            if poly.len() < 3 {
                continue;
            }
            let n = poly.len();
            let mut j = n - 1;
            for i in 0..n {
                let xi = poly[i].x;
                let yi = poly[i].y;
                let xj = poly[j].x;
                let yj = poly[j].y;
                if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
                    inside = !inside;
                }
                j = i;
            }
        }
        inside
    } else {
        let mut winding = 0;
        for poly in subpaths {
            if poly.len() < 3 {
                continue;
            }
            let n = poly.len();
            for i in 0..n {
                let p1 = poly[i];
                let p2 = poly[(i + 1) % n];
                if p1.y <= py {
                    if p2.y > py {
                        let cross = (p2.x - p1.x) * (py - p1.y) - (px - p1.x) * (p2.y - p1.y);
                        if cross > 0.0 {
                            winding += 1;
                        }
                    }
                } else if p2.y <= py {
                    let cross = (p2.x - p1.x) * (py - p1.y) - (px - p1.x) * (p2.y - p1.y);
                    if cross < 0.0 {
                        winding -= 1;
                    }
                }
            }
        }
        winding != 0
    }
}

pub fn signed_polygon_area(verts: &[AnchorPoint]) -> f64 {
    let mut area = 0.0;
    let n = verts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += verts[i].x * verts[j].y - verts[j].x * verts[i].y;
    }
    area * 0.5
}

pub fn polygon_centroid(verts: &[AnchorPoint]) -> AnchorPoint {
    let n = verts.len() as f64;
    if n == 0.0 {
        return AnchorPoint::new(0.0, 0.0);
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    for v in verts {
        cx += v.x;
        cy += v.y;
    }
    AnchorPoint::new(cx / n, cy / n)
}

pub fn line_segment_intersection(
    a1: AnchorPoint,
    a2: AnchorPoint,
    b1: AnchorPoint,
    b2: AnchorPoint,
) -> Option<AnchorPoint> {
    let d = (b2.y - b1.y) * (a2.x - a1.x) - (b2.x - b1.x) * (a2.y - a1.y);
    if d.abs() < 1e-6 {
        return None;
    }

    let ua = ((b2.x - b1.x) * (a1.y - b1.y) - (b2.y - b1.y) * (a1.x - b1.x)) / d;
    let ub = ((a2.x - a1.x) * (a1.y - b1.y) - (a2.y - a1.y) * (a1.x - b1.x)) / d;

    if (0.001..=0.999).contains(&ua) && (0.001..=0.999).contains(&ub) {
        Some(AnchorPoint::new(
            a1.x + ua * (a2.x - a1.x),
            a1.y + ua * (a2.y - a1.y),
        ))
    } else {
        None
    }
}

pub fn distance_to_segment(p: AnchorPoint, a: AnchorPoint, b: AnchorPoint) -> f64 {
    distance_point_to_segment(p, a, b)
}

pub fn distance_point_to_segment(p: AnchorPoint, a: AnchorPoint, b: AnchorPoint) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-6 {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    let proj = AnchorPoint::new(a.x + t * dx, a.y + t * dy);
    p.distance(proj)
}

pub fn transform_point_if_needed(p: &AnchorPoint, m: &[f64; 6]) -> AnchorPoint {
    AnchorPoint::new(
        m[0] * p.x + m[2] * p.y + m[4],
        m[1] * p.x + m[3] * p.y + m[5],
    )
}

pub fn offset_polygon(polygon: &[AnchorPoint], delta: f64) -> Vec<AnchorPoint> {
    if polygon.len() < 3 || delta.abs() < 1e-4 {
        return polygon.to_vec();
    }

    let n = polygon.len();
    let mut offset_poly = Vec::with_capacity(n);
    let centroid = polygon_centroid(polygon);

    for curr in polygon {
        let dx = curr.x - centroid.x;
        let dy = curr.y - centroid.y;
        let len = (dx * dx + dy * dy).sqrt().max(1e-5);
        let nx = dx / len;
        let ny = dy / len;
        offset_poly.push(AnchorPoint::new(curr.x + nx * delta, curr.y + ny * delta));
    }

    offset_poly
}

/// Live-corner radius from a local-space cursor: the depth into the rectangle
/// from the two edges meeting at local vertex `corner`, clamped like
/// `PathData::from_rect` (`0 ..= min(w, h) / 2`).
pub fn compute_corner_radius(
    width: f64,
    height: f64,
    local_x: f64,
    local_y: f64,
    corner: (f64, f64),
) -> f64 {
    let edge_left = local_x;
    let edge_right = width - local_x;
    let edge_top = local_y;
    let edge_bottom = height - local_y;
    let dx = if corner.0 <= width - corner.0 {
        edge_left
    } else {
        edge_right
    };
    let dy = if corner.1 <= height - corner.1 {
        edge_top
    } else {
        edge_bottom
    };
    dx.min(dy)
        .clamp(0.0, width.abs().min(height.abs()) * 0.5)
}
