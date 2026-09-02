use crate::core::boolean::{execute_pathfinder, BooleanOp};
use crate::core::document::{BlendMode, ObjectType, Object};
use crate::core::morph::morph_paths;
use crate::core::offset::{offset_path, outline_stroke};
use crate::core::path::{ArrowHead, FillStyle, FillType, GradientStop, LinearGradient, RadialGradient, StrokeCap, StrokeJoin, StrokeStyle};
use crate::core::state::{AppState, Tool};
use egui::{Color32, RichText, Ui, Vec2};

pub struct PropertyPanel;

impl PropertyPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🎨 Properties").strong());
        ui.add_space(4.0);

        // Tool Specific Defaults
        ui.collapsing("Tool Options", |ui| {
            match state.current_tool {
                Tool::Rectangle => {
                    ui.horizontal(|ui| {
                        ui.label("Corner Radius:");
                        ui.add(egui::DragValue::new(&mut state.corner_radius).speed(1.0).range(0.0..=200.0));
                    });
                }
                Tool::Star => {
                    ui.horizontal(|ui| {
                        ui.label("Points:");
                        ui.add(egui::DragValue::new(&mut state.star_points).speed(1.0).range(3..=32));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Inner Ratio:");
                        ui.add(egui::Slider::new(&mut state.star_inner_ratio, 0.1..=0.9).step_by(0.05));
                    });
                }
                Tool::Polygon => {
                    ui.horizontal(|ui| {
                        ui.label("Sides:");
                        ui.add(egui::DragValue::new(&mut state.polygon_sides).speed(1.0).range(3..=24));
                    });
                }
                Tool::Text => {
                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        ui.add(egui::DragValue::new(&mut state.font_size).speed(1.0).range(8.0..=256.0));
                    });
                    ui.label("Default Text:");
                    ui.text_edit_singleline(&mut state.text_input_buf);
                }
                _ => {
                    ui.label("Configure tool specific parameters here.");
                }
            }
        });

        ui.add_space(4.0);
        ui.separator();

        // Appearance
        ui.label(RichText::new("Appearance").strong());
        
        // Fill section
        ui.horizontal(|ui| {
            ui.label("Fill:");
            let mut fill = state.fill_color;
            if ui.color_edit_button_rgba_premultiplied(&mut fill).changed() {
                state.fill_color = fill;
                for id in &state.selected_ids {
                    for (_, obj) in state.document.all_objects_mut() {
                        if &obj.id == id {
                            obj.fill = Some(FillStyle::solid(fill));
                        }
                    }
                }
            }
        });

        // Swatches
        render_swatches(ui, state);

        // Stroke section
        ui.horizontal(|ui| {
            ui.label("Stroke:");
            let mut stroke = state.stroke_color;
            if ui.color_edit_button_rgba_premultiplied(&mut stroke).changed() {
                state.stroke_color = stroke;
                for id in &state.selected_ids {
                    for (_, obj) in state.document.all_objects_mut() {
                        if &obj.id == id {
                            if let Some(ref mut s) = obj.stroke {
                                s.color = stroke;
                            } else {
                                obj.stroke = Some(StrokeStyle {
                                    color: stroke,
                                    width: state.stroke_width,
                                    dash_pattern: None,
                                    ..StrokeStyle::default()
                                });
                            }
                        }
                    }
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Stroke Width:");
            if ui.add(egui::DragValue::new(&mut state.stroke_width).speed(0.2).range(0.0..=100.0)).changed() {
                for id in &state.selected_ids {
                    for (_, obj) in state.document.all_objects_mut() {
                        if &obj.id == id {
                            if let Some(ref mut s) = obj.stroke {
                                s.width = state.stroke_width;
                            }
                        }
                    }
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Opacity:");
            if ui.add(egui::Slider::new(&mut state.opacity, 0.0..=1.0).show_value(true)).changed() {
                for id in &state.selected_ids {
                    for (_, obj) in state.document.all_objects_mut() {
                        if &obj.id == id {
                            obj.opacity = state.opacity;
                        }
                    }
                }
            }
        });

        ui.separator();

        // Selected Object Details
        if let Some(id) = state.selected_ids.first().cloned() {
            ui.label(RichText::new("Selected Object").strong());
            let mut tx = 0.0;
            let mut ty = 0.0;
            let mut sx = 1.0;
            let mut sy = 1.0;
            let mut rot = 0.0;
            let mut vis = true;
            let mut lck = false;
            let mut opac = 1.0;
            let mut obj_name = String::new();
            let mut found = false;

            for (_, obj) in state.document.all_objects() {
                if obj.id == id {
                    tx = obj.transform.x;
                    ty = obj.transform.y;
                    sx = obj.transform.scale_x;
                    sy = obj.transform.scale_y;
                    rot = obj.transform.rotation.to_degrees();
                    vis = obj.visible;
                    lck = obj.locked;
                    opac = obj.opacity;
                    obj_name = obj.name.clone();
                    found = true;
                    break;
                }
            }

            if found {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut obj_name);
                });

                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut tx).speed(1.0));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut ty).speed(1.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Scale X:");
                    ui.add(egui::DragValue::new(&mut sx).speed(0.01));
                    ui.label("Scale Y:");
                    ui.add(egui::DragValue::new(&mut sy).speed(0.01));
                });

                ui.horizontal(|ui| {
                    ui.label("Rotation:");
                    ui.add(egui::DragValue::new(&mut rot).speed(1.0).suffix("°"));
                });

                // Specific object properties (e.g. Text, Rectangle)
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        match &mut obj.object_type {
                            ObjectType::Text { text, font_size } => {
                                ui.separator();
                                ui.label("Text Content:");
                                ui.text_edit_multiline(text);
                                ui.horizontal(|ui| {
                                    ui.label("Font Size:");
                                    ui.add(egui::DragValue::new(font_size).speed(1.0).range(6.0..=300.0));
                                });
                            }
                            ObjectType::Rectangle { width, height, corner_radius } => {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("W:");
                                    ui.add(egui::DragValue::new(width).speed(1.0).range(1.0..=10000.0));
                                    ui.label("H:");
                                    ui.add(egui::DragValue::new(height).speed(1.0).range(1.0..=10000.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Corner Radius:");
                                    ui.add(egui::DragValue::new(corner_radius).speed(1.0).range(0.0..=500.0));
                                });
                            }
                            _ => {}
                        }
                        break;
                    }
                }

                ui.horizontal(|ui| {
                    ui.checkbox(&mut vis, "Visible");
                    ui.checkbox(&mut lck, "Locked");
                });

                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        obj.name = obj_name;
                        obj.transform.x = tx;
                        obj.transform.y = ty;
                        obj.transform.scale_x = sx;
                        obj.transform.scale_y = sy;
                        obj.transform.rotation = rot.to_radians();
                        obj.visible = vis;
                        obj.locked = lck;
                        obj.opacity = opac;
                        break;
                    }
                }
            }
        }
    }
}

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
                    ui.add(egui::DragValue::new(&mut current_shadow.blur_radius).speed(0.5).range(0.0..=100.0));
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
                    ui.add(egui::DragValue::new(&mut current_glow.radius).speed(1.0).range(1.0..=100.0));
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
                    obj.shadow = if has_shadow { Some(current_shadow) } else { None };
                    obj.glow = if has_glow { Some(current_glow) } else { None };
                    break;
                }
            }
        } else {
            ui.label(RichText::new("Select an object to add effects").weak());
        }
    }
}

pub struct OffsetPanel;

impl OffsetPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📐 Path Tools").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        ui.horizontal(|ui| {
            if ui.add_enabled(has_sel, egui::Button::new("Outline Stroke")).clicked() {
                let targets: Vec<crate::core::document::Object> = state
                    .selected_ids
                    .iter()
                    .filter_map(|id| state.document.all_objects().find(|(_, o)| &o.id == id).map(|(_, o)| o.clone()))
                    .collect();

                for obj in targets {
                    let stroke_w = obj.stroke.as_ref().map(|s| s.width).unwrap_or(2.0);
                    let path = obj.to_path_data();
                    let outlined = outline_stroke(&path, stroke_w);
                    let mut new_obj = Object::new_path(&format!("{} (Outlined)", obj.name), outlined);
                    new_obj.transform = obj.transform.clone();
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.add_enabled(has_sel, egui::Button::new("Offset Path (+10px)")).clicked() {
                let targets: Vec<crate::core::document::Object> = state
                    .selected_ids
                    .iter()
                    .filter_map(|id| state.document.all_objects().find(|(_, o)| &o.id == id).map(|(_, o)| o.clone()))
                    .collect();

                for obj in targets {
                    let path = obj.to_path_data();
                    let offset = offset_path(&path, 10.0);
                    let mut new_obj = Object::new_path(&format!("{} (Offset)", obj.name), offset);
                    new_obj.transform = obj.transform.clone();
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }
        });
    }
}

pub struct MorphPanel;

impl MorphPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🧬 Shape Morphing").strong());
        ui.add_space(4.0);

        let sel_count = state.selected_ids.len();
        if sel_count == 2 {
            ui.label("Select 2 shapes to interpolate/morph between them:");
            let mut t = 0.5;
            ui.add(egui::Slider::new(&mut t, 0.0..=1.0).text("Morph (t)").step_by(0.05));

            if ui.button("Create Morphed In-between Shape").clicked() {
                let obj1 = state.document.all_objects().find(|(_, o)| o.id == state.selected_ids[0]).map(|(_, o)| o.clone());
                let obj2 = state.document.all_objects().find(|(_, o)| o.id == state.selected_ids[1]).map(|(_, o)| o.clone());

                if let (Some(o1), Some(o2)) = (obj1, obj2) {
                    let mut path1 = o1.to_path_data();
                    path1.transform(&o1.transform.matrix());
                    let mut path2 = o2.to_path_data();
                    path2.transform(&o2.transform.matrix());

                    let morphed = morph_paths(&path1, &path2, t);
                    let new_obj = Object::new_path("Morph In-Between", morphed);
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }
        } else {
            ui.label(RichText::new("Select exactly 2 objects to morph").weak());
        }
    }
}

pub struct PathfinderPanel;

impl PathfinderPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("✂ Pathfinder").strong());
        ui.label(RichText::new("Combine 2 or more vector shapes").weak().size(11.0));
        ui.add_space(4.0);

        let sel_count = state.selected_ids.len();
        let is_enabled = sel_count >= 2;

        let ops = [
            (BooleanOp::Union, "Unite", "Combine all shapes into one"),
            (BooleanOp::Subtract, "Minus Front", "Subtract front shapes from back"),
            (BooleanOp::Intersect, "Intersect", "Keep overlapping area"),
            (BooleanOp::Exclude, "Exclude", "Exclude overlapping area"),
        ];

        ui.horizontal_wrapped(|ui| {
            for (op, name, tooltip) in ops {
                let btn = egui::Button::new(RichText::new(format!("{} {}", op.icon(), name)).strong())
                    .min_size(Vec2::new(95.0, 26.0));

                let clicked = ui.add_enabled(is_enabled, btn).on_hover_text(tooltip).clicked();

                if clicked {
                    Self::apply_op(state, op);
                }
            }
        });
    }

    fn apply_op(state: &mut AppState, op: BooleanOp) {
        let mut selected_objs: Vec<Object> = Vec::new();
        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                selected_objs.push(obj.clone());
            }
        }

        if selected_objs.len() < 2 {
            return;
        }

        let obj_refs: Vec<&Object> = selected_objs.iter().collect();
        if let Some(result_obj) = execute_pathfinder(&obj_refs, op) {
            for id in &state.selected_ids {
                state.document.remove_object(id);
            }
            let new_id = result_obj.id.clone();
            let cmd = Box::new(crate::core::history::AddObjectCommand::new(result_obj));
            state.undo_manager.execute(cmd, &mut state.document);
            state.selected_ids = vec![new_id];
        }
    }
}

