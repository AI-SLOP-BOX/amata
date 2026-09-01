use super::document::Object;
use super::path::{AnchorPoint, FillStyle, PathData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientMeshPreset {
    Sunset,
    Cyberpunk,
    Aurora,
    Gold,
}

pub fn generate_gradient_mesh(
    preset: GradientMeshPreset,
    width: f64,
    height: f64,
    rows: usize,
    cols: usize,
) -> Vec<Object> {
    let r_count = rows.max(2);
    let c_count = cols.max(2);
    let mut objects = Vec::with_capacity((r_count - 1) * (c_count - 1));

    // Define corner colors based on preset
    let (c_tl, c_tr, c_bl, c_br) = match preset {
        GradientMeshPreset::Sunset => (
            [1.0, 0.2, 0.4, 0.95],
            [1.0, 0.6, 0.1, 0.95],
            [0.4, 0.1, 0.6, 0.95],
            [0.1, 0.05, 0.3, 0.95],
        ),
        GradientMeshPreset::Cyberpunk => (
            [0.0, 0.9, 1.0, 0.95],
            [1.0, 0.1, 0.6, 0.95],
            [0.1, 0.0, 0.4, 0.95],
            [0.0, 1.0, 0.5, 0.95],
        ),
        GradientMeshPreset::Aurora => (
            [0.1, 0.9, 0.5, 0.95],
            [0.1, 0.5, 0.9, 0.95],
            [0.5, 0.1, 0.8, 0.95],
            [0.05, 0.2, 0.4, 0.95],
        ),
        GradientMeshPreset::Gold => (
            [1.0, 0.9, 0.5, 0.95],
            [0.9, 0.6, 0.2, 0.95],
            [0.6, 0.4, 0.1, 0.95],
            [0.3, 0.2, 0.05, 0.95],
        ),
    };

    let sample_color = |u: f64, v: f64| -> [f32; 4] {
        let u_f = u as f32;
        let v_f = v as f32;
        let mut out = [0.0; 4];
        for i in 0..4 {
            let top = c_tl[i] + u_f * (c_tr[i] - c_tl[i]);
            let bot = c_bl[i] + u_f * (c_br[i] - c_bl[i]);
            out[i] = top + v_f * (bot - top);
        }
        out
    };

    let sub_steps = 4; // Sub-division for smooth vector color patches
    let cell_w = width / (c_count - 1) as f64;
    let cell_h = height / (r_count - 1) as f64;

    for r in 0..r_count - 1 {
        for c in 0..c_count - 1 {
            let x0 = c as f64 * cell_w;
            let y0 = r as f64 * cell_h;

            for sr in 0..sub_steps {
                for sc in 0..sub_steps {
                    let u0 = (c as f64 + sc as f64 / sub_steps as f64) / (c_count - 1) as f64;
                    let v0 = (r as f64 + sr as f64 / sub_steps as f64) / (r_count - 1) as f64;
                    let u1 = (c as f64 + (sc + 1) as f64 / sub_steps as f64) / (c_count - 1) as f64;
                    let v1 = (r as f64 + (sr + 1) as f64 / sub_steps as f64) / (r_count - 1) as f64;

                    let px0 = x0 + (sc as f64 / sub_steps as f64) * cell_w;
                    let py0 = y0 + (sr as f64 / sub_steps as f64) * cell_h;
                    let px1 = x0 + ((sc + 1) as f64 / sub_steps as f64) * cell_w;
                    let py1 = y0 + ((sr + 1) as f64 / sub_steps as f64) * cell_h;

                    let pts = vec![
                        AnchorPoint::new(px0, py0),
                        AnchorPoint::new(px1, py0),
                        AnchorPoint::new(px1, py1),
                        AnchorPoint::new(px0, py1),
                    ];

                    let color = sample_color((u0 + u1) * 0.5, (v0 + v1) * 0.5);
                    let mut path = PathData::from_polygon_points(&pts, true);
                    path.fill = Some(FillStyle::solid(color));
                    path.stroke = None;

                    let obj = Object::new_path(
                        &format!("Mesh Patch [{},{}]-({},{})", r, c, sr, sc),
                        path,
                    );
                    objects.push(obj);
                }
            }
        }
    }

    objects
}
