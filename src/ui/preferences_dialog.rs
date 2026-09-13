use crate::core::state::AppState;
use eframe::egui::{self, Color32, RichText, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefCategory {
    General,
    Interface,
    Performance,
    FileHandling,
    GuidesAndGrid,
    Units,
    Type,
    Plugins,
    Shortcuts,
}

pub struct PreferencesDialog {
    pub is_open: bool,
    pub selected_category: PrefCategory,
    // Form fields mirroring Illustrator CC General preferences
    pub show_home_on_startup: bool,
    pub open_last_doc: bool,
    pub show_new_doc_dialog: bool,
    pub sync_cloud_docs: bool,
    pub recent_files_count: usize,
    pub history_states_count: usize,
    pub warn_on_low_memory: bool,
    pub show_bounding_box: bool,
    pub show_anchor_points: bool,
    pub show_tool_hints: bool,
    pub show_smart_guides_on_transform: bool,
    pub anchor_point_size: f32,
    pub handle_size: f32,
    pub selection_line_width: f32,
    pub point_handle_color_mode: String,
    pub color_theme: String,
    pub ui_scale: String,
    pub artboard_bg_mode: String,
    pub show_boundary_lines: bool,
    pub show_dimension_labels: bool,
    pub antialias_artwork: bool,
    pub notify_file_compat: bool,
    pub notify_font_substitute: bool,
    pub notify_plugin_load: bool,
    pub notify_perf_recommend: bool,
}

impl Default for PreferencesDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            selected_category: PrefCategory::General,
            show_home_on_startup: true,
            open_last_doc: true,
            show_new_doc_dialog: true,
            sync_cloud_docs: true,
            recent_files_count: 20,
            history_states_count: 100,
            warn_on_low_memory: true,
            show_bounding_box: true,
            show_anchor_points: true,
            show_tool_hints: true,
            show_smart_guides_on_transform: true,
            anchor_point_size: 4.0,
            handle_size: 4.0,
            selection_line_width: 1.0,
            point_handle_color_mode: "デフォルト".to_string(),
            color_theme: "ダーク".to_string(),
            ui_scale: "100%".to_string(),
            artboard_bg_mode: "ホワイト".to_string(),
            show_boundary_lines: true,
            show_dimension_labels: true,
            antialias_artwork: true,
            notify_file_compat: true,
            notify_font_substitute: true,
            notify_plugin_load: true,
            notify_perf_recommend: false,
        }
    }
}

