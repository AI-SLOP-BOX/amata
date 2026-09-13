use super::path::{AnchorPoint, PathData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeformType {
    SineWave,
    TurbulentNoise,
    JitterGlitch,
}

/// Apply procedural wave or noise deformation to a PathData
pub fn deform_path(
    path: &PathData,
    deform_type: DeformType,
    amplitude: f64,
    frequency: f64,
    seed: f64,
) -> PathData {
    let poly = path.to_polygon(24);
    if poly.len() < 2 {
        return path.clone();
    }

    let n = poly.len();
    let mut deformed_pts = Vec::with_capacity(n);

    for i in 0..n {
        let prev = if i == 0 {
            if path.closed {
                poly[n - 1]
            } else {
                poly[0]
            }
        } else {
            poly[i - 1]
        };
        let next = if i + 1 >= n {
            if path.closed {
                poly[0]
            } else {
                poly[n - 1]
            }
        } else {
            poly[i + 1]
        };
        let curr = poly[i];

        // Normal vector at vertex
        let dx = next.x - prev.x;
        let dy = next.y - prev.y;
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let nx = -dy / len;
        let ny = dx / len;

        let t = i as f64 * frequency + seed;

        let disp = match deform_type {
            DeformType::SineWave => (t * std::f64::consts::TAU).sin() * amplitude,
            DeformType::TurbulentNoise => {
                // Multi-octave pseudo-perlin noise approximation
                let n1 = (t * 1.0).sin();
                let n2 = (t * 2.3 + 1.2).sin() * 0.5;
                let n3 = (t * 4.7 + 2.4).sin() * 0.25;
                (n1 + n2 + n3) * amplitude * 0.57
            }
            DeformType::JitterGlitch => {
                let h = ((t * 12.9898 + 78.233).sin() * 43758.5453).fract();
                (h - 0.5) * 2.0 * amplitude
            }
        };

        deformed_pts.push(AnchorPoint::new(curr.x + nx * disp, curr.y + ny * disp));
    }

    let mut new_path = PathData::from_polygon_points(&deformed_pts, path.closed);
    new_path.fill = path.fill.clone();
    new_path.stroke = path.stroke.clone();
    new_path
}
