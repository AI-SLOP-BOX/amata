//! Pixel-art (dot絵) canvas editing: pencil / eraser strokes and bucket fill.
//!
//! All mutations go through the whole-object snapshot mechanism, so one
//! click or one drag is always exactly one undo step.

use super::CanvasWidget;
use crate::core::document::ObjectType;
use crate::core::pixel::TRANSPARENT;
use crate::core::state::AppState;
use crate::tools::pixel::{hit_pixel_object, world_to_cell, PixelStroke};

impl CanvasWidget {
    /// Pencil/eraser value for the next dab: selected palette index, or
    /// transparency for the eraser. Clamped so a stale selection (e.g.
    /// after switching objects with smaller palettes) can never panic.
    fn pixel_value(state: &AppState, obj_id: &str, erase: bool) -> Option<u8> {
        if erase {
            return Some(TRANSPARENT);
        }
        let obj = state.document.find_object(obj_id)?;
        if let ObjectType::PixelArt(p) = &obj.object_type {
            if p.palette.is_empty() {
                return None;
            }
            Some((state.pixel_palette_index.min(p.palette.len() - 1)) as u8)
        } else {
            None
        }
    }

    /// Begin a stroke (drag-started path). Snapshots first so one Undo
    /// restores the pre-stroke grid.
    pub(super) fn pixel_stroke_begin(
        &mut self,
        state: &mut AppState,
        wx: f64,
        wy: f64,
        erase: bool,
    ) -> bool {
        let Some(obj_id) = hit_pixel_object(state, wx, wy) else {
            return false;
        };
        let Some(value) = Self::pixel_value(state, &obj_id, erase) else {
            return false;
        };
        state.ensure_object_snapshot(&obj_id);
        let cell = state
            .document
            .find_object(&obj_id)
            .map(|o| world_to_cell(o, wx, wy))
            .unwrap_or((0, 0));
        if let Some(obj) = state.document.find_object_mut(&obj_id) {
            if let ObjectType::PixelArt(p) = &mut obj.object_type {
                p.normalize();
                p.set(cell.0, cell.1, value);
            }
        }
        state.selected_ids = vec![obj_id.clone()];
        self.pixel_stroke = Some(PixelStroke::new(obj_id, cell, value));
        true
    }

    /// Extend the active stroke toward the pointer (dragged path).
    pub(super) fn pixel_stroke_extend(&mut self, state: &mut AppState, wx: f64, wy: f64) {
        let Some(stroke) = self.pixel_stroke.as_mut() else {
            return;
        };
        let (obj_id, value, last) = (stroke.obj_id.clone(), stroke.value, stroke.last);
        let cell = match state.document.find_object(&obj_id) {
            Some(o) => world_to_cell(o, wx, wy),
            None => {
                self.pixel_stroke = None;
                return;
            }
        };
        if cell == last {
            return;
        }
        if let Some(obj) = state.document.find_object_mut(&obj_id) {
            if let ObjectType::PixelArt(p) = &mut obj.object_type {
                p.stroke_line(last.0, last.1, cell.0, cell.1, value);
            }
        }
        if let Some(stroke) = self.pixel_stroke.as_mut() {
            stroke.last = cell;
        }
    }

    /// Single dab without dragging (click path).
    pub(super) fn pixel_dab(&mut self, state: &mut AppState, wx: f64, wy: f64, erase: bool) {
        let Some(obj_id) = hit_pixel_object(state, wx, wy) else {
            return;
        };
        let Some(value) = Self::pixel_value(state, &obj_id, erase) else {
            return;
        };
        state.ensure_object_snapshot(&obj_id);
        let cell = state
            .document
            .find_object(&obj_id)
            .map(|o| world_to_cell(o, wx, wy))
            .unwrap_or((0, 0));
        if let Some(obj) = state.document.find_object_mut(&obj_id) {
            if let ObjectType::PixelArt(p) = &mut obj.object_type {
                p.normalize();
                p.set(cell.0, cell.1, value);
            }
        }
        state.selected_ids = vec![obj_id];
        state.commit_object_edits(if erase { "Pixel Erase" } else { "Pixel Dab" });
    }

    /// Flood fill at the pointer (click or drag-start path). Commits
    /// immediately: a bucket fill is a single atomic edit.
    pub(super) fn pixel_bucket(&mut self, state: &mut AppState, wx: f64, wy: f64) {
        let Some(obj_id) = hit_pixel_object(state, wx, wy) else {
            return;
        };
        let Some(value) = Self::pixel_value(state, &obj_id, false) else {
            return;
        };
        state.ensure_object_snapshot(&obj_id);
        let cell = state
            .document
            .find_object(&obj_id)
            .map(|o| world_to_cell(o, wx, wy))
            .unwrap_or((0, 0));
        if let Some(obj) = state.document.find_object_mut(&obj_id) {
            if let ObjectType::PixelArt(p) = &mut obj.object_type {
                p.normalize();
                p.flood_fill(cell.0, cell.1, value);
            }
        }
        state.selected_ids = vec![obj_id];
        state.commit_object_edits("Pixel Bucket Fill");
    }

    /// Finish the active stroke (drag-stopped path).
    pub(super) fn pixel_stroke_end(&mut self, state: &mut AppState) {
        if self.pixel_stroke.take().is_some() {
            state.commit_object_edits("Pixel Stroke");
        }
    }

    /// Eyedropper sampling from a pixel object: pick the cell's palette
    /// index (opaque cells only). Returns true when something was picked.
    pub(super) fn pixel_pick(&mut self, state: &mut AppState, wx: f64, wy: f64) -> bool {
        let Some(obj_id) = hit_pixel_object(state, wx, wy) else {
            return false;
        };
        let (cell_value, color) = match state.document.find_object(&obj_id) {
            Some(obj) => {
                let cell = world_to_cell(obj, wx, wy);
                if let ObjectType::PixelArt(p) = &obj.object_type {
                    let v = p.get(cell.0, cell.1);
                    let c = if v == TRANSPARENT {
                        None
                    } else {
                        p.palette.get(v as usize).copied()
                    };
                    (v, c)
                } else {
                    return false;
                }
            }
            None => return false,
        };
        let Some(color) = color else { return false };
        state.pixel_palette_index = cell_value as usize;
        state.fill_color = color;
        state.selected_ids = vec![obj_id];
        true
    }
}
