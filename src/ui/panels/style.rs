use super::color_utils::{hsv_to_rgb, rgb_to_hsv};
use crate::core::document::BlendMode;
use crate::core::path::{
    ArrowHead, FillStyle, GradientStop, LinearGradient, RadialGradient, StrokeCap, StrokeJoin,
    StrokeStyle,
};
use crate::core::state::AppState;
use egui::{Color32, RichText, Ui, Vec2};

pub struct StrokePanel;

impl StrokePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🖌 Stroke").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select an object to edit stroke").weak());
            return;
        }

        let id = state.selected_ids[0].clone();
        let mut width = 1.0;
        let mut color = [0.0, 0.0, 0.0, 1.0];
        let mut cap = StrokeCap::Butt;
        let mut join = StrokeJoin::Miter;
        let mut miter_limit = 4.0;
        let mut dash_pattern = String::new();
        let mut arrow_start = ArrowHead::None;
        let mut arrow_end = ArrowHead::None;
        let mut found = false;

        for (_, obj) in state.document.all_objects() {
            if obj.id == id {
                if let Some(ref s) = obj.stroke {
                    width = s.width;
                    color = s.color;
                    cap = s.cap;
                    join = s.join;
                    miter_limit = s.miter_limit;
                    arrow_start = s.arrow_start;
                    arrow_end = s.arrow_end;
                    if let Some(ref dashes) = s.dash_pattern {
                        dash_pattern = dashes
                            .iter()
                            .map(|d| d.to_string())
                            .collect::<Vec<_>>()
                            .join(", ");
                    }
                }
                found = true;
                break;
            }
        }

        if !found {
            return;
        }

        // Stroke Color
        ui.horizontal(|ui| {
            ui.label("Color:");
            let mut c = color;
            if ui.color_edit_button_rgba_premultiplied(&mut c).changed() {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke {
                            s.color = c;
                        }
                    }
                }
            }
        });

        // Stroke Width
        ui.horizontal(|ui| {
            ui.label("Width:");
            let mut w = width;
            if ui
                .add(
                    egui::DragValue::new(&mut w)
                        .speed(0.5)
                        .range(0.0..=200.0)
                        .suffix("px"),
                )
                .changed()
            {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke {
                            s.width = w;
                        }
                    }
                }
            }
        });

        ui.separator();

        // Cap
        ui.horizontal(|ui| {
            ui.label("Cap:");
            let mut new_cap = cap;
            for cap_type in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
                if ui
                    .selectable_label(new_cap == cap_type, cap_type.name())
                    .clicked()
                {
                    new_cap = cap_type;
                }
            }
            if new_cap != cap {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke {
                            s.cap = new_cap;
                        }
                    }
                }
            }
        });

        // Join
        ui.horizontal(|ui| {
            ui.label("Join:");
            let mut new_join = join;
            for join_type in [StrokeJoin::Miter, StrokeJoin::Round, StrokeJoin::Bevel] {
                if ui
                    .selectable_label(new_join == join_type, join_type.name())
                    .clicked()
                {
                    new_join = join_type;
                }
            }
            if new_join != join {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke {
                            s.join = new_join;
                        }
                    }
                }
            }
        });

        // Miter Limit
        if join == StrokeJoin::Miter {
            ui.horizontal(|ui| {
                ui.label("Miter Limit:");
                let mut ml = miter_limit;
                if ui
                    .add(egui::DragValue::new(&mut ml).speed(0.5).range(1.0..=100.0))
                    .changed()
                {
                    for (_, obj) in state.document.all_objects_mut() {
                        if obj.id == id {
                            if let Some(ref mut s) = obj.stroke {
                                s.miter_limit = ml;
                            }
                        }
                    }
                }
            });
        }

        ui.separator();

        // Dash Pattern
        ui.horizontal(|ui| {
            ui.label("Dash:");
            let mut dp = dash_pattern.clone();
            if ui.text_edit_singleline(&mut dp).changed() {
                let new_dashes: Option<Vec<f64>> = if dp.trim().is_empty() {
                    None
                } else {
                    Some(
                        dp.split(',')
                            .filter_map(|s| s.trim().parse().ok())
                            .collect(),
                    )
                };
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke {
                            s.dash_pattern = new_dashes.clone();
                        }
                    }
                }
            }
        });
        ui.label(
            RichText::new("Comma-separated (e.g. 10, 5)")
                .weak()
                .size(10.0),
        );

        ui.separator();

        // Arrowheads
        ui.label("Arrowheads:");
        ui.horizontal(|ui| {
            ui.label("Start:");
            let mut new_as = arrow_start;
            for ah in [
                ArrowHead::None,
                ArrowHead::Triangle,
                ArrowHead::Arrow,
                ArrowHead::Circle,
                ArrowHead::Diamond,
                ArrowHead::Square,
                ArrowHead::Barbed,
            ] {
                if ui.selectable_label(new_as == ah, ah.name()).clicked() {
                    new_as = ah;
                }
            }
            if new_as != arrow_start {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke {
                            s.arrow_start = new_as;
                        }
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("End:");
            let mut new_ae = arrow_end;
            for ah in [
                ArrowHead::None,
                ArrowHead::Triangle,
                ArrowHead::Arrow,
                ArrowHead::Circle,
                ArrowHead::Diamond,
                ArrowHead::Square,
                ArrowHead::Barbed,
            ] {
                if ui.selectable_label(new_ae == ah, ah.name()).clicked() {
                    new_ae = ah;
                }
            }
            if new_ae != arrow_end {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke {
                            s.arrow_end = new_ae;
                        }
                    }
                }
            }
        });
    }
}

