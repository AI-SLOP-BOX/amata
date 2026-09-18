use crate::core::history::{BatchCommand, Command, MoveObjectCommand};
use crate::core::state::AppState;
use egui::{RichText, Ui};

/// Execute collected absolute moves as one undoable step.
/// (Align/distribute previously wrote transforms directly, leaving the
/// operation un-undoable and invisible to dirty tracking.)
fn execute_moves(state: &mut AppState, label: &str, moves: Vec<(String, f64, f64, f64, f64)>) {
    if moves.is_empty() {
        return;
    }
    if moves.len() == 1 {
        let (id, old_x, old_y, new_x, new_y) = moves.into_iter().next().unwrap();
        state.undo_manager.execute(
            Box::new(MoveObjectCommand {
                object_id: id,
                old_x,
                old_y,
                new_x,
                new_y,
            }),
            &mut state.document,
        );
    } else {
        let cmds: Vec<Box<dyn Command>> = moves
            .into_iter()
            .map(
                |(id, old_x, old_y, new_x, new_y)| {
                    Box::new(MoveObjectCommand {
                        object_id: id,
                        old_x,
                        old_y,
                        new_x,
                        new_y,
                    }) as Box<dyn Command>
                },
            )
            .collect();
        state.undo_manager.execute(
            Box::new(BatchCommand::new(label, cmds)),
            &mut state.document,
        );
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
            if ui
                .add_enabled(multi, egui::Button::new("⇹ Center H"))
                .clicked()
            {
                align_center_h(state, &sel);
            }
            if ui
                .add_enabled(multi, egui::Button::new("⇥ Right"))
                .clicked()
            {
                align_right(state, &sel);
            }
        });

        ui.horizontal(|ui| {
            if ui.add_enabled(multi, egui::Button::new("⤒ Top")).clicked() {
                align_top(state, &sel);
            }
            if ui
                .add_enabled(multi, egui::Button::new("⇕ Center V"))
                .clicked()
            {
                align_center_v(state, &sel);
            }
            if ui
                .add_enabled(multi, egui::Button::new("⤓ Bottom"))
                .clicked()
            {
                align_bottom(state, &sel);
            }
        });

        ui.add_space(4.0);
        ui.label(RichText::new("Distribute:").weak().size(11.0));
        ui.horizontal(|ui| {
            if ui
                .add_enabled(sel.len() >= 3, egui::Button::new("⬌ Distribute H"))
                .clicked()
            {
                distribute_h(state, &sel);
            }
            if ui
                .add_enabled(sel.len() >= 3, egui::Button::new("⬍ Distribute V"))
                .clicked()
            {
                distribute_v(state, &sel);
            }
        });

        ui.add_space(4.0);
        ui.label(RichText::new("Flip & Arrange:").weak().size(11.0));
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!sel.is_empty(), egui::Button::new("⇆ Flip H"))
                .clicked()
            {
                for id in &sel {
                    for (_, obj) in state.document.all_objects_mut() {
                        if &obj.id == id {
                            obj.transform.scale_x *= -1.0;
                        }
                    }
                }
            }
            if ui
                .add_enabled(!sel.is_empty(), egui::Button::new("⇅ Flip V"))
                .clicked()
            {
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
            if ui
                .add_enabled(has_sel, egui::Button::new("⬍ To Front"))
                .on_hover_text("Ctrl+Shift+]")
                .clicked()
            {
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
            if ui
                .add_enabled(has_sel, egui::Button::new("↑ Forward"))
                .on_hover_text("Ctrl+]")
                .clicked()
            {
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
            if ui
                .add_enabled(has_sel, egui::Button::new("↓ Backward"))
                .on_hover_text("Ctrl+[")
                .clicked()
            {
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
            if ui
                .add_enabled(has_sel, egui::Button::new("⬌ To Back"))
                .on_hover_text("Ctrl+Shift+[")
                .clicked()
            {
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

    pub fn align_left(state: &mut AppState, sel: &[String]) {
        align_left(state, sel);
    }
    pub fn align_right(state: &mut AppState, sel: &[String]) {
        align_right(state, sel);
    }
    pub fn align_center_h(state: &mut AppState, sel: &[String]) {
        align_center_h(state, sel);
    }
    pub fn align_top(state: &mut AppState, sel: &[String]) {
        align_top(state, sel);
    }
    pub fn align_bottom(state: &mut AppState, sel: &[String]) {
        align_bottom(state, sel);
    }
    pub fn align_center_v(state: &mut AppState, sel: &[String]) {
        align_center_v(state, sel);
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
    let mut moves = Vec::new();
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, _)) = obj.bounding_box() {
                let (ox, oy) = (obj.transform.x, obj.transform.y);
                moves.push((id.clone(), ox, oy, ox + (min_x - bb_min.x), oy));
            }
        }
    }
    execute_moves(state, "Align Left", moves);
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
    let mut moves = Vec::new();
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                let obj_center = (bb_min.x + bb_max.x) / 2.0;
                let (ox, oy) = (obj.transform.x, obj.transform.y);
                moves.push((id.clone(), ox, oy, ox + (center - obj_center), oy));
            }
        }
    }
    execute_moves(state, "Align Center H", moves);
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
    let mut moves = Vec::new();
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((_, bb_max)) = obj.bounding_box() {
                let (ox, oy) = (obj.transform.x, obj.transform.y);
                moves.push((id.clone(), ox, oy, ox + (max_x - bb_max.x), oy));
            }
        }
    }
    execute_moves(state, "Align Right", moves);
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
    let mut moves = Vec::new();
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, _)) = obj.bounding_box() {
                let (ox, oy) = (obj.transform.x, obj.transform.y);
                moves.push((id.clone(), ox, oy, ox, oy + (min_y - bb_min.y)));
            }
        }
    }
    execute_moves(state, "Align Top", moves);
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
    let mut moves = Vec::new();
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                let obj_center = (bb_min.y + bb_max.y) / 2.0;
                let (ox, oy) = (obj.transform.x, obj.transform.y);
                moves.push((id.clone(), ox, oy, ox, oy + (center - obj_center)));
            }
        }
    }
    execute_moves(state, "Align Center V", moves);
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
    let mut moves = Vec::new();
    for id in sel {
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            if let Some((_, bb_max)) = obj.bounding_box() {
                let (ox, oy) = (obj.transform.x, obj.transform.y);
                moves.push((id.clone(), ox, oy, ox, oy + (max_y - bb_max.y)));
            }
        }
    }
    execute_moves(state, "Align Bottom", moves);
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
    let mut moves = Vec::new();
    for (id, orig_min, w) in &items {
        let delta = current_pos - orig_min;
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            let (ox, oy) = (obj.transform.x, obj.transform.y);
            moves.push((id.clone(), ox, oy, ox + delta, oy));
        }
        current_pos += w + gap;
    }
    execute_moves(state, "Distribute H", moves);
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
    let mut moves = Vec::new();
    for (id, orig_min, h) in &items {
        let delta = current_pos - orig_min;
        if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
            let (ox, oy) = (obj.transform.x, obj.transform.y);
            moves.push((id.clone(), ox, oy, ox, oy + delta));
        }
        current_pos += h + gap;
    }
    execute_moves(state, "Distribute V", moves);
}

