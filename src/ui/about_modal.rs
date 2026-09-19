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

        egui::Window::new("Amata について")
            .open(&mut is_open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size(Vec2::new(420.0, 480.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    // Official Amata Vector Emblem Logo
                    let (logo_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(110.0), egui::Sense::hover());
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
                        ui.add_space(20.0);
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
                            ui.set_width(360.0);
                            egui::Grid::new("about_shortcuts_grid")
                                .num_columns(2)
                                .spacing([24.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("V / A")
                                            .strong()
                                            .color(Color32::from_rgb(0, 210, 255)),
                                    );
                                    ui.label(
                                        RichText::new("選択 / ダイレクト選択")
                                            .size(11.0)
                                            .color(Color32::from_gray(200)),
                                    );
                                    ui.end_row();

                                    ui.label(
                                        RichText::new("P / B")
                                            .strong()
                                            .color(Color32::from_rgb(0, 210, 255)),
                                    );
                                    ui.label(
                                        RichText::new("ペンツール / ブラシ")
                                            .size(11.0)
                                            .color(Color32::from_gray(200)),
                                    );
                                    ui.end_row();

                                    ui.label(
                                        RichText::new("M / L")
                                            .strong()
                                            .color(Color32::from_rgb(0, 210, 255)),
                                    );
                                    ui.label(
                                        RichText::new("長方形 / 楕円")
                                            .size(11.0)
                                            .color(Color32::from_gray(200)),
                                    );
                                    ui.end_row();

                                    ui.label(
                                        RichText::new("Space + ドラッグ")
                                            .strong()
                                            .color(Color32::from_rgb(0, 210, 255)),
                                    );
                                    ui.label(
                                        RichText::new("手のひらツール（キャンバス移動）")
                                            .size(11.0)
                                            .color(Color32::from_gray(200)),
                                    );
                                    ui.end_row();

                                    ui.label(
                                        RichText::new("Cmd+Z / Cmd+Shift+Z")
                                            .strong()
                                            .color(Color32::from_rgb(0, 210, 255)),
                                    );
                                    ui.label(
                                        RichText::new("元に戻す / やり直す")
                                            .size(11.0)
                                            .color(Color32::from_gray(200)),
                                    );
                                    ui.end_row();
                                });
                        });

                    ui.add_space(16.0);
                    if ui.button(RichText::new("閉じる").size(12.0)).clicked() {
                        close_clicked = true;
                    }
                    ui.add_space(8.0);
                });
            });

        if close_clicked {
            self.is_open = false;
        } else {
            self.is_open = is_open;
        }
    }
}
