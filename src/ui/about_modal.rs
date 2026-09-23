use crate::app::icons::icon_amata_logo;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

#[derive(Default)]
pub struct AboutModal {
    pub is_open: bool,
}

impl AboutModal {
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        let mut is_open = self.is_open;
        let mut close_clicked = false;

        let screen = ctx.screen_rect();
        let (size, min_size, pos) = crate::ui::window_defaults(
            screen,
            Vec2::new(420.0, 480.0),
            Vec2::new(300.0, 240.0),
        );
        egui::Window::new("Amata について")
            .open(&mut is_open)
            .resizable(true)
            .collapsible(false)
            .default_pos(pos)
            .default_size(size)
            .min_size(min_size)
            .show(ctx, |ui| {
                let total_w = ui.available_width();
                crate::ui::modal_body(ui, "about_body", &mut |ui, is_body| {
                    if !is_body {
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("閉じる").strong().color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(20, 115, 230))
                                    .min_size(Vec2::new(96.0, 28.0)),
                                )
                                .clicked()
                            {
                                close_clicked = true;
                            }
                        });
                        return;
                    }
                    ui.set_width(total_w);
                    ui.vertical_centered(|ui| {
                            ui.add_space(12.0);

                            // Official Amata Vector Emblem Logo
                            let (logo_rect, _) = ui
                                .allocate_exact_size(Vec2::splat(110.0), egui::Sense::hover());
                            let p = ui.painter();
                            // Dark emblem card background
                            p.rect_filled(logo_rect, 16.0, Color32::from_rgb(26, 26, 32));
                            p.rect_stroke(
                                logo_rect,
                                16.0,
                                Stroke::new(1.0_f32, Color32::from_rgb(50, 50, 62)),
                                egui::StrokeKind::Outside,
                            );
                            icon_amata_logo(p, logo_rect.shrink(10.0));

                            ui.add_space(14.0);
                            ui.label(
                                RichText::new("Amata (数多)")
                                    .size(22.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.label(
                                RichText::new("次世代ベクターグラフィック & デザインツール")
                                    .size(12.5)
                                    .color(Color32::from_rgb(160, 160, 180)),
                            );
                            ui.label(
                                RichText::new("Version 0.1.0 (Rust & wgpu Native)")
                                    .size(11.0)
                                    .color(Color32::from_rgb(120, 120, 140)),
                            );

                            ui.add_space(16.0);
                            ui.separator();
                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(
                                        "Amata（数多）は、プロフェッショナルなクリエイターのための\n\
                                         高速・高精度なベクターイラストレーション & デザイン環境です。\n\
                                         ベジェ曲線編集、ブーリアン演算、3D VFX、レイヤー合成、\n\
                                         SVG/PNG/PDF書き出し、CLI自動化などをフルサポート。",
                                    )
                                    .size(11.5)
                                    .color(Color32::from_rgb(200, 200, 210)),
                                );
                            });

                            ui.add_space(16.0);

                            // Key Shortcuts Cheat Sheet Box
                            egui::Frame::NONE
                                .fill(Color32::from_rgb(20, 20, 24))
                                .corner_radius(6.0)
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(40, 40, 48)))
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    let grid_w = ui.available_width();
                                    egui::Grid::new("about_shortcuts_grid")
                                        .num_columns(2)
                                        .spacing([24.0, 6.0])
                                        .min_col_width((grid_w * 0.32).max(90.0))
                                        .show(ui, |ui| {
                                            let undo_keys = format!(
                                                "{}+Z / {}+Shift+Z",
                                                crate::app::control_bar::mod_key(),
                                                crate::app::control_bar::mod_key()
                                            );
                                            let rows: [(&str, &str); 5] = [
                                                ("V / A", "選択 / ダイレクト選択"),
                                                ("P / B", "ペンツール / ブラシ"),
                                                ("M / L", "シェイプビルダー / 直線"),
                                                (
                                                    "Space + ドラッグ",
                                                    "手のひらツール（キャンバス移動）",
                                                ),
                                                (&undo_keys, "元に戻す / やり直す"),
                                            ];
                                            for (keys, desc) in rows {
                                                ui.label(
                                                    RichText::new(keys)
                                                        .strong()
                                                        .color(Color32::from_rgb(0, 210, 255)),
                                                );
                                                ui.label(
                                                    RichText::new(desc)
                                                        .size(11.0)
                                                        .color(Color32::from_gray(200)),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });

                            ui.add_space(8.0);
                        });
                });
            });

        if close_clicked {
            self.is_open = false;
        } else {
            self.is_open = is_open;
        }
    }
}
