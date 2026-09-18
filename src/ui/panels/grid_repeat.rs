use crate::core::state::AppState;
use egui::{RichText, Ui};

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
            ui.add(
                egui::DragValue::new(&mut state.repeat_start_angle)
                    .range(-360.0..=360.0)
                    .suffix("°"),
            );
        });

        ui.add_space(8.0);

        if ui.button("Create Grid Repeat").clicked() {
            if let Some(id) = state.selected_ids.first() {
                let obj = state
                    .document
                    .all_objects()
                    .find(|(_, o)| &o.id == id)
                    .map(|(_, o)| o.clone());
                if let Some(obj) = obj {
                    let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
                    for row in 0..state.repeat_rows {
                        for col in 0..state.repeat_cols {
                            if row == 0 && col == 0 {
                                continue;
                            }
                            let mut new_obj = obj.clone();
                            new_obj.id = uuid::Uuid::new_v4().to_string();
                            new_obj.name = format!("{} ({},{})", obj.name, col, row);
                            new_obj.transform.x += col as f64 * state.repeat_h_gap;
                            new_obj.transform.y += row as f64 * state.repeat_v_gap;
                            cmds.push(Box::new(
                                crate::core::history::AddObjectCommand::new(new_obj),
                            )
                                as Box<dyn crate::core::history::Command>);
                        }
                    }
                    if !cmds.is_empty() {
                        let batch = Box::new(crate::core::history::BatchCommand::new(
                            "Grid Repeat",
                            cmds,
                        ));
                        state.undo_manager.execute(batch, &mut state.document);
                    }
                }
            }
        }

        if ui.button("Create Radial Repeat").clicked() {
            if let Some(id) = state.selected_ids.first() {
                let obj = state
                    .document
                    .all_objects()
                    .find(|(_, o)| &o.id == id)
                    .map(|(_, o)| o.clone());
                if let Some(obj) = obj {
                    let angle_step = 360.0 / state.repeat_radial_count.max(1) as f64;
                    let start_rad = state.repeat_start_angle.to_radians();
                    let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
                    for i in 1..state.repeat_radial_count {
                        let angle = start_rad + (i as f64) * angle_step.to_radians();
                        let mut new_obj = obj.clone();
                        new_obj.id = uuid::Uuid::new_v4().to_string();
                        new_obj.name = format!("{} R{}", obj.name, i);
                        new_obj.transform.x =
                            obj.transform.x + angle.cos() * state.repeat_radial_radius;
                        new_obj.transform.y =
                            obj.transform.y + angle.sin() * state.repeat_radial_radius;
                        new_obj.transform.rotation = angle;
                        cmds.push(Box::new(
                            crate::core::history::AddObjectCommand::new(new_obj),
                        )
                            as Box<dyn crate::core::history::Command>);
                    }
                    if !cmds.is_empty() {
                        let batch = Box::new(crate::core::history::BatchCommand::new(
                            "Radial Repeat",
                            cmds,
                        ));
                        state.undo_manager.execute(batch, &mut state.document);
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// ColorHarmonyPanel: Color harmony / complementary / analogous schemes
// ═══════════════════════════════════════════════════════════════════
