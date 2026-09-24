use super::ActiveTab;
use super::IrasuApp;
use crate::core::state::Tool;
use egui::{self, Color32, Vec2};

// Progressive disclosure thresholds: below each width the matching
// controls hide so the bar stays usable at the 640px window minimum.
pub(crate) const SHOW_XY_W: f32 = 760.0;
pub(crate) const SHOW_STYLE_W: f32 = 640.0;
pub(crate) const SHOW_COMBOS_W: f32 = 520.0;
pub(crate) const SHOW_MORE_W: f32 = 360.0;

/// Platform modifier shown in UI labels ("Cmd" on macOS, "Ctrl" elsewhere).
pub(crate) fn mod_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}

impl IrasuApp {
    pub(super) fn show_control_bar(&mut self, ctx: &egui::Context) {
        // Top Horizontal Control / Options Bar (Illustrator Signature Bar)
        egui::TopBottomPanel::top("control_bar")
            .resizable(false)
            .show(ctx, |ui| {
                // Wraps instead of clipping when the window is at the
                // 640px minimum (the old single horizontal row overflowed
                // and pushed the right-hand buttons off-screen).
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let has_sel = !self.state.selected_ids.is_empty();
                    let avail_before = ui.available_width();
                    // Progressive disclosure: core controls stay visible,
                    // secondary combos hide once the row is tight.
                    let show_combos = avail_before >= SHOW_COMBOS_W;
                    let show_style = avail_before >= SHOW_STYLE_W;
                    let show_xy = avail_before >= SHOW_XY_W;
                    let show_more = avail_before >= SHOW_MORE_W;
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
                                let fill_resp = ui
                                    .color_edit_button_srgba_unmultiplied(&mut fill_c)
                                    .on_hover_text("塗り");
                                if fill_resp.changed() {
                                    let new_fill = [
                                        fill_c[0] as f32 / 255.0,
                                        fill_c[1] as f32 / 255.0,
                                        fill_c[2] as f32 / 255.0,
                                        fill_c[3] as f32 / 255.0,
                                    ];
                                    self.state.fill_color = new_fill;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        self.state.ensure_object_snapshot(id);
                                        for (_, obj) in
                                            self.state.document.all_objects_mut()
                                        {
                                            if &obj.id == id {
                                                obj.fill = Some(
                                                    crate::core::path::FillStyle::solid(
                                                        new_fill,
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                }
                                if fill_resp.drag_stopped() {
                                    self.state.commit_object_edits("Edit Fill");
                                }

                                // Stroke Swatch Picker
                                let mut stroke_c = [
                                    (obj_stroke[0] * 255.0) as u8,
                                    (obj_stroke[1] * 255.0) as u8,
                                    (obj_stroke[2] * 255.0) as u8,
                                    (obj_stroke[3] * 255.0) as u8,
                                ];
                                let stroke_resp = ui
                                    .color_edit_button_srgba_unmultiplied(&mut stroke_c)
                                    .on_hover_text("線カラー");
                                if stroke_resp.changed() {
                                    let new_sc = [
                                        stroke_c[0] as f32 / 255.0,
                                        stroke_c[1] as f32 / 255.0,
                                        stroke_c[2] as f32 / 255.0,
                                        stroke_c[3] as f32 / 255.0,
                                    ];
                                    self.state.stroke_color = new_sc;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        self.state.ensure_object_snapshot(id);
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
                                if stroke_resp.drag_stopped() {
                                    self.state.commit_object_edits("Edit Stroke");
                                }

                                // Stroke Width (label drops first at the
                                // tightest tier; the unit suffix remains)
                                if show_more {
                                    ui.label(egui::RichText::new("線:").size(11.0));
                                }
                                let su = self.state.prefs.stroke_unit;
                                let mut sw = su.from_px(obj_sw);
                                let sw_resp = ui.add(
                                    egui::DragValue::new(&mut sw)
                                        .speed(su.from_px(0.1))
                                        .range(0.0..=su.from_px(200.0))
                                        .suffix(su.suffix_label()),
                                );
                                if sw_resp.changed() {
                                    let sw = su.to_px(sw);
                                    self.state.stroke_width = sw;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        self.state.ensure_object_snapshot(id);
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id {
                                                if let Some(ref mut s) = obj.stroke {
                                                    s.width = sw;
                                                }
                                            }
                                        }
                                    }
                                }
                                if sw_resp.drag_stopped() {
                                    self.state.commit_object_edits("Edit Stroke Width");
                                }

                                ui.separator();

                                // Variable Width Profile combo
                                if show_combos {
                                    let profile_name = self
                                        .state
                                        .selected_ids
                                        .first()
                                        .and_then(|id| self.state.document.find_object(id))
                                        .and_then(|o| o.width_profile.as_ref())
                                        .map(profile_display_name)
                                        .unwrap_or_else(|| "均等".to_string());
                                    egui::ComboBox::from_id_salt("ctrl_profile_sel")
                                .selected_text(&profile_name)
                                        .width(60.0)
                                        .show_ui(ui, |ui| {
                                            let selected = self
                                                .state
                                                .selected_ids
                                                .first()
                                                .and_then(|id| self.state.document.find_object(id))
                                                .and_then(|o| o.width_profile.as_ref())
                                                .map(profile_display_name)
                                                .unwrap_or_else(|| "均等".to_string());
                                            for name in
                                                ["均等", "線幅プロファイル 1", "線幅プロファイル 2"]
                                            {
                                                if ui
                                                    .selectable_label(selected == name, name)
                                                    .clicked()
                                                {
                                                    apply_width_profile_preset(
                                                        &mut self.state,
                                                        name,
                                                    );
                                                }
                                            }
                                        });

                                    // Brush Definition combo — adopts the brush
                                    // panel's params (undoable outline is done
                                    // from the Brush panel; this sets the nib).
                                    let brush_label = match self.state.brush_kind_idx {
                                        0 => {
                                            format!("• {:.0} pt. 丸筆", self.state.brush_size)
                                        }
                                        1 => "アートブラシ".to_string(),
                                        2 => "パターンブラシ".to_string(),
                                        _ => "毛先ブラシ".to_string(),
                                    };
                                    egui::ComboBox::from_id_salt("ctrl_brush_sel")
                                        .selected_text(brush_label)
                                        .width(85.0)
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    self.state.brush_kind_idx == 0,
                                                    "• 3 pt. 丸筆",
                                                )
                                                .clicked()
                                            {
                                                self.state.brush_kind_idx = 0;
                                                self.state.brush_roundness = 100.0;
                                                self.state.brush_size = 3.0;
                                            }
                                            if ui
                                                .selectable_label(
                                                    self.state.brush_kind_idx == 0
                                                        && self.state.brush_roundness < 80.0,
                                                    "• 5 pt. 楕円筆",
                                                )
                                                .clicked()
                                            {
                                                self.state.brush_kind_idx = 0;
                                                self.state.brush_roundness = 50.0;
                                                self.state.brush_size = 5.0;
                                            }
                                            if ui
                                                .selectable_label(
                                                    self.state.brush_kind_idx == 3,
                                                    "木炭・鉛筆",
                                                )
                                                .clicked()
                                            {
                                                self.state.brush_kind_idx = 3;
                                                self.state.brush_bristles = 16.0;
                                                self.state.brush_scatter = 0.5;
                                            }
                                        });

                                    ui.separator();
                                }

                                // Opacity
                                let mut op = self.state.opacity;
                                if show_more {
                                    ui.label(egui::RichText::new("不透明度:").size(11.0));
                                }
                                let op_resp = ui.add(
                                    egui::Slider::new(&mut op, 0.0..=1.0)
                                        .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                                );
                                if op_resp.changed() {
                                    self.state.opacity = op;
                                    let sel = self.state.selected_ids.clone();
                                    for id in &sel {
                                        self.state.ensure_object_snapshot(id);
                                        for (_, obj) in self.state.document.all_objects_mut() {
                                            if &obj.id == id {
                                                obj.opacity = op;
                                            }
                                        }
                                    }
                                }
                                if op_resp.drag_stopped() {
                                    self.state.commit_object_edits("Edit Opacity");
                                }

                                if show_style {
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
                                }

                                if show_xy {
                                    ui.separator();

                                    // Transform coordinates: X, Y, W, H —
                                    // shown in the ruler's display unit and
                                    // converted back to document px on write.
                                    let unit = self.state.prefs.ruler_unit;
                                    let speed = unit.from_px(0.5);
                                    let decimals = unit.field_decimals();
                                    let suffix = unit.suffix_label();

                                    let mut tx = unit.from_px(obj_tx);
                                    ui.label(egui::RichText::new("X").weak().size(11.0));
                                    let tx_resp = ui.add(
                                        egui::DragValue::new(&mut tx)
                                            .speed(speed)
                                            .max_decimals(decimals)
                                            .suffix(suffix.clone()),
                                    );
                                    if tx_resp.changed() {
                                        let tx = unit.to_px(tx);
                                        let sel = self.state.selected_ids.clone();
                                        for id in &sel {
                                            self.state.ensure_object_snapshot(id);
                                            for (_, o) in
                                                self.state.document.all_objects_mut()
                                            {
                                                if &o.id == id {
                                                    o.transform.x = tx;
                                                }
                                            }
                                        }
                                    }
                                    if tx_resp.drag_stopped() {
                                        self.state.commit_object_edits("Edit Transform");
                                    }

                                    let mut ty = unit.from_px(obj_ty);
                                    ui.label(egui::RichText::new("Y").weak().size(11.0));
                                    let ty_resp = ui.add(
                                        egui::DragValue::new(&mut ty)
                                            .speed(speed)
                                            .max_decimals(decimals)
                                            .suffix(suffix.clone()),
                                    );
                                    if ty_resp.changed() {
                                        let ty = unit.to_px(ty);
                                        let sel = self.state.selected_ids.clone();
                                        for id in &sel {
                                            self.state.ensure_object_snapshot(id);
                                            for (_, o) in
                                                self.state.document.all_objects_mut()
                                            {
                                                if &o.id == id {
                                                    o.transform.y = ty;
                                                }
                                            }
                                        }
                                    }
                                    if ty_resp.drag_stopped() {
                                        self.state.commit_object_edits("Edit Transform");
                                    }

                                    // W/H are scale ratios, and a ratio is
                                    // unit-independent — only the bounds and
                                    // the number shown need converting.
                                    let mut bw = unit.from_px(obj_bw);
                                    ui.label(egui::RichText::new("W").weak().size(11.0));
                                    let bw_resp = ui.add(
                                        egui::DragValue::new(&mut bw)
                                            .speed(speed)
                                            .max_decimals(decimals)
                                            .range(unit.from_px(0.1)..=unit.from_px(99999.0)),
                                    );
                                    if bw_resp.changed() && obj_bw > 0.0 {
                                        let scale = unit.to_px(bw) / obj_bw;
                                        let sel = self.state.selected_ids.clone();
                                        for id in &sel {
                                            self.state.ensure_object_snapshot(id);
                                            for (_, o) in self.state.document.all_objects_mut() {
                                                if &o.id == id {
                                                    o.transform.scale_x *= scale;
                                                }
                                            }
                                        }
                                    }
                                    if bw_resp.drag_stopped() {
                                        self.state.commit_object_edits("Edit Transform");
                                    }

                                    let mut bh = unit.from_px(obj_bh);
                                    ui.label(egui::RichText::new("H").weak().size(11.0));
                                    let bh_resp = ui.add(
                                        egui::DragValue::new(&mut bh)
                                            .speed(speed)
                                            .max_decimals(decimals)
                                            .range(unit.from_px(0.1)..=unit.from_px(99999.0)),
                                    );
                                    if bh_resp.changed() && obj_bh > 0.0 {
                                        let scale = unit.to_px(bh) / obj_bh;
                                        let sel = self.state.selected_ids.clone();
                                        for id in &sel {
                                            self.state.ensure_object_snapshot(id);
                                            for (_, o) in self.state.document.all_objects_mut() {
                                                if &o.id == id {
                                                    o.transform.scale_y *= scale;
                                                }
                                            }
                                        }
                                    }
                                    if bh_resp.drag_stopped() {
                                        self.state.commit_object_edits("Edit Transform");
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
                        let su = self.state.prefs.stroke_unit;
                        if show_more {
                            ui.label(egui::RichText::new("線:").size(11.0));
                        }
                        let mut sw = su.from_px(self.state.stroke_width);
                        if ui
                            .add(
                                egui::DragValue::new(&mut sw)
                                    .speed(su.from_px(0.1))
                                    .range(0.0..=su.from_px(200.0))
                                    .suffix(su.suffix_label()),
                            )
                            .changed()
                        {
                            self.state.stroke_width = su.to_px(sw);
                        }

                        ui.separator();

                        // Variable Width Profile / Brush combos hide first
                        // when the row is tight.
                        if show_combos {
                            let profile_name = self
                                .state
                                .selected_ids
                                .first()
                                .and_then(|id| self.state.document.find_object(id))
                                .and_then(|o| o.width_profile.as_ref())
                                .map(profile_display_name)
                                .unwrap_or_else(|| "均等".to_string());
                            egui::ComboBox::from_id_salt("ctrl_profile")
                                .selected_text(&profile_name)
                                .width(65.0)
                                .show_ui(ui, |ui| {
                                    let selected = profile_name.clone();
                                    for name in
                                        ["均等", "線幅プロファイル 1", "線幅プロファイル 2"]
                                    {
                                        if ui.selectable_label(selected == name, name).clicked() {
                                            apply_width_profile_preset(&mut self.state, name);
                                        }
                                    }
                                });

                            let brush_label = match self.state.brush_kind_idx {
                                0 => format!("• {:.0} pt. 丸筆", self.state.brush_size),
                                1 => "アートブラシ".to_string(),
                                2 => "パターンブラシ".to_string(),
                                _ => "毛先ブラシ".to_string(),
                            };
                            egui::ComboBox::from_id_salt("ctrl_brush")
                                .selected_text(brush_label)
                                .width(90.0)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            self.state.brush_kind_idx == 0,
                                            "• 3 pt. 丸筆",
                                        )
                                        .clicked()
                                    {
                                        self.state.brush_kind_idx = 0;
                                        self.state.brush_roundness = 100.0;
                                        self.state.brush_size = 3.0;
                                    }
                                    if ui
                                        .selectable_label(
                                            self.state.brush_kind_idx == 0
                                                && self.state.brush_roundness < 80.0,
                                            "• 5 pt. 楕円筆",
                                        )
                                        .clicked()
                                    {
                                        self.state.brush_kind_idx = 0;
                                        self.state.brush_roundness = 50.0;
                                        self.state.brush_size = 5.0;
                                    }
                                    if ui
                                        .selectable_label(
                                            self.state.brush_kind_idx == 3,
                                            "木炭・鉛筆",
                                        )
                                        .clicked()
                                    {
                                        self.state.brush_kind_idx = 3;
                                        self.state.brush_bristles = 16.0;
                                        self.state.brush_scatter = 0.5;
                                    }
                                });

                            ui.separator();
                        }

                        // Opacity
                        let mut op = self.state.opacity;
                        if show_more {
                            ui.label(egui::RichText::new("不透明度:").size(11.0));
                        }
                        if ui
                            .add(
                                egui::Slider::new(&mut op, 0.0..=1.0)
                                    .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                            )
                            .changed()
                        {
                            self.state.opacity = op;
                        }

                        if show_style {
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
                    }

                    // Right side of Control Bar (Document Setup, Preferences)
                    ui.separator();
                    if ui
                        .small_button("ドキュメント設定")
                        .on_hover_text("アートボードおよびドキュメント寸法の変更")
                        .clicked()
                    {
                        self.active_tab = ActiveTab::Properties;
                    }
                    if ui
                        .small_button("環境設定")
                        .on_hover_text(format!("環境設定ダイアログを開く ({}+K)", mod_key()))
                        .clicked()
                    {
                        self.preferences_dialog.is_open = true;
                    }
                });
            });
    }
}

