//! Print panel: color mode, bleed, marks, spot library, preflight and
//! press-PDF export.

use crate::core::document::ColorMode;
use crate::core::print::{self, PreflightLevel};
use crate::core::state::AppState;
use egui::{Color32, RichText, Ui};

pub struct PrintPanel;

impl PrintPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("🖨 Print").strong());
        ui.add_space(4.0);

        // Color mode + bleed.
        ui.horizontal(|ui| {
            ui.label("Mode:");
            let cmyk = state.document.color_mode == ColorMode::Cmyk;
            if ui
                .selectable_label(cmyk, "CMYK")
                .on_hover_text("印刷用（書き出し時にCMYK/特色で出力）")
                .clicked()
            {
                state.document.color_mode = ColorMode::Cmyk;
                state.notify_info("CMYKモードに切り替えました");
            }
            if ui
                .selectable_label(!cmyk, "RGB")
                .on_hover_text("画面用")
                .clicked()
            {
                state.document.color_mode = ColorMode::Rgb;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Bleed:");
            let mut mm = state.document.bleed * 25.4 / 72.0;
            if ui
                .add(egui::DragValue::new(&mut mm).speed(0.1).range(0.0..=20.0).suffix("mm"))
                .changed()
            {
                state.document.bleed = (mm * 72.0 / 25.4).max(0.0);
            }
        });
        ui.checkbox(&mut state.print_marks, "トンボ・レジスターマーク");
        ui.checkbox(&mut state.print_pdfx, "PDF/X-1a互換出力");
        ui.add_space(4.0);
        ui.separator();

        // Spot library.
        ui.label(RichText::new("特色ライブラリ").strong());
        let mut delete: Option<String> = None;
        for spot in state.document.spots.clone() {
            ui.horizontal(|ui| {
                let rgb = spot.preview_rgb();
                let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(16.0), egui::Sense::hover());
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
                if ui.small_button("✕").clicked() {
                    delete = Some(spot.name.clone());
                }
            });
        }
        if let Some(name) = delete {
            state.document.spots.retain(|s| s.name != name);
            // Dangling references fall back to process color (exporter +
            // preflight handle them); nothing else to repair.
            state.notify_info(format!("特色「{name}」を削除しました"));
        }
        if ui.button("現在の塗り色を特色登録").clicked() {
            let c = state.fill_color;
            // Register with the same ink the exporter would write, so the
            // spot's process fallback and the plates cannot drift apart.
            let ink = crate::core::icc::rgb_to_cmyk([c[0], c[1], c[2], 1.0]);
            let n = state.document.spots.len() + 1;
            let name = format!("Spot {n}");
            state.document.spots.push(print::SpotColor::new(name.clone(), ink));
            state.notify_success(format!("特色「{name}」を登録しました"));
        }
        ui.add_space(4.0);
        ui.separator();

        // Preflight.
        ui.label(RichText::new("プリフライト").strong());
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
        if ui.button("印刷用PDFを書き出し").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("print.pdf")
                .save_file()
            {
                let opts = crate::io::pdf_print::PrintPdfOptions {
                    marks: state.print_marks,
                    bleed: None,
                    pdfx: state.print_pdfx,
                };
                let (bytes, warnings) =
                    crate::io::pdf_print::export_pdf_print(&state.document, &opts);
                match crate::io::atomic::atomic_write_bytes(&path, &bytes) {
                    Ok(_) => {
                        let mut msg = format!("印刷用PDFを書き出しました ({}KB)", bytes.len() / 1024);
                        if !warnings.is_empty() {
                            msg.push_str(&format!(" — {}", warnings.join(" / ")));
                        }
                        state.notify_success(msg);
                    }
                    Err(e) => state.notify_error(format!("書き出しに失敗しました: {e}")),
                }
            }
        }
    }
}
