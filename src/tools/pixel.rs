//! Pixel-art (dot絵) stroke state and hit-testing helpers.
//!
//! Strokes paint directly into [`PixelArt`](crate::core::pixel::PixelArt)
//! cells; undo is handled by the standard whole-object snapshot
//! (`ensure_object_snapshot` at stroke start, `commit_object_edits` at
//! stroke end), so one drag is always one undo step.

use crate::core::document::Object;
use crate::core::state::AppState;

pub struct PixelStroke {
    pub obj_id: String,
    pub last: (i32, i32),
    pub value: u8,
}

impl PixelStroke {
    pub fn new(obj_id: String, cell: (i32, i32), value: u8) -> Self {
        Self {
            obj_id,
            last: cell,
            value,
        }
    }
}

/// World coordinates → cell coordinates via the object's inverse transform.
/// No grid/object snapping: dots live on exact integer cells.
pub fn world_to_cell(obj: &Object, wx: f64, wy: f64) -> (i32, i32) {
    let (lx, ly) = obj.transform.inverse_transform_point(wx, wy);
    (lx.floor() as i32, ly.floor() as i32)
}

/// Topmost unlocked, visible pixel-art object under a world point.
/// Layers paint back-to-front, so the last hit in document order wins.
pub fn hit_pixel_object(state: &AppState, wx: f64, wy: f64) -> Option<String> {
    let mut hit = None;
    for (_, obj) in state.document.all_objects() {
        if !obj.visible || obj.locked {
            continue;
        }
        if !matches!(
            obj.object_type,
            crate::core::document::ObjectType::PixelArt(_)
        ) {
            continue;
        }
        let (lx, ly) = obj.transform.inverse_transform_point(wx, wy);
        if let crate::core::document::ObjectType::PixelArt(p) = &obj.object_type {
            if lx >= 0.0
                && ly >= 0.0
                && lx < p.width as f64
                && ly < p.height as f64
            {
                hit = Some(obj.id.clone());
            }
        }
    }
    hit
}
