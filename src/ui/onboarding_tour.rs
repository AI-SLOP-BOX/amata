use crate::core::state::AppState;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};

pub struct OnboardingTour {
    pub is_active: bool,
    pub step: usize, // 1 to 5
    pub completed_lessons: usize,
    pub is_panel_open: bool,
    pub startup_tour_checked: bool,
}

impl Default for OnboardingTour {
    fn default() -> Self {
        Self {
            is_active: false,
            step: 1,
            completed_lessons: 0,
            is_panel_open: true,
            startup_tour_checked: true,
        }
    }
}

impl OnboardingTour {
    pub fn show(&mut self, ctx: &egui::Context, _state: &mut AppState) {
        if !self.is_active && !self.is_panel_open {
            return;
        }

        let screen = ctx.screen_rect();

        // 1. Interactive 5-Step Guided Tour Bubbles overlaying target controls
        if self.is_active {
            egui::Area::new("onboarding_tour_overlay".into())
                .fixed_pos(Pos2::ZERO)
                .interactable(true)
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    let bubble = Vec2::new(210.0, 120.0);
                    let clamp = |p: Pos2| -> Pos2 {
                        Pos2::new(
                            p.x.clamp(8.0, (screen.max.x - bubble.x - 8.0).max(8.0)),
                            p.y.clamp(48.0, (screen.max.y - bubble.y - 36.0).max(48.0)),
                        )
                    };
                    match self.step {
                        1 => self.render_bubble(ui, clamp(Pos2::new(75.0, 130.0)), Pos2::new(35.0, 150.0), 1, "ツールパネル", "ここから描画や編集のツールを選びます。まずはペンツールを使ってみましょう。"),
                        2 => self.render_bubble(ui, clamp(Pos2::new(80.0, 310.0)), Pos2::new(35.0, 320.0), 2, "ペンツール  [P]", "アンカーポイントを打って、自由なパスを描くことができます。"),
                        3 => self.render_bubble(ui, clamp(Pos2::new(screen.center().x - 110.0, screen.center().y - 40.0)), Pos2::new(screen.center().x, screen.center().y - 80.0), 3, "アートボード", "ここにイラストやデザインを作成します。ズームや移動で、自由に作業できます。"),
                        4 => self.render_bubble(ui, clamp(Pos2::new(screen.max.x - 470.0, 230.0)), Pos2::new(screen.max.x - 300.0, 140.0), 4, "レイヤーパネル", "オブジェクトをレイヤーで管理できます。複雑なデザインも整理して編集できます。"),
                        5 => self.render_bubble(ui, clamp(Pos2::new(screen.max.x - 450.0, screen.max.y - 280.0)), Pos2::new(screen.max.x - 280.0, screen.max.y - 280.0), 5, "プロパティパネル", "選択したオブジェクトの設定をここで調整できます。"),
                        _ => {}
                    }
                });
        }

        // 2. Right "はじめてガイド" Panel (Illustrator Right Guide Panel)
        if self.is_panel_open {
            let panel_w = 260.0;
            let panel_rect = Rect::from_min_max(
                Pos2::new(screen.max.x - panel_w, 48.0),
                Pos2::new(screen.max.x, screen.max.y - 28.0),
            );

            egui::Area::new("onboarding_guide_panel".into())
                .fixed_pos(panel_rect.min)
                .order(egui::Order::Middle)
                .show(ctx, |ui| {
                    ui.painter().rect_filled(panel_rect, 0.0, Color32::from_rgb(32, 32, 34));
                    ui.painter().line_segment([panel_rect.left_top(), panel_rect.left_bottom()], Stroke::new(1.0_f32, Color32::from_rgb(50, 50, 52)));

                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(panel_rect.shrink(10.0)), |ui| {
                        // Header
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("はじめてガイド").strong().size(13.5).color(Color32::WHITE));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("✕").clicked() {
                                    self.is_panel_open = false;
                                }
                            });
                        });

                        ui.add_space(4.0);
                        ui.label(RichText::new("基本操作を学んで、すぐにデザインを始めましょう。このチュートリアルでは、描画の基本から書き出しまでをステップごとに案内します。").size(10.0).color(Color32::from_rgb(160, 160, 160)));

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("学習の進捗").size(10.5).color(Color32::from_rgb(170, 170, 170)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(format!("{}/6", self.completed_lessons)).strong().size(10.5).color(Color32::WHITE));
                            });
                        });

                        ui.add_space(8.0);

                        // Lessons + hint need scrolling on short windows.
                        egui::ScrollArea::vertical()
                            .id_salt("onboarding_lessons_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                        // Lessons list
                        let lessons = [
                            (1, "作業画面の基本", "ツール、パネル、アートボードの\n役割を知ろう", "2分", true),
                            (2, "図形を描いてみよう", "基本的な図形の作成と編集", "3分", false),
                            (3, "ペンツールで描く", "パスを使って自由な形を作成", "4分", false),
                            (4, "オブジェクトを編集する", "選択・変形・整列の基本", "4分", false),
                            (5, "レイヤーを使いこなす", "整理して効率的に作業", "3分", false),
                            (6, "書き出して完成！", "画像やPDFで書き出す", "2分", false),
                        ];

                        for (num, title, sub, dur, active) in lessons {
                            let (l_rect, l_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 48.0), egui::Sense::click());
                            let bg_c = if active { Color32::from_rgb(22, 50, 85) } else { Color32::from_rgb(38, 38, 40) };
                            let border_c = if active { Color32::from_rgb(20, 115, 230) } else { Color32::from_rgb(50, 50, 52) };
                            ui.painter().rect_filled(l_rect, 4.0, bg_c);
                            ui.painter().rect_stroke(l_rect, 4.0, Stroke::new(1.0_f32, border_c), StrokeKind::Inside);

                            // Number / Icon
                            let icon_rect = Rect::from_min_size(l_rect.min + Vec2::new(6.0, 8.0), Vec2::splat(18.0));
                            if active {
                                ui.painter().circle_filled(icon_rect.center(), 9.0, Color32::from_rgb(20, 115, 230));
                                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, "▶", egui::FontId::proportional(9.0), Color32::WHITE);
                            } else {
                                ui.painter().circle_stroke(icon_rect.center(), 8.0, Stroke::new(1.0_f32, Color32::from_gray(100)));
                                ui.painter().text(icon_rect.center(), egui::Align2::CENTER_CENTER, format!("{}", num), egui::FontId::proportional(9.0), Color32::from_gray(160));
                            }

                            // Title & Subtitle
                            ui.painter().text(Pos2::new(l_rect.min.x + 30.0, l_rect.min.y + 7.0), egui::Align2::LEFT_TOP, title, egui::FontId::proportional(11.0), Color32::WHITE);
                            ui.painter().text(Pos2::new(l_rect.min.x + 30.0, l_rect.min.y + 22.0), egui::Align2::LEFT_TOP, sub.lines().next().unwrap_or(""), egui::FontId::proportional(9.0), Color32::from_rgb(150, 150, 150));

                            // Duration / Lock
                            if active {
                                 ui.painter().text(Pos2::new(l_rect.min.x + 8.0, l_rect.max.y - 12.0), egui::Align2::LEFT_BOTTOM, dur, egui::FontId::proportional(9.0), Color32::from_rgb(140, 180, 230));
                            } else {
                                let lock_box = Rect::from_center_size(Pos2::new(l_rect.max.x - 16.0, l_rect.center().y), Vec2::splat(14.0));
                                crate::app::icons::icon_lock(ui.painter(), lock_box, Color32::from_gray(120));
                            }

                            if l_resp.clicked() {
                                self.is_active = true;
                                self.step = 1;
                            }
                            ui.add_space(4.0);
                        }

                        ui.add_space(10.0);

                        // One-point Hint card
                        let (h_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 90.0), egui::Sense::hover());
                        ui.painter().rect_filled(h_rect, 4.0, Color32::from_rgb(38, 38, 40));

                        // Bulb vector icon
                        let bulb_box = Rect::from_min_size(Pos2::new(h_rect.min.x + 8.0, h_rect.min.y + 6.0), Vec2::splat(16.0));
                        crate::app::icons::icon_bulb(ui.painter(), bulb_box, Color32::from_rgb(250, 200, 80));
                        ui.painter().text(Pos2::new(h_rect.min.x + 28.0, h_rect.min.y + 8.0), egui::Align2::LEFT_TOP, "ワンポイントヒント", egui::FontId::proportional(11.0), Color32::from_rgb(250, 200, 80));
                        ui.painter().text(Pos2::new(h_rect.min.x + 8.0, h_rect.min.y + 26.0), egui::Align2::LEFT_TOP, "ツールにマウスを合わせると、\nショートカットキーが表示されます。\nキーボードを活用すると、\nよりスムーズに作業できます。", egui::FontId::proportional(9.5), Color32::from_rgb(160, 160, 160));

                        let link_btn = Rect::from_min_max(Pos2::new(h_rect.max.x - 90.0, h_rect.max.y - 18.0), Pos2::new(h_rect.max.x - 8.0, h_rect.max.y - 4.0));
                        ui.painter().text(link_btn.center(), egui::Align2::CENTER_CENTER, "さらに詳しく見る ↗", egui::FontId::proportional(9.0), Color32::from_rgb(180, 180, 180));
                            });
                    });
                });
        }
    }

    fn render_bubble(
        &mut self,
        ui: &mut Ui,
        bubble_pos: Pos2,
        target_pos: Pos2,
        step_num: usize,
        title: &str,
        body: &str,
    ) {
        let bubble_size = Vec2::new(210.0, 120.0);
        let rect = Rect::from_min_size(bubble_pos, bubble_size);

        // Connecting Indicator Curve / Line with dot
        ui.painter()
            .circle_filled(target_pos, 4.5, Color32::from_rgb(20, 115, 230));
        ui.painter()
            .circle_stroke(target_pos, 5.5, Stroke::new(1.5_f32, Color32::WHITE));
        ui.painter().line_segment(
            [target_pos, rect.left_center()],
            Stroke::new(1.5_f32, Color32::from_rgb(20, 115, 230)),
        );

        // Bubble Frame (Navy / Charcoal with Blue outline)
        ui.painter()
            .rect_filled(rect, 8.0, Color32::from_rgb(28, 32, 42));
        ui.painter().rect_stroke(
            rect,
            8.0,
            Stroke::new(1.5_f32, Color32::from_rgb(20, 115, 230)),
            StrokeKind::Outside,
        );

        // Step pill
        let num_rect = Rect::from_min_size(rect.min + Vec2::new(10.0, 10.0), Vec2::splat(18.0));
        ui.painter()
            .circle_filled(num_rect.center(), 9.0, Color32::from_rgb(20, 115, 230));
        ui.painter().text(
            num_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}", step_num),
            egui::FontId::proportional(11.0),
            Color32::WHITE,
        );

        // Title
        ui.painter().text(
            Pos2::new(rect.min.x + 34.0, rect.min.y + 12.0),
            egui::Align2::LEFT_TOP,
            title,
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );

        // Body: wrap to the bubble width instead of a fixed char split,
        // which used to dump the entire remainder onto line2.
        let body_font = egui::FontId::proportional(10.0);
        let body_color = Color32::from_rgb(200, 200, 200);
        let body_rect = Rect::from_min_size(
            Pos2::new(rect.min.x + 10.0, rect.min.y + 36.0),
            Vec2::new(rect.width() - 20.0, 44.0),
        );
        let body_galley = ui.fonts(|f| {
            f.layout(body.to_string(), body_font, body_color, body_rect.width())
        });
        ui.painter().galley(body_rect.min, body_galley, body_color);

        // Progress Text "1/5"
        ui.painter().text(
            Pos2::new(rect.min.x + 12.0, rect.max.y - 20.0),
            egui::Align2::LEFT_CENTER,
            format!("{}/5", step_num),
            egui::FontId::proportional(10.0),
            Color32::from_rgb(140, 140, 140),
        );

        // Buttons
        let mut btn_x = rect.max.x - 12.0;

        // Next / Finish Button
        let next_text = if step_num == 5 { "完了" } else { "次へ >" };
        let next_rect = Rect::from_min_max(
            Pos2::new(btn_x - 52.0, rect.max.y - 28.0),
            Pos2::new(btn_x, rect.max.y - 8.0),
        );
        let next_resp = ui.allocate_rect(next_rect, egui::Sense::click());
        ui.painter()
            .rect_filled(next_rect, 4.0, Color32::from_rgb(20, 115, 230));
        ui.painter().text(
            next_rect.center(),
            egui::Align2::CENTER_CENTER,
            next_text,
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );

        if next_resp.clicked() {
            if self.step < 5 {
                self.step += 1;
            } else {
                self.is_active = false;
                self.completed_lessons = (self.completed_lessons + 1).min(6);
            }
        }
        btn_x -= 58.0;

        // Back Button
        if step_num > 1 {
            let back_rect = Rect::from_min_max(
                Pos2::new(btn_x - 46.0, rect.max.y - 28.0),
                Pos2::new(btn_x, rect.max.y - 8.0),
            );
            let back_resp = ui.allocate_rect(back_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(back_rect, 4.0, Color32::from_rgb(46, 46, 50));
            ui.painter().text(
                back_rect.center(),
                egui::Align2::CENTER_CENTER,
                "戻る",
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
            if back_resp.clicked() {
                self.step -= 1;
            }
        }
    }
}