pub struct AlignPanel;

impl AlignPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("⇲ Align & Transform").strong());
        ui.add_space(4.0);

        let sel = state.selected_ids.clone();
        let multi = sel.len() >= 2;

        ui.label(RichText::new("Align:").weak().size(11.0));
        ui.horizontal(|ui| {
            if ui.add_enabled(multi, egui::Button::new("⇤ Left")).clicked() {
                align_left(state, &sel);
            }
            if ui.add_enabled(multi, egui::Button::new("⇹ Center H")).clicked() {
                align_center_h(state, &sel);
            }
            if ui.add_enabled(multi, egui::Button::new("⇥ Right")).clicked() {
                align_right(state, &sel);
            }
        });

        ui.horizontal(|ui| {
            if ui.add_enabled(multi, egui::Button::new("⤒ Top")).clicked() {
                align_top(state, &sel);
            }
            if ui.add_enabled(multi, egui::Button::new("⇕ Center V")).clicked() {
                align_center_v(state, &sel);
            }
            if ui.add_enabled(multi, egui::Button::new("⤓ Bottom")).clicked() {
                align_bottom(state, &sel);
            }
        });

        ui.add_space(4.0);
        ui.label(RichText::new("Distribute:").weak().size(11.0));
        ui.horizontal(|ui| {
            if ui.add_enabled(sel.len() >= 3, egui::Button::new("⬌ Distribute H")).clicked() {
                distribute_h(state, &sel);
            }
            if ui.add_enabled(sel.len() >= 3, egui::Button::new("⬍ Distribute V")).clicked() {
                distribute_v(state, &sel);
            }
        });

        ui.add_space(4.0);
        ui.label(RichText::new("Flip & Arrange:").weak().size(11.0));
        ui.horizontal(|ui| {
            if ui.add_enabled(!sel.is_empty(), egui::Button::new("⇆ Flip H")).clicked() {
                for id in &sel {
                    for (_, obj) in state.document.all_objects_mut() {
                        if &obj.id == id {
                            obj.transform.scale_x *= -1.0;
                        }
                    }
                }
            }
            if ui.add_enabled(!sel.is_empty(), egui::Button::new("⇅ Flip V")).clicked() {
                for id in &sel {
                    for (_, obj) in state.document.all_objects_mut() {
                        if &obj.id == id {
                            obj.transform.scale_y *= -1.0;
                        }
                    }
                }
            }
        });

        ui.add_space(4.0);
        ui.label(RichText::new("Arrange:").weak().size(11.0));
        ui.horizontal(|ui| {
            let has_sel = !sel.is_empty();
            if ui.add_enabled(has_sel, egui::Button::new("⬍ To Front")).on_hover_text("Ctrl+Shift+]").clicked() {
                for id in &sel {
                    for layer in state.document.layers.iter_mut() {
                        if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                            let obj = layer.objects.remove(pos);
                            layer.objects.push(obj);
                            break;
                        }
                    }
                }
            }
            if ui.add_enabled(has_sel, egui::Button::new("↑ Forward")).on_hover_text("Ctrl+]").clicked() {
                for id in &sel {
                    for layer in state.document.layers.iter_mut() {
                        if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                            if pos + 1 < layer.objects.len() {
                                layer.objects.swap(pos, pos + 1);
                            }
                            break;
                        }
                    }
                }
            }
            if ui.add_enabled(has_sel, egui::Button::new("↓ Backward")).on_hover_text("Ctrl+[").clicked() {
                for id in &sel {
                    for layer in state.document.layers.iter_mut() {
                        if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                            if pos > 0 {
                                layer.objects.swap(pos, pos - 1);
                            }
                            break;
                        }
                    }
                }
            }
            if ui.add_enabled(has_sel, egui::Button::new("⬌ To Back")).on_hover_text("Ctrl+Shift+[").clicked() {
                for id in &sel {
                    for layer in state.document.layers.iter_mut() {
                        if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                            let obj = layer.objects.remove(pos);
                            layer.objects.insert(0, obj);
                            break;
                        }
                    }
                }
            }
        });
    }
}

fn align_left(state: &mut AppState, sel: &[String]) {
    let mut min_x = f64::MAX;
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, _)) = obj.bounding_box() {
                min_x = min_x.min(bb_min.x);
            }
        }
    }
    for id in sel {
        for (_, obj) in state.document.all_objects_mut() {
            if &obj.id == id {
                if let Some((bb_min, _)) = obj.bounding_box() {
                    obj.transform.x += min_x - bb_min.x;
                }
                break;
            }
        }
    }
}

fn align_center_h(state: &mut AppState, sel: &[String]) {
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                min_x = min_x.min(bb_min.x);
                max_x = max_x.max(bb_max.x);
            }
        }
    }
    let center = (min_x + max_x) / 2.0;
    for id in sel {
        for (_, obj) in state.document.all_objects_mut() {
            if &obj.id == id {
                if let Some((bb_min, bb_max)) = obj.bounding_box() {
                    let obj_center = (bb_min.x + bb_max.x) / 2.0;
                    obj.transform.x += center - obj_center;
                }
                break;
            }
        }
    }
}

fn align_right(state: &mut AppState, sel: &[String]) {
    let mut max_x = f64::MIN;
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((_, bb_max)) = obj.bounding_box() {
                max_x = max_x.max(bb_max.x);
            }
        }
    }
    for id in sel {
        for (_, obj) in state.document.all_objects_mut() {
            if &obj.id == id {
                if let Some((_, bb_max)) = obj.bounding_box() {
                    obj.transform.x += max_x - bb_max.x;
                }
                break;
            }
        }
    }
}

fn align_top(state: &mut AppState, sel: &[String]) {
    let mut min_y = f64::MAX;
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, _)) = obj.bounding_box() {
                min_y = min_y.min(bb_min.y);
            }
        }
    }
    for id in sel {
        for (_, obj) in state.document.all_objects_mut() {
            if &obj.id == id {
                if let Some((bb_min, _)) = obj.bounding_box() {
                    obj.transform.y += min_y - bb_min.y;
                }
                break;
            }
        }
    }
}

fn align_center_v(state: &mut AppState, sel: &[String]) {
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                min_y = min_y.min(bb_min.y);
                max_y = max_y.max(bb_max.y);
            }
        }
    }
    let center = (min_y + max_y) / 2.0;
    for id in sel {
        for (_, obj) in state.document.all_objects_mut() {
            if &obj.id == id {
                if let Some((bb_min, bb_max)) = obj.bounding_box() {
                    let obj_center = (bb_min.y + bb_max.y) / 2.0;
                    obj.transform.y += center - obj_center;
                }
                break;
            }
        }
    }
}

fn align_bottom(state: &mut AppState, sel: &[String]) {
    let mut max_y = f64::MIN;
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((_, bb_max)) = obj.bounding_box() {
                max_y = max_y.max(bb_max.y);
            }
        }
    }
    for id in sel {
        for (_, obj) in state.document.all_objects_mut() {
            if &obj.id == id {
                if let Some((_, bb_max)) = obj.bounding_box() {
                    obj.transform.y += max_y - bb_max.y;
                }
                break;
            }
        }
    }
}

fn distribute_h(state: &mut AppState, sel: &[String]) {
    let mut items: Vec<(String, f64, f64)> = Vec::new();
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                items.push((id.clone(), bb_min.x, bb_max.x - bb_min.x));
            }
        }
    }
    if items.len() < 3 {
        return;
    }
    items.sort_by(|a, b| a.1.total_cmp(&b.1));

    let first_min = items.first().unwrap().1;
    let last_max = items.last().unwrap().1 + items.last().unwrap().2;
    let total_width: f64 = items.iter().map(|it| it.2).sum();
    let total_gap = (last_max - first_min) - total_width;
    let gap = total_gap / (items.len() - 1) as f64;

    let mut current_pos = first_min;
    for (id, orig_min, w) in items {
        let delta = current_pos - orig_min;
        for (_, obj) in state.document.all_objects_mut() {
            if obj.id == id {
                obj.transform.x += delta;
                break;
            }
        }
        current_pos += w + gap;
    }
}

fn distribute_v(state: &mut AppState, sel: &[String]) {
    let mut items: Vec<(String, f64, f64)> = Vec::new();
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                items.push((id.clone(), bb_min.y, bb_max.y - bb_min.y));
            }
        }
    }
    if items.len() < 3 {
        return;
    }
    items.sort_by(|a, b| a.1.total_cmp(&b.1));

    let first_min = items.first().unwrap().1;
    let last_max = items.last().unwrap().1 + items.last().unwrap().2;
    let total_height: f64 = items.iter().map(|it| it.2).sum();
    let total_gap = (last_max - first_min) - total_height;
    let gap = total_gap / (items.len() - 1) as f64;

    let mut current_pos = first_min;
    for (id, orig_min, h) in items {
        let delta = current_pos - orig_min;
        for (_, obj) in state.document.all_objects_mut() {
            if obj.id == id {
                obj.transform.y += delta;
                break;
            }
        }
        current_pos += h + gap;
    }
}

