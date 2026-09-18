use crate::core::path::{FillStyle, FillType, GradientStop, LinearGradient, RadialGradient};
use crate::core::state::AppState;
use crate::ui::canvas::sample_gradient_stops;
use egui::{Color32, Pos2, Rect, RichText, Stroke, Ui, Vec2};

pub struct GradientPanel;

impl GradientPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌈 Gradient Editor").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select an object to edit gradient").weak());
            return;
        }

        let id = state.selected_ids[0].clone();
        let mut fill_type_name = String::from("Solid");
        let mut linear_stops = Vec::new();
        let mut linear_start = [0.0f32; 2];
        let mut linear_end = [1.0f32; 2];
        let mut radial_radius = 0.5f32;
        let mut radial_stops = Vec::new();
        let mut found = false;

        for (_, obj) in state.document.all_objects() {
            if obj.id == id {
                if let Some(ref f) = obj.fill {
                    match &f.fill_type {
                        FillType::Solid(_) => {
                            fill_type_name = "Solid".into();
                        }
                        FillType::Linear(g) => {
                            fill_type_name = "Linear".into();
                            linear_stops = g.stops.clone();
                            linear_start = [g.start_x, g.start_y];
                            linear_end = [g.end_x, g.end_y];
                        }
                        FillType::Radial(g) => {
                            fill_type_name = "Radial".into();
                            radial_stops = g.stops.clone();
                            radial_radius = g.radius;
                        }
                        FillType::Pattern(_) => {
                            fill_type_name = "Pattern".into();
                        }
                    }
                }
                found = true;
                break;
            }
        }

        if !found {
            return;
        }

        // Fill Type Selector
        ui.horizontal(|ui| {
            ui.label("Type:");
            let mut new_type = fill_type_name.clone();
            for t in ["Solid", "Linear", "Radial"] {
                if ui.selectable_label(fill_type_name == t, t).clicked() {
                    new_type = t.into();
                }
            }
            if new_type != fill_type_name {
                let new_fill = match new_type.as_str() {
                    "Linear" => {
                        let g = LinearGradient {
                            stops: vec![
                                GradientStop {
                                    offset: 0.0,
                                    color: state.fill_color,
                                },
                                GradientStop {
                                    offset: 1.0,
                                    color: [1.0, 1.0, 1.0, 1.0],
                                },
                            ],
                            ..LinearGradient::default()
                        };
                        FillStyle::linear_gradient(g)
                    }
                    "Radial" => {
                        let g = RadialGradient {
                            stops: vec![
                                GradientStop {
                                    offset: 0.0,
                                    color: state.fill_color,
                                },
                                GradientStop {
                                    offset: 1.0,
                                    color: [1.0, 1.0, 1.0, 1.0],
                                },
                            ],
                            ..RadialGradient::default()
                        };
                        FillStyle::radial_gradient(g)
                    }
                    _ => FillStyle::solid(state.fill_color),
                };
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        obj.fill = Some(new_fill.clone());
                    }
                }
            }
        });

        ui.separator();

        // Gradient Stops Editor
        match fill_type_name.as_str() {
            "Linear" => {
                ui.label("Linear Gradient:");

                // Angle
                let dx = linear_end[0] - linear_start[0];
                let dy = linear_end[1] - linear_start[1];
                let mut angle = dy.atan2(dx).to_degrees();
                ui.horizontal(|ui| {
                    ui.label("Angle:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut angle)
                                .speed(1.0)
                                .range(-360.0..=360.0)
                                .suffix("°"),
                        )
                        .changed()
                    {
                        let rad = angle.to_radians();
                        let new_end = [linear_start[0] + rad.cos(), linear_start[1] + rad.sin()];
                        for (_, obj) in state.document.all_objects_mut() {
                            if obj.id == id {
                                if let Some(ref mut f) = obj.fill {
                                    if let FillType::Linear(ref mut g) = f.fill_type {
                                        g.end_x = new_end[0];
                                        g.end_y = new_end[1];
                                    }
                                }
                            }
                        }
                    }
                });

                // Color Stops
                Self::render_stops(ui, state, &id, &mut linear_stops, "Linear");
            }
            "Radial" => {
                ui.label("Radial Gradient:");

                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    let mut r = radial_radius;
                    if ui
                        .add(egui::Slider::new(&mut r, 0.01..=2.0).show_value(true))
                        .changed()
                    {
                        for (_, obj) in state.document.all_objects_mut() {
                            if obj.id == id {
                                if let Some(ref mut f) = obj.fill {
                                    if let FillType::Radial(ref mut g) = f.fill_type {
                                        g.radius = r;
                                    }
                                }
                            }
                        }
                    }
                });

                Self::render_stops(ui, state, &id, &mut radial_stops, "Radial");
            }
            _ => {
                ui.label("Solid fill (no gradient stops)");
            }
        }
    }

    fn render_stops(
        ui: &mut Ui,
        state: &mut AppState,
        obj_id: &str,
        stops: &mut Vec<GradientStop>,
        grad_type: &str,
    ) {
        ui.label(RichText::new("Color Stops:").strong());

        // Visual Gradient Ramp Bar (Illustrator CC style: Image 3)
        let (ramp_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(160.0), 18.0),
            egui::Sense::hover(),
        );
        let p = ui.painter();
        p.rect_filled(ramp_rect, 2.0, Color32::from_rgb(20, 20, 20));
        p.rect_stroke(
            ramp_rect,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgb(70, 70, 70)),
            egui::StrokeKind::Outside,
        );

        let n_slices = 48;
        let slice_w = ramp_rect.width() / n_slices as f32;
        for s in 0..n_slices {
            let t = (s as f32 + 0.5) / n_slices as f32;
            let c = sample_gradient_stops(stops, t);
            let col = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (c[3] * 255.0) as u8,
            );
            let s_rect = Rect::from_min_size(
                Pos2::new(ramp_rect.min.x + s as f32 * slice_w, ramp_rect.min.y + 1.0),
                Vec2::new(slice_w + 0.5, ramp_rect.height() - 2.0),
            );
            p.rect_filled(s_rect, 0.0, col);
        }

        // Stop pins below ramp
        for stop in stops.iter() {
            let pin_x = ramp_rect.min.x + stop.offset * ramp_rect.width();
            let pin_y = ramp_rect.max.y + 4.0;
            p.circle_filled(
                Pos2::new(pin_x, pin_y),
                4.0,
                Color32::from_rgba_unmultiplied(
                    (stop.color[0] * 255.0) as u8,
                    (stop.color[1] * 255.0) as u8,
                    (stop.color[2] * 255.0) as u8,
                    255,
                ),
            );
            p.circle_stroke(
                Pos2::new(pin_x, pin_y),
                4.0,
                Stroke::new(1.0_f32, Color32::WHITE),
            );
        }
        ui.add_space(8.0);

        let mut to_remove = None;
        let stop_count = stops.len();
        let mut color_updates: Vec<(usize, [f32; 4])> = Vec::new();
        let mut offset_updates: Vec<(usize, f32)> = Vec::new();

        for i in 0..stop_count {
            let stop = &stops[i];
            ui.horizontal(|ui| {
                let mut c = stop.color;
                if ui.color_edit_button_rgba_premultiplied(&mut c).changed() {
                    color_updates.push((i, c));
                }

                let mut offset = stop.offset;
                if ui
                    .add(
                        egui::Slider::new(&mut offset, 0.0..=1.0)
                            .show_value(true)
                            .step_by(0.01),
                    )
                    .changed()
                {
                    offset_updates.push((i, offset));
                }

                if stops.len() > 2 && ui.small_button("✕").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        let mut changed = false;
        for &(i, c) in &color_updates {
            stops[i].color = c;
            changed = true;
        }
        for &(i, o) in &offset_updates {
            stops[i].offset = o;
            changed = true;
        }
        if let Some(idx) = to_remove {
            stops.remove(idx);
            changed = true;
        }
        if changed {
            Self::apply_stops(state, obj_id, stops, grad_type);
        }

        // Add stop button
        if ui.button("+ Add Color Stop").clicked() {
            let offset = if stops.len() >= 2 {
                (stops[stops.len() - 2].offset + stops[stops.len() - 1].offset) / 2.0
            } else {
                0.5
            };
            stops.push(GradientStop {
                offset,
                color: state.fill_color,
            });
            // total_cmp: imported files can carry NaN offsets
            // ("NaN".parse::<f32>() succeeds), which would panic unwrap().
            stops.sort_by(|a, b| a.offset.total_cmp(&b.offset));
            Self::apply_stops(state, obj_id, stops, grad_type);
        }
    }

    fn apply_stops(state: &mut AppState, obj_id: &str, stops: &[GradientStop], grad_type: &str) {
        for (_, obj) in state.document.all_objects_mut() {
            if obj.id == obj_id {
                if let Some(ref mut f) = obj.fill {
                    match grad_type {
                        "Linear" => {
                            if let FillType::Linear(ref mut g) = f.fill_type {
                                g.stops = stops.to_vec();
                            }
                        }
                        "Radial" => {
                            if let FillType::Radial(ref mut g) = f.fill_type {
                                g.stops = stops.to_vec();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// BlendModePanel: Object blending modes
// ═══════════════════════════════════════════════════════════════════

pub struct EffectsPanel;

impl EffectsPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("✨ Effects & Shadows").strong());
        ui.add_space(4.0);

        if let Some(id) = state.selected_ids.first().cloned() {
            let mut shadow = None;
            let mut glow = None;

            for (_, obj) in state.document.all_objects() {
                if obj.id == id {
                    shadow = obj.shadow.clone();
                    glow = obj.glow.clone();
                    break;
                }
            }

            let mut has_shadow = shadow.is_some();
            let mut current_shadow = shadow.unwrap_or_default();

            ui.checkbox(&mut has_shadow, "Drop Shadow");
            if has_shadow {
                ui.horizontal(|ui| {
                    ui.label("Offset X:");
                    ui.add(egui::DragValue::new(&mut current_shadow.offset_x).speed(1.0));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut current_shadow.offset_y).speed(1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Blur:");
                    ui.add(
                        egui::DragValue::new(&mut current_shadow.blur_radius)
                            .speed(0.5)
                            .range(0.0..=100.0),
                    );
                    ui.label("Opacity:");
                    ui.add(egui::Slider::new(&mut current_shadow.opacity, 0.0..=1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_rgba_premultiplied(&mut current_shadow.color);
                });
            }

            let mut has_glow = glow.is_some();
            let mut current_glow = glow.unwrap_or_default();

            ui.checkbox(&mut has_glow, "Outer Glow");
            if has_glow {
                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    ui.add(
                        egui::DragValue::new(&mut current_glow.radius)
                            .speed(1.0)
                            .range(1.0..=100.0),
                    );
                    ui.label("Intensity:");
                    ui.add(egui::Slider::new(&mut current_glow.intensity, 0.0..=1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Glow Color:");
                    ui.color_edit_button_rgba_premultiplied(&mut current_glow.color);
                });
            }

            for (_, obj) in state.document.all_objects_mut() {
                if obj.id == id {
                    obj.shadow = if has_shadow {
                        Some(current_shadow)
                    } else {
                        None
                    };
                    obj.glow = if has_glow { Some(current_glow) } else { None };
                    break;
                }
            }
        } else {
            ui.label(RichText::new("Select an object to add effects").weak());
        }
    }
}

pub struct NeonGlowPanel;

impl NeonGlowPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("✨ Vector Neon Glow & Laser").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id)
                .map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(has_sel, egui::Button::new("Cyan Neon"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let neon_layers = crate::core::neon_glow::generate_neon_glow(
                            &path,
                            [0.0, 1.0, 0.9, 1.0],
                            18.0,
                            6,
                        );
                        for layer in neon_layers {
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(layer));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Magenta Neon"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let neon_layers = crate::core::neon_glow::generate_neon_glow(
                            &path,
                            [1.0, 0.1, 0.7, 1.0],
                            18.0,
                            6,
                        );
                        for layer in neon_layers {
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(layer));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Gold Laser"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let neon_layers = crate::core::neon_glow::generate_neon_glow(
                            &path,
                            [1.0, 0.8, 0.1, 1.0],
                            18.0,
                            6,
                        );
                        for layer in neon_layers {
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(layer));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                }
            });
        } else {
            ui.label(
                RichText::new("Select an object to generate vector neon halo")
                    .weak()
                    .size(11.0),
            );
        }
    }
}
