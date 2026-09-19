use super::{zoom_to_fit, IrasuApp};
use crate::core::document::{Document, Object, ObjectType};
use crate::core::state::Tool;
use egui::{self};

/// Shared z-order mutation used by the arrange keyboard shortcuts.
fn arrange_move(sel: &[String], doc: &mut Document, forward: bool, jump: bool) {
    for id in sel {
        for layer in doc.layers.iter_mut() {
            if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                if jump {
                    let obj = layer.objects.remove(pos);
                    if forward {
                        layer.objects.push(obj);
                    } else {
                        layer.objects.insert(0, obj);
                    }
                } else if forward {
                    if pos + 1 < layer.objects.len() {
                        layer.objects.swap(pos, pos + 1);
                    }
                } else if pos > 0 {
                    layer.objects.swap(pos, pos - 1);
                }
                break;
            }
        }
    }
}
impl IrasuApp {
    pub(super) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Keyboard shortcuts
        ctx.input(|i| {
            if !i.modifiers.ctrl && !i.modifiers.mac_cmd && !i.modifiers.alt {
                if i.key_pressed(egui::Key::V) {
                    self.state.current_tool = Tool::Select;
                }
                if i.key_pressed(egui::Key::A) {
                    self.state.current_tool = Tool::Node;
                }
                if i.key_pressed(egui::Key::P) {
                    self.state.current_tool = Tool::Pen;
                }
                if i.key_pressed(egui::Key::N) {
                    self.state.current_tool = Tool::Pencil;
                }
                if i.key_pressed(egui::Key::U) {
                    self.state.current_tool = Tool::Rectangle;
                }
                if i.key_pressed(egui::Key::O) {
                    self.state.current_tool = Tool::Ellipse;
                }
                if i.key_pressed(egui::Key::S) {
                    self.state.current_tool = Tool::Star;
                }
                if i.key_pressed(egui::Key::G) {
                    self.state.current_tool = Tool::Polygon;
                }
                if i.key_pressed(egui::Key::L) {
                    self.state.current_tool = Tool::Line;
                }
                if i.key_pressed(egui::Key::T) {
                    self.state.current_tool = Tool::Text;
                }
                if i.key_pressed(egui::Key::I) {
                    self.state.previous_tool = self.state.current_tool;
                    self.state.current_tool = Tool::Eyedropper;
                }
                if i.key_pressed(egui::Key::H) {
                    self.state.current_tool = Tool::Hand;
                }
                if i.key_pressed(egui::Key::B) {
                    self.state.current_tool = Tool::Brush;
                }
                if i.key_pressed(egui::Key::E) {
                    self.state.current_tool = Tool::Eraser;
                }
                if i.key_pressed(egui::Key::M) {
                    self.state.current_tool = Tool::ShapeBuilder;
                }
                if i.key_pressed(egui::Key::Z) {
                    self.state.current_tool = Tool::Zoom;
                }

                // Default Colors (D key)
                if i.key_pressed(egui::Key::D) {
                    self.state.fill_color = [1.0, 1.0, 1.0, 1.0];
                    self.state.stroke_color = [0.0, 0.0, 0.0, 1.0];
                    self.state.stroke_width = 1.0;
                    for id in &self.state.selected_ids {
                        for (_, obj) in self.state.document.all_objects_mut() {
                            if &obj.id == id {
                                obj.fill = Some(crate::core::path::FillStyle::solid(
                                    self.state.fill_color,
                                ));
                                obj.stroke = Some(crate::core::path::StrokeStyle {
                                    color: self.state.stroke_color,
                                    width: self.state.stroke_width,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }

                // None / Transparent (/ key)
                if i.key_pressed(egui::Key::Slash) {
                    self.state.fill_color = [0.0, 0.0, 0.0, 0.0];
                    for id in &self.state.selected_ids {
                        for (_, obj) in self.state.document.all_objects_mut() {
                            if &obj.id == id {
                                obj.fill = None;
                            }
                        }
                    }
                }
            }

            // Swap Fill and Stroke (Shift+X)
            if i.modifiers.shift
                && !i.modifiers.ctrl
                && !i.modifiers.mac_cmd
                && i.key_pressed(egui::Key::X)
            {
                std::mem::swap(&mut self.state.fill_color, &mut self.state.stroke_color);
                for id in &self.state.selected_ids {
                    for (_, obj) in self.state.document.all_objects_mut() {
                        if &obj.id == id {
                            let old_fill = obj
                                .fill
                                .as_ref()
                                .map(|f| f.color)
                                .unwrap_or([0.0, 0.0, 0.0, 0.0]);
                            let old_stroke = obj
                                .stroke
                                .as_ref()
                                .map(|s| s.color)
                                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                            obj.fill = Some(crate::core::path::FillStyle::solid(old_stroke));
                            if let Some(ref mut s) = obj.stroke {
                                s.color = old_fill;
                            }
                        }
                    }
                }
            }

            // Escape: exit group isolation first, else cancel pen/deselect
            if i.key_pressed(egui::Key::Escape) {
                if self.state.isolated_group_id.is_some() {
                    self.state.exit_isolation();
                } else {
                    self.canvas.pen_state.cancel();
                    self.state.selected_ids.clear();
                }
            }

            // Enter to finish pen path
            if i.key_pressed(egui::Key::Enter) {
                if let Some(obj) = self.canvas.pen_state.finish_path(
                    self.state.fill_color,
                    self.state.stroke_color,
                    self.state.stroke_width,
                ) {
                    let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                    self.state
                        .undo_manager
                        .execute(cmd, &mut self.state.document);
                }
            }

            // Delete selected
            if i.key_pressed(egui::Key::Delete)
                || (i.key_pressed(egui::Key::Backspace)
                    && !i.modifiers.ctrl
                    && !i.modifiers.mac_cmd)
            {
                let ids: Vec<String> = self.state.selected_ids.clone();
                let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
                for id in &ids {
                    // Clone first (nested children included), then locate for
                    // an undo that restores the true parent/position.
                    if let Some(obj) = self
                        .state
                        .document
                        .find_object(id)
                        .cloned()
                    {
                        let cmd =
                            crate::core::history::RemoveObjectCommand::located(
                                obj,
                                &self.state.document,
                            );
                        cmds.push(Box::new(cmd));
                    }
                }
                if cmds.len() == 1 {
                    self.state
                        .undo_manager
                        .execute(cmds.pop().unwrap(), &mut self.state.document);
                } else if !cmds.is_empty() {
                    let batch = Box::new(crate::core::history::BatchCommand::new(
                        "Delete Objects",
                        cmds,
                    ));
                    self.state
                        .undo_manager
                        .execute(batch, &mut self.state.document);
                }
                self.state.selected_ids.clear();
            }

            // New Document (Cmd+N / Ctrl+N)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && !i.modifiers.shift
                && i.key_pressed(egui::Key::N)
            {
                self.new_doc_modal.is_open = true;
            }

            // Save (Cmd+S / Ctrl+S) — format-aware: never overwrite .amata/.json with SVG
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && !i.modifiers.shift
                && i.key_pressed(egui::Key::S)
            {
                if self.file_watcher.is_none() {
                    // Previously a silent no-op on new documents.
                    self.state.notify_info(
                        "保存先がありません。メニューの保存から選んでください".to_string(),
                    );
                }
                if let Some(ref mut watcher) = self.file_watcher {
                    self.state.sync_doc_extras();
                    match crate::cli::handlers::common::save_any_document(
                        &self.state.document,
                        &watcher.file_path,
                    ) {
                        Ok(_) => {
                            if let Ok(content) =
                                std::fs::read_to_string(&watcher.file_path)
                            {
                                watcher.mark_saved(&content);
                            } else {
                                watcher.update_timestamp();
                            }
                            self.state.undo_manager.mark_saved();
                            self.version_history_panel
                                .refresh_history(&watcher.file_path);
                            crate::io::recent::push_recent(
                                &watcher.file_path,
                                self.state.document.width,
                                self.state.document.height,
                            );
                            crate::io::project::clear_recovery();
                            self.state.notify_success("ファイルを保存しました");
                        }
                        Err(e) => {
                            self.state.notify_error(format!("保存に失敗しました: {e}"));
                        }
                    }
                }
            }

            // Undo
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && i.key_pressed(egui::Key::Z)
                && !i.modifiers.shift
            {
                self.state.undo_manager.undo(&mut self.state.document);
            }

            // Redo
            if ((i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Y))
                || ((i.modifiers.ctrl || i.modifiers.mac_cmd)
                    && i.modifiers.shift
                    && i.key_pressed(egui::Key::Z))
            {
                self.state.undo_manager.redo(&mut self.state.document);
            }

            // Select All (Ctrl+A / Cmd+A) & Deselect (Ctrl+Shift+A / Cmd+Shift+A)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::A) {
                if i.modifiers.shift {
                    self.state.selected_ids.clear();
                } else {
                    self.state.selected_ids = self
                        .state
                        .document
                        .all_objects()
                        .filter(|(_, o)| o.visible && !o.locked)
                        .map(|(_, o)| o.id.clone())
                        .collect();
                }
            }

            // Arrange z-order (Cmd/Ctrl+] [ +Shift for front/back).
            // These were advertised on the arrange buttons but never bound.
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && (i.key_pressed(egui::Key::CloseBracket)
                    || i.key_pressed(egui::Key::OpenBracket))
            {
                let forward = i.key_pressed(egui::Key::CloseBracket);
                let jump = i.modifiers.shift;
                let sel = self.state.selected_ids.clone();
                if forward && jump {
                    self.state.reorder_objects_undoable("Bring to Front", |doc| {
                        arrange_move(&sel, doc, true, true);
                    });
                } else if forward {
                    self.state.reorder_objects_undoable("Bring Forward", |doc| {
                        arrange_move(&sel, doc, true, false);
                    });
                } else if jump {
                    self.state.reorder_objects_undoable("Send to Back", |doc| {
                        arrange_move(&sel, doc, false, true);
                    });
                } else {
                    self.state.reorder_objects_undoable("Send Backward", |doc| {
                        arrange_move(&sel, doc, false, false);
                    });
                }
            }

            // Group (Ctrl+G) as one replace step.
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && !i.modifiers.shift
                && i.key_pressed(egui::Key::G)
                && self.state.selected_ids.len() >= 2
            {
                self.state.replace_selected("Group", |objects| {
                    if objects.len() >= 2 {
                        let group = Object::new_group("Group", objects);
                        let gid = group.id.clone();
                        Some((vec![group], vec![gid]))
                    } else {
                        None
                    }
                });
            }

            // Ungroup (Ctrl+Shift+G) as one replace step.
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && i.modifiers.shift
                && i.key_pressed(egui::Key::G)
            {
                self.state.replace_selected("Ungroup", |objects| {
                    let mut added = Vec::new();
                    let mut new_ids = Vec::new();
                    for obj in objects {
                        if let ObjectType::Group(children) = obj.object_type {
                            for child in children {
                                new_ids.push(child.id.clone());
                                added.push(child);
                            }
                        } else {
                            new_ids.push(obj.id.clone());
                            added.push(obj);
                        }
                    }
                    Some((added, new_ids))
                });
            }

            // Create Outlines (Ctrl+Shift+O / Cmd+Shift+O) as one step.
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && i.modifiers.shift
                && i.key_pressed(egui::Key::O)
            {
                self.state.replace_selected("Create Outlines", |objects| {
                    let mut added = Vec::new();
                    let mut new_ids = Vec::new();
                    for obj in objects {
                        if let Some(outlined) =
                            crate::core::text_path::create_text_outlines(&obj)
                        {
                            new_ids.push(outlined.id.clone());
                            added.push(outlined);
                        } else {
                            new_ids.push(obj.id.clone());
                            added.push(obj);
                        }
                    }
                    Some((added, new_ids))
                });
            }

            // Zoom to fit all (Ctrl+0)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Num0) {
                self.state.start_zoom = self.state.zoom;
                self.state.start_pan_x = self.state.pan_x;
                self.state.start_pan_y = self.state.pan_y;
                zoom_to_fit(&mut self.state);
                self.state.zoom_animation_progress = 0.0;
            }

            // Zoom to 100% (Ctrl+1)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Num1) {
                self.state.start_zoom = self.state.zoom;
                self.state.start_pan_x = self.state.pan_x;
                self.state.start_pan_y = self.state.pan_y;
                self.state.target_zoom = 1.0;
                self.state.target_pan_x = 0.0;
                self.state.target_pan_y = 0.0;
                self.state.zoom_animation_progress = 0.0;
            }

            // Clipping Mask (Ctrl+7) as one replace step. The mask is the
            // first SELECTED object, so restore selection order here
            // (snapshots arrive in layer order).
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Num7) {
                let sel = self.state.selected_ids.clone();
                self.state.replace_selected("Clipping Mask", |objects| {
                    let ordered: Vec<Object> = sel
                        .iter()
                        .filter_map(|id| objects.iter().find(|o| &o.id == id).cloned())
                        .collect();
                    if ordered.len() < 2 {
                        return None;
                    }
                    let mut parts = ordered.into_iter();
                    let mask = parts.next().unwrap();
                    let mask_path = mask.to_path_data();
                    let mut children = vec![Object::new_path("Mask", mask_path)];
                    children.extend(parts);
                    let clipping = Object {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: "Clipping Mask".into(),
                        object_type: ObjectType::ClippingMask { children },
                        ..Object::new_rect("Clipping Mask", 0.0, 0.0, 100.0, 100.0, 0.0)
                    };
                    let nid = clipping.id.clone();
                    Some((vec![clipping], vec![nid]))
                });
            }

            // Make / Release Compound Path (Ctrl+8 / Ctrl+Alt+Shift+8).
            // Conversion runs BEFORE any mutation: a failed compound
            // leaves the document untouched.
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Num8) {
                if i.modifiers.shift && i.modifiers.alt {
                    self.state.replace_selected_where(
                        "Release Compound",
                        |o| o.release_compound_path().len() > 1,
                        |objects| {
                            let mut added = Vec::new();
                            let mut new_ids = Vec::new();
                            for obj in objects {
                                for r in obj.release_compound_path() {
                                    new_ids.push(r.id.clone());
                                    added.push(r);
                                }
                            }
                            Some((added, new_ids))
                        },
                    );
                } else {
                    self.state.replace_selected("Make Compound Path", |objects| {
                        if objects.len() < 2 {
                            return None;
                        }
                        Object::make_compound_path(&objects).map(|compound| {
                            let nid = compound.id.clone();
                            (vec![compound], vec![nid])
                        })
                    });
                }
            }

            // Arrow key nudging: Arrow=1px, Shift+Arrow=10px, Ctrl+Arrow=10px, Ctrl+Shift+Arrow=100px
            let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
            let shift = i.modifiers.shift;
            let nudge = if ctrl && shift {
                100.0
            } else if ctrl || shift {
                10.0
            } else {
                1.0
            };
            let mut nudge_x = 0.0;
            let mut nudge_y = 0.0;
            if i.key_pressed(egui::Key::ArrowLeft) {
                nudge_x -= nudge;
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                nudge_x += nudge;
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                nudge_y -= nudge;
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                nudge_y += nudge;
            }

            // Arrow-key nudge: one undo step per keypress (previously
            // invisible to undo and dirty tracking entirely).
            if nudge_x != 0.0 || nudge_y != 0.0 {
                let ids = self.state.selected_ids.clone();
                let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
                for id in &ids {
                    if let Some(obj) = self.state.document.find_object(id) {
                        cmds.push(Box::new(crate::core::history::MoveObjectCommand {
                            object_id: id.clone(),
                            old_x: obj.transform.x,
                            old_y: obj.transform.y,
                            new_x: obj.transform.x + nudge_x,
                            new_y: obj.transform.y + nudge_y,
                        })
                            as Box<dyn crate::core::history::Command>);
                    }
                }
                if cmds.len() == 1 {
                    let cmd = cmds.pop().unwrap();
                    self.state.undo_manager.execute(cmd, &mut self.state.document);
                } else if !cmds.is_empty() {
                    let batch = Box::new(crate::core::history::BatchCommand::new(
                        "Nudge",
                        cmds,
                    ));
                    self.state.undo_manager.execute(batch, &mut self.state.document);
                }
            }

            // Swap Fill and Stroke (Shift+X)
            if i.modifiers.shift && i.key_pressed(egui::Key::X) {
                std::mem::swap(&mut self.state.fill_color, &mut self.state.stroke_color);
            }

            // Default Fill and Stroke (D)
            if !i.modifiers.ctrl
                && !i.modifiers.mac_cmd
                && !i.modifiers.alt
                && i.key_pressed(egui::Key::D)
            {
                self.state.fill_color = [1.0, 1.0, 1.0, 1.0];
                self.state.stroke_color = [0.0, 0.0, 0.0, 1.0];
                self.state.stroke_width = 1.0;
            }

            // Set Fill to None (Slash /)
            if !i.modifiers.ctrl
                && !i.modifiers.mac_cmd
                && !i.modifiers.alt
                && i.key_pressed(egui::Key::Slash)
            {
                self.state.fill_color = [0.0, 0.0, 0.0, 0.0];
            }

            // Zoom In (Ctrl + Plus / Equal)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && (i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus))
            {
                self.state.start_zoom = self.state.zoom;
                self.state.start_pan_x = self.state.pan_x;
                self.state.start_pan_y = self.state.pan_y;
                self.state.target_zoom = (self.state.target_zoom * 1.25).clamp(0.01, 100.0);
                self.state.zoom_animation_progress = 0.0;
            }

            // Zoom Out (Ctrl + Minus)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::Minus) {
                self.state.start_zoom = self.state.zoom;
                self.state.start_pan_x = self.state.pan_x;
                self.state.start_pan_y = self.state.pan_y;
                self.state.target_zoom = (self.state.target_zoom / 1.25).clamp(0.01, 100.0);
                self.state.zoom_animation_progress = 0.0;
            }

            // Object ordering: detect Ctrl+]/Ctrl+[ via text events
            let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
            let shift = i.modifiers.shift;
            for event in &i.events {
                if let egui::Event::Text(text) = event {
                    if ctrl && text == "]" {
                        let sel = self.state.selected_ids.clone();
                        if shift {
                            // Bring to Front (Ctrl+Shift+])
                            for id in &sel {
                                for layer in self.state.document.layers.iter_mut() {
                                    if let Some(pos) =
                                        layer.objects.iter().position(|o| &o.id == id)
                                    {
                                        let obj = layer.objects.remove(pos);
                                        layer.objects.push(obj);
                                        break;
                                    }
                                }
                            }
                        } else {
                            // Bring Forward (Ctrl+])
                            for id in &sel {
                                for layer in self.state.document.layers.iter_mut() {
                                    if let Some(pos) =
                                        layer.objects.iter().position(|o| &o.id == id)
                                    {
                                        if pos + 1 < layer.objects.len() {
                                            layer.objects.swap(pos, pos + 1);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    } else if ctrl && text == "[" {
                        let sel = self.state.selected_ids.clone();
                        if shift {
                            // Send to Back (Ctrl+Shift+[)
                            for id in &sel {
                                for layer in self.state.document.layers.iter_mut() {
                                    if let Some(pos) =
                                        layer.objects.iter().position(|o| &o.id == id)
                                    {
                                        let obj = layer.objects.remove(pos);
                                        layer.objects.insert(0, obj);
                                        break;
                                    }
                                }
                            }
                        } else {
                            // Send Backward (Ctrl+[)
                            for id in &sel {
                                for layer in self.state.document.layers.iter_mut() {
                                    if let Some(pos) =
                                        layer.objects.iter().position(|o| &o.id == id)
                                    {
                                        if pos > 0 {
                                            layer.objects.swap(pos, pos - 1);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Cut (Ctrl+X / Cmd+X): one undo step restoring all parts.
            if (i.modifiers.ctrl || i.modifiers.mac_cmd)
                && !i.modifiers.shift
                && i.key_pressed(egui::Key::X)
            {
                self.state.clipboard.clear();
                let ids = self.state.selected_ids.clone();
                let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
                for id in &ids {
                    if let Some(obj) = self.state.document.find_object(id).cloned() {
                        self.state.clipboard.push(obj.clone());
                        cmds.push(Box::new(
                            crate::core::history::RemoveObjectCommand::located(
                                obj,
                                &self.state.document,
                            ),
                        )
                            as Box<dyn crate::core::history::Command>);
                    }
                }
                if cmds.len() == 1 {
                    let cmd = cmds.pop().unwrap();
                    self.state.undo_manager.execute(cmd, &mut self.state.document);
                } else if !cmds.is_empty() {
                    let batch = Box::new(crate::core::history::BatchCommand::new(
                        "Cut Objects",
                        cmds,
                    ));
                    self.state.undo_manager.execute(batch, &mut self.state.document);
                }
                self.state.selected_ids.clear();
            }

            // Copy (Ctrl+C)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::C) {
                self.state.clipboard.clear();
                for id in &self.state.selected_ids {
                    if let Some((_, obj)) =
                        self.state.document.all_objects().find(|(_, o)| &o.id == id)
                    {
                        self.state.clipboard.push(obj.clone());
                    }
                }
            }

            // Paste (Ctrl+V) & Paste in Place (Ctrl+Shift+V / Cmd+Shift+V):
            // one undo step no matter how many parts are pasted.
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::V) {
                let in_place = i.modifiers.shift;
                self.state.selected_ids.clear();
                let mut offset = 0.0;
                let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
                let mut new_ids = Vec::new();
                for obj in &self.state.clipboard.clone() {
                    let mut new_obj = obj.clone();
                    new_obj.id = uuid::Uuid::new_v4().to_string();
                    new_obj.name = format!("{} (copy)", obj.name);
                    if !in_place {
                        new_obj.transform.x += 20.0 + offset;
                        new_obj.transform.y += 20.0 + offset;
                        offset += 15.0;
                    }
                    new_ids.push(new_obj.id.clone());
                    cmds.push(Box::new(
                        crate::core::history::AddObjectCommand::new(new_obj),
                    )
                        as Box<dyn crate::core::history::Command>);
                }
                if cmds.len() == 1 {
                    let cmd = cmds.pop().unwrap();
                    self.state.undo_manager.execute(cmd, &mut self.state.document);
                } else if !cmds.is_empty() {
                    let batch = Box::new(crate::core::history::BatchCommand::new(
                        "Paste Objects",
                        cmds,
                    ));
                    self.state.undo_manager.execute(batch, &mut self.state.document);
                }
                self.state.selected_ids = new_ids;
            }

            // Duplicate (Ctrl+D)
            if (i.modifiers.ctrl || i.modifiers.mac_cmd) && i.key_pressed(egui::Key::D) {
                let ids: Vec<String> = self.state.selected_ids.clone();
                let mut new_objs = Vec::new();
                for id in &ids {
                    if let Some((_, obj)) =
                        self.state.document.all_objects().find(|(_, o)| &o.id == id)
                    {
                        let mut new_obj = obj.clone();
                        new_obj.id = uuid::Uuid::new_v4().to_string();
                        new_obj.name = format!("{} (copy)", obj.name);
                        new_obj.transform.x += 20.0;
                        new_obj.transform.y += 20.0;
                        new_objs.push(new_obj);
                    }
                }
                let mut new_ids = Vec::new();
                let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
                for obj in new_objs {
                    new_ids.push(obj.id.clone());
                    cmds.push(Box::new(crate::core::history::AddObjectCommand::new(obj))
                        as Box<dyn crate::core::history::Command>);
                }
                if cmds.len() == 1 {
                    let cmd = cmds.pop().unwrap();
                    self.state.undo_manager.execute(cmd, &mut self.state.document);
                } else if !cmds.is_empty() {
                    let batch = Box::new(crate::core::history::BatchCommand::new(
                        "Duplicate Objects",
                        cmds,
                    ));
                    self.state.undo_manager.execute(batch, &mut self.state.document);
                }
                self.state.selected_ids = new_ids;
            }
        });
    }
}
