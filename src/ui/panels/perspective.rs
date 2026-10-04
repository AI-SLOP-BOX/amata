//! Perspective grid panel: setup, vanishing points, snap.

use crate::core::perspective::PerspectiveGrid;
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct PerspectivePanel;

impl PerspectivePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        ui.heading(RichText::new("Perspective Grid").strong());
        ui.add_space(4.0);

        if state.document.perspective.is_none() {
            if ui.button("透視グリッドを作成").clicked() {
                state.push_perspective_undo(
                    "Create Perspective Grid",
                    None,
                    Some(PerspectiveGrid::default_for_doc(
                        state.document.width,
                        state.document.height,
                    )),
                );
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
        // borrow scopes simple, then write back on change. Drags coalesce
        // into one undo step via the pending snapshot (committed when no
        // widget is mid-drag); clicks/typed values commit immediately.
        let mut grid = state.document.perspective.clone().unwrap();
        let before = grid.clone();
        let mut dragging = false;
        dragging |= ui.checkbox(&mut grid.show, "グリッド表示").dragged();
        dragging |= ui.checkbox(&mut grid.snap, "ガイドにスナップ").dragged();
        dragging |= ui
            .checkbox(&mut grid.two_point, "二点透視（OFFで一点）")
            .dragged();
        ui.horizontal(|ui| {
            ui.label("水平線Y:");
            dragging |= ui
                .add(egui::DragValue::new(&mut grid.horizon_y).speed(1.0))
                .dragged();
        });
        ui.horizontal(|ui| {
            ui.label("左VP:");
            dragging |= ui
                .add(egui::DragValue::new(&mut grid.left_vp.0).speed(2.0))
                .dragged();
            dragging |= ui
                .add(egui::DragValue::new(&mut grid.left_vp.1).speed(2.0))
                .dragged();
        });
        if grid.two_point {
            ui.horizontal(|ui| {
                ui.label("右VP:");
                dragging |= ui
                    .add(egui::DragValue::new(&mut grid.right_vp.0).speed(2.0))
                    .dragged();
                dragging |= ui
                    .add(egui::DragValue::new(&mut grid.right_vp.1).speed(2.0))
                    .dragged();
            });
        }
        ui.horizontal(|ui| {
            ui.label("レイ数:");
            dragging |= ui
                .add(egui::DragValue::new(&mut grid.rays).range(2..=64))
                .dragged();
        });
        if grid != before {
            grid.normalize();
            state.ensure_perspective_snapshot();
            state.document.perspective = Some(grid);
        }
        if !dragging {
            state.commit_perspective_edits("Edit Perspective Grid");
        }
        if ui.button("グリッドを削除").clicked() {
            // Close any in-progress drag gesture first so its step lands
            // before the delete instead of leaking into `pending`.
            state.commit_perspective_edits("Edit Perspective Grid");
            state.push_perspective_undo(
                "Delete Perspective Grid",
                state.document.perspective.clone(),
                None,
            );
        }
    }
}
