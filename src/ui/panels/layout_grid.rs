//! Figma-style layout grid panel: columns / rows / square grid per artboard.

use crate::core::layout_grid::{LayoutGrid, LayoutGridKind};
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct LayoutGridPanel;

impl LayoutGridPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("▦ レイアウトグリッド").strong());
        ui.add_space(4.0);

        // Implicit artboard (document.artboards empty): materialize it so
        // the grid has somewhere to live.
        if state.document.artboards.is_empty() {
            let w = state.document.width;
            let h = state.document.height;
            state
                .document
                .artboards
                .push(crate::core::document::Artboard::new("Artboard 1", 0.0, 0.0, w, h));
        }
        if state.active_artboard_idx >= state.document.artboards.len() {
            state.active_artboard_idx = state.document.artboards.len() - 1;
        }
        let idx = state.active_artboard_idx;
        let enabled = state.document.artboards[idx].layout_grid.is_some();

        if !enabled {
            if ui.button("レイアウトグリッドを追加").clicked() {
                state.document.artboards[idx].layout_grid = Some(LayoutGrid::default());
                state.notify_success("レイアウトグリッドを追加しました");
            }
            ui.label(
                RichText::new("カラム / 行 / 方眼をアートボードに重ねて表示します。")
                    .weak()
                    .size(11.0),
            );
            return;
        }

        let mut grid = state.document.artboards[idx].layout_grid.clone().unwrap();
        let before = grid.clone();
        grid.normalize();

        ui.checkbox(&mut grid.show, "表示");
        ui.horizontal(|ui| {
            ui.label("種類:");
            ui.selectable_value(&mut grid.kind, LayoutGridKind::Columns, "カラム");
            ui.selectable_value(&mut grid.kind, LayoutGridKind::Rows, "行");
            ui.selectable_value(&mut grid.kind, LayoutGridKind::Grid, "方眼");
        });

        match grid.kind {
            LayoutGridKind::Columns | LayoutGridKind::Rows => {
                ui.horizontal(|ui| {
                    ui.label("本数:");
                    ui.add(egui::DragValue::new(&mut grid.count).range(1..=64));
                });
                ui.horizontal(|ui| {
                    ui.label("ガター:");
                    ui.add(egui::DragValue::new(&mut grid.gutter).speed(1.0).range(0.0..=500.0));
                });
                ui.horizontal(|ui| {
                    ui.label("マージン:");
                    ui.add(egui::DragValue::new(&mut grid.margin).speed(1.0).range(0.0..=1000.0));
                });
            }
            LayoutGridKind::Grid => {
                ui.horizontal(|ui| {
                    ui.label("セル:");
                    ui.add(egui::DragValue::new(&mut grid.size).speed(1.0).range(1.0..=500.0));
                });
            }
        }
        ui.horizontal(|ui| {
            ui.label("不透明度:");
            ui.add(egui::DragValue::new(&mut grid.opacity).speed(0.01).range(0.0..=1.0));
        });

        if grid != before {
            state.document.artboards[idx].layout_grid = Some(grid);
        }
        if ui.button("削除").clicked() {
            state.document.artboards[idx].layout_grid = None;
        }
    }
}
