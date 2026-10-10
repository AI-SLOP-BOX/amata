use crate::core::prefs::Prefs;
use crate::core::state::AppState;
use crate::core::unit::LengthUnit;
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

/// The 環境設定 dialog.
///
/// The dialog owns no preference values of its own: every control writes
/// straight into [`AppState::prefs`], the single struct the renderer reads
/// live and that is persisted to `preferences.json` when the dialog closes
/// (*キャンセル* / Escape restore the snapshot taken on open instead).
#[derive(Debug, Clone, PartialEq)]
pub struct PreferencesDialog {
    pub is_open: bool,
    pub selected_category: PrefCategory,
    /// Preferences as they were when the dialog opened — *キャンセル* and
    /// Escape discard the live edits back to this.
    open_snapshot: Option<Prefs>,
}

impl Default for PreferencesDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            selected_category: PrefCategory::General,
            open_snapshot: None,
        }
    }
}

impl PreferencesDialog {
    /// Push the preferences that live *outside* `AppState::prefs` into the
    /// running session: the mirrored session toggles and the runtime stack
    /// limits. Called once at startup and again whenever the dialog closes.
    pub fn apply_to_session(state: &mut AppState) {
        state.show_smart_guides = state.prefs.show_smart_guides_on_transform;
        state.show_grid = state.prefs.show_grid;
        state.grid_size = state.prefs.grid_size;
        state.snap_to_grid = state.prefs.snap_to_grid;
        state.snap_to_objects = state.prefs.snap_to_objects;
        state.snap_to_guides = state.prefs.snap_to_guides;
        state.snap_to_points = state.prefs.snap_to_points;
        state.snap_to_pixels = state.prefs.snap_to_pixels;
        state.show_rulers = state.prefs.show_rulers;
        crate::io::recent::set_recent_limit(state.prefs.recent_files_count);
        state
            .undo_manager
            .set_max_steps(state.prefs.history_states_count);
    }

    /// Session → prefs for the mirrored toggles. Runs when the dialog opens
    /// (so the controls reflect what *View* / the panels last set) and every
    /// frame it is up (those menus stay usable behind the window).
    fn mirror_session(state: &mut AppState) {
        state.prefs.show_smart_guides_on_transform = state.show_smart_guides;
        state.prefs.show_grid = state.show_grid;
        state.prefs.grid_size = state.grid_size;
        state.prefs.snap_to_grid = state.snap_to_grid;
        state.prefs.snap_to_objects = state.snap_to_objects;
        state.prefs.snap_to_guides = state.snap_to_guides;
        state.prefs.snap_to_points = state.snap_to_points;
        state.prefs.snap_to_pixels = state.snap_to_pixels;
        state.prefs.show_rulers = state.show_rulers;
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if !self.is_open {
            // Closed from outside this function (Escape / a menu): discard
            // the live edits, exactly like *キャンセル*.
            if let Some(snapshot) = self.open_snapshot.take() {
                state.prefs = snapshot;
                Self::apply_to_session(state);
            }
            return;
        }

        // Grid / snap / rulers / smart guides are session state (View menu,
        // panels and `U` toggle them directly) — mirror the live values into
        // prefs so the controls and the persisted default never drift apart.
        Self::mirror_session(state);
        if self.open_snapshot.is_none() {
            self.open_snapshot = Some(state.prefs.clone());
        }

        let mut is_open = self.is_open;
        let mut ok_clicked = false;
        let mut cancel_clicked = false;
        let language = state.prefs.language.clone();
        let screen = ctx.screen_rect();
        let (size, min_size, pos) =
            crate::ui::window_defaults(screen, Vec2::new(560.0, 520.0), Vec2::new(300.0, 260.0));
        egui::Window::new(crate::ui::i18n::text(&language, "preferences.title"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_pos(pos)
            .default_size(size)
            .min_size(min_size)
            .show(ctx, |ui| {
                let total_w = ui.available_width();
                let categories = [
                    (PrefCategory::General, "prefs.category.general"),
                    (PrefCategory::Interface, "prefs.category.interface"),
                    (PrefCategory::Performance, "prefs.category.performance"),
                    (PrefCategory::FileHandling, "prefs.category.files"),
                    (PrefCategory::GuidesAndGrid, "prefs.category.guides_grid"),
                    (PrefCategory::Units, "prefs.category.units"),
                    (PrefCategory::Type, "prefs.category.type"),
                    (PrefCategory::Plugins, "prefs.category.plugins"),
                    (PrefCategory::Shortcuts, "prefs.category.shortcuts"),
                ];

                // Category tabs across the top (always wrapped — no sidebar,
                // no width breakpoints).
                ui.horizontal_wrapped(|ui| {
                    for (cat, label) in categories {
                        let is_sel = self.selected_category == cat;
                        if ui
                            .selectable_label(
                                is_sel,
                                RichText::new(crate::ui::i18n::text(&language, label))
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
                                ok_clicked = true;
                            }
                            if ui
                                .add(
                                    egui::Button::new(crate::ui::i18n::text(
                                        &state.prefs.language,
                                        "prefs.cancel",
                                    ))
                                    .min_size(Vec2::new(80.0, 26.0)),
                                )
                                .clicked()
                            {
                                cancel_clicked = true;
                            }
                        });
                        return;
                    }
                    ui.set_width(total_w);
                    self.show_content(ui, total_w, state);
                    ui.add_space(8.0);
                    if ui
                        .button(
                            RichText::new(crate::ui::i18n::text(
                                &state.prefs.language,
                                "prefs.reset",
                            ))
                            .size(10.5),
                        )
                        .clicked()
                    {
                        state.prefs = Prefs::default();
                        self.selected_category = PrefCategory::General;
                    }
                });
            });
        if ok_clicked || cancel_clicked {
            self.is_open = false;
        } else {
            self.is_open = is_open;
        }
        if self.is_open {
            // Live: prefs → the mirrored session toggles while the dialog is up.
            Self::apply_to_session(state);
        } else if cancel_clicked {
            // *キャンセル*: forget the edits — restore what was there on open
            // and push it back into the session.
            if let Some(snapshot) = self.open_snapshot.take() {
                state.prefs = snapshot;
            }
            Self::apply_to_session(state);
        } else {
            // OK / window close. Reached exactly once — the function returns
            // early on the frames after the dialog has closed.
            self.open_snapshot = None;
            state.prefs.save();
            Self::apply_to_session(state);
        }
    }