// ═══════════════════════════════════════════════════════════════════
// GradientPanel: Linear/Radial gradient editing with color stops
// ═══════════════════════════════════════════════════════════════════

pub struct BlendModePanel;

impl BlendModePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🎨 Blend Mode").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select an object to set blend mode").weak());
            return;
        }

        let id = state.selected_ids[0].clone();
        let mut current = BlendMode::Normal;

        for (_, obj) in state.document.all_objects() {
            if obj.id == id {
                current = obj.blend_mode;
                break;
            }
        }

        for mode in BlendMode::all() {
            let is_selected = current == *mode;
            if ui.selectable_label(is_selected, mode.name()).clicked() && !is_selected {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        obj.blend_mode = *mode;
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// TransformPanel: Precise numeric transforms (X/Y/W/H/Rotation)
// ═══════════════════════════════════════════════════════════════════

pub struct AppearancePanel;

impl AppearancePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🎭 Appearance").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select an object").weak());
            return;
        }

        let id = state.selected_ids[0].clone();

        // Blend Mode
        ui.horizontal(|ui| {
            ui.label("Blend:");
            let mut current = BlendMode::Normal;
            for (_, obj) in state.document.all_objects() {
                if obj.id == id {
                    current = obj.blend_mode;
                    break;
                }
            }
            egui::ComboBox::from_id_salt("blend_mode_combo")
                .selected_text(current.name())
                .show_ui(ui, |ui| {
                    for mode in BlendMode::all() {
                        if ui.selectable_label(current == *mode, mode.name()).clicked() {
                            for (_, obj) in state.document.all_objects_mut() {
                                if obj.id == id {
                                    obj.blend_mode = *mode;
                                }
                            }
                        }
                    }
                });
        });

        // Opacity
        ui.horizontal(|ui| {
            ui.label("Opacity:");
            let mut opac = 1.0;
            for (_, obj) in state.document.all_objects() {
                if obj.id == id {
                    opac = obj.opacity;
                    break;
                }
            }
            let opac_resp = ui.add(egui::Slider::new(&mut opac, 0.0..=1.0).show_value(true));
            if opac_resp.changed() {
                state.object_edit(&id, &opac_resp, |o| {
                    o.opacity = opac;
                });
            }
            if opac_resp.drag_stopped() {
                state.commit_object_edits("Edit Object");
            }
        });

        ui.separator();

        // Fill
        ui.label(RichText::new("Fill").strong());
        ui.horizontal(|ui| {
            let mut fill_color = state.fill_color;
            let fill_resp = ui.color_edit_button_rgba_premultiplied(&mut fill_color);
            if fill_resp.changed() {
                state.fill_color = fill_color;
                state.object_edit(&id, &fill_resp, |o| {
                    o.fill = Some(FillStyle::solid(fill_color));
                });
            }
            if fill_resp.drag_stopped() {
                state.commit_object_edits("Edit Object");
            }
            if ui.button("No Fill").clicked() {
                state.ensure_object_snapshot(&id);
                if let Some(o) = state.document.find_object_mut(&id) {
                    o.fill = None;
                }
                state.commit_object_edits("Edit Object");
            }
        });

        ui.separator();

        // Stroke
        ui.label(RichText::new("Stroke").strong());
        ui.horizontal(|ui| {
            let mut stroke_color = state.stroke_color;
            let stroke_resp = ui.color_edit_button_rgba_premultiplied(&mut stroke_color);
            if stroke_resp.changed() {
                state.stroke_color = stroke_color;
                state.object_edit(&id, &stroke_resp, |o| {
                    if let Some(ref mut s) = o.stroke {
                        s.color = stroke_color;
                    } else {
                        o.stroke = Some(StrokeStyle {
                            color: stroke_color,
                            ..StrokeStyle::default()
                        });
                    }
                });
            }
            if stroke_resp.drag_stopped() {
                state.commit_object_edits("Edit Object");
            }
            if ui.button("No Stroke").clicked() {
                state.ensure_object_snapshot(&id);
                if let Some(o) = state.document.find_object_mut(&id) {
                    o.stroke = None;
                }
                state.commit_object_edits("Edit Object");
            }
        });

        ui.horizontal(|ui| {
            ui.label("Width:");
            let mut sw = 1.0;
            for (_, obj) in state.document.all_objects() {
                if obj.id == id {
                    if let Some(ref s) = obj.stroke {
                        sw = s.width;
                    }
                }
            }
            let sw_resp =
                ui.add(egui::DragValue::new(&mut sw).speed(0.5).range(0.0..=200.0));
            if sw_resp.changed() {
                state.object_edit(&id, &sw_resp, |o| {
                    if let Some(ref mut s) = o.stroke {
                        s.width = sw;
                    }
                });
            }
            if sw_resp.drag_stopped() {
                state.commit_object_edits("Edit Object");
            }
        });

        ui.separator();

        // Effects summary
        ui.label(RichText::new("Effects").strong());
        let mut has_shadow = false;
        let mut has_glow = false;
        for (_, obj) in state.document.all_objects() {
            if obj.id == id {
                has_shadow = obj.shadow.is_some();
                has_glow = obj.glow.is_some();
                break;
            }
        }
        ui.label(format!(
            "Shadow: {} | Glow: {}",
            if has_shadow { "On" } else { "Off" },
            if has_glow { "On" } else { "Off" }
        ));
    }
}

