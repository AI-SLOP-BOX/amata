pub mod control_bar;
pub mod dock;
pub mod icons;
pub mod menu_bar;
pub mod shortcuts;
pub mod toolbar;

use crate::core::state::AppState;
use crate::ui::{
    apply_adobe_theme, AboutModal, CanvasWidget, ExportModal, HomeView, NewDocModal,
    OnboardingTour, PreferencesDialog, TimelineWidget,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ActiveTab {
    Properties,
    Layers,
    Pathfinder,
    ThreeDAndVfx,
    Generative,
    Symbols,
    Components,
    VersionHistory,
    Export,
    Guides,
}

pub struct IrasuApp {
    state: AppState,
    canvas: CanvasWidget,
    active_tab: ActiveTab,
    preferences_dialog: PreferencesDialog,
    export_modal: ExportModal,
    new_doc_modal: NewDocModal,
    about_modal: AboutModal,
    home_view: HomeView,
    onboarding_tour: OnboardingTour,
    search_query: String,
    version_history_panel: crate::ui::panels::VersionHistoryPanel,
    file_watcher: Option<crate::core::watcher::FileWatcher>,
    external_change_dialog: crate::ui::ExternalChangeDialog,
}

/// AmataApp is the primary application struct for the Amata vector editor.
pub type AmataApp = IrasuApp;

fn is_project_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "amata" | "json"
    )
}

fn parse_watched_doc(path: &std::path::Path, content: &str) -> Option<crate::core::document::Document> {
    if is_project_file(path) {
        serde_json::from_str(content).ok().map(|mut doc| {
            crate::core::document::Document::normalize(&mut doc);
            doc
        })
    } else {
        Some(crate::io::svg::parse_svg_document(content))
    }
}

fn serialize_watched_doc(
    doc: &crate::core::document::Document,
    path: &std::path::Path,
) -> Option<String> {
    if is_project_file(path) {
        serde_json::to_string_pretty(doc).ok()
    } else {
        Some(crate::io::svg::export_svg(doc))
    }
}

impl IrasuApp {
    pub fn with_file(path: Option<std::path::PathBuf>) -> Self {
        let mut app = Self::default();
        if let Some(p) = path {
            if let Ok(doc) = crate::cli::handlers::common::load_any_document(&p) {
                app.state.document = doc;
                app.state.zoom_to_fit();
                app.version_history_panel.refresh_history(&p);
                let mut watcher = crate::core::watcher::FileWatcher::new(p);
                if let Ok(content) = std::fs::read_to_string(&watcher.file_path) {
                    watcher.mark_saved(&content);
                }
                app.file_watcher = Some(watcher);
            } else {
                eprintln!("⚠️ Failed to load file: {}", p.display());
            }
        }
        app
    }
}

impl Default for IrasuApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
            canvas: CanvasWidget::new(),
            active_tab: ActiveTab::Properties,
            preferences_dialog: PreferencesDialog::default(),
            export_modal: ExportModal::default(),
            new_doc_modal: NewDocModal::default(),
            about_modal: AboutModal::default(),
            home_view: HomeView::default(),
            onboarding_tour: OnboardingTour::default(),
            search_query: String::new(),
            version_history_panel: crate::ui::panels::VersionHistoryPanel::default(),
            file_watcher: None,
            external_change_dialog: crate::ui::ExternalChangeDialog::default(),
        }
    }
}

