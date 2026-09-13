use super::path::{AnchorPoint, FillStyle, PathData};
use std::collections::HashMap;

/// Extract vector contours from a grayscale bitmap using Marching Squares
pub fn trace_bitmap_to_polygons(
    width: usize,
    height: usize,
    pixels: &[u8],
    threshold: u8,
) -> Vec<Vec<AnchorPoint>> {
    if width < 2 || height < 2 || pixels.len() < width * height {
        return Vec::new();
    }

    let sample = |x: usize, y: usize| -> bool { pixels[y * width + x] >= threshold };

    let mut segments: Vec<((i64, i64), (i64, i64))> = Vec::new();

    // Marching squares over 1x1 cells
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let tl = sample(x, y);
            let tr = sample(x + 1, y);
            let br = sample(x + 1, y + 1);
            let bl = sample(x, y + 1);

            let case = (tl as u8) | ((tr as u8) << 1) | ((br as u8) << 2) | ((bl as u8) << 3);

            // Coordinates scaled by 2 to allow exact mid-points on grid edges
            let cx = (x as i64) * 2;
            let cy = (y as i64) * 2;

            let top = (cx + 1, cy);
            let right = (cx + 2, cy + 1);
            let bottom = (cx + 1, cy + 2);
            let left = (cx, cy + 1);

            match case {
                1 => segments.push((left, top)),
                2 => segments.push((top, right)),
                3 => segments.push((left, right)),
                4 => segments.push((right, bottom)),
                5 => {
                    segments.push((left, top));
                    segments.push((right, bottom));
                }
                6 => segments.push((top, bottom)),
                7 => segments.push((left, bottom)),
                8 => segments.push((bottom, left)),
                9 => segments.push((bottom, top)),
                10 => {
                    segments.push((top, right));
                    segments.push((bottom, left));
                }
                11 => segments.push((bottom, right)),
                12 => segments.push((right, left)),
                13 => segments.push((right, top)),
                14 => segments.push((top, left)),
                _ => {}
            }
        }
    }

    // Link segments into closed polygon contours
    let mut edge_map: HashMap<(i64, i64), (i64, i64)> = HashMap::new();
    for (p1, p2) in segments {
        edge_map.insert(p1, p2);
    }

    let mut polygons = Vec::new();
    let mut visited = std::collections::HashSet::new();

    for &start_pt in edge_map.keys() {
        if visited.contains(&start_pt) {
            continue;
        }

        let mut current = start_pt;
        let mut loop_pts = Vec::new();

        while let Some(&next) = edge_map.get(&current) {
            visited.insert(current);
            loop_pts.push(AnchorPoint::new(
                current.0 as f64 * 0.5,
                current.1 as f64 * 0.5,
            ));
            current = next;

            if current == start_pt || visited.contains(&current) {
                break;
            }
        }

        if loop_pts.len() >= 4 {
            let simplified = simplify_polygon(&loop_pts, 0.75);
            if simplified.len() >= 3 {
                polygons.push(simplified);
            }
        }
    }

    polygons
}

/// Simplify a polygon using collinear angle threshold
pub fn simplify_polygon(pts: &[AnchorPoint], tolerance: f64) -> Vec<AnchorPoint> {
    let n = pts.len();
    if n < 4 {
        return pts.to_vec();
    }

    let mut result = Vec::with_capacity(n);
    result.push(pts[0]);

    for i in 1..n - 1 {
        let p0 = *result.last().unwrap();
        let p1 = pts[i];
        let p2 = pts[i + 1];

        // Distance of p1 to segment p0-p2
        let dx = p2.x - p0.x;
        let dy = p2.y - p0.y;
        let len_sq = dx * dx + dy * dy;
        let dist = if len_sq > 1e-6 {
            ((p1.x - p0.x) * dy - (p1.y - p0.y) * dx).abs() / len_sq.sqrt()
        } else {
            p1.distance(p0)
        };

        if dist > tolerance {
            result.push(p1);
        }
    }

    result.push(*pts.last().unwrap());
    result
}

/// Convert bitmap contours into a composite PathData
pub fn trace_bitmap_to_path(width: usize, height: usize, pixels: &[u8], threshold: u8) -> PathData {
    let polys = trace_bitmap_to_polygons(width, height, pixels, threshold);
    let mut combined = PathData::new();

    for poly in &polys {
        let mut poly_path = PathData::from_polygon_points(poly, true);
        combined.elements.append(&mut poly_path.elements);
    }

    combined.closed = !polys.is_empty();
    combined.fill = Some(FillStyle::solid([0.1, 0.1, 0.1, 1.0]));
    combined.stroke = None;
    combined
}
