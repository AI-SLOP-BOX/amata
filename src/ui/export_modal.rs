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
    pub export_scope: ExportScope,
    pub custom_range: String,
    pub color_profile: String,
    pub embed_color_profile: bool,
    pub assets: Vec<ExportAssetItem>,
}

/// Locale-independent export range. The old `String` held Japanese
/// literals (`"すべてのアートボード"` …) compared verbatim, so any
/// catalog change silently broke filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportScope {
    #[default]
    All,
    Selected,
    Range,
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
    Ai,
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
            export_scope: ExportScope::All,
            custom_range: "1".to_string(),
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
        let locale = state.prefs.language.clone();

        let mut is_open = self.is_open;
        let mut close_clicked = false;
        let mut export_clicked = false;
        let screen = ctx.screen_rect();
        let (size, min_size, pos) =
            crate::ui::window_defaults(screen, Vec2::new(720.0, 640.0), Vec2::new(320.0, 300.0));
        let total_h_outer = size.y;
        egui::Window::new(crate::ui::i18n::text(&locale, "export.title"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_pos(pos)
            .default_size(size)
            .min_size(min_size)
            .show(ctx, |ui| {
                let total_w = ui.available_width();
                crate::ui::modal_body(ui, "exp_body", &mut |ui, is_body| {
                    if !is_body {
                        let btn_h = 28.0;
                        let gap = 8.0;
                        let avail = ui.available_width();
                        let cancel_w = ((avail - gap) * 0.35).max(80.0);
                        let export_w = (avail - gap - cancel_w).max(90.0);
                        if ui
                            .add(
                                egui::Button::new(crate::ui::i18n::text(&locale, "export.cancel"))
                                    .min_size(Vec2::new(cancel_w, btn_h)),
                            )
                            .clicked()
                        {
                            close_clicked = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(crate::ui::i18n::text(&locale, "export.submit"))
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(20, 115, 230))
                                .min_size(Vec2::new(export_w, btn_h)),
                            )
                            .clicked()
                        {
                            export_clicked = true;
                        }
                        return;
                    }
                    ui.set_width(total_w);
                    self.show_left_sidebar(ui, total_w, &locale);
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    self.show_center_preview_and_table(ui, state, total_w, total_h_outer, &locale);
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    self.show_right_settings(ui, state, total_w, &locale);
                });
            });
        if export_clicked {
            if self.run_export(state) {
                self.is_open = false;
            }
        } else if close_clicked {
            self.is_open = false;
        } else {
            self.is_open = is_open;
        }
    }

    fn show_left_sidebar(&mut self, ui: &mut Ui, w: f32, locale: &str) {
        ui.set_width(w);
        ui.add_space(4.0);
        ui.label(
            RichText::new(crate::ui::i18n::text(locale, "export.title"))
                .strong()
                .size(13.5)
                .color(Color32::WHITE),
        );
        ui.add_space(8.0);

        // Mode tabs: always wrapped across the full width (no sidebar).
        ui.horizontal_wrapped(|ui| {
            let main_tabs = [
                (
                    ExportSidebarTab::Export,
                    crate::ui::i18n::text(locale, "export.title"),
                ),
                (
                    ExportSidebarTab::ExportForWeb,
                    crate::ui::i18n::text(locale, "export.tab.web"),
                ),
                (
                    ExportSidebarTab::ExportAssets,
                    crate::ui::i18n::text(locale, "export.tab.assets"),
                ),
            ];
            for (tab, label) in main_tabs {
                let is_sel = self.active_sidebar_tab == tab;
                if ui
                    .selectable_label(
                        is_sel,
                        RichText::new(label).size(11.5).color(if is_sel {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(180, 180, 180)
                        }),
                    )
                    .clicked()
                {
                    self.active_sidebar_tab = tab;
                }
            }
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui::i18n::text(locale, "export.presets"))
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
        ui.add_space(4.0);

        let presets = [
            ("Web (SVG)", ExportFormatTab::Svg),
            ("export.preset.pdf", ExportFormatTab::Pdf),
            ("export.preset.ai", ExportFormatTab::Ai),
            ("export.preset.png", ExportFormatTab::Png),
            ("export.preset.jpeg", ExportFormatTab::Jpeg),
            ("export.preset.webp", ExportFormatTab::Webp),
            ("export.preset.avif", ExportFormatTab::Avif),
        ];
        ui.horizontal_wrapped(|ui| {
            for (p_name, fmt) in presets {
                let is_active = self.active_format == fmt;
                if ui
                    .selectable_label(
                        is_active,
                        RichText::new(crate::ui::i18n::text(locale, p_name))
                            .size(10.5)
                            .color(if is_active {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(160, 160, 160)
                            }),
                    )
                    .clicked()
                {
                    self.active_format = fmt;
                }
            }
        });
    }

    fn show_center_preview_and_table(
        &mut self,
        ui: &mut Ui,
        _state: &AppState,
        center_w: f32,
        avail_h: f32,
        locale: &str,
    ) {
        ui.set_width(center_w);

        // Preview Toolbar
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui::i18n::text(locale, "export.preview"))
                    .strong()
                    .size(12.0)
                    .color(Color32::WHITE),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label("🔍 ➕ ➖ ⛶");
                ui.label("100%");
                ui.label(crate::ui::i18n::text(locale, "export.artboard_one"));
            });
        });

        ui.add_space(4.0);

        // Preview shrinks with window height so the table below stays reachable.
        let preview_h = (avail_h * 0.35).clamp(120.0, 260.0).min(center_w * 0.7);
        let (p_rect, _) =
            ui.allocate_exact_size(Vec2::new(center_w, preview_h), egui::Sense::hover());
        ui.painter()
            .rect_filled(p_rect, 4.0, Color32::from_rgb(28, 28, 30));

        // Artboard White Paper Area (proportional to preview rect)
        let paper_size = Vec2::new(p_rect.width() * 0.67, p_rect.height() * 0.82);
        let paper_rect = Rect::from_center_size(p_rect.center(), paper_size);
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
        ui.painter()
            .circle_filled(c2, 35.0, Color32::from_rgba_unmultiplied(20, 180, 180, 220));

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
                RichText::new(crate::ui::i18n::text(locale, "export.assets_table"))
                    .strong()
                    .size(11.5)
                    .color(Color32::WHITE),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(crate::ui::i18n::text(locale, "export.table_actions"))
                        .size(10.0)
                        .color(Color32::from_rgb(160, 160, 160)),
                );
            });
        });

        ui.add_space(4.0);

        // Table height follows row count (min one row) so nothing is clipped.
        let table_h = 12.0 + (self.assets.len().max(1) as f32) * 44.0;
        let (table_rect, _) =
            ui.allocate_exact_size(Vec2::new(center_w, table_h), egui::Sense::hover());
        ui.painter()
            .rect_filled(table_rect, 4.0, Color32::from_rgb(33, 33, 35));

        // Column x-offsets as fractions of row width so they track resizing.
        // On narrow centers, hide size/scale/filename to keep name+format readable.
        let row_inner_w = table_rect.width() - 16.0;
        let col = |frac: f32| table_rect.min.x + 8.0 + row_inner_w * frac;
        let show_detail = center_w >= 480.0;
        let show_meta = center_w >= 380.0;

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
                Pos2::new(col(0.045), row_rect.center().y),
                egui::Align2::LEFT_CENTER,
                &item.name,
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );

            // Thumbnail pill (hide on narrow centers)
            if show_meta {
                let thumb_r = Rect::from_center_size(
                    Pos2::new(col(0.185), row_rect.center().y),
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
            }

            // Artboard num
            if show_meta {
                ui.painter().text(
                    Pos2::new(col(0.29), row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{}", item.artboard_num),
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(180, 180, 180),
                );
            }

            // Format pill
            let fmt_name = match item.format {
                ExportFormatTab::Svg => "SVG",
                ExportFormatTab::Pdf => "PDF",
                ExportFormatTab::Ai => "AI",
                ExportFormatTab::Png => "PNG",
                ExportFormatTab::Jpeg => "JPEG",
                ExportFormatTab::Webp => "WebP",
                ExportFormatTab::Avif => "AVIF",
            };
            ui.painter().text(
                Pos2::new(
                    if show_meta { col(0.375) } else { col(0.22) },
                    row_rect.center().y,
                ),
                egui::Align2::LEFT_CENTER,
                fmt_name,
                egui::FontId::proportional(10.5),
                Color32::WHITE,
            );

            // Size
            if show_detail {
                ui.painter().text(
                    Pos2::new(col(0.49), row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &item.size,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(160, 160, 160),
                );
            }

            // Scale
            if show_detail {
                ui.painter().text(
                    Pos2::new(col(0.65), row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &item.scale,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(180, 180, 180),
                );
            }

            // File Name
            if show_detail {
                ui.painter().text(
                    Pos2::new(col(0.75), row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &item.file_name,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(200, 200, 200),
                );
            }

            ty += 44.0;
        }
    }

    fn show_right_settings(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        right_w: f32,
        locale: &str,
    ) {
        ui.set_width(right_w);
        ui.add_space(4.0);
        ui.label(
            RichText::new(crate::ui::i18n::text(locale, "export.settings"))
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
                (ExportFormatTab::Ai, "AI"),
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
            crate::ui::i18n::text(locale, "export.outline_fonts"),
        );
        ui.checkbox(
            &mut self.embed_images,
            crate::ui::i18n::text(locale, "export.embed_images"),
        );
        ui.checkbox(
            &mut self.preserve_editability,
            crate::ui::i18n::text(locale, "export.preserve_editability"),
        );

        ui.add_space(6.0);

        // Raster Resolution (DPI / PPI) options for PNG / JPEG / PDF
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(crate::ui::i18n::text(locale, "export.resolution"))
                    .size(10.5)
                    .color(Color32::from_rgb(180, 180, 180)),
            );
            let combo_w = ui.available_width().max(140.0);
            let scale = state.export_scale;
            let approx = |a: f32, b: f32| (a - b).abs() < 1e-4;
            let selected: String = if approx(scale, 1.0) {
                crate::ui::i18n::text(locale, "export.resolution.screen").into_owned()
            } else if approx(scale, 2.0) {
                "Retina (2x)".into()
            } else if approx(scale, 150.0 / 72.0) {
                crate::ui::i18n::text(locale, "export.resolution.medium").into_owned()
            } else if approx(scale, 300.0 / 72.0) {
                crate::ui::i18n::text(locale, "export.resolution.high").into_owned()
            } else {
                crate::ui::i18n::format(
                    locale,
                    "export.resolution.custom",
                    &[("scale", &format!("{}", (scale * 100.0).round() / 100.0))],
                )
            };
            egui::ComboBox::from_id_salt("exp_modal_ppi")
                .selected_text(selected)
                .width(combo_w.min(280.0))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            approx(scale, 1.0),
                            crate::ui::i18n::text(locale, "export.resolution.screen"),
                        )
                        .clicked()
                    {
                        state.export_scale = 1.0;
                    }
                    if ui
                        .selectable_label(
                            approx(scale, 150.0 / 72.0),
                            crate::ui::i18n::text(locale, "export.resolution.medium"),
                        )
                        .clicked()
                    {
                        state.export_scale = 150.0 / 72.0;
                    }
                    if ui
                        .selectable_label(
                            approx(scale, 300.0 / 72.0),
                            crate::ui::i18n::text(locale, "export.resolution.high"),
                        )
                        .clicked()
                    {
                        state.export_scale = 300.0 / 72.0;
                    }
                    if ui
                        .selectable_label(
                            approx(scale, 2.0),
                            crate::ui::i18n::text(locale, "export.resolution.retina"),
                        )
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
            RichText::new(crate::ui::i18n::text(locale, "export.scope"))
                .strong()
                .size(11.0)
                .color(Color32::from_rgb(180, 180, 180)),
        );
        ui.radio_value(
            &mut self.export_scope,
            ExportScope::All,
            crate::ui::i18n::text(locale, "export.scope.all"),
        );
        ui.radio_value(
            &mut self.export_scope,
            ExportScope::Selected,
            crate::ui::i18n::text(locale, "export.scope.selected"),
        );
        ui.horizontal_wrapped(|ui| {
            ui.radio_value(
                &mut self.export_scope,
                ExportScope::Range,
                crate::ui::i18n::text(locale, "export.scope.range"),
            );
            ui.add(egui::TextEdit::singleline(&mut self.custom_range).desired_width(70.0));
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        // Color profile
        ui.label(
            RichText::new(crate::ui::i18n::text(locale, "export.color_profile"))
                .strong()
                .size(11.0)
                .color(Color32::from_rgb(180, 180, 180)),
        );
        let profile_w = ui.available_width().max(160.0);
        egui::ComboBox::from_id_salt("exp_color_profile")
            .selected_text(&self.color_profile)
            .width(profile_w.min(320.0))
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
            crate::ui::i18n::text(locale, "export.embed_profile"),
        );
    }

    /// Keep only objects whose bbox center falls on one of the given
    /// (0-based) artboards. Out-of-range indices select nothing.
    fn filter_artboards(
        doc: &crate::core::document::Document,
        indices: &[usize],
    ) -> crate::core::document::Document {
        let boards = doc.effective_artboards();
        let mut out = doc.clone();
        for layer in &mut out.layers {
            layer.objects.retain(|o| {
                let Some((mn, mx)) = o.bounding_box() else {
                    return true;
                };
                let cx = (mn.x + mx.x) / 2.0;
                let cy = (mn.y + mx.y) / 2.0;
                indices.iter().any(|&i| {
                    boards.get(i).is_some_and(|b| {
                        cx >= b.x && cx <= b.x + b.width && cy >= b.y && cy <= b.y + b.height
                    })
                })
            });
        }
        out
    }

    /// Parse "1,3" / "2-4" (1-based, like the dialog hint) into 0-based
    /// artboard indices. `None` = unparseable.
    fn parse_range(spec: &str) -> Option<Vec<usize>> {
        let mut ids = Vec::new();
        let spec = spec.trim();
        if spec.is_empty() {
            return None;
        }
        for part in spec.split(',') {
            let part = part.trim();
            if let Some((a, b)) = part.split_once('-') {
                let (a, b): (usize, usize) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
                if a == 0 || b == 0 || a > b {
                    return None;
                }
                ids.extend((a - 1)..b);
            } else {
                let n: usize = part.parse().ok()?;
                if n == 0 {
                    return None;
                }
                ids.push(n - 1);
            }
        }
        if ids.is_empty() {
            return None;
        }
        ids.sort_unstable();
        ids.dedup();
        Some(ids)
    }

    fn run_export(&mut self, state: &mut AppState) -> bool {
        let locale = state.prefs.language.clone();
        // Single scale source: the visible PPI combo writes
        // `state.export_scale`; every raster arm below consumes it.
        // (A second `scale_factor` string used to shadow it and was
        // always 1x — dead PPI choice.)
        let scale = state.export_scale;
        let scale_label = format!("{:.2}x", (scale * 100.0).round() / 100.0);
        // Scope filters by enum (locale-independent):
        // - All: whole document
        // - Selected: active artboard only
        // - Range: 1-based list/ranges like "1,3" or "2-4".
        let export_doc;
        let doc_ref: &crate::core::document::Document = match self.export_scope {
            ExportScope::Selected => {
                export_doc = Self::filter_artboards(&state.document, &[state.active_artboard_idx]);
                &export_doc
            }
            ExportScope::Range => match Self::parse_range(&self.custom_range) {
                Some(ids) => {
                    export_doc = Self::filter_artboards(&state.document, &ids);
                    &export_doc
                }
                None => {
                    state.notify_error(
                        crate::ui::i18n::text(&locale, "export.range_invalid").into_owned(),
                    );
                    return false;
                }
            },
            ExportScope::All => &state.document,
        };
        // `true` = a file was chosen and written (or attempted); the
        // caller closes the dialog. `false` = validation failure or
        // file-dialog cancel; keep the dialog open for retry.
        let mut chose_file = false;

        match self.active_format {
            ExportFormatTab::Svg => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("SVG", &["svg"])
                    .set_file_name(format!("{}.svg", self.file_name))
                    .save_file()
                {
                    chose_file = true;
                    let svg = crate::io::svg::export_svg_with_options(
                        doc_ref,
                        state.export_svg_embed_fonts,
                        self.embed_color_profile
                            .then_some(self.color_profile.as_str()),
                        state.export_outline_text,
                    );
                    match crate::io::atomic::atomic_write_str(&path, &svg) {
                        Ok(_) => state.notify_info(crate::ui::i18n::format(
                            &locale,
                            "export.saved.plain",
                            &[
                                ("format", "SVG"),
                                (
                                    "file",
                                    path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                                ),
                            ],
                        )),
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "export.svg_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                }
            }
            ExportFormatTab::Pdf => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PDF", &["pdf"])
                    .set_file_name(format!("{}.pdf", self.file_name))
                    .save_file()
                {
                    chose_file = true;
                    let pdf_bytes = crate::io::pdf::export_pdf_with_profile(
                        doc_ref,
                        self.embed_color_profile
                            .then_some(self.color_profile.as_str()),
                    );
                    match crate::io::atomic::atomic_write_bytes(&path, &pdf_bytes) {
                        Ok(_) => state.notify_info(crate::ui::i18n::format(
                            &locale,
                            "export.saved.plain",
                            &[
                                ("format", "PDF"),
                                (
                                    "file",
                                    path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                                ),
                            ],
                        )),
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "export.pdf_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                }
            }
            ExportFormatTab::Ai => {
                // PDF-compatible .ai: Illustrator opens the PDF portion.
                // No Illustrator private edit data is written (honest
                // subset — layers/structure do not round-trip).
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Illustrator", &["ai"])
                    .set_file_name(format!("{}.ai", self.file_name))
                    .save_file()
                {
                    chose_file = true;
                    let ai_bytes = crate::io::pdf::export_pdf(doc_ref);
                    match crate::io::atomic::atomic_write_bytes(&path, &ai_bytes) {
                        Ok(_) => state.notify_info(crate::ui::i18n::format(
                            &locale,
                            "export.ai_saved",
                            &[(
                                "file",
                                path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                            )],
                        )),
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "export.write_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                }
            }
            ExportFormatTab::Png => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PNG", &["png"])
                    .set_file_name(format!("{}.png", self.file_name))
                    .save_file()
                {
                    chose_file = true;
                    match crate::io::raster::export_png_with_outline(
                        doc_ref,
                        scale,
                        true,
                        state.export_outline_text,
                    ) {
                        Ok(png_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &png_bytes) {
                                Ok(_) => state.notify_info(crate::ui::i18n::format(
                                    &locale,
                                    "export.saved_with_scale",
                                    &[
                                        ("format", "PNG"),
                                        ("scale", &scale_label),
                                        (
                                            "file",
                                            path.file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("file"),
                                        ),
                                    ],
                                )),
                                Err(e) => state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "export.write_failed",
                                    &[("error", &e.to_string())],
                                )),
                            }
                        }
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "export.raster_failed",
                            &[("format", "PNG"), ("error", &e.to_string())],
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
                    chose_file = true;
                    match crate::io::raster::export_jpeg(doc_ref, scale) {
                        Ok(jpeg_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &jpeg_bytes) {
                                Ok(_) => state.notify_info(crate::ui::i18n::format(
                                    &locale,
                                    "export.saved_with_scale",
                                    &[
                                        ("format", "JPEG"),
                                        ("scale", &scale_label),
                                        (
                                            "file",
                                            path.file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("file"),
                                        ),
                                    ],
                                )),
                                Err(e) => state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "export.write_failed",
                                    &[("error", &e.to_string())],
                                )),
                            }
                        }
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "export.raster_failed",
                            &[("format", "JPEG"), ("error", &e.to_string())],
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
                    chose_file = true;
                    match crate::io::raster::export_webp(doc_ref, scale, true) {
                        Ok(webp_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &webp_bytes) {
                                Ok(_) => state.notify_info(crate::ui::i18n::format(
                                    &locale,
                                    "export.saved_with_scale",
                                    &[
                                        ("format", "WebP"),
                                        ("scale", &scale_label),
                                        (
                                            "file",
                                            path.file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("file"),
                                        ),
                                    ],
                                )),
                                Err(e) => state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "export.write_failed",
                                    &[("error", &e.to_string())],
                                )),
                            }
                        }
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "export.raster_failed",
                            &[("format", "WebP"), ("error", &e.to_string())],
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
                    chose_file = true;
                    match crate::io::raster::export_avif(doc_ref, scale, true) {
                        Ok(avif_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &avif_bytes) {
                                Ok(_) => state.notify_info(crate::ui::i18n::format(
                                    &locale,
                                    "export.saved_with_scale",
                                    &[
                                        ("format", "AVIF"),
                                        ("scale", &scale_label),
                                        (
                                            "file",
                                            path.file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("file"),
                                        ),
                                    ],
                                )),
                                Err(e) => state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "export.write_failed",
                                    &[("error", &e.to_string())],
                                )),
                            }
                        }
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "export.raster_failed",
                            &[("format", "AVIF"), ("error", &e.to_string())],
                        )),
                    }
                }
            }
        }
        chose_file
    }
}
