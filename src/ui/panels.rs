use crate::core::boolean::{execute_pathfinder, BooleanOp};
use crate::core::document::{ObjectType, Object};
use crate::core::morph::morph_paths;
use crate::core::offset::{offset_path, outline_stroke};
use crate::core::path::{FillStyle, StrokeStyle};
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

pub struct LayerPanel;

impl LayerPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📑 Layers").strong());
        ui.separator();

        let layer_count = state.document.layers.len();
        let active_idx = state.document.active_layer_idx;

        let mut to_add_layer = false;
        let mut to_remove_layer = false;
        let mut to_select_obj: Option<String> = None;
        let mut to_remove_obj: Option<(usize, usize)> = None;
        let mut to_toggle_vis: Option<usize> = None;
        let mut to_toggle_lock: Option<usize> = None;

        for (i, layer) in state.document.layers.iter().enumerate() {
            let is_active = i == active_idx;
            let text = if is_active {
                RichText::new(format!("📁 {}", layer.name)).strong().color(Color32::from_rgb(100, 180, 255))
            } else {
                RichText::new(format!("📁 {}", layer.name))
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
