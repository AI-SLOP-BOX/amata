use crate::core::state::AppState;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};

pub struct NewDocModal {
    pub is_open: bool,
    pub active_tab: NewDocCategory,
    pub doc_name: String,
    pub width: f64,
    pub height: f64,
    pub unit: crate::core::unit::LengthUnit,
    pub orientation: Orientation,
    pub artboard_count: usize,
    pub bleed_top: f64,
    pub bleed_bottom: f64,
    pub bleed_left: f64,
    pub bleed_right: f64,
    pub bleed_linked: bool,
    pub color_mode: String,
    pub raster_dpi: String,
    pub preview_mode: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewDocCategory {
    Print,
    Web,
    Mobile,
    Social,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
}

impl Default for NewDocModal {
    fn default() -> Self {
        Self {
            is_open: false,
            active_tab: NewDocCategory::Print,
            doc_name: "名称未設定-2".to_string(),
            width: 210.0,
            height: 297.0,
            unit: crate::core::unit::LengthUnit::Mm,
            orientation: Orientation::Portrait,
            artboard_count: 1,
            bleed_top: 3.0,
            bleed_bottom: 3.0,
            bleed_left: 3.0,
            bleed_right: 3.0,
            bleed_linked: true,
            color_mode: "RGB カラー".to_string(),
            raster_dpi: "高解像度 (300 ppi)".to_string(),
            preview_mode: "デフォルト".to_string(),
        }
    }
}

/// A validated new-document request: dimensions already converted to px.
pub struct NewDocRequest {
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub color_mode: crate::core::document::ColorMode,
    pub artboard_count: usize,
    /// Bleed in points (max of the four sides).
    pub bleed: f64,
}

impl NewDocModal {
    /// Unit conversion applied on create (previously the unit selector was
    /// decorative: 210mm produced a 210px canvas).
    fn dims_to_px(&self) -> (f64, f64) {
        let factor = self.unit.px_per_unit();
        (
            (self.width * factor).clamp(10.0, 16384.0),
            (self.height * factor).clamp(10.0, 16384.0),
        )
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> Option<NewDocRequest> {
        let locale = state.prefs.language.clone();
        if !self.is_open {
            // While closed, keep the preselected unit on the preference
            // default. Home-screen presets write `unit` immediately before
            // opening, and this runs on every frame *before* that, so a
            // preset still wins for the preset it picked.
            self.unit = state.prefs.default_doc_unit;
            return None;
        }
        // "新規ドキュメントを作成するときに設定ダイアログを表示" is off: the
        // current fields (defaults or a home-screen preset) *are* the settings,
        // so create straight away instead of showing the dialog.
        if !state.prefs.show_new_doc_dialog {
            self.is_open = false;
            return Some(self.build_request());
        }
        let mut request = None;
        let mut close_clicked = false;

        let mut is_open = self.is_open;
        let screen = ctx.screen_rect();
        let (size, min_size, pos) =
            crate::ui::window_defaults(screen, Vec2::new(720.0, 580.0), Vec2::new(320.0, 320.0));
        egui::Window::new(crate::ui::i18n::text(&locale, "new_doc.title"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_pos(pos)
            .default_size(size)
            .min_size(min_size)
            .show(ctx, |ui| {
                let total_w = ui.available_width();
                crate::ui::modal_body(ui, "nd_body", &mut |ui, is_body| {
                    if !is_body {
                        let btn_h = 28.0;
                        let gap = 8.0;
                        let avail = ui.available_width();
                        let cancel_w = ((avail - gap) * 0.35).max(80.0);
                        let create_w = (avail - gap - cancel_w).max(90.0);
                        if ui
                            .add(
                                egui::Button::new(crate::ui::i18n::text(&locale, "new_doc.cancel"))
                                    .min_size(Vec2::new(cancel_w, btn_h)),
                            )
                            .clicked()
                        {
                            close_clicked = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(crate::ui::i18n::text(&locale, "new_doc.create"))
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(20, 115, 230))
                                .min_size(Vec2::new(create_w, btn_h)),
                            )
                            .clicked()
                        {
                            request = Some(self.build_request());
                            self.is_open = false;
                        }
                        return;
                    }
                    ui.set_width(total_w);
                    self.show_presets(ui, total_w, &locale);
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    self.show_details(ui, total_w, &locale);
                });
            });
        // A submitted request always closes the modal (the old code
        // restored `is_open` unconditionally, so Create never closed it).
        if request.is_some() || close_clicked {
            self.is_open = false;
        } else {
            self.is_open = is_open;
        }
        request
    }

    /// Validate the current fields into a request. Shared by the *作成*
    /// button and the no-dialog path of [`Self::show`].
    fn build_request(&self) -> NewDocRequest {
        let (w, h) = self.dims_to_px();
        let name = if self.doc_name.trim().is_empty() {
            "名称未設定".to_string()
        } else {
            self.doc_name.trim().to_string()
        };
        let color_mode = if self.color_mode.contains("CMYK") {
            crate::core::document::ColorMode::Cmyk
        } else {
            crate::core::document::ColorMode::Rgb
        };
        NewDocRequest {
            name,
            width: w,
            height: h,
            color_mode,
            artboard_count: self.artboard_count,
            bleed: crate::core::print::mm_to_pt(
                self.bleed_top
                    .max(self.bleed_bottom)
                    .max(self.bleed_left)
                    .max(self.bleed_right)
                    .max(0.0),
            ),
        }
    }

    fn show_presets(&mut self, ui: &mut Ui, left_w: f32, locale: &str) {
        ui.set_width(left_w);

        // Category Tabs: 印刷 | Web | モバイル | ソーシャル | カスタム
        ui.horizontal_wrapped(|ui| {
            let tabs = [
                (
                    NewDocCategory::Print,
                    crate::ui::i18n::text(locale, "new_doc.category.print"),
                ),
                (
                    NewDocCategory::Web,
                    crate::ui::i18n::text(locale, "new_doc.category.web"),
                ),
                (
                    NewDocCategory::Mobile,
                    crate::ui::i18n::text(locale, "new_doc.category.mobile"),
                ),
                (
                    NewDocCategory::Social,
                    crate::ui::i18n::text(locale, "new_doc.category.social"),
                ),
                (
                    NewDocCategory::Custom,
                    crate::ui::i18n::text(locale, "new_doc.category.custom"),
                ),
            ];
            for (cat, name) in tabs {
                let is_active = self.active_tab == cat;
                let txt = if is_active {
                    RichText::new(name)
                        .strong()
                        .color(Color32::WHITE)
                        .size(12.5)
                } else {
                    RichText::new(name)
                        .color(Color32::from_rgb(170, 170, 170))
                        .size(12.5)
                };
                if ui.selectable_label(is_active, txt).clicked() {
                    self.active_tab = cat;
                }
            }
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(crate::ui::i18n::text(locale, "new_doc.blank_presets"))
                .strong()
                .size(11.5)
                .color(Color32::from_rgb(200, 200, 200)),
        );
        ui.add_space(6.0);

        let presets = [
            (
                "doc",
                "A4",
                "210 × 297 mm",
                210.0,
                297.0,
                crate::core::unit::LengthUnit::Mm,
            ),
            (
                "doc",
                "A3",
                "297 × 420 mm",
                297.0,
                420.0,
                crate::core::unit::LengthUnit::Mm,
            ),
            (
                "doc",
                "A5",
                "148 × 210 mm",
                148.0,
                210.0,
                crate::core::unit::LengthUnit::Mm,
            ),
            (
                "doc",
                "B5",
                "182 × 257 mm",
                182.0,
                257.0,
                crate::core::unit::LengthUnit::Mm,
            ),
            (
                "doc",
                "US レター",
                "8.5 × 11 in",
                612.0,
                792.0,
                crate::core::unit::LengthUnit::Pt,
            ),
            (
                "doc",
                "US リーガル",
                "8.5 × 14 in",
                612.0,
                1008.0,
                crate::core::unit::LengthUnit::Pt,
            ),
            (
                "desktop",
                "Web (横長)",
                "1920 × 1080 px",
                1920.0,
                1080.0,
                crate::core::unit::LengthUnit::Px,
            ),
            (
                "desktop",
                "Web (正方形)",
                "1080 × 1080 px",
                1080.0,
                1080.0,
                crate::core::unit::LengthUnit::Px,
            ),
            (
                "phone",
                "iPhone 15",
                "1179 × 2556 px",
                1179.0,
                2556.0,
                crate::core::unit::LengthUnit::Px,
            ),
            (
                "camera",
                "Instagram 投稿",
                "1080 × 1080 px",
                1080.0,
                1080.0,
                crate::core::unit::LengthUnit::Px,
            ),
            (
                "camera",
                "Instagram ストーリー",
                "1080 × 1920 px",
                1080.0,
                1920.0,
                crate::core::unit::LengthUnit::Px,
            ),
            (
                "more",
                "その他のプリセット",
                "カスタム",
                800.0,
                600.0,
                crate::core::unit::LengthUnit::Px,
            ),
        ];

        // Card grid: column count and card width follow the pane width.
        let cols = (((left_w - 10.0) / 155.0).floor() as usize).clamp(1, 4);
        let gap = 10.0;
        let card_w =
            (((left_w - gap * (cols as f32 - 1.0)) / cols as f32).clamp(110.0, 160.0)).floor();

        egui::Grid::new("new_doc_presets_grid")
            .spacing(Vec2::new(gap, gap))
            .show(ui, |ui| {
                for (idx, (icon_type, name, dim, w, h, u)) in presets.iter().enumerate() {
                    let preset_key = match idx {
                        4 => "new_doc.preset.letter",
                        5 => "new_doc.preset.legal",
                        6 => "new_doc.preset.web_landscape",
                        7 => "new_doc.preset.web_square",
                        9 => "new_doc.preset.instagram_post",
                        10 => "new_doc.preset.instagram_story",
                        11 => "new_doc.preset.other",
                        _ => "",
                    };
                    let display_name = if preset_key.is_empty() {
                        std::borrow::Cow::Borrowed(*name)
                    } else {
                        crate::ui::i18n::text(locale, preset_key)
                    };
                    let display_dim = if idx == 11 {
                        crate::ui::i18n::text(locale, "new_doc.preset.custom")
                    } else {
                        std::borrow::Cow::Borrowed(*dim)
                    };
                    let is_sel = self.width == *w && self.height == *h;
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(card_w, 100.0), egui::Sense::click());
                    let hovered = resp.hovered();

                    let bg_c = if is_sel {
                        Color32::from_rgb(26, 42, 66)
                    } else if hovered {
                        Color32::from_rgb(44, 44, 48)
                    } else {
                        Color32::from_rgb(33, 33, 35)
                    };

                    let stroke_c = if is_sel {
                        Color32::from_rgb(20, 115, 230)
                    } else if hovered {
                        Color32::from_rgb(80, 80, 85)
                    } else {
                        Color32::from_rgb(46, 46, 50)
                    };

                    ui.painter().rect_filled(rect, 4.0, bg_c);
                    ui.painter().rect_stroke(
                        rect,
                        4.0,
                        Stroke::new(if is_sel { 1.5_f32 } else { 1.0_f32 }, stroke_c),
                        StrokeKind::Inside,
                    );

                    // Draw crisp vector preset icon
                    let icon_box = Rect::from_center_size(
                        Pos2::new(rect.center().x, rect.min.y + 24.0),
                        Vec2::splat(26.0),
                    );
                    let icon_col = if is_sel {
                        Color32::from_rgb(20, 115, 230)
                    } else if hovered {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(180)
                    };
                    match *icon_type {
                        "doc" => crate::app::icons::icon_document(ui.painter(), icon_box, icon_col),
                        "desktop" => {
                            crate::app::icons::icon_desktop(ui.painter(), icon_box, icon_col)
                        }
                        "phone" => crate::app::icons::icon_phone(ui.painter(), icon_box, icon_col),
                        "camera" => {
                            crate::app::icons::icon_camera(ui.painter(), icon_box, icon_col)
                        }
                        _ => crate::app::icons::icon_more_dots(ui.painter(), icon_box, icon_col),
                    }

                    // Name
                    ui.painter().text(
                        Pos2::new(rect.center().x, rect.min.y + 58.0),
                        egui::Align2::CENTER_CENTER,
                        display_name,
                        egui::FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                    // Dimension
                    ui.painter().text(
                        Pos2::new(rect.center().x, rect.min.y + 78.0),
                        egui::Align2::CENTER_CENTER,
                        display_dim,
                        egui::FontId::proportional(9.5),
                        Color32::from_rgb(140, 140, 140),
                    );

                    if resp.clicked() {
                        self.width = *w;
                        self.height = *h;
                        self.unit = *u;
                    }

                    if (idx + 1) % cols == 0 {
                        ui.end_row();
                    }
                }
            });

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui::i18n::text(locale, "new_doc.search_templates"))
                    .size(11.0)
                    .color(Color32::from_rgb(160, 160, 160)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(crate::ui::i18n::text(locale, "new_doc.stock_assets"))
                        .size(11.0)
                        .color(Color32::from_rgb(20, 115, 230)),
                );
            });
        });
    }

    fn show_details(&mut self, ui: &mut Ui, right_w: f32, locale: &str) {
        ui.set_width(right_w);
        ui.label(
            RichText::new(crate::ui::i18n::text(locale, "new_doc.details"))
                .strong()
                .size(12.5)
                .color(Color32::WHITE),
        );
        ui.add_space(8.0);

        ui.label(
            RichText::new(crate::ui::i18n::text(locale, "new_doc.name"))
                .size(10.5)
                .color(Color32::from_rgb(170, 170, 170)),
        );
        ui.text_edit_singleline(&mut self.doc_name);
        ui.add_space(6.0);

        let half_w = ((right_w - 16.0) / 2.0).max(90.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(half_w);
                ui.label(
                    RichText::new(crate::ui::i18n::text(locale, "new_doc.width"))
                        .size(10.5)
                        .color(Color32::from_rgb(170, 170, 170)),
                );
                ui.add(
                    egui::DragValue::new(&mut self.width)
                        .speed(1.0)
                        .range(10.0..=10000.0),
                );
            });
            ui.vertical(|ui| {
                ui.set_width(half_w);
                ui.label(
                    RichText::new(crate::ui::i18n::text(locale, "new_doc.unit"))
                        .size(10.5)
                        .color(Color32::from_rgb(170, 170, 170)),
                );
                use crate::core::unit::LengthUnit;
                let display_unit = match self.unit {
                    LengthUnit::Mm => crate::ui::i18n::text(locale, "home.unit.millimeters"),
                    LengthUnit::Px => crate::ui::i18n::text(locale, "home.unit.pixels"),
                    LengthUnit::Pt => crate::ui::i18n::text(locale, "home.unit.points"),
                    LengthUnit::In => crate::ui::i18n::text(locale, "new_doc.unit.inches"),
                };
                egui::ComboBox::from_id_salt("doc_unit")
                    .selected_text(display_unit)
                    .width((half_w - 10.0).max(80.0))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.unit,
                            LengthUnit::Mm,
                            crate::ui::i18n::text(locale, "home.unit.millimeters"),
                        );
                        ui.selectable_value(
                            &mut self.unit,
                            LengthUnit::Px,
                            crate::ui::i18n::text(locale, "home.unit.pixels"),
                        );
                        ui.selectable_value(
                            &mut self.unit,
                            LengthUnit::Pt,
                            crate::ui::i18n::text(locale, "home.unit.points"),
                        );
                        ui.selectable_value(
                            &mut self.unit,
                            LengthUnit::In,
                            crate::ui::i18n::text(locale, "new_doc.unit.inches"),
                        );
                    });
            });
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(half_w);
                ui.label(
                    RichText::new(crate::ui::i18n::text(locale, "new_doc.height"))
                        .size(10.5)
                        .color(Color32::from_rgb(170, 170, 170)),
                );
                ui.add(
                    egui::DragValue::new(&mut self.height)
                        .speed(1.0)
                        .range(10.0..=10000.0),
                );
            });
            ui.vertical(|ui| {
                ui.set_width(half_w);
                ui.label(
                    RichText::new(crate::ui::i18n::text(locale, "new_doc.direction"))
                        .size(10.5)
                        .color(Color32::from_rgb(170, 170, 170)),
                );
                ui.horizontal(|ui| {
                    let p_sel = self.orientation == Orientation::Portrait;
                    if ui
                        .selectable_label(p_sel, crate::ui::i18n::text(locale, "new_doc.portrait"))
                        .clicked()
                    {
                        self.orientation = Orientation::Portrait;
                        if self.width > self.height {
                            std::mem::swap(&mut self.width, &mut self.height);
                        }
                    }
                    let l_sel = self.orientation == Orientation::Landscape;
                    if ui
                        .selectable_label(l_sel, crate::ui::i18n::text(locale, "new_doc.landscape"))
                        .clicked()
                    {
                        self.orientation = Orientation::Landscape;
                        if self.height > self.width {
                            std::mem::swap(&mut self.width, &mut self.height);
                        }
                    }
                });
            });
        });

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(crate::ui::i18n::text(locale, "new_doc.artboard_count")).size(10.5),
            );
            ui.add(egui::DragValue::new(&mut self.artboard_count).range(1..=100));
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        // 塗り足し (Bleed)
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui::i18n::text(locale, "new_doc.bleed"))
                    .size(10.5)
                    .color(Color32::from_rgb(170, 170, 170)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let link_sym = if self.bleed_linked { "🔗" } else { "⛓" };
                if ui.selectable_label(self.bleed_linked, link_sym).clicked() {
                    self.bleed_linked = !self.bleed_linked;
                }
            });
        });

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(crate::ui::i18n::text(locale, "new_doc.top")).size(10.0));
            if ui
                .add(egui::DragValue::new(&mut self.bleed_top).speed(0.5))
                .changed()
                && self.bleed_linked
            {
                self.bleed_bottom = self.bleed_top;
                self.bleed_left = self.bleed_top;
                self.bleed_right = self.bleed_top;
            }
            ui.label(RichText::new(crate::ui::i18n::text(locale, "new_doc.bottom")).size(10.0));
            if ui
                .add(egui::DragValue::new(&mut self.bleed_bottom).speed(0.5))
                .changed()
                && self.bleed_linked
            {
                self.bleed_top = self.bleed_bottom;
                self.bleed_left = self.bleed_bottom;
                self.bleed_right = self.bleed_bottom;
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(crate::ui::i18n::text(locale, "new_doc.left")).size(10.0));
            if ui
                .add(egui::DragValue::new(&mut self.bleed_left).speed(0.5))
                .changed()
                && self.bleed_linked
            {
                self.bleed_top = self.bleed_left;
                self.bleed_bottom = self.bleed_left;
                self.bleed_right = self.bleed_left;
            }
            ui.label(RichText::new(crate::ui::i18n::text(locale, "new_doc.right")).size(10.0));
            if ui
                .add(egui::DragValue::new(&mut self.bleed_right).speed(0.5))
                .changed()
                && self.bleed_linked
            {
                self.bleed_top = self.bleed_right;
                self.bleed_bottom = self.bleed_right;
                self.bleed_left = self.bleed_right;
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        // Color mode & Raster effects
        ui.label(
            RichText::new(crate::ui::i18n::text(locale, "new_doc.color_mode"))
                .size(10.5)
                .color(Color32::from_rgb(170, 170, 170)),
        );
        let mode_w = ui.available_width().max(160.0);
        let display_color_mode = if self.color_mode.contains("CMYK") {
            crate::ui::i18n::text(locale, "new_doc.cmyk")
        } else {
            crate::ui::i18n::text(locale, "new_doc.rgb")
        };
        egui::ComboBox::from_id_salt("doc_color_mode")
            .selected_text(display_color_mode)
            .width(mode_w.min(320.0))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.color_mode,
                    "RGB カラー".into(),
                    crate::ui::i18n::text(locale, "new_doc.rgb"),
                );
                ui.selectable_value(
                    &mut self.color_mode,
                    "CMYK カラー".into(),
                    crate::ui::i18n::text(locale, "new_doc.cmyk"),
                );
            });

        ui.add_space(4.0);
        ui.label(
            RichText::new(crate::ui::i18n::text(locale, "new_doc.raster_effects"))
                .size(10.5)
                .color(Color32::from_rgb(170, 170, 170)),
        );
        let dpi_w = ui.available_width().max(160.0);
        let display_dpi = if self.raster_dpi.contains("72") {
            crate::ui::i18n::text(locale, "new_doc.screen_dpi")
        } else {
            crate::ui::i18n::text(locale, "new_doc.high_dpi")
        };
        egui::ComboBox::from_id_salt("doc_raster_dpi")
            .selected_text(display_dpi)
            .width(dpi_w.min(320.0))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.raster_dpi,
                    "高解像度 (300 ppi)".into(),
                    crate::ui::i18n::text(locale, "new_doc.high_dpi"),
                );
                ui.selectable_value(
                    &mut self.raster_dpi,
                    "スクリーン (72 ppi)".into(),
                    crate::ui::i18n::text(locale, "new_doc.screen_dpi"),
                );
            });
    }
}
