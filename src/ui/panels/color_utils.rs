use crate::core::path::FillStyle;
use crate::core::state::AppState;
use egui::{Color32, Response, Sense, Ui, Vec2};

/// Adobe-style colour button: a swatch that opens our own picker popup
/// (SV square + hue strip + hex + alpha) instead of the stock egui
/// picker, so colour editing matches the app's dark theme and the
/// unmultiplied-sRGB convention everywhere.
pub fn color_edit_srgba(ui: &mut Ui, color: &mut [f32; 4]) -> Response {
    adobe_color_button(ui, color)
}

/// Byte-component variant: converts through `[f32; 4]`, same popup.
pub fn color_edit_srgba_u8(ui: &mut Ui, color: &mut [u8; 4]) -> Response {
    let mut f = [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
        color[3] as f32 / 255.0,
    ];
    let resp = adobe_color_button(ui, &mut f);
    if resp.changed() {
        color[0] = (f[0].clamp(0.0, 1.0) * 255.0) as u8;
        color[1] = (f[1].clamp(0.0, 1.0) * 255.0) as u8;
        color[2] = (f[2].clamp(0.0, 1.0) * 255.0) as u8;
        color[3] = (f[3].clamp(0.0, 1.0) * 255.0) as u8;
    }
    resp
}

fn f32_to_c32(c: [f32; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
        (c[3].clamp(0.0, 1.0) * 255.0) as u8,
    )
}

// 1px white texture for the SV mesh, cached per egui Context.
// `TextureHandle` is not `SerializableAny`, so it cannot live in egui's
// persisted typemap; a process-wide cache keyed by context pointer is the
// standard workaround. Entries outlive their context (one 1px texture),
// which is negligible and documented here instead of hidden.
static WHITE_TEX: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, egui::TextureHandle>>,
> = std::sync::OnceLock::new();

fn white_tex_map(
) -> &'static std::sync::Mutex<std::collections::HashMap<usize, egui::TextureHandle>> {
    WHITE_TEX.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn white_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let key = ctx as *const egui::Context as usize;
    if let Some(handle) = white_tex_map().lock().unwrap().get(&key).cloned() {
        return handle;
    }
    let img = egui::ColorImage::new([1, 1], Color32::WHITE);
    let handle = ctx.load_texture("amata_white_tex", img, egui::TextureOptions::NEAREST);
    white_tex_map().lock().unwrap().insert(key, handle.clone());
    handle
}

/// Swatch button + custom picker popup. Returns a response whose
/// `changed()` is true while the popup edits the colour.
pub fn adobe_color_button(ui: &mut Ui, color: &mut [f32; 4]) -> Response {
    let before = *color;
    let c32 = f32_to_c32(*color);
    // Checkerboard behind translucent colours so alpha reads correctly.
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(44.0, 18.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let cell = 4.5_f32;
        let mut y = rect.min.y;
        let mut row = 0;
        while y < rect.max.y {
            let mut x = rect.min.x;
            let mut col = row;
            while x < rect.max.x {
                let cell_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(x, y),
                    Vec2::new(cell.min(rect.max.x - x), cell.min(rect.max.y - y)),
                );
                let shade = if col % 2 == 0 { 200 } else { 150 };
                ui.painter()
                    .rect_filled(cell_rect, 0.0, Color32::from_gray(shade));
                x += cell;
                col += 1;
            }
            y += cell;
            row += 1;
        }
        ui.painter().rect_filled(rect, 2.0, c32);
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0_f32, Color32::from_gray(90)),
            egui::StrokeKind::Middle,
        );
        if resp.hovered() {
            ui.painter().rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0_f32, Color32::from_gray(200)),
                egui::StrokeKind::Outside,
            );
        }
    }
    let popup_id = egui::Id::new(("adobe_color_popup", resp.id));
    if resp.clicked() {
        ui.ctx().memory_mut(|m| {
            m.toggle_popup(popup_id);
        });
    }
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &resp,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            adobe_color_popup_body(ui, color, egui::Id::new(("adobe_hex", resp.id)));
        },
    );
    if *color != before {
        resp.mark_changed();
    }
    resp
}

