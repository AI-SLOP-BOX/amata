use super::document::Object;
use super::path::{AnchorPoint, PathData, StrokeStyle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowFieldPreset {
    Vortex,
    MagneticDipole,
    CyberChaos,
}

pub fn generate_flowfield_streamlines(
    preset: FlowFieldPreset,
    width: f64,
    height: f64,
    line_count: usize,
    step_count: usize,
    step_length: f64,
) -> Vec<Object> {
    let mut objects = Vec::with_capacity(line_count);
    let cx = width * 0.5;
    let cy = height * 0.5;

    for i in 0..line_count {
        let seed_x = ((i as f64 * 37.123 + 12.34).sin() * 43758.5453).fract().abs() * width;
        let seed_y = ((i as f64 * 91.567 + 84.12).sin() * 43758.5453).fract().abs() * height;

        let mut x = seed_x;
        let mut y = seed_y;
        let mut pts = Vec::with_capacity(step_count);
        pts.push(AnchorPoint::new(x, y));

        for _ in 0..step_count {
            let (vx, vy) = match preset {
                FlowFieldPreset::Vortex => {
                    let dx = x - cx;
                    let dy = y - cy;
                    let dist = (dx * dx + dy * dy).sqrt().max(10.0);
                    let angle = dy.atan2(dx) + std::f64::consts::FRAC_PI_2 + (dist * 0.01).sin() * 0.4;
                    (angle.cos(), angle.sin())
                }
                FlowFieldPreset::MagneticDipole => {
                    let d1x = x - (cx - 150.0);
                    let d1y = y - cy;
                    let r1 = (d1x * d1x + d1y * d1y).max(100.0);

                    let d2x = x - (cx + 150.0);
                    let d2y = y - cy;
                    let r2 = (d2x * d2x + d2y * d2y).max(100.0);

                    let fx = (d1x / r1) - (d2x / r2);
                    let fy = (d1y / r1) - (d2y / r2);
                    let flen = (fx * fx + fy * fy).sqrt().max(1e-6);
                    (fx / flen, fy / flen)
                }
                FlowFieldPreset::CyberChaos => {
                    let angle = ((x * 0.005).sin() + (y * 0.005).cos()) * std::f64::consts::PI * 2.0;
                    (angle.cos(), angle.sin())
                }
            };

            x += vx * step_length;
            y += vy * step_length;

            if x < 0.0 || x > width || y < 0.0 || y > height {
                break;
            }
            pts.push(AnchorPoint::new(x, y));
        }

        if pts.len() >= 2 {
            let mut path = PathData::from_polygon_points(&pts, false);
            path.fill = None;

            let hue = (i as f32 / line_count as f32) * 0.8;
            path.stroke = Some(StrokeStyle {
                color: match preset {
                    FlowFieldPreset::Vortex => [0.0, 0.8 + 0.2 * hue, 1.0, 0.75],
                    FlowFieldPreset::MagneticDipole => [1.0, 0.3 + 0.7 * hue, 0.2, 0.75],
                    FlowFieldPreset::CyberChaos => [0.2 + 0.8 * hue, 1.0, 0.4, 0.75],
                },
                width: 1.5,
                dash_pattern: None,
                ..StrokeStyle::default()
            });

            let obj = Object::new_path(&format!("Streamline {}", i + 1), path);
            objects.push(obj);
        }
    }

    objects
}
