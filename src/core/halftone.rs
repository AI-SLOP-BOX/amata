use super::geometry::point_in_polygon;
use super::path::{AnchorPoint, FillStyle, PathData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HalftonePattern {
    CircularGrid,
    HexagonalGrid,
    ScanlineMatrix,
}

/// Generate vector halftone dots inside a vector shape's bounding box
pub fn generate_halftone_from_path(
    path: &PathData,
    dot_spacing: f64,
    max_radius: f64,
    pattern: HalftonePattern,
) -> PathData {
    let poly = path.to_polygon(24);
    if poly.len() < 3 {
        return PathData::new();
    }

    let min_x = poly.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = poly.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = poly.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = poly.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

    let mut combined = PathData::new();
    let spacing = dot_spacing.max(2.0);
    let r_max = max_radius.max(1.0).min(spacing * 0.5);

    let mut y = min_y + spacing * 0.5;
    let mut row = 0;

    while y <= max_y {
        let x_offset = match pattern {
            HalftonePattern::HexagonalGrid if row % 2 != 0 => spacing * 0.5,
            _ => 0.0,
        };

        let mut x = min_x + spacing * 0.5 + x_offset;
        while x <= max_x {
            if point_in_polygon(x, y, &poly) {
                // Modulate dot radius based on proximity to center for aesthetic gradient
                let cx = (min_x + max_x) * 0.5;
                let cy = (min_y + max_y) * 0.5;
                let dist_c = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                let max_dist = ((max_x - min_x).hypot(max_y - min_y) * 0.5).max(1.0);
                let factor = (1.0_f64 - (dist_c / max_dist).clamp(0.0, 0.8)).powf(1.2);
                let dot_r = r_max * factor;

                if dot_r > 0.5 {
                    match pattern {
                        HalftonePattern::ScanlineMatrix => {
                            let mut line = PathData::new();
                            line.push_move_to(x - spacing * 0.4, y);
                            line.push_line_to(x + spacing * 0.4, y);
                            combined.elements.append(&mut line.elements);
                        }
                        _ => {
                            let circle = PathData::from_polygon_points(
                                &circle_polygon(x, y, dot_r, 12),
                                true,
                            );
                            let mut circle_elems = circle.elements;
                            combined.elements.append(&mut circle_elems);
                        }
                    }
                }
            }
            x += spacing;
        }
        y += match pattern {
            HalftonePattern::HexagonalGrid => spacing * 0.866,
            _ => spacing,
        };
        row += 1;
    }

    combined.fill = Some(FillStyle::solid([0.1, 0.1, 0.1, 1.0]));
    combined
}

fn circle_polygon(cx: f64, cy: f64, r: f64, segments: usize) -> Vec<AnchorPoint> {
    let mut pts = Vec::with_capacity(segments);
    for i in 0..segments {
        let angle = (i as f64 / segments as f64) * std::f64::consts::TAU;
        pts.push(AnchorPoint::new(cx + angle.cos() * r, cy + angle.sin() * r));
    }
    pts
}
