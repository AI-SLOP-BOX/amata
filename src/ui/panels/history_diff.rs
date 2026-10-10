use crate::core::diff::{compute_semantic_diff, ObjectDiffStatus, SemanticDiff};
use crate::core::document::Document;
use crate::core::state::AppState;
use crate::io::git::{self, GitCommitEntry};
use crate::io::svg::parse_svg_document;
use egui::{self, Color32, RichText, Ui, Vec2};
use std::path::{Path, PathBuf};

/// Parse stored file content according to the watched file's format.
/// Checkpoints of `.amata`/`.json` projects hold JSON, not SVG — parsing
/// them as SVG used to yield an empty document and destroy the canvas.
pub fn parse_stored_doc(file_path: &Path, content: &str) -> Option<Document> {
    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "amata" || ext == "json" {
        serde_json::from_str(content).ok().map(|mut doc: Document| {
            doc.normalize();
            doc
        })
    } else {
        Some(parse_svg_document(content))
    }
}

#[derive(Default)]
pub struct VersionHistoryPanel {
    pub current_file: Option<PathBuf>,
    pub commits: Vec<GitCommitEntry>,
    pub selected_commit_idx: Option<usize>,
    pub active_diff: Option<SemanticDiff>,
    pub is_comparing: bool,
    pub checkpoint_input: String,
    pub is_advanced_mode: bool,
    /// Document stashed while previewing an old revision; restored on close
    /// so previewing never discards unsaved work.
    preview_backup: Option<Document>,
}