fn render_swatches(ui: &mut Ui, state: &mut AppState) {
    let swatches: [[f32; 4]; 12] = [
        [0.0, 0.0, 0.0, 1.0],       // Black
        [1.0, 1.0, 1.0, 1.0],       // White
        [0.9, 0.2, 0.2, 1.0],       // Red
        [0.95, 0.6, 0.1, 1.0],      // Orange
        [0.95, 0.85, 0.15, 1.0],    // Yellow
        [0.2, 0.8, 0.3, 1.0],       // Green
        [0.1, 0.7, 0.9, 1.0],       // Cyan
        [0.2, 0.5, 0.9, 1.0],       // Blue
        [0.6, 0.25, 0.85, 1.0],     // Purple
        [0.9, 0.3, 0.6, 1.0],       // Pink
        [0.55, 0.35, 0.2, 1.0],     // Brown
        [0.5, 0.55, 0.6, 1.0],      // Gray
    ];

    ui.horizontal_wrapped(|ui| {
        for color in swatches {
            let c32 = Color32::from_rgba_unmultiplied(
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                (color[3] * 255.0) as u8,
            );
            let (rect, response) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 2.0_f32, c32);
            ui.painter().rect_stroke(rect, 2.0_f32, egui::Stroke::new(1.0_f32, Color32::from_gray(80)), egui::StrokeKind::Inside);

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

// ═══════════════════════════════════════════════════════════════════
// StrokePanel: Dash pattern, Cap, Join, Arrowheads
// ═══════════════════════════════════════════════════════════════════

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
                        dash_pattern = dashes.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", ");
                    }
                }
                found = true;
                break;
            }
        }

        if !found { return; }

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
            if ui.add(egui::DragValue::new(&mut w).speed(0.5).range(0.0..=200.0).suffix("px")).changed() {
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
                if ui.selectable_label(new_cap == cap_type, cap_type.name()).clicked() {
                    new_cap = cap_type;
                }
            }
            if new_cap != cap {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke { s.cap = new_cap; }
                    }
                }
            }
        });

        // Join
        ui.horizontal(|ui| {
            ui.label("Join:");
            let mut new_join = join;
            for join_type in [StrokeJoin::Miter, StrokeJoin::Round, StrokeJoin::Bevel] {
                if ui.selectable_label(new_join == join_type, join_type.name()).clicked() {
                    new_join = join_type;
                }
            }
            if new_join != join {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke { s.join = new_join; }
                    }
                }
            }
        });

        // Miter Limit
        if join == StrokeJoin::Miter {
            ui.horizontal(|ui| {
                ui.label("Miter Limit:");
                let mut ml = miter_limit;
                if ui.add(egui::DragValue::new(&mut ml).speed(0.5).range(1.0..=100.0)).changed() {
                    for (_, obj) in state.document.all_objects_mut() {
                        if obj.id == id {
                            if let Some(ref mut s) = obj.stroke { s.miter_limit = ml; }
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
                    Some(dp.split(',').filter_map(|s| s.trim().parse().ok()).collect())
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
        ui.label(RichText::new("Comma-separated (e.g. 10, 5)").weak().size(10.0));

        ui.separator();

        // Arrowheads
        ui.label("Arrowheads:");
        ui.horizontal(|ui| {
            ui.label("Start:");
            let mut new_as = arrow_start;
            for ah in [ArrowHead::None, ArrowHead::Triangle, ArrowHead::Arrow, ArrowHead::Circle, ArrowHead::Diamond, ArrowHead::Square, ArrowHead::Barbed] {
                if ui.selectable_label(new_as == ah, ah.name()).clicked() {
                    new_as = ah;
                }
            }
            if new_as != arrow_start {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke { s.arrow_start = new_as; }
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("End:");
            let mut new_ae = arrow_end;
            for ah in [ArrowHead::None, ArrowHead::Triangle, ArrowHead::Arrow, ArrowHead::Circle, ArrowHead::Diamond, ArrowHead::Square, ArrowHead::Barbed] {
                if ui.selectable_label(new_ae == ah, ah.name()).clicked() {
                    new_ae = ah;
                }
            }
            if new_ae != arrow_end {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke { s.arrow_end = new_ae; }
                    }
                }
            }
        });
    }
}

// ═══════════════════════════════════════════════════════════════════
// GradientPanel: Linear/Radial gradient editing with color stops
// ═══════════════════════════════════════════════════════════════════

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
                        FillType::Solid(_) => { fill_type_name = "Solid".into(); }
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
                        FillType::Pattern(_) => { fill_type_name = "Pattern".into(); }
                    }
                }
                found = true;
                break;
            }
        }

        if !found { return; }

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
                                GradientStop { offset: 0.0, color: state.fill_color },
                                GradientStop { offset: 1.0, color: [1.0, 1.0, 1.0, 1.0] },
                            ],
                            ..LinearGradient::default()
                        };
                        FillStyle::linear_gradient(g)
                    }
                    "Radial" => {
                        let g = RadialGradient {
                            stops: vec![
                                GradientStop { offset: 0.0, color: state.fill_color },
                                GradientStop { offset: 1.0, color: [1.0, 1.0, 1.0, 1.0] },
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
                    if ui.add(egui::DragValue::new(&mut angle).speed(1.0).range(-360.0..=360.0).suffix("°")).changed() {
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
                    if ui.add(egui::Slider::new(&mut r, 0.01..=2.0).show_value(true)).changed() {
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

    fn render_stops(ui: &mut Ui, state: &mut AppState, obj_id: &str, stops: &mut Vec<GradientStop>, grad_type: &str) {
        ui.label(RichText::new("Color Stops:").strong());

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
                if ui.add(egui::Slider::new(&mut offset, 0.0..=1.0).show_value(true).step_by(0.01)).changed() {
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
            stops.push(GradientStop { offset, color: state.fill_color });
            stops.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
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

pub struct TransformPanel;

impl TransformPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📐 Transform").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select an object to transform").weak());
            return;
        }

        let id = state.selected_ids[0].clone();
        let mut tx = 0.0f64;
        let mut ty = 0.0f64;
        let mut sx = 1.0f64;
        let mut sy = 1.0f64;
        let mut rot = 0.0f64;
        let mut skew_x = 0.0f64;
        let mut skew_y = 0.0f64;
        let mut found = false;

        for (_, obj) in state.document.all_objects() {
            if obj.id == id {
                tx = obj.transform.x;
                ty = obj.transform.y;
                sx = obj.transform.scale_x;
                sy = obj.transform.scale_y;
                rot = obj.transform.rotation.to_degrees();
                skew_x = obj.transform.skew_x;
                skew_y = obj.transform.skew_y;
                if let Some((bb_min, bb_max)) = obj.bounding_box() {
                    let _w = bb_max.x - bb_min.x;
                    let _h = bb_max.y - bb_min.y;
                }
                found = true;
                break;
            }
        }

        if !found { return; }

        // Position
        ui.collapsing("Position", |ui| {
            ui.horizontal(|ui| {
                ui.label("X:");
                if ui.add(egui::DragValue::new(&mut tx).speed(1.0)).changed() {
                    Self::set_transform(state, &id, |t| t.x = tx);
                }
                ui.label("Y:");
                if ui.add(egui::DragValue::new(&mut ty).speed(1.0)).changed() {
                    Self::set_transform(state, &id, |t| t.y = ty);
                }
            });
        });

        // Scale
        ui.collapsing("Scale", |ui| {
            ui.horizontal(|ui| {
                ui.label("W:");
                if ui.add(egui::DragValue::new(&mut sx).speed(0.01).range(0.001..=100.0)).changed() {
                    Self::set_transform(state, &id, |t| t.scale_x = sx);
                }
                ui.label("H:");
                if ui.add(egui::DragValue::new(&mut sy).speed(0.01).range(0.001..=100.0)).changed() {
                    Self::set_transform(state, &id, |t| t.scale_y = sy);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Lock Aspect").clicked() {
                    let avg = (sx + sy) / 2.0;
                    Self::set_transform(state, &id, |t| { t.scale_x = avg; t.scale_y = avg; });
                }
                if ui.button("Reset Scale").clicked() {
                    Self::set_transform(state, &id, |t| { t.scale_x = 1.0; t.scale_y = 1.0; });
                }
            });
        });

        // Rotation
        ui.collapsing("Rotation", |ui| {
            ui.horizontal(|ui| {
                ui.label("°");
                if ui.add(egui::DragValue::new(&mut rot).speed(1.0).range(-360.0..=360.0).suffix("°")).changed() {
                    Self::set_transform(state, &id, |t| t.rotation = rot.to_radians());
                }
            });
            ui.horizontal(|ui| {
                if ui.button("0°").clicked() {
                    Self::set_transform(state, &id, |t| t.rotation = 0.0);
                }
                if ui.button("45°").clicked() {
                    Self::set_transform(state, &id, |t| t.rotation = std::f64::consts::FRAC_PI_4);
                }
                if ui.button("90°").clicked() {
                    Self::set_transform(state, &id, |t| t.rotation = std::f64::consts::FRAC_PI_2);
                }
                if ui.button("180°").clicked() {
                    Self::set_transform(state, &id, |t| t.rotation = std::f64::consts::PI);
                }
            });
        });

        // Skew
        ui.collapsing("Skew", |ui| {
            ui.horizontal(|ui| {
                ui.label("Skew X:");
                if ui.add(egui::DragValue::new(&mut skew_x).speed(1.0).range(-89.0..=89.0).suffix("°")).changed() {
                    Self::set_transform(state, &id, |t| t.skew_x = skew_x);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Skew Y:");
                if ui.add(egui::DragValue::new(&mut skew_y).speed(1.0).range(-89.0..=89.0).suffix("°")).changed() {
                    Self::set_transform(state, &id, |t| t.skew_y = skew_y);
                }
            });
        });

        // Quick actions
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Flip H").clicked() {
                Self::set_transform(state, &id, |t| t.scale_x = -t.scale_x);
            }
            if ui.button("Flip V").clicked() {
                Self::set_transform(state, &id, |t| t.scale_y = -t.scale_y);
            }
            if ui.button("Reset All").clicked() {
                Self::set_transform(state, &id, |t| {
                    t.x = 0.0; t.y = 0.0;
                    t.scale_x = 1.0; t.scale_y = 1.0;
                    t.rotation = 0.0;
                    t.skew_x = 0.0; t.skew_y = 0.0;
                });
            }
        });
    }

    fn set_transform(state: &mut AppState, obj_id: &str, f: impl FnOnce(&mut crate::core::document::Transform)) {
        for (_, obj) in state.document.all_objects_mut() {
            if obj.id == obj_id {
                f(&mut obj.transform);
                break;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// HSV helper functions
// ═══════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════
// ClippingMaskPanel: Create/manage clipping masks
// ═══════════════════════════════════════════════════════════════════

pub struct ClippingMaskPanel;

impl ClippingMaskPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("✂ Clipping Mask").strong());
        ui.add_space(4.0);

        let sel_count = state.selected_ids.len();
        let has_mask_shape = sel_count >= 2;

        ui.label("Select a mask shape (top) and content objects (below).");
        ui.label(RichText::new("Ctrl+7 or click below to create mask").weak().size(11.0));
        ui.add_space(4.0);

        if ui.add_enabled(has_mask_shape, egui::Button::new("Create Clipping Mask")).clicked() {
            // The first selected object is the mask, rest are content
            let mask_id = state.selected_ids[0].clone();
            let content_ids: Vec<String> = state.selected_ids[1..].to_vec();

            let mut mask_obj = None;
            let mut content_objs = Vec::new();
            let mut ids_to_remove = Vec::new();

            for (_, obj) in state.document.all_objects() {
                if obj.id == mask_id {
                    mask_obj = Some(obj.clone());
                } else if content_ids.contains(&obj.id) {
                    content_objs.push(obj.clone());
                }
            }

            if let Some(mask) = mask_obj {
                ids_to_remove.push(mask_id.clone());
                ids_to_remove.extend(content_ids.clone());

                let mask_path = mask.to_path_data();
                let mut children = vec![Object::new_path("Mask", mask_path)];
                children.extend(content_objs);

                let clipping = Object {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "Clipping Mask".into(),
                    object_type: ObjectType::ClippingMask { children },
                    ..Object::new_rect("Clipping Mask", 0.0, 0.0, 100.0, 100.0, 0.0)
                };

                for remove_id in &ids_to_remove {
                    state.document.remove_object(remove_id);
                }

                let cmd = Box::new(crate::core::history::AddObjectCommand::new(clipping));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        }

        ui.add_space(4.0);
        ui.label(RichText::new(format!("Selected: {} objects", sel_count)).weak());
    }
}

// ═══════════════════════════════════════════════════════════════════
// AppearancePanel: Multiple fills/strokes per object
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
                if obj.id == id { current = obj.blend_mode; break; }
            }
            egui::ComboBox::from_id_salt("blend_mode_combo").selected_text(current.name()).show_ui(ui, |ui| {
                for mode in BlendMode::all() {
                    if ui.selectable_label(current == *mode, mode.name()).clicked() {
                        for (_, obj) in state.document.all_objects_mut() {
                            if obj.id == id { obj.blend_mode = *mode; }
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
                if obj.id == id { opac = obj.opacity; break; }
            }
            if ui.add(egui::Slider::new(&mut opac, 0.0..=1.0).show_value(true)).changed() {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id { obj.opacity = opac; }
                }
            }
        });

        ui.separator();

        // Fill
        ui.label(RichText::new("Fill").strong());
        ui.horizontal(|ui| {
            let mut fill_color = state.fill_color;
            if ui.color_edit_button_rgba_premultiplied(&mut fill_color).changed() {
                state.fill_color = fill_color;
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        obj.fill = Some(FillStyle::solid(fill_color));
                    }
                }
            }
            if ui.button("No Fill").clicked() {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id { obj.fill = None; }
                }
            }
        });

        ui.separator();

        // Stroke
        ui.label(RichText::new("Stroke").strong());
        ui.horizontal(|ui| {
            let mut stroke_color = state.stroke_color;
            if ui.color_edit_button_rgba_premultiplied(&mut stroke_color).changed() {
                state.stroke_color = stroke_color;
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke {
                            s.color = stroke_color;
                        } else {
                            obj.stroke = Some(StrokeStyle { color: stroke_color, ..StrokeStyle::default() });
                        }
                    }
                }
            }
            if ui.button("No Stroke").clicked() {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id { obj.stroke = None; }
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Width:");
            let mut sw = 1.0;
            for (_, obj) in state.document.all_objects() {
                if obj.id == id {
                    if let Some(ref s) = obj.stroke { sw = s.width; }
                }
            }
            if ui.add(egui::DragValue::new(&mut sw).speed(0.5).range(0.0..=200.0)).changed() {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let Some(ref mut s) = obj.stroke { s.width = sw; }
                    }
                }
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
        ui.label(format!("Shadow: {} | Glow: {}", if has_shadow { "On" } else { "Off" }, if has_glow { "On" } else { "Off" }));
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
                let (rect, response) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 2.0, c32);
                ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0_f32, Color32::from_gray(80)), egui::StrokeKind::Inside);
                if response.clicked() {
                    state.fill_color = color;
                    for id in &state.selected_ids {
                        for (_, obj) in state.document.all_objects_mut() {
                            if &obj.id == id { obj.fill = Some(FillStyle::solid(color)); }
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
                ("Sunset", FillStyle::linear_gradient(LinearGradient {
                    start_x: 0.0, start_y: 0.0, end_x: 1.0, end_y: 1.0,
                    stops: vec![
                        GradientStop { offset: 0.0, color: [1.0, 0.3, 0.1, 1.0] },
                        GradientStop { offset: 0.5, color: [1.0, 0.7, 0.0, 1.0] },
                        GradientStop { offset: 1.0, color: [0.8, 0.1, 0.5, 1.0] },
                    ],
                })),
                ("Ocean", FillStyle::linear_gradient(LinearGradient {
                    start_x: 0.0, start_y: 0.0, end_x: 0.0, end_y: 1.0,
                    stops: vec![
                        GradientStop { offset: 0.0, color: [0.0, 0.4, 0.8, 1.0] },
                        GradientStop { offset: 1.0, color: [0.0, 0.7, 0.9, 1.0] },
                    ],
                })),
                ("Forest", FillStyle::linear_gradient(LinearGradient {
                    start_x: 0.0, start_y: 0.0, end_x: 1.0, end_y: 1.0,
                    stops: vec![
                        GradientStop { offset: 0.0, color: [0.1, 0.4, 0.1, 1.0] },
                        GradientStop { offset: 1.0, color: [0.3, 0.7, 0.2, 1.0] },
                    ],
                })),
                ("Radial Glow", FillStyle::radial_gradient(RadialGradient {
                    center_x: 0.5, center_y: 0.5, radius: 0.5,
                    focus_x: 0.5, focus_y: 0.5,
                    stops: vec![
                        GradientStop { offset: 0.0, color: [1.0, 1.0, 1.0, 1.0] },
                        GradientStop { offset: 1.0, color: [0.2, 0.2, 0.8, 1.0] },
                    ],
                })),
                ("Fire", FillStyle::linear_gradient(LinearGradient {
                    start_x: 0.5, start_y: 1.0, end_x: 0.5, end_y: 0.0,
                    stops: vec![
                        GradientStop { offset: 0.0, color: [1.0, 0.0, 0.0, 1.0] },
                        GradientStop { offset: 0.5, color: [1.0, 0.5, 0.0, 1.0] },
                        GradientStop { offset: 1.0, color: [1.0, 1.0, 0.0, 1.0] },
                    ],
                })),
                ("Purple Haze", FillStyle::radial_gradient(RadialGradient {
                    center_x: 0.5, center_y: 0.5, radius: 0.7,
                    focus_x: 0.3, focus_y: 0.3,
                    stops: vec![
                        GradientStop { offset: 0.0, color: [0.8, 0.2, 0.9, 1.0] },
                        GradientStop { offset: 1.0, color: [0.2, 0.0, 0.5, 1.0] },
                    ],
                })),
            ];

            for (name, fill) in presets {
                let preview_color = fill.color;
                let c32 = Color32::from_rgba_unmultiplied(
                    (preview_color[0] * 255.0) as u8,
                    (preview_color[1] * 255.0) as u8,
                    (preview_color[2] * 255.0) as u8,
                    (preview_color[3] * 255.0) as u8,
                );
                let (rect, response) = ui.allocate_exact_size(Vec2::new(50.0, 20.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 3.0, c32);
                ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0_f32, Color32::from_gray(100)), egui::StrokeKind::Inside);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, name, egui::FontId::proportional(9.0), Color32::WHITE);
                if response.clicked() {
                    for id in &state.selected_ids {
                        for (_, obj) in state.document.all_objects_mut() {
                            if &obj.id == id { obj.fill = Some(fill.clone()); }
                        }
                    }
                }
            }
        });
    }
}

