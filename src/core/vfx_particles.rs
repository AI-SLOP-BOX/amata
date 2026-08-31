use super::path::PathData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfxParticle {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub lifetime: f64,
    pub size: f64,
    pub color: [f32; 4],
}

/// Generate VFX particle bursts along a vector path for AEVFX Studio particle systems
pub fn generate_particle_trail(
    path: &PathData,
    count: usize,
    speed: f64,
    spread: f64,
) -> Vec<VfxParticle> {
    let poly = path.to_polygon(24);
    if poly.len() < 2 || count == 0 {
        return Vec::new();
    }

    let mut lengths = vec![0.0];
    let mut total_len = 0.0;
    for i in 0..poly.len() - 1 {
        let seg_len = poly[i].distance(poly[i + 1]);
        total_len += seg_len;
        lengths.push(total_len);
    }

    if total_len <= 1e-6 {
        return Vec::new();
    }

    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let t = i as f64 / count as f64;
        let target_dist = t * total_len;

        for j in 0..poly.len() - 1 {
            let d0 = lengths[j];
            let d1 = lengths[j + 1];
            if target_dist >= d0 && target_dist <= d1 {
                let seg_len = (d1 - d0).max(1e-6);
                let seg_t = (target_dist - d0) / seg_len;
                let p0 = poly[j];
                let p1 = poly[j + 1];

                let px = p0.x + seg_t * (p1.x - p0.x);
                let py = p0.y + seg_t * (p1.y - p0.y);

                let dx = p1.x - p0.x;
                let dy = p1.y - p0.y;
                let len = (dx * dx + dy * dy).sqrt().max(1e-6);

                let nx = -dy / len;
                let ny = dx / len;

                // Randomness hash for seed stability
                let hash = ((i as f64 * 12.9898 + 78.233).sin() * 43758.5453).fract().abs();
                let hash2 = ((i as f64 * 39.346 + 11.135).sin() * 43758.5453).fract().abs();

                let offset_perp = (hash - 0.5) * 2.0 * spread;
                let v_speed = speed * (0.8 + 0.4 * hash2);

                let vx = nx * offset_perp * 2.0 + (dx / len) * v_speed;
                let vy = ny * offset_perp * 2.0 + (dy / len) * v_speed;

                particles.push(VfxParticle {
                    position: [px + nx * offset_perp, py + ny * offset_perp, 0.0],
                    velocity: [vx, vy, (hash2 - 0.5) * speed * 0.5],
                    lifetime: 1.5 + hash * 1.5,
                    size: 3.0 + hash2 * 4.0,
                    color: [
                        0.2 + 0.8 * hash as f32,
                        0.5 + 0.5 * hash2 as f32,
                        1.0,
                        0.9,
                    ],
                });
                break;
            }
        }
    }

    particles
}