/// Popup content: SV square, hue strip, hex field, alpha slider.
fn adobe_color_popup_body(ui: &mut Ui, color: &mut [f32; 4], hex_id: egui::Id) {
    let (mut h, mut s, mut v) = rgb_to_hsv(color[0], color[1], color[2]);
    if v == 0.0 {
        // Black has no hue/saturation signal; keep the previous hue corner
        // stable instead of snapping to red.
        h = 0.0;
        s = 0.0;
    }
    ui.set_min_width(220.0);
    // Shared pointer-to-HSV helper for both click and drag.
    let set_sv = |pos: egui::Pos2, sq_rect: egui::Rect, s: &mut f32, v: &mut f32| {
        *s = ((pos.x - sq_rect.min.x) / sq_rect.width().max(1.0)).clamp(0.0, 1.0);
        *v = (1.0 - (pos.y - sq_rect.min.y) / sq_rect.height().max(1.0)).clamp(0.0, 1.0);
    };
    ui.horizontal(|ui| {
        // Saturation (x) x Value (y) square: 16x16 subdivided mesh so the
        // bilinear ramp is exact, not a 4-vertex diagonal blend.
        let sq = Vec2::splat(150.0);
        let (sq_rect, sq_resp) = ui.allocate_exact_size(sq, Sense::click_and_drag());
        let cell = |ss: f32, vv: f32| {
            let (r, g, b) = hsv_to_rgb(h, ss.clamp(0.0, 1.0), vv.clamp(0.0, 1.0));
            f32_to_c32([r, g, b, 1.0])
        };
        if ui.is_rect_visible(sq_rect) {
            const N: usize = 16;
            let tex = white_texture(ui.ctx());
            let mut mesh = egui::epaint::Mesh::with_texture(tex.id());
            let p = |ix: usize, iy: usize| {
                egui::Pos2::new(
                    sq_rect.min.x + ix as f32 / N as f32 * sq_rect.width(),
                    sq_rect.min.y + iy as f32 / N as f32 * sq_rect.height(),
                )
            };
            // Grid vertices row by row; two triangles per cell.
            for iy in 0..=N {
                for ix in 0..=N {
                    let vv = egui::epaint::Vertex {
                        pos: p(ix, iy),
                        uv: egui::epaint::WHITE_UV,
                        color: cell(ix as f32 / N as f32, 1.0 - iy as f32 / N as f32),
                    };
                    mesh.vertices.push(vv);
                }
            }
            let at = |ix: usize, iy: usize| (iy * (N + 1) + ix) as u32;
            for iy in 0..N {
                for ix in 0..N {
                    mesh.add_triangle(at(ix, iy), at(ix, iy + 1), at(ix + 1, iy));
                    mesh.add_triangle(at(ix + 1, iy), at(ix, iy + 1), at(ix + 1, iy + 1));
                }
            }
            ui.painter().add(mesh);
            ui.painter().rect_stroke(
                sq_rect,
                2.0,
                egui::Stroke::new(1.0_f32, Color32::from_gray(90)),
                egui::StrokeKind::Middle,
            );
            // Crosshair at the current SV.
            let dot = egui::Pos2::new(
                sq_rect.min.x + s.clamp(0.0, 1.0) * sq_rect.width(),
                sq_rect.min.y + (1.0 - v.clamp(0.0, 1.0)) * sq_rect.height(),
            );
            ui.painter()
                .circle_stroke(dot, 4.0, egui::Stroke::new(1.5_f32, Color32::WHITE));
            ui.painter()
                .circle_stroke(dot, 4.0, egui::Stroke::new(0.75_f32, Color32::BLACK));
        }
        // Click jumps, drag slides (click_and_drag reports both).
        if sq_resp.clicked() {
            if let Some(pos) = sq_resp.interact_pointer_pos() {
                set_sv(pos, sq_rect, &mut s, &mut v);
            }
        }
        if sq_resp.dragged() {
            if let Some(pos) = sq_resp.interact_pointer_pos() {
                set_sv(pos, sq_rect, &mut s, &mut v);
            }
        }
        // Hue strip.
        let strip_w = 16.0_f32;
        let (hue_rect, hue_resp) =
            ui.allocate_exact_size(Vec2::new(strip_w, sq.y), Sense::click_and_drag());
        if ui.is_rect_visible(hue_rect) {
            let segs = 24;
            for i in 0..segs {
                let t0 = i as f32 / segs as f32;
                let t1 = (i + 1) as f32 / segs as f32;
                let (r, g, b) = hsv_to_rgb(t0 * 360.0, 1.0, 1.0);
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::Pos2::new(hue_rect.min.x, hue_rect.min.y + t0 * hue_rect.height()),
                        Vec2::new(strip_w, (t1 - t0) * hue_rect.height() + 1.0),
                    ),
                    0.0,
                    f32_to_c32([r, g, b, 1.0]),
                );
            }
            let mark_y = hue_rect.min.y + (h / 360.0).clamp(0.0, 1.0) * hue_rect.height();
            ui.painter().line_segment(
                [
                    egui::Pos2::new(hue_rect.min.x - 2.0, mark_y),
                    egui::Pos2::new(hue_rect.max.x + 2.0, mark_y),
                ],
                egui::Stroke::new(2.0_f32, Color32::WHITE),
            );
        }
        let set_hue = |pos: egui::Pos2| {
            ((pos.y - hue_rect.min.y) / hue_rect.height().max(1.0)).clamp(0.0, 1.0) * 360.0
        };
        if hue_resp.clicked() {
            if let Some(pos) = hue_resp.interact_pointer_pos() {
                h = set_hue(pos);
            }
        }
        if hue_resp.dragged() {
            if let Some(pos) = hue_resp.interact_pointer_pos() {
                h = set_hue(pos);
            }
        }
    });
    let (r, g, b) = hsv_to_rgb(h, s, v);
    color[0] = r;
    color[1] = g;
    color[2] = b;
    ui.horizontal(|ui| {
        ui.label("Hex:");
        // Per-popup edit buffer: rebuilding the string from `color` every
        // frame would eat keystrokes mid-typing. Resync only when the
        // color moved while the field was unfocused.
        #[derive(Clone, Default)]
        struct HexBuf {
            text: String,
            synced: [f32; 4],
            was_focused: bool,
        }
        let hex_of = |c: &[f32; 4]| {
            format!(
                "#{:02X}{:02X}{:02X}",
                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
            )
        };
        let mut hb: HexBuf = ui.memory(|m| m.data.get_temp(hex_id)).unwrap_or_default();
        if hb.synced != *color && !hb.was_focused {
            hb.text = hex_of(color);
            hb.synced = *color;
        }
        if hb.text.is_empty() {
            hb.text = hex_of(color);
            hb.synced = *color;
        }
        let hex_resp = ui.add(egui::TextEdit::singleline(&mut hb.text).desired_width(70.0));
        hb.was_focused = hex_resp.has_focus();
        if hex_resp.lost_focus() {
            let digits = hb.text.trim().trim_start_matches('#');
            if digits.len() == 6 {
                if let (Ok(rr), Ok(gg), Ok(bb)) = (
                    u8::from_str_radix(&digits[0..2], 16),
                    u8::from_str_radix(&digits[2..4], 16),
                    u8::from_str_radix(&digits[4..6], 16),
                ) {
                    color[0] = rr as f32 / 255.0;
                    color[1] = gg as f32 / 255.0;
                    color[2] = bb as f32 / 255.0;
                    hb.synced = *color;
                }
            }
            hb.text = hex_of(color);
            hb.synced = *color;
        }
        ui.memory_mut(|m| {
            m.data.insert_temp(hex_id, hb);
        });
        ui.label("\u{3b1}:");
        ui.add(egui::Slider::new(&mut color[3], 0.0..=1.0).show_value(false));
    });
}

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