// ═══════════════════════════════════════════════════════════════════
// StrokePanel: Dash pattern, Cap, Join, Arrowheads
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

        if !found {
            return;
        }

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
                if ui
                    .add(
                        egui::DragValue::new(&mut sx)
                            .speed(0.01)
                            .range(0.001..=100.0),
                    )
                    .changed()
                {
                    Self::set_transform(state, &id, |t| t.scale_x = sx);
                }
                ui.label("H:");
                if ui
                    .add(
                        egui::DragValue::new(&mut sy)
                            .speed(0.01)
                            .range(0.001..=100.0),
                    )
                    .changed()
                {
                    Self::set_transform(state, &id, |t| t.scale_y = sy);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Lock Aspect").clicked() {
                    let avg = (sx + sy) / 2.0;
                    Self::set_transform(state, &id, |t| {
                        t.scale_x = avg;
                        t.scale_y = avg;
                    });
                }
                if ui.button("Reset Scale").clicked() {
                    Self::set_transform(state, &id, |t| {
                        t.scale_x = 1.0;
                        t.scale_y = 1.0;
                    });
                }
            });
        });

        // Rotation
        ui.collapsing("Rotation", |ui| {
            ui.horizontal(|ui| {
                ui.label("°");
                if ui
                    .add(
                        egui::DragValue::new(&mut rot)
                            .speed(1.0)
                            .range(-360.0..=360.0)
                            .suffix("°"),
                    )
                    .changed()
                {
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
                if ui
                    .add(
                        egui::DragValue::new(&mut skew_x)
                            .speed(1.0)
                            .range(-89.0..=89.0)
                            .suffix("°"),
                    )
                    .changed()
                {
                    Self::set_transform(state, &id, |t| t.skew_x = skew_x);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Skew Y:");
                if ui
                    .add(
                        egui::DragValue::new(&mut skew_y)
                            .speed(1.0)
                            .range(-89.0..=89.0)
                            .suffix("°"),
                    )
                    .changed()
                {
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
                    t.x = 0.0;
                    t.y = 0.0;
                    t.scale_x = 1.0;
                    t.scale_y = 1.0;
                    t.rotation = 0.0;
                    t.skew_x = 0.0;
                    t.skew_y = 0.0;
                });
            }
        });
    }

    fn set_transform(
        state: &mut AppState,
        obj_id: &str,
        f: impl FnOnce(&mut crate::core::document::Transform),
    ) {
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
