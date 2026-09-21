//! Brush panel: calligraphy / art / pattern brushes applied as baked
//! outlines (one AddObject per spine, fully undoable).

use crate::core::brush::{builtin_motif, apply_brush, BrushDefinition, BrushKind};
use crate::core::document::{Object, ObjectType};
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct BrushPanel;

fn current_def(state: &AppState) -> Option<BrushDefinition> {
    match state.brush_kind_idx {
        0 => Some(BrushDefinition::calligraphy(
            state.brush_angle,
            state.brush_roundness / 100.0,
            state.brush_size,
        )),
        1 => {
            let artwork = builtin_motif(&state.brush_motif)?;
            Some(BrushDefinition {
                name: format!("Art ({})", state.brush_motif),
                kind: BrushKind::Art { artwork },
            })
        }
        _ => {
            let artwork = builtin_motif(&state.brush_motif)?;
            Some(BrushDefinition {
                name: format!("Pattern ({})", state.brush_motif),
                kind: BrushKind::Pattern {
                    artwork,
                    spacing: state.brush_spacing.max(1.0),
                    scale: state.brush_scale.clamp(0.1, 10.0),
                },
            })
        }
    }
}

impl BrushPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🖌️ Brushes").strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            for (i, label) in ["Calligraphy", "Art", "Pattern"].iter().enumerate() {
                if ui
                    .selectable_label(state.brush_kind_idx == i, *label)
                    .clicked()
                {
                    state.brush_kind_idx = i;
                }
            }
        });
        ui.add_space(4.0);

        match state.brush_kind_idx {
            0 => {
                ui.horizontal(|ui| {
                    ui.label("Angle:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_angle)
                            .range(-90.0..=90.0)
                            .suffix("°"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Roundness:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_roundness)
                            .range(5.0..=100.0)
                            .suffix("%"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.add(
                        egui::DragValue::new(&mut state.brush_size)
                            .range(1.0..=200.0)
                            .suffix("pt"),
                    );
                });
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.label("Motif:");
                    for motif in ["arrow", "leaf", "wave"] {
                        if ui
                            .selectable_label(state.brush_motif == motif, motif)
                            .clicked()
                        {
                            state.brush_motif = motif.to_string();
                        }
                    }
                });
                if state.brush_kind_idx == 1 {
                    ui.label(RichText::new("Motif height := path stroke width.").weak().size(11.0));
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Spacing:");
                        ui.add(
                            egui::DragValue::new(&mut state.brush_spacing)
                                .range(1.0..=500.0),
                        );
                        ui.label("Scale:");
                        ui.add(
                            egui::DragValue::new(&mut state.brush_scale)
                                .range(0.1..=10.0),
                        );
                    });
                }
            }
        }
        ui.add_space(4.0);

        let has_sel = !state.selected_ids.is_empty();
        if ui
            .add_enabled(has_sel, egui::Button::new("Apply Brush to Selection"))
            .clicked()
        {
            if let Some(def) = current_def(state) {
                let targets: Vec<Object> = state
                    .selected_ids
                    .iter()
                    .filter_map(|id| {
                        state
                            .document
                            .all_objects()
                            .find(|(_, o)| &o.id == id)
                            .map(|(_, o)| o.clone())
                    })
                    .collect();
                let mut applied = 0;
                for obj in targets {
                    // Brushing a text/image bbox is never what the user
                    // wants — those need outlining/rasterizing first.
                    if matches!(
                        obj.object_type,
                        ObjectType::Text { .. }
                            | ObjectType::Image { .. }
                            | ObjectType::PixelArt(_)
                    ) {
                        continue;
                    }
                    let spine = obj.to_path_data();
                    if spine.elements.is_empty() {
                        continue;
                    }
                    let stroke_w = obj.stroke.as_ref().map(|s| s.width).unwrap_or(2.0);
                    if let Some(mut brushed) = apply_brush(&spine, &def, stroke_w) {
                        brushed.fill = obj
                            .fill
                            .clone()
                            .or_else(|| {
                                Some(crate::core::path::FillStyle::solid(state.fill_color))
                            });
                        let mut new_obj =
                            Object::new_path(&format!("{} (Brush)", obj.name), brushed);
                        new_obj.transform = obj.transform.clone();
                        let cmd =
                            Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        applied += 1;
                    }
                }
                if applied > 0 {
                    state.notify_success(format!("ブラシを適用しました（{applied}件）"));
                } else {
                    state.notify_error("適用できるパスがありません");
                }
            }
        }
        if !has_sel {
            ui.label(RichText::new("Select a path or shape first.").weak().size(11.0));
        }
    }
}
