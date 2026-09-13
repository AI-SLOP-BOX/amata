use super::ActiveTab;
use super::IrasuApp;
use crate::core::state::Tool;
use egui::{self, Color32, Vec2};

impl IrasuApp {
    pub(super) fn show_control_bar(&mut self, ctx: &egui::Context) {
        // Top Horizontal Control / Options Bar (Illustrator Signature Bar)
        egui::TopBottomPanel::top("control_bar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let has_sel = !self.state.selected_ids.is_empty();
                    if has_sel {
                        // Read live values from first selected object
                        let first_id = self.state.selected_ids.first().cloned();
                        if let Some(ref fid) = first_id {
                            #[allow(clippy::type_complexity)]
                            let snap: Option<(
                                [f32; 4],
                                [f32; 4],
                                f64,
                                f64,
                                f64,
                                f64,
                                f64,
                            )> = self
                                .state
                                .document
                                .all_objects()
                                .find(|(_, o)| &o.id == fid)
                                .map(|(_, obj)| {
                                    let fill = obj
                                        .fill
                                        .as_ref()
                                        .map(|f| f.color)
                                        .unwrap_or([0.0, 0.0, 0.0, 0.0]);
                                    let stroke = obj
                                        .stroke
                                        .as_ref()
                                        .map(|s| s.color)
                                        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                                    let sw = obj.stroke.as_ref().map(|s| s.width).unwrap_or(1.0);
                                    let (bw, bh) = obj
                                        .bounding_box()
                                        .map(|(mn, mx)| (mx.x - mn.x, mx.y - mn.y))
                                        .unwrap_or((0.0, 0.0));
                                    (fill, stroke, sw, obj.transform.x, obj.transform.y, bw, bh)
                                });

                            if let Some((
                                obj_fill,
                                obj_stroke,
                                obj_sw,
                                obj_tx,
                                obj_ty,
                                obj_bw,
                                obj_bh,
                            )) = snap
                            {
                                ui.label(
                                    egui::RichText::new("選択したオブジェクト")
                                        .weak()
                                        .size(11.0),
                                );
                                ui.separator();

                                // Fill Swatch Picker
                                let mut fill_c = [
                                    (obj_fill[0] * 255.0) as u8,
                                    (obj_fill[1] * 255.0) as u8,
                                    (obj_fill[2] * 255.0) as u8,
                                    (obj_fill[3] * 255.0) as u8,
                                ];
                                if ui
                                    .color_edit_button_srgba_unmultiplied(&mut fill_c)
                                    .on_hover_text("塗りカラー")
                                    .changed()
                                {
                                    let new_fill = [
                                        fill_c[0] as f32 / 255.0,
                                        fill_c[1] as f32 / 255.0,
                                        fill_c[2] as f32 / 255.0,
                                        fill_c[3] as f32 / 255.0,
                                    ];
                                    self.state.fill_color = new_fill;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id {
                                                obj.fill = Some(
                                                    crate::core::path::FillStyle::solid(new_fill),
                                                );
                                            }
                                        }
                                    }
                                }

                                // Stroke Swatch Picker
                                let mut stroke_c = [
                                    (obj_stroke[0] * 255.0) as u8,
                                    (obj_stroke[1] * 255.0) as u8,
                                    (obj_stroke[2] * 255.0) as u8,
                                    (obj_stroke[3] * 255.0) as u8,
                                ];
                                if ui
                                    .color_edit_button_srgba_unmultiplied(&mut stroke_c)
                                    .on_hover_text("線カラー")
                                    .changed()
                                {
                                    let new_sc = [
                                        stroke_c[0] as f32 / 255.0,
                                        stroke_c[1] as f32 / 255.0,
                                        stroke_c[2] as f32 / 255.0,
                                        stroke_c[3] as f32 / 255.0,
                                    ];
                                    self.state.stroke_color = new_sc;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id {
                                                match obj.stroke.as_mut() {
                                                    Some(s) => s.color = new_sc,
                                                    None => {
                                                        obj.stroke =
                                                            Some(crate::core::path::StrokeStyle {
                                                                color: new_sc,
                                                                width: obj_sw,
                                                                dash_pattern: None,
                                                                ..Default::default()
                                                            })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Stroke Width
                                ui.label(egui::RichText::new("線:").size(11.0));
                                let mut sw = obj_sw;
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut sw)
                                            .speed(0.1)
                                            .range(0.0..=200.0)
                                            .suffix(" pt"),
                                    )
                                    .changed()
                                {
                                    self.state.stroke_width = sw;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id {
                                                if let Some(ref mut s) = obj.stroke {
                                                    s.width = sw;
                                                }
                                            }
                                        }
                                    }
                                }

                                ui.separator();

                                // Variable Width Profile combo
                                egui::ComboBox::from_id_salt("ctrl_profile_sel")
                                    .selected_text("均等")
                                    .width(60.0)
                                    .show_ui(ui, |ui| {
                                        let _ = ui.selectable_label(true, "均等");
                                        let _ = ui.selectable_label(false, "線幅プロファイル 1");
                                        let _ = ui.selectable_label(false, "線幅プロファイル 2");
                                    });

                                // Brush Definition combo
                                egui::ComboBox::from_id_salt("ctrl_brush_sel")
                                    .selected_text("• 3 pt. 丸筆")
                                    .width(85.0)
                                    .show_ui(ui, |ui| {
                                        let _ = ui.selectable_label(true, "• 3 pt. 丸筆");
                                        let _ = ui.selectable_label(false, "• 5 pt. 楕円筆");
                                        let _ = ui.selectable_label(false, "木炭・鉛筆");
                                    });

                                ui.separator();

                                // Opacity
                                let mut op = self.state.opacity;
                                ui.label(egui::RichText::new("不透明度:").size(11.0));
                                if ui
                                    .add(
                                        egui::Slider::new(&mut op, 0.0..=1.0)
                                            .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                                    )
                                    .changed()
                                {
                                    self.state.opacity = op;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id {
                                                obj.opacity = op;
                                            }
                                        }
                                    }
                                }

                                ui.separator();

                                // Style
                                ui.label(egui::RichText::new("スタイル:").size(11.0));
                                let (sq_rect, _) = ui.allocate_exact_size(
                                    Vec2::new(16.0, 16.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(sq_rect, 2.0, Color32::WHITE);
                                ui.painter().rect_stroke(
                                    sq_rect,
                                    2.0,
                                    egui::Stroke::new(1.0_f32, Color32::from_rgb(90, 90, 90)),
                                    egui::StrokeKind::Outside,
                                );

                                ui.separator();

                                // Transform coordinates: X, Y, W, H
                                let mut tx = obj_tx;
                                ui.label(egui::RichText::new("X").weak().size(11.0));
                                if ui
                                    .add(egui::DragValue::new(&mut tx).speed(0.5).max_decimals(1))
                                    .changed()
                                {
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, o) in self.state.document.all_objects_mut() {
                                            if &o.id == id {
                                                o.transform.x = tx;
                                            }
                                        }
                                    }
                                }

                                let mut ty = obj_ty;
                                ui.label(egui::RichText::new("Y").weak().size(11.0));
                                if ui
                                    .add(egui::DragValue::new(&mut ty).speed(0.5).max_decimals(1))
                                    .changed()
                                {
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, o) in self.state.document.all_objects_mut() {
                                            if &o.id == id {
                                                o.transform.y = ty;
                                            }
                                        }
                                    }
                                }

                                let mut bw = obj_bw;
                                ui.label(egui::RichText::new("W").weak().size(11.0));
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut bw)
                                            .speed(0.5)
                                            .max_decimals(1)
                                            .range(0.1..=99999.0),
                                    )
                                    .changed()
                                    && obj_bw > 0.0
                                {
                                    let scale = bw / obj_bw;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, o) in self.state.document.all_objects_mut() {
                                            if &o.id == id {
                                                o.transform.scale_x *= scale;
                                            }
                                        }
                                    }
                                }

                                let mut bh = obj_bh;
                                ui.label(egui::RichText::new("H").weak().size(11.0));
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut bh)
                                            .speed(0.5)
                                            .max_decimals(1)
                                            .range(0.1..=99999.0),
                                    )
                                    .changed()
                                    && obj_bh > 0.0
                                {
                                    let scale = bh / obj_bh;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        for (_, o) in self.state.document.all_objects_mut() {
                                            if &o.id == id {
                                                o.transform.scale_y *= scale;
                                            }
                                        }
                                    }
                                }

                                ui.separator();
                                if ui.small_button("シェイプを編集").clicked() {
                                    self.state.current_tool = Tool::Node;
                                }
                                if ui.small_button("変形").clicked() {
                                    self.active_tab = ActiveTab::Properties;
                                }
                            }
                        }
                    } else {
                        // Illustrator CC Signature Unselected Control Bar (Image 1 & Image 3)
                        ui.label(egui::RichText::new("選択なし").weak().size(11.0));
                        ui.separator();

                        // Fill Swatch Picker button
                        let mut fill_c = [
                            (self.state.fill_color[0] * 255.0) as u8,
                            (self.state.fill_color[1] * 255.0) as u8,
                            (self.state.fill_color[2] * 255.0) as u8,
                            (self.state.fill_color[3] * 255.0) as u8,
                        ];
                        if ui
                            .color_edit_button_srgba_unmultiplied(&mut fill_c)
                            .on_hover_text("塗りカラー")
                            .changed()
                        {
                            self.state.fill_color = [
                                fill_c[0] as f32 / 255.0,
                                fill_c[1] as f32 / 255.0,
                                fill_c[2] as f32 / 255.0,
                                fill_c[3] as f32 / 255.0,
                            ];
                        }

                        // Stroke Swatch Picker button
                        let mut stroke_c = [
                            (self.state.stroke_color[0] * 255.0) as u8,
                            (self.state.stroke_color[1] * 255.0) as u8,
                            (self.state.stroke_color[2] * 255.0) as u8,
                            (self.state.stroke_color[3] * 255.0) as u8,
                        ];
                        if ui
                            .color_edit_button_srgba_unmultiplied(&mut stroke_c)
                            .on_hover_text("線カラー")
                            .changed()
                        {
                            self.state.stroke_color = [
                                stroke_c[0] as f32 / 255.0,
                                stroke_c[1] as f32 / 255.0,
                                stroke_c[2] as f32 / 255.0,
                                stroke_c[3] as f32 / 255.0,
                            ];
                        }

                        // Stroke Width
                        ui.label(egui::RichText::new("線:").size(11.0));
                        let mut sw = self.state.stroke_width;
                        if ui
                            .add(
                                egui::DragValue::new(&mut sw)
                                    .speed(0.1)
                                    .range(0.0..=200.0)
                                    .suffix(" pt"),
                            )
                            .changed()
                        {
                            self.state.stroke_width = sw;
                        }

                        ui.separator();

                        // Variable Width Profile combo
                        egui::ComboBox::from_id_salt("ctrl_profile")
                            .selected_text("均等")
                            .width(65.0)
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "均等");
                                let _ = ui.selectable_label(false, "線幅プロファイル 1");
                                let _ = ui.selectable_label(false, "線幅プロファイル 2");
                            });

                        // Brush Definition combo
                        egui::ComboBox::from_id_salt("ctrl_brush")
                            .selected_text("• 3 pt. 丸筆")
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "• 3 pt. 丸筆");
                                let _ = ui.selectable_label(false, "• 5 pt. 楕円筆");
                                let _ = ui.selectable_label(false, "木炭・鉛筆");
                            });

                        ui.separator();

                        // Opacity
                        let mut op = self.state.opacity;
                        ui.label(egui::RichText::new("不透明度:").size(11.0));
                        if ui
                            .add(
                                egui::Slider::new(&mut op, 0.0..=1.0)
                                    .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                            )
                            .changed()
                        {
                            self.state.opacity = op;
                        }

                        ui.separator();

                        // Graphic Style Picker
                        ui.label(egui::RichText::new("スタイル:").size(11.0));
                        let (sq_rect, _) =
                            ui.allocate_exact_size(Vec2::new(18.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(sq_rect, 2.0, Color32::WHITE);
                        ui.painter().rect_stroke(
                            sq_rect,
                            2.0,
                            egui::Stroke::new(1.0_f32, Color32::from_rgb(90, 90, 90)),
                            egui::StrokeKind::Outside,
                        );
                    }

                    // Right side of Control Bar (Document Setup, Preferences)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("環境設定")
                            .on_hover_text("環境設定ダイアログを開く (Cmd+K)")
                            .clicked()
                        {
                            self.preferences_dialog.is_open = true;
                        }
                        if ui
                            .small_button("ドキュメント設定")
                            .on_hover_text("アートボードおよびドキュメント寸法の変更")
                            .clicked()
                        {
                            self.active_tab = ActiveTab::Properties;
                        }
                    });
                });
            });
    }
}
