use crate::core::boolean::{execute_pathfinder, BooleanOp};
use crate::core::document::{Document, Object, ObjectType};
use crate::core::morph::morph_paths;
use crate::core::offset::{offset_path, outline_stroke};
use crate::core::state::AppState;
use egui::{Color32, Pos2, RichText, Ui, Vec2};

/// One visible row in the layer tree (flattened so rendering does not
/// hold a `Document` borrow while mutating UI state).
#[derive(Clone)]
struct TreeRow {
    /// Parent object id; `None` = layer top level.
    #[allow(dead_code)]
    parent: Option<String>,
    id: String,
    name: String,
    icon: &'static str,
    depth: usize,
    is_group: bool,
    visible: bool,
    locked: bool,
    selected: bool,
}

/// Deferred mutations collected during the frame, applied after all
/// immutable document borrows end.
enum TreeAction {
    Select(String),
    SetActiveLayer(usize),
    StartRename(String),
    /// Keep the rename buffer in sync every frame while editing.
    SyncRename(Option<(String, String)>),
    CommitRename(String, String),
    CancelRename,
    ToggleCollapse(String),
    ToggleVis(String),
    ToggleLock(String),
    Delete(String),
    /// Move one step toward the front (higher sibling index).
    BringForward(String),
    /// Move one step toward the back (lower sibling index).
    SendBackward(String),
    Drop {
        dragged: String,
        target: DropTarget,
    },
}

#[derive(Debug, Clone)]
enum DropTarget {
    /// Insert immediately before this object (same parent as the target).
    Before(String),
    /// Insert immediately after this object.
    After(String),
    /// Append as the last child of this group / clipping mask.
    Into(String),
    /// Append to this layer's top level (front-most).
    LayerEnd(usize),
}

fn type_icon(obj: &Object) -> &'static str {
    match &obj.object_type {
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
        ObjectType::PixelArt(_) => "👾",
        ObjectType::GradientMesh(_) => "🌈",
        ObjectType::TextOnPath { .. } => "↻",
        ObjectType::Envelope { .. } => "🌀",
    }
}

fn flatten_tree(
    objects: &[Object],
    layer_idx: usize,
    parent: Option<&str>,
    depth: usize,
    collapsed: &std::collections::HashSet<String>,
    selected: &[String],
    out: &mut Vec<TreeRow>,
) {
    for obj in objects {
        let children = Document::children_of(obj);
        let is_group = !children.is_empty()
            || matches!(
                obj.object_type,
                ObjectType::Group(_) | ObjectType::ClippingMask { .. }
            );
        out.push(TreeRow {
            parent: parent.map(|p| p.to_string()),
            id: obj.id.clone(),
            name: obj.name.clone(),
            icon: type_icon(obj),
            depth,
            is_group,
            visible: obj.visible,
            locked: obj.locked,
            selected: selected.contains(&obj.id),
        });
        if is_group && !collapsed.contains(&obj.id) {
            flatten_tree(
                children,
                layer_idx,
                Some(&obj.id),
                depth + 1,
                collapsed,
                selected,
                out,
            );
        }
    }
}

