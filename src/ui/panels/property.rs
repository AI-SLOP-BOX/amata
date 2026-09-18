use crate::core::document::ObjectType;
use crate::core::path::FillStyle;
use crate::core::state::{AppState, Tool};
use egui::{Color32, Pos2, RichText, Stroke, StrokeKind, Ui, Vec2};

pub struct PropertyPanel;

impl PropertyPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.label(
            RichText::new("プロパティ")
                .strong()
                .size(13.0)
                .color(Color32::WHITE),
        );
        ui.add_space(4.0);

        // If objects are selected, show Illustrator CC Object Properties (Image 6)
        // Otherwise, show Document and Canvas setup (Image 3)
        if let Some(id) = state.selected_ids.first().cloned() {
            let mut obj_name = String::new();
            let mut tx = 0.0;
            let mut ty = 0.0;
            let mut bw = 0.0;
            let mut bh = 0.0;
            let mut rot = 0.0;
            let mut fill_opt = None;
            let mut stroke_opt = None;
            let mut opac = 1.0;
            let mut is_path = false;

            for (_, obj) in state.document.all_objects() {
                if obj.id == id {
                    obj_name = obj.name.clone();
                    tx = obj.transform.x;
                    ty = obj.transform.y;
                    if let Some((min, max)) = obj.bounding_box() {
                        bw = max.x - min.x;
                        bh = max.y - min.y;
                    }
                    rot = obj.transform.rotation.to_degrees();
                    fill_opt = obj.fill.clone();
                    stroke_opt = obj.stroke.clone();
                    opac = obj.opacity;
                    is_path = matches!(obj.object_type, ObjectType::Path(_));
                    break;
                }
            }

            // Header editable name (e.g. "パス", "長方形", "グループ" or custom name)
            ui.horizontal(|ui| {
                let default_hint = if is_path {
                    "パス"
                } else {
                    "オブジェクト"
                };
                let mut edit_name = obj_name.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut edit_name)
                            .hint_text(default_hint)
                            .desired_width(180.0),
                    )
                    .changed()
                {
                    for (_, o) in state.document.all_objects_mut() {
                        if o.id == id {
                            o.name = edit_name.clone();
                            break;
                        }
                    }
                }
            });
            ui.add_space(4.0);

            // ─── 変形 (Transform) — Image 6 Reference Layout ───
            ui.label(
                RichText::new("変形")
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                // 9-point reference anchor proxy widget (Image 6 left of X/Y)
                let (proxy_rect, _) =
                    ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                ui.painter().rect_stroke(
                    proxy_rect,
                    1.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(80, 80, 80)),
                    StrokeKind::Inside,
                );
                for row in 0..3 {
                    for col in 0..3 {
                        let dot_pos = Pos2::new(
                            proxy_rect.min.x + 5.0 + col as f32 * 9.0,
                            proxy_rect.min.y + 5.0 + row as f32 * 9.0,
                        );
                        let is_center = row == 1 && col == 1;
                        let dot_color = if is_center {
                            Color32::from_rgb(20, 115, 230)
                        } else {
                            Color32::from_rgb(120, 120, 120)
                        };
                        ui.painter().circle_filled(dot_pos, 2.0, dot_color);
                    }
                }

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("X:")
                                .size(10.5)
                                .color(Color32::from_rgb(150, 150, 150)),
                        );
                        let mut x_val = tx;
                        let x_resp = ui.add(
                            egui::DragValue::new(&mut x_val).speed(0.5).suffix(" mm"),
                        );
                        if x_resp.changed() {
                            // Move every selected object by the same delta so
                            // multi-selection keeps its relative layout.
                            // (Previously all objects were stacked onto the
                            // first object's absolute value.)
                            let dx = x_val - tx;
                            let sel = state.selected_ids.clone();
                            for id in &sel {
                                state.ensure_transform_snapshot(id);
                                if let Some(o) = state.document.find_object_mut(id) {
                                    o.transform.x += dx;
                                }
                            }
                            if !x_resp.dragged() {
                                state.commit_transform_edits("Edit Transform");
                            }
                        }
                        if x_resp.drag_stopped() {
                            state.commit_transform_edits("Edit Transform");
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("W:")
                                .size(10.5)
                                .color(Color32::from_rgb(150, 150, 150)),
                        );
                        let mut w_val = bw;
                        let w_resp = ui.add(
                            egui::DragValue::new(&mut w_val).speed(0.5).suffix(" mm"),
                        );
                        if w_resp.changed() && bw > 0.0 {
                            let scale = w_val / bw;
                            let sel = state.selected_ids.clone();
                            for id in &sel {
                                state.ensure_transform_snapshot(id);
                                if let Some(o) = state.document.find_object_mut(id) {
                                    o.transform.scale_x *= scale;
                                }
                            }
                            if !w_resp.dragged() {
                                state.commit_transform_edits("Edit Transform");
                            }
                        }
                        if w_resp.drag_stopped() {
                            state.commit_transform_edits("Edit Transform");
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Y:")
                                .size(10.5)
                                .color(Color32::from_rgb(150, 150, 150)),
                        );
                        let mut y_val = ty;
                        let y_resp = ui.add(
                            egui::DragValue::new(&mut y_val).speed(0.5).suffix(" mm"),
                        );
                        if y_resp.changed() {
                            let dy = y_val - ty;
                            let sel = state.selected_ids.clone();
                            for id in &sel {
                                state.ensure_transform_snapshot(id);
                                if let Some(o) = state.document.find_object_mut(id) {
                                    o.transform.y += dy;
                                }
                            }
                            if !y_resp.dragged() {
                                state.commit_transform_edits("Edit Transform");
                            }
                        }
                        if y_resp.drag_stopped() {
                            state.commit_transform_edits("Edit Transform");
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("H:")
                                .size(10.5)
                                .color(Color32::from_rgb(150, 150, 150)),
                        );
                        let mut h_val = bh;
                        let h_resp = ui.add(
                            egui::DragValue::new(&mut h_val).speed(0.5).suffix(" mm"),
                        );
                        if h_resp.changed() && bh > 0.0 {
                            let scale = h_val / bh;
                            let sel = state.selected_ids.clone();
                            for id in &sel {
                                state.ensure_transform_snapshot(id);
                                if let Some(o) = state.document.find_object_mut(id) {
                                    o.transform.scale_y *= scale;
                                }
                            }
                            if !h_resp.dragged() {
                                state.commit_transform_edits("Edit Transform");
                            }
                        }
                        if h_resp.drag_stopped() {
                            state.commit_transform_edits("Edit Transform");
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("∠:")
                                .size(10.5)
                                .color(Color32::from_rgb(150, 150, 150)),
                        );
                        let mut r_val = rot;
                        let r_resp = ui.add(
                            egui::DragValue::new(&mut r_val).speed(1.0).suffix("°"),
                        );
                        if r_resp.changed() {
                            let sel = state.selected_ids.clone();
                            for id in &sel {
                                state.ensure_transform_snapshot(id);
                                if let Some(o) = state.document.find_object_mut(id) {
                                    o.transform.rotation = r_val.to_radians();
                                }
                            }
                            if !r_resp.dragged() {
                                state.commit_transform_edits("Edit Transform");
                            }
                        }
                        if r_resp.drag_stopped() {
                            state.commit_transform_edits("Edit Transform");
                        }
                    });
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // ─── アピアランス (Appearance) — Image 6 Reference Layout ───
            ui.label(
                RichText::new("アピアランス")
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(4.0);

            // Fill swatch line
            ui.horizontal(|ui| {
                let fill_c = fill_opt
                    .as_ref()
                    .map(|f| f.color)
                    .unwrap_or(state.fill_color);
                let mut c_rgba = [
                    (fill_c[0] * 255.0) as u8,
                    (fill_c[1] * 255.0) as u8,
                    (fill_c[2] * 255.0) as u8,
                    (fill_c[3] * 255.0) as u8,
                ];
                if ui
                    .color_edit_button_srgba_unmultiplied(&mut c_rgba)
                    .changed()
                {
                    let new_fill = [
                        c_rgba[0] as f32 / 255.0,
                        c_rgba[1] as f32 / 255.0,
                        c_rgba[2] as f32 / 255.0,
                        c_rgba[3] as f32 / 255.0,
                    ];
                    state.fill_color = new_fill;
                    let sel = state.selected_ids.clone();
                    for id in &sel {
                        for (_, o) in state.document.all_objects_mut() {
                            if &o.id == id {
                                o.fill = Some(FillStyle::solid(new_fill));
                            }
                        }
                    }
                }
                ui.label(RichText::new("塗り").size(11.0).color(Color32::WHITE));
            });

            // Stroke swatch line
            ui.horizontal(|ui| {
                let stroke_c = stroke_opt
                    .as_ref()
                    .map(|s| s.color)
                    .unwrap_or(state.stroke_color);
                let mut sc_rgba = [
                    (stroke_c[0] * 255.0) as u8,
                    (stroke_c[1] * 255.0) as u8,
                    (stroke_c[2] * 255.0) as u8,
                    (stroke_c[3] * 255.0) as u8,
                ];
                if ui
                    .color_edit_button_srgba_unmultiplied(&mut sc_rgba)
                    .changed()
                {
                    let new_sc = [
                        sc_rgba[0] as f32 / 255.0,
                        sc_rgba[1] as f32 / 255.0,
                        sc_rgba[2] as f32 / 255.0,
                        sc_rgba[3] as f32 / 255.0,
                    ];
                    state.stroke_color = new_sc;
                    let sel = state.selected_ids.clone();
                    for id in &sel {
                        for (_, o) in state.document.all_objects_mut() {
                            if &o.id == id {
                                if let Some(ref mut s) = o.stroke {
                                    s.color = new_sc;
                                } else {
                                    o.stroke = Some(crate::core::path::StrokeStyle {
                                        color: new_sc,
                                        width: state.stroke_width.max(1.0),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
                ui.label(RichText::new("線").size(11.0).color(Color32::WHITE));

                let mut sw = stroke_opt
                    .as_ref()
                    .map(|s| s.width)
                    .unwrap_or(state.stroke_width);
                if ui
                    .add(
                        egui::DragValue::new(&mut sw)
                            .speed(0.2)
                            .range(0.0..=100.0)
                            .suffix(" pt"),
                    )
                    .changed()
                {
                    state.stroke_width = sw;
                    let sel = state.selected_ids.clone();
                    for id in &sel {
                        for (_, o) in state.document.all_objects_mut() {
                            if &o.id == id {
                                if let Some(ref mut s) = o.stroke {
                                    s.width = sw;
                                } else if sw > 0.0 {
                                    o.stroke = Some(crate::core::path::StrokeStyle {
                                        color: state.stroke_color,
                                        width: sw,
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
            });

            // Opacity line
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("不透明度")
                        .size(11.0)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                let mut op = opac;
                if ui
                    .add(
                        egui::Slider::new(&mut op, 0.0..=1.0)
                            .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                    )
                    .changed()
                {
                    state.opacity = op;
                    let sel = state.selected_ids.clone();
                    for id in &sel {
                        for (_, o) in state.document.all_objects_mut() {
                            if &o.id == id {
                                o.opacity = op;
                            }
                        }
                    }
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // ─── クイックアクション (Quick Actions) — Image 6 Reference Layout ───
            ui.label(
                RichText::new("クイックアクション")
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("パスのオフセット").size(10.5))
                            .min_size(Vec2::new(105.0, 24.0)),
                    )
                    .clicked()
                {
                    let sel = state.selected_ids.clone();
                    for id in &sel {
                        for (_, o) in state.document.all_objects_mut() {
                            if &o.id == id {
                                let path = o.to_path_data();
                                let off = crate::core::offset::offset_path(&path, 5.0);
                                o.object_type = ObjectType::Path(off);
                            }
                        }
                    }
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new("パスを拡張").size(10.5))
                            .min_size(Vec2::new(105.0, 24.0)),
                    )
                    .clicked()
                {
                    let sel = state.selected_ids.clone();
                    for id in &sel {
                        for (_, o) in state.document.all_objects_mut() {
                            if &o.id == id {
                                let path = o.to_path_data();
                                let stroke = o.stroke.clone().unwrap_or_default();
                                let outlined =
                                    crate::core::offset::outline_stroke(&path, stroke.width);
                                o.object_type = ObjectType::Path(outlined);
                            }
                        }
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("シェイプに変換").size(10.5))
                            .min_size(Vec2::new(105.0, 24.0)),
                    )
                    .clicked()
                {
                    state.current_tool = Tool::ShapeBuilder;
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new("ピクセルグリッドに整合").size(10.5))
                            .min_size(Vec2::new(105.0, 24.0)),
                    )
                    .clicked()
                {
                    let sel = state.selected_ids.clone();
                    for id in &sel {
                        for (_, o) in state.document.all_objects_mut() {
                            if &o.id == id {
                                o.transform.x = o.transform.x.round();
                                o.transform.y = o.transform.y.round();
                            }
                        }
                    }
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // ─── 整列 (Align) — Image 6 Reference Layout ───
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("整列")
                        .strong()
                        .size(12.0)
                        .color(Color32::WHITE),
                );
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let sel = state.selected_ids.clone();
                if ui.small_button("⇠").on_hover_text("左揃え").clicked() {
                    crate::ui::AlignPanel::align_left(state, &sel);
                }
                if ui
                    .small_button("↔")
                    .on_hover_text("水平方向中央揃え")
                    .clicked()
                {
                    crate::ui::AlignPanel::align_center_h(state, &sel);
                }
                if ui.small_button("⇥").on_hover_text("右揃え").clicked() {
                    crate::ui::AlignPanel::align_right(state, &sel);
                }
                ui.add_space(4.0);
                if ui.small_button("⇡").on_hover_text("上揃え").clicked() {
                    crate::ui::AlignPanel::align_top(state, &sel);
                }
                if ui
                    .small_button("↕")
                    .on_hover_text("垂直方向中央揃え")
                    .clicked()
                {
                    crate::ui::AlignPanel::align_center_v(state, &sel);
                }
                if ui.small_button("⇣").on_hover_text("下揃え").clicked() {
                    crate::ui::AlignPanel::align_bottom(state, &sel);
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // ─── パスファインダー (Pathfinder) — Image 6 Reference Layout ───
            ui.label(
                RichText::new("パスファインダー")
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("合体").size(10.5)).clicked() {
                    crate::ui::PathfinderPanel::apply_op(
                        state,
                        crate::core::boolean::BooleanOp::Union,
                    );
                }
                if ui
                    .button(RichText::new("前面オブジェクトで型抜き").size(10.5))
                    .clicked()
                {
                    crate::ui::PathfinderPanel::apply_op(
                        state,
                        crate::core::boolean::BooleanOp::Subtract,
                    );
                }
                if ui.button(RichText::new("交差").size(10.5)).clicked() {
                    crate::ui::PathfinderPanel::apply_op(
                        state,
                        crate::core::boolean::BooleanOp::Intersect,
                    );
                }
                if ui.button(RichText::new("中マド").size(10.5)).clicked() {
                    crate::ui::PathfinderPanel::apply_op(
                        state,
                        crate::core::boolean::BooleanOp::Exclude,
                    );
                }
            });
        } else {
            // Adobe CC Property Panel: Complete Document & Canvas Settings (Image 2 & 3)
            ui.label(
                RichText::new("選択なし")
                    .size(11.0)
                    .color(Color32::from_rgb(140, 140, 140)),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new("ドキュメント")
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("単位:")
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                egui::ComboBox::from_id_salt("prop_doc_unit")
                    .selected_text("ミリメートル")
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_label(true, "ミリメートル");
                        let _ = ui.selectable_label(false, "ピクセル");
                        let _ = ui.selectable_label(false, "ポイント");
                        let _ = ui.selectable_label(false, "インチ");
                    });
            });

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("アートボード:")
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                ui.label(RichText::new("1").size(11.0).color(Color32::WHITE));
                let _ = ui.small_button("◀");
                let _ = ui.small_button("▶");
            });

            ui.add_space(4.0);
            if ui
                .add(
                    egui::Button::new(RichText::new("アートボードを編集").size(11.0))
                        .min_size(Vec2::new(ui.available_width(), 24.0)),
                )
                .clicked()
            {
                // Toggle artboard editing
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // 定規とグリッド (Image 1 & 3: 3つのアイコンボタン [📏][▦][▧])
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("定規とグリッド")
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let transp_btn = egui::Button::new("▧");
                    let _ = ui.add(transp_btn).on_hover_text("透明グリッドを表示");
                    let g_btn = if state.show_grid {
                        egui::Button::new("▦").fill(Color32::from_rgb(20, 115, 230))
                    } else {
                        egui::Button::new("▦")
                    };
                    if ui
                        .add(g_btn)
                        .on_hover_text("グリッドを表示/非表示")
                        .clicked()
                    {
                        state.show_grid = !state.show_grid;
                    }
                    let r_btn = if state.show_rulers {
                        egui::Button::new("📏").fill(Color32::from_rgb(20, 115, 230))
                    } else {
                        egui::Button::new("📏")
                    };
                    if ui.add(r_btn).on_hover_text("定規を表示/非表示").clicked() {
                        state.show_rulers = !state.show_rulers;
                    }
                });
            });

            ui.add_space(4.0);
            // ガイド (Image 1 & 3: 3つのアイコンボタン [⚡][🔒][⇹])
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("ガイド")
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let rel_btn = egui::Button::new("⇹");
                    let _ = ui.add(rel_btn).on_hover_text("ガイドを解除");
                    let g_lock = if state.snap_to_guides {
                        egui::Button::new("🔒").fill(Color32::from_rgb(20, 115, 230))
                    } else {
                        egui::Button::new("🔓")
                    };
                    if ui.add(g_lock).on_hover_text("ガイドにスナップ").clicked() {
                        state.snap_to_guides = !state.snap_to_guides;
                    }
                    let sg_btn = if state.show_smart_guides {
                        egui::Button::new("⚡").fill(Color32::from_rgb(20, 115, 230))
                    } else {
                        egui::Button::new("⚡")
                    };
                    if ui
                        .add(sg_btn)
                        .on_hover_text("スマートガイド (Cmd+U)")
                        .clicked()
                    {
                        state.show_smart_guides = !state.show_smart_guides;
                    }
                });
            });

            ui.add_space(4.0);
            // スナップオプション (Image 1 & 3: 3つのアイコンボタン [🧲][☵][☶])
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("スナップオプション")
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let px_btn = egui::Button::new("☶");
                    let _ = ui.add(px_btn).on_hover_text("ピクセルにスナップ");
                    let sg_btn = if state.snap_to_grid {
                        egui::Button::new("☵").fill(Color32::from_rgb(20, 115, 230))
                    } else {
                        egui::Button::new("☵")
                    };
                    if ui.add(sg_btn).on_hover_text("グリッドにスナップ").clicked() {
                        state.snap_to_grid = !state.snap_to_grid;
                    }
                    let sp_btn = if state.snap_to_objects {
                        egui::Button::new("🧲").fill(Color32::from_rgb(20, 115, 230))
                    } else {
                        egui::Button::new("🧲")
                    };
                    if ui.add(sp_btn).on_hover_text("ポイントにスナップ").clicked() {
                        state.snap_to_objects = !state.snap_to_objects;
                    }
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // 環境設定 (Image 3下部)
            ui.label(
                RichText::new("環境設定")
                    .strong()
                    .size(11.5)
                    .color(Color32::WHITE),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("キー入力:")
                        .size(10.5)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                ui.label(RichText::new("0.3528 mm").size(10.5).color(Color32::WHITE));
            });

            let mut dummy_preview_bounds = false;
            let mut dummy_scale_corners = true;
            let mut dummy_scale_strokes = true;
            ui.checkbox(&mut dummy_preview_bounds, "プレビュー境界を使用");
            ui.checkbox(&mut dummy_scale_corners, "角を拡大・縮小");
            ui.checkbox(&mut dummy_scale_strokes, "線幅と効果を拡大・縮小");
        }
    }
}
