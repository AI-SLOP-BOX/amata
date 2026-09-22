use serde::{Deserialize, Serialize};

/// Figma-style auto layout on a group: stack children along one axis with
/// gap and padding. Sizing is hug-content (group bbox follows children).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoLayoutDirection {
    Horizontal,
    Vertical,
}

impl Default for AutoLayoutDirection {
    fn default() -> Self {
        Self::Horizontal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutoLayout {
    pub direction: AutoLayoutDirection,
    pub gap: f64,
    pub padding_top: f64,
    pub padding_right: f64,
    pub padding_bottom: f64,
    pub padding_left: f64,
    /// Stack children in reverse document order.
    pub reverse: bool,
}

impl Default for AutoLayout {
    fn default() -> Self {
        Self {
            direction: AutoLayoutDirection::Horizontal,
            gap: 10.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            reverse: false,
        }
    }
}

impl AutoLayout {
    pub fn normalize(&mut self) {
        for v in [
            &mut self.gap,
            &mut self.padding_top,
            &mut self.padding_right,
            &mut self.padding_bottom,
            &mut self.padding_left,
        ] {
            if !v.is_finite() || *v < 0.0 {
                *v = 0.0;
            }
        }
    }

    /// Compute new `transform.x/y` for each child so their bboxes stack
    /// along the layout axis. Input is `(id, bbox_min_x, bbox_min_y,
    /// size_along, cur_tx, cur_ty)` in group-local space; output is
    /// `(id, new_tx, new_ty)` absolute transform positions.
    pub fn layout_positions(
        &self,
        items: &[(String, f64, f64, f64, f64, f64)],
    ) -> Vec<(String, f64, f64)> {
        if items.is_empty() {
            return Vec::new();
        }
        let horizontal = self.direction == AutoLayoutDirection::Horizontal;
        // Sort along the main axis by current bbox min (stable).
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by(|&a, &b| {
            let ka = if horizontal { items[a].1 } else { items[a].2 };
            let kb = if horizontal { items[b].1 } else { items[b].2 };
            ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
        });
        if self.reverse {
            order.reverse();
        }

        let mut cursor = if horizontal {
            self.padding_left
        } else {
            self.padding_top
        };
        let cross_pad = if horizontal {
            self.padding_top
        } else {
            self.padding_left
        };

        let mut out = Vec::with_capacity(items.len());
        for &i in &order {
            let (id, min_x, min_y, extent, cur_tx, cur_ty) = &items[i];
            let (bbox_min, cross_min) = if horizontal {
                (*min_x, *min_y)
            } else {
                (*min_y, *min_x)
            };
            // Move so bbox.min lands on the cursor / cross padding.
            let shift_main = cursor - bbox_min;
            let shift_cross = cross_pad - cross_min;
            let (new_tx, new_ty) = if horizontal {
                (cur_tx + shift_main, cur_ty + shift_cross)
            } else {
                (cur_tx + shift_cross, cur_ty + shift_main)
            };
            out.push((id.clone(), new_tx, new_ty));
            cursor += extent + self.gap;
        }
        out
    }
}
