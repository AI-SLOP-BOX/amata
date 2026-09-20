use super::icons::{icon_button, tool_icon_button};
use super::zoom_to_fit;
use super::IrasuApp;
use crate::core::state::{AppState, Tool};
use egui::CornerRadius;
use egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};

impl IrasuApp {
    pub(super) fn show_document_tab_bar(&mut self, ctx: &egui::Context) {
        // Document Tab Bar (Illustrator Signature Tab: "名称未設定-1* @ 100% (RGB/プレビュー) ×")
        egui::TopBottomPanel::top("document_tab_bar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
                    let doc_name = if self.state.document.name.is_empty() {
                        "名称未設定-1"
                    } else {
                        &self.state.document.name
                    };
                    let tab_title = format!(
                        "{}* @ {:.0}% (RGB/プレビュー)",
                        doc_name,
                        self.state.zoom * 100.0
                    );

                    // Active Tab Button with dark filled tab background
                    let tab_btn = egui::Button::new(
                        RichText::new(format!("  {}  ×  ", tab_title))
                            .size(11.0)
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(38, 38, 38))
                    .corner_radius(CornerRadius {
                        nw: 4,
                        ne: 4,
                        sw: 0,
                        se: 0,
                    });

                    ui.add(tab_btn);
                });
            });
    }

    pub(super) fn show_status_bar(&mut self, ctx: &egui::Context) {
        // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Zoom Presets Dropdown
                    egui::ComboBox::from_id_salt("zoom_select")
                        .selected_text(format!("{:.0}%", self.state.zoom * 100.0))
                        .width(60.0)
                        .show_ui(ui, |ui| {
                            let set_zoom = |state: &mut AppState, z: f32| {
                                state.start_zoom = state.zoom;
                                state.start_pan_x = state.pan_x;
                                state.start_pan_y = state.pan_y;
                                state.target_zoom = z;
                                state.target_pan_x = state.pan_x;
                                state.target_pan_y = state.pan_y;
                                state.zoom_animation_progress = 0.0;
                            };
                            if ui
                                .selectable_label(self.state.zoom == 0.25, "25%")
                                .clicked()
                            {
                                set_zoom(&mut self.state, 0.25);
                            }
                            if ui.selectable_label(self.state.zoom == 0.5, "50%").clicked() {
                                set_zoom(&mut self.state, 0.5);
                            }
                            if ui
                                .selectable_label(self.state.zoom == 1.0, "100%")
                                .clicked()
                            {
                                set_zoom(&mut self.state, 1.0);
                            }
                            if ui
                                .selectable_label(self.state.zoom == 2.0, "200%")
                                .clicked()
                            {
                                set_zoom(&mut self.state, 2.0);
                            }
                            if ui
                                .selectable_label(self.state.zoom == 4.0, "400%")
                                .clicked()
                            {
                                set_zoom(&mut self.state, 4.0);
                            }
                            if ui.button("画面に合わせる (Cmd+0)").clicked() {
                                self.state.start_zoom = self.state.zoom;
                                self.state.start_pan_x = self.state.pan_x;
                                self.state.start_pan_y = self.state.pan_y;
                                zoom_to_fit(&mut self.state);
                                self.state.zoom_animation_progress = 0.0;
                            }
                        });

                    // Canvas Rotation Angle (Image 1 & 3: "0° ∨")
                    egui::ComboBox::from_id_salt("canvas_rotation")
                        .selected_text("0°")
                        .width(42.0)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_label(true, "0°");
                            let _ = ui.selectable_label(false, "90°");
                            let _ = ui.selectable_label(false, "180°");
                            let _ = ui.selectable_label(false, "270°");
                        });

                    ui.separator();

                    // Artboard Navigator Pager
                    let ab_count = self.state.document.effective_artboards().len();
                    if ui.small_button("|◀").on_hover_text("最初のアートボード").clicked() {
                        self.state.active_artboard_idx = 0;
                    }
                    if ui.small_button("◀").on_hover_text("前のアートボード").clicked() {
                        self.state.active_artboard_idx = self.state.active_artboard_idx.saturating_sub(1);
                    }
                    ui.label(
                        RichText::new(format!(" {} ", self.state.active_artboard_idx + 1))
                            .size(11.0)
                            .monospace()
                            .color(Color32::WHITE),
                    );
                    if ui.small_button("▶").on_hover_text("次のアートボード").clicked() {
                        let next = self.state.active_artboard_idx + 1;
                        if next < ab_count {
                            self.state.active_artboard_idx = next;
                        }
                    }
                    if ui.small_button("▶|").on_hover_text("最後のアートボード").clicked()
                        && ab_count > 0 {
                            self.state.active_artboard_idx = ab_count - 1;
                        }

                    ui.separator();

                    // Status / Mode Label (Image 1 & 3: "選択")
                    ui.label(
                        RichText::new("選択")
                            .size(11.0)
                            .color(Color32::from_rgb(175, 175, 175)),
                    );

                    // Right-aligned coordinates and selection info
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Live cursor position
                        if let Some((cx, cy)) = self.state.cursor_world {
                            ui.label(
                                egui::RichText::new(format!("X: {:.1} pt   Y: {:.1} pt", cx, cy))
                                    .size(11.0)
                                    .monospace(),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("X: —   Y: —")
                                    .size(11.0)
                                    .weak()
                                    .monospace(),
                            );
                        }

                        ui.separator();

                        // Selection stats
                        let sel_count = self.state.selected_ids.len();
                        if sel_count == 0 {
                            ui.label(
                                egui::RichText::new("0 個のオブジェクトを選択")
                                    .weak()
                                    .size(11.0),
                            );
                        } else if sel_count == 1 {
                            let fid = &self.state.selected_ids[0];
                            if let Some((_, obj)) = self
                                .state
                                .document
                                .all_objects()
                                .find(|(_, o)| &o.id == fid)
                            {
                                if let Some((mn, mx)) = obj.bounding_box() {
                                    let w = mx.x - mn.x;
                                    let h = mx.y - mn.y;
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "1 個選択中 [幅: {:.1} pt  高さ: {:.1} pt]",
                                            w, h
                                        ))
                                        .size(11.0),
                                    );
                                } else {
                                    ui.label(egui::RichText::new("1 個選択中").size(11.0));
                                }
                            }
                        } else {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} 個のオブジェクトを選択中",
                                    sel_count
                                ))
                                .size(11.0),
                            );
                        }
                    });
                });
            });
    }

    pub(super) fn tool_button(&mut self, ui: &mut egui::Ui, tool: Tool) {
        let is_active = self.state.current_tool == tool;
        let response = tool_icon_button(ui, tool, is_active, Vec2::new(32.0, 30.0))
            .on_hover_text(format!("{} ({})", tool.name(), tool.shortcut()));

        if response.clicked() {
            if self.state.current_tool == Tool::Pen && self.canvas.pen_state.is_drawing {
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
            self.state.previous_tool = self.state.current_tool;
            self.state.current_tool = tool;
        }
    }

    pub(super) fn show_toolbar(&mut self, ctx: &egui::Context) {
        // Left Vertical Toolbar (Classic 2-Column Illustrator Grid)
        egui::SidePanel::left("toolbar")
            .resizable(false)
            .default_width(74.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);

                    let tool_pairs = [
                        (Tool::Select, Tool::Node),
                        (Tool::Pen, Tool::Pencil),
                        (Tool::Rectangle, Tool::Ellipse),
                        (Tool::Star, Tool::Polygon),
                        (Tool::Line, Tool::Text),
                        (Tool::Brush, Tool::Eraser),
                        (Tool::ShapeBuilder, Tool::Eyedropper),
                        (Tool::Hand, Tool::Zoom),
                    ];
                    let pixel_tools = [
                        Tool::PixelPencil,
                        Tool::PixelEraser,
                        Tool::PixelBucket,
                    ];

                    for (t1, t2) in tool_pairs {
                        ui.horizontal(|ui| {
                            for tool in [t1, t2] {
                                self.tool_button(ui, tool);
                            }
                        });
                    }

                    // Pixel-art (dot絵) tools get their own row so the vector
                    // pairs above stay untouched.
                    ui.horizontal(|ui| {
                        for tool in pixel_tools {
                            self.tool_button(ui, tool);
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Illustrator Signature Overlapping Fill & Stroke Swatches
                    let (swatch_rect, _) =
                        ui.allocate_exact_size(Vec2::new(48.0, 42.0), egui::Sense::hover());
                    let painter = ui.painter();

                    let fill_rgba = Color32::from_rgba_unmultiplied(
                        (self.state.fill_color[0] * 255.0) as u8,
                        (self.state.fill_color[1] * 255.0) as u8,
                        (self.state.fill_color[2] * 255.0) as u8,
                        (self.state.fill_color[3] * 255.0) as u8,
                    );
                    let stroke_rgba = Color32::from_rgba_unmultiplied(
                        (self.state.stroke_color[0] * 255.0) as u8,
                        (self.state.stroke_color[1] * 255.0) as u8,
                        (self.state.stroke_color[2] * 255.0) as u8,
                        (self.state.stroke_color[3] * 255.0) as u8,
                    );

                    // Stroke square (back, offset bottom-right)
                    let stroke_box = Rect::from_min_size(
                        Pos2::new(swatch_rect.min.x + 14.0, swatch_rect.min.y + 12.0),
                        Vec2::new(26.0, 26.0),
                    );
                    painter.rect_filled(stroke_box, 1.0, stroke_rgba);
                    painter.rect_stroke(
                        stroke_box,
                        1.0,
                        Stroke::new(1.0_f32, Color32::from_rgb(30, 30, 30)),
                        egui::StrokeKind::Outside,
                    );
                    // Hollow inner cutout for stroke box
                    let stroke_inner = stroke_box.shrink(5.0);
                    painter.rect_filled(stroke_inner, 0.0, Color32::from_rgb(45, 45, 45));

                    // Fill square (front, offset top-left)
                    let fill_box = Rect::from_min_size(
                        Pos2::new(swatch_rect.min.x + 4.0, swatch_rect.min.y + 2.0),
                        Vec2::new(26.0, 26.0),
                    );
                    painter.rect_filled(fill_box, 1.0, fill_rgba);
                    painter.rect_stroke(
                        fill_box,
                        1.0,
                        Stroke::new(1.0_f32, Color32::from_rgb(30, 30, 30)),
                        egui::StrokeKind::Outside,
                    );
                    if self.state.fill_color[3] <= 0.001 {
                        // Red slash for transparent fill
                        painter.line_segment(
                            [fill_box.left_top(), fill_box.right_bottom()],
                            Stroke::new(1.5_f32, Color32::from_rgb(220, 40, 40)),
                        );
                    }

                    ui.add_space(2.0);

                    // Color picker buttons
                    let mut fill_c = [
                        (self.state.fill_color[0] * 255.0) as u8,
                        (self.state.fill_color[1] * 255.0) as u8,
                        (self.state.fill_color[2] * 255.0) as u8,
                        (self.state.fill_color[3] * 255.0) as u8,
                    ];
                    let mut stroke_c = [
                        (self.state.stroke_color[0] * 255.0) as u8,
                        (self.state.stroke_color[1] * 255.0) as u8,
                        (self.state.stroke_color[2] * 255.0) as u8,
                        (self.state.stroke_color[3] * 255.0) as u8,
                    ];
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("F:").size(10.0).weak());
                        if ui
                            .color_edit_button_srgba_unmultiplied(&mut fill_c)
                            .changed()
                        {
                            self.state.fill_color = [
                                fill_c[0] as f32 / 255.0,
                                fill_c[1] as f32 / 255.0,
                                fill_c[2] as f32 / 255.0,
                                fill_c[3] as f32 / 255.0,
                            ];
                        }
                        ui.label(egui::RichText::new("S:").size(10.0).weak());
                        if ui
                            .color_edit_button_srgba_unmultiplied(&mut stroke_c)
                            .changed()
                        {
                            self.state.stroke_color = [
                                stroke_c[0] as f32 / 255.0,
                                stroke_c[1] as f32 / 255.0,
                                stroke_c[2] as f32 / 255.0,
                                stroke_c[3] as f32 / 255.0,
                            ];
                        }
                    });

                    // Swap / Default / None — painted as clean vector icons
                    ui.horizontal(|ui| {
                        // Swap arrows icon
                        let swap_resp = icon_button(ui, Vec2::splat(22.0), |p, r, col| {
                            let c = r.center();
                            let w = r.width() * 0.35;
                            let h = r.height() * 0.20;
                            // Top arrow →
                            p.line_segment(
                                [Pos2::new(c.x - w, c.y - h), Pos2::new(c.x + w, c.y - h)],
                                Stroke::new(1.3_f32, col),
                            );
                            p.line_segment(
                                [
                                    Pos2::new(c.x + w - 4.0, c.y - h - 3.0),
                                    Pos2::new(c.x + w, c.y - h),
                                ],
                                Stroke::new(1.3_f32, col),
                            );
                            p.line_segment(
                                [
                                    Pos2::new(c.x + w - 4.0, c.y - h + 3.0),
                                    Pos2::new(c.x + w, c.y - h),
                                ],
                                Stroke::new(1.3_f32, col),
                            );
                            // Bottom arrow ←
                            p.line_segment(
                                [Pos2::new(c.x + w, c.y + h), Pos2::new(c.x - w, c.y + h)],
                                Stroke::new(1.3_f32, col),
                            );
                            p.line_segment(
                                [
                                    Pos2::new(c.x - w + 4.0, c.y + h - 3.0),
                                    Pos2::new(c.x - w, c.y + h),
                                ],
                                Stroke::new(1.3_f32, col),
                            );
                            p.line_segment(
                                [
                                    Pos2::new(c.x - w + 4.0, c.y + h + 3.0),
                                    Pos2::new(c.x - w, c.y + h),
                                ],
                                Stroke::new(1.3_f32, col),
                            );
                        })
                        .on_hover_text("Swap Fill and Stroke (Shift+X)");
                        if swap_resp.clicked() {
                            std::mem::swap(
                                &mut self.state.fill_color,
                                &mut self.state.stroke_color,
                            );
                        }

                        // Default colors icon (white square outline)
                        let default_resp = icon_button(ui, Vec2::splat(22.0), |p, r, col| {
                            let sq = r.shrink(r.width() * 0.22);
                            p.rect_stroke(
                                sq,
                                1.0,
                                Stroke::new(1.4_f32, col),
                                egui::StrokeKind::Middle,
                            );
                            // Small "D" text hint
                            p.text(
                                r.center() + Vec2::new(0.0, 0.5),
                                egui::Align2::CENTER_CENTER,
                                "D",
                                egui::FontId::proportional(8.0),
                                col,
                            );
                        })
                        .on_hover_text("Default Colors: White Fill, Black Stroke (D)");
                        if default_resp.clicked() {
                            self.state.fill_color = [1.0, 1.0, 1.0, 1.0];
                            self.state.stroke_color = [0.0, 0.0, 0.0, 1.0];
                            self.state.stroke_width = 1.0;
                        }

                        // None/transparent icon (circle with slash)
                        let none_resp = icon_button(ui, Vec2::splat(22.0), |p, r, col| {
                            let c = r.center();
                            let rad = r.size().min_elem() * 0.32;
                            p.circle_stroke(c, rad, Stroke::new(1.4_f32, col));
                            let diag = rad * 0.72;
                            p.line_segment(
                                [
                                    Pos2::new(c.x - diag, c.y + diag),
                                    Pos2::new(c.x + diag, c.y - diag),
                                ],
                                Stroke::new(1.4_f32, Color32::from_rgb(200, 50, 50)),
                            );
                        })
                        .on_hover_text("None / Transparent (/)");
                        if none_resp.clicked() {
                            self.state.fill_color = [0.0, 0.0, 0.0, 0.0];
                        }
                    });

                    ui.add_space(6.0);
                    // Customize Toolbar Button (···)
                    let more_btn = egui::Button::new(
                        RichText::new("···")
                            .size(14.0)
                            .strong()
                            .color(Color32::from_rgb(170, 170, 170)),
                    )
                    .fill(Color32::TRANSPARENT)
                    .min_size(Vec2::new(32.0, 22.0));
                    let _ = ui.add(more_btn).on_hover_text("ツールバーをカスタマイズ");
                });
            });
    }
}