    fn show_content(&mut self, ui: &mut egui::Ui, content_w: f32, state: &mut AppState) {
        ui.set_width(content_w);
        let prefs = &mut state.prefs;
        match self.selected_category {
            PrefCategory::General => {
                ui.label(
                    RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.general"))
                        .strong()
                        .size(14.0),
                );
                ui.add_space(6.0);

                ui.label(
                    RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.startup"))
                        .strong()
                        .size(11.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.checkbox(
                    &mut prefs.show_home_on_startup,
                    crate::ui::i18n::text(&prefs.language, "prefs.home_startup"),
                );
                ui.checkbox(
                    &mut prefs.open_last_doc,
                    crate::ui::i18n::text(&prefs.language, "prefs.reopen_last"),
                );
                ui.checkbox(
                    &mut prefs.show_new_doc_dialog,
                    crate::ui::i18n::text(&prefs.language, "prefs.show_new_dialog"),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.recent_count"))
                            .size(11.0),
                    );
                    egui::ComboBox::from_id_salt("recent_files")
                        .selected_text(format!("{}", prefs.recent_files_count))
                        .width(60.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut prefs.recent_files_count, 10, "10");
                            ui.selectable_value(&mut prefs.recent_files_count, 20, "20");
                            ui.selectable_value(&mut prefs.recent_files_count, 50, "50");
                        });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.undo_count"))
                            .size(11.0),
                    );
                    egui::ComboBox::from_id_salt("history_count")
                        .selected_text(format!("{}", prefs.history_states_count))
                        .width(60.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut prefs.history_states_count, 50, "50");
                            ui.selectable_value(&mut prefs.history_states_count, 100, "100");
                            ui.selectable_value(&mut prefs.history_states_count, 200, "200");
                        });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    RichText::new(crate::ui::i18n::text(
                        &prefs.language,
                        "prefs.selection_display",
                    ))
                    .strong()
                    .size(11.5)
                    .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.checkbox(
                    &mut prefs.show_bounding_box,
                    crate::ui::i18n::text(&prefs.language, "prefs.show_bounds"),
                );
                ui.checkbox(
                    &mut prefs.show_anchor_points,
                    crate::ui::i18n::text(&prefs.language, "prefs.show_anchors"),
                );
                ui.checkbox(
                    &mut prefs.show_tool_hints,
                    crate::ui::i18n::text(&prefs.language, "prefs.show_tool_hints"),
                );
                ui.checkbox(
                    &mut prefs.show_smart_guides_on_transform,
                    crate::ui::i18n::text(&prefs.language, "prefs.show_smart_guides"),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.anchor_size"))
                            .size(11.0),
                    );
                    ui.add(
                        egui::Slider::new(&mut prefs.anchor_point_size, 2.0..=8.0).suffix(" px"),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.handle_size"))
                            .size(11.0),
                    );
                    ui.add(egui::Slider::new(&mut prefs.handle_size, 2.0..=8.0).suffix(" px"));
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(
                            &prefs.language,
                            "prefs.selection_line_width",
                        ))
                        .size(11.0),
                    );
                    ui.add(
                        egui::Slider::new(&mut prefs.selection_line_width, 0.5..=3.0).suffix(" px"),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.handle_color"))
                            .size(11.0),
                    );
                    egui::ComboBox::from_id_salt("handle_color")
                        .selected_text(match prefs.point_handle_color_mode {
                            crate::core::prefs::HandleColorMode::HighContrast => {
                                crate::ui::i18n::text(&prefs.language, "prefs.value.high_contrast")
                            }
                            _ => crate::ui::i18n::text(&prefs.language, "prefs.value.default_blue"),
                        })
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut prefs.point_handle_color_mode,
                                crate::core::prefs::HandleColorMode::Default,
                                crate::ui::i18n::text(&prefs.language, "prefs.value.default_blue"),
                            );
                            ui.selectable_value(
                                &mut prefs.point_handle_color_mode,
                                crate::core::prefs::HandleColorMode::HighContrast,
                                crate::ui::i18n::text(&prefs.language, "prefs.value.high_contrast"),
                            );
                        });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.appearance"))
                        .strong()
                        .size(11.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.color_theme"))
                            .size(11.0),
                    );
                    egui::ComboBox::from_id_salt("pref_theme")
                        .selected_text(match prefs.color_theme {
                            crate::core::prefs::ColorTheme::MediumDark => {
                                crate::ui::i18n::text(&prefs.language, "prefs.value.medium_dark")
                            }
                            crate::core::prefs::ColorTheme::Light => {
                                crate::ui::i18n::text(&prefs.language, "prefs.value.light")
                            }
                            _ => crate::ui::i18n::text(&prefs.language, "prefs.value.dark_adobe"),
                        })
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut prefs.color_theme,
                                crate::core::prefs::ColorTheme::Dark,
                                crate::ui::i18n::text(&prefs.language, "prefs.value.dark_adobe"),
                            );
                            ui.selectable_value(
                                &mut prefs.color_theme,
                                crate::core::prefs::ColorTheme::MediumDark,
                                crate::ui::i18n::text(&prefs.language, "prefs.value.medium_dark"),
                            );
                            ui.selectable_value(
                                &mut prefs.color_theme,
                                crate::core::prefs::ColorTheme::Light,
                                crate::ui::i18n::text(&prefs.language, "prefs.value.light"),
                            );
                        });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.ui_scale"))
                            .size(11.0),
                    );
                    egui::ComboBox::from_id_salt("pref_scale")
                        .selected_text(&prefs.ui_scale)
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut prefs.ui_scale,
                                "100%".to_string(),
                                crate::ui::i18n::text(
                                    &prefs.language,
                                    "prefs.value.scale_standard",
                                ),
                            );
                            ui.selectable_value(&mut prefs.ui_scale, "125%".to_string(), "125%");
                            ui.selectable_value(&mut prefs.ui_scale, "150%".to_string(), "150%");
                        });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(
                            &prefs.language,
                            "prefs.artboard_background",
                        ))
                        .size(11.0),
                    );
                    egui::ComboBox::from_id_salt("artboard_bg")
                        .selected_text(match prefs.artboard_bg_mode {
                            crate::core::prefs::ArtboardBgMode::Checker => crate::ui::i18n::text(
                                &prefs.language,
                                "prefs.value.transparency_grid",
                            ),
                            _ => crate::ui::i18n::text(&prefs.language, "prefs.value.white"),
                        })
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut prefs.artboard_bg_mode,
                                crate::core::prefs::ArtboardBgMode::White,
                                crate::ui::i18n::text(&prefs.language, "prefs.value.white"),
                            );
                            ui.selectable_value(
                                &mut prefs.artboard_bg_mode,
                                crate::core::prefs::ArtboardBgMode::Checker,
                                crate::ui::i18n::text(
                                    &prefs.language,
                                    "prefs.value.transparency_grid",
                                ),
                            );
                        });
                });
                ui.checkbox(
                    &mut prefs.show_boundary_lines,
                    crate::ui::i18n::text(&prefs.language, "prefs.show_boundaries"),
                );
                ui.checkbox(
                    &mut prefs.show_dimension_labels,
                    crate::ui::i18n::text(&prefs.language, "prefs.show_dimensions"),
                );

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    RichText::new(crate::ui::i18n::text(
                        &prefs.language,
                        "prefs.notifications",
                    ))
                    .strong()
                    .size(11.5)
                    .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.checkbox(
                    &mut prefs.notify_file_compat,
                    crate::ui::i18n::text(&prefs.language, "prefs.file_compat"),
                );
                ui.checkbox(
                    &mut prefs.notify_font_substitute,
                    crate::ui::i18n::text(&prefs.language, "prefs.font_substitution"),
                );
                ui.checkbox(
                    &mut prefs.notify_plugin_load,
                    crate::ui::i18n::text(&prefs.language, "prefs.plugin_messages"),
                );
            }
            PrefCategory::Interface => {
                ui.label(
                    RichText::new(crate::ui::i18n::text(
                        &prefs.language,
                        "prefs.category.interface",
                    ))
                    .strong()
                    .size(14.0),
                );
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(crate::ui::i18n::text(
                        &prefs.language,
                        "prefs.language.label",
                    ));
                    egui::ComboBox::from_id_salt("pref_language")
                        .selected_text(crate::ui::i18n::locale_name(&prefs.language))
                        .show_ui(ui, |ui| {
                            for (locale, native_name) in crate::ui::i18n::available_locales() {
                                ui.selectable_value(
                                    &mut prefs.language,
                                    locale.to_string(),
                                    native_name,
                                );
                            }
                        });
                });
                ui.label(
                    RichText::new(crate::ui::i18n::text(
                        &prefs.language,
                        "prefs.language.note",
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(150, 150, 150)),
                );
            }
            PrefCategory::GuidesAndGrid => {
                ui.label(
                    RichText::new(crate::ui::i18n::text(
                        &prefs.language,
                        "prefs.guides_grid_title",
                    ))
                    .strong()
                    .size(14.0),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.grid"))
                        .strong()
                        .size(11.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.checkbox(
                    &mut prefs.show_grid,
                    crate::ui::i18n::text(&prefs.language, "prefs.show_grid"),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.grid_spacing"))
                            .size(11.0),
                    );
                    ui.add(
                        egui::DragValue::new(&mut prefs.grid_size)
                            .speed(1.0)
                            .range(1.0..=500.0)
                            .suffix(" px"),
                    );
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.snap"))
                        .strong()
                        .size(11.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.checkbox(
                    &mut prefs.snap_to_grid,
                    crate::ui::i18n::text(&prefs.language, "prefs.snap_grid"),
                );
                ui.checkbox(
                    &mut prefs.snap_to_objects,
                    crate::ui::i18n::text(&prefs.language, "prefs.snap_objects"),
                );
                ui.checkbox(
                    &mut prefs.snap_to_guides,
                    crate::ui::i18n::text(&prefs.language, "prefs.snap_guides"),
                );
                ui.checkbox(
                    &mut prefs.snap_to_points,
                    crate::ui::i18n::text(&prefs.language, "prefs.snap_anchors"),
                );
                ui.checkbox(
                    &mut prefs.snap_to_pixels,
                    crate::ui::i18n::text(&prefs.language, "prefs.snap_pixels"),
                );

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.ruler"))
                        .strong()
                        .size(11.5)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.checkbox(
                    &mut prefs.show_rulers,
                    crate::ui::i18n::text(&prefs.language, "prefs.show_rulers"),
                );
            }
            PrefCategory::Units => {
                ui.label(
                    RichText::new(crate::ui::i18n::text(&prefs.language, "prefs.units_title"))
                        .strong()
                        .size(14.0),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(crate::ui::i18n::text(
                        &prefs.language,
                        "prefs.units_description",
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(150, 150, 150)),
                );
                ui.add_space(8.0);
                unit_row(
                    ui,
                    &crate::ui::i18n::text(&prefs.language, "prefs.unit_ruler_coordinates_size"),
                    &mut prefs.ruler_unit,
                );
                unit_row(
                    ui,
                    &crate::ui::i18n::text(&prefs.language, "prefs.unit_stroke"),
                    &mut prefs.stroke_unit,
                );
                unit_row(
                    ui,
                    &crate::ui::i18n::text(&prefs.language, "prefs.unit_new_document"),
                    &mut prefs.default_doc_unit,
                );
            }
            other => {
                ui.label(RichText::new(format!("{:?}", other)).strong().size(14.0));
                ui.add_space(10.0);
                ui.label(crate::ui::i18n::text(
                    &prefs.language,
                    "prefs.category_fallback",
                ));
            }
        }
    }
}

/// One *label + ComboBox* row for a [`LengthUnit`] preference. The id salt is
/// the row label, so the three rows on the 単位 page never collide.
fn unit_row(ui: &mut egui::Ui, label: &str, value: &mut LengthUnit) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0));
        egui::ComboBox::from_id_salt(label)
            .selected_text(value.label())
            .width(150.0)
            .show_ui(ui, |ui| {
                for unit in LengthUnit::ALL {
                    ui.selectable_value(value, unit, unit.label());
                }
            });
    });
}
