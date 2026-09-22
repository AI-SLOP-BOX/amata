//! Perspective grid panel: setup, vanishing points, snap.

use crate::core::perspective::PerspectiveGrid;
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct PerspectivePanel;

impl PerspectivePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("📐 Perspective Grid").strong());
        ui.add_space(4.0);

        if state.document.perspective.is_none() {
            if ui.button("透視グリッドを作成").clicked() {
                state.document.perspective = Some(PerspectiveGrid::default_for_doc(
                    state.document.width,
                    state.document.height,
                ));
                state.notify_success("透視グリッドを作成しました");
            }
            ui.label(
                RichText::new("一点/二点透視のガイド＋スナップです。")
                    .weak()
                    .size(11.0),
            );
            return;
        }

        // From here the grid exists; read-modify through a copy to keep
        // borrow scopes simple, then write back on change.
        let mut grid = state.document.perspective.clone().unwrap();
        let before = grid.clone();
        ui.checkbox(&mut grid.show, "グリッド表示");
        ui.checkbox(&mut grid.snap, "ガイドにスナップ");
        ui.checkbox(&mut grid.two_point, "二点透視（OFFで一点）");
        ui.horizontal(|ui| {
            ui.label("Horizon Y:");
            ui.add(egui::DragValue::new(&mut grid.horizon_y).speed(1.0));
        });
        ui.horizontal(|ui| {
            ui.label("Left VP:");
            ui.add(egui::DragValue::new(&mut grid.left_vp.0).speed(2.0));
            ui.add(egui::DragValue::new(&mut grid.left_vp.1).speed(2.0));
        });
        if grid.two_point {
            ui.horizontal(|ui| {
                ui.label("Right VP:");
                ui.add(egui::DragValue::new(&mut grid.right_vp.0).speed(2.0));
                ui.add(egui::DragValue::new(&mut grid.right_vp.1).speed(2.0));
            });
        }
        ui.horizontal(|ui| {
            ui.label("Rays:");
            ui.add(egui::DragValue::new(&mut grid.rays).range(2..=64));
        });
        if grid != before {
            grid.normalize();
            state.document.perspective = Some(grid);
        }
        if ui.button("グリッドを削除").clicked() {
            state.document.perspective = None;
        }
    }
}
