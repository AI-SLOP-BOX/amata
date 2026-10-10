//! Print panel: color mode, bleed, marks, spot library, preflight and
//! press-PDF export.

use crate::core::document::ColorMode;
use crate::core::print::{self, PreflightLevel};
use crate::core::state::AppState;
use egui::{Color32, RichText, Ui};

pub struct PrintPanel;

/// (Re)generate spread traps on the Traps layer (undoable, one step).
fn generate_traps(state: &mut AppState) {
    use crate::core::document::Layer;
    use crate::core::history::{AddLayerCommand, BatchCommand, Command, RemoveLayerCommand};
    use crate::core::trap::{find_trap_strokes, trap_object, TRAP_LAYER_NAME};
    let locale = state.prefs.language.clone();
    let width = state.document.trap_width;
    if width <= 0.0 {
        state.notify_info(crate::ui::i18n::text(&locale, "print.trap_width_error").into_owned());
        return;
    }
    let (strokes, skipped) = find_trap_strokes(&state.document, width);
    if strokes.is_empty() {
        let mut msg = crate::ui::i18n::text(&locale, "print.no_shared_edges").into_owned();
        if skipped > 0 {
            msg.push_str(&crate::ui::i18n::format(
                &locale,
                "print.skipped_spot_edges",
                &[("count", &skipped.to_string())],
            ));
        }
        state.notify_info(msg);
        return;
    }
    let mut cmds: Vec<Box<dyn Command>> = Vec::new();
    // Regeneration keeps the old slot so a manually-ordered Traps layer
    // does not jump to the end on every rebuild.
    let old_pos = state
        .document
        .layers
        .iter()
        .position(|l| l.name == TRAP_LAYER_NAME);
    if let Some(pos) = old_pos {
        cmds.push(Box::new(RemoveLayerCommand {
            layer: state.document.layers[pos].clone(),
            index: pos,
        }));
    }
    let mut layer = Layer::new(TRAP_LAYER_NAME);
    for (i, s) in strokes.iter().enumerate() {
        layer.objects.push(trap_object(s, width, i + 1));
    }
    let n = strokes.len();
    cmds.push(Box::new(AddLayerCommand {
        layer,
        index: old_pos.unwrap_or(state.document.layers.len()),
        prev_active: state.document.active_layer_idx,
    }));
    state.undo_manager.execute(
        Box::new(BatchCommand::new("Generate Traps", cmds)),
        &mut state.document,
    );
    let mut msg =
        crate::ui::i18n::format(&locale, "print.traps_placed", &[("count", &n.to_string())]);
    if skipped > 0 {
        msg.push_str(&crate::ui::i18n::format(
            &locale,
            "print.skipped_spot_edges",
            &[("count", &skipped.to_string())],
        ));
    }
    state.notify_success(msg);
}

/// Delete the Traps layer (undoable).
fn remove_traps(state: &mut AppState) {
    use crate::core::history::RemoveLayerCommand;
    use crate::core::trap::TRAP_LAYER_NAME;
    let locale = state.prefs.language.clone();
    let Some(pos) = state
        .document
        .layers
        .iter()
        .position(|l| l.name == TRAP_LAYER_NAME)
    else {
        state.notify_info(crate::ui::i18n::text(&locale, "print.no_traps").into_owned());
        return;
    };
    let cmd = RemoveLayerCommand {
        layer: state.document.layers[pos].clone(),
        index: pos,
    };
    state.undo_manager.execute(
        Box::new(cmd) as Box<dyn crate::core::history::Command>,
        &mut state.document,
    );
    state.notify_success(crate::ui::i18n::text(&locale, "print.traps_removed").into_owned());
}