pub struct LayerPanel;

impl LayerPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📑 Layers").strong());
        ui.separator();

        let layer_count = state.document.layers.len();
        let active_idx = state.document.active_layer_idx;

        let mut to_add_layer = false;
        let mut to_remove_layer = false;
        let mut to_duplicate_layer: Option<usize> = None;
        let mut to_select_obj: Option<String> = None;
        let mut to_remove_obj: Option<(usize, usize)> = None;
        let mut to_toggle_vis: Option<usize> = None;
        let mut to_toggle_lock: Option<usize> = None;

        for (i, layer) in state.document.layers.iter().enumerate() {
            let is_active = i == active_idx;
            let obj_count = layer.objects.len();
            let text = if is_active {
                RichText::new(format!("📁 {} ({} objects)", layer.name, obj_count)).strong().color(Color32::from_rgb(100, 180, 255))
            } else {
                RichText::new(format!("📁 {} ({} objects)", layer.name, obj_count))
            };

            ui.horizontal(|ui| {
                if ui.selectable_label(is_active, text).clicked() {
                    state.document.active_layer_idx = i;
                }

                let vis_icon = if layer.visible { "👁" } else { "🚫" };
                if ui.small_button(vis_icon).on_hover_text("Toggle Visibility").clicked() {
                    to_toggle_vis = Some(i);
                }

                let lock_icon = if layer.locked { "🔒" } else { "🔓" };
                if ui.small_button(lock_icon).on_hover_text("Toggle Lock").clicked() {
                    to_toggle_lock = Some(i);
                }

                if ui.small_button("⧉").on_hover_text("Duplicate Layer").clicked() {
                    to_duplicate_layer = Some(i);
                }
            });

            if is_active {
                ui.indent("objects", |ui| {
                    for (j, obj) in layer.objects.iter().enumerate() {
                        let is_selected = state.selected_ids.contains(&obj.id);
                        let icon = match &obj.object_type {
                            ObjectType::Path(_) => "✒",
                            ObjectType::Rectangle { .. } => "▭",
                            ObjectType::Ellipse { .. } => "◯",
                            ObjectType::Star { .. } => "★",
                            ObjectType::Polygon { .. } => "⬡",
                            ObjectType::Line { .. } => "╱",
                            ObjectType::Text { .. } => "𝐓",
                            ObjectType::Group(_) => "🗂",
                            ObjectType::ClippingMask { .. } => "⬛",
                        };
                        let obj_text = format!("{icon} {}", obj.name);

                        ui.horizontal(|ui| {
                            if ui.selectable_label(is_selected, &obj_text).clicked() {
                                to_select_obj = Some(obj.id.clone());
                            }

                            if ui.small_button("×").on_hover_text("Delete").clicked() {
                                to_remove_obj = Some((i, j));
                            }
                        });
                    }
                });
            }
        }

        if let Some(id) = to_select_obj {
            state.selected_ids.clear();
            state.selected_ids.push(id);
        }

        if let Some((layer_idx, obj_idx)) = to_remove_obj {
            let obj = state.document.layers[layer_idx].objects.remove(obj_idx);
            let cmd = Box::new(crate::core::history::RemoveObjectCommand::new(obj, layer_idx, obj_idx));
            state.undo_manager.execute(cmd, &mut state.document);
        }

        if let Some(i) = to_toggle_vis {
            state.document.layers[i].visible = !state.document.layers[i].visible;
        }

        if let Some(i) = to_toggle_lock {
            state.document.layers[i].locked = !state.document.layers[i].locked;
        }

        if let Some(i) = to_duplicate_layer {
            let new_name = format!("{} (copy)", state.document.layers[i].name);
            let mut new_layer = crate::core::document::Layer::new(&new_name);
            for obj in &state.document.layers[i].objects {
                let mut dup = obj.clone();
                dup.id = uuid::Uuid::new_v4().to_string();
                dup.name = format!("{} (copy)", obj.name);
                new_layer.objects.push(dup);
            }
            new_layer.visible = state.document.layers[i].visible;
            new_layer.locked = state.document.layers[i].locked;
            state.document.layers.push(new_layer);
            state.document.active_layer_idx = state.document.layers.len() - 1;
        }

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("+ New Layer").clicked() {
                to_add_layer = true;
            }
            if ui.button("- Delete Layer").clicked() && layer_count > 1 {
                to_remove_layer = true;
            }
        });

        if to_add_layer {
            let name = format!("Layer {}", layer_count + 1);
            state.document.layers.push(crate::core::document::Layer::new(&name));
            state.document.active_layer_idx = state.document.layers.len() - 1;
        }

        if to_remove_layer {
            let idx = state.document.active_layer_idx;
            state.document.layers.remove(idx);
            state.document.active_layer_idx = state.document.active_layer_idx.min(state.document.layers.len() - 1);
        }
    }
}

pub struct HistoryPanel;

impl HistoryPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("↩ History").strong());
        ui.add_space(4.0);

        let undo_depth = state.undo_manager.undo_depth();
        let redo_depth = state.undo_manager.redo_depth();

        ui.label(RichText::new(format!("Undo: {} | Redo: {}", undo_depth, redo_depth)).weak());
        ui.separator();

        if undo_depth == 0 && redo_depth == 0 {
            ui.label(RichText::new("No history yet").weak());
            return;
        }

        ui.collapsing(format!("Undo Stack ({})", undo_depth), |ui| {
            let names: Vec<String> = state.undo_manager.undo_stack().iter().map(|c| c.name().to_string()).collect();
            for (i, name) in names.iter().enumerate() {
                let is_last = i == names.len() - 1;
                let text = if is_last {
                    RichText::new(format!("{}. {} ●", i + 1, name)).strong().color(Color32::from_rgb(100, 180, 255))
                } else {
                    RichText::new(format!("{}. {}", i + 1, name))
                };
                ui.label(text);
            }
        });

        ui.collapsing(format!("Redo Stack ({})", redo_depth), |ui| {
            let names: Vec<String> = state.undo_manager.redo_stack().iter().rev().map(|c| c.name().to_string()).collect();
            for (i, name) in names.iter().enumerate() {
                let text = RichText::new(format!("{}. {}", i + 1, name));
                ui.label(text);
            }
        });
    }
}

pub struct PresetPanel;

