use crate::core::state::AppState;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};

pub struct ExportModal {
    pub is_open: bool,
    pub active_sidebar_tab: ExportSidebarTab,
    pub active_format: ExportFormatTab,
    pub file_name: String,
    pub convert_to_outlines: bool,
    pub embed_images: bool,
    pub preserve_editability: bool,
    pub export_scope: String,
    pub custom_range: String,
    pub scale_factor: String,
    pub color_profile: String,
    pub embed_color_profile: bool,
    pub assets: Vec<ExportAssetItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportSidebarTab {
    Export,
    ExportForWeb,
    ExportAssets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormatTab {
    Svg,
    Pdf,
    Png,
    Jpeg,
    Webp,
    Avif,
}

pub struct ExportAssetItem {
    pub selected: bool,
    pub name: String,
    pub artboard_num: usize,
    pub format: ExportFormatTab,
    pub size: String,
    pub scale: String,
    pub file_name: String,
    pub color: Color32,
    pub accent: Color32,
}

impl Default for ExportModal {
    fn default() -> Self {
        Self {
            is_open: false,
            active_sidebar_tab: ExportSidebarTab::Export,
            active_format: ExportFormatTab::Svg,
            file_name: "名称未設定-1".to_string(),
            convert_to_outlines: true,
            embed_images: true,
            preserve_editability: false,
            export_scope: "すべてのアートボード".to_string(),
            custom_range: "1".to_string(),
            scale_factor: "1x".to_string(),
            color_profile: "sRGB IEC61966-2.1".to_string(),
            embed_color_profile: false,
            assets: vec![
                ExportAssetItem {
                    selected: true,
                    name: "ロゴ".into(),
                    artboard_num: 1,
                    format: ExportFormatTab::Svg,
                    size: "512 × 512".into(),
                    scale: "1x".into(),
                    file_name: "logo.svg".into(),
                    color: Color32::from_rgb(20, 115, 230),
                    accent: Color32::from_rgb(20, 200, 180),
                },
                ExportAssetItem {
                    selected: true,
                    name: "アイコン".into(),
                    artboard_num: 2,
                    format: ExportFormatTab::Png,
                    size: "256 × 256".into(),
                    scale: "2x".into(),
                    file_name: "icon.png".into(),
                    color: Color32::from_rgb(30, 130, 240),
                    accent: Color32::from_rgb(80, 180, 255),
                },
                ExportAssetItem {
                    selected: true,
                    name: "バナー".into(),
                    artboard_num: 1,
                    format: ExportFormatTab::Jpeg,
                    size: "1920 × 1080".into(),
                    scale: "1x".into(),
                    file_name: "banner.jpg".into(),
                    color: Color32::from_rgb(40, 80, 160),
                    accent: Color32::from_rgb(20, 180, 200),
                },
            ],
        }
    }
}

impl ExportModal {
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if !self.is_open {
            return;
        }

        let mut is_open = self.is_open;
        egui::Window::new("書き出し")
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(1000.0, 640.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // 1. Left Sidebar (160px): Export modes & presets
                    self.show_left_sidebar(ui);

                    ui.separator();

                    // 2. Central Area (560px): Live Preview & Assets Table
                    self.show_center_preview_and_table(ui, state);

                    ui.separator();

                    // 3. Right Settings Pane (250px): Format specifics, scope, color profiles
                    self.show_right_settings(ui, state);
                });
            });
        self.is_open = is_open;
    }

    fn show_left_sidebar(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.set_width(150.0);
            ui.add_space(4.0);
            ui.label(
                RichText::new("← 書き出し")
                    .strong()
                    .size(13.5)
                    .color(Color32::WHITE),
            );
            ui.add_space(10.0);

            let main_tabs = [
                (ExportSidebarTab::Export, "書き出し"),
                (ExportSidebarTab::ExportForWeb, "Web 用に書き出し"),
                (ExportSidebarTab::ExportAssets, "アセットを書き出し"),
            ];
            for (tab, label) in main_tabs {
                let is_sel = self.active_sidebar_tab == tab;
                let btn = if is_sel {
                    egui::Button::new(
                        RichText::new(label)
                            .strong()
                            .size(11.5)
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(38, 52, 75))
                    .min_size(Vec2::new(145.0, 26.0))
                } else {
                    egui::Button::new(
                        RichText::new(label)
                            .size(11.5)
                            .color(Color32::from_rgb(180, 180, 180)),
                    )
                    .fill(Color32::TRANSPARENT)
                    .min_size(Vec2::new(145.0, 26.0))
                };
                if ui.add(btn).clicked() {
                    self.active_sidebar_tab = tab;
                }
            }

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("書き出しプリセット")
                        .size(10.5)
                        .color(Color32::from_rgb(150, 150, 150)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("+")
                            .size(11.0)
                            .color(Color32::from_rgb(180, 180, 180)),
                    );
                });
            });
            ui.add_space(6.0);

            let presets = [
                ("Web (SVG)", ExportFormatTab::Svg),
                ("印刷用 (PDF)", ExportFormatTab::Pdf),
                ("スクリーン用 (PNG)", ExportFormatTab::Png),
                ("ソーシャル投稿 (JPEG)", ExportFormatTab::Jpeg),
                ("Web 最適化 (WebP)", ExportFormatTab::Webp),
                ("次世代 Web (AVIF)", ExportFormatTab::Avif),
            ];
            for (p_name, fmt) in presets {
                let is_active = self.active_format == fmt;
                let btn =
                    egui::Button::new(RichText::new(format!("  {}", p_name)).size(10.5).color(
                        if is_active {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(160, 160, 160)
                        },
                    ))
                    .fill(if is_active {
                        Color32::from_rgb(45, 45, 48)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .min_size(Vec2::new(145.0, 22.0));
                if ui.add(btn).clicked() {
                    self.active_format = fmt;
                }
            }
        });
    }

    fn show_center_preview_and_table(&mut self, ui: &mut Ui, _state: &AppState) {
        ui.vertical(|ui| {
            ui.set_width(540.0);

            // Preview Toolbar
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("プレビュー")
                        .strong()
                        .size(12.0)
                        .color(Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("🔍 ➕ ➖ ⛶");
                    ui.label("100%");
                    ui.label("アートボード 1  ▾");
                });
            });

            ui.add_space(4.0);

            // Preview Canvas (White paper display like Image 5)
            let (p_rect, _) = ui.allocate_exact_size(Vec2::new(540.0, 280.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(p_rect, 4.0, Color32::from_rgb(28, 28, 30));

            // Artboard White Paper Area
            let paper_rect = Rect::from_center_size(p_rect.center(), Vec2::new(360.0, 230.0));
            ui.painter().rect_filled(paper_rect, 2.0, Color32::WHITE);
            ui.painter().rect_stroke(
                paper_rect,
                2.0,
                Stroke::new(1.0_f32, Color32::from_rgb(200, 200, 200)),
                StrokeKind::Outside,
            );

            // Two vector circles branding demo inside preview paper
            let c1 = Pos2::new(paper_rect.center().x - 30.0, paper_rect.center().y - 20.0);
            let c2 = Pos2::new(paper_rect.center().x + 20.0, paper_rect.center().y - 10.0);
            ui.painter()
                .circle_filled(c1, 45.0, Color32::from_rgb(20, 115, 230));
            ui.painter().circle_filled(
                c2,
                35.0,
                Color32::from_rgba_unmultiplied(20, 180, 180, 220),
            );

            // Brand text under circles
            ui.painter().text(
                Pos2::new(paper_rect.center().x, paper_rect.center().y + 50.0),
                egui::Align2::CENTER_CENTER,
                "B R A N D",
                egui::FontId::proportional(15.0),
                Color32::from_rgb(20, 20, 20),
            );
            ui.painter().text(
                Pos2::new(paper_rect.center().x, paper_rect.center().y + 68.0),
                egui::Align2::CENTER_CENTER,
                "SIMPLE IDEAS FOR A BRIGHTER TOMORROW",
                egui::FontId::proportional(7.5),
                Color32::from_rgb(120, 120, 120),
            );

            ui.add_space(8.0);

            // Export Assets Table Header
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("書き出すアートボード・アセット")
                        .strong()
                        .size(11.5)
                        .color(Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("＋ 追加  − 削除  ⛶ すべて選択")
                            .size(10.0)
                            .color(Color32::from_rgb(160, 160, 160)),
                    );
                });
            });

            ui.add_space(4.0);

            // Table of assets
            let (table_rect, _) =
                ui.allocate_exact_size(Vec2::new(540.0, 150.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(table_rect, 4.0, Color32::from_rgb(33, 33, 35));

            let mut ty = table_rect.min.y + 6.0;
            for item in &mut self.assets {
                let row_rect = Rect::from_min_max(
                    Pos2::new(table_rect.min.x + 8.0, ty),
                    Pos2::new(table_rect.max.x - 8.0, ty + 38.0),
                );
                ui.painter()
                    .rect_filled(row_rect, 3.0, Color32::from_rgb(40, 40, 44));

                // Checkbox
                let cb_rect = Rect::from_min_size(
                    Pos2::new(row_rect.min.x + 8.0, row_rect.center().y - 7.0),
                    Vec2::splat(14.0),
                );
                ui.painter().rect_filled(
                    cb_rect,
                    2.0,
                    if item.selected {
                        Color32::from_rgb(20, 115, 230)
                    } else {
                        Color32::from_rgb(60, 60, 64)
                    },
                );
                if item.selected {
                    ui.painter().text(
                        cb_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "✓",
                        egui::FontId::proportional(10.0),
                        Color32::WHITE,
                    );
                }

                // Title
                ui.painter().text(
                    Pos2::new(row_rect.min.x + 32.0, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &item.name,
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                );

                // Thumbnail pill
                let thumb_r = Rect::from_center_size(
                    Pos2::new(row_rect.min.x + 105.0, row_rect.center().y),
                    Vec2::new(42.0, 22.0),
                );
                ui.painter().rect_filled(thumb_r, 2.0, Color32::WHITE);
                ui.painter().circle_filled(
                    Pos2::new(thumb_r.center().x - 6.0, thumb_r.center().y),
                    6.0,
                    item.color,
                );
                ui.painter().circle_filled(
                    Pos2::new(thumb_r.center().x + 5.0, thumb_r.center().y),
                    5.0,
                    item.accent,
                );

                // Artboard num
                ui.painter().text(
                    Pos2::new(row_rect.min.x + 160.0, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{}", item.artboard_num),
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(180, 180, 180),
                );

                // Format pill
                let fmt_name = match item.format {
                    ExportFormatTab::Svg => "SVG",
                    ExportFormatTab::Pdf => "PDF",
                    ExportFormatTab::Png => "PNG",
                    ExportFormatTab::Jpeg => "JPEG",
                    ExportFormatTab::Webp => "WebP",
                    ExportFormatTab::Avif => "AVIF",
                };
                ui.painter().text(
                    Pos2::new(row_rect.min.x + 205.0, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    fmt_name,
                    egui::FontId::proportional(10.5),
                    Color32::WHITE,
                );

                // Size
                ui.painter().text(
                    Pos2::new(row_rect.min.x + 265.0, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &item.size,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(160, 160, 160),
                );

                // Scale
                ui.painter().text(
                    Pos2::new(row_rect.min.x + 350.0, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &item.scale,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(180, 180, 180),
                );

                // File Name
                ui.painter().text(
                    Pos2::new(row_rect.min.x + 400.0, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &item.file_name,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(200, 200, 200),
                );

                ty += 44.0;
            }
        });
    }

    fn show_right_settings(&mut self, ui: &mut Ui, state: &mut AppState) {
        ui.vertical(|ui| {
            ui.set_width(240.0);
            ui.add_space(4.0);
            ui.label(
                RichText::new("書き出し設定")
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.add_space(6.0);

            // Format Toggle Pill Bar: [SVG] [PDF] [PNG] [JPEG] [WebP] [AVIF]
            ui.horizontal_wrapped(|ui| {
                let fmts = [
                    (ExportFormatTab::Svg, "SVG"),
                    (ExportFormatTab::Pdf, "PDF"),
                    (ExportFormatTab::Png, "PNG"),
                    (ExportFormatTab::Jpeg, "JPEG"),
                    (ExportFormatTab::Webp, "WebP"),
                    (ExportFormatTab::Avif, "AVIF"),
                ];
                for (f, name) in fmts {
                    let is_active = self.active_format == f;
                    let btn = if is_active {
                        egui::Button::new(
                            RichText::new(name)
                                .strong()
                                .size(10.5)
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(20, 115, 230))
                        .min_size(Vec2::new(48.0, 24.0))
                    } else {
                        egui::Button::new(
                            RichText::new(name)
                                .size(10.5)
                                .color(Color32::from_rgb(180, 180, 180)),
                        )
                        .fill(Color32::from_rgb(44, 44, 48))
                        .min_size(Vec2::new(48.0, 24.0))
                    };
                    if ui.add(btn).clicked() {
                        self.active_format = f;
                    }
                }
            });

            ui.add_space(8.0);
            ui.checkbox(
                &mut self.convert_to_outlines,
                "フォントをアウトラインに変換",
            );
            ui.checkbox(&mut self.embed_images, "画像を埋め込み");
            ui.checkbox(
                &mut self.preserve_editability,
                "編集用のSVGを保持 (拡張メタデータ)",
            );

            ui.add_space(6.0);

            // Raster Resolution (DPI / PPI) options for PNG / JPEG / PDF
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("解像度:")
                        .size(10.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                egui::ComboBox::from_id_salt("exp_modal_ppi")
                    .selected_text(match (state.export_scale * 72.0).round() as i32 {
                        300 => "高 (300 ppi / 印刷)",
                        150 => "中 (150 ppi)",
                        _ => "スクリーン (72 ppi / 1x)",
                    })
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(state.export_scale == 1.0, "スクリーン (72 ppi / 1x)")
                            .clicked()
                        {
                            state.export_scale = 1.0;
                        }
                        if ui
                            .selectable_label(state.export_scale == 2.0833333, "中 (150 ppi)")
                            .clicked()
                        {
                            state.export_scale = 150.0 / 72.0;
                        }
                        if ui
                            .selectable_label(
                                state.export_scale == 4.1666665,
                                "高 (300 ppi / 印刷)",
                            )
                            .clicked()
                        {
                            state.export_scale = 300.0 / 72.0;
                        }
                        if ui
                            .selectable_label(state.export_scale == 2.0, "Retina (2x)")
                            .clicked()
                        {
                            state.export_scale = 2.0;
                        }
                    });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            // Export Scope
            ui.label(
                RichText::new("書き出し範囲")
                    .strong()
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 180)),
            );
            ui.radio_value(
                &mut self.export_scope,
                "すべてのアートボード".to_string(),
                "すべてのアートボード",
            );
            ui.radio_value(
                &mut self.export_scope,
                "選択したアートボード".to_string(),
                "選択したアートボード",
            );
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.export_scope, "範囲指定".to_string(), "範囲指定:");
                ui.add(egui::TextEdit::singleline(&mut self.custom_range).desired_width(70.0));
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Color profile
            ui.label(
                RichText::new("カラープロファイル")
                    .strong()
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 180)),
            );
            egui::ComboBox::from_id_salt("exp_color_profile")
                .selected_text(&self.color_profile)
                .width(220.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.color_profile,
                        "sRGB IEC61966-2.1".into(),
                        "sRGB IEC61966-2.1",
                    );
                    ui.selectable_value(
                        &mut self.color_profile,
                        "Adobe RGB (1998)".into(),
                        "Adobe RGB (1998)",
                    );
                    ui.selectable_value(&mut self.color_profile, "Display P3".into(), "Display P3");
                });
            ui.checkbox(
                &mut self.embed_color_profile,
                "カラープロファイルを埋め込む",
            );

            ui.add_space(14.0);

            // Action Buttons
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("キャンセル").min_size(Vec2::new(85.0, 28.0)))
                    .clicked()
                {
                    self.is_open = false;
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new("書き出し").strong().color(Color32::WHITE))
                            .fill(Color32::from_rgb(20, 115, 230))
                            .min_size(Vec2::new(115.0, 28.0)),
                    )
                    .clicked()
                {
                    match self.active_format {
                        ExportFormatTab::Svg => {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("SVG", &["svg"])
                                .set_file_name(format!("{}.svg", self.file_name))
                                .save_file()
                            {
                                let svg = crate::io::svg::export_svg(&state.document);
                                match crate::io::atomic::atomic_write_str(&path, &svg) {
                                    Ok(_) => state.notify_info(format!(
                                        "SVGを書き出しました: {}",
                                        path.file_name().and_then(|n| n.to_str()).unwrap_or("file")
                                    )),
                                    Err(e) => state
                                        .notify_error(format!("SVG書き出しに失敗しました: {e}")),
                                }
                            }
                        }
                        ExportFormatTab::Pdf => {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PDF", &["pdf"])
                                .set_file_name(format!("{}.pdf", self.file_name))
                                .save_file()
                            {
                                let pdf_bytes = crate::io::pdf::export_pdf(&state.document);
                                match crate::io::atomic::atomic_write_bytes(&path, &pdf_bytes) {
                                    Ok(_) => state.notify_info(format!(
                                        "PDFを書き出しました: {}",
                                        path.file_name().and_then(|n| n.to_str()).unwrap_or("file")
                                    )),
                                    Err(e) => state
                                        .notify_error(format!("PDF書き出しに失敗しました: {e}")),
                                }
                            }
                        }
                        ExportFormatTab::Png => {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PNG", &["png"])
                                .set_file_name(format!("{}.png", self.file_name))
                                .save_file()
                            {
                                let scale = match self.scale_factor.as_str() {
                                    "2x" => 2.0_f32,
                                    "3x" => 3.0_f32,
                                    "4x" => 4.0_f32,
                                    "0.5x" => 0.5_f32,
                                    _ => 1.0_f32,
                                };
                                match crate::io::raster::export_png(&state.document, scale, true) {
                                    Ok(png_bytes) => {
                                        match crate::io::atomic::atomic_write_bytes(
                                            &path,
                                            &png_bytes,
                                        ) {
                                            Ok(_) => state.notify_info(format!(
                                                "PNGを書き出しました ({}): {}",
                                                self.scale_factor,
                                                path.file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("file")
                                            )),
                                            Err(e) => {
                                                state.notify_error(format!("保存に失敗しました: {e}"))
                                            }
                                        }
                                    }
                                    Err(e) => state.notify_error(format!(
                                        "PNGラスタライズに失敗しました: {e}"
                                    )),
                                }
                            }
                        }
                        ExportFormatTab::Jpeg => {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("JPEG", &["jpg", "jpeg"])
                                .set_file_name(format!("{}.jpg", self.file_name))
                                .save_file()
                            {
                                let scale = match self.scale_factor.as_str() {
                                    "2x" => 2.0_f32,
                                    "3x" => 3.0_f32,
                                    "4x" => 4.0_f32,
                                    "0.5x" => 0.5_f32,
                                    _ => 1.0_f32,
                                };
                                match crate::io::raster::export_jpeg(&state.document, scale) {
                                    Ok(jpeg_bytes) => {
                                        match crate::io::atomic::atomic_write_bytes(
                                            &path,
                                            &jpeg_bytes,
                                        ) {
                                            Ok(_) => state.notify_info(format!(
                                                "JPEGを書き出しました ({}): {}",
                                                self.scale_factor,
                                                path.file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("file")
                                            )),
                                            Err(e) => {
                                                state.notify_error(format!("保存に失敗しました: {e}"))
                                            }
                                        }
                                    }
                                    Err(e) => state.notify_error(format!(
                                        "JPEGラスタライズに失敗しました: {e}"
                                    )),
                                }
                            }
                        }
                        ExportFormatTab::Webp => {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("WebP", &["webp"])
                                .set_file_name(format!("{}.webp", self.file_name))
                                .save_file()
                            {
                                let scale = match self.scale_factor.as_str() {
                                    "2x" => 2.0_f32,
                                    "3x" => 3.0_f32,
                                    "4x" => 4.0_f32,
                                    "0.5x" => 0.5_f32,
                                    _ => 1.0_f32,
                                };
                                match crate::io::raster::export_webp(
                                    &state.document,
                                    scale,
                                    true,
                                ) {
                                    Ok(webp_bytes) => {
                                        match crate::io::atomic::atomic_write_bytes(
                                            &path,
                                            &webp_bytes,
                                        ) {
                                            Ok(_) => state.notify_info(format!(
                                                "WebPを書き出しました ({}): {}",
                                                self.scale_factor,
                                                path.file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("file")
                                            )),
                                            Err(e) => {
                                                state.notify_error(format!("保存に失敗しました: {e}"))
                                            }
                                        }
                                    }
                                    Err(e) => state.notify_error(format!(
                                        "WebPラスタライズに失敗しました: {e}"
                                    )),
                                }
                            }
                        }
                        ExportFormatTab::Avif => {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("AVIF", &["avif"])
                                .set_file_name(format!("{}.avif", self.file_name))
                                .save_file()
                            {
                                let scale = match self.scale_factor.as_str() {
                                    "2x" => 2.0_f32,
                                    "3x" => 3.0_f32,
                                    "4x" => 4.0_f32,
                                    "0.5x" => 0.5_f32,
                                    _ => 1.0_f32,
                                };
                                match crate::io::raster::export_avif(
                                    &state.document,
                                    scale,
                                    true,
                                ) {
                                    Ok(avif_bytes) => {
                                        match crate::io::atomic::atomic_write_bytes(
                                            &path,
                                            &avif_bytes,
                                        ) {
                                            Ok(_) => state.notify_info(format!(
                                                "AVIFを書き出しました ({}): {}",
                                                self.scale_factor,
                                                path.file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("file")
                                            )),
                                            Err(e) => {
                                                state.notify_error(format!("保存に失敗しました: {e}"))
                                            }
                                        }
                                    }
                                    Err(e) => state.notify_error(format!(
                                        "AVIFラスタライズに失敗しました: {e}"
                                    )),
                                }
                            }
                        }
                    }
                    self.is_open = false;
                }
            });
        });
    }
}
