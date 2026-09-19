use crate::core::boolean::{execute_pathfinder, BooleanOp};
use crate::core::document::{Object, ObjectType};
use crate::core::morph::morph_paths;
use crate::core::offset::{offset_path, outline_stroke};
use crate::core::state::AppState;
use egui::{Color32, RichText, Ui, Vec2};

pub struct LayerPanel;

impl LayerPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.label(
            RichText::new("レイヤー")
                .strong()
                .size(13.0)
                .color(Color32::WHITE),
        );
        ui.separator();

        let layer_count = state.document.layers.len();
        let active_idx = state.document.active_layer_idx;

        let mut to_add_layer = false;
        let mut to_remove_layer = false;
        let mut to_move_layer_up: Option<usize> = None;
        let mut to_move_layer_down: Option<usize> = None;
        let mut to_move_obj_up: Option<(usize, usize)> = None;
        let mut to_move_obj_down: Option<(usize, usize)> = None;
        let mut to_duplicate_layer: Option<usize> = None;
        let mut to_select_obj: Option<String> = None;
        let mut to_remove_obj: Option<(usize, usize)> = None;
        let mut to_toggle_vis: Option<usize> = None;
        let mut to_toggle_lock: Option<usize> = None;
        let mut to_toggle_obj_vis: Option<String> = None;
        let mut to_toggle_obj_lock: Option<String> = None;
        let mut to_set_layer_opacity: Option<(String, f32, bool)> = None;
        let mut to_commit_layer = false;

        for (i, layer) in state.document.layers.iter().enumerate() {
            let is_active = i == active_idx;
            let obj_count = layer.objects.len();
            let text = if is_active {
                RichText::new(format!("📁 {} ({})", layer.name, obj_count))
                    .strong()
                    .color(Color32::from_rgb(100, 180, 255))
            } else {
                RichText::new(format!("📁 {} ({})", layer.name, obj_count))
            };

            ui.horizontal(|ui| {
                if ui.selectable_label(is_active, text).clicked() {
                    state.document.active_layer_idx = i;
                }

                if ui
                    .small_button("▲")
                    .on_hover_text("Move Layer Up")
                    .clicked()
                    && i + 1 < layer_count
                {
                    to_move_layer_up = Some(i);
                }
                if ui
                    .small_button("▼")
                    .on_hover_text("Move Layer Down")
                    .clicked()
                    && i > 0
                {
                    to_move_layer_down = Some(i);
                }

                let vis_icon = if layer.visible { "👁" } else { "🚫" };
                if ui
                    .small_button(vis_icon)
                    .on_hover_text("Toggle Visibility")
                    .clicked()
                {
                    to_toggle_vis = Some(i);
                }

                let lock_icon = if layer.locked { "🔒" } else { "🔓" };
                if ui
                    .small_button(lock_icon)
                    .on_hover_text("Toggle Lock")
                    .clicked()
                {
                    to_toggle_lock = Some(i);
                }

                if ui
                    .small_button("⧉")
                    .on_hover_text("Duplicate Layer")
                    .clicked()
                {
                    to_duplicate_layer = Some(i);
                }
            });

            if is_active {
                // Per-layer opacity with drag-coalesced undo. Previously the
                // model field existed but had no UI and was ignored by
                // canvas and vector export alike. Applied after the loop:
                // the layer iterator borrows the document, so &mut state
                // calls must be deferred like the other to_* actions.
                let layer_id = layer.id.clone();
                let mut lop = layer.opacity;
                ui.indent("layer_opacity", |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("不透明度")
                                .size(10.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)),
                        );
                        let op_resp = ui.add(
                            egui::Slider::new(&mut lop, 0.0..=1.0)
                                .show_value(true)
                                .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                        );
                        if op_resp.changed() {
                            to_set_layer_opacity =
                                Some((layer_id, lop, op_resp.dragged()));
                        }
                        if op_resp.drag_stopped() {
                            to_commit_layer = true;
                        }
                    });
                });
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
                            ObjectType::ClippingMask { .. } => "🎭",
                            ObjectType::Use { .. } => "❖",
                            ObjectType::Image { .. } => "🖼",
                        };
                        let obj_text = format!("{icon} {}", obj.name);

                        ui.horizontal(|ui| {
                            if ui.selectable_label(is_selected, &obj_text).clicked() {
                                to_select_obj = Some(obj.id.clone());
                            }

                            if ui
                                .small_button("↑")
                                .on_hover_text("Bring Forward")
                                .clicked()
                                && j + 1 < layer.objects.len()
                            {
                                to_move_obj_up = Some((i, j));
                            }
                            if ui
                                .small_button("↓")
                                .on_hover_text("Send Backward")
                                .clicked()
                                && j > 0
                            {
                                to_move_obj_down = Some((i, j));
                            }

                            let o_vis_icon = if obj.visible { "👁" } else { "🚫" };
                            if ui
                                .small_button(o_vis_icon)
                                .on_hover_text("Toggle Object Visibility")
                                .clicked()
                            {
                                to_toggle_obj_vis = Some(obj.id.clone());
                            }

                            let o_lock_icon = if obj.locked { "🔒" } else { "🔓" };
                            if ui
                                .small_button(o_lock_icon)
                                .on_hover_text("Toggle Object Lock")
                                .clicked()
                            {
                                to_toggle_obj_lock = Some(obj.id.clone());
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
            let cmd = Box::new(crate::core::history::RemoveObjectCommand::new(
                obj, layer_idx, obj_idx,
            ));
            state.undo_manager.execute(cmd, &mut state.document);
        }

        if let Some(i) = to_toggle_vis {
            state.document.layers[i].visible = !state.document.layers[i].visible;
        }

        if let Some(i) = to_toggle_lock {
            state.document.layers[i].locked = !state.document.layers[i].locked;
        }

        if let Some(id) = to_toggle_obj_vis {
            for (_, obj) in state.document.all_objects_mut() {
                if obj.id == id {
                    obj.visible = !obj.visible;
                    break;
                }
            }
        }

        if let Some(id) = to_toggle_obj_lock {
            for (_, obj) in state.document.all_objects_mut() {
                if obj.id == id {
                    obj.locked = !obj.locked;
                    break;
                }
            }
        }

        // Layer/object reorder + layer add/duplicate/delete all execute as
        // undoable commands (previously direct mutations: destructive and
        // invisible to dirty tracking).
        if let Some(i) = to_move_layer_up {
            let old_order: Vec<String> =
                state.document.layers.iter().map(|l| l.id.clone()).collect();
            state.document.move_layer_up(i);
            let new_order: Vec<String> =
                state.document.layers.iter().map(|l| l.id.clone()).collect();
            if old_order != new_order {
                state.undo_manager.execute(
                    Box::new(crate::core::history::ReorderLayersCommand {
                        old_order,
                        new_order,
                    }),
                    &mut state.document,
                );
            }
        }

        if let Some(i) = to_move_layer_down {
            let old_order: Vec<String> =
                state.document.layers.iter().map(|l| l.id.clone()).collect();
            state.document.move_layer_down(i);
            let new_order: Vec<String> =
                state.document.layers.iter().map(|l| l.id.clone()).collect();
            if old_order != new_order {
                state.undo_manager.execute(
                    Box::new(crate::core::history::ReorderLayersCommand {
                        old_order,
                        new_order,
                    }),
                    &mut state.document,
                );
            }
        }

        if let Some((l, o)) = to_move_obj_up {
            if let Some(layer) = state.document.layers.get(l) {
                let layer_id = layer.id.clone();
                let old_order: Vec<String> =
                    layer.objects.iter().map(|o| o.id.clone()).collect();
                state.document.move_object_up(l, o);
                let new_order: Vec<String> = state.document.layers[l]
                    .objects
                    .iter()
                    .map(|o| o.id.clone())
                    .collect();
                if old_order != new_order {
                    state.undo_manager.execute(
                        Box::new(crate::core::history::ReorderObjectsCommand {
                            layer_id,
                            old_order,
                            new_order,
                        }),
                        &mut state.document,
                    );
                }
            }
        }

        if let Some((l, o)) = to_move_obj_down {
            if let Some(layer) = state.document.layers.get(l) {
                let layer_id = layer.id.clone();
                let old_order: Vec<String> =
                    layer.objects.iter().map(|o| o.id.clone()).collect();
                state.document.move_object_down(l, o);
                let new_order: Vec<String> = state.document.layers[l]
                    .objects
                    .iter()
                    .map(|o| o.id.clone())
                    .collect();
                if old_order != new_order {
                    state.undo_manager.execute(
                        Box::new(crate::core::history::ReorderObjectsCommand {
                            layer_id,
                            old_order,
                            new_order,
                        }),
                        &mut state.document,
                    );
                }
            }
        }

        if let Some(i) = to_duplicate_layer {
            if let Some(src) = state.document.layers.get(i) {
                let new_name = format!("{} (copy)", src.name);
                let mut new_layer = crate::core::document::Layer::new(&new_name);
                for obj in &src.objects {
                    let mut dup = obj.clone();
                    dup.id = uuid::Uuid::new_v4().to_string();
                    dup.name = format!("{} (copy)", obj.name);
                    new_layer.objects.push(dup);
                }
                new_layer.visible = src.visible;
                new_layer.locked = src.locked;
                let prev_active = state.document.active_layer_idx;
                let index = state.document.layers.len();
                state.undo_manager.execute(
                    Box::new(crate::core::history::AddLayerCommand {
                        layer: new_layer,
                        index,
                        prev_active,
                    }),
                    &mut state.document,
                );
            }
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
            let prev_active = state.document.active_layer_idx;
            let index = state.document.layers.len();
            state.undo_manager.execute(
                Box::new(crate::core::history::AddLayerCommand {
                    layer: crate::core::document::Layer::new(&name),
                    index,
                    prev_active,
                }),
                &mut state.document,
            );
        }

        if let Some((lid, v, dragged)) = to_set_layer_opacity {
            state.ensure_layer_snapshot(&lid);
            if let Some(l) = state.document.layers.iter_mut().find(|l| l.id == lid) {
                l.opacity = v;
            }
            if !dragged {
                state.commit_layer_edits("Edit Layer");
            }
        }
        if to_commit_layer {
            state.commit_layer_edits("Edit Layer");
        }

        if to_remove_layer {
            let idx = state.document.active_layer_idx;
            if let Some(layer) = state.document.layers.get(idx).cloned() {
                state.undo_manager.execute(
                    Box::new(crate::core::history::RemoveLayerCommand {
                        layer,
                        index: idx,
                    }),
                    &mut state.document,
                );
                // Drop selection of objects that no longer exist anywhere.
                let alive: Vec<String> = state
                    .selected_ids
                    .iter()
                    .filter(|sid| state.document.find_object(sid).is_some())
                    .cloned()
                    .collect();
                state.selected_ids = alive;
            }
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
            let names: Vec<String> = state
                .undo_manager
                .undo_stack()
                .iter()
                .map(|c| c.name().to_string())
                .collect();
            for (i, name) in names.iter().enumerate() {
                let is_last = i == names.len() - 1;
                let text = if is_last {
                    RichText::new(format!("{}. {} ●", i + 1, name))
                        .strong()
                        .color(Color32::from_rgb(100, 180, 255))
                } else {
                    RichText::new(format!("{}. {}", i + 1, name))
                };
                ui.label(text);
            }
        });

        ui.collapsing(format!("Redo Stack ({})", redo_depth), |ui| {
            let names: Vec<String> = state
                .undo_manager
                .redo_stack()
                .iter()
                .rev()
                .map(|c| c.name().to_string())
                .collect();
            for (i, name) in names.iter().enumerate() {
                let text = RichText::new(format!("{}. {}", i + 1, name));
                ui.label(text);
            }
        });
    }
}

