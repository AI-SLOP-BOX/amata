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
            let dx = wx - start.0;
            let dy = wy - start.1;
            // Isolated children live in group-local space.
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
