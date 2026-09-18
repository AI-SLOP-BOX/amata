use super::document::Object;
use super::path::{AnchorPoint, PathElement};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarpPreset {
    Bulge,
    Pinch,
    TwistS,
    WaveWarp,
}

pub fn apply_lattice_warp(
    obj: &Object,
    grid_cols: usize,
    grid_rows: usize,
    preset: WarpPreset,
    intensity: f64,
) -> Object {
    let (min_pt, max_pt) = obj
        .bounding_box()
        .unwrap_or((AnchorPoint::new(0.0, 0.0), AnchorPoint::new(100.0, 100.0)));
    let min_x = min_pt.x;
    let min_y = min_pt.y;
    let width = (max_pt.x - min_pt.x).max(1.0);
    let height = (max_pt.y - min_pt.y).max(1.0);

    let cols = grid_cols.clamp(2, 256);
    let rows = grid_rows.clamp(2, 256);

    // Initialize regular lattice grid
    let mut lattice = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut row = Vec::with_capacity(cols);
        let v = r as f64 / (rows - 1) as f64;
        for c in 0..cols {
            let u = c as f64 / (cols - 1) as f64;
            row.push(AnchorPoint::new(min_x + u * width, min_y + v * height));
        }
        lattice.push(row);
    }

    // Displace lattice grid based on preset
    let cx = min_x + width * 0.5;
    let cy = min_y + height * 0.5;

    for row in &mut lattice {
        for pt in row {
            let dx = (pt.x - cx) / (width * 0.5);
            let dy = (pt.y - cy) / (height * 0.5);
            let dist = (dx * dx + dy * dy).sqrt().min(1.0);

            match preset {
                WarpPreset::Bulge => {
                    let factor = (1.0 - dist) * intensity * 0.4;
                    pt.x += dx * width * factor;
                    pt.y += dy * height * factor;
                }
                WarpPreset::Pinch => {
                    let factor = (1.0 - dist) * intensity * -0.4;
                    pt.x += dx * width * factor;
                    pt.y += dy * height * factor;
                }
                WarpPreset::TwistS => {
                    let angle = (1.0 - dist) * intensity * 1.5;
                    let nx = dx * angle.cos() - dy * angle.sin();
                    let ny = dx * angle.sin() + dy * angle.cos();
                    pt.x = cx + nx * width * 0.5;
                    pt.y = cy + ny * height * 0.5;
                }
                WarpPreset::WaveWarp => {
                    let offset = (dx * std::f64::consts::PI).sin() * height * 0.2 * intensity;
                    pt.y += offset;
                }
            }
        }
    }

    // Warp points of object
    let warp_point = |p: AnchorPoint| -> AnchorPoint {
        let u = ((p.x - min_x) / width).clamp(0.0, 1.0);
        let v = ((p.y - min_y) / height).clamp(0.0, 1.0);

        let c_float = u * (cols - 1) as f64;
        let r_float = v * (rows - 1) as f64;

        let c0 = (c_float.floor() as usize).min(cols - 2);
        let r0 = (r_float.floor() as usize).min(rows - 2);
        let c1 = c0 + 1;
        let r1 = r0 + 1;

        let fu = c_float - c0 as f64;
        let fv = r_float - r0 as f64;

        let p00 = lattice[r0][c0];
        let p10 = lattice[r0][c1];
        let p01 = lattice[r1][c0];
        let p11 = lattice[r1][c1];

        // Bilinear interpolation
        let top_x = p00.x + fu * (p10.x - p00.x);
        let top_y = p00.y + fu * (p10.y - p00.y);
        let bot_x = p01.x + fu * (p11.x - p01.x);
        let bot_y = p01.y + fu * (p11.y - p01.y);

        let out_x = top_x + fv * (bot_x - top_x);
        let out_y = top_y + fv * (bot_y - top_y);

        AnchorPoint::new(out_x, out_y)
    };

    let mut clone = obj.clone();
    clone.id = uuid::Uuid::new_v4().to_string();
    clone.name = format!("{} (Warped)", obj.name);

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