/// Human label for a variable-width stroke profile (control-bar combo).
fn profile_display_name(p: &crate::core::document::WidthProfile) -> String {
    let widths: Vec<f64> = p.points.iter().map(|w| w.width).collect();
    if widths.iter().all(|w| (w - 1.0).abs() < 1e-6) {
        "均等".to_string()
    } else if widths.first().copied().unwrap_or(1.0) < widths.last().copied().unwrap_or(1.0) {
        "線幅プロファイル 1".to_string()
    } else {
        "線幅プロファイル 2".to_string()
    }
}

/// Apply a named width-profile preset to every selected stroked object
/// as one undoable step.
fn apply_width_profile_preset(state: &mut crate::core::state::AppState, name: &str) {
    use crate::core::document::{WidthPoint, WidthProfile, WidthSide};
    let preset: WidthProfile = match name {
        "線幅プロファイル 1" => WidthProfile {
            points: vec![
                WidthPoint { position: 0.0, width: 0.3, side: WidthSide::Both },
                WidthPoint { position: 0.5, width: 1.4, side: WidthSide::Both },
                WidthPoint { position: 1.0, width: 0.6, side: WidthSide::Both },
            ],
        },
        "線幅プロファイル 2" => WidthProfile {
            points: vec![
                WidthPoint { position: 0.0, width: 1.5, side: WidthSide::Both },
                WidthPoint { position: 0.5, width: 0.5, side: WidthSide::Both },
                WidthPoint { position: 1.0, width: 1.2, side: WidthSide::Both },
            ],
        },
        _ => WidthProfile::default(),
    };
    let ids = state.selected_ids.clone();
    if ids.is_empty() {
        return;
    }
    state.undoable_snapshot("Set Width Profile", &ids, |doc| {
        for id in &ids {
            if let Some(obj) = doc.find_object_mut(id) {
                obj.width_profile = Some(preset.clone());
            }
        }
    });
}
