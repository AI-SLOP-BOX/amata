use super::document::Object;
use super::path::{AnchorPoint, FillStyle, PathData, StrokeStyle};

/// Compute Voronoi cells using half-plane clipping for a set of seed points
pub fn generate_voronoi_cells(
    width: f64,
    height: f64,
    seeds: &[AnchorPoint],
    padding: f64,
) -> Vec<Object> {
    if seeds.len() < 2 {
        return Vec::new();
    }

    // Clipping is O(n^2); cap seeds so CLI typos cannot hang the process.
    let seeds: &[AnchorPoint] = if seeds.len() > 2048 {
        &seeds[..2048]
    } else {
        seeds
    };

    let mut objects = Vec::with_capacity(seeds.len());

    let initial_box = vec![
        AnchorPoint::new(0.0, 0.0),
        AnchorPoint::new(width, 0.0),
        AnchorPoint::new(width, height),
        AnchorPoint::new(0.0, height),
    ];

    for (i, &pi) in seeds.iter().enumerate() {
        let mut cell = initial_box.clone();

        for (j, &pj) in seeds.iter().enumerate() {
            if i == j {
                continue;
            }

            // Perpendicular bisector between pi and pj
            // Midpoint: (pi + pj) / 2
            // Normal pointing toward pi: (pi - pj)
            let mid_x = (pi.x + pj.x) * 0.5;
            let mid_y = (pi.y + pj.y) * 0.5;
            let nx = pi.x - pj.x;
            let ny = pi.y - pj.y;

            cell = clip_polygon_halfplane(&cell, mid_x, mid_y, nx, ny);
            if cell.len() < 3 {
                break;
            }
        }

        if cell.len() >= 3 {
            // Apply padding / insetting towards centroid
            let cx = cell.iter().map(|p| p.x).sum::<f64>() / cell.len() as f64;
            let cy = cell.iter().map(|p| p.y).sum::<f64>() / cell.len() as f64;

            let inset_pts: Vec<AnchorPoint> = cell
                .iter()
                .map(|p| {
                    let dx = cx - p.x;
                    let dy = cy - p.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
                    let shift = padding.min(dist * 0.4);
                    AnchorPoint::new(p.x + (dx / dist) * shift, p.y + (dy / dist) * shift)
                })
                .collect();

            let path = PathData::from_polygon_points(&inset_pts, true);
            let mut obj = Object::new_path(&format!("Voronoi Cell {}", i + 1), path);

            // Vibrant algorithmic color
            let hue = (i as f32 * 0.618_034_f32).fract();
            let [r, g, b] = hsv_to_rgb(hue, 0.7, 0.9);
            obj.fill = Some(FillStyle::solid([r, g, b, 0.9]));
            obj.stroke = Some(StrokeStyle {
                color: [0.1, 0.1, 0.1, 1.0],
                width: 1.5,
                dash_pattern: None,
                ..StrokeStyle::default()
            });

            objects.push(obj);
        }
    }

    objects
}

/// Clip a polygon against a half-plane defined by point (px, py) and normal (nx, ny) pointing inside
fn clip_polygon_halfplane(
    poly: &[AnchorPoint],
    px: f64,
    py: f64,
    nx: f64,
    ny: f64,
) -> Vec<AnchorPoint> {
    let mut out = Vec::new();
    let n = poly.len();
    if n < 3 {
        return out;
    }

    let is_inside = |p: AnchorPoint| -> bool { (p.x - px) * nx + (p.y - py) * ny >= 0.0 };

    for i in 0..n {
        let p1 = poly[i];
        let p2 = poly[(i + 1) % n];

        let in1 = is_inside(p1);
        let in2 = is_inside(p2);

        if in1 && in2 {
            out.push(p2);
        } else if in1 && !in2 {
            // Line intersection with bisector
            if let Some(ip) = line_intersection(p1, p2, px, py, nx, ny) {
                out.push(ip);
            }
        } else if !in1 && in2 {
            if let Some(ip) = line_intersection(p1, p2, px, py, nx, ny) {
                out.push(ip);
            }
            out.push(p2);
        }
    }

    out
}

fn line_intersection(
    p1: AnchorPoint,
    p2: AnchorPoint,
    px: f64,
    py: f64,
    nx: f64,
    ny: f64,
) -> Option<AnchorPoint> {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let denom = dx * nx + dy * ny;
    if denom.abs() < 1e-8 {
        return None;
    }
    let t = ((px - p1.x) * nx + (py - p1.y) * ny) / denom;
    Some(AnchorPoint::new(p1.x + t * dx, p1.y + t * dy))
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let i = (h * 6.0).floor() as usize;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match i % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}
