use super::CanvasWidget;
use crate::core::document::ObjectType;
use crate::core::state::AppState;
use egui::{Pos2, Response, Ui};

impl CanvasWidget {
    pub(super) fn show_context_menu(
        &mut self,
        _ui: &mut Ui,
        state: &mut AppState,
        response: &Response,
        _origin: Pos2,
    ) {
        // Right-Click Context Menu (Illustrator style)
        response.context_menu(|ui| {
            let has_sel = !state.selected_ids.is_empty();
            let multi_sel = state.selected_ids.len() >= 2;

            if has_sel {
                ui.label(egui::RichText::new("選択項目").weak().size(10.0));
                ui.separator();

                if ui.button("カット (切り取り)   Cmd+X").clicked() {
                    state.clipboard.clear();
                    let ids = state.selected_ids.clone();
                    for id in &ids {
                        if let Some(obj) = state.document.remove_object(id) {
                            state.clipboard.push(obj);
                        }
                    }
                    state.selected_ids.clear();
                    ui.close_menu();
                }

                if ui.button("コピー   Cmd+C").clicked() {
                    state.clipboard.clear();
                    for id in &state.selected_ids {
                        if let Some((_, obj)) =
                            state.document.all_objects().find(|(_, o)| &o.id == id)
                        {
                            state.clipboard.push(obj.clone());
                        }
                    }
                    ui.close_menu();
                }

                if ui.button("ペースト (貼り付け)   Cmd+V").clicked() {
                    state.selected_ids.clear();
                    let mut offset = 0.0;
                    for obj in &state.clipboard {
                        let mut new_obj = obj.clone();
                        new_obj.id = uuid::Uuid::new_v4().to_string();
                        new_obj.name = format!("{} のコピー", obj.name);
                        new_obj.transform.x += 20.0 + offset;
                        new_obj.transform.y += 20.0 + offset;
                        offset += 15.0;
                        let id = new_obj.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids.push(id);
                    }
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("複製   Cmd+D").clicked() {
                    let ids = state.selected_ids.clone();
                    let mut new_objs = Vec::new();
                    for id in &ids {
                        if let Some((_, obj)) =
                            state.document.all_objects().find(|(_, o)| &o.id == id)
                        {
                            let mut c = obj.clone();
                            c.id = uuid::Uuid::new_v4().to_string();
                            c.transform.x += 20.0;
                            c.transform.y += 20.0;
                            new_objs.push(c);
                        }
                    }
                    let mut new_ids = Vec::new();
                    for obj in new_objs {
                        new_ids.push(obj.id.clone());
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                    state.selected_ids = new_ids;
                    ui.close_menu();
                }

                if ui.button("削除   Del").clicked() {
                    let ids: Vec<String> = state.selected_ids.clone();
                    let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
                    for id in &ids {
                        let mut found = None;
                        for (l_idx, layer) in state.document.layers.iter().enumerate() {
                            if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                found = Some((layer.objects[pos].clone(), l_idx, pos));
                                break;
                            }
                        }
                        if let Some((obj, layer_idx, pos)) = found {
                            cmds.push(Box::new(crate::core::history::RemoveObjectCommand::new(
                                obj, layer_idx, pos,
                            )));
                        }
                    }
                    if cmds.len() == 1 {
                        state
                            .undo_manager
                            .execute(cmds.pop().unwrap(), &mut state.document);
                    } else if !cmds.is_empty() {
                        let batch = Box::new(crate::core::history::BatchCommand::new(
                            "Delete Objects",
                            cmds,
                        ));
                        state.undo_manager.execute(batch, &mut state.document);
                    }
                    state.selected_ids.clear();
                    ui.close_menu();
                }

                ui.separator();
                ui.menu_button("重ね順 (Arrange)", |ui| {
                    if ui.button("最前面へ   Cmd+Shift+]").clicked() {
                        let sel = state.selected_ids.clone();
                        for id in &sel {
                            for layer in state.document.layers.iter_mut() {
                                if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                    let obj = layer.objects.remove(pos);
                                    layer.objects.push(obj);
                                    break;
                                }
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("前面へ   Cmd+]").clicked() {
                        let sel = state.selected_ids.clone();
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
                        ui.close_menu();
                    }
                    if ui.button("背面へ   Cmd+[").clicked() {
                        let sel = state.selected_ids.clone();
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
                        ui.close_menu();
                    }
                    if ui.button("最背面へ   Cmd+Shift+[").clicked() {
                        let sel = state.selected_ids.clone();
                        for id in &sel {
                            for layer in state.document.layers.iter_mut() {
                                if let Some(pos) = layer.objects.iter().position(|o| &o.id == id) {
                                    let obj = layer.objects.remove(pos);
                                    layer.objects.insert(0, obj);
                                    break;
                                }
                            }
                        }
                        ui.close_menu();
                    }
                });

                if multi_sel {
                    ui.separator();
                    if ui.button("グループ化   Cmd+G").clicked() {
                        let sel = state.selected_ids.clone();
                        let mut children = Vec::new();
                        for id in &sel {
                            if let Some(obj) = state.document.remove_object(id) {
                                children.push(obj);
                            }
                        }
                        let grp = crate::core::document::Object::new_group("グループ", children);
                        let new_id = grp.id.clone();
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(grp));
                        state.undo_manager.execute(cmd, &mut state.document);
                        state.selected_ids = vec![new_id];
                        ui.close_menu();
                    }
                }

                if has_sel {
                    let has_group = state.selected_ids.iter().any(|id| {
                        state.document.all_objects().any(|(_, o)| {
                            o.id == *id && matches!(o.object_type, ObjectType::Group(_))
                        })
                    });
                    if has_group && ui.button("グループ解除   Cmd+Shift+G").clicked() {
                        let ids = state.selected_ids.clone();
                        let mut new_ids = Vec::new();
                        for id in &ids {
                            if let Some(obj) = state.document.remove_object(id) {
                                if let ObjectType::Group(children) = obj.object_type {
                                    for child in children {
                                        new_ids.push(child.id.clone());
                                        let cmd = Box::new(
                                            crate::core::history::AddObjectCommand::new(child),
                                        );
                                        state.undo_manager.execute(cmd, &mut state.document);
                                    }
                                } else {
                                    new_ids.push(obj.id.clone());
                                    let cmd =
                                        Box::new(crate::core::history::AddObjectCommand::new(obj));
                                    state.undo_manager.execute(cmd, &mut state.document);
                                }
                            }
                        }
                        state.selected_ids = new_ids;
                        ui.close_menu();
                    }
                }
            } else {
                ui.label(egui::RichText::new("キャンバス").weak().size(10.0));
                ui.separator();
                if ui.button("画面に合わせる (フィット)   Cmd+0").clicked() {
                    state.start_zoom = state.zoom;
                    state.start_pan_x = state.pan_x;
                    state.start_pan_y = state.pan_y;
                    state.zoom_to_fit();
                    state.zoom_animation_progress = 0.0;
                    ui.close_menu();
                }
                if ui.button("等倍表示 (100%)   Cmd+1").clicked() {
                    state.start_zoom = state.zoom;
                    state.start_pan_x = state.pan_x;
                    state.start_pan_y = state.pan_y;
                    state.target_zoom = 1.0;
                    state.target_pan_x = 0.0;
                    state.target_pan_y = 0.0;
                    state.zoom_animation_progress = 0.0;
                    ui.close_menu();
                }
                ui.separator();
                ui.checkbox(&mut state.show_grid, "グリッドを表示");
                ui.checkbox(&mut state.show_rulers, "定規を表示");
                ui.checkbox(&mut state.show_smart_guides, "スマートガイド");
                ui.checkbox(&mut state.snap_to_objects, "オブジェクトにスナップ");
            }
        });
    }
}
