use crate::app::icons::{
    icon_arrow, icon_gear, icon_heart, icon_portal, icon_speech, icon_text_button,
};
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct PresetPanel;

/// Shared "outline text" switch: outline mode converts glyphs to paths so
/// OpenType features (和欧間 / palt / vertical `vert` forms) survive into
/// SVG and PNG — the raster engine ignores `font-feature-settings`.
fn outline_checkbox(ui: &mut Ui, state: &mut AppState, locale: &str) {
    ui.checkbox(
        &mut state.export_outline_text,
        crate::ui::i18n::text(locale, "utility.outline_text"),
    )
    .on_hover_text(crate::ui::i18n::text(locale, "utility.outline_text_tip"));
}

impl PresetPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        // Every preset drops one shape in the middle of the canvas and
        // selects it, so the row below is pure shape-picking.
        ui.horizontal_wrapped(|ui| {
            if icon_text_button(
                ui,
                icon_heart,
                crate::ui::i18n::text(&locale, "utility.heart").as_ref(),
            )
            .on_hover_text(crate::ui::i18n::text(&locale, "utility.heart_tip"))
            .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::heart("Heart", cx, cy, 120.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if icon_text_button(
                ui,
                icon_arrow,
                crate::ui::i18n::text(&locale, "utility.arrow").as_ref(),
            )
            .on_hover_text(crate::ui::i18n::text(&locale, "utility.arrow_tip"))
            .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::arrow("Arrow", cx, cy, 160.0, 40.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if icon_text_button(
                ui,
                icon_gear,
                crate::ui::i18n::text(&locale, "utility.gear").as_ref(),
            )
            .on_hover_text(crate::ui::i18n::text(&locale, "utility.gear_tip"))
            .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::gear("Gear", cx, cy, 8, 40.0, 60.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if icon_text_button(
                ui,
                icon_speech,
                crate::ui::i18n::text(&locale, "utility.speech").as_ref(),
            )
            .on_hover_text(crate::ui::i18n::text(&locale, "utility.speech_tip"))
            .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::speech_bubble(
                    "Speech Bubble",
                    cx,
                    cy,
                    150.0,
                    100.0,
                );
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if icon_text_button(
                ui,
                icon_portal,
                crate::ui::i18n::text(&locale, "utility.portal").as_ref(),
            )
            .on_hover_text(crate::ui::i18n::text(&locale, "utility.portal_tip"))
            .clicked()
            {
                let obj =
                    crate::core::presets::PresetLibrary::vfx_portal("VFX Portal", cx, cy, 80.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }
        });
    }
}

pub struct SmartGuidesPanel;

impl SmartGuidesPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.heading(RichText::new(crate::ui::i18n::text(&locale, "utility.smart_guides")).strong());
        ui.add_space(4.0);

        // Snapping options
        ui.label(RichText::new(crate::ui::i18n::text(&locale, "utility.snap_targets")).strong());
        ui.checkbox(
            &mut state.snap_to_grid,
            crate::ui::i18n::text(&locale, "utility.grid"),
        );
        ui.checkbox(
            &mut state.snap_to_objects,
            crate::ui::i18n::text(&locale, "utility.objects"),
        );
        ui.checkbox(
            &mut state.snap_to_guides,
            crate::ui::i18n::text(&locale, "utility.guides"),
        );
        ui.checkbox(
            &mut state.snap_to_points,
            crate::ui::i18n::text(&locale, "utility.anchor_points"),
        );
        ui.checkbox(
            &mut state.snap_to_pixels,
            crate::ui::i18n::text(&locale, "utility.pixels"),
        );

        ui.add_space(4.0);
        ui.separator();

        // Grid settings
        ui.label(RichText::new(crate::ui::i18n::text(&locale, "utility.grid")).strong());
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "utility.spacing"));
            ui.add(
                egui::DragValue::new(&mut state.grid_size)
                    .speed(1.0)
                    .range(1.0..=100.0)
                    .suffix("px"),
            );
        });

        ui.add_space(4.0);
        ui.separator();

        // Guides
        ui.label(RichText::new(crate::ui::i18n::text(&locale, "utility.custom_guides")).strong());
        ui.horizontal(|ui| {
            if ui
                .button(crate::ui::i18n::text(
                    &locale,
                    "utility.add_horizontal_guide",
                ))
                .clicked()
            {
                state.guides.push(crate::core::state::Guide {
                    orientation: crate::core::state::GuideOrientation::Horizontal,
                    position: state.pan_y as f64 / state.zoom as f64,
                });
            }
            if ui
                .button(crate::ui::i18n::text(&locale, "utility.add_vertical_guide"))
                .clicked()
            {
                state.guides.push(crate::core::state::Guide {
                    orientation: crate::core::state::GuideOrientation::Vertical,
                    position: state.pan_x as f64 / state.zoom as f64,
                });
            }
        });

        if !state.guides.is_empty() {
            ui.add_space(2.0);
            let mut to_remove = None;
            for (i, guide) in state.guides.iter().enumerate() {
                ui.horizontal(|ui| {
                    let orient = match guide.orientation {
                        crate::core::state::GuideOrientation::Horizontal => "H",
                        crate::core::state::GuideOrientation::Vertical => "V",
                    };
                    ui.label(format!("{}: {:.1}", orient, guide.position));
                    if ui.small_button("×").clicked() {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(idx) = to_remove {
                state.guides.remove(idx);
            }
            if ui
                .button(crate::ui::i18n::text(&locale, "utility.delete_all"))
                .clicked()
            {
                state.guides.clear();
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// ExportPanel: Export to PNG/SVG/WebP/AVIF/JSON
// ═══════════════════════════════════════════════════════════════════

pub struct ExportPanel;

impl ExportPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.heading(RichText::new(crate::ui::i18n::text(&locale, "utility.export")).strong());
        ui.add_space(4.0);

        // Export format
        ui.label(crate::ui::i18n::text(&locale, "utility.format"));
        let mut format = state.export_format.clone();

        ui.horizontal_wrapped(|ui| {
            for f in ["SVG", "PNG", "WEBP", "AVIF", "JSON"] {
                if ui.selectable_label(format == f, f).clicked() {
                    format = f.into();
                    state.export_format = format.clone();
                }
            }
        });

        ui.add_space(4.0);

        // Export settings
        match format.as_str() {
            "PNG" => {
                ui.horizontal(|ui| {
                    ui.label(crate::ui::i18n::text(&locale, "utility.width"));
                    ui.add(egui::DragValue::new(&mut state.export_width).range(16.0..=8192.0));
                    ui.label(crate::ui::i18n::text(&locale, "utility.height"));
                    ui.add(egui::DragValue::new(&mut state.export_height).range(16.0..=8192.0));
                });
                ui.horizontal(|ui| {
                    ui.label(crate::ui::i18n::text(&locale, "utility.scale"));
                    ui.add(egui::Slider::new(&mut state.export_scale, 0.1..=4.0).show_value(true));
                });
                ui.checkbox(
                    &mut state.export_transparent,
                    crate::ui::i18n::text(&locale, "utility.transparent_bg"),
                );
                outline_checkbox(ui, state, &locale);
            }
            "SVG" => {
                ui.checkbox(
                    &mut state.export_svg_viewbox,
                    crate::ui::i18n::text(&locale, "utility.viewbox"),
                );
                ui.checkbox(
                    &mut state.export_svg_embed_fonts,
                    crate::ui::i18n::text(&locale, "utility.embed_fonts"),
                );
                outline_checkbox(ui, state, &locale);
            }
            _ => {}
        }

        ui.add_space(4.0);
        ui.separator();

        // Export scope
        ui.label(crate::ui::i18n::text(&locale, "utility.range"));
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(
                    state.export_scope == "All",
                    crate::ui::i18n::text(&locale, "utility.all_objects"),
                )
                .clicked()
            {
                state.export_scope = "All".into();
            }
            if ui
                .selectable_label(
                    state.export_scope == "Selected",
                    crate::ui::i18n::text(&locale, "utility.selected_objects"),
                )
                .clicked()
            {
                state.export_scope = "Selected".into();
            }
        });

        ui.add_space(8.0);

        // Export button
        if ui
            .button(crate::ui::i18n::text(&locale, "utility.export_button"))
            .clicked()
        {
            let filter = match format.as_str() {
                "JSON" => &["json"][..],
                "PNG" => &["png"][..],
                "WEBP" => &["webp"][..],
                "AVIF" => &["avif"][..],
                _ => &["svg"][..],
            };
            if let Some(path) = rfd::FileDialog::new()
                .set_title(crate::ui::i18n::text(&locale, "utility.export_dialog_title").as_ref())
                .add_filter(format.as_str(), filter)
                .save_file()
            {
                state.sync_doc_extras();
                // "Selected Only" previously did nothing and exported the
                // whole document anyway.
                let mut export_doc;
                let doc_ref = if state.export_scope == "Selected" && !state.selected_ids.is_empty()
                {
                    export_doc = state.document.clone();
                    for layer in &mut export_doc.layers {
                        layer.objects.retain(|o| state.selected_ids.contains(&o.id));
                    }
                    &export_doc
                } else {
                    &state.document
                };
                if format == "SVG" {
                    let svg = crate::io::svg::export_svg_with_options(
                        doc_ref,
                        state.export_svg_embed_fonts,
                        None,
                        state.export_outline_text,
                    );
                    match crate::io::atomic::atomic_write_str(&path, &svg) {
                        Ok(_) => state.notify_info(
                            crate::ui::i18n::text(&locale, "utility.svg_exported").into_owned(),
                        ),
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "utility.svg_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                } else if format == "PNG" {
                    match crate::io::raster::export_png_with_outline(
                        doc_ref,
                        state.export_scale,
                        state.export_transparent,
                        state.export_outline_text,
                    ) {
                        Ok(png_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &png_bytes) {
                                Ok(_) => state.notify_info(crate::ui::i18n::format(
                                    &locale,
                                    "utility.raster_exported",
                                    &[("format", "PNG")],
                                )),
                                Err(e) => state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "utility.save_failed",
                                    &[("format", "PNG"), ("error", &e.to_string())],
                                )),
                            }
                        }
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "utility.raster_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                } else if format == "WEBP" {
                    match crate::io::raster::export_webp(
                        doc_ref,
                        state.export_scale,
                        state.export_transparent,
                    ) {
                        Ok(webp_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &webp_bytes) {
                                Ok(_) => state.notify_info(crate::ui::i18n::format(
                                    &locale,
                                    "utility.raster_exported",
                                    &[("format", "WebP")],
                                )),
                                Err(e) => state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "utility.save_failed",
                                    &[("format", "WebP"), ("error", &e.to_string())],
                                )),
                            }
                        }
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "utility.raster_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                } else if format == "AVIF" {
                    match crate::io::raster::export_avif(
                        doc_ref,
                        state.export_scale,
                        state.export_transparent,
                    ) {
                        Ok(avif_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &avif_bytes) {
                                Ok(_) => state.notify_info(crate::ui::i18n::format(
                                    &locale,
                                    "utility.raster_exported",
                                    &[("format", "AVIF")],
                                )),
                                Err(e) => state.notify_error(crate::ui::i18n::format(
                                    &locale,
                                    "utility.save_failed",
                                    &[("format", "AVIF"), ("error", &e.to_string())],
                                )),
                            }
                        }
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "utility.raster_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                } else if format == "JSON" {
                    match serde_json::to_string_pretty(doc_ref) {
                        Ok(json) => match crate::io::atomic::atomic_write_str(&path, &json) {
                            Ok(_) => state.notify_info(
                                crate::ui::i18n::text(&locale, "utility.json_saved").into_owned(),
                            ),
                            Err(e) => state.notify_error(crate::ui::i18n::format(
                                &locale,
                                "utility.json_save_failed",
                                &[("error", &e.to_string())],
                            )),
                        },
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "utility.serialize_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                }
                state.export_path = Some(path.to_string_lossy().to_string());
                state.pending_export = false;
            }
        }

        if let Some(ref p) = state.export_path {
            ui.label(RichText::new(format!("→ {}", p)).weak().size(10.0));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// GridRepeatPanel: Grid and radial repeat
// ═══════════════════════════════════════════════════════════════════

pub struct ShortcutsHelpPanel;

impl ShortcutsHelpPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.heading(RichText::new(crate::ui::i18n::text(&locale, "shortcuts.title")).strong());
        ui.add_space(4.0);

        let mk = crate::app::control_bar::mod_key();
        let shortcuts: [(&str, &str); 32] = [
            ("V", "shortcut.select_tool"),
            ("A", "shortcut.node_tool"),
            ("P", "shortcut.pen_tool"),
            ("N", "shortcut.pencil_tool"),
            ("U", "shortcut.rectangle_tool"),
            ("O", "shortcut.ellipse_tool"),
            ("S", "shortcut.star_tool"),
            ("G", "shortcut.polygon_tool"),
            ("L", "shortcut.line_tool"),
            ("T", "shortcut.text_tool"),
            ("I", "shortcut.eyedropper_tool"),
            ("H", "shortcut.hand_tool"),
            ("B", "shortcut.brush_tool"),
            ("E", "shortcut.eraser_tool"),
            ("D", "shortcut.default_colors"),
            ("/", "shortcut.no_fill"),
            ("Shift+X", "shortcut.swap_fill_stroke"),
            ("Delete", "shortcut.delete_selection"),
            ("Escape", "shortcut.cancel"),
            ("Enter", "shortcut.commit_pen"),
            ("__MK__+Z", "shortcut.undo"),
            ("__MK__+Y", "shortcut.redo"),
            ("__MK__+A", "shortcut.select_all"),
            ("__MK__+G", "shortcut.group"),
            ("__MK__+Shift+G", "shortcut.ungroup"),
            ("__MK__+D", "shortcut.duplicate"),
            ("__MK__+C", "shortcut.copy"),
            ("__MK__+V", "shortcut.paste"),
            ("__MK__+0", "shortcut.fit_view"),
            ("__MK__+1", "shortcut.zoom_100"),
            ("__MK__+7", "shortcut.clipping_mask"),
            ("__ARROWS__", "shortcut.nudge"),
        ];

        for (key, action) in shortcuts {
            let key = if key == "__ARROWS__" {
                crate::ui::i18n::text(&locale, "shortcut.arrow_keys").into_owned()
            } else {
                key.replace("__MK__", mk)
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(key).strong().monospace().size(11.0));
                ui.separator();
                ui.label(RichText::new(crate::ui::i18n::text(&locale, action)).size(11.0));
            });
        }
    }
}
