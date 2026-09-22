use crate::core::state::AppState;

pub struct SelectState {
    pub is_dragging: bool,
    pub drag_start: Option<(f64, f64)>,
    pub drag_object_start: Option<(f64, f64)>,
    pub drag_object_id: Option<String>,
    pub drag_starts: Vec<(String, f64, f64)>,
}

impl SelectState {
    pub fn new() -> Self {
        Self {
            is_dragging: false,
            drag_start: None,
            drag_object_start: None,
            drag_object_id: None,
            drag_starts: Vec::new(),
        }
    }

    pub fn hit_test(&self, state: &AppState, wx: f64, wy: f64) -> Option<String> {
        if state.isolated_group_id.is_some() {
            return self.hit_test_isolated(state, wx, wy);
        }
        for (_, obj) in state.document.all_objects().rev() {
            if obj.visible && !obj.locked && obj.hit_test(wx, wy) {
                return Some(obj.id.clone());
            }
        }
        None
    }

    /// Hit-test direct children of the isolated group. The click arrives in
    /// world space; children test in group-local space.
    pub fn hit_test_isolated(
        &self,
        state: &AppState,
        wx: f64,
        wy: f64,
    ) -> Option<String> {
        let group = state.isolated_group()?;
        let (gx, gy) = group.transform.inverse_transform_point(wx, wy);
        let children = match &group.object_type {
            crate::core::document::ObjectType::Group(children) => children,
            _ => return None,
        };
        for obj in children.iter().rev() {
            if obj.visible && !obj.locked && obj.hit_test(gx, gy) {
                return Some(obj.id.clone());
            }
        }
        None
    }

    /// Convert a world-space drag delta into the isolated group's local
    /// space (inverse of the group's linear part) so children of rotated
    /// or scaled groups track the cursor.
    pub fn parent_delta(state: &AppState, dx: f64, dy: f64) -> (f64, f64) {
        let Some(group) = state.isolated_group() else {
            return (dx, dy);
        };
        let m = group.transform.matrix();
        let det = m[0] * m[3] - m[1] * m[2];
        if det.abs() < 1e-9 {
            return (dx, dy);
        }
        (
            (m[3] * dx - m[2] * dy) / det,
            (-m[1] * dx + m[0] * dy) / det,
        )
    }

    pub fn start_drag(&mut self, state: &mut AppState, wx: f64, wy: f64) {
        if let Some(id) = self.hit_test(state, wx, wy) {
            self.drag_starts.clear();
            if !state.selected_ids.contains(&id) {
                state.selected_ids = vec![id.clone()];
            }

            for sel_id in &state.selected_ids {
                if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == sel_id)
                {
                    self.drag_starts
                        .push((sel_id.clone(), obj.transform.x, obj.transform.y));
                }
            }

            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| o.id == id) {
                self.drag_object_start = Some((obj.transform.x, obj.transform.y));
            }
            self.is_dragging = true;
            self.drag_start = Some((wx, wy));
            self.drag_object_id = Some(id);
        }
    }

    pub fn update_drag(&mut self, state: &mut AppState, wx: f64, wy: f64) {
        if !self.is_dragging {
            return;
        }
        if let Some(start) = self.drag_start {
            let mut dx = wx - start.0;
            let mut dy = wy - start.1;

            // Smart-guide snap: pull the moving bbox onto nearby edges /
            // centres of other objects while dragging (Figma-style magnetism).
            if state.show_smart_guides && state.isolated_group_id.is_none() {
                let tol = 4.0 / f64::max(f64::from(state.zoom), 1e-9);
                let mut s_min = (f64::MAX, f64::MAX);
                let mut s_max = (f64::MIN, f64::MIN);
                let mut any = false;
                for (id, sx, sy) in &self.drag_starts {
                    if let Some(obj) = state.document.find_object(id) {
                        if let Some((bb_min, bb_max)) = obj.bounding_box() {
                            // bbox offset from the (pure-translation) transform
                            let ox0 = bb_min.x - obj.transform.x;
                            let oy0 = bb_min.y - obj.transform.y;
                            let ox1 = bb_max.x - obj.transform.x;
                            let oy1 = bb_max.y - obj.transform.y;
                            let min_x = *sx + dx + ox0;
                            let min_y = *sy + dy + oy0;
                            let max_x = *sx + dx + ox1;
                            let max_y = *sy + dy + oy1;
                            s_min.0 = s_min.0.min(min_x);
                            s_min.1 = s_min.1.min(min_y);
                            s_max.0 = s_max.0.max(max_x);
                            s_max.1 = s_max.1.max(max_y);
                            any = true;
                        }
                    }
                }
                if any {
                    let mut targets_x: Vec<f64> = Vec::new();
                    let mut targets_y: Vec<f64> = Vec::new();
                    for (_, other) in state.document.all_objects() {
                        if !other.visible
                            || self.drag_starts.iter().any(|(id, _, _)| id == &other.id)
                        {
                            continue;
                        }
                        if let Some((o_min, o_max)) = other.bounding_box() {
                            targets_x.push(o_min.x);
                            targets_x.push((o_min.x + o_max.x) * 0.5);
                            targets_x.push(o_max.x);
                            targets_y.push(o_min.y);
                            targets_y.push((o_min.y + o_max.y) * 0.5);
                            targets_y.push(o_max.y);
                        }
                    }
                    let moving_x = [s_min.0, (s_min.0 + s_max.0) * 0.5, s_max.0];
                    let moving_y = [s_min.1, (s_min.1 + s_max.1) * 0.5, s_max.1];
                    dx += crate::core::smart_guides::snap_axis(moving_x, &targets_x, tol);
                    dy += crate::core::smart_guides::snap_axis(moving_y, &targets_y, tol);
                }
            }

            let (dx, dy) = Self::parent_delta(state, dx, dy);

            // Move all selected objects by the delta relative to their recorded start
            for (id, start_x, start_y) in &self.drag_starts {
                let new_x = *start_x + dx;
                let new_y = *start_y + dy;
                for (_, obj) in state.document.all_objects_mut() {
                    if &obj.id == id {
                        obj.transform.x = new_x;
                        obj.transform.y = new_y;
                        break;
                    }
                }
            }
        }
    }

    pub fn end_drag(&mut self, state: &mut AppState) -> Vec<(String, f64, f64, f64, f64)> {
        let mut moved = Vec::new();
        if self.is_dragging {
            for (id, start_x, start_y) in self.drag_starts.drain(..) {
                let current_pos = state
                    .document
                    .all_objects()
                    .find(|(_, o)| o.id == id)
                    .map(|(_, o)| (o.transform.x, o.transform.y))
                    .unwrap_or((start_x, start_y));

                if (current_pos.0 - start_x).abs() > 1e-4 || (current_pos.1 - start_y).abs() > 1e-4
                {
                    moved.push((id, start_x, start_y, current_pos.0, current_pos.1));
                }
            }

            self.is_dragging = false;
            self.drag_start = None;
            self.drag_object_start = None;
            self.drag_object_id = None;
        }
        moved
    }
}

impl Default for SelectState {
    fn default() -> Self {
        Self::new()
    }
}
