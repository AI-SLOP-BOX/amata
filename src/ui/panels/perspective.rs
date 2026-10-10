//! Perspective grid panel: setup, vanishing points, snap.

use crate::core::perspective::PerspectiveGrid;
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct PerspectivePanel;

impl PerspectivePanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.heading(RichText::new(crate::ui::i18n::text(&locale, "perspective.title")).strong());
        ui.add_space(4.0);

        if state.document.perspective.is_none() {
            if ui
                .button(crate::ui::i18n::text(&locale, "perspective.create"))
                .clicked()
            {
                state.push_perspective_undo(
                    "Create Perspective Grid",
                    None,
                    Some(PerspectiveGrid::default_for_doc(
                        state.document.width,
                        state.document.height,
                    )),
                );
                state.notify_success(
                    crate::ui::i18n::text(&locale, "perspective.created").into_owned(),
                );
            }
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "perspective.description"))
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
        dragging |= ui
            .checkbox(
                &mut grid.show,
                crate::ui::i18n::text(&locale, "perspective.visible"),
            )
            .dragged();
        dragging |= ui
            .checkbox(
                &mut grid.snap,
                crate::ui::i18n::text(&locale, "perspective.snap"),
            )
            .dragged();
        dragging |= ui
            .checkbox(
                &mut grid.two_point,
                crate::ui::i18n::text(&locale, "perspective.two_point"),
            )
            .dragged();
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "perspective.horizon_y"));
            dragging |= ui
                .add(egui::DragValue::new(&mut grid.horizon_y).speed(1.0))
                .dragged();
        });
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "perspective.left_vp"));
            dragging |= ui
                .add(egui::DragValue::new(&mut grid.left_vp.0).speed(2.0))
                .dragged();
            dragging |= ui
                .add(egui::DragValue::new(&mut grid.left_vp.1).speed(2.0))
                .dragged();
        });
        if grid.two_point {
            ui.horizontal(|ui| {
                ui.label(crate::ui::i18n::text(&locale, "perspective.right_vp"));
                dragging |= ui
                    .add(egui::DragValue::new(&mut grid.right_vp.0).speed(2.0))
                    .dragged();
                dragging |= ui
                    .add(egui::DragValue::new(&mut grid.right_vp.1).speed(2.0))
                    .dragged();
            });
        }
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "perspective.rays"));
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
        if ui
            .button(crate::ui::i18n::text(&locale, "perspective.delete"))
            .clicked()
        {
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