// ═══════════════════════════════════════════════════════════════════
// SwatchesPanel: Extended swatches with gradient presets
// ═══════════════════════════════════════════════════════════════════

pub struct SwatchesPanel;

impl SwatchesPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🎨 Swatches").strong());
        ui.add_space(4.0);

        // Basic color swatches
        ui.label(RichText::new("Colors").strong().size(11.0));
        let swatches: [[f32; 4]; 20] = [
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [0.9, 0.2, 0.2, 1.0],
            [0.95, 0.6, 0.1, 1.0],
            [0.95, 0.85, 0.15, 1.0],
            [0.2, 0.8, 0.3, 1.0],
            [0.1, 0.7, 0.9, 1.0],
            [0.2, 0.5, 0.9, 1.0],
            [0.6, 0.25, 0.85, 1.0],
            [0.9, 0.3, 0.6, 1.0],
            [0.55, 0.35, 0.2, 1.0],
            [0.5, 0.55, 0.6, 1.0],
            [0.8, 0.0, 0.0, 1.0],
            [0.0, 0.8, 0.0, 1.0],
            [0.0, 0.0, 0.8, 1.0],
            [1.0, 0.5, 0.0, 1.0],
            [0.5, 0.0, 0.5, 1.0],
            [0.0, 0.5, 0.5, 1.0],
            [0.8, 0.8, 0.0, 1.0],
            [0.4, 0.2, 0.0, 1.0],
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
                    ui.allocate_exact_size(Vec2::new(18.0, 18.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 2.0, c32);
                ui.painter().rect_stroke(
                    rect,
                    2.0,
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

        ui.add_space(8.0);

        // Adobe CC Hex & RGB color picker field
        ui.horizontal(|ui| {
            ui.label(RichText::new("Hex:").size(11.0));
            let r = (state.fill_color[0] * 255.0) as u8;
            let g = (state.fill_color[1] * 255.0) as u8;
            let b = (state.fill_color[2] * 255.0) as u8;
            let mut hex_buf = format!("#{:02X}{:02X}{:02X}", r, g, b);
            if ui
                .add(egui::TextEdit::singleline(&mut hex_buf).desired_width(70.0))
                .changed()
            {
                if let Some(c) = crate::io::svg::parse_svg_color(&hex_buf) {
                    state.fill_color = [c[0], c[1], c[2], state.fill_color[3]];
                    for id in &state.selected_ids {
                        for (_, obj) in state.document.all_objects_mut() {
                            if &obj.id == id {
                                obj.fill = Some(FillStyle::solid(state.fill_color));
                            }
                        }
                    }
                }
            }

            let mut c_rgba = state.fill_color;
            if ui
                .color_edit_button_rgba_premultiplied(&mut c_rgba)
                .changed()
            {
                state.fill_color = c_rgba;
                for id in &state.selected_ids {
                    for (_, obj) in state.document.all_objects_mut() {
                        if &obj.id == id {
                            obj.fill = Some(FillStyle::solid(c_rgba));
                        }
                    }
                }
            }
        });

        ui.add_space(6.0);

        // Gradient presets
        ui.label(RichText::new("Gradient Presets").strong().size(11.0));
        ui.horizontal_wrapped(|ui| {
            let presets: Vec<(&str, FillStyle)> = vec![
                (
                    "Sunset",
                    FillStyle::linear_gradient(LinearGradient {
                        start_x: 0.0,
                        start_y: 0.0,
                        end_x: 1.0,
                        end_y: 1.0,
                        stops: vec![
                            GradientStop {
                                offset: 0.0,
                                color: [1.0, 0.3, 0.1, 1.0],
                            },
                            GradientStop {
                                offset: 0.5,
                                color: [1.0, 0.7, 0.0, 1.0],
                            },
                            GradientStop {
                                offset: 1.0,
                                color: [0.8, 0.1, 0.5, 1.0],
                            },
                        ],
                    }),
                ),
                (
                    "Ocean",
                    FillStyle::linear_gradient(LinearGradient {
                        start_x: 0.0,
                        start_y: 0.0,
                        end_x: 0.0,
                        end_y: 1.0,
                        stops: vec![
                            GradientStop {
                                offset: 0.0,
                                color: [0.0, 0.4, 0.8, 1.0],
                            },
                            GradientStop {
                                offset: 1.0,
                                color: [0.0, 0.7, 0.9, 1.0],
                            },
                        ],
                    }),
                ),
                (
                    "Forest",
                    FillStyle::linear_gradient(LinearGradient {
                        start_x: 0.0,
                        start_y: 0.0,
                        end_x: 1.0,
                        end_y: 1.0,
                        stops: vec![
                            GradientStop {
                                offset: 0.0,
                                color: [0.1, 0.4, 0.1, 1.0],
                            },
                            GradientStop {
                                offset: 1.0,
                                color: [0.3, 0.7, 0.2, 1.0],
                            },
                        ],
                    }),
                ),
                (
                    "Radial Glow",
                    FillStyle::radial_gradient(RadialGradient {
                        center_x: 0.5,
                        center_y: 0.5,
                        radius: 0.5,
                        focus_x: 0.5,
                        focus_y: 0.5,
                        stops: vec![
                            GradientStop {
                                offset: 0.0,
                                color: [1.0, 1.0, 1.0, 1.0],
                            },
                            GradientStop {
                                offset: 1.0,
                                color: [0.2, 0.2, 0.8, 1.0],
                            },
                        ],
                    }),
                ),
                (
                    "Fire",
                    FillStyle::linear_gradient(LinearGradient {
                        start_x: 0.5,
                        start_y: 1.0,
                        end_x: 0.5,
                        end_y: 0.0,
                        stops: vec![
                            GradientStop {
                                offset: 0.0,
                                color: [1.0, 0.0, 0.0, 1.0],
                            },
                            GradientStop {
                                offset: 0.5,
                                color: [1.0, 0.5, 0.0, 1.0],
                            },
                            GradientStop {
                                offset: 1.0,
                                color: [1.0, 1.0, 0.0, 1.0],
                            },
                        ],
                    }),
                ),
                (
                    "Purple Haze",
                    FillStyle::radial_gradient(RadialGradient {
                        center_x: 0.5,
                        center_y: 0.5,
                        radius: 0.7,
                        focus_x: 0.3,
                        focus_y: 0.3,
                        stops: vec![
                            GradientStop {
                                offset: 0.0,
                                color: [0.8, 0.2, 0.9, 1.0],
                            },
                            GradientStop {
                                offset: 1.0,
                                color: [0.2, 0.0, 0.5, 1.0],
                            },
                        ],
                    }),
                ),
            ];

            for (name, fill) in presets {
                let preview_color = fill.color;
                let c32 = Color32::from_rgba_unmultiplied(
                    (preview_color[0] * 255.0) as u8,
                    (preview_color[1] * 255.0) as u8,
                    (preview_color[2] * 255.0) as u8,
                    (preview_color[3] * 255.0) as u8,
                );
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(50.0, 20.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 3.0, c32);
                ui.painter().rect_stroke(
                    rect,
                    3.0,
                    egui::Stroke::new(1.0_f32, Color32::from_gray(100)),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(9.0),
                    Color32::WHITE,
                );
                if response.clicked() {
                    for id in &state.selected_ids {
                        for (_, obj) in state.document.all_objects_mut() {
                            if &obj.id == id {
                                obj.fill = Some(fill.clone());
                            }
                        }
                    }
                }
            }
        });
    }
}

pub struct ColorHarmonyPanel;

impl ColorHarmonyPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🎨 Color Harmony").strong());
        ui.add_space(4.0);

        let base = state.fill_color;
        let (h, s, v) = rgb_to_hsv(base[0], base[1], base[2]);

        ui.label(format!(
            "Base: H {:.0}° S {:.0}% V {:.0}%",
            h,
            s * 100.0,
            v * 100.0
        ));

        ui.separator();

        let harmonies: [(&str, Vec<f32>); 5] = [
            ("Complementary", vec![h + 180.0]),
            ("Analogous", vec![h - 30.0, h + 30.0]),
            ("Triadic", vec![h + 120.0, h + 240.0]),
            ("Split-Complementary", vec![h + 150.0, h + 210.0]),
            ("Tetradic", vec![h + 90.0, h + 180.0, h + 270.0]),
        ];

        for (name, offsets) in harmonies {
            ui.label(RichText::new(name).strong().size(11.0));
            ui.horizontal_wrapped(|ui| {
                // Base swatch
                Self::render_color_swatch(ui, state, base);

                for offset in offsets {
                    let nh = offset.rem_euclid(360.0);
                    let (nr, ng, nb) = hsv_to_rgb(nh, s, v);
                    let color = [nr, ng, nb, 1.0];
                    Self::render_color_swatch(ui, state, color);
                }
            });
            ui.add_space(2.0);
        }
    }

    fn render_color_swatch(ui: &mut Ui, state: &mut AppState, color: [f32; 4]) {
        let c32 = Color32::from_rgba_unmultiplied(
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
            (color[3] * 255.0) as u8,
        );
        let (rect, response) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), egui::Sense::click());
        ui.painter().rect_filled(rect, 3.0, c32);
        ui.painter().rect_stroke(
            rect,
            3.0,
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
}

// ═══════════════════════════════════════════════════════════════════
// ShortcutsHelpPanel: Quick-reference for keyboard shortcuts
// ═══════════════════════════════════════════════════════════════════
