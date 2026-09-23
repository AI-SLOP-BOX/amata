use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct PresetPanel;

impl PresetPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📦 Asset Library & Presets").strong());
        ui.add_space(4.0);

        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        ui.horizontal_wrapped(|ui| {
            if ui
                .button("❤️ Heart")
                .on_hover_text("Add Heart shape")
                .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::heart("Heart", cx, cy, 120.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui
                .button("➡️ Arrow")
                .on_hover_text("Add Arrow symbol")
                .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::arrow("Arrow", cx, cy, 160.0, 40.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui
                .button("⚙️ Gear")
                .on_hover_text("Add Cog / Gear")
                .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::gear("Gear", cx, cy, 8, 40.0, 60.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if ui
                .button("💬 Speech")
                .on_hover_text("Add Speech Bubble")
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

            if ui
                .button("🌀 VFX Portal")
                .on_hover_text("Add Sci-Fi Hexagonal VFX Ring")
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
        ui.heading(RichText::new("📏 Smart Guides").strong());
        ui.add_space(4.0);

        // Snapping options
        ui.label(RichText::new("Snap To:").strong());
        ui.checkbox(&mut state.snap_to_grid, "Grid");
        ui.checkbox(&mut state.snap_to_objects, "Objects");
        ui.checkbox(&mut state.snap_to_guides, "Guides");
        ui.checkbox(&mut state.snap_to_points, "Anchor Points");
        ui.checkbox(&mut state.snap_to_pixels, "Pixels (integer units)");

        ui.add_space(4.0);
        ui.separator();

        // Grid settings
        ui.label(RichText::new("Grid").strong());
        ui.horizontal(|ui| {
            ui.label("Size:");
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
        ui.label(RichText::new("Custom Guides").strong());
        ui.horizontal(|ui| {
            if ui.button("Add H Guide").clicked() {
                state.guides.push(crate::core::state::Guide {
                    orientation: crate::core::state::GuideOrientation::Horizontal,
                    position: state.pan_y as f64 / state.zoom as f64,
                });
            }
            if ui.button("Add V Guide").clicked() {
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
                    if ui.small_button("✕").clicked() {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(idx) = to_remove {
                state.guides.remove(idx);
            }
            if ui.button("Clear All").clicked() {
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
        ui.heading(RichText::new("📤 Export").strong());
        ui.add_space(4.0);

        // Export format
        ui.label("Format:");
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
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut state.export_width).range(16.0..=8192.0));
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut state.export_height).range(16.0..=8192.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    ui.add(egui::Slider::new(&mut state.export_scale, 0.1..=4.0).show_value(true));
                });
                ui.checkbox(&mut state.export_transparent, "Transparent Background");
            }
            "SVG" => {
                ui.checkbox(&mut state.export_svg_viewbox, "Include ViewBox");
                ui.checkbox(&mut state.export_svg_embed_fonts, "Embed Fonts");
            }
            _ => {}
        }

        ui.add_space(4.0);
        ui.separator();

        // Export scope
        ui.label("Scope:");
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(state.export_scope == "All", "All Objects")
                .clicked()
            {
                state.export_scope = "All".into();
            }
            if ui
                .selectable_label(state.export_scope == "Selected", "Selected Only")
                .clicked()
            {
                state.export_scope = "Selected".into();
            }
        });

        ui.add_space(8.0);

        // Export button
        if ui.button("Export...").clicked() {
            let filter = match format.as_str() {
                "JSON" => &["json"][..],
                "PNG" => &["png"][..],
                "WEBP" => &["webp"][..],
                "AVIF" => &["avif"][..],
                _ => &["svg"][..],
            };
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Export As")
                .add_filter(format.as_str(), filter)
                .save_file()
            {
                state.sync_doc_extras();
                // "Selected Only" previously did nothing and exported the
                // whole document anyway.
                let mut export_doc;
                let doc_ref = if state.export_scope == "Selected"
                    && !state.selected_ids.is_empty()
                {
                    export_doc = state.document.clone();
                    for layer in &mut export_doc.layers {
                        layer
                            .objects
                            .retain(|o| state.selected_ids.contains(&o.id));
                    }
                    &export_doc
                } else {
                    &state.document
                };
                if format == "SVG" {
                    let svg = crate::io::svg::export_svg(doc_ref);
                    match crate::io::atomic::atomic_write_str(&path, &svg) {
                        Ok(_) => state.notify_info("SVGを書き出しました"),
                        Err(e) => state.notify_error(format!("SVG書き出しに失敗しました: {e}")),
                    }
                } else if format == "PNG" {
                    match crate::io::raster::export_png(
                        doc_ref,
                        state.export_scale,
                        state.export_transparent,
                    ) {
                        Ok(png_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &png_bytes) {
                                Ok(_) => state.notify_info("PNGを書き出しました"),
                                Err(e) => {
                                    state.notify_error(format!("PNG保存に失敗しました: {e}"))
                                }
                            }
                        }
                        Err(e) => state.notify_error(format!("ラスタライズに失敗しました: {e}")),
                    }
                } else if format == "WEBP" {
                    match crate::io::raster::export_webp(
                        doc_ref,
                        state.export_scale,
                        state.export_transparent,
                    ) {
                        Ok(webp_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &webp_bytes) {
                                Ok(_) => state.notify_info("WebPを書き出しました"),
                                Err(e) => {
                                    state.notify_error(format!("WebP保存に失敗しました: {e}"))
                                }
                            }
                        }
                        Err(e) => state.notify_error(format!("ラスタライズに失敗しました: {e}")),
                    }
                } else if format == "AVIF" {
                    match crate::io::raster::export_avif(
                        doc_ref,
                        state.export_scale,
                        state.export_transparent,
                    ) {
                        Ok(avif_bytes) => {
                            match crate::io::atomic::atomic_write_bytes(&path, &avif_bytes) {
                                Ok(_) => state.notify_info("AVIFを書き出しました"),
                                Err(e) => {
                                    state.notify_error(format!("AVIF保存に失敗しました: {e}"))
                                }
                            }
                        }
                        Err(e) => state.notify_error(format!("ラスタライズに失敗しました: {e}")),
                    }
                } else if format == "JSON" {
                    match serde_json::to_string_pretty(doc_ref) {
                        Ok(json) => match crate::io::atomic::atomic_write_str(&path, &json) {
                            Ok(_) => state.notify_info("JSONを保存しました"),
                            Err(e) => state.notify_error(format!("保存に失敗しました: {e}")),
                        },
                        Err(e) => state.notify_error(format!("シリアライズに失敗しました: {e}")),
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
    pub fn show(ui: &mut Ui, _state: &mut AppState) {
        ui.heading(RichText::new("⌨ Keyboard Shortcuts").strong());
        ui.add_space(4.0);

        let mk = crate::app::control_bar::mod_key();
        let shortcuts: [(&str, &str); 32] = [
            ("V", "Select Tool"),
            ("A", "Node / Direct Select"),
            ("P", "Pen Tool"),
            ("N", "Pencil Tool"),
            ("U", "Rectangle Tool"),
            ("O", "Ellipse Tool"),
            ("S", "Star Tool"),
            ("G", "Polygon Tool"),
            ("L", "Line Tool"),
            ("T", "Text Tool"),
            ("I", "Eyedropper"),
            ("H", "Hand / Pan"),
            ("B", "Brush Tool"),
            ("E", "Eraser Tool"),
            ("D", "Default Fill & Stroke"),
            ("/", "Set Fill to None"),
            ("Shift+X", "Swap Fill & Stroke"),
            ("Delete", "Delete Selected"),
            ("Escape", "Deselect / Cancel"),
            ("Enter", "Finish Pen Path"),
            ("__MK__+Z", "Undo"),
            ("__MK__+Y", "Redo"),
            ("__MK__+A", "Select All"),
            ("__MK__+G", "Group"),
            ("__MK__+Shift+G", "Ungroup"),
            ("__MK__+D", "Duplicate"),
            ("__MK__+C", "Copy"),
            ("__MK__+V", "Paste"),
            ("__MK__+0", "Zoom to Fit"),
            ("__MK__+1", "Zoom 100%"),
            ("__MK__+7", "Clipping Mask"),
            ("Arrow Keys", "Nudge (Shift=10x)"),
        ];

        for (key, action) in shortcuts {
            let key = key.replace("__MK__", mk);
            ui.horizontal(|ui| {
                ui.label(RichText::new(key).strong().monospace().size(11.0));
                ui.separator();
                ui.label(RichText::new(action).size(11.0));
            });
        }
    }
}