impl eframe::App for IrasuApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Apply Adobe Charcoal Theme
        apply_adobe_theme(ctx);

        // Timeline animation playback tick
        if self.state.timeline.is_playing {
            self.state.timeline.advance_frame();
            self.state
                .timeline
                .apply_to_document(&mut self.state.document);
            ctx.request_repaint();
        }

        // Check for external file modifications (AI / CLI / external editor)
        if let Some(ref mut watcher) = self.file_watcher {
            if let Some(external_content) = watcher.check_for_external_content() {
                let file_path = watcher.file_path.clone();
                if let Some(external_doc) = parse_watched_doc(&file_path, &external_content) {
                    if let Some(pre_edit_svg) =
                        serialize_watched_doc(&self.state.document, &file_path)
                    {
                        let external_svg = external_content;
                        let diff = crate::core::diff::compute_semantic_diff(
                            &self.state.document,
                            &external_doc,
                        );
                        let is_conflict = self.state.is_dirty();

                        self.external_change_dialog.set_notice(
                            crate::ui::ExternalChangeNotice {
                                file_path: watcher.file_path.clone(),
                                external_svg,
                                external_doc,
                                diff,
                                is_conflict,
                                pre_edit_svg,
                            },
                        );
                    }
                }
            }
        }

        // Handle External Change Dialog actions
        if let Some(action) = self.external_change_dialog.show(ctx) {
            match action {
                crate::ui::ExternalChangeAction::Compare => {
                    if let Some(notice) = &self.external_change_dialog.notice {
                        self.state.active_diff = Some(notice.diff.clone());
                        self.state.is_comparing_diff = true;
                        self.active_tab = ActiveTab::VersionHistory;
                        self.state
                            .notify_info("キャンバスと差分パネルに変更箇所をハイライト表示中");
                    }
                }
                crate::ui::ExternalChangeAction::Accept => {
                    if let Some(notice) = self.external_change_dialog.notice.take() {
                        self.state.document = notice.external_doc;
                        self.state.undo_manager.mark_saved();
                        if let Some(ref mut w) = self.file_watcher {
                            w.mark_saved(&notice.external_svg);
                        }
                        self.version_history_panel
                            .refresh_history(&notice.file_path);
                        self.state.active_diff = None;
                        self.state.is_comparing_diff = false;
                        self.state
                            .notify_success("外部変更を採用し、キャンバスへ反映しました");
                    }
                }
                crate::ui::ExternalChangeAction::Revert => {
                    if let Some(notice) = self.external_change_dialog.notice.take() {
                        if let Err(e) = std::fs::write(&notice.file_path, &notice.pre_edit_svg) {
                            self.state.notify_error(format!("復元に失敗しました: {e}"));
                        } else if let Some(doc) =
                            parse_watched_doc(&notice.file_path, &notice.pre_edit_svg)
                        {
                            self.state.document = doc;
                            self.state.undo_manager.mark_saved();
                            if let Some(ref mut w) = self.file_watcher {
                                w.mark_saved(&notice.pre_edit_svg);
                            }
                            self.state.active_diff = None;
                            self.state.is_comparing_diff = false;
                            self.state
                                .notify_info("外部変更を破棄し、編集前の状態に復元しました");
                        } else {
                            self.state
                                .notify_error("復元データの解析に失敗しました".to_string());
                        }
                    }
                }
                crate::ui::ExternalChangeAction::KeepLocal => {
                    if let Some(notice) = self.external_change_dialog.notice.take() {
                        // 1. Safely preserve external version to prevent data loss
                        if let Err(e) = crate::io::git::preserve_external_version(
                            &notice.file_path,
                            &notice.external_svg,
                        ) {
                            self.state.notify_error(format!(
                                "外部版の退避保存に失敗したため、上書きを中止しました: {e}"
                            ));
                            self.external_change_dialog.set_notice(notice);
                            return;
                        }

                        let Some(current_content) =
                            serialize_watched_doc(&self.state.document, &notice.file_path)
                        else {
                            self.state.notify_error("保存データの生成に失敗しました".to_string());
                            self.external_change_dialog.set_notice(notice);
                            return;
                        };
                        if let Err(e) = std::fs::write(&notice.file_path, &current_content) {
                            self.state.notify_error(format!("保存に失敗しました: {e}"));
                        } else {
                            self.state.undo_manager.mark_saved();
                            if let Some(ref mut w) = self.file_watcher {
                                w.mark_saved(&current_content);
                            }
                            self.version_history_panel
                                .refresh_history(&notice.file_path);
                            self.state.active_diff = None;
                            self.state.is_comparing_diff = false;
                            self.state.notify_info(
                                "外部変更をスナップショットへ退避後、ローカル作業を保存しました",
                            );
                        }
                    }
                }
                crate::ui::ExternalChangeAction::Dismiss => {
                    self.external_change_dialog.clear();
                    self.state.is_comparing_diff = false;
                }
            }
        }

        // Keyboard shortcuts
        self.handle_shortcuts(ctx);

        // Top Menu Bar
        self.show_menu_bar(ctx);

        // Top Horizontal Control / Options Bar
        self.show_control_bar(ctx);

        // Document Tab Bar
        self.show_document_tab_bar(ctx);

        // Bottom Status Bar
        self.show_status_bar(ctx);

        // Left Vertical Toolbar
        self.show_toolbar(ctx);

        // Check if Home Hub is open (Image 1)
        if self.home_view.is_open {
            let mut tour_open = false;
            self.home_view.show(
                ctx,
                &mut self.state,
                &mut self.new_doc_modal,
                &mut tour_open,
            );
            if tour_open {
                self.home_view.is_open = false;
                self.onboarding_tour.is_active = true;
                self.onboarding_tour.is_panel_open = true;
            }
            self.preferences_dialog.show(ctx, &mut self.state);
            self.export_modal.show(ctx, &mut self.state);
            self.new_doc_modal.show(ctx, &mut self.state);
            self.about_modal.show(ctx);
            return;
        }

        // Right Tabbed Sidebar Panels
        self.show_right_dock(ctx);

        // Bottom Timeline Panel
        if self.state.show_timeline {
            egui::TopBottomPanel::bottom("timeline_panel")
                .resizable(true)
                .default_height(70.0)
                .show(ctx, |ui| {
                    TimelineWidget::show(ui, &mut self.state);
                });
        }

        // Central Drawing Canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            let device_queue = frame.wgpu_render_state().map(|s| {
                (
                    std::sync::Arc::new(s.device.clone()),
                    std::sync::Arc::new(s.queue.clone()),
                )
            });
            let (device, queue) = match device_queue {
                Some((d, q)) => (Some(d), Some(q)),
                None => (None, None),
            };
            self.canvas.show(ui, &mut self.state, device, queue);
        });

        // Interactive Tour Overlay & Lesson Guide (Image 3)
        self.onboarding_tour.show(ctx, &mut self.state);

        // Show Modals if open (Images 2, 4, 5)
        self.preferences_dialog.show(ctx, &mut self.state);
        self.export_modal.show(ctx, &mut self.state);
        self.new_doc_modal.show(ctx, &mut self.state);
        self.about_modal.show(ctx);

        // Floating Toast Notification
        self.state.clear_toast_if_expired();
        if let Some(toast) = &self.state.toast {
            ctx.request_repaint();
            egui::Area::new(egui::Id::new("toast_notification_area"))
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -42.0))
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    let (bg_col, border_col, icon, icon_col) = if toast.is_error {
                        (
                            egui::Color32::from_rgb(45, 20, 22),
                            egui::Color32::from_rgb(220, 70, 70),
                            "⚠ ",
                            egui::Color32::from_rgb(255, 100, 100),
                        )
                    } else {
                        (
                            egui::Color32::from_rgb(20, 32, 48),
                            egui::Color32::from_rgb(60, 140, 220),
                            "ℹ ",
                            egui::Color32::from_rgb(100, 190, 255),
                        )
                    };

                    egui::Frame::popup(ui.style())
                        .fill(bg_col)
                        .stroke(egui::Stroke::new(1.2_f32, border_col))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(16, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(icon)
                                        .size(14.0)
                                        .color(icon_col)
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(&toast.message)
                                        .size(12.5)
                                        .color(egui::Color32::WHITE),
                                );
                            });
                        });
                });
        }
    }
}

pub fn zoom_to_fit(state: &mut AppState) {
    state.zoom_to_fit();
}