impl PresetPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📦 Asset Library & Presets").strong());
        ui.add_space(4.0);

        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        ui.horizontal_wrapped(|ui| {
            if ui.button("❤️ Heart").on_hover_text("Add Heart shape").clicked() {
                let obj = crate::core::presets::PresetLibrary::heart("Heart", cx, cy, 120.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui.button("➡️ Arrow").on_hover_text("Add Arrow symbol").clicked() {
                let obj = crate::core::presets::PresetLibrary::arrow("Arrow", cx, cy, 160.0, 40.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui.button("⚙️ Gear").on_hover_text("Add Cog / Gear").clicked() {
                let obj = crate::core::presets::PresetLibrary::gear("Gear", cx, cy, 8, 40.0, 60.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui.button("💬 Speech").on_hover_text("Add Speech Bubble").clicked() {
                let obj = crate::core::presets::PresetLibrary::speech_bubble("Speech Bubble", cx, cy, 150.0, 100.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui.button("🌀 VFX Portal").on_hover_text("Add Sci-Fi Hexagonal VFX Ring").clicked() {
                let obj = crate::core::presets::PresetLibrary::vfx_portal("VFX Portal", cx, cy, 80.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }
        });
    }
}

pub struct TracePanel;

impl TracePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🖼️ Live Auto-Trace").strong());
        ui.label(RichText::new("Vectorize bitmap into paths").weak().size(11.0));
        ui.add_space(4.0);

        if ui.button("Open Image to Trace (PNG/JPG)...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg", "bmp"])
                .pick_file()
            {
                if let Ok(img) = image::open(&path) {
                    let gray = img.to_luma8();
                    let w = gray.width() as usize;
                    let h = gray.height() as usize;
                    let path_data = crate::core::trace::trace_bitmap_to_path(w, h, gray.as_raw(), 128);
                    let mut obj = Object::new_path(&format!("Traced {}", path.file_stem().and_then(|s| s.to_str()).unwrap_or("Image")), path_data);
                    obj.transform.x = 50.0;
                    obj.transform.y = 50.0;
                    let id = obj.id.clone();
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                    state.selected_ids = vec![id];
                }
            }
        }
    }
}

pub struct FormulaPanel;

impl FormulaPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌀 Math & Formula Curves").strong());
        ui.add_space(4.0);

        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        ui.horizontal_wrapped(|ui| {
            if ui.button("🌀 Spiral").on_hover_text("Archimedean Spiral").clicked() {
                let path = crate::core::formula::FormulaCurves::spiral(cx, cy, 4.0, 5.0, 3.0, 180);
                let obj = Object::new_path("Spiral", path);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui.button("〰️ Lissajous").on_hover_text("Oscilloscope Waveform").clicked() {
                let path = crate::core::formula::FormulaCurves::lissajous(cx, cy, 3.0, 2.0, 0.5, 200.0, 160.0, 240);
                let obj = Object::new_path("Lissajous", path);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui.button("💮 Spirograph").on_hover_text("Geometric Spirograph Pattern").clicked() {
                let path = crate::core::formula::FormulaCurves::spirograph(cx, cy, 100.0, 42.0, 60.0, 8, 48);
                let obj = Object::new_path("Spirograph", path);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui.button("🌸 Rose Curve").on_hover_text("Rhodonea Mathematical Flower").clicked() {
                let path = crate::core::formula::FormulaCurves::rose_curve(cx, cy, 4.0, 90.0, 200);
                let obj = Object::new_path("Rose Curve", path);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }
        });
    }
}

pub struct VfxTrailPanel;

impl VfxTrailPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("⚡ VFX Particle Trails").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            if ui.add_enabled(has_sel, egui::Button::new("Export Particle Trails (.json)...")).clicked() {
                if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| o.id == id) {
                    let mut path = obj.to_path_data();
                    path.transform(&obj.transform.matrix());
                    let particles = crate::core::vfx_particles::generate_particle_trail(&path, 200, 50.0, 10.0);

                    if let Some(save_path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .save_file()
                    {
                        if let Ok(json) = serde_json::to_string_pretty(&particles) {
                            let _ = std::fs::write(&save_path, json);
                        }
                    }
                }
            }
        } else {
            ui.label(RichText::new("Select a path to generate particle trails").weak().size(11.0));
        }
    }
}

pub struct HalftonePanel;

impl HalftonePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🏁 Halftone & Dot Matrix").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui.add_enabled(has_sel, egui::Button::new("Grid Dots")).clicked() {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let ht = crate::core::halftone::generate_halftone_from_path(
                            &path,
                            10.0,
                            4.5,
                            crate::core::halftone::HalftonePattern::CircularGrid,
                        );
                        let mut new_obj = Object::new_path(&format!("{} (Halftone)", obj.name), ht);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Hex Dots")).clicked() {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let ht = crate::core::halftone::generate_halftone_from_path(
                            &path,
                            10.0,
                            4.5,
                            crate::core::halftone::HalftonePattern::HexagonalGrid,
                        );
                        let mut new_obj = Object::new_path(&format!("{} (Hex Halftone)", obj.name), ht);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(RichText::new("Select an object to generate halftone dots").weak().size(11.0));
        }
    }
}

pub struct IsometricPanel;

impl IsometricPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📐 2.5D Isometric Transformer").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui.add_enabled(has_sel, egui::Button::new("Top Plane")).clicked() {
                    if let Some(obj) = &target_obj {
                        let iso = crate::core::isometric::apply_isometric_transform(obj, crate::core::isometric::IsometricPlane::Top);
                        let new_id = iso.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(iso));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Left Plane")).clicked() {
                    if let Some(obj) = &target_obj {
                        let iso = crate::core::isometric::apply_isometric_transform(obj, crate::core::isometric::IsometricPlane::Left);
                        let new_id = iso.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(iso));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Right Plane")).clicked() {
                    if let Some(obj) = &target_obj {
                        let iso = crate::core::isometric::apply_isometric_transform(obj, crate::core::isometric::IsometricPlane::Right);
                        let new_id = iso.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(iso));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(RichText::new("Select an object to project into isometric plane").weak().size(11.0));
        }
    }
}

pub struct SymmetryPanel;

impl SymmetryPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("☸️ Radial Symmetry & Mandala").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            ui.horizontal(|ui| {
                if ui.add_enabled(has_sel, egui::Button::new("4-Fold")).clicked() {
                    Self::apply_sym(state, &id, 4, false);
                }
                if ui.add_enabled(has_sel, egui::Button::new("6-Fold")).clicked() {
                    Self::apply_sym(state, &id, 6, false);
                }
                if ui.add_enabled(has_sel, egui::Button::new("8-Fold Mirror")).clicked() {
                    Self::apply_sym(state, &id, 8, true);
                }
            });
        } else {
            ui.label(RichText::new("Select an object to create symmetry mandala").weak().size(11.0));
        }
    }

    fn apply_sym(state: &mut AppState, id: &str, folds: usize, mirror: bool) {
        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;
        let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
        if let Some(obj) = target_obj {
            let clones = crate::core::symmetry::create_radial_symmetry(&obj, cx, cy, folds, mirror);
            for clone in clones {
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(clone));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        }
    }
}

pub struct VoronoiPanel;

impl VoronoiPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🔷 Voronoi & Mosaic Shatter").strong());
        ui.add_space(4.0);

        if ui.button("Generate Voronoi Mosaic (40 Cells)").clicked() {
            let w = state.document.width;
            let h = state.document.height;
            let mut seeds = Vec::with_capacity(40);
            for i in 0..40 {
                let hx = ((i as f64 * 37.123 + 12.34).sin() * 43758.5453).fract().abs();
                let hy = ((i as f64 * 91.567 + 84.12).sin() * 43758.5453).fract().abs();
                seeds.push(crate::core::path::AnchorPoint::new(hx * w, hy * h));
            }
            let cells = crate::core::voronoi::generate_voronoi_cells(w, h, &seeds, 2.5);
            for cell in cells {
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(cell));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        }
    }
}

pub struct LSystemPanel;