impl PrintPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.heading(RichText::new(crate::ui::i18n::text(&locale, "print.title")).strong());
        ui.add_space(4.0);

        // Color mode + bleed.
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "print.mode"));
            let cmyk = state.document.color_mode == ColorMode::Cmyk;
            if ui
                .selectable_label(cmyk, "CMYK")
                .on_hover_text(crate::ui::i18n::text(&locale, "print.cmyk_tip"))
                .clicked()
            {
                state.document.color_mode = ColorMode::Cmyk;
                state.notify_info(crate::ui::i18n::text(&locale, "print.cmyk_enabled"));
            }
            if ui
                .selectable_label(!cmyk, "RGB")
                .on_hover_text(crate::ui::i18n::text(&locale, "print.rgb_tip"))
                .clicked()
            {
                state.document.color_mode = ColorMode::Rgb;
            }
        });
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "print.bleed"));
            let mut mm = state.document.bleed * 25.4 / 72.0;
            if ui
                .add(
                    egui::DragValue::new(&mut mm)
                        .speed(0.1)
                        .range(0.0..=20.0)
                        .suffix("mm"),
                )
                .changed()
            {
                state.document.bleed = (mm * 72.0 / 25.4).max(0.0);
            }
        });
        ui.checkbox(
            &mut state.print_marks,
            crate::ui::i18n::text(&locale, "print.marks"),
        );
        ui.checkbox(
            &mut state.export_outline_text,
            crate::ui::i18n::text(&locale, "print.outline_text"),
        )
        .on_hover_text(crate::ui::i18n::text(&locale, "utility.outline_text_tip"));
        ui.checkbox(
            &mut state.print_pdfx,
            crate::ui::i18n::text(&locale, "print.pdfx"),
        );
        ui.add_space(4.0);

        // Separations preview: pick which plate the canvas shows.
        // Preview-only; the document is untouched.
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "print.separations_preview"));
            let mut plates: Vec<print::PreviewPlate> = vec![
                print::PreviewPlate::Composite,
                print::PreviewPlate::Cyan,
                print::PreviewPlate::Magenta,
                print::PreviewPlate::Yellow,
                print::PreviewPlate::Black,
            ];
            for spot in &state.document.spots {
                plates.push(print::PreviewPlate::Spot(spot.name.clone()));
            }
            // A deleted spot falls back to composite instead of sticking
            // on a plate that no longer exists.
            if !plates.contains(&state.preview_plate) {
                state.preview_plate = print::PreviewPlate::Composite;
            }
            egui::ComboBox::from_id_salt("print_plate")
                .selected_text(state.preview_plate.label())
                .show_ui(ui, |ui| {
                    for plate in plates {
                        if ui
                            .selectable_label(state.preview_plate == plate, plate.label())
                            .clicked()
                        {
                            state.preview_plate = plate;
                        }
                    }
                });
        });
        if state.preview_plate != print::PreviewPlate::Composite {
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "print.preview_notice"))
                    .weak()
                    .size(10.0),
            );
        }
        ui.add_space(4.0);
        ui.separator();

        // Spot library.
        ui.label(RichText::new(crate::ui::i18n::text(&locale, "print.spot_library")).strong());
        let mut delete: Option<String> = None;
        for spot in state.document.spots.clone() {
            ui.horizontal(|ui| {
                let rgb = spot.preview_rgb();
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(16.0), egui::Sense::hover());
                if ui.is_rect_visible(rect) {
                    ui.painter().rect_filled(
                        rect,
                        2.0,
                        Color32::from_rgba_unmultiplied(
                            (rgb[0] * 255.0) as u8,
                            (rgb[1] * 255.0) as u8,
                            (rgb[2] * 255.0) as u8,
                            255,
                        ),
                    );
                }
                ui.label(format!(
                    "{} ({:.0}/{:.0}/{:.0}/{:.0})",
                    spot.name,
                    spot.cmyk[0] * 100.0,
                    spot.cmyk[1] * 100.0,
                    spot.cmyk[2] * 100.0,
                    spot.cmyk[3] * 100.0
                ));
                if ui.small_button("×").clicked() {
                    delete = Some(spot.name.clone());
                }
            });
        }
        if let Some(name) = delete {
            state.document.spots.retain(|s| s.name != name);
            // Dangling references fall back to process color (exporter +
            // preflight handle them); nothing else to repair.
            state.notify_info(crate::ui::i18n::format(
                &locale,
                "print.spot_deleted",
                &[("name", &name)],
            ));
        }
        if ui
            .button(crate::ui::i18n::text(&locale, "print.register_fill_spot"))
            .clicked()
        {
            let c = state.fill_color;
            // Register with the same ink the exporter would write, so the
            // spot's process fallback and the plates cannot drift apart.
            let ink = crate::core::icc::rgb_to_cmyk([c[0], c[1], c[2], 1.0]);
            let n = state.document.spots.len() + 1;
            let name =
                crate::ui::i18n::format(&locale, "print.spot_name", &[("count", &n.to_string())]);
            state
                .document
                .spots
                .push(print::SpotColor::new(name.clone(), ink));
            state.notify_success(crate::ui::i18n::format(
                &locale,
                "print.spot_registered",
                &[("name", &name)],
            ));
        }
        // Pantone kit lookup: kit ink when the formula is known, the
        // current fill as the honest fallback otherwise.
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "print.pantone_name"));
            let resp = ui.add(
                egui::TextEdit::singleline(&mut state.spot_kit_query)
                    .desired_width(150.0)
                    .hint_text("Pantone 185 C"),
            );
            if resp.changed() {
                state.spot_kit_last_hit =
                    crate::core::print::pantone_display_name(state.spot_kit_query.trim());
            }
            if let Some(hit) = state.spot_kit_last_hit {
                if let Some(cmyk) = crate::core::print::pantone_cmyk(hit) {
                    let ink = cmyk
                        .iter()
                        .take(3)
                        .map(|c| (c * 100.0).round() as i32)
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join("/");
                    ui.label(crate::ui::i18n::format(
                        &locale,
                        "print.pantone_hit",
                        &[("ink", &ink)],
                    ));
                }
            }
            if ui
                .button(crate::ui::i18n::text(&locale, "print.pantone_register"))
                .clicked()
            {
                let q = state.spot_kit_query.trim().to_string();
                if !q.is_empty() {
                    let (name, ink, known) = match crate::core::print::pantone_display_name(&q) {
                        Some(display) => {
                            let cmyk = crate::core::print::pantone_cmyk(&q).unwrap_or([0.0; 4]);
                            (display.to_string(), cmyk, true)
                        }
                        None => {
                            let c = state.fill_color;
                            (
                                q.clone(),
                                crate::core::icc::rgb_to_cmyk([c[0], c[1], c[2], 1.0]),
                                false,
                            )
                        }
                    };
                    if !state.document.spots.iter().any(|s| s.name == name) {
                        state
                            .document
                            .spots
                            .push(print::SpotColor::new(name.clone(), ink));
                    }
                    if known {
                        let ink_txt = ink
                            .iter()
                            .map(|c| (c * 100.0).round() as i32)
                            .map(|c| c.to_string())
                            .collect::<Vec<_>>()
                            .join("/");
                        state.notify_success(crate::ui::i18n::format(
                            &locale,
                            "print.pantone_registered",
                            &[("name", &name), ("ink", &ink_txt)],
                        ));
                    } else {
                        state.notify_info(crate::ui::i18n::format(
                            &locale,
                            "print.pantone_unknown",
                            &[("name", &q)],
                        ));
                    }
                }
            }
        });
        // Adobe Swatch Exchange import (read-only subset; see io::ase).
        if ui
            .button(crate::ui::i18n::text(&locale, "print.ase_import"))
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Adobe Swatch Exchange", &["ase"])
                .pick_file()
            {
                match crate::io::ase::import_ase(&path) {
                    Ok(spots) if spots.is_empty() => {
                        state.notify_info(crate::ui::i18n::format(
                            &locale,
                            "print.ase_imported",
                            &[("count", "0")],
                        ));
                    }
                    Ok(spots) => {
                        let n = spots.len();
                        for spot in spots {
                            if !state.document.spots.iter().any(|s| s.name == spot.name) {
                                state.document.spots.push(spot);
                            }
                        }
                        state.notify_success(crate::ui::i18n::format(
                            &locale,
                            "print.ase_imported",
                            &[("count", &n.to_string())],
                        ));
                    }
                    Err(e) => {
                        state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "print.ase_failed",
                            &[("reason", &e.to_string())],
                        ));
                    }
                }
            }
        }
        // Bundled swatch libraries (our own traditional-color set).
        if !crate::io::library::built_in_swatches().is_empty() {
            ui.label(crate::ui::i18n::text(&locale, "print.swatch_libraries"));
            for lib in crate::io::library::built_in_swatches() {
                if ui.button(&lib.name).clicked() {
                    let mut added = 0usize;
                    for spot in lib.to_spots() {
                        if !state.document.spots.iter().any(|s| s.name == spot.name) {
                            state.document.spots.push(spot);
                            added += 1;
                        }
                    }
                    state.notify_success(crate::ui::i18n::format(
                        &locale,
                        "print.swatch_imported",
                        &[("name", &lib.name), ("count", &added.to_string())],
                    ));
                }
            }
        }
        ui.add_space(4.0);
        ui.separator();

        // Trapping: spread strokes along shared edges.
        ui.label(RichText::new(crate::ui::i18n::text(&locale, "print.trapping")).strong());
        ui.horizontal(|ui| {
            // Direct document-field edit (same convention as bleed/mode
            // above): scalar doc settings are not undo-tracked anywhere.
            ui.label(crate::ui::i18n::text(&locale, "print.width"));
            ui.add(
                egui::DragValue::new(&mut state.document.trap_width)
                    .speed(0.05)
                    .range(0.0..=3.0)
                    .suffix("pt"),
            );
        });
        let trap_count: usize = state
            .document
            .layers
            .iter()
            .filter(|l| l.name == crate::core::trap::TRAP_LAYER_NAME)
            .map(|l| l.objects.len())
            .sum();
        ui.horizontal(|ui| {
            if ui
                .button(crate::ui::i18n::text(&locale, "print.generate_traps"))
                .on_hover_text(crate::ui::i18n::text(&locale, "print.generate_traps_tip"))
                .clicked()
            {
                generate_traps(state);
            }
            if ui
                .button(crate::ui::i18n::text(&locale, "print.remove_traps"))
                .clicked()
            {
                remove_traps(state);
            }
        });
        ui.label(
            RichText::new(crate::ui::i18n::format(
                &locale,
                "print.traps_count",
                &[("count", &trap_count.to_string())],
            ))
            .weak()
            .size(10.0),
        );
        ui.add_space(4.0);
        ui.separator();

        // Preflight.
        ui.label(RichText::new(crate::ui::i18n::text(&locale, "print.preflight")).strong());
        for issue in print::preflight(&state.document) {
            let (icon, color) = match issue.level {
                PreflightLevel::Pass => ("●", Color32::from_rgb(80, 200, 120)),
                PreflightLevel::Warn => ("●", Color32::from_rgb(255, 190, 60)),
                PreflightLevel::Fail => ("●", Color32::from_rgb(235, 90, 90)),
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).color(color));
                ui.label(format!("{}: {}", issue.check, issue.detail));
            });
        }
        ui.add_space(4.0);

        // Export.
        if ui
            .button(crate::ui::i18n::text(&locale, "print.export_pdf"))
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("print.pdf")
                .save_file()
            {
                let opts = crate::io::pdf_print::PrintPdfOptions {
                    marks: state.print_marks,
                    bleed: None,
                    pdfx: state.print_pdfx,
                    outline_text: state.export_outline_text,
                };
                let (bytes, warnings) =
                    crate::io::pdf_print::export_pdf_print(&state.document, &opts);
                // X-1a gate: a file tagged PDF/X-1a must not contain live
                // transparency or RGB plates. Refuse to write it and say
                // exactly why, instead of shipping a bogus compliant file.
                // (Uncheck PDF/X-1a to export the same content untagged.)
                if opts.pdfx && warnings.iter().any(|w| w.starts_with("PDF/X-1a違反")) {
                    state.notify_error(crate::ui::i18n::format(
                        &locale,
                        "print.pdfx_violation",
                        &[("details", &warnings.join(" / "))],
                    ));
                } else {
                    match crate::io::atomic::atomic_write_bytes(&path, &bytes) {
                        Ok(_) => {
                            let mut msg = crate::ui::i18n::format(
                                &locale,
                                "print.pdf_exported",
                                &[("size", &(bytes.len() / 1024).to_string())],
                            );
                            if !warnings.is_empty() {
                                msg.push_str(&format!(" — {}", warnings.join(" / ")));
                            }
                            state.notify_success(msg);
                        }
                        Err(e) => state.notify_error(crate::ui::i18n::format(
                            &locale,
                            "print.export_failed",
                            &[("error", &e.to_string())],
                        )),
                    }
                }
            }
        }
    }
}
