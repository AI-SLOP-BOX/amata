use super::{CanvasWidget, HANDLE_HIT_RADIUS};
use crate::core::document::{Object, ObjectType};
use crate::core::path::{AnchorPoint, FillStyle, PathElement};
use crate::core::state::{AppState, HandleCorner, Tool};
use egui::{Pos2, Rect};

impl CanvasWidget {
    pub(super) fn handle_click(
        &mut self,
        state: &mut AppState,
        wx: f64,
        wy: f64,
        screen_pos: Pos2,
        _origin: Pos2,
        shift: bool,
    ) {
        // Pixel tools paint on exact cells: ignore snapping.
        let (rx, ry) = state.screen_to_world(screen_pos.x, screen_pos.y);
        match state.current_tool {
            Tool::PixelPencil => {
                self.pixel_dab(state, rx, ry, false);
                return;
            }
            Tool::PixelEraser => {
                self.pixel_dab(state, rx, ry, true);
                return;
            }
            Tool::PixelBucket => {
                self.pixel_bucket(state, rx, ry);
                return;
            }
            _ => {}
        }
        match state.current_tool {
            Tool::Pen => {
                if !self.pen_state.is_drawing {
                    self.pen_state.start_path(wx, wy);
                } else if self.pen_state.points.len() >= 2 {
                    let (sx0, sy0) = state.world_to_screen(
                        self.pen_state.points[0].anchor.x,
                        self.pen_state.points[0].anchor.y,
                    );
                    let (sx, sy) = state.world_to_screen(wx, wy);
                    let dist = ((sx - sx0).powi(2) + (sy - sy0).powi(2)).sqrt();
                    if dist <= 12.0 {
                        // Close path
                        let first_anchor = self.pen_state.points[0].anchor;
                        self.pen_state.add_point(first_anchor.x, first_anchor.y);
                        if let Some(obj) = self.pen_state.finish_path(
                            state.fill_color,
                            state.stroke_color,
                            state.stroke_width,
                        ) {
                            let new_id = obj.id.clone();
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                            state.selected_ids = vec![new_id];
                        }
                    } else {
                        self.pen_state.add_point(wx, wy);
                    }
                } else {
                    self.pen_state.add_point(wx, wy);
                }
            }
            Tool::Text => {
                let mut obj =
                    Object::new_text("Text", &state.text_input_buf, wx, wy, state.font_size);
                obj.fill = Some(FillStyle::solid(state.fill_color));
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }
            Tool::Zoom => {
                let zoom_mult = if shift { 0.5_f32 } else { 2.0_f32 };
                state.start_zoom = state.zoom;
                state.start_pan_x = state.pan_x;
                state.start_pan_y = state.pan_y;
                state.target_zoom = (state.zoom * zoom_mult).clamp(0.01, 100.0);
                state.zoom_animation_progress = 0.0;
            }
            Tool::Eyedropper => {
                // Pixel cells first: picking a dot selects its palette index.
                if self.pixel_pick(state, rx, ry) {
                    let prev = state.previous_tool;
                    state.current_tool = prev;
                    return;
                }
                if let Some(id) = self.select_state.hit_test(state, wx, wy) {
                    if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| o.id == id) {
                        if let Some(ref fill) = obj.fill {
                            state.fill_color = fill.color;
                        } else if let Some(ref stroke) = obj.stroke {
                            // No fill on object: use stroke color as fill
                            state.fill_color = stroke.color;
                        }
                        if let Some(ref stroke) = obj.stroke {
                            state.stroke_color = stroke.color;
                            state.stroke_width = stroke.width;
                        }
                    }
                }
                let prev = state.previous_tool;
                state.current_tool = prev;
            }
            Tool::Select => {
                if let Some(id) = self.select_state.hit_test(state, wx, wy) {
                    if shift {
                        if let Some(pos) = state.selected_ids.iter().position(|x| x == &id) {
                            state.selected_ids.remove(pos);
                        } else {
                            state.selected_ids.push(id);
                        }
                    } else {
                        state.selected_ids = vec![id];
                    }
                } else if !shift {
                    state.selected_ids.clear();
                }
            }
            Tool::ShapeBuilder => {
                // Collect selected objects
                let mut sel_objs: Vec<Object> = Vec::new();
                for id in &state.selected_ids {
                    if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id)
                    {
                        sel_objs.push(obj.clone());
                    }
                }
                if !sel_objs.is_empty() {
                    let obj_refs: Vec<&Object> = sel_objs.iter().collect();
                    let frags =
                        crate::core::shape_builder::decompose_shapes_into_fragments(&obj_refs);
                    // Find which fragment contains (wx, wy)
                    for frag in frags {
                        if crate::core::geometry::point_in_polygon(wx, wy, &frag.polygon) {
                            // Extract clicked fragment as independent object
                            let mut base = sel_objs
                                [frag.original_object_indices.first().copied().unwrap_or(0)]
                            .clone();
                            base.fill = Some(crate::core::path::FillStyle::solid(state.fill_color));
                            let new_obj =
                                crate::core::shape_builder::fragment_to_object(&frag, &base);
                            let new_id = new_obj.id.clone();
                            let cmd =
                                Box::new(crate::core::history::AddObjectCommand::new(new_obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                            state.selected_ids = vec![new_id];
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn hit_test_handles(
        &self,
        obj: &Object,
        screen_pos: Pos2,
        origin: Pos2,
        state: &AppState,
    ) -> Option<HandleCorner> {
        if let Some((bb_min, bb_max)) = obj.bounding_box() {
            let min_p = Pos2::new(
                origin.x + bb_min.x as f32 * state.zoom,
                origin.y + bb_min.y as f32 * state.zoom,
            );
            let max_p = Pos2::new(
                origin.x + bb_max.x as f32 * state.zoom,
                origin.y + bb_max.y as f32 * state.zoom,
            );
            let rect = Rect::from_min_max(min_p, max_p);

            let rot_p = Pos2::new(rect.right_top().x + 12.0_f32, rect.right_top().y - 12.0_f32);
            if screen_pos.distance(rot_p) <= HANDLE_HIT_RADIUS {
                return Some(HandleCorner::RotateTopRight);
            }

            let handles = [
                (rect.left_top(), HandleCorner::TopLeft),
                (rect.right_top(), HandleCorner::TopRight),
                (rect.right_bottom(), HandleCorner::BottomRight),
                (rect.left_bottom(), HandleCorner::BottomLeft),
                (rect.center_top(), HandleCorner::Top),
                (rect.center_bottom(), HandleCorner::Bottom),
                (rect.left_center(), HandleCorner::Left),
                (rect.right_center(), HandleCorner::Right),
            ];

            for (p, corner) in handles {
                if screen_pos.distance(p) <= HANDLE_HIT_RADIUS {
                    return Some(corner);
                }
            }
        }
        None
    }

    pub(super) fn update_rotate(&self, state: &mut AppState, wx: f64, wy: f64, snap_15_deg: bool) {
        // Snapshot once per gesture so rotation is a single undo step.
        for id in &state.selected_ids.clone() {
            state.ensure_transform_snapshot(id);
        }
        for id in &state.selected_ids.clone() {
            if let Some(obj) = state.document.find_object_mut(id) {
                let cx = obj.transform.x;
                let cy = obj.transform.y;
                let mut angle = (wy - cy).atan2(wx - cx);
                if snap_15_deg {
                    let snap_step = std::f64::consts::PI / 12.0; // 15 degrees
                    angle = (angle / snap_step).round() * snap_step;
                }
                obj.transform.rotation = angle;
            }
        }
    }

    pub(super) fn update_resize(
        &self,
        state: &mut AppState,
        corner: HandleCorner,
        wx: f64,
        wy: f64,
    ) {
        // Snapshot once per gesture; the stop arm commits one undo step.
        if let Some(id) = state.selected_ids.first().cloned() {
            state.ensure_transform_snapshot(&id);
        }
        if let Some(ref drag) = self.drag {
            let (start_wx, start_wy) = drag.start_world;
            let id = match state.selected_ids.first() {
                Some(id) => id.clone(),
                None => return,
            };

            // Get current bounding box
            let (bb_min, bb_max) =
                if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| o.id == id) {
                    if let Some((min, max)) = obj.bounding_box() {
                        (min, max)
                    } else {
                        return;
                    }
                } else {
                    return;
                };

            let bb_w = bb_max.x - bb_min.x;
            let bb_h = bb_max.y - bb_min.y;
            if bb_w < 1.0 || bb_h < 1.0 {
                return;
            }

            let dx = wx - start_wx;
            let dy = wy - start_wy;

            if let Some(obj) = state.document.find_object_mut(&id) {
                    match corner {
                        HandleCorner::BottomRight => {
                            let new_w = (bb_w + dx).max(5.0);
                            let new_h = (bb_h + dy).max(5.0);
                            obj.transform.scale_x = new_w / bb_w;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::TopLeft => {
                            let new_w = (bb_w - dx).max(5.0);
                            let new_h = (bb_h - dy).max(5.0);
                            obj.transform.x += bb_w - new_w;
                            obj.transform.y += bb_h - new_h;
                            obj.transform.scale_x = new_w / bb_w;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::TopRight => {
                            let new_w = (bb_w + dx).max(5.0);
                            let new_h = (bb_h - dy).max(5.0);
                            obj.transform.y += bb_h - new_h;
                            obj.transform.scale_x = new_w / bb_w;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::BottomLeft => {
                            let new_w = (bb_w - dx).max(5.0);
                            let new_h = (bb_h + dy).max(5.0);
                            obj.transform.x += bb_w - new_w;
                            obj.transform.scale_x = new_w / bb_w;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::Top => {
                            let new_h = (bb_h - dy).max(5.0);
                            obj.transform.y += bb_h - new_h;
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::Bottom => {
                            let new_h = (bb_h + dy).max(5.0);
                            obj.transform.scale_y = new_h / bb_h;
                        }
                        HandleCorner::Left => {
                            let new_w = (bb_w - dx).max(5.0);
                            obj.transform.x += bb_w - new_w;
                            obj.transform.scale_x = new_w / bb_w;
                        }
                        HandleCorner::Right => {
                            let new_w = (bb_w + dx).max(5.0);
                            obj.transform.scale_x = new_w / bb_w;
                        }
                        _ => {}
                    }
            }
        }
    }

    pub(super) fn hit_test_nodes(
        &self,
        state: &AppState,
        screen_pos: Pos2,
        origin: Pos2,
    ) -> Option<(super::NodeTarget, String)> {
        let active_target = self.node_edit_state.selected_target;
        let active_node_idx = active_target.map(|t| t.elem_idx());
        let active_obj_id = self.node_edit_state.selected_object_id.as_deref();

        // 1. Check handles of currently selected active anchor first
        if let (Some(obj_id), Some(idx)) = (active_obj_id, active_node_idx) {
            if let Some(obj) = state.document.find_object(obj_id) {
                let path_data = obj.to_path_data();
                let elements = &path_data.elements;
                if let Some(PathElement::CurveTo(seg)) = elements.get(idx) {
                    {
                        let (c2_wx, c2_wy) = obj
                            .transform
                            .transform_point(seg.control2.x, seg.control2.y);
                        let c2_sp = Pos2::new(
                            origin.x + c2_wx as f32 * state.zoom,
                            origin.y + c2_wy as f32 * state.zoom,
                        );
                        if screen_pos.distance(c2_sp) <= HANDLE_HIT_RADIUS {
                            return Some((super::NodeTarget::Control2(idx), obj_id.to_string()));
                        }
                    }
                }
                if let Some(PathElement::CurveTo(next_seg)) = elements.get(idx + 1) {
                    let (c1_wx, c1_wy) = obj
                        .transform
                        .transform_point(next_seg.control1.x, next_seg.control1.y);
                    let c1_sp = Pos2::new(
                        origin.x + c1_wx as f32 * state.zoom,
                        origin.y + c1_wy as f32 * state.zoom,
                    );
                    if screen_pos.distance(c1_sp) <= HANDLE_HIT_RADIUS {
                        return Some((super::NodeTarget::Control1(idx + 1), obj_id.to_string()));
                    }
                }
            }
        }

        // 2. Check all anchors of selected objects
        for id in &state.selected_ids {
            if let Some(obj) = state.document.find_object(id) {
                let path_data = obj.to_path_data();
                for (idx, elem) in path_data.elements.iter().enumerate() {
                    let anchor_local = match elem {
                        PathElement::MoveTo(p) | PathElement::LineTo(p) => *p,
                        PathElement::CurveTo(seg) => seg.end,
                        PathElement::ClosePath => continue,
                    };
                    let (awx, awy) = obj
                        .transform
                        .transform_point(anchor_local.x, anchor_local.y);
                    let sp = Pos2::new(
                        origin.x + awx as f32 * state.zoom,
                        origin.y + awy as f32 * state.zoom,
                    );
                    if screen_pos.distance(sp) <= HANDLE_HIT_RADIUS {
                        return Some((super::NodeTarget::Anchor(idx), id.clone()));
                    }
                }
            }
        }
        None
    }

    pub(super) fn move_node(
        &self,
        state: &mut AppState,
        target: super::NodeTarget,
        wx: f64,
        wy: f64,
    ) {
        if let Some(ref obj_id) = self.node_edit_state.selected_object_id.clone() {
            if let Some(obj) = state.document.find_object_mut(obj_id) {
                    if !matches!(obj.object_type, ObjectType::Path(_)) {
                        let p = obj.to_path_data();
                        obj.object_type = ObjectType::Path(p);
                    }
                    let (lx, ly) = obj.transform.inverse_transform_point(wx, wy);
                    if let ObjectType::Path(ref mut path) = obj.object_type {
                        match target {
                            super::NodeTarget::Anchor(elem_idx) => {
                                let total = path.elements.len();
                                if elem_idx < total {
                                    let old_anchor = match &path.elements[elem_idx] {
                                        PathElement::MoveTo(p) | PathElement::LineTo(p) => *p,
                                        PathElement::CurveTo(seg) => seg.end,
                                        PathElement::ClosePath => return,
                                    };
                                    let dx = lx - old_anchor.x;
                                    let dy = ly - old_anchor.y;

                                    match &mut path.elements[elem_idx] {
                                        PathElement::MoveTo(p) | PathElement::LineTo(p) => {
                                            *p = AnchorPoint::new(lx, ly);
                                        }
                                        PathElement::CurveTo(seg) => {
                                            seg.end = AnchorPoint::new(lx, ly);
                                            seg.control2.x += dx;
                                            seg.control2.y += dy;
                                        }
                                        PathElement::ClosePath => {}
                                    }

                                    if elem_idx + 1 < total {
                                        if let PathElement::CurveTo(ref mut next_seg) =
                                            path.elements[elem_idx + 1]
                                        {
                                            next_seg.start = AnchorPoint::new(lx, ly);
                                            next_seg.control1.x += dx;
                                            next_seg.control1.y += dy;
                                        }
                                    }
                                }
                            }
                            super::NodeTarget::Control1(elem_idx) => {
                                if let Some(PathElement::CurveTo(ref mut seg)) =
                                    path.elements.get_mut(elem_idx)
                                {
                                    seg.control1 = AnchorPoint::new(lx, ly);
                                }
                            }
                            super::NodeTarget::Control2(elem_idx) => {
                                if let Some(PathElement::CurveTo(ref mut seg)) =
                                    path.elements.get_mut(elem_idx)
                                {
                                    seg.control2 = AnchorPoint::new(lx, ly);
                                }
                            }
                        }
                    }
            }
        }
    }
}