impl LSystemPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌿 L-System Fractals").strong());
        ui.add_space(4.0);

        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        ui.horizontal_wrapped(|ui| {
            if ui.button("🌲 Tree").clicked() {
                let path = crate::core::lsystem::generate_lsystem(crate::core::lsystem::LSystemPreset::Tree, 4, cx, cy + 150.0, 12.0);
                let obj = Object::new_path("Fractal Tree", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("🐉 Dragon").clicked() {
                let path = crate::core::lsystem::generate_lsystem(crate::core::lsystem::LSystemPreset::Dragon, 10, cx - 100.0, cy, 6.0);
                let obj = Object::new_path("Dragon Curve", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("❄️ Snowflake").clicked() {
                let path = crate::core::lsystem::generate_lsystem(crate::core::lsystem::LSystemPreset::Snowflake, 3, cx - 100.0, cy - 50.0, 5.0);
                let obj = Object::new_path("Koch Snowflake", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("🔲 Hilbert").clicked() {
                let path = crate::core::lsystem::generate_lsystem(crate::core::lsystem::LSystemPreset::Hilbert, 4, cx - 100.0, cy - 100.0, 14.0);
                let obj = Object::new_path("Hilbert Curve", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        });
    }
}

pub struct QrCodePanel;

impl QrCodePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📱 Vector QR & Barcode").strong());
        ui.add_space(4.0);

        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        ui.horizontal(|ui| {
            if ui.button("Generate QR Code...").clicked() {
                if let Ok(path) = crate::core::barcode::generate_vector_qr("https://github.com/AI-SLOP-BOX/amata", cx, cy, 160.0) {
                    let obj = Object::new_path("Vector QR Code", path);
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("Barcode (Code-128)").clicked() {
                let path = crate::core::barcode::generate_vector_barcode("IRASU-AEVFX-2026", cx, cy, 200.0, 60.0);
                let obj = Object::new_path("Vector Barcode", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        });
    }
}

pub struct DeformPanel;

impl DeformPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌊 Noise & Wave Deformer").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui.add_enabled(has_sel, egui::Button::new("🌊 Wave")).clicked() {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let def = crate::core::noise::deform_path(&path, crate::core::noise::DeformType::SineWave, 10.0, 0.08, 0.0);
                        let mut new_obj = Object::new_path(&format!("{} (Wave)", obj.name), def);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("🌪️ Noise")).clicked() {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let def = crate::core::noise::deform_path(&path, crate::core::noise::DeformType::TurbulentNoise, 12.0, 0.05, 1.23);
                        let mut new_obj = Object::new_path(&format!("{} (Noise)", obj.name), def);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("⚡ Glitch")).clicked() {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let def = crate::core::noise::deform_path(&path, crate::core::noise::DeformType::JitterGlitch, 8.0, 0.2, 5.67);
                        let mut new_obj = Object::new_path(&format!("{} (Glitch)", obj.name), def);
                        new_obj.transform = obj.transform.clone();
                        let new_id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(RichText::new("Select an object to deform").weak().size(11.0));
        }
    }
}

pub struct FlowFieldPanel;

impl FlowFieldPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌌 Vector Flow Field").strong());
        ui.add_space(4.0);

        let w = state.document.width;
        let h = state.document.height;

        ui.horizontal(|ui| {
            if ui.button("🌀 Vortex").clicked() {
                let lines = crate::core::flowfield::generate_flowfield_streamlines(
                    crate::core::flowfield::FlowFieldPreset::Vortex,
                    w, h, 60, 80, 5.0,
                );
                for line in lines {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(line));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("🧲 Magnetic").clicked() {
                let lines = crate::core::flowfield::generate_flowfield_streamlines(
                    crate::core::flowfield::FlowFieldPreset::MagneticDipole,
                    w, h, 60, 80, 5.0,
                );
                for line in lines {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(line));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("⚡ Cyber").clicked() {
                let lines = crate::core::flowfield::generate_flowfield_streamlines(
                    crate::core::flowfield::FlowFieldPreset::CyberChaos,
                    w, h, 60, 80, 5.0,
                );
                for line in lines {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(line));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }
        });
    }
}

pub struct ScatterBrushPanel;

impl ScatterBrushPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🖌️ Scatter & Pattern Brush").strong());
        ui.add_space(4.0);

        let has_sel = state.selected_ids.len() >= 2;

        if ui.add_enabled(has_sel, egui::Button::new("Scatter 1st (Motif) along 2nd (Path)")).clicked() {
            let id_motif = state.selected_ids[0].clone();
            let id_path = state.selected_ids[1].clone();

            let target_motif = state.document.all_objects().find(|(_, o)| o.id == id_motif).map(|(_, o)| o.clone());
            let target_path = state.document.all_objects().find(|(_, o)| o.id == id_path).map(|(_, o)| o.clone());

            if let (Some(motif), Some(path_obj)) = (target_motif, target_path) {
                let mut traj = path_obj.to_path_data();
                traj.transform(&path_obj.transform.matrix());

                let clones = crate::core::brush::scatter_brush_along_path(&traj, &motif, 30.0, 0.3, true);
                for clone in clones {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(clone));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }
        } else if !has_sel {
            ui.label(RichText::new("Select 2 objects (Motif + Curve)").weak().size(11.0));
        }
    }
}

pub struct AudioWavePanel;

impl AudioWavePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🎵 Audio Waveform (LogicPro DSP)").strong());
        ui.add_space(4.0);

        let w = state.document.width;
        let h = state.document.height;

        ui.horizontal_wrapped(|ui| {
            if ui.button("〰️ Sine Wave").clicked() {
                let path = crate::core::audio_curve::generate_audio_waveform(crate::core::audio_curve::WaveformType::Sine, 4.0, 1, w, h, 200);
                let obj = Object::new_path("Audio Sine Wave", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("📐 Sawtooth").clicked() {
                let path = crate::core::audio_curve::generate_audio_waveform(crate::core::audio_curve::WaveformType::Sawtooth, 4.0, 1, w, h, 200);
                let obj = Object::new_path("Audio Saw Wave", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("🎹 Harmonics").clicked() {
                let path = crate::core::audio_curve::generate_audio_waveform(crate::core::audio_curve::WaveformType::Harmonics, 3.0, 5, w, h, 250);
                let obj = Object::new_path("Audio Harmonics", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }

            if ui.button("⚡ FM Synth").clicked() {
                let path = crate::core::audio_curve::generate_audio_waveform(crate::core::audio_curve::WaveformType::FM, 3.0, 1, w, h, 300);
                let obj = Object::new_path("Audio FM Synth Wave", path);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        });
    }
}

pub struct MeshWarpPanel;

impl MeshWarpPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🦴 2D Mesh Warp (Live2D FFD)").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui.add_enabled(has_sel, egui::Button::new("⭕ Bulge")).clicked() {
                    if let Some(obj) = &target_obj {
                        let warped = crate::core::mesh_warp::apply_lattice_warp(obj, 4, 4, crate::core::mesh_warp::WarpPreset::Bulge, 1.0);
                        let new_id = warped.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(warped));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("🌀 Twist")).clicked() {
                    if let Some(obj) = &target_obj {
                        let warped = crate::core::mesh_warp::apply_lattice_warp(obj, 4, 4, crate::core::mesh_warp::WarpPreset::TwistS, 1.0);
                        let new_id = warped.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(warped));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("🌊 Wave")).clicked() {
                    if let Some(obj) = &target_obj {
                        let warped = crate::core::mesh_warp::apply_lattice_warp(obj, 4, 4, crate::core::mesh_warp::WarpPreset::WaveWarp, 1.0);
                        let new_id = warped.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(warped));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(RichText::new("Select an object to warp with FFD lattice").weak().size(11.0));
        }
    }
}

pub struct GradientMeshPanel;

impl GradientMeshPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌈 Gradient Mesh Generator").strong());
        ui.add_space(4.0);

        let w = state.document.width;
        let h = state.document.height;

        ui.horizontal_wrapped(|ui| {
            if ui.button("🌅 Sunset Mesh").clicked() {
                let patches = crate::core::gradient_mesh::generate_gradient_mesh(
                    crate::core::gradient_mesh::GradientMeshPreset::Sunset,
                    w, h, 3, 3,
                );
                for patch in patches {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(patch));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("🌆 Cyber Mesh").clicked() {
                let patches = crate::core::gradient_mesh::generate_gradient_mesh(
                    crate::core::gradient_mesh::GradientMeshPreset::Cyberpunk,
                    w, h, 3, 3,
                );
                for patch in patches {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(patch));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui.button("🌌 Aurora Mesh").clicked() {
                let patches = crate::core::gradient_mesh::generate_gradient_mesh(
                    crate::core::gradient_mesh::GradientMeshPreset::Aurora,
                    w, h, 3, 3,
                );
                for patch in patches {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(patch));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }
        });
    }
}

pub struct AxonometricPanel;

impl AxonometricPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📐 Axonometric Architectural Projections").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(has_sel, egui::Button::new("Dimetric")).clicked() {
                    if let Some(obj) = &target_obj {
                        let proj = crate::core::axonometric::apply_axonometric_projection(obj, crate::core::axonometric::AxonometricMode::Dimetric);
                        let new_id = proj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(proj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Trimetric")).clicked() {
                    if let Some(obj) = &target_obj {
                        let proj = crate::core::axonometric::apply_axonometric_projection(obj, crate::core::axonometric::AxonometricMode::Trimetric);
                        let new_id = proj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(proj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Cabinet (Oblique)")).clicked() {
                    if let Some(obj) = &target_obj {
                        let proj = crate::core::axonometric::apply_axonometric_projection(obj, crate::core::axonometric::AxonometricMode::Cabinet);
                        let new_id = proj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(proj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Cavalier")).clicked() {
                    if let Some(obj) = &target_obj {
                        let proj = crate::core::axonometric::apply_axonometric_projection(obj, crate::core::axonometric::AxonometricMode::Cavalier);
                        let new_id = proj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(proj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(RichText::new("Select an object for axonometric projection").weak().size(11.0));
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
            let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui.add_enabled(has_sel, egui::Button::new("Cyan Neon")).clicked() {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let neon_layers = crate::core::neon_glow::generate_neon_glow(&path, [0.0, 1.0, 0.9, 1.0], 18.0, 6);
                        for layer in neon_layers {
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(layer));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Magenta Neon")).clicked() {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let neon_layers = crate::core::neon_glow::generate_neon_glow(&path, [1.0, 0.1, 0.7, 1.0], 18.0, 6);
                        for layer in neon_layers {
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(layer));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Gold Laser")).clicked() {
                    if let Some(obj) = &target_obj {
                        let path = obj.to_path_data();
                        let neon_layers = crate::core::neon_glow::generate_neon_glow(&path, [1.0, 0.8, 0.1, 1.0], 18.0, 6);
                        for layer in neon_layers {
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(layer));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                }
            });
        } else {
            ui.label(RichText::new("Select an object to generate vector neon halo").weak().size(11.0));
        }
    }
}

pub struct RevolvePanel;

impl RevolvePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🏺 3D Revolve & Lathe Modeler").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
            if ui.add_enabled(has_sel, egui::Button::new("Export 3D Revolve OBJ...")).clicked() {
                if let Some(obj) = &target_obj {
                    let axis = obj.bounding_box().map(|(min, _)| min.x).unwrap_or(0.0);
                    let obj_data = crate::core::revolve::generate_3d_revolve_obj(obj, axis, 360.0, 32);
                    if let Some(path) = rfd::FileDialog::new().add_filter("OBJ 3D Model", &["obj"]).save_file() {
                        let _ = std::fs::write(path, obj_data);
                    }
                }
            }
        } else {
            ui.label(RichText::new("Select a profile path to revolve in 3D").weak().size(11.0));
        }
    }
}

pub struct EnvelopePanel;

impl EnvelopePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🚩 Envelope Distort & Shape Mold").strong());
        ui.add_space(4.0);

        let has_sel = state.selected_ids.len() >= 2;

        if ui.add_enabled(has_sel, egui::Button::new("Mold 1st (Art) inside 2nd (Frame)")).clicked() {
            let id_art = state.selected_ids[0].clone();
            let id_env = state.selected_ids[1].clone();

            let target_art = state.document.all_objects().find(|(_, o)| o.id == id_art).map(|(_, o)| o.clone());
            let target_env = state.document.all_objects().find(|(_, o)| o.id == id_env).map(|(_, o)| o.clone());

            if let (Some(art), Some(env)) = (target_art, target_env) {
                let warped = crate::core::envelope::apply_envelope_distort(&art, &env);
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(warped));
                state.undo_manager.execute(cmd, &mut state.document);
            }
        } else if !has_sel {
            ui.label(RichText::new("Select 2 objects (Art + Envelope Frame)").weak().size(11.0));
        }
    }
}

pub struct PolarPanel;

impl PolarPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🌐 Polar Coordinates & Planet Wrap").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();
        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;
        let w = state.document.width;
        let h = state.document.height;

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui.add_enabled(has_sel, egui::Button::new("Rect -> Polar")).clicked() {
                    if let Some(obj) = &target_obj {
                        let polar = crate::core::polar::apply_polar_transform(obj, cx, cy, w, h, crate::core::polar::PolarMode::RectToPolar);
                        let new_id = polar.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(polar));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Polar -> Rect")).clicked() {
                    if let Some(obj) = &target_obj {
                        let rect = crate::core::polar::apply_polar_transform(obj, cx, cy, w, h, crate::core::polar::PolarMode::PolarToRect);
                        let new_id = rect.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(rect));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                    }
                }
            });
        } else {
            ui.label(RichText::new("Select an object to wrap into circular polar coordinates").weak().size(11.0));
        }
    }
}

pub struct KnifePanel;

