use crate::core::state::AppState;
use eframe::egui::{self, Color32, RichText, Vec2};
use serde::{Deserialize, Serialize};

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

/// On-disk shape of [`PreferencesDialog`] (serde-friendly subset of fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedPrefs {
    show_home_on_startup: bool,
    open_last_doc: bool,
    show_new_doc_dialog: bool,
    sync_cloud_docs: bool,
    recent_files_count: usize,
    history_states_count: usize,
    warn_on_low_memory: bool,
    show_bounding_box: bool,
    show_anchor_points: bool,
    show_tool_hints: bool,
    show_smart_guides_on_transform: bool,
    anchor_point_size: f32,
    handle_size: f32,
    selection_line_width: f32,
    point_handle_color_mode: String,
    color_theme: String,
    ui_scale: String,
    artboard_bg_mode: String,
    show_boundary_lines: bool,
    show_dimension_labels: bool,
    antialias_artwork: bool,
    notify_file_compat: bool,
    notify_font_substitute: bool,
    notify_plugin_load: bool,
    notify_perf_recommend: bool,
}

fn prefs_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(std::path::PathBuf::from)
            .map(|p| p.join("Amata"))
            .or_else(|| {
                std::env::var("USERPROFILE")
                    .ok()
                    .map(|h| std::path::PathBuf::from(h).join(".amata"))
            })
            .map(|d| d.join("preferences.json"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| {
                std::path::PathBuf::from(h)
                    .join(".config")
                    .join("amata")
                    .join("preferences.json")
            })
    }
}

fn load_persisted() -> Option<PersistedPrefs> {
    let path = prefs_path()?;
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn save_persisted(p: &PersistedPrefs) {
    if let Some(path) = prefs_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(p) {
            let _ = std::fs::write(path, json);
        }
    }
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
        let mut d = Self {
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
        };
        if let Some(p) = load_persisted() {
            d.show_home_on_startup = p.show_home_on_startup;
            d.open_last_doc = p.open_last_doc;
            d.show_new_doc_dialog = p.show_new_doc_dialog;
            d.sync_cloud_docs = p.sync_cloud_docs;
            d.recent_files_count = p.recent_files_count;
            d.history_states_count = p.history_states_count;
            d.warn_on_low_memory = p.warn_on_low_memory;
            d.show_bounding_box = p.show_bounding_box;
            d.show_anchor_points = p.show_anchor_points;
            d.show_tool_hints = p.show_tool_hints;
            d.show_smart_guides_on_transform = p.show_smart_guides_on_transform;
            d.anchor_point_size = p.anchor_point_size;
            d.handle_size = p.handle_size;
            d.selection_line_width = p.selection_line_width;
            d.point_handle_color_mode = p.point_handle_color_mode;
            d.color_theme = p.color_theme;
            d.ui_scale = p.ui_scale;
            d.artboard_bg_mode = p.artboard_bg_mode;
            d.show_boundary_lines = p.show_boundary_lines;
            d.show_dimension_labels = p.show_dimension_labels;
            d.antialias_artwork = p.antialias_artwork;
            d.notify_file_compat = p.notify_file_compat;
            d.notify_font_substitute = p.notify_font_substitute;
            d.notify_plugin_load = p.notify_plugin_load;
            d.notify_perf_recommend = p.notify_perf_recommend;
        }
        d
    }
}

