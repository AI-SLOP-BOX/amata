use super::{ActiveTab, IrasuApp};
use crate::app::icons::paint_focus_ring;
use crate::ui::panels::*;
use egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

impl IrasuApp {
    pub(super) fn show_right_dock(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("properties")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                let more_resp = self.show_dock_tab_strip(ui);

                // Extra panels in popup
                egui::popup::popup_below_widget(
                    ui,
                    egui::Id::new("dock_more_popup"),
                    &more_resp,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.set_min_width(160.0);
                        for (label, variant) in &[
                            ("パスファインダー", ActiveTab::Pathfinder),
                            ("3D & VFX", ActiveTab::ThreeDAndVfx),
                            ("ジェネレーティブ幾何学", ActiveTab::Generative),
                            ("ドット絵", ActiveTab::PixelArt),
                            ("アセット書き出し", ActiveTab::Export),
                            ("スマートガイド設定", ActiveTab::Guides),
                        ] {
                            if ui
                                .selectable_label(self.active_tab == *variant, *label)
                                .clicked()
                            {
                                self.active_tab = variant.clone();
                                ui.close_menu();
                            }
                        }
                    },
                );

                // ─── Panel Content ───────────────────────────────────────────
                egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
                    ActiveTab::Properties => {
                        ui.collapsing("プロパティ", |ui| {
                            PropertyPanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("整列", |ui| {
                            AlignPanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("線", |ui| {
                            StrokePanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("グラデーション", |ui| {
                            GradientPanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("合成モード", |ui| {
                            BlendModePanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("変形", |ui| {
                            TransformPanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("アピアランス", |ui| {
                            AppearancePanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("テキスト", |ui| {
                            TextPanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("線幅ツール", |ui| {
                            WidthToolPanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("パターン", |ui| {
                            PatternPanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("プリセット", |ui| {
                            PresetPanel::show(ui, &mut self.state);
                        });
                        ui.collapsing("カラーハーモニー", |ui| {
                            ColorHarmonyPanel::show(ui, &mut self.state);
                        });
                    }
                    ActiveTab::Pathfinder => {
                        PathfinderPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        ClippingMaskPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        KnifePanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        EnvelopePanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        OffsetPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        MorphPanel::show(ui, &mut self.state);
                    }
                    ActiveTab::ThreeDAndVfx => {
                        RevolvePanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        NeonGlowPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        VfxTrailPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        EffectsPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        TracePanel::show(ui, &mut self.state);
                    }
                    ActiveTab::Generative => {
                        SwatchesPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        GradientMeshPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        PolarPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        AudioWavePanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        FlowFieldPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        DeformPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        ScatterBrushPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        BrushPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        MeshWarpPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        VoronoiPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        LSystemPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        HalftonePanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        IsometricPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        SymmetryPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        FormulaPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        QrCodePanel::show(ui, &mut self.state);
                    }
                    ActiveTab::Layers => {
                        LayerPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        HistoryPanel::show(ui, &mut self.state);
                    }
                    ActiveTab::Symbols => {
                        SymbolsPanel::show(ui, &mut self.state);
                    }
                    ActiveTab::Components => {
                        ComponentPanel::show(ui, &mut self.state);
                    }
                    ActiveTab::VersionHistory => {
                        self.version_history_panel.show(ui, &mut self.state);
                    }
                    ActiveTab::Export => {
                        ExportPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        PrintPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        GridRepeatPanel::show(ui, &mut self.state);
                    }
                    ActiveTab::Guides => {
                        SmartGuidesPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        LayoutGridPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        AutoLayoutPanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        PerspectivePanel::show(ui, &mut self.state);
                        ui.add_space(8.0);
                        ui.separator();
                        ShortcutsHelpPanel::show(ui, &mut self.state);
                    }
                    ActiveTab::PixelArt => {
                        PixelPanel::show(ui, &mut self.state);
                    }
                });
            });
    }
    /// Tab strip of the right dock. Returns the overflow response
    /// so the caller can anchor the extra-panels popup to it.
    fn show_dock_tab_strip(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let tab_h = 28.0;
        let (strip_rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), tab_h), egui::Sense::hover());

        // Strip background
        ui.painter()
            .rect_filled(strip_rect, 0.0, Color32::from_rgb(26, 26, 26));

        // Tab definitions: (label, ActiveTab variant)
        let tabs: &[(&str, ActiveTab)] = &[
            ("プロパティ", ActiveTab::Properties),
            ("レイヤー", ActiveTab::Layers),
            ("コンポーネント", ActiveTab::Components),
            ("変更履歴", ActiveTab::VersionHistory),
        ];

        let accent = Color32::from_rgb(20, 115, 230);
        let tab_font = egui::FontId::proportional(11.5);
        let more_w = 28.0_f32;
        let avail = strip_rect.width();
        let mut tab_widths: Vec<f32> = tabs
            .iter()
            .map(|(label, _)| {
                let tw = ui.fonts(|f| {
                    f.layout_no_wrap((*label).to_string(), tab_font.clone(), Color32::WHITE)
                        .size()
                        .x
                });
                (tw + 24.0).max(56.0)
            })
            .collect();

        // Drop trailing tabs when the strip would collide with ⋯.
        let max_tabs = {
            let mut used = 0.0_f32;
            let mut n = 0;
            for w in &tab_widths {
                if used + *w > avail - more_w - 4.0 {
                    break;
                }
                used += *w;
                n += 1;
            }
            n.max(1).min(tabs.len())
        };
        tab_widths.truncate(max_tabs);

        let mut x = strip_rect.min.x;
        for ((label, variant), tab_w) in tabs.iter().zip(tab_widths.iter()) {
            let tab_rect =
                Rect::from_min_size(Pos2::new(x, strip_rect.min.y), Vec2::new(*tab_w, tab_h));
            x += *tab_w;
            let is_active = self.active_tab == *variant;

            // Hit test & hover
            let resp = ui
                .allocate_rect(tab_rect, egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            paint_focus_ring(ui, &resp, tab_rect);

            let text_color = if is_active {
                Color32::WHITE
            } else if resp.hovered() {
                Color32::from_gray(210)
            } else {
                Color32::from_gray(150)
            };

            // Hover bg
            if resp.hovered() && !is_active {
                ui.painter()
                    .rect_filled(tab_rect, 0.0, Color32::from_rgb(40, 40, 40));
            }

            // Tab label
            ui.painter().text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                tab_font.clone(),
                text_color,
            );

            // Active underline bar, animated: each tab lerps its own
            // activeness so switching tabs grows/shrinks the bar smoothly.
            let glow = ui.ctx().animate_bool_with_time(
                egui::Id::new(("dock_tab_active", *label)),
                is_active,
                0.18,
            );
            if glow > 0.01 {
                let w = (tab_rect.width() - 8.0) * glow;
                let x = tab_rect.center().x - w / 2.0;
                ui.painter().rect_filled(
                    Rect::from_min_size(Pos2::new(x, tab_rect.max.y - 2.0), Vec2::new(w, 2.0)),
                    1.0,
                    accent.lerp_to_gamma(Color32::TRANSPARENT, 1.0 - glow),
                );
            }

            if resp.clicked() {
                self.active_tab = variant.clone();
            }
        }

        // More panels dropdown (⋯) — right-aligned
        let more_rect = Rect::from_min_size(
            Pos2::new(strip_rect.max.x - 28.0, strip_rect.min.y),
            Vec2::new(28.0, tab_h),
        );
        ui.painter().text(
            more_rect.center(),
            egui::Align2::CENTER_CENTER,
            "⋯",
            egui::FontId::proportional(13.0),
            Color32::from_gray(140),
        );
        let more_resp = ui.allocate_rect(more_rect, egui::Sense::click());
        if more_resp.clicked() {
            // popup_below_widget only opens when memory toggles it.
            ui.ctx().memory_mut(|m| {
                m.toggle_popup(egui::Id::new("dock_more_popup"));
            });
        }

        // Separator below tabs
        ui.painter().line_segment(
            [
                Pos2::new(strip_rect.min.x, strip_rect.max.y),
                Pos2::new(strip_rect.max.x, strip_rect.max.y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(48, 48, 48)),
        );
        ui.add_space(tab_h + 1.0);
        more_resp
    }
}
