use crate::core::state::AppState;

pub struct SelectState {
    pub is_dragging: bool,
    pub drag_start: Option<(f64, f64)>,
    pub drag_object_start: Option<(f64, f64)>,
    pub drag_object_id: Option<String>,
}

impl SelectState {
    pub fn new() -> Self {
        Self {
            is_dragging: false,
            drag_start: None,
            drag_object_start: None,
            drag_object_id: None,
        }
    }

    pub fn hit_test(&self, state: &AppState, wx: f64, wy: f64) -> Option<String> {
        for (_, obj) in state.document.all_objects().rev() {
            if obj.visible && !obj.locked && obj.hit_test(wx, wy) {
                return Some(obj.id.clone());
            }
        }
        None
    }

    pub fn start_drag(&mut self, state: &mut AppState, wx: f64, wy: f64) {
        if let Some(id) = self.hit_test(state, wx, wy) {
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

            // Move all selected objects by the delta
            for id in &state.selected_ids {
                // Find the original position for this object
                let orig = if id == self.drag_object_id.as_ref().unwrap_or(&String::new()) {
                    self.drag_object_start
                } else {
                    // For other selected objects, get their current position as "start"
                    state.document.all_objects().find(|(_, o)| &o.id == id).map(|(_, o)| (o.transform.x, o.transform.y))
                };

                if let Some(obj_start) = orig {
                    let new_x = obj_start.0 + dx;
                    let new_y = obj_start.1 + dy;
                    for (_, obj) in state.document.all_objects_mut() {
                        if obj.id == *id {
                            obj.transform.x = new_x;
                            obj.transform.y = new_y;
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn end_drag(&mut self, state: &mut AppState) -> Option<(String, f64, f64, f64, f64)> {
        if self.is_dragging {
            let old = self.drag_object_start.unwrap_or((0.0, 0.0));
            let id = self.drag_object_id.as_ref().unwrap().clone();
            let new = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == id)
                .map(|(_, o)| (o.transform.x, o.transform.y))
                .unwrap_or(old);

            self.is_dragging = false;
            self.drag_start = None;
            self.drag_object_start = None;

            if old != new {
                return Some((id, old.0, old.1, new.0, new.1));
            }
            self.drag_object_id = None;
        }
        None
    }
}

impl Default for SelectState {
    fn default() -> Self {
        Self::new()
    }
}