impl PreferencesDialog {
    fn persist(&self) {
        save_persisted(&PersistedPrefs {
            show_home_on_startup: self.show_home_on_startup,
            open_last_doc: self.open_last_doc,
            show_new_doc_dialog: self.show_new_doc_dialog,
            sync_cloud_docs: self.sync_cloud_docs,
            recent_files_count: self.recent_files_count,
            history_states_count: self.history_states_count,
            warn_on_low_memory: self.warn_on_low_memory,
            show_bounding_box: self.show_bounding_box,
            show_anchor_points: self.show_anchor_points,
            show_tool_hints: self.show_tool_hints,
            show_smart_guides_on_transform: self.show_smart_guides_on_transform,
            anchor_point_size: self.anchor_point_size,
            handle_size: self.handle_size,
            selection_line_width: self.selection_line_width,
            point_handle_color_mode: self.point_handle_color_mode.clone(),
            color_theme: self.color_theme.clone(),
            ui_scale: self.ui_scale.clone(),
            artboard_bg_mode: self.artboard_bg_mode.clone(),
            show_boundary_lines: self.show_boundary_lines,
            show_dimension_labels: self.show_dimension_labels,
            antialias_artwork: self.antialias_artwork,
            notify_file_compat: self.notify_file_compat,
            notify_font_substitute: self.notify_font_substitute,
            notify_plugin_load: self.notify_plugin_load,
            notify_perf_recommend: self.notify_perf_recommend,
        });
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if !self.is_open {
            return;
        }

        // Seed from live state so toggles reflect reality (they were pure
        // dead UI: nothing read or wrote these fields).
        self.show_smart_guides_on_transform = state.show_smart_guides;
        self.show_bounding_box = state.show_grid;

        let mut is_open = self.is_open;
        let mut close_clicked = false;
        let screen = ctx.screen_rect();
        let (size, min_size, pos) = crate::ui::window_defaults(
            screen,
            Vec2::new(560.0, 520.0),
            Vec2::new(300.0, 260.0),
        );
        egui::Window::new("環境設定")
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_pos(pos)
            .default_size(size)
            .min_size(min_size)
            .show(ctx, |ui| {
                let total_w = ui.available_width();
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

                // Category tabs across the top (always wrapped — no sidebar,
                // no width breakpoints).
                ui.horizontal_wrapped(|ui| {
                    for (cat, label) in categories {
                        let is_sel = self.selected_category == cat;
                        if ui
                            .selectable_label(
                                is_sel,
                                RichText::new(label)
                                    .size(11.5)
                                    .color(if is_sel {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(180, 180, 180)
                                    }),
                            )
                            .clicked()
                        {
                            self.selected_category = cat;
                        }
                    }
                });
                ui.separator();

                crate::ui::modal_body(ui, "pref_body", &mut |ui, is_body| {
                    if !is_body {
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("OK").strong().color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(20, 115, 230))
                                    .min_size(Vec2::new(80.0, 26.0)),
                                )
                                .clicked()
                            {
                                close_clicked = true;
                            }
                            if ui
                                .add(egui::Button::new("キャンセル").min_size(Vec2::new(80.0, 26.0)))
                                .clicked()
                            {
                                close_clicked = true;
                            }
                        });
                        return;
                    }
                    ui.set_width(total_w);
                    self.show_content(ui, total_w);
                    ui.add_space(8.0);
                    if ui
                        .button(RichText::new("すべての環境設定をリセット").size(10.5))
                        .clicked()
                    {
                        *self = Self::default();
                        self.is_open = true;
                    }
                });
            });
        if close_clicked {
            self.is_open = false;
        } else {
            self.is_open = is_open;
        }
        // Push live toggles into AppState every frame the dialog is open
        // (previously these 25 fields were never read or written anywhere).
        if self.is_open {
            state.show_smart_guides = self.show_smart_guides_on_transform;
            state.show_grid = self.show_bounding_box;
        } else {
            // Persist once when the dialog closes (or Escape dismisses it).
            self.persist();
        }
    }

    fn show_content(&mut self, ui: &mut egui::Ui, content_w: f32) {
        ui.set_width(content_w);
        match self.selected_category {
            PrefCategory::General => {
                ui.label(RichText::new("一般").strong().size(14.0));
                ui.add_space(6.0);

                ui.label(
                    RichText::new("起動・作業環境")
                        .strong()
                        .size(11.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.checkbox(&mut self.show_home_on_startup, "起動時にホームを表示");
                ui.checkbox(&mut self.open_last_doc, "前回のドキュメントを開く");
                ui.checkbox(
                    &mut self.show_new_doc_dialog,
                    "新規ドキュメントを作成するときに設定ダイアログを表示",
                );
                ui.checkbox(&mut self.sync_cloud_docs, "クラウドドキュメントを自動的に同期");
                ui.horizontal_wrapped(|ui| {
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
                ui.horizontal_wrapped(|ui| {
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

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    RichText::new("選択・表示")
                        .strong()
                        .size(11.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.checkbox(
                    &mut self.show_bounding_box,
                    "オブジェクトを選択したときにバウンディングボックスを表示",
                );
                ui.checkbox(&mut self.show_anchor_points, "アンカーポイントを表示");
                ui.checkbox(&mut self.show_tool_hints, "ツールヒントを表示");
                ui.checkbox(
                    &mut self.show_smart_guides_on_transform,
                    "変形時にスマートガイドを表示",
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("アンカーポイントのサイズ:").size(11.0));
                    ui.add(
                        egui::Slider::new(&mut self.anchor_point_size, 2.0..=8.0).suffix(" px"),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("ハンドルのサイズ:").size(11.0));
                    ui.add(egui::Slider::new(&mut self.handle_size, 2.0..=8.0).suffix(" px"));
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("選択範囲の線の太さ:").size(11.0));
                    ui.add(
                        egui::Slider::new(&mut self.selection_line_width, 0.5..=3.0)
                            .suffix(" px"),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("ポイントとハンドルのカラー:").size(11.0));
                    egui::ComboBox::from_id_salt("handle_color")
                        .selected_text(&self.point_handle_color_mode)
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.point_handle_color_mode,
                                "デフォルト".to_string(),
                                "デフォルト (青)",
                            );
                            ui.selectable_value(
                                &mut self.point_handle_color_mode,
                                "ハイコントラスト".to_string(),
                                "ハイコントラスト",
                            );
                        });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    RichText::new("外観")
                        .strong()
                        .size(11.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("カラーテーマ:").size(11.0));
                    egui::ComboBox::from_id_salt("pref_theme")
                        .selected_text(&self.color_theme)
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.color_theme,
                                "ダーク".to_string(),
                                "ダーク (Adobe Charcoal)",
                            );
                            ui.selectable_value(
                                &mut self.color_theme,
                                "ミディアムダーク".to_string(),
                                "ミディアムダーク",
                            );
                            ui.selectable_value(
                                &mut self.color_theme,
                                "ライト".to_string(),
                                "ライト",
                            );
                        });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("UIのスケール:").size(11.0));
                    egui::ComboBox::from_id_salt("pref_scale")
                        .selected_text(&self.ui_scale)
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.ui_scale,
                                "100%".to_string(),
                                "100% (標準)",
                            );
                            ui.selectable_value(&mut self.ui_scale, "125%".to_string(), "125%");
                            ui.selectable_value(&mut self.ui_scale, "150%".to_string(), "150%");
                        });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("アートボードの背景:").size(11.0));
                    egui::ComboBox::from_id_salt("artboard_bg")
                        .selected_text(&self.artboard_bg_mode)
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.artboard_bg_mode,
                                "ホワイト".to_string(),
                                "ホワイト",
                            );
                            ui.selectable_value(
                                &mut self.artboard_bg_mode,
                                "透明グリッド".to_string(),
                                "透明グリッド",
                            );
                        });
                });
                ui.checkbox(&mut self.show_boundary_lines, "境界線を表示");
                ui.checkbox(&mut self.show_dimension_labels, "ディメンションラベルを表示");
                ui.checkbox(
                    &mut self.antialias_artwork,
                    "ズーム時にアートワークをアンチエイリアス表示",
                );

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    RichText::new("通知")
                        .strong()
                        .size(11.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.checkbox(
                    &mut self.notify_file_compat,
                    "ファイルの互換性に関する問題",
                );
                ui.checkbox(
                    &mut self.notify_font_substitute,
                    "フォントの置換が発生したとき",
                );
                ui.checkbox(
                    &mut self.notify_plugin_load,
                    "プラグインの読み込みに関するメッセージ",
                );
                ui.checkbox(
                    &mut self.notify_perf_recommend,
                    "パフォーマンスに関する推奨事項",
                );
            }
            other => {
                ui.label(
                    RichText::new(format!("{:?}", other))
                        .strong()
                        .size(14.0),
                );
                ui.add_space(10.0);
                ui.label("このカテゴリのすべての標準プロファイルが適用されています。");
            }
        }
    }
}
