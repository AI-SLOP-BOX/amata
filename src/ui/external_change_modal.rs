use crate::core::diff::SemanticDiff;
use crate::core::document::Document;
use egui::{self, Color32, RichText, Vec2};
use std::path::PathBuf;

/// Structure storing information about an external file modification
#[derive(Debug, Clone)]
pub struct ExternalChangeNotice {
    pub file_path: PathBuf,
    pub external_svg: String,
    pub external_doc: Document,
    pub diff: SemanticDiff,
    pub is_conflict: bool,
    /// Pre-edit snapshot to allow exact restoration
    pub pre_edit_svg: String,
}

pub enum ExternalChangeAction {
    Compare,
    Accept,
    Revert,
    KeepLocal,
    Dismiss,
}

#[derive(Default)]
pub struct ExternalChangeDialog {
    pub notice: Option<ExternalChangeNotice>,
}

impl ExternalChangeDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_notice(&mut self, notice: ExternalChangeNotice) {
        self.notice = Some(notice);
    }

    pub fn clear(&mut self) {
        self.notice = None;
    }

    pub fn is_active(&self) -> bool {
        self.notice.is_some()
    }

    /// Render notification banner or conflict modal
    pub fn show(&mut self, ctx: &egui::Context, locale: &str) -> Option<ExternalChangeAction> {
        let notice = self.notice.as_ref()?;
        let mut action: Option<ExternalChangeAction> = None;

        let title = if notice.is_conflict {
            crate::ui::i18n::text(locale, "external.title.conflict")
        } else {
            crate::ui::i18n::text(locale, "external.title.changed")
        };

        let screen = ctx.screen_rect();
        let (size, min_size, pos) =
            crate::ui::window_defaults(screen, Vec2::new(480.0, 360.0), Vec2::new(300.0, 220.0));
        egui::Window::new(RichText::new(title).strong().size(14.0))
            .collapsible(false)
            .resizable(true)
            .default_pos(pos)
            .default_size(size)
            .min_size(min_size)
            .show(ctx, |ui| {
                let total_w = ui.available_width();
                crate::ui::modal_body(ui, "ext_change_body", &mut |ui, is_body| {
                    if !is_body {
                        ui.horizontal_wrapped(|ui| {
                            if notice.is_conflict {
                                if ui
                                    .button(
                                        RichText::new(crate::ui::i18n::text(
                                            locale,
                                            "external.keep_local",
                                        ))
                                        .color(Color32::from_rgb(100, 200, 255)),
                                    )
                                    .clicked()
                                {
                                    action = Some(ExternalChangeAction::KeepLocal);
                                }
                                if ui
                                    .button(
                                        RichText::new(crate::ui::i18n::text(
                                            locale,
                                            "external.compare",
                                        ))
                                        .color(Color32::from_rgb(255, 200, 100)),
                                    )
                                    .clicked()
                                {
                                    action = Some(ExternalChangeAction::Compare);
                                }
                                if ui
                                    .button(
                                        RichText::new(crate::ui::i18n::text(
                                            locale,
                                            "external.accept_conflict",
                                        ))
                                        .color(Color32::from_rgb(255, 100, 100)),
                                    )
                                    .clicked()
                                {
                                    action = Some(ExternalChangeAction::Accept);
                                }
                            } else {
                                if ui
                                    .button(
                                        RichText::new(crate::ui::i18n::text(
                                            locale,
                                            "external.compare_short",
                                        ))
                                        .strong()
                                        .color(Color32::from_rgb(100, 200, 255)),
                                    )
                                    .clicked()
                                {
                                    action = Some(ExternalChangeAction::Compare);
                                }
                                if ui
                                    .button(
                                        RichText::new(crate::ui::i18n::text(
                                            locale,
                                            "external.accept",
                                        ))
                                        .strong()
                                        .color(Color32::from_rgb(46, 204, 113)),
                                    )
                                    .clicked()
                                {
                                    action = Some(ExternalChangeAction::Accept);
                                }
                                if ui
                                    .button(
                                        RichText::new(crate::ui::i18n::text(
                                            locale,
                                            "external.revert",
                                        ))
                                        .color(Color32::from_rgb(241, 196, 15)),
                                    )
                                    .clicked()
                                {
                                    action = Some(ExternalChangeAction::Revert);
                                }
                                if ui
                                    .button(
                                        RichText::new(crate::ui::i18n::text(
                                            locale,
                                            "external.close",
                                        ))
                                        .weak(),
                                    )
                                    .clicked()
                                {
                                    action = Some(ExternalChangeAction::Dismiss);
                                }
                            }
                        });
                        return;
                    }
                    ui.set_width(total_w);
                    ui.add_space(4.0);

                    if notice.is_conflict {
                        ui.label(
                            RichText::new(crate::ui::i18n::text(
                                locale,
                                "external.conflict_message",
                            ))
                            .color(Color32::from_rgb(255, 180, 80)),
                        );
                        ui.label(
                            RichText::new(crate::ui::i18n::text(
                                locale,
                                "external.conflict_safety",
                            ))
                            .weak()
                            .size(11.5),
                        );
                    } else {
                        let total_changes = notice.diff.summary.added_count
                            + notice.diff.summary.removed_count
                            + notice.diff.summary.modified_count;
                        ui.label(
                            RichText::new(crate::ui::i18n::format(
                                locale,
                                "external.change_count",
                                &[("count", &total_changes.to_string())],
                            ))
                            .strong()
                            .size(12.5),
                        );
                    }

                    ui.add_space(6.0);

                    // Summary breakdown badges
                    let text_changes = notice.diff.count_text_changes();
                    let geom_changes = notice.diff.count_geometry_changes();
                    let style_changes = notice.diff.count_style_changes();
                    let added = notice.diff.summary.added_count;
                    let removed = notice.diff.summary.removed_count;

                    ui.group(|ui| {
                        ui.label(
                            RichText::new(crate::ui::i18n::text(locale, "external.summary"))
                                .strong()
                                .size(11.0),
                        );
                        ui.horizontal_wrapped(|ui| {
                            if text_changes > 0 {
                                ui.label(
                                    RichText::new(crate::ui::i18n::format(
                                        locale,
                                        "external.text_changes",
                                        &[("count", &text_changes.to_string())],
                                    ))
                                    .color(Color32::from_rgb(100, 200, 255)),
                                );
                            }
                            if geom_changes > 0 {
                                ui.label(
                                    RichText::new(crate::ui::i18n::format(
                                        locale,
                                        "external.geometry_changes",
                                        &[("count", &geom_changes.to_string())],
                                    ))
                                    .color(Color32::from_rgb(255, 200, 100)),
                                );
                            }
                            if style_changes > 0 {
                                ui.label(
                                    RichText::new(crate::ui::i18n::format(
                                        locale,
                                        "external.style_changes",
                                        &[("count", &style_changes.to_string())],
                                    ))
                                    .color(Color32::from_rgb(200, 150, 255)),
                                );
                            }
                            if added > 0 {
                                ui.label(
                                    RichText::new(crate::ui::i18n::format(
                                        locale,
                                        "external.added",
                                        &[("count", &added.to_string())],
                                    ))
                                    .color(Color32::from_rgb(46, 204, 113)),
                                );
                            }
                            if removed > 0 {
                                ui.label(
                                    RichText::new(crate::ui::i18n::format(
                                        locale,
                                        "external.removed",
                                        &[("count", &removed.to_string())],
                                    ))
                                    .color(Color32::from_rgb(231, 76, 60)),
                                );
                            }
                        });

                        // List changed IDs
                        let changed_names: Vec<String> = notice
                            .diff
                            .objects
                            .iter()
                            .take(5)
                            .map(|o| {
                                let id = if !o.id.is_empty() {
                                    format!("#{}", o.id)
                                } else {
                                    o.name.clone()
                                };
                                format!("{id} ({})", o.object_type)
                            })
                            .collect();

                        if !changed_names.is_empty() {
                            ui.add_space(3.0);
                            let more = if notice.diff.objects.len() > 5 {
                                crate::ui::i18n::format(
                                    locale,
                                    "external.more",
                                    &[("count", &(notice.diff.objects.len() - 5).to_string())],
                                )
                            } else {
                                String::new()
                            };
                            ui.label(
                                RichText::new(crate::ui::i18n::format(
                                    locale,
                                    "external.targets",
                                    &[("items", &changed_names.join(", ")), ("more", &more)],
                                ))
                                .weak()
                                .size(10.5),
                            );
                        }
                    });
                });
            });

        action
    }
}
