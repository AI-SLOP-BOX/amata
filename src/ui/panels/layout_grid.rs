//! Figma-style layout grid panel: columns / rows / square grid per artboard.

use crate::core::layout_grid::{LayoutGrid, LayoutGridKind};
use crate::core::state::AppState;
use egui::{RichText, Ui};

pub struct LayoutGridPanel;

impl LayoutGridPanel {
    pub fn show(ui: &mut Ui, state: &mut AppState) {
        let locale = state.prefs.language.clone();
        ui.heading(RichText::new(crate::ui::i18n::text(&locale, "layout_grid.title")).strong());
        ui.add_space(4.0);

        // Implicit artboard (document.artboards empty): materialize it so
        // the grid has somewhere to live. Undoable like every other
        // artboard-list edit so it can be taken back and marks dirty.
        if state.document.artboards.is_empty() {
            let w = state.document.width;
            let h = state.document.height;
            let before = state.document.artboards.clone();
            state
                .document
                .artboards
                .push(crate::core::document::Artboard::new(
                    "Artboard 1",
                    0.0,
                    0.0,
                    w,
                    h,
                ));
            let after = state.document.artboards.clone();
            state.push_artboards_undo("Add Artboard", before, after);
        }
        if state.active_artboard_idx >= state.document.artboards.len() {
            state.active_artboard_idx = state.document.artboards.len() - 1;
        }
        let idx = state.active_artboard_idx;
        let enabled = state.document.artboards[idx].layout_grid.is_some();

        if !enabled {
            if ui
                .button(crate::ui::i18n::text(&locale, "layout_grid.add"))
                .clicked()
            {
                let before = state.document.artboards.clone();
                state.document.artboards[idx].layout_grid = Some(LayoutGrid::default());
                let after = state.document.artboards.clone();
                state.push_artboards_undo("Add Layout Grid", before, after);
                state.notify_success(
                    crate::ui::i18n::text(&locale, "layout_grid.added").into_owned(),
                );
            }
            ui.label(
                RichText::new(crate::ui::i18n::text(&locale, "layout_grid.description"))
                    .weak()
                    .size(11.0),
            );
            return;
        }

        let mut grid = state.document.artboards[idx].layout_grid.clone().unwrap();
        let before = grid.clone();
        grid.normalize();

        ui.checkbox(
            &mut grid.show,
            crate::ui::i18n::text(&locale, "layout_grid.visible"),
        );
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "layout_grid.kind"));
            ui.selectable_value(
                &mut grid.kind,
                LayoutGridKind::Columns,
                crate::ui::i18n::text(&locale, "layout_grid.columns"),
            );
            ui.selectable_value(
                &mut grid.kind,
                LayoutGridKind::Rows,
                crate::ui::i18n::text(&locale, "layout_grid.rows"),
            );
            ui.selectable_value(
                &mut grid.kind,
                LayoutGridKind::Grid,
                crate::ui::i18n::text(&locale, "layout_grid.grid"),
            );
        });

        match grid.kind {
            LayoutGridKind::Columns | LayoutGridKind::Rows => {
                ui.horizontal(|ui| {
                    ui.label(crate::ui::i18n::text(&locale, "layout_grid.count"));
                    ui.add(egui::DragValue::new(&mut grid.count).range(1..=64));
                });
                ui.horizontal(|ui| {
                    ui.label(crate::ui::i18n::text(&locale, "layout_grid.gutter"));
                    ui.add(
                        egui::DragValue::new(&mut grid.gutter)
                            .speed(1.0)
                            .range(0.0..=500.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(crate::ui::i18n::text(&locale, "layout_grid.margin"));
                    ui.add(
                        egui::DragValue::new(&mut grid.margin)
                            .speed(1.0)
                            .range(0.0..=1000.0),
                    );
                });
            }
            LayoutGridKind::Grid => {
                ui.horizontal(|ui| {
                    ui.label(crate::ui::i18n::text(&locale, "layout_grid.cell"));
                    ui.add(
                        egui::DragValue::new(&mut grid.size)
                            .speed(1.0)
                            .range(1.0..=500.0),
                    );
                });
            }
        }
        ui.horizontal(|ui| {
            ui.label(crate::ui::i18n::text(&locale, "layout_grid.opacity"));
            ui.add(
                egui::DragValue::new(&mut grid.opacity)
                    .speed(0.01)
                    .range(0.0..=1.0),
            );
        });

        if grid != before {
            let art_before = state.document.artboards.clone();
            state.document.artboards[idx].layout_grid = Some(grid);
            let art_after = state.document.artboards.clone();
            state.push_artboards_undo("Edit Layout Grid", art_before, art_after);
        }
        if ui
            .button(crate::ui::i18n::text(&locale, "common.delete"))
            .clicked()
        {
            let art_before = state.document.artboards.clone();
            state.document.artboards[idx].layout_grid = None;
            let art_after = state.document.artboards.clone();
            state.push_artboards_undo("Remove Layout Grid", art_before, art_after);
        }
    }
}
