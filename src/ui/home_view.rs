use crate::core::state::AppState;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};

// Minimum main-column width before the home screen splits into two
// columns; below it the right rail stacks under the main column.
pub(crate) const TWO_COL_MIN_W: f32 = 420.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeSidebarTab {
    Home,
    Recent,
    Cloud,
    Learn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomePresetCategory {
    Recommend,
    Print,
    Web,
    Mobile,
    Social,
    Custom,
}

pub struct RecentFileItem {
    pub title: String,
    pub date: String,
    pub dimensions: (f64, f64),
    pub color: Color32,
    pub accent: Color32,
    pub path: std::path::PathBuf,
}

/// Actions the home screen can request; handled by the app (file IO,
// watcher rebinding and history live there, not in the view).
pub enum HomeAction {
    OpenFile(std::path::PathBuf),
    RestoreRecovery,
    DismissRecovery,
}

pub struct HomeView {
    pub is_open: bool,
    pub current_tab: HomeSidebarTab,
    pub preset_cat: HomePresetCategory,
    pub search_query: String,
    pub recent_files: Vec<RecentFileItem>,
}

impl Default for HomeView {
    fn default() -> Self {
        let mut view = Self {
            is_open: true, // Show Home by default like CC
            current_tab: HomeSidebarTab::Home,
            preset_cat: HomePresetCategory::Recommend,
            search_query: String::new(),
            recent_files: Vec::new(),
        };
        view.refresh_recents();
        view
    }
}

impl HomeView {
    /// Reload real recent-file entries (missing files are filtered out by
    /// the loader, so the grid never shows dead cards).
    pub fn refresh_recents(&mut self) {
        self.recent_files = crate::io::recent::load_recents()
            .into_iter()
            .map(|e| {
                let (c, a) = crate::io::recent::entry_colors(&e.path);
                RecentFileItem {
                    title: e.name,
                    date: crate::io::recent::friendly_age(e.last_opened_secs),
                    dimensions: (e.width, e.height),
                    color: Color32::from_rgb(c[0], c[1], c[2]),
                    accent: Color32::from_rgb(a[0], a[1], a[2]),
                    path: std::path::PathBuf::from(e.path),
                }
            })
            .collect();
    }
}

impl HomeView {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        state: &mut AppState,
        new_doc_modal: &mut crate::ui::NewDocModal,
        tour_guide_open: &mut bool,
    ) -> Option<HomeAction> {
        if !self.is_open {
            return None;
        }

        let mut action = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(26, 26, 26)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left Global Navigation Bar (180px)
                    if let Some(a) = self.show_sidebar(ui, new_doc_modal) {
                        action = Some(a);
                    }

                    ui.add_space(8.0);

                    // Central & Right Scrollable Area
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            let avail_w = ui.available_width();
                            let right_w = 280.0;
                            let gap = 16.0;
                            // Two columns when there is room for a usable main
                            // column plus the right rail; otherwise stack so
                            // the tutorial rail is never clipped away.
                            let two_col = avail_w >= TWO_COL_MIN_W + gap + right_w;

                            if two_col {
                                ui.horizontal_top(|ui| {
                                    // Main Content (Welcome, New Doc Presets, Recent Files)
                                    let main_w = (avail_w - gap - right_w).min(780.0);
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(main_w, ui.available_height()),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.set_width(main_w);
                                            if let Some(a) = self.show_recovery_banner(ui) {
                                                action = Some(a);
                                            }
                                            self.show_welcome_banner(ui);
                                            ui.add_space(16.0);
                                            self.show_preset_section(ui, state, new_doc_modal);
                                            ui.add_space(20.0);
                                            if let Some(a) = self.show_recent_files(ui, state) {
                                                action = Some(a);
                                            }
                                        },
                                    );

                                    ui.add_space(gap);

                                    // Right Information Panel (Smooth workflow, tips, recent list)
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(right_w, ui.available_height()),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.set_width(right_w);
                                            self.show_tutorial_widget(ui, tour_guide_open);
                                            ui.add_space(14.0);
                                            self.show_tips_widget(ui);
                                            ui.add_space(14.0);
                                            if let Some(a) =
                                                self.show_recent_projects_list(ui, state)
                                            {
                                                action = Some(a);
                                            }
                                        },
                                    );
                                });
                            } else {
                                // Narrow: stack main above the right rail so
                                // the tutorial stays reachable.
                                ui.vertical(|ui| {
                                    if let Some(a) = self.show_recovery_banner(ui) {
                                        action = Some(a);
                                    }
                                    self.show_welcome_banner(ui);
                                    ui.add_space(16.0);
                                    self.show_preset_section(ui, state, new_doc_modal);
                                    ui.add_space(20.0);
                                    if let Some(a) = self.show_recent_files(ui, state) {
                                        action = Some(a);
                                    }
                                });
                                ui.add_space(gap);
                                ui.separator();
                                ui.add_space(gap);
                                ui.vertical(|ui| {
                                    let rail_w = avail_w.min(right_w);
                                    ui.set_width(rail_w);
                                    self.show_tutorial_widget(ui, tour_guide_open);
                                    ui.add_space(14.0);
                                    self.show_tips_widget(ui);
                                    ui.add_space(14.0);
                                    if let Some(a) = self.show_recent_projects_list(ui, state) {
                                        action = Some(a);
                                    }
                                });
                            }
                        });
                });
            });
        action
    }

    /// Autosave recovery banner (only when a recovery snapshot exists).
    fn show_recovery_banner(&mut self, ui: &mut Ui) -> Option<HomeAction> {
        if !crate::io::project::has_recovery() {
            return None;
        }
        let mut action = None;
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("🛟 未保存の作業データがあります")
                        .strong()
                        .color(Color32::from_rgb(255, 200, 100)),
                );
                if ui.button("復元する").clicked() {
                    action = Some(HomeAction::RestoreRecovery);
                }
                if ui.button("破棄する").clicked() {
                    action = Some(HomeAction::DismissRecovery);
                }
            });
        });
        ui.add_space(8.0);
        action
    }

    fn show_sidebar(
        &mut self,
        ui: &mut Ui,
        new_doc_modal: &mut crate::ui::NewDocModal,
    ) -> Option<HomeAction> {
        let mut action = None;
        ui.vertical(|ui| {
            ui.add_space(10.0);

            // Amata Brand Header in Sidebar
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                let (logo_rect, _) =
                    ui.allocate_exact_size(Vec2::new(28.0, 28.0), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(logo_rect, 5.0, Color32::from_rgb(26, 26, 32));
                p.rect_stroke(
                    logo_rect,
                    5.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(46, 46, 56)),
                    StrokeKind::Outside,
                );
                crate::app::icons::icon_amata_logo(p, logo_rect.shrink(2.0));

                ui.add_space(8.0);
                ui.label(
                    RichText::new("Amata")
                        .size(16.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.label(
                    RichText::new("数多")
                        .size(11.0)
                        .color(Color32::from_rgb(180, 180, 190)),
                );
            });

            ui.add_space(16.0);

            // Nav Tabs — clean professional typography matching Illustrator CC Home
            let tabs = [
                (HomeSidebarTab::Home, "ホーム"),
                (HomeSidebarTab::Recent, "最近使用したファイル"),
                (HomeSidebarTab::Cloud, "クラウドドキュメント"),
                (HomeSidebarTab::Learn, "学ぶ"),
            ];

            for (tab, label) in tabs {
                let is_sel = self.current_tab == tab;
                let btn = if is_sel {
                    egui::Button::new(
                        RichText::new(label)
                            .strong()
                            .size(12.0)
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(45, 45, 48))
                    .min_size(Vec2::new(160.0, 30.0))
                } else {
                    egui::Button::new(
                        RichText::new(label)
                            .size(12.0)
                            .color(Color32::from_rgb(190, 190, 190)),
                    )
                    .fill(Color32::TRANSPARENT)
                    .min_size(Vec2::new(160.0, 30.0))
                };
                if ui.add(btn).clicked() {
                    self.current_tab = tab;
                }
            }

            ui.add_space(24.0);

            // "新規作成" Pill Button
            let new_btn = egui::Button::new(
                RichText::new("新規作成")
                    .strong()
                    .size(12.5)
                    .color(Color32::from_rgb(20, 20, 20)),
            )
            .fill(Color32::from_rgb(240, 240, 240))
            .corner_radius(16.0)
            .min_size(Vec2::new(140.0, 32.0));
            if ui.add(new_btn).clicked() {
                new_doc_modal.is_open = true;
            }

            ui.add_space(8.0);

            // "開く..." Pill Outline Button
            let open_btn =
                egui::Button::new(RichText::new("開く...").size(12.0).color(Color32::WHITE))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(90, 90, 90)))
                    .corner_radius(16.0)
                    .min_size(Vec2::new(140.0, 30.0));
            if ui.add(open_btn).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "Amata / SVG / PDF / Images",
                        &["amata", "json", "svg", "pdf", "png", "jpg", "jpeg", "webp"],
                    )
                    .pick_file()
                {
                    action = Some(HomeAction::OpenFile(path));
                }
            }
        });
        action
    }

    fn show_welcome_banner(&self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().min(760.0), 100.0),
            egui::Sense::hover(),
        );
        // Gradient Banner Background
        ui.painter()
            .rect_filled(rect, 8.0, Color32::from_rgb(33, 33, 36));

        // Diagonal subtle glow
        let p = ui.painter();
        let glow_color = Color32::from_rgba_unmultiplied(79, 70, 229, 60);
        p.circle_filled(
            Pos2::new(rect.max.x - 80.0, rect.center().y),
            70.0,
            glow_color,
        );

        // Official Amata Vector Emblem on the right of banner
        let logo_badge_rect = Rect::from_center_size(
            Pos2::new(rect.max.x - 70.0, rect.center().y),
            Vec2::splat(76.0),
        );
        crate::app::icons::icon_amata_logo(p, logo_badge_rect);

        // Header Text
        p.text(
            Pos2::new(rect.min.x + 24.0, rect.min.y + 24.0),
            egui::Align2::LEFT_TOP,
            "Amata（数多）へようこそ",
            egui::FontId::proportional(22.0),
            Color32::WHITE,
        );
        p.text(
            Pos2::new(rect.min.x + 24.0, rect.min.y + 56.0),
            egui::Align2::LEFT_TOP,
            "美しいベクターグラフィックで、アイデアをカタチに。",
            egui::FontId::proportional(12.5),
            Color32::from_rgb(180, 180, 180),
        );
        p.text(
            Pos2::new(rect.max.x - 130.0, rect.max.y - 18.0),
            egui::Align2::RIGHT_BOTTOM,
            "つくる、ひろがる、もっと自由に。",
            egui::FontId::proportional(11.0),
            Color32::from_rgb(200, 180, 150),
        );
    }

    fn show_preset_section(
        &mut self,
        ui: &mut Ui,
        _state: &mut AppState,
        new_doc_modal: &mut crate::ui::NewDocModal,
    ) {
        ui.label(
            RichText::new("新規ドキュメントを作成")
                .strong()
                .size(13.5)
                .color(Color32::WHITE),
        );
        ui.add_space(4.0);

        // Category Subtabs
        ui.horizontal(|ui| {
            let cats = [
                (HomePresetCategory::Recommend, "おすすめ"),
                (HomePresetCategory::Print, "印刷"),
                (HomePresetCategory::Web, "Web"),
                (HomePresetCategory::Mobile, "モバイル"),
                (HomePresetCategory::Social, "ソーシャル"),
                (HomePresetCategory::Custom, "カスタム"),
            ];
            for (cat, name) in cats {
                let is_active = self.preset_cat == cat;
                let txt = if is_active {
                    RichText::new(name)
                        .strong()
                        .size(11.5)
                        .color(Color32::WHITE)
                } else {
                    RichText::new(name)
                        .size(11.5)
                        .color(Color32::from_rgb(160, 160, 160))
                };
                if ui.selectable_label(is_active, txt).clicked() {
                    self.preset_cat = cat;
                }
            }
        });

        ui.add_space(8.0);

        // 6 Preset Cards — units matter: routing through NewDocModal
        // applies mm/in/pt → px conversion (writing raw 210×297 into the
        // document used to create a 210px "A4").
        let presets = [
            ("doc", "A4", "210 × 297 mm", 210.0, 297.0, "ミリメートル"),
            ("doc", "US レター", "8.5 × 11 in", 612.0, 792.0, "ポイント"),
            (
                "desktop",
                "Web (横長)",
                "1920 × 1080 px",
                1920.0,
                1080.0,
                "ピクセル",
            ),
            (
                "phone",
                "iPhone 15",
                "1179 × 2556 px",
                1179.0,
                2556.0,
                "ピクセル",
            ),
            (
                "camera",
                "Instagram 投稿",
                "1080 × 1080 px",
                1080.0,
                1080.0,
                "ピクセル",
            ),
            (
                "more",
                "その他のプリセット",
                "カスタム",
                800.0,
                600.0,
                "ピクセル",
            ),
        ];

        ui.horizontal_wrapped(|ui| {
            for (icon_type, title, dim, w, h, unit) in presets {
                let card_w = ui.available_width().min(115.0);
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, 110.0), egui::Sense::click());
                let hovered = resp.hovered();

                let bg = if hovered {
                    Color32::from_rgb(46, 46, 52)
                } else {
                    Color32::from_rgb(36, 36, 38)
                };
                let stroke_c = if hovered {
                    Color32::from_rgb(20, 115, 230)
                } else {
                    Color32::from_rgb(50, 50, 54)
                };
                ui.painter().rect_filled(rect, 6.0, bg);
                ui.painter().rect_stroke(
                    rect,
                    6.0,
                    Stroke::new(1.0_f32, stroke_c),
                    StrokeKind::Inside,
                );

                let icon_box = Rect::from_center_size(
                    Pos2::new(rect.center().x, rect.min.y + 28.0),
                    Vec2::splat(28.0),
                );
                let icon_col = if hovered {
                    Color32::WHITE
                } else {
                    Color32::from_gray(180)
                };
                match icon_type {
                    "doc" => crate::app::icons::icon_document(ui.painter(), icon_box, icon_col),
                    "desktop" => crate::app::icons::icon_desktop(ui.painter(), icon_box, icon_col),
                    "phone" => crate::app::icons::icon_phone(ui.painter(), icon_box, icon_col),
                    "camera" => crate::app::icons::icon_camera(ui.painter(), icon_box, icon_col),
                    _ => crate::app::icons::icon_more_dots(ui.painter(), icon_box, icon_col),
                }

                let center_x = rect.center().x;
                ui.painter().text(
                    Pos2::new(center_x, rect.min.y + 62.0),
                    egui::Align2::CENTER_CENTER,
                    title,
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                );
                ui.painter().text(
                    Pos2::new(center_x, rect.min.y + 84.0),
                    egui::Align2::CENTER_CENTER,
                    dim,
                    egui::FontId::proportional(9.5),
                    Color32::from_rgb(150, 150, 150),
                );

                if resp.clicked() {
                    // Prefill and open the New Document dialog so unit
                    // conversion, bleed and artboard count stay consistent
                    // with File → New (the old path wrote raw values and
                    // closed the home screen with no confirmation).
                    new_doc_modal.is_open = true;
                    new_doc_modal.width = w;
                    new_doc_modal.height = h;
                    new_doc_modal.unit = unit.to_string();
                    new_doc_modal.orientation = if h >= w {
                        crate::ui::new_doc_modal::Orientation::Portrait
                    } else {
                        crate::ui::new_doc_modal::Orientation::Landscape
                    };
                    new_doc_modal.doc_name = if title == "その他のプリセット" {
                        "名称未設定".to_string()
                    } else {
                        title.to_string()
                    };
                }
            }
        });
    }

    fn show_recent_files(
        &mut self,
        ui: &mut Ui,
        _state: &mut AppState,
    ) -> Option<HomeAction> {
        if self.recent_files.is_empty() {
            ui.label(
                RichText::new("最近開いたファイルはここに表示されます")
                    .weak()
                    .size(11.0),
            );
            return None;
        }
        let mut action = None;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("最近使用したファイル")
                    .strong()
                    .size(13.5)
                    .color(Color32::WHITE),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("すべて表示 ➔")
                        .size(11.0)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
            });
        });

        ui.add_space(6.0);

        // Card grid: column count tracks pane width so cards never clip.
        let card_w = 175.0_f32;
        let spacing = 14.0_f32;
        let cols = (((ui.available_width() + spacing) / (card_w + spacing)).floor() as usize)
            .clamp(1, 4);
        egui::Grid::new("recent_files_grid")
            .num_columns(cols)
            .spacing(Vec2::new(spacing, spacing))
            .show(ui, |ui| {
                for (i, file) in self.recent_files.iter().enumerate() {
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(card_w, 140.0), egui::Sense::click());
                    let hovered = resp.hovered();

                    let bg = if hovered {
                        Color32::from_rgb(42, 42, 46)
                    } else {
                        Color32::from_rgb(33, 33, 35)
                    };
                    ui.painter().rect_filled(rect, 6.0, bg);
                    if hovered {
                        ui.painter().rect_stroke(
                            rect,
                            6.0,
                            Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)),
                            StrokeKind::Inside,
                        );
                    }

                    // Thumbnail canvas preview box (top 95px)
                    let thumb_rect = Rect::from_min_max(
                        rect.min + Vec2::new(6.0, 6.0),
                        Pos2::new(rect.max.x - 6.0, rect.min.y + 95.0),
                    );
                    ui.painter().rect_filled(thumb_rect, 4.0, file.color);

                    // Subtle vector art demo curves inside thumbnail
                    ui.painter()
                        .circle_filled(thumb_rect.center(), 22.0, file.accent);

                    // Title and timestamp
                    ui.painter().text(
                        Pos2::new(rect.min.x + 8.0, rect.min.y + 106.0),
                        egui::Align2::LEFT_TOP,
                        &file.title,
                        egui::FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                    ui.painter().text(
                        Pos2::new(rect.min.x + 8.0, rect.min.y + 122.0),
                        egui::Align2::LEFT_TOP,
                        &file.date,
                        egui::FontId::proportional(9.5),
                        Color32::from_rgb(140, 140, 140),
                    );

                    if resp.clicked() {
                        action = Some(HomeAction::OpenFile(file.path.clone()));
                    }

                    if (i + 1) % cols == 0 {
                        ui.end_row();
                    }
                }
            });
        action
    }

    fn show_tutorial_widget(&self, ui: &mut Ui, tour_guide_open: &mut bool) {
        let w = ui.available_width().min(265.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 200.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 6.0, Color32::from_rgb(33, 33, 36));

        ui.painter().text(
            Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0),
            egui::Align2::LEFT_TOP,
            "制作をもっとスムーズに",
            egui::FontId::proportional(12.5),
            Color32::WHITE,
        );

        // Vector path tutorial illustration preview box
        let preview_box = Rect::from_min_max(
            Pos2::new(rect.min.x + 12.0, rect.min.y + 36.0),
            Pos2::new(rect.max.x - 12.0, rect.min.y + 115.0),
        );
        ui.painter()
            .rect_filled(preview_box, 4.0, Color32::from_rgb(46, 46, 50));

        // Interactive bezier handle visual
        let p1 = Pos2::new(preview_box.min.x + 30.0, preview_box.max.y - 20.0);
        let p2 = Pos2::new(preview_box.max.x - 30.0, preview_box.min.y + 20.0);
        ui.painter().line_segment(
            [p1, p2],
            Stroke::new(2.0_f32, Color32::from_rgb(20, 115, 230)),
        );
        ui.painter().circle_filled(p1, 4.0, Color32::WHITE);
        ui.painter().circle_filled(p2, 4.0, Color32::WHITE);

        ui.painter().text(
            Pos2::new(rect.min.x + 12.0, rect.min.y + 124.0),
            egui::Align2::LEFT_TOP,
            "はじめてのベクター作成",
            egui::FontId::proportional(11.5),
            Color32::WHITE,
        );
        ui.painter().text(
            Pos2::new(rect.min.x + 12.0, rect.min.y + 140.0),
            egui::Align2::LEFT_TOP,
            "基本的なツールの使い方を学べます。",
            egui::FontId::proportional(9.5),
            Color32::from_rgb(160, 160, 160),
        );

        // "チュートリアルを開く" Button
        let btn_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + 12.0, rect.max.y - 32.0),
            Pos2::new(rect.max.x - 12.0, rect.max.y - 8.0),
        );
        let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
        let bg_c = if btn_resp.hovered() {
            Color32::from_rgb(50, 50, 56)
        } else {
            Color32::from_rgb(40, 40, 44)
        };
        ui.painter().rect_filled(btn_rect, 12.0, bg_c);
        ui.painter().rect_stroke(
            btn_rect,
            12.0,
            Stroke::new(1.0_f32, Color32::from_rgb(70, 70, 76)),
            StrokeKind::Inside,
        );
        ui.painter().text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "チュートリアルを開く",
            egui::FontId::proportional(11.0),
            Color32::WHITE,
        );

        if btn_resp.clicked() {
            *tour_guide_open = true;
        }
    }

    fn show_tips_widget(&self, ui: &mut Ui) {
        let w = ui.available_width().min(265.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 160.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 6.0, Color32::from_rgb(33, 33, 36));

        ui.painter().text(
            Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0),
            egui::Align2::LEFT_TOP,
            "ヒントと情報",
            egui::FontId::proportional(12.5),
            Color32::WHITE,
        );
        ui.painter().text(
            Pos2::new(rect.max.x - 12.0, rect.min.y + 12.0),
            egui::Align2::RIGHT_TOP,
            "すべて表示 ➔",
            egui::FontId::proportional(10.0),
            Color32::from_rgb(170, 170, 170),
        );

        let tips = [
            ("図形の作成と編集", "基本のツールを使いこなす"),
            ("デザインを効率化するショートカット", "作業をスピードアップ"),
            ("カラーテーマの活用", "一貫したデザインに仕上げる"),
            ("クラウドで共同作業", "どこからでも、チームで制作"),
        ];

        let mut y = rect.min.y + 36.0;
        for (title, desc) in tips {
            let row_rect = Rect::from_min_max(
                Pos2::new(rect.min.x + 8.0, y),
                Pos2::new(rect.max.x - 8.0, y + 26.0),
            );
            // Minimal bullet indicator
            ui.painter().circle_filled(
                Pos2::new(row_rect.min.x + 8.0, row_rect.min.y + 8.0),
                2.5,
                Color32::from_rgb(20, 115, 230),
            );
            ui.painter().text(
                Pos2::new(row_rect.min.x + 18.0, row_rect.min.y + 2.0),
                egui::Align2::LEFT_TOP,
                title,
                egui::FontId::proportional(10.5),
                Color32::WHITE,
            );
            ui.painter().text(
                Pos2::new(row_rect.min.x + 18.0, row_rect.min.y + 14.0),
                egui::Align2::LEFT_TOP,
                desc,
                egui::FontId::proportional(9.0),
                Color32::from_rgb(140, 140, 140),
            );
            ui.painter().text(
                Pos2::new(row_rect.max.x - 6.0, row_rect.min.y + 6.0),
                egui::Align2::RIGHT_TOP,
                "›",
                egui::FontId::proportional(13.0),
                Color32::from_rgb(120, 120, 120),
            );
            y += 28.0;
        }
    }

    fn show_recent_projects_list(
        &mut self,
        ui: &mut Ui,
        _state: &mut AppState,
    ) -> Option<HomeAction> {
        let mut action = None;
        let w = ui.available_width().min(265.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 170.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 6.0, Color32::from_rgb(33, 33, 36));

        ui.painter().text(
            Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0),
            egui::Align2::LEFT_TOP,
            "最近開いたプロジェクト",
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );
        ui.painter().text(
            Pos2::new(rect.max.x - 12.0, rect.min.y + 12.0),
            egui::Align2::RIGHT_TOP,
            "すべて表示 ➔",
            egui::FontId::proportional(10.0),
            Color32::from_rgb(170, 170, 170),
        );

        let mut y = rect.min.y + 36.0;
        for file in self.recent_files.iter().take(5) {
            let row_rect = Rect::from_min_max(
                Pos2::new(rect.min.x + 8.0, y),
                Pos2::new(rect.max.x - 8.0, y + 24.0),
            );
            let resp = ui.allocate_rect(row_rect, egui::Sense::click());
            if resp.hovered() {
                ui.painter()
                    .rect_filled(row_rect, 3.0, Color32::from_rgb(44, 44, 48));
            }
            let doc_icon_box = Rect::from_center_size(
                Pos2::new(row_rect.min.x + 12.0, row_rect.center().y),
                Vec2::splat(12.0),
            );
            crate::app::icons::icon_document(ui.painter(), doc_icon_box, Color32::from_gray(170));
            ui.painter().text(
                Pos2::new(row_rect.min.x + 24.0, row_rect.center().y),
                egui::Align2::LEFT_CENTER,
                &file.title,
                egui::FontId::proportional(10.5),
                Color32::WHITE,
            );
            ui.painter().text(
                Pos2::new(row_rect.max.x - 8.0, row_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                &file.date,
                egui::FontId::proportional(9.0),
                Color32::from_rgb(130, 130, 130),
            );

            if resp.clicked() {
                action = Some(HomeAction::OpenFile(file.path.clone()));
            }
            y += 25.0;
        }
        action
    }
}
