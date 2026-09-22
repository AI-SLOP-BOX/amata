use crate::core::auto_layout::{AutoLayout, AutoLayoutDirection};
use crate::core::document::object::{Object, ObjectType};
use crate::core::history::{BatchCommand, Command, ObjectCommand};
use crate::core::state::AppState;
use egui::Ui;

/// Panel for Figma-style Auto Layout on the selected group.
/// Also creates an auto-layout group from a multi-selection.
pub struct AutoLayoutPanel;

impl AutoLayoutPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading("Auto Layout");
        ui.add_space(6.0);

        let selected: Vec<String> = state.selected_ids.clone();

        // Multi-select → create group with auto layout.
        let group_target = if selected.len() >= 2 {
            if ui.button("Add auto layout (group selection)").clicked() {
                create_from_selection(state);
            }
            ui.separator();
            selected
                .first()
                .and_then(|id| state.document.find_object(id))
                .filter(|o| matches!(o.object_type, ObjectType::Group(_)))
                .map(|o| o.id.clone())
        } else {
            selected
                .first()
                .and_then(|id| state.document.find_object(id))
                .and_then(|o| {
                    if matches!(o.object_type, ObjectType::Group(_)) {
                        Some(o.id.clone())
                    } else {
                        None
                    }
                })
        };

        let Some(group_id) = group_target else {
            ui.label("Select a group (or multi-select objects) to apply Auto Layout.");
            return;
        };

        // Snapshot for editing params.
        let Some(group) = state.document.find_object(&group_id) else {
            return;
        };
        if !matches!(group.object_type, ObjectType::Group(ref c) if !c.is_empty()) {
            ui.label("Selected group is empty.");
            return;
        }

        let mut layout = group
            .auto_layout
            .unwrap_or_else(AutoLayout::default);
        let mut dirty = false;

        egui::Grid::new("auto_layout_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Direction");
                let mut horiz = layout.direction == AutoLayoutDirection::Horizontal;
                if ui
                    .selectable_value(&mut horiz, true, "Horizontal")
                    .changed()
                {
                    layout.direction = AutoLayoutDirection::Horizontal;
                    dirty = true;
                }
                if ui
                    .selectable_value(&mut horiz, false, "Vertical")
                    .changed()
                {
                    layout.direction = AutoLayoutDirection::Vertical;
                    dirty = true;
                }
                ui.end_row();

                ui.label("Gap");
                let mut gap = layout.gap as f32;
                if ui
                    .add(egui::DragValue::new(&mut gap).range(0.0..=10_000.0))
                    .changed()
                {
                    layout.gap = gap as f64;
                    dirty = true;
                }
                ui.end_row();

                ui.label("Padding");
                let mut pad = [
                    layout.padding_top as f32,
                    layout.padding_right as f32,
                    layout.padding_bottom as f32,
                    layout.padding_left as f32,
                ];
                let mut pad_changed = false;
                ui.horizontal(|ui| {
                    for (i, label) in ["T", "R", "B", "L"].iter().enumerate() {
                        if ui
                            .add(
                                egui::DragValue::new(&mut pad[i])
                                    .range(0.0..=10_000.0)
                                    .prefix(format!("{label} ")),
                            )
                            .changed()
                        {
                            pad_changed = true;
                        }
                    }
                });
                if pad_changed {
                    layout.padding_top = pad[0] as f64;
                    layout.padding_right = pad[1] as f64;
                    layout.padding_bottom = pad[2] as f64;
                    layout.padding_left = pad[3] as f64;
                    dirty = true;
                }
                ui.end_row();

                ui.label("Reverse");
                if ui
                    .checkbox(&mut layout.reverse, "reverse order")
                    .changed()
                {
                    dirty = true;
                }
                ui.end_row();
            });

        layout.normalize();

        if dirty || group.auto_layout != Some(layout) {
            apply_auto_layout(state, &group_id, layout, "Auto Layout");
        }
    }
}

fn create_from_selection(state: &mut AppState) {
    let layout = AutoLayout::default();
    state.replace_selected("Add Auto Layout", |objects| {
        if objects.len() < 2 {
            return None;
        }
        let mut group = Object::new_group("Auto Layout", objects);
        group.auto_layout = Some(layout);
        if let ObjectType::Group(children) = &mut group.object_type {
            let moves = compute_moves(&layout, children);
            for (i, new_tx, new_ty) in moves {
                children[i].transform.x = new_tx;
                children[i].transform.y = new_ty;
            }
        }
        let id = group.id.clone();
        Some((vec![group], vec![id]))
    });
}

/// Compute moves as (child_index, new_tx, new_ty) for children of a group
/// in group-local space (child.bounding_box uses only child transform).
fn compute_moves(layout: &AutoLayout, children: &[Object]) -> Vec<(usize, f64, f64)> {
    let items: Vec<(String, f64, f64, f64, f64, f64)> = children
        .iter()
        .filter_map(|c| {
            let (min, max) = c.bounding_box()?;
            let (extent, min_along) = if layout.direction == AutoLayoutDirection::Horizontal {
                (max.x - min.x, min.x)
            } else {
                (max.y - min.y, min.y)
            };
            // cross min is the other axis min
            let _ = min_along;
            let cross_min = if layout.direction == AutoLayoutDirection::Horizontal {
                min.y
            } else {
                min.x
            };
            let main_min = if layout.direction == AutoLayoutDirection::Horizontal {
                min.x
            } else {
                min.y
            };
            Some((c.id.clone(), main_min, cross_min, extent, c.transform.x, c.transform.y))
        })
        .collect();

    // layout_positions returns absolute (id, new_tx, new_ty)
    let positioned = layout.layout_positions(&items);
    let mut out = Vec::with_capacity(positioned.len());
    for (id, new_tx, new_ty) in positioned {
        if let Some(idx) = children.iter().position(|c| c.id == id) {
            out.push((idx, new_tx, new_ty));
        }
    }
    out
}

fn apply_auto_layout(state: &mut AppState, group_id: &str, layout: AutoLayout, label: &str) {
    let Some(old_group) = state.document.find_object(group_id).cloned() else {
        return;
    };
    let children = match &old_group.object_type {
        ObjectType::Group(c) if !c.is_empty() => c.clone(),
        _ => return,
    };

    let moves = compute_moves(&layout, &children);
    let mut new_group = old_group.clone();
    new_group.auto_layout = Some(layout);
    if let ObjectType::Group(kids) = &mut new_group.object_type {
        for (idx, new_tx, new_ty) in &moves {
            if let Some(c) = kids.get_mut(*idx) {
                c.transform.x = *new_tx;
                c.transform.y = *new_ty;
            }
        }
    }

    // One batch: swap whole group (params + child positions) — undo restores all.
    if new_group.auto_layout == old_group.auto_layout && moves.is_empty() {
        return;
    }
    let cmds: Vec<Box<dyn Command>> = vec![Box::new(ObjectCommand {
        object_id: group_id.to_string(),
        old_obj: old_group,
        new_obj: new_group,
    })];
    let batch = BatchCommand::new(label.to_string(), cmds);
    state
        .undo_manager
        .execute(Box::new(batch), &mut state.document);
}
