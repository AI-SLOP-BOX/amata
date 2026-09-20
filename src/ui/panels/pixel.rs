//! Pixel-art (dot絵) panel: palette editing, canvas creation, tool shortcuts.

use crate::core::document::{Object, ObjectType};
use crate::core::pixel::{PixelArt, MAX_PIXEL_DIM};
use crate::core::state::{AppState, Tool};
use egui::{Color32, RichText, Ui, Vec2};

pub struct PixelPanel;

/// The pixel object this panel edits: selected pixel object first,
///
/// otherwise the first pixel object in the document.
fn target_id(state: &AppState) -> Option<String> {
    for id in &state.selected_ids {
        if let Some(obj) = state.document.find_object(id) {
            if matches!(obj.object_type, ObjectType::PixelArt(_)) {
                return Some(id.clone());
            }
        }
    }
    state
        .document
        .all_objects()
        .find(|(_, o)| matches!(o.object_type, ObjectType::PixelArt(_)))
        .map(|(_, o)| o.id.clone())
}

fn rgba_to_color32(c: [f32; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c[0] * 255.0).round().clamp(0.0, 255.0) as u8,
        (c[1] * 255.0).round().clamp(0.0, 255.0) as u8,
        (c[2] * 255.0).round().clamp(0.0, 255.0) as u8,
        (c[3] * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

impl PixelPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("👾 Pixel Art").strong());
        ui.add_space(4.0);

        // Tool quick-switch.
        ui.horizontal(|ui| {
            for tool in [Tool::PixelPencil, Tool::PixelEraser, Tool::PixelBucket] {
                let active = state.current_tool == tool;
                if ui
                    .selectable_label(active, format!("{} {}", tool.icon(), tool.name()))
                    .on_hover_text(format!("ショートカット: {}", tool.shortcut()))
                    .clicked()
                {
                    state.previous_tool = state.current_tool;
                    state.current_tool = tool;
                }
            }
        });
        ui.add_space(4.0);

        // New canvas.
        ui.collapsing("＋ 新規ドット絵キャンバス", |ui| {
            ui.horizontal(|ui| {
                ui.label("サイズ:");
                ui.add(
                    egui::DragValue::new(&mut state.pixel_new_size)
                        .range(8..=MAX_PIXEL_DIM)
                        .suffix(" px"),
                );
                for preset in [16u32, 32, 48, 64, 128] {
                    if ui.small_button(format!("{preset}")).clicked() {
                        state.pixel_new_size = preset;
                    }
                }
            });
            if ui.button("キャンバスを作成 (PICO-8パレット)").clicked() {
                let size = state.pixel_new_size.clamp(1, MAX_PIXEL_DIM);
                let art = PixelArt::new(size, size, PixelArt::pico8_palette());
                let (cx, cy) = state.screen_to_world(state.canvas_center_x, state.canvas_center_y);
                let mut obj = Object::new_pixel_art(
                    &format!("Pixel {size}x{size}"),
                    cx - size as f64 / 2.0,
                    cy - size as f64 / 2.0,
                    art,
                );
                obj.fill = None;
                let id = obj.id.clone();
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
                state.selected_ids = vec![id];
                state.current_tool = Tool::PixelPencil;
                state.notify_success(format!("{size}x{size} のドット絵キャンバスを作成しました"));
            }
        });
        ui.add_space(4.0);

        let Some(id) = target_id(state) else {
            ui.label(RichText::new("ドット絵レイヤーがありません。上で作成してください。").weak());
            return;
        };

        // Snapshot of grid info for display (borrow ends immediately).
        let (gw, gh, painted, pal_len) = match state.document.find_object(&id) {
            Some(obj) => {
                if let ObjectType::PixelArt(p) = &obj.object_type {
                    (p.width, p.height, p.painted_count(), p.palette.len())
                } else {
                    return;
                }
            }
            None => return,
        };
        ui.label(format!("編集中: {gw}×{gh} / {painted} dots / {pal_len} colors"));
        if !state.selected_ids.contains(&id) {
            ui.horizontal(|ui| {
                ui.label(RichText::new("選択中のレイヤーではありません。").weak());
                if ui.small_button("このレイヤーを選択").clicked() {
                    state.selected_ids = vec![id.clone()];
                }
            });
        }
        ui.add_space(4.0);

        // Palette grid.
        ui.label(RichText::new("パレット").weak());
        let palette: Vec<[f32; 4]> = match state.document.find_object(&id) {
            Some(obj) => {
                if let ObjectType::PixelArt(p) = &obj.object_type {
                    p.palette.clone()
                } else {
                    return;
                }
            }
            None => return,
        };
        let cols = 8;
        egui::Grid::new("pixel_palette_grid")
            .spacing(Vec2::new(3.0, 3.0))
            .show(ui, |ui| {
                for (i, col) in palette.iter().enumerate() {
                    if i % cols == 0 && i > 0 {
                        ui.end_row();
                    }
                    let selected = state.pixel_palette_index == i;
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
                    if ui.is_rect_visible(rect) {
                        let p = ui.painter();
                        p.rect_filled(rect.expand(if selected { 2.0 } else { 0.0 }), 3.0, rgba_to_color32(*col));
                        if selected {
                            p.rect_stroke(
                                rect.expand(2.0),
                                3.0,
                                egui::Stroke::new(2.0, Color32::WHITE),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                    if resp.clicked() {
                        state.pixel_palette_index = i;
                        state.fill_color = *col;
                    }
                    resp.on_hover_text(format!("{i}: #{:02X}{:02X}{:02X}", (col[0]*255.0) as u8, (col[1]*255.0) as u8, (col[2]*255.0) as u8));
                }
            });
        ui.add_space(4.0);

        // Palette actions (each commits its own undo step immediately).
        ui.horizontal(|ui| {
            if ui.small_button("PICO-8").clicked() {
                state.ensure_object_snapshot(&id);
                if let Some(obj) = state.document.find_object_mut(&id) {
                    if let ObjectType::PixelArt(p) = &mut obj.object_type {
                        p.palette = PixelArt::pico8_palette();
                        p.normalize();
                    }
                }
                state.pixel_palette_index = 0;
                state.commit_object_edits("Pixel Palette Preset");
            }
            if ui.small_button("Grayscale").clicked() {
                state.ensure_object_snapshot(&id);
                if let Some(obj) = state.document.find_object_mut(&id) {
                    if let ObjectType::PixelArt(p) = &mut obj.object_type {
                        p.palette = PixelArt::gray_palette();
                        p.normalize();
                    }
                }
                state.pixel_palette_index = 0;
                state.commit_object_edits("Pixel Palette Preset");
            }
            let mut fill_c = [
                (state.fill_color[0] * 255.0) as u8,
                (state.fill_color[1] * 255.0) as u8,
                (state.fill_color[2] * 255.0) as u8,
                (state.fill_color[3] * 255.0) as u8,
            ];
            if ui
                .color_edit_button_srgba_unmultiplied(&mut fill_c)
                .on_hover_text("現在の塗り色をパレットに追加")
                .changed()
            {
                let color = [
                    fill_c[0] as f32 / 255.0,
                    fill_c[1] as f32 / 255.0,
                    fill_c[2] as f32 / 255.0,
                    1.0,
                ];
                state.fill_color = color;
                state.ensure_object_snapshot(&id);
                let mut added = None;
                if let Some(obj) = state.document.find_object_mut(&id) {
                    if let ObjectType::PixelArt(p) = &mut obj.object_type {
                        added = p.push_color(color);
                    }
                }
                match added {
                    Some(i) => {
                        state.pixel_palette_index = i as usize;
                        state.commit_object_edits("Pixel Palette Add");
                    }
                    None => {
                        state.commit_object_edits("Pixel Palette Add");
                        state.notify_error("パレットが満杯です（最大255色）");
                    }
                }
            }
        });
        ui.add_space(4.0);

        ui.checkbox(&mut state.pixel_show_grid, "ドットグリッドを表示");
        ui.label(
            RichText::new("X: ペン / C: 消しゴム / K: バケツ / I: スポイト")
                .weak()
                .size(11.0),
        );
    }
}