impl KnifePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("✂️ Knife & Vector Slicer").strong());
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();

        if let Some(id) = state.selected_ids.first().cloned() {
            let target_obj = state.document.all_objects().find(|(_, o)| o.id == id).map(|(_, o)| o.clone());
            ui.horizontal(|ui| {
                if ui.add_enabled(has_sel, egui::Button::new("Slice Horizontally")).clicked() {
                    if let Some(obj) = &target_obj {
                        if let Some((min, max)) = obj.bounding_box() {
                            let mid_y = (min.y + max.y) * 0.5;
                            let p1 = crate::core::path::AnchorPoint::new(min.x - 10.0, mid_y);
                            let p2 = crate::core::path::AnchorPoint::new(max.x + 10.0, mid_y);

                            if let Some((part_a, part_b)) = crate::core::knife::slice_object_with_line(obj, p1, p2) {
                                let cmd1 = Box::new(crate::core::history::RemoveObjectCommand::new(obj.clone(), 0, 0));
                                state.undo_manager.execute(cmd1, &mut state.document);

                                let cmd2 = Box::new(crate::core::history::AddObjectCommand::new(part_a));
                                state.undo_manager.execute(cmd2, &mut state.document);

                                let cmd3 = Box::new(crate::core::history::AddObjectCommand::new(part_b));
                                state.undo_manager.execute(cmd3, &mut state.document);
                            }
                        }
                    }
                }

                if ui.add_enabled(has_sel, egui::Button::new("Slice Vertically")).clicked() {
                    if let Some(obj) = &target_obj {
                        if let Some((min, max)) = obj.bounding_box() {
                            let mid_x = (min.x + max.x) * 0.5;
                            let p1 = crate::core::path::AnchorPoint::new(mid_x, min.y - 10.0);
                            let p2 = crate::core::path::AnchorPoint::new(mid_x, max.y + 10.0);

                            if let Some((part_a, part_b)) = crate::core::knife::slice_object_with_line(obj, p1, p2) {
                                let cmd1 = Box::new(crate::core::history::RemoveObjectCommand::new(obj.clone(), 0, 0));
                                state.undo_manager.execute(cmd1, &mut state.document);

                                let cmd2 = Box::new(crate::core::history::AddObjectCommand::new(part_a));
                                state.undo_manager.execute(cmd2, &mut state.document);

                                let cmd3 = Box::new(crate::core::history::AddObjectCommand::new(part_b));
                                state.undo_manager.execute(cmd3, &mut state.document);
                            }
                        }
                    }
                }
            });
        } else {
            ui.label(RichText::new("Select an object to slice in half").weak().size(11.0));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// SymbolsPanel: Reusable object library
// ═══════════════════════════════════════════════════════════════════

pub struct SymbolsPanel;

impl SymbolsPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("⭐ Symbols").strong());
        ui.add_space(4.0);

        // Save selected as symbol
        let has_sel = !state.selected_ids.is_empty();
        if ui.add_enabled(has_sel, egui::Button::new("Save Selection as Symbol")).clicked() {
            if let Some(id) = state.selected_ids.first() {
                if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                    let sym = crate::core::document::Symbol::new(
                        &format!("Symbol {}", state.symbols.len() + 1),
                        obj.clone(),
                    );
                    state.symbols.push(sym);
                }
            }
        }

        ui.add_space(4.0);
        ui.separator();

        // Symbol library
        if state.symbols.is_empty() {
            ui.label(RichText::new("No symbols yet").weak());
            return;
        }

        ui.label(RichText::new(format!("Library ({} symbols)", state.symbols.len())).strong());
        ui.add_space(2.0);

        let mut to_remove = None;
        for (i, sym) in state.symbols.iter().enumerate() {
            ui.horizontal(|ui| {
                // Preview thumbnail
                let c32 = Color32::from_rgba_unmultiplied(100, 150, 200, 255);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(24.0, 24.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 3.0, c32);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "S", egui::FontId::proportional(12.0), Color32::WHITE);

                ui.vertical(|ui| {
                    ui.label(RichText::new(&sym.name).strong().size(11.0));
                    ui.label(RichText::new(format!("Used: {} times", sym.use_count)).weak().size(10.0));
                });

                if ui.small_button("Place").clicked() {
                    let mut new_obj = sym.object.clone();
                    new_obj.id = uuid::Uuid::new_v4().to_string();
                    new_obj.name = format!("{} Instance", sym.name);
                    new_obj.transform.x += 50.0;
                    new_obj.transform.y += 50.0;
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                }

                if ui.small_button("✕").clicked() {
                    to_remove = Some(i);
                }
            });
        }

        if let Some(idx) = to_remove {
            state.symbols.remove(idx);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// WidthToolPanel: Variable stroke width
// ═══════════════════════════════════════════════════════════════════

pub struct WidthToolPanel;

impl WidthToolPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("〰 Width Tool").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select a stroked object").weak());
            return;
        }

        let id = state.selected_ids[0].clone();
        let mut width_profile = crate::core::document::WidthProfile::default();
        let mut has_stroke = false;

        for (_, obj) in state.document.all_objects() {
            if obj.id == id && obj.stroke.is_some() {
                has_stroke = true;
                width_profile = state.width_profiles.get(&id).cloned().unwrap_or_default();
                break;
            }
        }

        if !has_stroke {
            ui.label(RichText::new("Object has no stroke").weak());
            return;
        }

        ui.label("Add width points along the stroke path:");
        ui.add_space(4.0);

        let mut to_remove = None;
        let point_count = width_profile.points.len();
        for i in 0..point_count {
            let mut pos = width_profile.points[i].position;
            let mut width = width_profile.points[i].width;
            ui.horizontal(|ui| {
                ui.label(format!("{:.0}%", pos * 100.0));
                if ui.add(egui::Slider::new(&mut pos, 0.0..=1.0).show_value(false).step_by(0.01)).changed() {
                    width_profile.points[i].position = pos;
                }
                if ui.add(egui::DragValue::new(&mut width).speed(0.1).range(0.01..=10.0).suffix("x")).changed() {
                    width_profile.points[i].width = width;
                }
                if point_count > 2 && ui.small_button("✕").clicked() {
                        to_remove = Some(i);
                }
            });
        }

        if let Some(idx) = to_remove {
            width_profile.points.remove(idx);
        }

        if ui.button("+ Add Width Point").clicked() {
            let last_pos = width_profile.points.last().map(|p| p.position).unwrap_or(0.5);
            width_profile.points.push(crate::core::document::WidthPoint {
                position: (last_pos + 0.5).min(1.0),
                width: 1.0,
                side: crate::core::document::WidthSide::Both,
            });
            width_profile.points.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
        }

        // Reset profile
        if ui.button("Reset Profile").clicked() {
            width_profile = crate::core::document::WidthProfile::default();
        }

        state.width_profiles.insert(id, width_profile);
    }
}

// ═══════════════════════════════════════════════════════════════════
// PatternPanel: Repeating pattern fills
// ═══════════════════════════════════════════════════════════════════

pub struct PatternPanel;

impl PatternPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🔲 Pattern Fill").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select an object to apply pattern").weak());
            return;
        }

        let id = state.selected_ids[0].clone();

        // Pattern type selector
        ui.label("Pattern Type:");
        let mut pattern = crate::core::path::PatternFill::default();

        ui.horizontal_wrapped(|ui| {
            for (ptype, label) in [
                (crate::core::path::PatternType::Grid, "Grid"),
                (crate::core::path::PatternType::Hex, "Hex"),
                (crate::core::path::PatternType::Brick, "Brick"),
                (crate::core::path::PatternType::Dots, "Dots"),
            ] {
                if ui.selectable_label(pattern.pattern_type == ptype, label).clicked() {
                    pattern.pattern_type = ptype;
                }
            }
        });

        ui.add_space(4.0);

        // Tile size
        ui.horizontal(|ui| {
            ui.label("Tile W:");
            ui.add(egui::DragValue::new(&mut pattern.tile_width).speed(1.0).range(5.0..=500.0));
            ui.label("H:");
            ui.add(egui::DragValue::new(&mut pattern.tile_height).speed(1.0).range(5.0..=500.0));
        });

        // Scale
        ui.horizontal(|ui| {
            ui.label("Scale:");
            ui.add(egui::Slider::new(&mut pattern.scale, 0.1..=5.0).show_value(true));
        });

        // Rotation
        ui.horizontal(|ui| {
            ui.label("Rotation:");
            ui.add(egui::DragValue::new(&mut pattern.rotation).speed(1.0).range(-180.0..=180.0).suffix("°"));
        });

        ui.add_space(4.0);

        // Apply pattern
        if ui.button("Apply Pattern").clicked() {
            let fill = FillStyle {
                color: state.fill_color,
                fill_type: FillType::Pattern(pattern.clone()),
                rule: crate::core::path::FillRule::NonZero,
            };
            for (_, obj) in state.document.all_objects_mut() {
                if obj.id == id {
                    obj.fill = Some(fill.clone());
                }
            }
        }

        // Preset patterns
        ui.separator();
        ui.label(RichText::new("Presets").strong().size(11.0));
        ui.horizontal_wrapped(|ui| {
            for (name, pw, ph, ptype) in [
                ("Small Grid", 20.0, 20.0, crate::core::path::PatternType::Grid),
                ("Large Grid", 60.0, 60.0, crate::core::path::PatternType::Grid),
                ("Hex Small", 25.0, 25.0, crate::core::path::PatternType::Hex),
                ("Dots Small", 30.0, 30.0, crate::core::path::PatternType::Dots),
                ("Brick", 50.0, 25.0, crate::core::path::PatternType::Brick),
            ] {
                if ui.selectable_label(false, name).clicked() {
                    let p = crate::core::path::PatternFill {
                        pattern_type: ptype,
                        tile_width: pw,
                        tile_height: ph,
                        scale: 1.0,
                        ..Default::default()
                    };
                    let fill = FillStyle {
                        color: state.fill_color,
                        fill_type: FillType::Pattern(p),
                        rule: crate::core::path::FillRule::NonZero,
                    };
                    for (_, obj) in state.document.all_objects_mut() {
                        if obj.id == id {
                            obj.fill = Some(fill.clone());
                        }
                    }
                }
            }
        });
    }
}

// ═══════════════════════════════════════════════════════════════════
// TextPanel: Font/Size/Alignment/Spacing
// ═══════════════════════════════════════════════════════════════════

pub struct TextPanel;

