use crate::core::diff::{compute_semantic_diff, ObjectDiffStatus, SemanticDiff};
use crate::core::state::AppState;
use crate::io::git::{self, GitCommitEntry};
use crate::io::svg::parse_svg_document;
use egui::{self, Color32, RichText, Ui, Vec2};
use std::path::PathBuf;

pub struct VersionHistoryPanel {
    pub current_file: Option<PathBuf>,
    pub commits: Vec<GitCommitEntry>,
    pub selected_commit_idx: Option<usize>,
    pub active_diff: Option<SemanticDiff>,
    pub is_comparing: bool,
    pub checkpoint_input: String,
    pub is_advanced_mode: bool,
}

impl Default for VersionHistoryPanel {
    fn default() -> Self {
        Self {
            current_file: None,
            commits: Vec::new(),
            selected_commit_idx: None,
            active_diff: None,
            is_comparing: false,
            checkpoint_input: String::new(),
            is_advanced_mode: false,
        }
    }
}

impl VersionHistoryPanel {
    pub fn refresh_history(&mut self, file_path: &PathBuf) {
        self.current_file = Some(file_path.clone());
        if git::is_git_repository(file_path) {
            if let Ok(entries) = git::get_file_commit_history(file_path, 30) {
                self.commits = entries;
            }
        } else {
            self.commits.clear();
        }
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🕒 バージョン履歴 (Version History)").strong());
        ui.label(
            RichText::new("デザインの節目を安全に記録・比較・復元できます")
                .weak()
                .size(11.0),
        );
        ui.add_space(4.0);

        // Checkpoint creation section
        ui.group(|ui| {
            ui.label(
                RichText::new("📌 今の状態を保存 (Checkpoint)")
                    .strong()
                    .size(11.5),
            );
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.checkpoint_input);
                let can_commit = !self.checkpoint_input.trim().is_empty();
                if ui
                    .add_enabled(can_commit, egui::Button::new("保存"))
                    .clicked()
                {
                    let msg = self.checkpoint_input.trim().to_string();
                    let file_path = self
                        .current_file
                        .clone()
                        .unwrap_or_else(|| PathBuf::from("poster.svg"));
                    // Auto-save current document to file
                    let svg_content = crate::io::svg::export_svg(&state.document);
                    if let Ok(_) = std::fs::write(&file_path, svg_content) {
                        if !git::is_git_repository(&file_path) {
                            let parent = file_path.parent().unwrap_or(std::path::Path::new("."));
                            let _ = git::init_git_repository(parent);
                        }
                        match git::create_checkpoint(&file_path, &msg) {
                            Ok(_) => {
                                state.notify_info(format!(
                                    "チェックポイント「{}」を保存しました",
                                    msg
                                ));
                                self.checkpoint_input.clear();
                                self.refresh_history(&file_path);
                            }
                            Err(e) => state.notify_error(format!("保存失敗: {e}")),
                        }
                    }
                }
            });

