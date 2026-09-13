use super::{CanvasWidget, DragMode};
use crate::core::document::Object;
use crate::core::path::{FillStyle, PathData, StrokeStyle};
use crate::core::state::AppState;

impl CanvasWidget {
    pub(super) fn handle_drag_stopped(
        &mut self,
        state: &mut AppState,
        shift_down: bool,
        alt_down: bool,
    ) {
        if let Some(drag) = self.drag.take() {
            match drag.mode {
                DragMode::CreateRect => {
                    let mut w = drag.current_world.0 - drag.start_world.0;
                    let mut h = drag.current_world.1 - drag.start_world.1;
                    if shift_down {
                        let sz = w.abs().max(h.abs());
                        w = sz * w.signum();
                        h = sz * h.signum();
                    }
                    if w.abs() > 4.0 && h.abs() > 4.0 {
                        let (x, y, w_val, h_val) = if alt_down {
                            (
                                drag.start_world.0 - w.abs(),
                                drag.start_world.1 - h.abs(),
                                w.abs() * 2.0,
                                h.abs() * 2.0,
                            )
                        } else {
                            (
                                drag.start_world.0.min(drag.current_world.0),
                                drag.start_world.1.min(drag.current_world.1),
                                w.abs(),
                                h.abs(),
                            )
                        };
                        let mut obj =
                            Object::new_rect("Rectangle", x, y, w_val, h_val, state.corner_radius);
                        obj.fill = Some(FillStyle::solid(state.fill_color));
                        obj.stroke = Some(StrokeStyle {
                            color: state.stroke_color,
                            width: state.stroke_width,
                            dash_pattern: None,
                            ..StrokeStyle::default()
                        });
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                }
                DragMode::CreateEllipse => {
                    let mut rx = ((drag.current_world.0 - drag.start_world.0) / 2.0).abs();
                    let mut ry = ((drag.current_world.1 - drag.start_world.1) / 2.0).abs();
                    if shift_down {
                        let r = rx.max(ry);
                        rx = r;
                        ry = r;
                    }
                    if rx > 2.0 && ry > 2.0 {
                        let (cx, cy) = if alt_down {
                            (drag.start_world.0, drag.start_world.1)
                        } else {
                            (
                                (drag.start_world.0 + drag.current_world.0) / 2.0,
                                (drag.start_world.1 + drag.current_world.1) / 2.0,
                            )
                        };
                        let mut obj = Object::new_ellipse("Ellipse", cx, cy, rx, ry);
                        obj.fill = Some(FillStyle::solid(state.fill_color));
                        obj.stroke = Some(StrokeStyle {
                            color: state.stroke_color,
                            width: state.stroke_width,
                            dash_pattern: None,
                            ..StrokeStyle::default()
                        });
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                }
                DragMode::CreateStar => {
                    let dx = drag.current_world.0 - drag.start_world.0;
                    let dy = drag.current_world.1 - drag.start_world.1;
                    let outer_radius = (dx * dx + dy * dy).sqrt();
                    if outer_radius > 4.0 {
                        let inner_radius = outer_radius * state.star_inner_ratio;
                        let mut obj = Object::new_star(
                            "Star",
                            drag.start_world.0,
                            drag.start_world.1,
                            state.star_points,
                            inner_radius,
                            outer_radius,
                        );
                        obj.fill = Some(FillStyle::solid(state.fill_color));
                        obj.stroke = Some(StrokeStyle {
                            color: state.stroke_color,
                            width: state.stroke_width,
                            dash_pattern: None,
                            ..StrokeStyle::default()
                        });
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                }
                DragMode::CreatePolygon => {
                    let dx = drag.current_world.0 - drag.start_world.0;
                    let dy = drag.current_world.1 - drag.start_world.1;
                    let radius = (dx * dx + dy * dy).sqrt();
                    if radius > 4.0 {
                        let mut obj = Object::new_polygon(
                            "Polygon",
                            drag.start_world.0,
                            drag.start_world.1,
                            state.polygon_sides,
                            radius,
                        );
                        obj.fill = Some(FillStyle::solid(state.fill_color));
                        obj.stroke = Some(StrokeStyle {
                            color: state.stroke_color,
                            width: state.stroke_width,
                            dash_pattern: None,
                            ..StrokeStyle::default()
                        });
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                }
                DragMode::CreateLine => {
                    let mut dx = drag.current_world.0 - drag.start_world.0;
                    let mut dy = drag.current_world.1 - drag.start_world.1;
                    if shift_down {
                        let angle = dy.atan2(dx);
                        let snap_step = std::f64::consts::FRAC_PI_4;
                        let snapped = (angle / snap_step).round() * snap_step;
                        let len = (dx * dx + dy * dy).sqrt();
                        dx = len * snapped.cos();
                        dy = len * snapped.sin();
                    }
                    if (dx * dx + dy * dy).sqrt() > 2.0 {
                        let mut obj = Object::new_line(
                            "Line",
                            drag.start_world.0,
                            drag.start_world.1,
                            drag.start_world.0 + dx,
                            drag.start_world.1 + dy,
                        );
                        obj.stroke = Some(StrokeStyle {
                            color: state.stroke_color,
                            width: state.stroke_width,
                            dash_pattern: None,
                            ..StrokeStyle::default()
                        });
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                }
                DragMode::MoveObject => {
                    let moved = self.select_state.end_drag(state);
                    if alt_down {
                        // Alt+Drag duplicates selected objects!
                        let mut duplicated = Vec::new();
                        for id in &state.selected_ids {
                            if let Some((_, obj)) =
                                state.document.all_objects().find(|(_, o)| &o.id == id)
                            {
                                let mut dup = obj.clone();
                                dup.id = uuid::Uuid::new_v4().to_string();
                                dup.name = format!("{} Copy", obj.name);
                                duplicated.push(dup);
                            }
                        }
                        if duplicated.len() == 1 {
                            let dup = duplicated.pop().unwrap();
                            let new_id = dup.id.clone();
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(dup));
                            state.undo_manager.execute(cmd, &mut state.document);
                            state.selected_ids = vec![new_id];
                        } else if !duplicated.is_empty() {
                            let mut new_ids = Vec::new();
                            let mut cmds: Vec<Box<dyn crate::core::history::Command>> = Vec::new();
                            for dup in duplicated {
                                new_ids.push(dup.id.clone());
                                cmds.push(Box::new(crate::core::history::AddObjectCommand::new(
                                    dup,
                                )));
                            }
                            let batch = Box::new(crate::core::history::BatchCommand::new(
                                "Duplicate Objects",
                                cmds,
                            ));
                            state.undo_manager.execute(batch, &mut state.document);
                            state.selected_ids = new_ids;
                        }
                    } else {
                        // Record MoveObjectCommand so movement is undoable and redoable
                        if moved.len() == 1 {
                            let (id, old_x, old_y, new_x, new_y) =
                                moved.into_iter().next().unwrap();
                            let cmd = Box::new(crate::core::history::MoveObjectCommand {
                                object_id: id,
                                old_x,
                                old_y,
                                new_x,
                                new_y,
                            });
                            state.undo_manager.execute(cmd, &mut state.document);
                        } else if !moved.is_empty() {
                            let cmds: Vec<Box<dyn crate::core::history::Command>> = moved
                                .into_iter()
                                .map(|(id, old_x, old_y, new_x, new_y)| {
                                    Box::new(crate::core::history::MoveObjectCommand {
                                        object_id: id,
                                        old_x,
                                        old_y,
                                        new_x,
                                        new_y,
                                    })
                                        as Box<dyn crate::core::history::Command>
                                })
                                .collect();
                            let batch = Box::new(crate::core::history::BatchCommand::new(
                                "Move Objects",
                                cmds,
                            ));
                            state.undo_manager.execute(batch, &mut state.document);
                        }
                    }
                }
                DragMode::PencilDraw => {
                    if drag.pencil_points.len() >= 2 {
                        let mut path = PathData::from_smooth_points(&drag.pencil_points);
                        path.fill = None;
                        path.stroke = Some(StrokeStyle {
                            color: state.stroke_color,
                            width: state.stroke_width,
                            dash_pattern: None,
                            ..StrokeStyle::default()
                        });
                        let obj = Object::new_path("Pencil Stroke", path);
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                }
                DragMode::BrushDraw => {
                    if drag.pencil_points.len() >= 2 {
                        let mut path = PathData::from_smooth_points(&drag.pencil_points);
                        path.fill = None;
                        path.stroke = Some(StrokeStyle {
                            color: state.fill_color,
                            width: 4.0,
                            dash_pattern: None,
                            ..StrokeStyle::default()
                        });
                        let obj = Object::new_path("Brush Stroke", path);
                        let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                        state.undo_manager.execute(cmd, &mut state.document);
                    }
                }
                DragMode::EraserDrag => {
                    if drag.pencil_points.len() >= 2 {
                        let min_x = drag
                            .pencil_points
                            .iter()
                            .map(|p| p.x)
                            .fold(f64::INFINITY, f64::min);
                        let max_x = drag
                            .pencil_points
                            .iter()
                            .map(|p| p.x)
                            .fold(f64::NEG_INFINITY, f64::max);
                        let min_y = drag
                            .pencil_points
                            .iter()
                            .map(|p| p.y)
                            .fold(f64::INFINITY, f64::min);
                        let max_y = drag
                            .pencil_points
                            .iter()
                            .map(|p| p.y)
                            .fold(f64::NEG_INFINITY, f64::max);

                        let eraser_margin = 10.0;
                        let eraser_rect_min = (min_x - eraser_margin, min_y - eraser_margin);
                        let eraser_rect_max = (max_x + eraser_margin, max_y + eraser_margin);

                        let mut ids_to_remove = Vec::new();
                        for (_, obj) in state.document.all_objects() {
                            if !obj.visible || obj.locked {
                                continue;
                            }
                            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                                let intersects = bb_max.x >= eraser_rect_min.0
                                    && bb_min.x <= eraser_rect_max.0
                                    && bb_max.y >= eraser_rect_min.1
                                    && bb_min.y <= eraser_rect_max.1;
                                if intersects {
                                    ids_to_remove.push(obj.id.clone());
                                }
                            }
                        }
                        for id in &ids_to_remove {
                            let layer_idx = state
                                .document
                                .layers
                                .iter()
                                .position(|l| l.objects.iter().any(|o| &o.id == id))
                                .unwrap_or(0);
                            if let Some(obj) = state.document.remove_object(id) {
                                let cmd = Box::new(crate::core::history::RemoveObjectCommand::new(
                                    obj, layer_idx, 0,
                                ));
                                state.undo_manager.execute(cmd, &mut state.document);
                            }
                        }
                    }
                }
                DragMode::Select => {
                    let min_x = drag.start_world.0.min(drag.current_world.0);
                    let max_x = drag.start_world.0.max(drag.current_world.0);
                    let min_y = drag.start_world.1.min(drag.current_world.1);
                    let max_y = drag.start_world.1.max(drag.current_world.1);

                    state.selected_ids.clear();
                    for (_, obj) in state.document.all_objects() {
                        if !obj.visible || obj.locked {
                            continue;
                        }
                        if let Some((bb_min, bb_max)) = obj.bounding_box() {
                            if bb_max.x >= min_x
                                && bb_min.x <= max_x
                                && bb_max.y >= min_y
                                && bb_min.y <= max_y
                            {
                                state.selected_ids.push(obj.id.clone());
                            }
                        }
                    }
                }
                DragMode::DragGuideHorizontal => {
                    state.guides.push(crate::core::state::Guide {
                        orientation: crate::core::state::GuideOrientation::Horizontal,
                        position: drag.current_world.1,
                    });
                }
                DragMode::DragGuideVertical => {
                    state.guides.push(crate::core::state::Guide {
                        orientation: crate::core::state::GuideOrientation::Vertical,
                        position: drag.current_world.0,
                    });
                }
                DragMode::MoveNode(_) => {
                    if let Some(initial) = drag.initial_elements {
                        if let Some(ref obj_id) = self.node_edit_state.selected_object_id {
                            let current_elements = state
                                .document
                                .all_objects()
                                .find(|(_, o)| &o.id == obj_id)
                                .and_then(|(_, o)| match &o.object_type {
                                    crate::core::document::ObjectType::Path(p) => {
                                        Some(p.elements.clone())
                                    }
                                    _ => None,
                                });
                            if let Some(current) = current_elements {
                                if current != initial {
                                    let cmd =
                                        Box::new(crate::core::history::ModifyPathCommand::new(
                                            obj_id.clone(),
                                            initial,
                                            current,
                                        ));
                                    state.undo_manager.execute(cmd, &mut state.document);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
