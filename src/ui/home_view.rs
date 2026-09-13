use crate::core::state::AppState;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};

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
        Self {
            is_open: true, // Show Home by default like CC
            current_tab: HomeSidebarTab::Home,
            preset_cat: HomePresetCategory::Recommend,
            search_query: String::new(),
            recent_files: vec![
                RecentFileItem {
                    title: "ブランドガイド.ai".into(),
                    date: "昨日 14:32".into(),
                    dimensions: (1920.0, 1080.0),
                    color: Color32::from_rgb(220, 110, 60),
                    accent: Color32::from_rgb(240, 190, 80),
                },
                RecentFileItem {
                    title: "リーフレットデザイン.ai".into(),
                    date: "3日前 11:20".into(),
                    dimensions: (210.0, 297.0),
                    color: Color32::from_rgb(80, 150, 100),
                    accent: Color32::from_rgb(120, 190, 140),
                },
                RecentFileItem {
                    title: "ポスター案.ai".into(),
                    date: "4日前 16:05".into(),
                    dimensions: (297.0, 420.0),
                    color: Color32::from_rgb(60, 110, 150),
                    accent: Color32::from_rgb(180, 210, 220),
                },
                RecentFileItem {
                    title: "SNSバナー.ai".into(),
                    date: "6日前 9:18".into(),
                    dimensions: (1080.0, 1080.0),
                    color: Color32::from_rgb(40, 60, 140),
                    accent: Color32::from_rgb(180, 160, 200),
                },
                RecentFileItem {
                    title: "パッケージ展開図.ai".into(),
                    date: "7日前 13:41".into(),
                    dimensions: (350.0, 240.0),
                    color: Color32::from_rgb(190, 170, 150),
                    accent: Color32::from_rgb(220, 200, 180),
                },
                RecentFileItem {
                    title: "ロゴバリエーション.ai".into(),
                    date: "1週間前".into(),
                    dimensions: (800.0, 600.0),
                    color: Color32::from_rgb(50, 50, 50),
                    accent: Color32::from_rgb(240, 240, 240),
                },
                RecentFileItem {
                    title: "イラスト_風景.ai".into(),
                    date: "1週間前".into(),
                    dimensions: (1920.0, 1080.0),
                    color: Color32::from_rgb(70, 120, 160),
                    accent: Color32::from_rgb(120, 170, 130),
                },
                RecentFileItem {
                    title: "パターン素材.ai".into(),
                    date: "1週間前".into(),
                    dimensions: (500.0, 500.0),
                    color: Color32::from_rgb(210, 130, 130),
                    accent: Color32::from_rgb(240, 200, 160),
                },
            ],
        }
    }
}

impl HomeView {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        state: &mut AppState,
        new_doc_modal: &mut crate::ui::NewDocModal,
        tour_guide_open: &mut bool,
    ) {
        if !self.is_open {
            return;
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(26, 26, 26)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left Global Navigation Bar (180px)
                    self.show_sidebar(ui, new_doc_modal);

                    ui.add_space(8.0);

                    // Central & Right Scrollable Area
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.horizontal_top(|ui| {
                                // Main Content (Welcome, New Doc Presets, Recent Files)
                                ui.vertical(|ui| {
                                    ui.set_max_width(780.0);
                                    self.show_welcome_banner(ui);
                                    ui.add_space(16.0);
                                    self.show_preset_section(ui, state);
                                    ui.add_space(20.0);
                                    self.show_recent_files(ui, state);
                                });

                                ui.add_space(16.0);

                                // Right Information Panel (Smooth workflow, tips, recent list)
                                ui.vertical(|ui| {
                                    ui.set_width(280.0);
                                    self.show_tutorial_widget(ui, tour_guide_open);
                                    ui.add_space(14.0);
                                    self.show_tips_widget(ui);
                                    ui.add_space(14.0);
                                    self.show_recent_projects_list(ui, state);
                                });
                            });
                        });
                });
            });
    }

    fn show_sidebar(&mut self, ui: &mut Ui, new_doc_modal: &mut crate::ui::NewDocModal) {
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
                // Open file dialog
            }
        });
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

    fn show_preset_section(&mut self, ui: &mut Ui, state: &mut AppState) {
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

        // 6 Preset Cards
        let presets = [
            ("doc", "A4", "210 × 297 mm", 595.0, 842.0),
            ("doc", "US レター", "8.5 × 11 in", 612.0, 792.0),
            ("desktop", "Web (横長)", "1920 × 1080 px", 1920.0, 1080.0),
            ("phone", "iPhone 15", "1179 × 2556 px", 1179.0, 2556.0),
            ("camera", "Instagram 投稿", "1080 × 1080 px", 1080.0, 1080.0),
            ("more", "その他のプリセット", "カスタム", 800.0, 600.0),
        ];

        ui.horizontal(|ui| {
            for (icon_type, title, dim, w, h) in presets {
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(115.0, 110.0), egui::Sense::click());
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
                    state.document.width = w;
                    state.document.height = h;
                    self.is_open = false; // Enter canvas
                }
            }
        });
    }

    fn show_recent_files(&mut self, ui: &mut Ui, state: &mut AppState) {
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

        // 2 rows of 4 cards
        egui::Grid::new("recent_files_grid")
            .spacing(Vec2::new(14.0, 14.0))
            .show(ui, |ui| {
                for (i, file) in self.recent_files.iter().enumerate() {
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(175.0, 140.0), egui::Sense::click());
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
                        state.document.width = file.dimensions.0;
                        state.document.height = file.dimensions.1;
                        self.is_open = false; // Open into workspace
                    }

                    if (i + 1) % 4 == 0 {
                        ui.end_row();
                    }
                }
            });
    }

    fn show_tutorial_widget(&self, ui: &mut Ui, tour_guide_open: &mut bool) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(265.0, 200.0), egui::Sense::hover());
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
        let (rect, _) = ui.allocate_exact_size(Vec2::new(265.0, 160.0), egui::Sense::hover());
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

    fn show_recent_projects_list(&self, ui: &mut Ui, _state: &mut AppState) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(265.0, 170.0), egui::Sense::hover());
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

        let projects = [
            ("ブランドガイド.ai", "昨日 14:32"),
            ("リーフレットデザイン.ai", "3日前 11:20"),
            ("ポスター案.ai", "4日前 16:05"),
            ("SNSバナー.ai", "6日前 9:18"),
            ("パッケージ展開図.ai", "7日前 13:41"),
        ];

        let mut y = rect.min.y + 36.0;
        for (name, date) in projects {
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
                name,
                egui::FontId::proportional(10.5),
                Color32::WHITE,
            );
            ui.painter().text(
                Pos2::new(row_rect.max.x - 8.0, row_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                date,
                egui::FontId::proportional(9.0),
                Color32::from_rgb(130, 130, 130),
            );

            if resp.clicked() {
                // Open project
            }
            y += 25.0;
        }
    }
}