pub struct ClippingMaskPanel;

impl ClippingMaskPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("✂ Clipping Mask").strong());
        ui.add_space(4.0);

        let sel_count = state.selected_ids.len();
        let has_mask_shape = sel_count >= 2;

        ui.label("Select a mask shape (top) and content objects (below).");
        ui.label(
            RichText::new("Ctrl+7 or click below to create mask")
                .weak()
                .size(11.0),
        );
        ui.add_space(4.0);

        if ui
            .add_enabled(has_mask_shape, egui::Button::new("Create Clipping Mask"))
            .clicked()
        {
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

pub struct PathfinderPanel;

impl PathfinderPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("✂ Pathfinder").strong());
        ui.label(
            RichText::new("Combine 2 or more vector shapes")
                .weak()
                .size(11.0),
        );
        ui.add_space(4.0);

        let sel_count = state.selected_ids.len();
        let is_enabled = sel_count >= 2;

        let ops = [
            (BooleanOp::Union, "Unite", "Combine all shapes into one"),
            (
                BooleanOp::Subtract,
                "Minus Front",
                "Subtract front shapes from back",
            ),
            (BooleanOp::Intersect, "Intersect", "Keep overlapping area"),
            (BooleanOp::Exclude, "Exclude", "Exclude overlapping area"),
        ];

        ui.horizontal_wrapped(|ui| {
            for (op, name, tooltip) in ops {
                let btn =
                    egui::Button::new(RichText::new(format!("{} {}", op.icon(), name)).strong())
                        .min_size(Vec2::new(95.0, 26.0));

                let clicked = ui
                    .add_enabled(is_enabled, btn)
                    .on_hover_text(tooltip)
                    .clicked();

                if clicked {
                    Self::apply_op(state, op);
                }
            }
        });

        ui.add_space(4.0);
        ui.separator();
        ui.label(RichText::new("単純化・複合パス").strong().size(11.0));
        ui.horizontal(|ui| {
            ui.label("許容値:");
            ui.add(
                egui::Slider::new(&mut state.simplify_tolerance, 0.5..=50.0)
                    .show_value(true),
            );
            if ui
                .add_enabled(sel_count >= 1, egui::Button::new("単純化"))
                .on_hover_text("共線点を削減（ベジェは保持）")
                .clicked()
            {
                Self::apply_simplify(state, state.simplify_tolerance);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(is_enabled, egui::Button::new("⧉ 複合パス化"))
                .on_hover_text("重なりを中マド化 (EvenOdd)")
                .clicked()
            {
                Self::apply_compound(state);
            }
            if ui
                .add_enabled(sel_count >= 1, egui::Button::new("複合解除"))
                .on_hover_text("複合パスを分割")
                .clicked()
            {
                Self::apply_release_compound(state);
            }
        });
    }

    pub fn apply_op(state: &mut AppState, op: BooleanOp) {
        let mut selected_objs: Vec<Object> = Vec::new();
        for id in &state.selected_ids {
            if let Some(obj) = state.document.find_object(id) {
                selected_objs.push(obj.clone());
            }
        }

        if selected_objs.len() < 2 {
            return;
        }

        let obj_refs: Vec<&Object> = selected_objs.iter().collect();
        if let Some(result_obj) = execute_pathfinder(&obj_refs, op) {
            // Atomic: removals (with true locations) + result in one step.
            // Previously the sources were deleted outside any command, so
            // Undo removed the result while the originals stayed lost.
            let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
            for obj in &selected_objs {
                cmds.push(Box::new(
                    crate::core::history::RemoveObjectCommand::located(
                        obj.clone(),
                        &state.document,
                    ),
                )
                    as Box<dyn crate::core::history::Command>);
            }
            let new_id = result_obj.id.clone();
            cmds.push(Box::new(crate::core::history::AddObjectCommand::new(
                result_obj,
            ))
                as Box<dyn crate::core::history::Command>);
            let batch = Box::new(crate::core::history::BatchCommand::new(
                "Pathfinder",
                cmds,
            ));
            state.undo_manager.execute(batch, &mut state.document);
            state.selected_ids = vec![new_id];
        }
    }

    /// Simplify selected paths (Visvalingam, curves preserved) as one step.
    pub fn apply_simplify(state: &mut AppState, tolerance: f64) {
        let ids = state.selected_ids.clone();
        let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
        for id in &ids {
            if let Some(obj) = state.document.find_object(id) {
                if let ObjectType::Path(ref p) = obj.object_type {
                    let simplified =
                        crate::core::simplify::simplify_path_visvalingam(p, tolerance);
                    if simplified.elements != p.elements {
                        cmds.push(Box::new(
                            crate::core::history::ModifyPathCommand::new(
                                id.clone(),
                                p.elements.clone(),
                                simplified.elements,
                            ),
                        )
                            as Box<dyn crate::core::history::Command>);
                    }
                }
            }
        }
        if cmds.len() == 1 {
            let cmd = cmds.pop().unwrap();
            state.undo_manager.execute(cmd, &mut state.document);
        } else if !cmds.is_empty() {
            let batch = Box::new(crate::core::history::BatchCommand::new(
                "Simplify Path",
                cmds,
            ));
            state.undo_manager.execute(batch, &mut state.document);
        }
    }

    /// Combine selection into a compound path (EvenOdd holes) as one step.
    pub fn apply_compound(state: &mut AppState) {
        let mut selected_objs: Vec<Object> = Vec::new();
        for id in &state.selected_ids {
            if let Some(obj) = state.document.find_object(id) {
                selected_objs.push(obj.clone());
            }
        }
        if selected_objs.len() < 2 {
            return;
        }
        if let Some(compound) = Object::make_compound_path(&selected_objs) {
            let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
            for obj in &selected_objs {
                cmds.push(Box::new(
                    crate::core::history::RemoveObjectCommand::located(
                        obj.clone(),
                        &state.document,
                    ),
                )
                    as Box<dyn crate::core::history::Command>);
            }
            let new_id = compound.id.clone();
            cmds.push(Box::new(crate::core::history::AddObjectCommand::new(
                compound,
            ))
                as Box<dyn crate::core::history::Command>);
            let batch = Box::new(crate::core::history::BatchCommand::new(
                "Compound Path",
                cmds,
            ));
            state.undo_manager.execute(batch, &mut state.document);
            state.selected_ids = vec![new_id];
        }
    }

    /// Release a compound path back into parts as one step.
    pub fn apply_release_compound(state: &mut AppState) {
        let ids = state.selected_ids.clone();
        let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
        let mut new_ids = Vec::new();
        for id in &ids {
            if let Some(obj) = state.document.find_object(id) {
                let parts = obj.release_compound_path();
                if parts.len() > 1 {
                    cmds.push(Box::new(
                        crate::core::history::RemoveObjectCommand::located(
                            obj.clone(),
                            &state.document,
                        ),
                    )
                        as Box<dyn crate::core::history::Command>);
                    for part in parts {
                        new_ids.push(part.id.clone());
                        cmds.push(Box::new(crate::core::history::AddObjectCommand::new(
                            part,
                        ))
                            as Box<dyn crate::core::history::Command>);
                    }
                }
            }
        }
        if !cmds.is_empty() {
            let batch = Box::new(crate::core::history::BatchCommand::new(
                "Release Compound",
                cmds,
            ));
            state.undo_manager.execute(batch, &mut state.document);
            state.selected_ids = new_ids;
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
            if ui
                .add_enabled(has_sel, egui::Button::new("Outline Stroke"))
                .clicked()
            {
                let targets: Vec<crate::core::document::Object> = state
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

                for obj in targets {
                    let stroke_w = obj.stroke.as_ref().map(|s| s.width).unwrap_or(2.0);
                    let path = obj.to_path_data();
                    let outlined = outline_stroke(&path, stroke_w);
                    let mut new_obj =
                        Object::new_path(&format!("{} (Outlined)", obj.name), outlined);
                    new_obj.transform = obj.transform.clone();
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                    state.undo_manager.execute(cmd, &mut state.document);
                }
            }

            if ui
                .add_enabled(has_sel, egui::Button::new("Offset Path (+10px)"))
                .clicked()
            {
                let targets: Vec<crate::core::document::Object> = state
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
            ui.add(
                egui::Slider::new(&mut t, 0.0..=1.0)
                    .text("Morph (t)")
                    .step_by(0.05),
            );

            if ui.button("Create Morphed In-between Shape").clicked() {
                let obj1 = state
                    .document
                    .all_objects()
                    .find(|(_, o)| o.id == state.selected_ids[0])
                    .map(|(_, o)| o.clone());
                let obj2 = state
                    .document
                    .all_objects()
                    .find(|(_, o)| o.id == state.selected_ids[1])
                    .map(|(_, o)| o.clone());

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

pub struct KnifePanel;

impl KnifePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("✂️ Knife & Vector Slicer").strong());
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
                    .add_enabled(has_sel, egui::Button::new("Slice Horizontally"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        if let Some((min, max)) = obj.bounding_box() {
                            let mid_y = (min.y + max.y) * 0.5;
                            let p1 = crate::core::path::AnchorPoint::new(min.x - 10.0, mid_y);
                            let p2 = crate::core::path::AnchorPoint::new(max.x + 10.0, mid_y);

                            if let Some((part_a, part_b)) =
                                crate::core::knife::slice_object_with_line(obj, p1, p2)
                            {
                                // One atomic undo step with the true layer/position
                                // so Undo restores the original z-order.
                                let mut found = None;
                                for (l_idx, layer) in
                                    state.document.layers.iter().enumerate()
                                {
                                    if let Some(pos) = layer
                                        .objects
                                        .iter()
                                        .position(|o| o.id == obj.id)
                                    {
                                        found = Some((l_idx, pos));
                                        break;
                                    }
                                }
                                if let Some((l_idx, pos)) = found {
                                    let rm = Box::new(
                                        crate::core::history::RemoveObjectCommand::new(
                                            obj.clone(),
                                            l_idx,
                                            pos,
                                        ),
                                    )
                                        as Box<dyn crate::core::history::Command>;
                                    let add_a = Box::new(
                                        crate::core::history::AddObjectCommand::new(part_a),
                                    )
                                        as Box<dyn crate::core::history::Command>;
                                    let add_b = Box::new(
                                        crate::core::history::AddObjectCommand::new(part_b),
                                    )
                                        as Box<dyn crate::core::history::Command>;
                                    let batch = Box::new(
                                        crate::core::history::BatchCommand::new(
                                            "Slice Object",
                                            vec![rm, add_a, add_b],
                                        ),
                                    );
                                    state.undo_manager.execute(batch, &mut state.document);
                                }
                            }
                        }
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Slice Vertically"))
                    .clicked()
                {
                    if let Some(obj) = &target_obj {
                        if let Some((min, max)) = obj.bounding_box() {
                            let mid_x = (min.x + max.x) * 0.5;
                            let p1 = crate::core::path::AnchorPoint::new(mid_x, min.y - 10.0);
                            let p2 = crate::core::path::AnchorPoint::new(mid_x, max.y + 10.0);

                            if let Some((part_a, part_b)) =
                                crate::core::knife::slice_object_with_line(obj, p1, p2)
                            {
                                let mut found = None;
                                for (l_idx, layer) in
                                    state.document.layers.iter().enumerate()
                                {
                                    if let Some(pos) = layer
                                        .objects
                                        .iter()
                                        .position(|o| o.id == obj.id)
                                    {
                                        found = Some((l_idx, pos));
                                        break;
                                    }
                                }
                                if let Some((l_idx, pos)) = found {
                                    let rm = Box::new(
                                        crate::core::history::RemoveObjectCommand::new(
                                            obj.clone(),
                                            l_idx,
                                            pos,
                                        ),
                                    )
                                        as Box<dyn crate::core::history::Command>;
                                    let add_a = Box::new(
                                        crate::core::history::AddObjectCommand::new(part_a),
                                    )
                                        as Box<dyn crate::core::history::Command>;
                                    let add_b = Box::new(
                                        crate::core::history::AddObjectCommand::new(part_b),
                                    )
                                        as Box<dyn crate::core::history::Command>;
                                    let batch = Box::new(
                                        crate::core::history::BatchCommand::new(
                                            "Slice Object",
                                            vec![rm, add_a, add_b],
                                        ),
                                    );
                                    state.undo_manager.execute(batch, &mut state.document);
                                }
                            }
                        }
                    }
                }
            });
        } else {
            ui.label(
                RichText::new("Select an object to slice in half")
                    .weak()
                    .size(11.0),
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// SymbolsPanel: Reusable object library
// ═══════════════════════════════════════════════════════════════════