impl PreferencesDialog {
    pub fn show(&mut self, ctx: &egui::Context, _state: &mut AppState) {
        if !self.is_open {
            return;
        }

        let mut is_open = self.is_open;
        egui::Window::new("環境設定")
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(760.0, 560.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left category sidebar
                    ui.vertical(|ui| {
                        ui.set_width(170.0);
                        let categories = [
                            (PrefCategory::General, "一般"),
                            (PrefCategory::Interface, "インターフェース"),
                            (PrefCategory::Performance, "パフォーマンス"),
                            (PrefCategory::FileHandling, "ファイルの取り扱い"),
                            (PrefCategory::GuidesAndGrid, "ガイド・グリッド"),
                            (PrefCategory::Units, "単位"),
                            (PrefCategory::Type, "書式 (テキスト)"),
                            (PrefCategory::Plugins, "プラグイン"),
                            (PrefCategory::Shortcuts, "ショートカット"),
                        ];

                        for (cat, label) in categories {
                            let is_sel = self.selected_category == cat;
                            let btn = if is_sel {
                                egui::Button::new(RichText::new(label).strong().size(12.0).color(Color32::WHITE))
                                    .fill(Color32::from_rgb(20, 115, 230))
                                    .min_size(Vec2::new(165.0, 26.0))
                            } else {
                                egui::Button::new(RichText::new(label).size(12.0).color(Color32::from_rgb(200, 200, 200)))
                                    .fill(Color32::TRANSPARENT)
                                    .min_size(Vec2::new(165.0, 26.0))
                            };

                            if ui.add(btn).clicked() {
                                self.selected_category = cat;
                            }
                        }

                        ui.add_space(90.0);
                        if ui.button(RichText::new("すべての環境設定をリセット").size(10.5)).clicked() {
                            *self = Self::default();
                            self.is_open = true;
                        }
                    });

                    ui.separator();

                    // Right Content Area
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                            match self.selected_category {
                                PrefCategory::General => {
                                    ui.label(RichText::new("一般").strong().size(14.0));
                                    ui.add_space(6.0);

                                    // 起動・作業環境
                                    ui.label(RichText::new("起動・作業環境").strong().size(11.5).color(Color32::from_rgb(180, 180, 180)));
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.set_width(260.0);
                                            ui.checkbox(&mut self.show_home_on_startup, "起動時にホームを表示");
                                            ui.checkbox(&mut self.open_last_doc, "前回のドキュメントを開く");
                                            ui.checkbox(&mut self.show_new_doc_dialog, "新規ドキュメントを作成するときに設定ダイアログを表示");
                                            ui.checkbox(&mut self.sync_cloud_docs, "クラウドドキュメントを自動的に同期");
                                        });
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("最近使用したファイルの表示数:").size(11.0));
                                                egui::ComboBox::from_id_salt("recent_files")
                                                    .selected_text(format!("{}", self.recent_files_count))
                                                    .width(60.0)
                                                    .show_ui(ui, |ui| {
                                                        ui.selectable_value(&mut self.recent_files_count, 10, "10");
                                                        ui.selectable_value(&mut self.recent_files_count, 20, "20");
                                                        ui.selectable_value(&mut self.recent_files_count, 50, "50");
                                                    });
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("取り消しの回数 (ヒストリー数):").size(11.0));
                                                egui::ComboBox::from_id_salt("history_count")
                                                    .selected_text(format!("{}", self.history_states_count))
                                                    .width(60.0)
                                                    .show_ui(ui, |ui| {
                                                        ui.selectable_value(&mut self.history_states_count, 50, "50");
                                                        ui.selectable_value(&mut self.history_states_count, 100, "100");
                                                        ui.selectable_value(&mut self.history_states_count, 200, "200");
                                                    });
                                            });
                                            ui.checkbox(&mut self.warn_on_low_memory, "低速な環境での警告を表示");
                                        });
                                    });

                                    ui.add_space(6.0);
                                    ui.separator();
                                    ui.add_space(6.0);

                                    // 選択・表示
                                    ui.label(RichText::new("選択・表示").strong().size(11.5).color(Color32::from_rgb(180, 180, 180)));
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.set_width(260.0);
                                            ui.checkbox(&mut self.show_bounding_box, "オブジェクトを選択したときにバウンディングボックスを表示");
                                            ui.checkbox(&mut self.show_anchor_points, "アンカーポイントを表示");
                                            ui.checkbox(&mut self.show_tool_hints, "ツールヒントを表示");
                                            ui.checkbox(&mut self.show_smart_guides_on_transform, "変形時にスマートガイドを表示");
                                        });
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("アンカーポイントのサイズ:").size(11.0));
                                                ui.add(egui::Slider::new(&mut self.anchor_point_size, 2.0..=8.0).suffix(" px"));
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("ハンドルのサイズ:").size(11.0));
                                                ui.add(egui::Slider::new(&mut self.handle_size, 2.0..=8.0).suffix(" px"));
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("選択範囲の線の太さ:").size(11.0));
                                                ui.add(egui::Slider::new(&mut self.selection_line_width, 0.5..=3.0).suffix(" px"));
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("ポイントとハンドルのカラー:").size(11.0));
                                                egui::ComboBox::from_id_salt("handle_color")
                                                    .selected_text(&self.point_handle_color_mode)
                                                    .width(90.0)
                                                    .show_ui(ui, |ui| {
                                                        ui.selectable_value(&mut self.point_handle_color_mode, "デフォルト".to_string(), "デフォルト (青)");
                                                        ui.selectable_value(&mut self.point_handle_color_mode, "ハイコントラスト".to_string(), "ハイコントラスト");
                                                    });
                                            });
                                        });
                                    });

                                    ui.add_space(6.0);
                                    ui.separator();
                                    ui.add_space(6.0);

                                    // 外観
                                    ui.label(RichText::new("外観").strong().size(11.5).color(Color32::from_rgb(180, 180, 180)));
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.set_width(260.0);
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("カラーテーマ:").size(11.0));
                                                egui::ComboBox::from_id_salt("pref_theme")
                                                    .selected_text(&self.color_theme)
                                                    .width(100.0)
                                                    .show_ui(ui, |ui| {
                                                        ui.selectable_value(&mut self.color_theme, "ダーク".to_string(), "ダーク (Adobe Charcoal)");
                                                        ui.selectable_value(&mut self.color_theme, "ミディアムダーク".to_string(), "ミディアムダーク");
                                                        ui.selectable_value(&mut self.color_theme, "ライト".to_string(), "ライト");
                                                    });
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("UIのスケール:").size(11.0));
                                                egui::ComboBox::from_id_salt("pref_scale")
                                                    .selected_text(&self.ui_scale)
                                                    .width(100.0)
                                                    .show_ui(ui, |ui| {
                                                        ui.selectable_value(&mut self.ui_scale, "100%".to_string(), "100% (標準)");
                                                        ui.selectable_value(&mut self.ui_scale, "125%".to_string(), "125%");
                                                        ui.selectable_value(&mut self.ui_scale, "150%".to_string(), "150%");
                                                    });
                                            });
                                        });
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("アートボードの背景:").size(11.0));
                                                egui::ComboBox::from_id_salt("artboard_bg")
                                                    .selected_text(&self.artboard_bg_mode)
                                                    .width(100.0)
                                                    .show_ui(ui, |ui| {
                                                        ui.selectable_value(&mut self.artboard_bg_mode, "ホワイト".to_string(), "ホワイト");
                                                        ui.selectable_value(&mut self.artboard_bg_mode, "透明グリッド".to_string(), "透明グリッド");
                                                    });
                                            });
                                            ui.checkbox(&mut self.show_boundary_lines, "境界線を表示");
                                            ui.checkbox(&mut self.show_dimension_labels, "ディメンションラベルを表示");
                                            ui.checkbox(&mut self.antialias_artwork, "ズーム時にアートワークをアンチエイリアス表示");
                                        });
                                    });

                                    ui.add_space(6.0);
                                    ui.separator();
                                    ui.add_space(6.0);

                                    // 通知
                                    ui.label(RichText::new("通知").strong().size(11.5).color(Color32::from_rgb(180, 180, 180)));
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.set_width(260.0);
                                            ui.checkbox(&mut self.notify_file_compat, "ファイルの互換性に関する問題");
                                            ui.checkbox(&mut self.notify_font_substitute, "フォントの置換が発生したとき");
                                        });
                                        ui.vertical(|ui| {
                                            ui.checkbox(&mut self.notify_plugin_load, "プラグインの読み込みに関するメッセージ");
                                            ui.checkbox(&mut self.notify_perf_recommend, "パフォーマンスに関する推奨事項");
                                        });
                                    });
                                }
                                _ => {
                                    ui.label(RichText::new(format!("{:?}", self.selected_category)).strong().size(14.0));
                                    ui.add_space(10.0);
                                    ui.label("このカテゴリのすべての標準プロファイルが適用されています。");
                                }
                            }
                        });

                        // Dialog Footer Buttons
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(RichText::new("OK").strong().color(Color32::WHITE))
                                    .fill(Color32::from_rgb(20, 115, 230))
                                    .min_size(Vec2::new(80.0, 26.0))).clicked()
                                {
                                    self.is_open = false;
                                }
                                if ui.add(egui::Button::new("キャンセル").min_size(Vec2::new(80.0, 26.0))).clicked() {
                                    self.is_open = false;
                                }
                            });
                        });
                    });
                });
            });
        self.is_open = is_open;
    }
}