impl TextPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("𝐓 Text").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select a text object").weak());
            return;
        }

        let id = state.selected_ids[0].clone();
        let mut text = String::new();
        let mut font_size = 32.0;
        let mut font_family = String::from("Sans-Serif");
        let mut align = TextAlignment::Left;
        let mut line_height = 1.2f64;
        let mut letter_spacing = 0.0f64;
        let mut word_spacing = 0.0f64;
        let mut found = false;

        for (_, obj) in state.document.all_objects() {
            if obj.id == id {
                if let ObjectType::Text { text: t, font_size: fs } = &obj.object_type {
                    text = t.clone();
                    font_size = *fs;
                    found = true;
                }
                break;
            }
        }

        if !found {
            ui.label(RichText::new("Not a text object").weak());
            return;
        }

        // Text content
        ui.label("Content:");
        ui.text_edit_multiline(&mut text);

        ui.add_space(4.0);

        // Font size
        ui.horizontal(|ui| {
            ui.label("Size:");
            let mut fs = font_size;
            if ui.add(egui::DragValue::new(&mut fs).speed(1.0).range(6.0..=500.0).suffix("pt")).changed() {
                for (_, obj) in state.document.all_objects_mut() {
                    if obj.id == id {
                        if let ObjectType::Text { font_size: ref mut s, .. } = obj.object_type {
                            *s = fs;
                        }
                    }
                }
            }
        });

        // Font family
        ui.horizontal(|ui| {
            ui.label("Font:");
            egui::ComboBox::from_id_salt("font_family").selected_text(&font_family).show_ui(ui, |ui| {
                for font in ["Sans-Serif", "Serif", "Mono", "Cursive", "Fantasy"] {
                    if ui.selectable_label(font_family == font, font).clicked() {
                        font_family = font.into();
                    }
                }
            });
        });

        ui.add_space(4.0);

        // Alignment
        ui.label("Alignment:");
        ui.horizontal(|ui| {
            for (a, label) in [
                (TextAlignment::Left, "Left"),
                (TextAlignment::Center, "Center"),
                (TextAlignment::Right, "Right"),
                (TextAlignment::Justify, "Justify"),
            ] {
                if ui.selectable_label(align == a, label).clicked() {
                    align = a;
                }
            }
        });

        ui.add_space(4.0);

        // Line height
        ui.horizontal(|ui| {
            ui.label("Line Height:");
            let mut lh = line_height;
            if ui.add(egui::Slider::new(&mut lh, 0.5..=3.0).show_value(true).step_by(0.1)).changed() {
                line_height = lh;
            }
        });

        // Letter spacing
        ui.horizontal(|ui| {
            ui.label("Letter Spacing:");
            let mut ls = letter_spacing;
            if ui.add(egui::DragValue::new(&mut ls).speed(0.5).range(-10.0..=50.0)).changed() {
                letter_spacing = ls;
            }
        });

        // Word spacing
        ui.horizontal(|ui| {
            ui.label("Word Spacing:");
            let mut ws = word_spacing;
            if ui.add(egui::DragValue::new(&mut ws).speed(1.0).range(-10.0..=100.0)).changed() {
                word_spacing = ws;
            }
        });

        ui.add_space(4.0);

        // Quick sizes
        ui.label("Quick Sizes:");
        ui.horizontal_wrapped(|ui| {
            for size in [9.0, 10.0, 11.0, 12.0, 14.0, 18.0, 24.0, 36.0, 48.0, 60.0, 72.0, 96.0] {
                if ui.selectable_label(font_size == size, format!("{}", size)).clicked() {
                    for (_, obj) in state.document.all_objects_mut() {
                        if obj.id == id {
                            if let ObjectType::Text { font_size: ref mut s, .. } = obj.object_type {
                                *s = size;
                            }
                        }
                    }
                }
            }
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
    Justify,
}

// ═══════════════════════════════════════════════════════════════════
// SmartGuidesPanel: Snapping and guides
// ═══════════════════════════════════════════════════════════════════

pub struct SmartGuidesPanel;

impl SmartGuidesPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📏 Smart Guides").strong());
        ui.add_space(4.0);

        // Snapping options
        ui.label(RichText::new("Snap To:").strong());
        ui.checkbox(&mut state.snap_to_grid, "Grid");
        ui.checkbox(&mut state.snap_to_objects, "Objects");
        ui.checkbox(&mut state.snap_to_guides, "Guides");
        ui.checkbox(&mut state.snap_to_points, "Anchor Points");

        ui.add_space(4.0);
        ui.separator();

        // Grid settings
        ui.label(RichText::new("Grid").strong());
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(egui::DragValue::new(&mut state.grid_size).speed(1.0).range(1.0..=100.0).suffix("px"));
        });

        ui.add_space(4.0);
        ui.separator();

        // Guides
        ui.label(RichText::new("Custom Guides").strong());
        ui.horizontal(|ui| {
            if ui.button("Add H Guide").clicked() {
                state.guides.push(crate::core::state::Guide {
                    orientation: crate::core::state::GuideOrientation::Horizontal,
                    position: state.pan_y as f64 / state.zoom as f64,
                });
            }
            if ui.button("Add V Guide").clicked() {
                state.guides.push(crate::core::state::Guide {
                    orientation: crate::core::state::GuideOrientation::Vertical,
                    position: state.pan_x as f64 / state.zoom as f64,
                });
            }
        });

        if !state.guides.is_empty() {
            ui.add_space(2.0);
            let mut to_remove = None;
            for (i, guide) in state.guides.iter().enumerate() {
                ui.horizontal(|ui| {
                    let orient = match guide.orientation {
                        crate::core::state::GuideOrientation::Horizontal => "H",
                        crate::core::state::GuideOrientation::Vertical => "V",
                    };
                    ui.label(format!("{}: {:.1}", orient, guide.position));
                    if ui.small_button("✕").clicked() {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(idx) = to_remove {
                state.guides.remove(idx);
            }
            if ui.button("Clear All").clicked() {
                state.guides.clear();
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// ExportPanel: Export to PNG/SVG/PDF
// ═══════════════════════════════════════════════════════════════════

pub struct ExportPanel;

impl ExportPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📤 Export").strong());
        ui.add_space(4.0);

        // Export format
        ui.label("Format:");
        let mut format = state.export_format.clone();

        ui.horizontal_wrapped(|ui| {
            for f in ["SVG", "PNG", "JSON"] {
                if ui.selectable_label(format == f, f).clicked() {
                    format = f.into();
                    state.export_format = format.clone();
                }
            }
        });

        ui.add_space(4.0);

        // Export settings
        match format.as_str() {
            "PNG" => {
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut state.export_width).range(16.0..=8192.0));
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut state.export_height).range(16.0..=8192.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    ui.add(egui::Slider::new(&mut state.export_scale, 0.1..=4.0).show_value(true));
                });
                ui.checkbox(&mut state.export_transparent, "Transparent Background");
            }
            "SVG" => {
                ui.checkbox(&mut state.export_svg_viewbox, "Include ViewBox");
                ui.checkbox(&mut state.export_svg_embed_fonts, "Embed Fonts");
            }
            _ => {}
        }

        ui.add_space(4.0);
        ui.separator();

        // Export scope
        ui.label("Scope:");
        ui.horizontal_wrapped(|ui| {
            if ui.selectable_label(state.export_scope == "All", "All Objects").clicked() {
                state.export_scope = "All".into();
            }
            if ui.selectable_label(state.export_scope == "Selected", "Selected Only").clicked() {
                state.export_scope = "Selected".into();
            }
        });

        ui.add_space(8.0);

        // Export button
        if ui.button("Export...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Export As")
                .add_filter("All Supported", &["svg", "png", "json"])
                .add_filter("SVG", &["svg"])
                .add_filter("PNG", &["png"])
                .add_filter("JSON", &["json"])
                .save_file()
            {
                state.export_path = Some(path.to_string_lossy().to_string());
                state.pending_export = true;
            }
        }

        if let Some(ref p) = state.export_path {
            ui.label(RichText::new(format!("→ {}", p)).weak().size(10.0));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// GridRepeatPanel: Grid and radial repeat
// ═══════════════════════════════════════════════════════════════════

pub struct GridRepeatPanel;

impl GridRepeatPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🔲 Grid Repeat").strong());
        ui.add_space(4.0);

        if state.selected_ids.is_empty() {
            ui.label(RichText::new("Select an object to repeat").weak());
            return;
        }

        ui.label("Grid Layout:");
        ui.horizontal(|ui| {
            ui.label("Columns:");
            ui.add(egui::DragValue::new(&mut state.repeat_cols).range(1..=50));
            ui.label("Rows:");
            ui.add(egui::DragValue::new(&mut state.repeat_rows).range(1..=50));
        });

        ui.horizontal(|ui| {
            ui.label("H Spacing:");
            ui.add(egui::DragValue::new(&mut state.repeat_h_gap).range(0.0..=500.0));
            ui.label("V Spacing:");
            ui.add(egui::DragValue::new(&mut state.repeat_v_gap).range(0.0..=500.0));
        });

        ui.add_space(4.0);

        ui.label("Radial Layout:");
        ui.horizontal(|ui| {
            ui.label("Copies:");
            ui.add(egui::DragValue::new(&mut state.repeat_radial_count).range(2..=100));
            ui.label("Radius:");
            ui.add(egui::DragValue::new(&mut state.repeat_radial_radius).range(10.0..=2000.0));
        });

        ui.horizontal(|ui| {
            ui.label("Start Angle:");
            ui.add(egui::DragValue::new(&mut state.repeat_start_angle).range(-360.0..=360.0).suffix("°"));
        });

        ui.add_space(8.0);

        if ui.button("Create Grid Repeat").clicked() {
            if let Some(id) = state.selected_ids.first() {
                let obj = state.document.all_objects().find(|(_, o)| &o.id == id).map(|(_, o)| o.clone());
                if let Some(obj) = obj {
                    for row in 0..state.repeat_rows {
                        for col in 0..state.repeat_cols {
                            if row == 0 && col == 0 { continue; }
                            let mut new_obj = obj.clone();
                            new_obj.id = uuid::Uuid::new_v4().to_string();
                            new_obj.name = format!("{} ({},{})", obj.name, col, row);
                            new_obj.transform.x += col as f64 * state.repeat_h_gap;
                            new_obj.transform.y += row as f64 * state.repeat_v_gap;
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                }
            }
        }

        if ui.button("Create Radial Repeat").clicked() {
            if let Some(id) = state.selected_ids.first() {
                let obj = state.document.all_objects().find(|(_, o)| &o.id == id).map(|(_, o)| o.clone());
                if let Some(obj) = obj {
                    let angle_step = 360.0 / state.repeat_radial_count as f64;
                    let start_rad = state.repeat_start_angle.to_radians();
                    for i in 1..state.repeat_radial_count {
                        let angle = start_rad + (i as f64) * angle_step.to_radians();
                        let mut new_obj = obj.clone();
                        new_obj.id = uuid::Uuid::new_v4().to_string();
                        new_obj.name = format!("{} R{}", obj.name, i);
                        new_obj.transform.x = obj.transform.x + angle.cos() * state.repeat_radial_radius;
                        new_obj.transform.y = obj.transform.y + angle.sin() * state.repeat_radial_radius;
                        new_obj.transform.rotation = angle;
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// ColorHarmonyPanel: Color harmony / complementary / analogous schemes
// ═══════════════════════════════════════════════════════════════════

pub struct ColorHarmonyPanel;

impl ColorHarmonyPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🎨 Color Harmony").strong());
        ui.add_space(4.0);

        let base = state.fill_color;
        let (h, s, v) = rgb_to_hsv(base[0], base[1], base[2]);

        ui.label(format!("Base: H {:.0}° S {:.0}% V {:.0}%", h, s * 100.0, v * 100.0));

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
        ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0_f32, Color32::from_gray(80)), egui::StrokeKind::Inside);

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

pub struct ShortcutsHelpPanel;

impl ShortcutsHelpPanel {
    pub fn show(ui: &mut Ui, _state: &mut AppState) {
        ui.heading(RichText::new("⌨ Keyboard Shortcuts").strong());
        ui.add_space(4.0);

        let shortcuts: [(&str, &str); 32] = [
            ("V", "Select Tool"),
            ("A", "Node / Direct Select"),
            ("P", "Pen Tool"),
            ("N", "Pencil Tool"),
            ("U", "Rectangle Tool"),
            ("O", "Ellipse Tool"),
            ("S", "Star Tool"),
            ("G", "Polygon Tool"),
            ("L", "Line Tool"),
            ("T", "Text Tool"),
            ("I", "Eyedropper"),
            ("H", "Hand / Pan"),
            ("B", "Brush Tool"),
            ("E", "Eraser Tool"),
            ("D", "Default Fill & Stroke"),
            ("/", "Set Fill to None"),
            ("Shift+X", "Swap Fill & Stroke"),
            ("Delete", "Delete Selected"),
            ("Escape", "Deselect / Cancel"),
            ("Enter", "Finish Pen Path"),
            ("Ctrl+Z", "Undo"),
            ("Ctrl+Y", "Redo"),
            ("Ctrl+A", "Select All"),
            ("Ctrl+G", "Group"),
            ("Ctrl+Shift+G", "Ungroup"),
            ("Ctrl+D", "Duplicate"),
            ("Ctrl+C", "Copy"),
            ("Ctrl+V", "Paste"),
            ("Ctrl+0", "Zoom to Fit"),
            ("Ctrl+1", "Zoom 100%"),
            ("Ctrl+7", "Clipping Mask"),
            ("Arrow Keys", "Nudge (Shift=10x)"),
        ];

        for (key, action) in shortcuts {
            ui.horizontal(|ui| {
                ui.label(RichText::new(key).strong().monospace().size(11.0));
                ui.separator();
                ui.label(RichText::new(action).size(11.0));
            });
        }
    }
}
