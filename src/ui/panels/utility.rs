use crate::app::icons::{
    icon_arrow, icon_gear, icon_heart, icon_portal, icon_speech, icon_text_button,
};
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct PresetPanel;

impl PresetPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let cx = state.document.width * 0.5;
        let cy = state.document.height * 0.5;

        // Every preset drops one shape in the middle of the canvas and
        // selects it, so the row below is pure shape-picking.
        ui.horizontal_wrapped(|ui| {
            if icon_text_button(ui, icon_heart, "ハート")
                .on_hover_text("ハートの形を追加")
                .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::heart("Heart", cx, cy, 120.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if icon_text_button(ui, icon_arrow, "矢印")
                .on_hover_text("矢印シンボルを追加")
                .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::arrow("Arrow", cx, cy, 160.0, 40.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if icon_text_button(ui, icon_gear, "歯車")
                .on_hover_text("歯車（コグ）を追加")
                .clicked()
            {
                let obj = crate::core::presets::PresetLibrary::gear("Gear", cx, cy, 8, 40.0, 60.0);
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
            }

            if icon_text_button(ui, icon_speech, "吹き出し")
                .on_hover_text("吹き出しを追加")
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

            if icon_text_button(ui, icon_portal, "VFXポータル")
                .on_hover_text("SF風の六角VFXリングを追加")
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
        ui.heading(RichText::new("スマートガイド").strong());
        ui.add_space(4.0);

        // Snapping options
        ui.label(RichText::new("スナップ対象:").strong());
        ui.checkbox(&mut state.snap_to_grid, "グリッド");
        ui.checkbox(&mut state.snap_to_objects, "オブジェクト");
        ui.checkbox(&mut state.snap_to_guides, "ガイド");
        ui.checkbox(&mut state.snap_to_points, "アンカーポイント");
        ui.checkbox(&mut state.snap_to_pixels, "ピクセル（整数単位）");

        ui.add_space(4.0);
        ui.separator();

        // Grid settings
        ui.label(RichText::new("グリッド").strong());
        ui.horizontal(|ui| {
            ui.label("間隔:");
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
        ui.label(RichText::new("カスタムガイド").strong());
        ui.horizontal(|ui| {
            if ui.button("水平ガイドを追加").clicked() {
                state.guides.push(crate::core::state::Guide {
                    orientation: crate::core::state::GuideOrientation::Horizontal,
                    position: state.pan_y as f64 / state.zoom as f64,
                });
            }
            if ui.button("垂直ガイドを追加").clicked() {
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
            if ui.button("すべて削除").clicked() {
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
        ui.heading(RichText::new("書き出し").strong());
        ui.add_space(4.0);

        // Export format
        ui.label("形式:");
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
                    ui.label("幅:");
                    ui.add(egui::DragValue::new(&mut state.export_width).range(16.0..=8192.0));
                    ui.label("高さ:");
                    ui.add(egui::DragValue::new(&mut state.export_height).range(16.0..=8192.0));
                });
                ui.horizontal(|ui| {
                    ui.label("倍率:");
                    ui.add(egui::Slider::new(&mut state.export_scale, 0.1..=4.0).show_value(true));
                });
                ui.checkbox(&mut state.export_transparent, "背景を透過");
            }
            "SVG" => {
                ui.checkbox(&mut state.export_svg_viewbox, "ViewBoxを含める");
                ui.checkbox(&mut state.export_svg_embed_fonts, "フォントを埋め込む");
            }
            _ => {}
        }

        ui.add_space(4.0);
        ui.separator();

        // Export scope
        ui.label("範囲:");
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(state.export_scope == "All", "すべてのオブジェクト")
                .clicked()
            {
                state.export_scope = "All".into();
            }
            if ui
                .selectable_label(state.export_scope == "Selected", "選択したオブジェクトのみ")
                .clicked()
            {
                state.export_scope = "Selected".into();
            }
        });

        ui.add_space(8.0);

        // Export button
        if ui.button("書き出し...").clicked() {
            let filter = match format.as_str() {
                "JSON" => &["json"][..],
                "PNG" => &["png"][..],
                "WEBP" => &["webp"][..],
                "AVIF" => &["avif"][..],
                _ => &["svg"][..],
            };
            if let Some(path) = rfd::FileDialog::new()
                .set_title("名前を付けて書き出し")
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
                    let svg = crate::io::svg::export_svg_with_options(
                        doc_ref,
                        state.export_svg_embed_fonts,
                        None,
                    );
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
        ui.heading(RichText::new("キーボードショートカット").strong());
        ui.add_space(4.0);

        let mk = crate::app::control_bar::mod_key();
        let shortcuts: [(&str, &str); 32] = [
            ("V", "選択ツール"),
            ("A", "ノード（ダイレクト選択）"),
            ("P", "ペンツール"),
            ("N", "鉛筆ツール"),
            ("U", "長方形ツール"),
            ("O", "楕円ツール"),
            ("S", "星形ツール"),
            ("G", "多角形ツール"),
            ("L", "ラインツール"),
            ("T", "テキストツール"),
            ("I", "スポイトツール"),
            ("H", "ハンドツール（スクロール）"),
            ("B", "ブラシツール"),
            ("E", "消しゴムツール"),
            ("D", "塗りと線をデフォルトに"),
            ("/", "塗りをなしに"),
            ("Shift+X", "塗りと線を入れ替え"),
            ("Delete", "選択を削除"),
            ("Escape", "選択解除／キャンセル"),
            ("Enter", "ペンのパスを確定"),
            ("__MK__+Z", "元に戻す"),
            ("__MK__+Y", "やり直す"),
            ("__MK__+A", "すべて選択"),
            ("__MK__+G", "グループ化"),
            ("__MK__+Shift+G", "グループ解除"),
            ("__MK__+D", "複製"),
            ("__MK__+C", "コピー"),
            ("__MK__+V", "ペースト"),
            ("__MK__+0", "画面に合わせて表示"),
            ("__MK__+1", "100%表示"),
            ("__MK__+7", "クリッピングマスク"),
            ("矢印キー", "微調整（Shiftで10倍）"),
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