impl VersionHistoryPanel {
    pub fn refresh_history(&mut self, file_path: &Path) {
        self.current_file = Some(file_path.to_path_buf());
        if git::is_git_repository(file_path) {
            if let Ok(entries) = git::get_file_commit_history(file_path, 30) {
                self.commits = entries;
            }
        } else {
            self.commits.clear();
        }
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.heading(RichText::new(crate::ui::i18n::text(&locale, "history.title")).strong());
        ui.label(
            RichText::new(crate::ui::i18n::text(&locale, "history.description"))
                .weak()
                .size(11.0),
        );
        ui.add_space(4.0);

        // Checkpoint creation section
        ui.group(|ui| {
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "history.checkpoint"))
                    .strong()
                    .size(11.5),
            );
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.checkpoint_input);
                let can_commit = !self.checkpoint_input.trim().is_empty();
                if ui
                    .add_enabled(
                        can_commit,
                        egui::Button::new(crate::ui::i18n::text(&locale, "history.save")),
                    )
                    .clicked()
                {
                    let msg = self.checkpoint_input.trim().to_string();
                    let file_path = self
                        .current_file
                        .clone()
                        .unwrap_or_else(|| PathBuf::from("poster.svg"));
                    // Auto-save current document to file in its own format
                    // (project files must stay JSON, never SVG bytes).
                    state.sync_doc_extras();
                    match crate::cli::handlers::common::save_any_document(
                        &state.document,
                        &file_path,
                    ) {
                        Err(e) => {
                            state.notify_error(crate::ui::i18n::format(
                                &locale,
                                "history.save_failed",
                                &[("error", &e.to_string())],
                            ));
                        }
                        Ok(_) => {
                            if !git::is_git_repository(&file_path) {
                                let parent =
                                    file_path.parent().unwrap_or(std::path::Path::new("."));
                                let _ = git::init_git_repository(parent);
                            }
                            match git::create_checkpoint(&file_path, &msg) {
                                Ok(_) => {
                                    state.notify_info(crate::ui::i18n::format(
                                        &locale,
                                        "history.checkpoint_saved",
                                        &[("name", &msg)],
                                    ));
                                    self.checkpoint_input.clear();
                                    self.refresh_history(&file_path);
                                }
                                Err(e) => state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "history.save_failed",
                                    &[("error", &e.to_string())],
                                )),
                            }
                        }
                    }
                }
            });

            ui.add_space(2.0);
            if ui
                .button(
                    RichText::new(crate::ui::i18n::text(&locale, "history.ai_checkpoint"))
                        .color(Color32::from_rgb(100, 200, 255)),
                )
                .on_hover_text(crate::ui::i18n::text(&locale, "history.ai_checkpoint_tip"))
                .clicked()
            {
                let file_path = self
                    .current_file
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("poster.svg"));
                state.sync_doc_extras();
                match crate::cli::handlers::common::save_any_document(&state.document, &file_path) {
                    Err(e) => {
                        state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "history.checkpoint_failed",
                            &[("error", &e.to_string())],
                        ));
                    }
                    Ok(_) => {
                        if !git::is_git_repository(&file_path) {
                            let parent = file_path.parent().unwrap_or(std::path::Path::new("."));
                            let _ = git::init_git_repository(parent);
                        }
                        let msg = crate::ui::i18n::text(&locale, "history.ai_before").into_owned();
                        match git::create_checkpoint(&file_path, &msg) {
                            Ok(_) => {
                                state.notify_info(crate::ui::i18n::text(
                                    &locale,
                                    "history.ai_recorded",
                                ));
                                self.refresh_history(&file_path);
                            }
                            Err(e) => state.notify_error(crate::ui::i18n::format(
                                &locale,
                                "history.checkpoint_failed",
                                &[("error", &e.to_string())],
                            )),
                        }
                    }
                }
            }
        });

        ui.add_space(6.0);

        // Mode switch: Friendly vs Advanced
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::ui::i18n::format(
                    &locale,
                    "history.list",
                    &[("count", &self.commits.len().to_string())],
                ))
                .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(
                    &mut self.is_advanced_mode,
                    crate::ui::i18n::text(&locale, "history.git_details"),
                );
            });
        });

        ui.separator();

        if self.commits.is_empty() {
            ui.label(RichText::new(crate::ui::i18n::text(&locale, "history.empty")).weak());
            if let Some(p) = self.current_file.clone() {
                if !git::is_git_repository(&p)
                    && ui
                        .button(crate::ui::i18n::text(&locale, "history.initialize"))
                        .clicked()
                {
                    let parent = p.parent().unwrap_or(std::path::Path::new("."));
                    if git::init_git_repository(parent).is_ok() {
                        state.sync_doc_extras();
                        match crate::cli::handlers::common::save_any_document(&state.document, &p) {
                            Err(e) => {
                                state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "history.initial_save_failed",
                                    &[("error", &e.to_string())],
                                ));
                            }
                            Ok(_) => match git::create_checkpoint(
                                &p,
                                crate::ui::i18n::text(&locale, "history.initial_checkpoint")
                                    .as_ref(),
                            ) {
                                Err(e) => state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "history.checkpoint_failed",
                                    &[("error", &e.to_string())],
                                )),
                                Ok(_) => {
                                    state.notify_info(crate::ui::i18n::text(
                                        &locale,
                                        "history.started",
                                    ));
                                    self.refresh_history(&p);
                                }
                            },
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
                                        RichText::new(crate::ui::i18n::format(
                                            &locale,
                                            "history.commit_author",
                                            &[
                                                ("hash", &commit.short_hash),
                                                ("author", &commit.author),
                                            ],
                                        ))
                                        .weak()
                                        .size(10.0),
                                    );
                                }
                            });
                        });

                        ui.horizontal(|ui| {
                            if ui
                                .small_button(crate::ui::i18n::text(&locale, "history.compare"))
                                .on_hover_text(crate::ui::i18n::text(
                                    &locale,
                                    "history.compare_tip",
                                ))
                                .clicked()
                            {
                                action_compare = Some(commit.hash.clone());
                                self.selected_commit_idx = Some(idx);
                            }
                            if ui
                                .small_button(crate::ui::i18n::text(&locale, "history.view"))
                                .clicked()
                            {
                                action_view = Some(commit.hash.clone());
                                self.selected_commit_idx = Some(idx);
                            }
                            if ui
                                .small_button(crate::ui::i18n::text(&locale, "history.restore"))
                                .on_hover_text(crate::ui::i18n::text(
                                    &locale,
                                    "history.restore_tip",
                                ))
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
            if let Some(ref file_path) = self.current_file.clone() {
                if state.is_dirty() {
                    state.notify_error(crate::ui::i18n::text(&locale, "history.unsaved"));
                } else {
                    match git::get_file_content_at_rev(file_path, &rev) {
                        Ok(content) => match parse_stored_doc(file_path, &content) {
                            Some(doc) => {
                                state.flush_pending_edits();
                                state.document = doc;
                                state.adopt_doc_extras();
                                state.clear_history();
                                state.selected_ids.clear();
                                state.notify_info(crate::ui::i18n::format(
                                    &locale,
                                    "history.restored",
                                    &[("version", &rev[..7.min(rev.len())])],
                                ));
                            }
                            None => state.notify_error(
                                crate::ui::i18n::text(&locale, "history.parse_restore_failed")
                                    .to_string(),
                            ),
                        },
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "history.checkpoint_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                }
            }
        }

        if let Some(rev) = action_view {
            if let Some(ref file_path) = self.current_file.clone() {
                match git::get_file_content_at_rev(file_path, &rev) {
                    Ok(content) => match parse_stored_doc(file_path, &content) {
                        Some(doc) => {
                            if self.preview_backup.is_none() {
                                self.preview_backup = Some(state.document.clone());
                            }
                            state.flush_pending_edits();
                            state.document = doc;
                            state.adopt_doc_extras();
                            // Whole-document swap: pending snapshots and the
                            // undo stack no longer match the on-screen doc.
                            state.pending_objects.clear();
                            state.pending_layers.clear();
                            state.pending_transforms.clear();
                            state.clear_history();
                            state.selected_ids.clear();
                            state.notify_info(crate::ui::i18n::format(
                                &locale,
                                "history.previewing",
                                &[("version", &rev[..7.min(rev.len())])],
                            ));
                        }
                        None => state.notify_error(
                            crate::ui::i18n::text(&locale, "history.parse_preview_failed")
                                .to_string(),
                        ),
                    },
                    Err(e) => state.notify_error(crate::ui::i18n::format(
                        &locale,
                        "history.load_failed",
                        &[("error", &e.to_string())],
                    )),
                }
            }
        }

        if let Some(rev) = action_compare {
            if let Some(ref file_path) = self.current_file.clone() {
                match git::get_file_content_at_rev(file_path, &rev) {
                    Ok(content) => match parse_stored_doc(file_path, &content) {
                        Some(doc_old) => {
                            let diff = compute_semantic_diff(&doc_old, &state.document);
                            self.active_diff = Some(diff);
                            self.is_comparing = true;
                            state.notify_info(crate::ui::i18n::text(
                                &locale,
                                "history.diff_generated",
                            ));
                        }
                        None => state.notify_error(
                            crate::ui::i18n::text(&locale, "history.parse_compare_failed")
                                .to_string(),
                        ),
                    },
                    Err(e) => state.notify_error(crate::ui::i18n::format(
                        &locale,
                        "history.compare_failed",
                        &[("error", &e.to_string())],
                    )),
                }
            }
        }

        // Diff Visualizer Section
        if self.is_comparing {
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new(crate::ui::i18n::text(&locale, "history.diff_title")).strong(),
                );
                if ui
                    .button(crate::ui::i18n::text(&locale, "history.close"))
                    .clicked()
                {
                    self.is_comparing = false;
                    self.active_diff = None;
                    if let Some(backup) = self.preview_backup.take() {
                        state.flush_pending_edits();
                        state.document = backup;
                        state.adopt_doc_extras();
                        state.notify_info(crate::ui::i18n::text(&locale, "history.preview_closed"));
                    }
                }
            });

            if let Some(ref diff) = self.active_diff {
                ui.label(
                    RichText::new(crate::ui::i18n::format(
                        &locale,
                        "history.diff_summary",
                        &[
                            ("added", &diff.summary.added_count.to_string()),
                            ("removed", &diff.summary.removed_count.to_string()),
                            ("modified", &diff.summary.modified_count.to_string()),
                        ],
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
                                ObjectDiffStatus::Added => (
                                    "＋",
                                    Color32::from_rgb(46, 204, 113),
                                    crate::ui::i18n::text(&locale, "history.added"),
                                ),
                                ObjectDiffStatus::Removed => (
                                    "ー",
                                    Color32::from_rgb(231, 76, 60),
                                    crate::ui::i18n::text(&locale, "history.removed"),
                                ),
                                ObjectDiffStatus::Modified { .. } => (
                                    "✎",
                                    Color32::from_rgb(241, 196, 15),
                                    crate::ui::i18n::text(&locale, "history.modified"),
                                ),
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
                                        && !obj.id.is_empty()
                                    {
                                        state.selected_ids = vec![obj.id.clone()];
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
