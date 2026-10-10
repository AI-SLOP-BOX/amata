use crate::app::icons::{
    icon_align_bottom, icon_align_center_h, icon_align_center_v, icon_align_left, icon_align_right,
    icon_align_top, icon_bolt, icon_button_labeled, icon_checkerboard, icon_grid, icon_ruler,
    icon_seek_next, icon_seek_prev, icon_snap_grid, icon_snap_pixels, icon_snap_points,
    icon_unlink, icon_unlock, toggle_icon_button_labeled,
};
use crate::core::document::ObjectType;
use crate::core::path::FillStyle;
use crate::core::state::{AppState, Tool};
use crate::ui::panels::color_utils::color_edit_srgba_u8;
use egui::{Color32, Pos2, RichText, Stroke, StrokeKind, Ui, Vec2};

pub struct PropertyPanel;

impl PropertyPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.label(
            RichText::new(crate::ui::i18n::text(&locale, "dock.properties"))
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
            let mut rect_wh: Option<(f64, f64, f64)> = None;

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
                    if let ObjectType::Rectangle {
                        width,
                        height,
                        corner_radius,
                    } = &obj.object_type
                    {
                        rect_wh = Some((*width, *height, *corner_radius));
                    }
                    break;
                }
            }

            // Header editable name (e.g. "パス", "長方形", "グループ" or custom name)
            ui.horizontal(|ui| {
                let default_hint = if is_path {
                    crate::ui::i18n::text(&locale, "property.object_path").into_owned()
                } else {
                    crate::ui::i18n::text(&locale, "property.object").into_owned()
                };
                let mut edit_name = obj_name.clone();
                let name_resp = ui.add(
                    egui::TextEdit::singleline(&mut edit_name)
                        .hint_text(default_hint)
                        .desired_width(180.0),
                );
                // Coalesce the whole rename (one keystroke per frame) into a
                // single undo step committed on focus loss.
                if name_resp.has_focus() {
                    state.ensure_object_snapshot(&id);
                }
                if name_resp.changed() {
                    if let Some(o) = state.document.find_object_mut(&id) {
                        o.name = edit_name.clone();
                    }
                }
                if name_resp.lost_focus() {
                    state.commit_object_edits("Rename Object");
                }
            });
            ui.add_space(4.0);

            // ─── 変形 (Transform) — Image 6 Reference Layout ───
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "property.transform"))
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(4.0);

            // Display unit for every coordinate/size field in this panel.
            let unit = state.prefs.ruler_unit;

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
                        let mut x_val = unit.from_px(tx);
                        let x_resp = ui.add(
                            egui::DragValue::new(&mut x_val)
                                .speed(unit.from_px(0.5))
                                .suffix(unit.suffix_label()),
                        );
                        if x_resp.changed() {
                            // Move every selected object by the same delta so
                            // multi-selection keeps its relative layout.
                            // (Previously all objects were stacked onto the
                            // first object's absolute value.)
                            let dx = unit.to_px(x_val) - tx;
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
                        let mut w_val = unit.from_px(bw);
                        let w_resp = ui.add(
                            egui::DragValue::new(&mut w_val)
                                .speed(unit.from_px(0.5))
                                .suffix(unit.suffix_label()),
                        );
                        if w_resp.changed() && bw > 0.0 {
                            let scale = unit.to_px(w_val) / bw;
                            let (scale_corners, scale_strokes_effects) =
                                (state.prefs.scale_corners, state.prefs.scale_strokes_effects);
                            let sel = state.selected_ids.clone();
                            for id in &sel {
                                state.ensure_transform_snapshot(id);
                                if let Some(o) = state.document.find_object_mut(id) {
                                    let before = o.visual_scale();
                                    o.transform.scale_x *= scale;
                                    // Resizing by a numeric field is a scale
                                    // just like the handles: honour the
                                    // 環境設定 switches about it.
                                    o.apply_scale_change(
                                        o.visual_scale() / before,
                                        scale_corners,
                                        scale_strokes_effects,
                                    );
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
                        let mut y_val = unit.from_px(ty);
                        let y_resp = ui.add(
                            egui::DragValue::new(&mut y_val)
                                .speed(unit.from_px(0.5))
                                .suffix(unit.suffix_label()),
                        );
                        if y_resp.changed() {
                            let dy = unit.to_px(y_val) - ty;
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
                        let mut h_val = unit.from_px(bh);
                        let h_resp = ui.add(
                            egui::DragValue::new(&mut h_val)
                                .speed(unit.from_px(0.5))
                                .suffix(unit.suffix_label()),
                        );
                        if h_resp.changed() && bh > 0.0 {
                            let scale = unit.to_px(h_val) / bh;
                            let (scale_corners, scale_strokes_effects) =
                                (state.prefs.scale_corners, state.prefs.scale_strokes_effects);
                            let sel = state.selected_ids.clone();
                            for id in &sel {
                                state.ensure_transform_snapshot(id);
                                if let Some(o) = state.document.find_object_mut(id) {
                                    let before = o.visual_scale();
                                    o.transform.scale_y *= scale;
                                    o.apply_scale_change(
                                        o.visual_scale() / before,
                                        scale_corners,
                                        scale_strokes_effects,
                                    );
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
                        let r_resp =
                            ui.add(egui::DragValue::new(&mut r_val).speed(1.0).suffix("°"));
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

            // ─── 角丸 (Corner Radius) — Rectangle only ───
            if let Some((rw, rh, cr)) = rect_wh {
                let max_r = rw.abs().min(rh.abs()) * 0.5;
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(&locale, "property.corner_radius"))
                            .size(10.5)
                            .color(Color32::from_rgb(150, 150, 150)),
                    );
                    let mut cr_val = unit.from_px(cr);
                    let cr_resp = ui.add(
                        egui::DragValue::new(&mut cr_val)
                            .speed(unit.from_px(0.5))
                            .range(0.0..=unit.from_px(max_r))
                            .suffix(unit.suffix_label()),
                    );
                    if cr_resp.changed() {
                        let cr = unit.to_px(cr_val).clamp(0.0, max_r);
                        state.object_edit(&id, &cr_resp, |o| {
                            if let ObjectType::Rectangle { corner_radius, .. } = &mut o.object_type
                            {
                                *corner_radius = cr;
                            }
                        });
                    }
                    if cr_resp.drag_stopped() {
                        state.commit_object_edits("Corner Radius");
                    }
                });
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // ─── アピアランス (Appearance) — Image 6 Reference Layout ───
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "property.appearance"))
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
                let fill_resp = color_edit_srgba_u8(ui, &mut c_rgba);
                if fill_resp.changed() {
                    let new_fill = [
                        c_rgba[0] as f32 / 255.0,
                        c_rgba[1] as f32 / 255.0,
                        c_rgba[2] as f32 / 255.0,
                        c_rgba[3] as f32 / 255.0,
                    ];
                    state.fill_color = new_fill;
                    let sel = state.selected_ids.clone();
                    state.objects_edit(&sel, &fill_resp, |o| {
                        o.fill = Some(FillStyle::solid(new_fill));
                    });
                }
                if fill_resp.drag_stopped() {
                    state.commit_object_edits("Edit Object");
                }
                ui.label(
                    RichText::new(crate::ui::i18n::text(&locale, "property.fill"))
                        .size(11.0)
                        .color(Color32::WHITE),
                );
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
                let sc_resp = color_edit_srgba_u8(ui, &mut sc_rgba);
                if sc_resp.changed() {
                    let new_sc = [
                        sc_rgba[0] as f32 / 255.0,
                        sc_rgba[1] as f32 / 255.0,
                        sc_rgba[2] as f32 / 255.0,
                        sc_rgba[3] as f32 / 255.0,
                    ];
                    state.stroke_color = new_sc;
                    let default_w = state.stroke_width.max(1.0);
                    let sel = state.selected_ids.clone();
                    state.objects_edit(&sel, &sc_resp, |o| {
                        if let Some(ref mut s) = o.stroke {
                            s.color = new_sc;
                        } else {
                            o.stroke = Some(crate::core::path::StrokeStyle {
                                color: new_sc,
                                width: default_w,
                                ..Default::default()
                            });
                        }
                    });
                }
                if sc_resp.drag_stopped() {
                    state.commit_object_edits("Edit Object");
                }
                ui.label(
                    RichText::new(crate::ui::i18n::text(&locale, "property.stroke"))
                        .size(11.0)
                        .color(Color32::WHITE),
                );

                let su = state.prefs.stroke_unit;
                let mut sw = su.from_px(
                    stroke_opt
                        .as_ref()
                        .map(|s| s.width)
                        .unwrap_or(state.stroke_width),
                );
                let sw_resp = ui.add(
                    egui::DragValue::new(&mut sw)
                        .speed(su.from_px(0.2))
                        .range(0.0..=su.from_px(100.0))
                        .suffix(su.suffix_label()),
                );
                if sw_resp.changed() {
                    let sw = su.to_px(sw);
                    state.stroke_width = sw;
                    let sc = state.stroke_color;
                    let sel = state.selected_ids.clone();
                    state.objects_edit(&sel, &sw_resp, |o| {
                        if let Some(ref mut s) = o.stroke {
                            s.width = sw;
                        } else if sw > 0.0 {
                            o.stroke = Some(crate::core::path::StrokeStyle {
                                color: sc,
                                width: sw,
                                ..Default::default()
                            });
                        }
                    });
                }
                if sw_resp.drag_stopped() {
                    state.commit_object_edits("Edit Object");
                }
            });

            // Opacity line
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(crate::ui::i18n::text(&locale, "property.opacity"))
                        .size(11.0)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                let mut op = opac;
                let op_resp = ui.add(
                    egui::Slider::new(&mut op, 0.0..=1.0)
                        .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                );
                if op_resp.changed() {
                    state.opacity = op;
                    let sel = state.selected_ids.clone();
                    state.objects_edit(&sel, &op_resp, |o| {
                        o.opacity = op;
                    });
                }
                if op_resp.drag_stopped() {
                    state.commit_object_edits("Edit Object");
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // ─── クイックアクション (Quick Actions) — Image 6 Reference Layout ───
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "property.quick_actions"))
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(crate::ui::i18n::text(&locale, "property.offset_path"))
                                .size(10.5),
                        )
                        .min_size(Vec2::new(105.0, 24.0)),
                    )
                    .clicked()
                {
                    let sel = state.selected_ids.clone();
                    state.undoable_snapshot("Offset Path", &sel, |doc| {
                        for id in &sel {
                            if let Some(o) = doc.find_object_mut(id) {
                                let path = o.to_path_data();
                                let off = crate::core::offset::offset_path(&path, 5.0);
                                o.object_type = ObjectType::Path(off);
                            }
                        }
                    });
                }
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(crate::ui::i18n::text(&locale, "property.expand_path"))
                                .size(10.5),
                        )
                        .min_size(Vec2::new(105.0, 24.0)),
                    )
                    .clicked()
                {
                    let sel = state.selected_ids.clone();
                    state.undoable_snapshot("Outline Stroke", &sel, |doc| {
                        for id in &sel {
                            if let Some(o) = doc.find_object_mut(id) {
                                let path = o.to_path_data();
                                let stroke = o.stroke.clone().unwrap_or_default();
                                let outlined =
                                    crate::core::offset::outline_stroke(&path, stroke.width);
                                o.object_type = ObjectType::Path(outlined);
                            }
                        }
                    });
                }
            });

            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(crate::ui::i18n::text(&locale, "property.convert_shape"))
                                .size(10.5),
                        )
                        .min_size(Vec2::new(105.0, 24.0)),
                    )
                    .clicked()
                {
                    state.current_tool = Tool::ShapeBuilder;
                }
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(crate::ui::i18n::text(&locale, "property.pixel_align"))
                                .size(10.5),
                        )
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
                    RichText::new(crate::ui::i18n::text(&locale, "property.align"))
                        .strong()
                        .size(12.0)
                        .color(Color32::WHITE),
                );
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let size = Vec2::new(22.0, 18.0);
                let sel = state.selected_ids.clone();
                if icon_button_labeled(
                    ui,
                    size,
                    icon_align_left,
                    crate::ui::i18n::text(&locale, "property.align_left"),
                )
                .on_hover_text(crate::ui::i18n::text(&locale, "property.align_left"))
                .clicked()
                {
                    crate::ui::AlignPanel::align_left(state, &sel);
                }
                if icon_button_labeled(
                    ui,
                    size,
                    icon_align_center_h,
                    crate::ui::i18n::text(&locale, "property.align_h_center"),
                )
                .on_hover_text(crate::ui::i18n::text(&locale, "property.align_h_center"))
                .clicked()
                {
                    crate::ui::AlignPanel::align_center_h(state, &sel);
                }
                if icon_button_labeled(
                    ui,
                    size,
                    icon_align_right,
                    crate::ui::i18n::text(&locale, "property.align_right"),
                )
                .on_hover_text(crate::ui::i18n::text(&locale, "property.align_right"))
                .clicked()
                {
                    crate::ui::AlignPanel::align_right(state, &sel);
                }
                ui.add_space(4.0);
                if icon_button_labeled(
                    ui,
                    size,
                    icon_align_top,
                    crate::ui::i18n::text(&locale, "property.align_top"),
                )
                .on_hover_text(crate::ui::i18n::text(&locale, "property.align_top"))
                .clicked()
                {
                    crate::ui::AlignPanel::align_top(state, &sel);
                }
                if icon_button_labeled(
                    ui,
                    size,
                    icon_align_center_v,
                    crate::ui::i18n::text(&locale, "property.align_v_center"),
                )
                .on_hover_text(crate::ui::i18n::text(&locale, "property.align_v_center"))
                .clicked()
                {
                    crate::ui::AlignPanel::align_center_v(state, &sel);
                }
                if icon_button_labeled(
                    ui,
                    size,
                    icon_align_bottom,
                    crate::ui::i18n::text(&locale, "property.align_bottom"),
                )
                .on_hover_text(crate::ui::i18n::text(&locale, "property.align_bottom"))
                .clicked()
                {
                    crate::ui::AlignPanel::align_bottom(state, &sel);
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // ─── パスファインダー (Pathfinder) — Image 6 Reference Layout ───
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "property.pathfinder"))
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui
                    .button(
                        RichText::new(crate::ui::i18n::text(&locale, "property.unite")).size(10.5),
                    )
                    .clicked()
                {
                    crate::ui::PathfinderPanel::apply_op(
                        state,
                        crate::core::boolean::BooleanOp::Union,
                    );
                }
                if ui
                    .button(
                        RichText::new(crate::ui::i18n::text(&locale, "property.subtract"))
                            .size(10.5),
                    )
                    .clicked()
                {
                    crate::ui::PathfinderPanel::apply_op(
                        state,
                        crate::core::boolean::BooleanOp::Subtract,
                    );
                }
                if ui
                    .button(
                        RichText::new(crate::ui::i18n::text(&locale, "property.intersect"))
                            .size(10.5),
                    )
                    .clicked()
                {
                    crate::ui::PathfinderPanel::apply_op(
                        state,
                        crate::core::boolean::BooleanOp::Intersect,
                    );
                }
                if ui
                    .button(
                        RichText::new(crate::ui::i18n::text(&locale, "property.exclude"))
                            .size(10.5),
                    )
                    .clicked()
                {
                    crate::ui::PathfinderPanel::apply_op(
                        state,
                        crate::core::boolean::BooleanOp::Exclude,
                    );
                }
            });
        } else {
            // Adobe CC Property Panel: Complete Document & Canvas Settings (Image 2 & 3)
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "property.none_selected"))
                    .size(11.0)
                    .color(Color32::from_rgb(140, 140, 140)),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "property.document"))
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(crate::ui::i18n::text(&locale, "property.units"))
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                egui::ComboBox::from_id_salt("prop_doc_unit")
                    .selected_text(state.prefs.ruler_unit.label())
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for u in crate::core::unit::LengthUnit::ALL {
                            if ui
                                .selectable_label(state.prefs.ruler_unit == u, u.label())
                                .clicked()
                            {
                                // Ruler, coordinates, sizes and the property
                                // fields all read `ruler_unit`.
                                state.prefs.ruler_unit = u;
                            }
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(crate::ui::i18n::text(&locale, "property.artboard"))
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                let ab_count = state.document.effective_artboards().len();
                let idx = state.active_artboard_idx;
                ui.label(
                    RichText::new(format!("{}", idx + 1))
                        .size(11.0)
                        .color(Color32::WHITE),
                );
                let nav = Vec2::new(20.0, 18.0);
                if icon_button_labeled(
                    ui,
                    nav,
                    icon_seek_prev,
                    crate::ui::i18n::text(&locale, "property.previous_artboard"),
                )
                .on_hover_text(crate::ui::i18n::text(&locale, "property.previous_artboard"))
                .clicked()
                    && idx > 0
                {
                    state.active_artboard_idx = idx - 1;
                }
                if icon_button_labeled(
                    ui,
                    nav,
                    icon_seek_next,
                    crate::ui::i18n::text(&locale, "property.next_artboard"),
                )
                .on_hover_text(crate::ui::i18n::text(&locale, "property.next_artboard"))
                .clicked()
                    && idx + 1 < ab_count
                {
                    state.active_artboard_idx = idx + 1;
                }
            });

            ui.add_space(4.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(if state.artboard_edit_open {
                            crate::ui::i18n::text(&locale, "property.close_artboard_edit")
                        } else {
                            crate::ui::i18n::text(&locale, "property.edit_artboard")
                        })
                        .size(11.0),
                    )
                    .min_size(Vec2::new(ui.available_width(), 24.0)),
                )
                .clicked()
            {
                state.artboard_edit_open = !state.artboard_edit_open;
            }
            if state.artboard_edit_open {
                Self::show_artboard_editor(ui, state);
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // 定規とグリッド (checkerboard / grid / ruler の3トグル)
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(crate::ui::i18n::text(&locale, "property.rulers_grid"))
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let size = Vec2::new(24.0, 20.0);
                    let checker =
                        state.prefs.artboard_bg_mode == crate::core::prefs::ArtboardBgMode::Checker;
                    if toggle_icon_button_labeled(
                        ui,
                        size,
                        checker,
                        icon_checkerboard,
                        crate::ui::i18n::text(&locale, "property.toggle_checkerboard"),
                    )
                    .on_hover_text(crate::ui::i18n::text(
                        &locale,
                        "property.toggle_checkerboard",
                    ))
                    .clicked()
                    {
                        state.prefs.artboard_bg_mode = if checker {
                            crate::core::prefs::ArtboardBgMode::White
                        } else {
                            crate::core::prefs::ArtboardBgMode::Checker
                        };
                    }
                    if toggle_icon_button_labeled(
                        ui,
                        size,
                        state.show_grid,
                        icon_grid,
                        crate::ui::i18n::text(&locale, "property.toggle_grid"),
                    )
                    .on_hover_text(crate::ui::i18n::text(&locale, "property.toggle_grid"))
                    .clicked()
                    {
                        state.show_grid = !state.show_grid;
                    }
                    if toggle_icon_button_labeled(
                        ui,
                        size,
                        state.show_rulers,
                        icon_ruler,
                        crate::ui::i18n::text(&locale, "property.toggle_rulers"),
                    )
                    .on_hover_text(crate::ui::i18n::text(&locale, "property.toggle_rulers"))
                    .clicked()
                    {
                        state.show_rulers = !state.show_rulers;
                    }
                });
            });

            ui.add_space(4.0);
            // ガイド (unlink / lock / bolt の3トグル)
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(crate::ui::i18n::text(&locale, "property.guides"))
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let size = Vec2::new(24.0, 20.0);
                    if toggle_icon_button_labeled(
                        ui,
                        size,
                        false,
                        icon_unlink,
                        crate::ui::i18n::text(&locale, "property.clear_guides"),
                    )
                    .on_hover_text(crate::ui::i18n::text(&locale, "property.clear_guides"))
                    .clicked()
                        && !state.guides.is_empty()
                    {
                        state.guides.clear();
                    }
                    let lock_painter: fn(&egui::Painter, egui::Rect, Color32) =
                        if state.snap_to_guides {
                            crate::app::icons::icon_lock
                        } else {
                            icon_unlock
                        };
                    if toggle_icon_button_labeled(
                        ui,
                        size,
                        state.snap_to_guides,
                        lock_painter,
                        crate::ui::i18n::text(&locale, "property.snap_guides"),
                    )
                    .on_hover_text(crate::ui::i18n::text(&locale, "property.snap_guides"))
                    .clicked()
                    {
                        state.snap_to_guides = !state.snap_to_guides;
                    }
                    if toggle_icon_button_labeled(
                        ui,
                        size,
                        state.show_smart_guides,
                        icon_bolt,
                        crate::ui::i18n::format(
                            &locale,
                            "property.smart_guides",
                            &[("key", crate::app::control_bar::mod_key())],
                        ),
                    )
                    .on_hover_text(crate::ui::i18n::format(
                        &locale,
                        "property.smart_guides",
                        &[("key", crate::app::control_bar::mod_key())],
                    ))
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
                    RichText::new(crate::ui::i18n::text(&locale, "property.snap_options"))
                        .size(11.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let size = Vec2::new(24.0, 20.0);
                    if toggle_icon_button_labeled(
                        ui,
                        size,
                        state.snap_to_pixels,
                        icon_snap_pixels,
                        crate::ui::i18n::text(&locale, "property.snap_pixels"),
                    )
                    .on_hover_text(crate::ui::i18n::text(&locale, "property.snap_pixels"))
                    .clicked()
                    {
                        state.snap_to_pixels = !state.snap_to_pixels;
                    }
                    if toggle_icon_button_labeled(
                        ui,
                        size,
                        state.snap_to_grid,
                        icon_snap_grid,
                        crate::ui::i18n::text(&locale, "property.snap_grid"),
                    )
                    .on_hover_text(crate::ui::i18n::text(&locale, "property.snap_grid"))
                    .clicked()
                    {
                        state.snap_to_grid = !state.snap_to_grid;
                    }
                    if toggle_icon_button_labeled(
                        ui,
                        size,
                        state.snap_to_objects,
                        icon_snap_points,
                        crate::ui::i18n::text(&locale, "property.snap_points"),
                    )
                    .on_hover_text(crate::ui::i18n::text(&locale, "property.snap_points"))
                    .clicked()
                    {
                        state.snap_to_objects = !state.snap_to_objects;
                    }
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // 環境設定 (Image 3下部)
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "property.preferences"))
                    .strong()
                    .size(11.5)
                    .color(Color32::WHITE),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(crate::ui::i18n::text(
                        &locale,
                        "property.keyboard_increment",
                    ))
                    .size(10.5)
                    .color(Color32::from_rgb(160, 160, 160)),
                );
                ui.label(RichText::new("0.3528 mm").size(10.5).color(Color32::WHITE));
            });

            // The three transform switches. They live in `Prefs`, so they
            // survive a restart like every other 環境設定 — unlike the dummy
            // locals these used to be, they actually reach the transform:
            // `Object::apply_scale_change` and `preview_bounds`.
            ui.checkbox(
                &mut state.prefs.use_preview_bounds,
                crate::ui::i18n::text(&locale, "property.preview_bounds"),
            )
            .on_hover_text(crate::ui::i18n::text(
                &locale,
                "property.preview_bounds_tip",
            ));
            ui.checkbox(
                &mut state.prefs.scale_corners,
                crate::ui::i18n::text(&locale, "property.scale_corners"),
            )
            .on_hover_text(crate::ui::i18n::text(&locale, "property.scale_corners_tip"));
            ui.checkbox(
                &mut state.prefs.scale_strokes_effects,
                crate::ui::i18n::text(&locale, "property.scale_strokes"),
            )
            .on_hover_text(crate::ui::i18n::text(&locale, "property.scale_strokes_tip"));
        }
    }

    /// Inline artboard editor behind the "アートボードを編集" button:
    /// rename, move and resize the active artboard, plus add / remove.
    /// Each gesture collapses into a single undo step through
    /// [`AppState::artboard_edit`] / [`AppState::commit_artboard_edits`].
    fn show_artboard_editor(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.add_space(4.0);

        // A fresh document only carries an *implicit* artboard derived from
        // the document size; materialize it so the edits have somewhere to
        // live (same approach as the layout-grid panel).
        if state.document.artboards.is_empty() {
            let w = state.document.width;
            let h = state.document.height;
            state
                .document
                .artboards
                .push(crate::core::document::Artboard::new(
                    "Artboard 1",
                    0.0,
                    0.0,
                    w,
                    h,
                ));
        }
        if state.active_artboard_idx >= state.document.artboards.len() {
            state.active_artboard_idx = state.document.artboards.len() - 1;
        }
        let idx = state.active_artboard_idx;

        let mut name = state.document.artboards[idx].name.clone();
        let name_resp = ui.add(
            egui::TextEdit::singleline(&mut name)
                .hint_text(crate::ui::i18n::text(&locale, "property.artboard_name")),
        );
        if name_resp.changed() {
            state.artboard_edit(&name_resp, "Rename Artboard", |abs| {
                if let Some(ab) = abs.get_mut(idx) {
                    ab.name = name;
                }
            });
        }

        let mut x = state.document.artboards[idx].x;
        let mut y = state.document.artboards[idx].y;
        let mut w = state.document.artboards[idx].width;
        let mut h = state.document.artboards[idx].height;

        let mut changed = false;
        let mut dragging = false;
        let mut stopped = false;
        let mut track = |r: &egui::Response| {
            changed |= r.changed();
            dragging |= r.dragged();
            stopped |= r.drag_stopped();
        };

        ui.horizontal(|ui| {
            ui.label("X:");
            track(&ui.add(egui::DragValue::new(&mut x).speed(1.0)));
            ui.label("Y:");
            track(&ui.add(egui::DragValue::new(&mut y).speed(1.0)));
        });
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "property.width"));
            track(
                &ui.add(
                    egui::DragValue::new(&mut w)
                        .speed(1.0)
                        .range(1.0..=100_000.0),
                ),
            );
            ui.label(crate::ui::i18n::text(&locale, "property.height"));
            track(
                &ui.add(
                    egui::DragValue::new(&mut h)
                        .speed(1.0)
                        .range(1.0..=100_000.0),
                ),
            );
        });

        if changed {
            state.ensure_artboard_snapshot();
            if let Some(ab) = state.document.artboards.get_mut(idx) {
                ab.x = x;
                ab.y = y;
                ab.width = w;
                ab.height = h;
            }
            if !dragging {
                state.commit_artboard_edits("Edit Artboard");
            }
        }
        if stopped {
            state.commit_artboard_edits("Edit Artboard");
        }

        ui.horizontal(|ui| {
            if ui
                .button(crate::ui::i18n::text(&locale, "property.add_artboard"))
                .clicked()
            {
                let before = state.document.artboards.clone();
                let cur = state.document.artboards[idx].clone();
                let count = state.document.artboards.len() + 1;
                state
                    .document
                    .artboards
                    .push(crate::core::document::Artboard::new(
                        &format!("Artboard {count}"),
                        cur.x + cur.width + 20.0,
                        cur.y,
                        cur.width,
                        cur.height,
                    ));
                state.active_artboard_idx = state.document.artboards.len() - 1;
                let after = state.document.artboards.clone();
                state.push_artboards_undo("Add Artboard", before, after);
                state.notify_success(
                    crate::ui::i18n::text(&locale, "property.artboard_added").into_owned(),
                );
            }
            let removable = state.document.artboards.len() > 1;
            if ui
                .add_enabled(
                    removable,
                    egui::Button::new(crate::ui::i18n::text(&locale, "common.delete")),
                )
                .on_disabled_hover_text(crate::ui::i18n::text(&locale, "property.minimum_artboard"))
                .clicked()
            {
                let before = state.document.artboards.clone();
                state.document.artboards.remove(idx);
                if state.active_artboard_idx >= state.document.artboards.len() {
                    state.active_artboard_idx = state.document.artboards.len() - 1;
                }
                let after = state.document.artboards.clone();
                state.push_artboards_undo("Remove Artboard", before, after);
                state.notify_success(
                    crate::ui::i18n::text(&locale, "property.artboard_removed").into_owned(),
                );
            }
        });
    }
}
