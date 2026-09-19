use crate::core::path::FillStyle;
use crate::core::state::AppState;
use egui::{Color32, Ui, Vec2};

/// Convert RGB (0.0..1.0) to CMYK (0.0..1.0 each).
/// Simple subtractive model: K = 1 - max(R,G,B), C/M/Y = (K - channel) / K.
pub fn rgb_to_cmyk(r: f32, g: f32, b: f32) -> (f32, f32, f32, f32) {
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return (0.0, 0.0, 0.0, 1.0);
    }
    let inv = 1.0 - k;
    let c = (inv - r) / inv;
    let m = (inv - g) / inv;
    let y = (inv - b) / inv;
    (c.clamp(0.0, 1.0), m.clamp(0.0, 1.0), y.clamp(0.0, 1.0), k)
}

/// Convert CMYK (0.0..1.0 each) to RGB (0.0..1.0).
pub fn cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> (f32, f32, f32) {
    let r = (1.0 - c) * (1.0 - k);
    let g = (1.0 - m) * (1.0 - k);
    let b = (1.0 - y) * (1.0 - k);
    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

pub fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;

    let s = if max == 0.0 { 0.0 } else { d / max };
    let v = max;

    let h = if d == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d) % 6.0
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    let h = (h * 60.0).rem_euclid(360.0);
    (h, s, v)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r + m, g + m, b + m)
}

pub fn render_swatches(ui: &mut Ui, state: &mut AppState) {
    let swatches: [[f32; 4]; 12] = [
        [0.0, 0.0, 0.0, 1.0],    // Black
        [1.0, 1.0, 1.0, 1.0],    // White
        [0.9, 0.2, 0.2, 1.0],    // Red
        [0.95, 0.6, 0.1, 1.0],   // Orange
        [0.95, 0.85, 0.15, 1.0], // Yellow
        [0.2, 0.8, 0.3, 1.0],    // Green
        [0.1, 0.7, 0.9, 1.0],    // Cyan
        [0.2, 0.5, 0.9, 1.0],    // Blue
        [0.6, 0.25, 0.85, 1.0],  // Purple
        [0.9, 0.3, 0.6, 1.0],    // Pink
        [0.55, 0.35, 0.2, 1.0],  // Brown
        [0.5, 0.55, 0.6, 1.0],   // Gray
    ];

    ui.horizontal_wrapped(|ui| {
        for color in swatches {
            let c32 = Color32::from_rgba_unmultiplied(
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                (color[3] * 255.0) as u8,
            );
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 2.0_f32, c32);
            ui.painter().rect_stroke(
                rect,
                2.0_f32,
                egui::Stroke::new(1.0_f32, Color32::from_gray(80)),
                egui::StrokeKind::Inside,
            );

            if response.clicked() {
                state.fill_color = color;
                for id in &state.selected_ids {
                    for (_, obj) in state.document.all_objects_mut() {
                        if &obj.id == id {
                            obj.fill = Some(FillStyle::solid(color));
                        }
                    }
                }
            }
        }
    });
}