/// Map a drop zone onto insertion coordinates in the current document,
/// rejecting cycles and self-drops. `new_index` is the insertion index
/// *after* the dragged object is removed from its source container.
fn plan_reparent(
    doc: &Document,
    dragged: &str,
    target: &DropTarget,
) -> Option<crate::core::history::ReparentObjectCommand> {
    if doc.find_object(dragged).is_none() {
        return None;
    }
    let (old_parent, old_layer, old_index) = doc.parent_of(dragged)?;

    let (new_parent, new_layer, raw_index): (Option<String>, usize, usize) = match target {
        DropTarget::Before(id) | DropTarget::After(id) => {
            if id == dragged || doc.is_descendant_of(id, dragged) {
                return None;
            }
            let (p, l, i) = doc.parent_of(id)?;
            let raw = match target {
                DropTarget::Before(_) => i,
                _ => i + 1,
            };
            (p, l, raw)
        }
        DropTarget::Into(gid) => {
            if gid == dragged || doc.is_descendant_of(gid, dragged) {
                return None;
            }
            let g = doc.find_object(gid)?;
            let (_, l, _) = doc.parent_of(gid)?;
            let len = Document::children_of(g).len();
            (Some(gid.clone()), l, len)
        }
        DropTarget::LayerEnd(li) => {
            let layer = doc.layers.get(*li)?;
            (None, *li, layer.objects.len())
        }
    };

    let same = old_parent == new_parent && old_layer == new_layer;
    let new_index = if same && old_index < raw_index {
        raw_index - 1
    } else {
        raw_index
    };

    Some(crate::core::history::ReparentObjectCommand {
        object_id: dragged.to_string(),
        old_parent,
        old_layer,
        old_index,
        new_parent,
        new_layer,
        new_index,
    })
}

