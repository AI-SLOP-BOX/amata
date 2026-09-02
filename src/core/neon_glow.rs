use super::document::Object;
use super::offset::offset_path;
use super::path::{FillStyle, PathData, StrokeStyle};

pub fn generate_neon_glow(
    path: &PathData,
    glow_color: [f32; 4],
    max_radius: f64,
    layers: usize,
) -> Vec<Object> {
    let layer_count = layers.clamp(3, 16);
    let mut objects = Vec::with_capacity(layer_count + 1);

    // 1. Outer ambient glow layers (thick with decaying alpha)
    for i in (1..=layer_count).rev() {
        let t = i as f64 / layer_count as f64;
        let r = max_radius * t;
        let alpha = (1.0 - t as f32).powf(1.8) * glow_color[3] * 0.4;

        let mut halo = if path.closed {
            offset_path(path, r)
        } else {
            path.clone()
        };

        if path.closed {
            halo.fill = Some(FillStyle::solid([
                glow_color[0],
                glow_color[1],
                glow_color[2],
                alpha * 0.5,
            ]));
            halo.stroke = Some(StrokeStyle {
                color: [glow_color[0], glow_color[1], glow_color[2], alpha],
                width: 1.5,
                dash_pattern: None,
                ..StrokeStyle::default()
            });
        } else {
            halo.fill = None;
            halo.stroke = Some(StrokeStyle {
                color: [glow_color[0], glow_color[1], glow_color[2], alpha],
                width: (r * 2.0).max(1.0),
                dash_pattern: None,
                ..StrokeStyle::default()
            });
        }

        let obj = Object::new_path(&format!("Neon Halo Tier {}", i), halo);
        objects.push(obj);
    }

    // 2. Ultra-bright core laser line / fill (white-hot center)
    let mut core = path.clone();
    if path.closed {
        core.fill = Some(FillStyle::solid([1.0, 1.0, 1.0, 0.95]));
        core.stroke = Some(StrokeStyle {
            color: [glow_color[0], glow_color[1], glow_color[2], 1.0],
            width: 2.0,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
    } else {
        core.fill = None;
        core.stroke = Some(StrokeStyle {
            color: [1.0, 1.0, 1.0, 0.95],
            width: 2.5,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
    }

    let core_obj = Object::new_path("Neon White-Hot Core", core);
    objects.push(core_obj);

    objects
}