            ui.add_space(2.0);
            if ui
                .button(
                    RichText::new("🛡️ AI / 外部編集前の状態を保存")
                        .color(Color32::from_rgb(100, 200, 255)),
                )
                .on_hover_text(
                    "外部AIやスクリプトを実行する直前の安全なスナップショットを作成します",
                )
                .clicked()
            {
                let file_path = self
                    .current_file
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("poster.svg"));
                let svg_content = crate::io::svg::export_svg(&state.document);
                if let Ok(_) = std::fs::write(&file_path, svg_content) {
                    if !git::is_git_repository(&file_path) {
                        let parent = file_path.parent().unwrap_or(std::path::Path::new("."));
                        let _ = git::init_git_repository(parent);
                    }
                    let msg = "AI編集前 (Before AI edit)";
                    match git::create_checkpoint(&file_path, msg) {
                        Ok(_) => {
                            state.notify_info("🛡️ AI編集前のチェックポイントを記録しました");
                            self.refresh_history(&file_path);
                        }
                        Err(e) => state.notify_error(format!("チェックポイント失敗: {e}")),
                    }
                }
            }
        });

        ui.add_space(6.0);

        // Mode switch: Friendly vs Advanced
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("履歴一覧 ({} 件)", self.commits.len())).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.is_advanced_mode, "Git詳細表示");
            });
        });

        ui.separator();

        if self.commits.is_empty() {
            ui.label(RichText::new("このファイルにはまだ履歴がありません").weak());
            if let Some(p) = self.current_file.clone() {
                if !git::is_git_repository(&p) {
                    if ui
                        .button("履歴管理を開始 (Gitリポジトリを初期化)")
                        .clicked()
                    {
                        let parent = p.parent().unwrap_or(std::path::Path::new("."));
                        if let Ok(_) = git::init_git_repository(parent) {
                            let svg_content = crate::io::svg::export_svg(&state.document);
                            let _ = std::fs::write(&p, svg_content);
                            let _ = git::create_checkpoint(&p, "初稿 (Initial layout)");
                            state.notify_info("バージョン管理を開始しました");
                            self.refresh_history(&p);
                        }
                    }
                }
            }
            return;
        }

        // Commit timeline list
        let mut action_restore: Option<String> = None;
        let mut action_compare: Option<String> = None;
        let mut action_view: Option<String> = None;

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (idx, commit) in self.commits.iter().enumerate() {
                    let _is_selected = self.selected_commit_idx == Some(idx);
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let dot_col = if idx == 0 {
                                Color32::from_rgb(46, 204, 113)
                            } else {
                                Color32::from_rgb(100, 150, 255)
                            };
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 4.0, dot_col);

                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&commit.message).strong().size(12.0));
                                    ui.label(
                                        RichText::new(&commit.relative_time).weak().size(10.5),
                                    );
                                });

                                if self.is_advanced_mode {
                                    ui.label(
                                        RichText::new(format!(
                                            "コミット: {} | 作成者: {}",
                                            commit.short_hash, commit.author
                                        ))
                                        .weak()
                                        .size(10.0),
                                    );
                                }
                            });
                        });

                        ui.horizontal(|ui| {
                            if ui
                                .small_button("🔍 比較")
                                .on_hover_text("現在版との違いを比較")
                                .clicked()
                            {
                                action_compare = Some(commit.hash.clone());
                                self.selected_commit_idx = Some(idx);
                            }
                            if ui.small_button("👁 この版を見る").clicked() {
                                action_view = Some(commit.hash.clone());
                                self.selected_commit_idx = Some(idx);
                            }
                            if ui
                                .small_button("↺ この版に戻す")
                                .on_hover_text("この時点のデザインへ復元")
                                .clicked()
                            {
                                action_restore = Some(commit.hash.clone());
                            }
                        });
                    });
                    ui.add_space(2.0);
                }
            });

        // Perform actions
        if let Some(rev) = action_restore {
            if let Some(ref file_path) = self.current_file {
                match git::get_file_content_at_rev(file_path, &rev) {
                    Ok(svg) => {
                        let doc = parse_svg_document(&svg);
                        state.document = doc;
                        state.undo_manager.clear();
                        state.selected_ids.clear();
                        state.notify_info(format!(
                            "バージョン {} に復元しました",
                            &rev[..7.min(rev.len())]
                        ));
                    }
                    Err(e) => state.notify_error(format!("復元失敗: {e}")),
                }
            }
        }

        if let Some(rev) = action_view {
            if let Some(ref file_path) = self.current_file {
                match git::get_file_content_at_rev(file_path, &rev) {
                    Ok(svg) => {
                        let doc = parse_svg_document(&svg);
                        state.document = doc;
                        state.notify_info(format!(
                            "バージョン {} をプレビュー中",
                            &rev[..7.min(rev.len())]
                        ));
                    }
                    Err(e) => state.notify_error(format!("読み込み失敗: {e}")),
                }
            }
        }

        if let Some(rev) = action_compare {
            if let Some(ref file_path) = self.current_file {
                match git::get_file_content_at_rev(file_path, &rev) {
                    Ok(svg_old) => {
                        let doc_old = parse_svg_document(&svg_old);
                        let diff = compute_semantic_diff(&doc_old, &state.document);
                        self.active_diff = Some(diff);
                        self.is_comparing = true;
                        state.notify_info("差分比較を生成しました");
                    }
                    Err(e) => state.notify_error(format!("比較対象の取得失敗: {e}")),
                }
            }
        }

        // Diff Visualizer Section
        if self.is_comparing {
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.heading(RichText::new("📊 変更内容の比較 (Visual Diff)").strong());
                if ui.button("閉じる").clicked() {
                    self.is_comparing = false;
                    self.active_diff = None;
                }
            });

            if let Some(ref diff) = self.active_diff {
                ui.label(
                    RichText::new(format!(
                        "変更点: 追加 {} 件 | 削除 {} 件 | 変更 {} 件",
                        diff.summary.added_count,
                        diff.summary.removed_count,
                        diff.summary.modified_count
                    ))
                    .size(11.5)
                    .strong(),
                );

                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for obj in &diff.objects {
                            let (icon, color, status_text) = match &obj.status {
                                ObjectDiffStatus::Added => {
                                    ("＋", Color32::from_rgb(46, 204, 113), "追加 (Added)")
                                }
                                ObjectDiffStatus::Removed => {
                                    ("ー", Color32::from_rgb(231, 76, 60), "削除 (Removed)")
                                }
                                ObjectDiffStatus::Modified { .. } => {
                                    ("✎", Color32::from_rgb(241, 196, 15), "変更 (Modified)")
                                }
                            };

                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(icon).color(color).strong());
                                    let id_display = if !obj.id.is_empty() {
                                        format!("#{}", obj.id)
                                    } else {
                                        obj.name.clone()
                                    };
                                    if ui
                                        .selectable_label(
                                            state.selected_ids.contains(&obj.id),
                                            &id_display,
                                        )
                                        .clicked()
                                    {
                                        if !obj.id.is_empty() {
                                            state.selected_ids = vec![obj.id.clone()];
                                        }
                                    }
                                    ui.label(
                                        RichText::new(format!("({})", obj.object_type))
                                            .weak()
                                            .size(10.5),
                                    );
                                    ui.label(RichText::new(status_text).color(color).size(10.5));
                                });

                                if let ObjectDiffStatus::Modified { changes } = &obj.status {
                                    for c in changes {
                                        ui.label(
                                            RichText::new(format!(
                                                "  • {}: {} ➔ {}",
                                                c.field, c.old_value, c.new_value
                                            ))
                                            .weak()
                                            .size(10.5),
                                        );
                                    }
                                }
                            });
                        }
                    });
            }
        }
    }
}