/// Classify where a pointer over `rect` would drop: top third → before,
/// bottom third → after, middle of a group → into.
fn drop_target_from_pointer(
    rect: egui::Rect,
    pointer: Pos2,
    row: &TreeRow,
) -> Option<DropTarget> {
    if !rect.contains(pointer) {
        return None;
    }
    let t = (pointer.y - rect.top()) / rect.height().max(1.0);
    if row.is_group && t > 0.33 && t < 0.67 {
        Some(DropTarget::Into(row.id.clone()))
    } else if t <= 0.5 {
        Some(DropTarget::Before(row.id.clone()))
    } else {
        Some(DropTarget::After(row.id.clone()))
    }
}

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

        let mut actions: Vec<TreeAction> = Vec::new();
        let mut to_add_layer = false;
        let mut to_remove_layer = false;
        let mut to_move_layer_up: Option<usize> = None;
        let mut to_move_layer_down: Option<usize> = None;
        let mut to_duplicate_layer: Option<usize> = None;
        let mut to_set_layer_opacity: Option<(String, f32, bool)> = None;
        let mut to_commit_layer = false;
        let mut layer_drop: Vec<(String, DropTarget)> = Vec::new();

        // Rename buffer lives across the frame so TextEdit stays controlled.
        let mut rename_buf = state
            .tree_rename
            .as_ref()
            .map(|(_, b)| b.clone())
            .unwrap_or_default();
        let rename_id = state.tree_rename.as_ref().map(|(i, _)| i.clone());
        let mut rename_focus_done = state.tree_rename_focused;

        // Flatten every layer's object tree (collapsed groups omitted).
        let collapsed = state.tree_collapsed.clone();
        let selected = state.selected_ids.clone();
        let mut rows: Vec<TreeRow> = Vec::new();
        for (li, layer) in state.document.layers.iter().enumerate() {
            flatten_tree(
                &layer.objects,
                li,
                None,
                0,
                &collapsed,
                &selected,
                &mut rows,
            );
        }

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

            let layer_row = ui.horizontal(|ui| {
                if ui.selectable_label(is_active, text).clicked() {
                    actions.push(TreeAction::SetActiveLayer(i));
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
                    actions.push(TreeAction::ToggleVis(format!("layer:{i}")));
                }

                let lock_icon = if layer.locked { "🔒" } else { "🔓" };
                if ui
                    .small_button(lock_icon)
                    .on_hover_text("Toggle Lock")
                    .clicked()
                {
                    actions.push(TreeAction::ToggleLock(format!("layer:{i}")));
                }

                if ui
                    .small_button("⧉")
                    .on_hover_text("Duplicate Layer")
                    .clicked()
                {
                    to_duplicate_layer = Some(i);
                }
            })
            .response;

            if let Some(payload) = layer_row.dnd_release_payload::<String>() {
                layer_drop.push(((*payload).clone(), DropTarget::LayerEnd(i)));
            } else if layer_row
                .dnd_hover_payload::<String>()
                .is_some()
                && ui.input(|inp| inp.pointer.any_down())
            {
                ui.painter().rect_filled(
                    layer_row.rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(20, 115, 230, 40),
                );
            }

            if is_active {
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
                            to_set_layer_opacity = Some((layer_id, lop, op_resp.dragged()));
                        }
                        if op_resp.drag_stopped() {
                            to_commit_layer = true;
                        }
                    });
                });
            }
        }

        // ─── Object tree rows (all layers, recursive depth) ───
        let pointer = ui.ctx().pointer_hover_pos();

        for row in &rows {
            let mut row_actions: Vec<TreeAction> = Vec::new();
            let mut local_drop: Option<DropTarget> = None;

            ui.horizontal(|ui| {
                ui.add_space(row.depth as f32 * 14.0);

                // Collapse toggle for groups.
                if row.is_group {
                    let open = !state.tree_collapsed.contains(&row.id);
                    let arrow = if open { "▾" } else { "▸" };
                    if ui
                        .small_button(arrow)
                        .on_hover_text(if open { "Collapse" } else { "Expand" })
                        .clicked()
                    {
                        row_actions.push(TreeAction::ToggleCollapse(row.id.clone()));
                    }
                } else {
                    ui.add_space(18.0);
                }

                // Drag source wrapping name + icon.
                let label = format!("{} {}", row.icon, row.name);
                let drag_id = egui::Id::new(("tree_item", row.id.clone()));
                let inner = ui.dnd_drag_source(drag_id, row.id.clone(), |ui| {
                    if rename_id.as_deref() == Some(row.id.as_str()) {
                        let te = ui.add(
                            egui::TextEdit::singleline(&mut rename_buf)
                                .desired_width(120.0)
                                .hint_text("名前"),
                        );
                        if !rename_focus_done {
                            te.request_focus();
                            rename_focus_done = true;
                        }
                        if te.lost_focus() {
                            if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) {
                                row_actions.push(TreeAction::CancelRename);
                            } else {
                                row_actions.push(TreeAction::CommitRename(
                                    row.id.clone(),
                                    rename_buf.clone(),
                                ));
                            }
                        } else {
                            row_actions.push(TreeAction::SyncRename(Some((
                                row.id.clone(),
                                rename_buf.clone(),
                            ))));
                        }
                    } else {
                        let resp = ui.selectable_label(row.selected, &label);
                        if resp.clicked() {
                            row_actions.push(TreeAction::Select(row.id.clone()));
                        }
                        if resp.double_clicked() {
                            rename_buf = row.name.clone();
                            row_actions.push(TreeAction::StartRename(row.id.clone()));
                        }
                    }
                });

                // Drop classification over the drag-source body.
                if let Some(ptr) = pointer {
                    if let Some(t) = drop_target_from_pointer(inner.response.rect, ptr, row) {
                        if inner.response.dnd_hover_payload::<String>().is_some() {
                            let y = match &t {
                                DropTarget::Before(_) => inner.response.rect.top(),
                                DropTarget::After(_) => inner.response.rect.bottom(),
                                DropTarget::Into(_) => inner.response.rect.center().y,
                                DropTarget::LayerEnd(_) => inner.response.rect.top(),
                            };
                            let c = Color32::from_rgb(20, 115, 230);
                            if matches!(t, DropTarget::Into(_)) {
                                ui.painter().rect_filled(
                                    inner.response.rect,
                                    2.0,
                                    Color32::from_rgba_unmultiplied(20, 115, 230, 35),
                                );
                            } else {
                                ui.painter().line_segment(
                                    [
                                        Pos2::new(inner.response.rect.left(), y),
                                        Pos2::new(inner.response.rect.right(), y),
                                    ],
                                    egui::Stroke::new(2.0_f32, c),
                                );
                            }
                        }
                        local_drop = Some(t);
                    }
                }
                if let Some(payload) = inner.response.dnd_release_payload::<String>() {
                    if let Some(t) = pointer
                        .and_then(|ptr| drop_target_from_pointer(inner.response.rect, ptr, row))
                    {
                        local_drop = Some(t);
                    }
                    row_actions.push(TreeAction::Drop {
                        dragged: (*payload).clone(),
                        target: local_drop.clone().unwrap_or_else(|| {
                            DropTarget::Before(row.id.clone())
                        }),
                    });
                }

                // Visibility / lock / reorder / delete
                let o_vis_icon = if row.visible { "👁" } else { "🚫" };
                if ui
                    .small_button(o_vis_icon)
                    .on_hover_text("Toggle Object Visibility")
                    .clicked()
                {
                    row_actions.push(TreeAction::ToggleVis(row.id.clone()));
                }

                let o_lock_icon = if row.locked { "🔒" } else { "🔓" };
                if ui
                    .small_button(o_lock_icon)
                    .on_hover_text("Toggle Object Lock")
                    .clicked()
                {
                    row_actions.push(TreeAction::ToggleLock(row.id.clone()));
                }

                if ui.small_button("↑").on_hover_text("Bring Forward").clicked() {
                    row_actions.push(TreeAction::BringForward(row.id.clone()));
                }
                if ui.small_button("↓").on_hover_text("Send Backward").clicked() {
                    row_actions.push(TreeAction::SendBackward(row.id.clone()));
                }

                if ui.small_button("×").on_hover_text("Delete").clicked() {
                    row_actions.push(TreeAction::Delete(row.id.clone()));
                }
            });

            actions.extend(row_actions);
        }

        // Persist rename focus once the TextEdit has claimed it this session.
        if state.tree_rename.is_some() {
            state.tree_rename_focused = rename_focus_done;
        } else {
            state.tree_rename_focused = false;
        }

        // ─── Apply deferred actions ───
        for (dragged, target) in layer_drop {
            actions.push(TreeAction::Drop { dragged, target });
        }

        for action in actions {
            match action {
                TreeAction::Select(id) => {
                    state.selected_ids.clear();
                    state.selected_ids.push(id);
                }
                TreeAction::SetActiveLayer(i) => {
                    state.document.active_layer_idx = i;
                }
                TreeAction::StartRename(id) => {
                    let name = state
                        .document
                        .find_object(&id)
                        .map(|o| o.name.clone())
                        .unwrap_or_default();
                    state.tree_rename = Some((id, name));
                    state.tree_rename_focused = false;
                }
                TreeAction::SyncRename(v) => {
                    if state.tree_rename.is_some() {
                        state.tree_rename = v;
                    }
                }
                TreeAction::CommitRename(id, name) => {
                    state.tree_rename = None;
                    state.tree_rename_focused = false;
                    let name = name.trim().to_string();
                    if !name.is_empty() {
                        state.ensure_object_snapshot(&id);
                        if let Some(o) = state.document.find_object_mut(&id) {
                            o.name = name;
                        }
                        state.commit_object_edits("Rename Object");
                    }
                }
                TreeAction::CancelRename => {
                    state.tree_rename = None;
                    state.tree_rename_focused = false;
                }
                TreeAction::ToggleCollapse(id) => {
                    if !state.tree_collapsed.remove(&id) {
                        state.tree_collapsed.insert(id);
                    }
                }
                TreeAction::ToggleVis(id) => {
                    if let Some(rest) = id.strip_prefix("layer:") {
                        if let Ok(li) = rest.parse::<usize>() {
                            if let Some(l) = state.document.layers.get_mut(li) {
                                l.visible = !l.visible;
                            }
                        }
                    } else {
                        for (_, obj) in state.document.all_objects_mut() {
                            if obj.id == id {
                                obj.visible = !obj.visible;
                                break;
                            }
                        }
                    }
                }
                TreeAction::ToggleLock(id) => {
                    if let Some(rest) = id.strip_prefix("layer:") {
                        if let Ok(li) = rest.parse::<usize>() {
                            if let Some(l) = state.document.layers.get_mut(li) {
                                l.locked = !l.locked;
                            }
                        }
                    } else {
                        for (_, obj) in state.document.all_objects_mut() {
                            if obj.id == id {
                                obj.locked = !obj.locked;
                                break;
                            }
                        }
                    }
                }
                TreeAction::Delete(id) => {
                    if let Some(obj) = state.document.find_object(&id).cloned() {
                        let cmd = crate::core::history::RemoveObjectCommand::located(
                            obj,
                            &state.document,
                        );
                        state
                            .undo_manager
                            .execute(Box::new(cmd), &mut state.document);
                        state.selected_ids.retain(|s| s != &id);
                    }
                }
                TreeAction::BringForward(id) => {
                    let Some((parent, layer, idx)) = state.document.parent_of(&id) else {
                        continue;
                    };
                    let next_id = match &parent {
                        Some(pid) => state
                            .document
                            .find_object(pid)
                            .and_then(|p| Document::children_of(p).get(idx + 1))
                            .map(|c| c.id.clone()),
                        None => state.document.layers[layer]
                            .objects
                            .get(idx + 1)
                            .map(|o| o.id.clone()),
                    };
                    if let Some(next_id) = next_id {
                        if let Some(cmd) =
                            plan_reparent(&state.document, &id, &DropTarget::After(next_id))
                        {
                            state
                                .undo_manager
                                .execute(Box::new(cmd), &mut state.document);
                        }
                    }
                }
                TreeAction::SendBackward(id) => {
                    let Some((parent, layer, idx)) = state.document.parent_of(&id) else {
                        continue;
                    };
                    if idx == 0 {
                        continue;
                    }
                    let prev_id = match &parent {
                        Some(pid) => state
                            .document
                            .find_object(pid)
                            .and_then(|p| Document::children_of(p).get(idx - 1))
                            .map(|c| c.id.clone()),
                        None => state.document.layers[layer]
                            .objects
                            .get(idx - 1)
                            .map(|o| o.id.clone()),
                    };
                    if let Some(prev_id) = prev_id {
                        if let Some(cmd) =
                            plan_reparent(&state.document, &id, &DropTarget::Before(prev_id))
                        {
                            state
                                .undo_manager
                                .execute(Box::new(cmd), &mut state.document);
                        }
                    }
                }
                TreeAction::Drop { dragged, target } => {
                    if let Some(cmd) = plan_reparent(&state.document, &dragged, &target) {
                        state
                            .undo_manager
                            .execute(Box::new(cmd), &mut state.document);
                    }
                }
            }
        }

        // Layer-level deferred ops (unchanged semantics).
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
        let name = format!("Pathfinder: {}", op.name());
        state.replace_selected(&name, |objects| {
            if objects.len() < 2 {
                return None;
            }
            let refs: Vec<&Object> = objects.iter().collect();
            execute_pathfinder(&refs, op).map(|result| {
                let new_id = result.id.clone();
                (vec![result], vec![new_id])
            })
        });
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
    /// Conversion runs before any mutation: failure leaves the document
    /// untouched.
    pub fn apply_compound(state: &mut AppState) {
        state.replace_selected("Make Compound Path", |objects| {
            if objects.len() < 2 {
                return None;
            }
            Object::make_compound_path(&objects).map(|compound| {
                let new_id = compound.id.clone();
                (vec![compound], vec![new_id])
            })
        });
    }

    /// Release a compound path back into parts as one step.
    pub fn apply_release_compound(state: &mut AppState) {
        state.replace_selected_where(
            "Release Compound",
            |o| o.release_compound_path().len() > 1,
            |objects| {
                let mut added = Vec::new();
                let mut new_ids = Vec::new();
                for obj in objects {
                    for part in obj.release_compound_path() {
                        new_ids.push(part.id.clone());
                        added.push(part);
                    }
                }
                Some((added, new_ids))
            },
        );
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
                                let removed =
                                    crate::core::history::collect_located_objects(
                                        &state.document,
                                        std::slice::from_ref(&obj.id),
                                    );
                                let cmd = Box::new(
                                    crate::core::history::ReplaceObjectsCommand::new(
                                        "Slice Object",
                                        removed,
                                        vec![part_a, part_b],
                                    ),
                                );
                                state.undo_manager.execute(cmd, &mut state.document);
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
                                let removed =
                                    crate::core::history::collect_located_objects(
                                        &state.document,
                                        std::slice::from_ref(&obj.id),
                                    );
                                let cmd = Box::new(
                                    crate::core::history::ReplaceObjectsCommand::new(
                                        "Slice Object",
                                        removed,
                                        vec![part_a, part_b],
                                    ),
                                );
                                state.undo_manager.execute(cmd, &mut state.document);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::document::Layer;

    fn rect(name: &str) -> Object {
        Object::new_rect(name, 0.0, 0.0, 10.0, 10.0, 0.0)
    }

    fn id_of(doc: &Document, name: &str) -> String {
        doc.layers[0]
            .objects
            .iter()
            .find(|o| o.name == name)
            .map(|o| o.id.clone())
            .unwrap()
    }

    #[test]
    fn plan_reparent_rejects_self_drop() {
        let mut doc = Document::default();
        doc.layers[0].objects.push(rect("A"));
        let a = id_of(&doc, "A");
        let cmd = plan_reparent(&doc, &a, &DropTarget::Into(a.clone()));
        assert!(cmd.is_none());
        let cmd = plan_reparent(&doc, &a, &DropTarget::Before(a.clone()));
        assert!(cmd.is_none());
    }

    #[test]
    fn plan_reparent_rejects_cycle_group_into_descendant() {
        let mut doc = Document::default();
        let leaf = rect("Leaf");
        let leaf_id = leaf.id.clone();
        let group = Object::new_group("G", vec![leaf]);
        let g_id = group.id.clone();
        doc.layers[0].objects.push(group);

        // Dropping the group before its own child would create a cycle.
        let cmd = plan_reparent(&doc, &g_id, &DropTarget::Before(leaf_id.clone()));
        assert!(cmd.is_none());
        let cmd = plan_reparent(&doc, &g_id, &DropTarget::After(leaf_id));
        assert!(cmd.is_none());
    }

    #[test]
    fn plan_reparent_same_container_after_adjusts_index() {
        let mut doc = Document::default();
        doc.layers[0].objects.push(rect("A"));
        doc.layers[0].objects.push(rect("B"));
        let a = id_of(&doc, "A");
        let b = id_of(&doc, "B");

        // Drop A after B: raw index would be 2 (B is 1, +1), same container
        // with old 0 < raw 2 → adjusted to 1 (post-removal insertion index).
        let cmd = plan_reparent(&doc, &a, &DropTarget::After(b)).unwrap();
        assert_eq!(cmd.old_index, 0);
        assert_eq!(cmd.new_index, 1);
        assert_eq!(cmd.new_parent, None);
        assert_eq!(cmd.new_layer, 0);
    }

    #[test]
    fn plan_reparent_into_group_sets_parent() {
        let mut doc = Document::default();
        doc.layers.push(Layer::new("Layer 2"));
        doc.layers[0].objects.push(rect("A"));
        doc.layers[0].objects.push(Object::new_group("G", vec![]));
        let a = id_of(&doc, "A");
        let g = id_of(&doc, "G");

        let cmd = plan_reparent(&doc, &a, &DropTarget::Into(g.clone())).unwrap();
        assert_eq!(cmd.new_parent, Some(g));
        assert_eq!(cmd.old_parent, None);
        assert_eq!(cmd.new_index, 0);
    }

    #[test]
    fn plan_reparent_missing_object_returns_none() {
        let doc = Document::default();
        assert!(plan_reparent(&doc, "nope", &DropTarget::LayerEnd(0)).is_none());
    }

    #[test]
    fn plan_reparent_layer_end_uses_layer_length() {
        let mut doc = Document::default();
        doc.layers.push(Layer::new("Layer 2"));
        doc.layers[0].objects.push(rect("A"));
        doc.layers[0].objects.push(rect("B"));
        doc.layers[1].objects.push(rect("C"));
        let a = id_of(&doc, "A");

        let cmd = plan_reparent(&doc, &a, &DropTarget::LayerEnd(1)).unwrap();
        assert_eq!(cmd.new_layer, 1);
        assert_eq!(cmd.new_parent, None);
        assert_eq!(cmd.new_index, 1);
        assert_eq!(cmd.old_layer, 0);
    }

    #[test]
    fn flatten_tree_marks_groups_and_depth() {
        let leaf = rect("Leaf");
        let group = Object::new_group("G", vec![leaf]);
        let objects = vec![rect("Top"), group];
        let collapsed = std::collections::HashSet::new();
        let mut rows = Vec::new();
        flatten_tree(&objects, 0, None, 0, &collapsed, &[], &mut rows);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "Top");
        assert!(!rows[0].is_group);
        assert_eq!(rows[1].name, "G");
        assert!(rows[1].is_group);
        assert_eq!(rows[2].name, "Leaf");
        assert_eq!(rows[2].depth, 1);
        assert_eq!(rows[2].parent, rows[1].id.clone().into());
    }

    #[test]
    fn flatten_tree_hides_children_when_collapsed() {
        let group = Object::new_group("G", vec![rect("Leaf")]);
        let mut collapsed = std::collections::HashSet::new();
        collapsed.insert(group.id.clone());
        let objects = vec![group];
        let mut rows = Vec::new();
        flatten_tree(&objects, 0, None, 0, &collapsed, &[], &mut rows);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_group);
    }

    #[test]
    fn drop_target_from_pointer_zones() {
        let rect_box = egui::Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(100.0, 30.0));
        let row = TreeRow {
            parent: None,
            id: "g".into(),
            name: "G".into(),
            icon: "🗂",
            depth: 0,
            is_group: true,
            visible: true,
            locked: false,
            selected: false,
        };
        let leaf_row = TreeRow {
            is_group: false,
            ..row.clone()
        };

        // Top third → Before
        match drop_target_from_pointer(rect_box, Pos2::new(50.0, 5.0), &row) {
            Some(DropTarget::Before(id)) => assert_eq!(id, "g"),
            other => panic!("expected Before, got {:?}", other),
        }
        // Middle of group → Into
        match drop_target_from_pointer(rect_box, Pos2::new(50.0, 15.0), &row) {
            Some(DropTarget::Into(id)) => assert_eq!(id, "g"),
            other => panic!("expected Into, got {:?}", other),
        }
        // Middle of leaf → not Into; t <= 0.5 classifies as Before.
        match drop_target_from_pointer(rect_box, Pos2::new(50.0, 15.0), &leaf_row) {
            Some(DropTarget::Before(id)) => assert_eq!(id, "g"),
            other => panic!("expected Before, got {:?}", other),
        }
        // Just past middle of leaf → After.
        match drop_target_from_pointer(rect_box, Pos2::new(50.0, 16.0), &leaf_row) {
            Some(DropTarget::After(id)) => assert_eq!(id, "g"),
            other => panic!("expected After, got {:?}", other),
        }
        // Bottom → After
        match drop_target_from_pointer(rect_box, Pos2::new(50.0, 25.0), &row) {
            Some(DropTarget::After(_)) => {}
            other => panic!("expected After, got {:?}", other),
        }
        // Outside → None
        assert!(drop_target_from_pointer(rect_box, Pos2::new(500.0, 5.0), &row).is_none());
    }
}

