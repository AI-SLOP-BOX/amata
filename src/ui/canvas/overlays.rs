use super::{CanvasWidget, DragState, HANDLE_SIZE, RULER_WIDTH};
use crate::core::document::Object;
use crate::core::path::PathData;
use crate::core::state::AppState;
use egui::{Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

impl CanvasWidget {
    pub(super) fn draw_grid(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        origin: Pos2,
        state: &AppState,
    ) {
        let grid_px = state.grid_size as f32 * state.zoom;
        if grid_px < 5.0_f32 {
            return;
        }

        let start_x = ((rect.min.x - origin.x) / grid_px).floor() * grid_px + origin.x;
        let start_y = ((rect.min.y - origin.y) / grid_px).floor() * grid_px + origin.y;

        let grid_stroke = Stroke::new(0.5_f32, Color32::from_rgb(50, 52, 58));

        let mut x = start_x;
        while x <= rect.max.x {
            painter.line_segment(
                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                grid_stroke,
            );
            x += grid_px;
        }

        let mut y = start_y;
        while y <= rect.max.y {
            painter.line_segment(
                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                grid_stroke,
            );
            y += grid_px;
        }
    }

    pub(super) fn draw_rulers(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        origin: Pos2,
        state: &AppState,
    ) {
        let ruler_color = Color32::from_rgb(34, 34, 34);
        let tick_major_color = Color32::from_rgb(140, 140, 140);
        let tick_minor_color = Color32::from_rgb(80, 80, 80);
        let font_id = FontId::proportional(8.5_f32);

        // Top ruler bar
        let top_ruler = Rect::from_min_size(rect.min, Vec2::new(rect.width(), RULER_WIDTH));
        painter.rect_filled(top_ruler, 0.0_f32, ruler_color);
        painter.line_segment(
            [
                Pos2::new(rect.min.x, rect.min.y + RULER_WIDTH),
                Pos2::new(rect.max.x, rect.min.y + RULER_WIDTH),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(50, 50, 50)),
        );

        // Left ruler bar
        let left_ruler = Rect::from_min_size(rect.min, Vec2::new(RULER_WIDTH, rect.height()));
        painter.rect_filled(left_ruler, 0.0_f32, ruler_color);
        painter.line_segment(
            [
                Pos2::new(rect.min.x + RULER_WIDTH, rect.min.y),
                Pos2::new(rect.min.x + RULER_WIDTH, rect.max.y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(50, 50, 50)),
        );

        // Top-left origin corner box
        painter.rect_filled(
            Rect::from_min_size(rect.min, Vec2::splat(RULER_WIDTH)),
            0.0_f32,
            Color32::from_rgb(28, 28, 28),
        );

        // Scale steps
        let step: f32 = if state.zoom > 3.0_f32 {
            20.0_f32
        } else if state.zoom > 1.5_f32 {
            50.0_f32
        } else if state.zoom < 0.3_f32 {
            500.0_f32
        } else {
            100.0_f32
        };
        let step_px = step * state.zoom;
        let start_val = ((rect.min.x - origin.x) / step_px).floor() * step;

        // Horizontal ruler ticks
        let mut val = start_val;
        while val * state.zoom + origin.x <= rect.max.x {
            let sx = origin.x + (val * state.zoom);
            if sx >= rect.min.x + RULER_WIDTH {
                // Major tick with label
                painter.line_segment(
                    [
                        Pos2::new(sx, rect.min.y + 11.0_f32),
                        Pos2::new(sx, rect.min.y + RULER_WIDTH),
                    ],
                    Stroke::new(1.0_f32, tick_major_color),
                );
                painter.text(
                    Pos2::new(sx + 2.0_f32, rect.min.y + 1.5_f32),
                    egui::Align2::LEFT_TOP,
                    format!("{val:.0}"),
                    font_id.clone(),
                    tick_major_color,
                );

                // Sub-ticks
                let mid_sx = sx + (step_px * 0.5);
                if mid_sx <= rect.max.x {
                    painter.line_segment(
                        [
                            Pos2::new(mid_sx, rect.min.y + 14.0_f32),
                            Pos2::new(mid_sx, rect.min.y + RULER_WIDTH),
                        ],
                        Stroke::new(0.8_f32, tick_minor_color),
                    );
                }
            }
            val += step;
        }

        // Vertical ruler ticks
        let mut val_y = ((rect.min.y - origin.y) / step_px).floor() * step;
        while val_y * state.zoom + origin.y <= rect.max.y {
            let sy = origin.y + (val_y * state.zoom);
            if sy >= rect.min.y + RULER_WIDTH {
                // Major tick with label
                painter.line_segment(
                    [
                        Pos2::new(rect.min.x + 11.0_f32, sy),
                        Pos2::new(rect.min.x + RULER_WIDTH, sy),
                    ],
                    Stroke::new(1.0_f32, tick_major_color),
                );
                painter.text(
                    Pos2::new(rect.min.x + 2.0_f32, sy + 1.5_f32),
                    egui::Align2::LEFT_TOP,
                    format!("{val_y:.0}"),
                    font_id.clone(),
                    tick_major_color,
                );

                // Sub-ticks
                let mid_sy = sy + (step_px * 0.5);
                if mid_sy <= rect.max.y {
                    painter.line_segment(
                        [
                            Pos2::new(rect.min.x + 14.0_f32, mid_sy),
                            Pos2::new(rect.min.x + RULER_WIDTH, mid_sy),
                        ],
                        Stroke::new(0.8_f32, tick_minor_color),
                    );
                }
            }
            val_y += step;
        }

        // Illustrator signature live cursor projection indicator line on rulers
        if let Some((cx, cy)) = state.cursor_world {
            let screen_cx = origin.x + (cx as f32 * state.zoom);
            let screen_cy = origin.y + (cy as f32 * state.zoom);

            // Cursor tick on top ruler
            if screen_cx >= rect.min.x + RULER_WIDTH && screen_cx <= rect.max.x {
                painter.line_segment(
                    [
                        Pos2::new(screen_cx, rect.min.y),
                        Pos2::new(screen_cx, rect.min.y + RULER_WIDTH),
                    ],
                    Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)),
                );
            }
            // Cursor tick on left ruler
            if screen_cy >= rect.min.y + RULER_WIDTH && screen_cy <= rect.max.y {
                painter.line_segment(
                    [
                        Pos2::new(rect.min.x, screen_cy),
                        Pos2::new(rect.min.x + RULER_WIDTH, screen_cy),
                    ],
                    Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)),
                );
            }
        }
    }

    /// Draw user-placed cyan guide lines (Illustrator signature cyan guides)
    pub(super) fn draw_user_guides(
        &self,
        painter: &egui::Painter,
        canvas_rect: Rect,
        origin: Pos2,
        state: &AppState,
    ) {
        if !state.show_smart_guides && state.guides.is_empty() {
            return;
        }
        let cyan_guide_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0, 200, 255));

        for guide in &state.guides {
            match guide.orientation {
                crate::core::state::GuideOrientation::Horizontal => {
                    let sy = origin.y + (guide.position as f32 * state.zoom);
                    if sy >= canvas_rect.min.y && sy <= canvas_rect.max.y {
                        painter.line_segment(
                            [
                                Pos2::new(canvas_rect.min.x, sy),
                                Pos2::new(canvas_rect.max.x, sy),
                            ],
                            cyan_guide_stroke,
                        );
                    }
                }
                crate::core::state::GuideOrientation::Vertical => {
                    let sx = origin.x + (guide.position as f32 * state.zoom);
                    if sx >= canvas_rect.min.x && sx <= canvas_rect.max.x {
                        painter.line_segment(
                            [
                                Pos2::new(sx, canvas_rect.min.y),
                                Pos2::new(sx, canvas_rect.max.y),
                            ],
                            cyan_guide_stroke,
                        );
                    }
                }
            }
        }
    }

    pub(super) fn draw_smart_guides(
        &self,
        painter: &egui::Painter,
        artboard: Rect,
        canvas_rect: Rect,
        origin: Pos2,
        state: &AppState,
    ) {
        if !state.show_smart_guides {
            return;
        }
        let guide_stroke = Stroke::new(1.0_f32, Color32::from_rgb(255, 0, 128)); // Adobe Smart Guide Magenta

        // Artboard center guides
        let center_x = artboard.center().x;
        let center_y = artboard.center().y;
        painter.line_segment(
            [
                Pos2::new(center_x, canvas_rect.min.y),
                Pos2::new(center_x, canvas_rect.max.y),
            ],
            guide_stroke,
        );
        painter.line_segment(
            [
                Pos2::new(canvas_rect.min.x, center_y),
                Pos2::new(canvas_rect.max.x, center_y),
            ],
            guide_stroke,
        );

        // Object alignment guides during selection
        if state.selected_ids.is_empty() {
            return;
        }

        let mut sel_min_opt: Option<crate::core::path::AnchorPoint> = None;
        let mut sel_max_opt: Option<crate::core::path::AnchorPoint> = None;
        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                if let Some((min, max)) = obj.bounding_box() {
                    sel_min_opt = Some(match sel_min_opt {
                        None => min,
                        Some(cur) => {
                            crate::core::path::AnchorPoint::new(cur.x.min(min.x), cur.y.min(min.y))
                        }
                    });
                    sel_max_opt = Some(match sel_max_opt {
                        None => max,
                        Some(cur) => {
                            crate::core::path::AnchorPoint::new(cur.x.max(max.x), cur.y.max(max.y))
                        }
                    });
                }
            }
        }

        if let (Some(s_min), Some(s_max)) = (sel_min_opt, sel_max_opt) {
            let s_cx = (s_min.x + s_max.x) * 0.5;
            let s_cy = (s_min.y + s_max.y) * 0.5;
            let tol = 4.0 / state.zoom as f64;

            for (_, other) in state.document.all_objects() {
                if state.selected_ids.contains(&other.id) || !other.visible {
                    continue;
                }
                if let Some((o_min, o_max)) = other.bounding_box() {
                    let o_cx = (o_min.x + o_max.x) * 0.5;
                    let o_cy = (o_min.y + o_max.y) * 0.5;

                    for x_val in [o_min.x, o_cx, o_max.x] {
                        if (s_min.x - x_val).abs() < tol
                            || (s_cx - x_val).abs() < tol
                            || (s_max.x - x_val).abs() < tol
                        {
                            let sx = origin.x + x_val as f32 * state.zoom;
                            painter.line_segment(
                                [
                                    Pos2::new(sx, canvas_rect.min.y),
                                    Pos2::new(sx, canvas_rect.max.y),
                                ],
                                guide_stroke,
                            );
                        }
                    }

                    for y_val in [o_min.y, o_cy, o_max.y] {
                        if (s_min.y - y_val).abs() < tol
                            || (s_cy - y_val).abs() < tol
                            || (s_max.y - y_val).abs() < tol
                        {
                            let sy = origin.y + y_val as f32 * state.zoom;
                            painter.line_segment(
                                [
                                    Pos2::new(canvas_rect.min.x, sy),
                                    Pos2::new(canvas_rect.max.x, sy),
                                ],
                                guide_stroke,
                            );
                        }
                    }
                }
            }
        }
    }

    pub(super) fn draw_selection(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        origin: Pos2,
        state: &AppState,
    ) {
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

            let sel_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230));
            painter.rect_stroke(rect, 0.0_f32, sel_stroke, StrokeKind::Outside);

            // Center target crosshair (+)
            let cp = rect.center();
            let c_size = 4.0_f32;
            let c_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230));
            painter.line_segment(
                [
                    Pos2::new(cp.x - c_size, cp.y),
                    Pos2::new(cp.x + c_size, cp.y),
                ],
                c_stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(cp.x, cp.y - c_size),
                    Pos2::new(cp.x, cp.y + c_size),
                ],
                c_stroke,
            );

            // 8 handles (Hollow white square with blue border)
            let corners = [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
                rect.center_top(),
                rect.center_bottom(),
                rect.left_center(),
                rect.right_center(),
            ];

            for p in corners {
                let h_rect = Rect::from_center_size(p, Vec2::splat(HANDLE_SIZE));
                painter.rect_filled(h_rect, 0.0_f32, Color32::WHITE);
                painter.rect_stroke(
                    h_rect,
                    0.0_f32,
                    Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)),
                    StrokeKind::Outside,
                );
            }

            // Top Rotation handle circle above right_top
            let rot_p = Pos2::new(rect.right_top().x + 12.0_f32, rect.right_top().y - 12.0_f32);
            painter.circle_filled(rot_p, 4.0_f32, Color32::from_rgb(20, 115, 230));
            painter.circle_stroke(rot_p, 4.0_f32, Stroke::new(1.0_f32, Color32::WHITE));

            // If rotated, show crisp angle readout badge
            if obj.transform.rotation.abs() > 0.001 {
                let deg = obj.transform.rotation.to_degrees();
                let badge_text = format!("∠ {:.1}°", deg);
                let badge_pos = Pos2::new(rot_p.x + 8.0, rot_p.y - 6.0);
                painter.text(
                    badge_pos,
                    egui::Align2::LEFT_CENTER,
                    badge_text,
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(20, 115, 230),
                );
            }
        }
    }

    pub(super) fn draw_node_edit(&self, painter: &egui::Painter, origin: Pos2, state: &AppState) {
        let active_target = self.node_edit_state.selected_target;
        let active_node_idx = active_target.map(|t| t.elem_idx());
        let active_obj_id = self.node_edit_state.selected_object_id.as_deref();

        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                let path_data = obj.to_path_data();
                let elements = &path_data.elements;

                // Draw wireframe path contour connecting nodes
                let poly = path_data.to_polygon(24);
                if poly.len() >= 2 {
                    let screen_pts: Vec<Pos2> = poly
                        .iter()
                        .map(|p| {
                            let (wx, wy) = obj.transform.transform_point(p.x, p.y);
                            Pos2::new(
                                origin.x + wx as f32 * state.zoom,
                                origin.y + wy as f32 * state.zoom,
                            )
                        })
                        .collect();
                    painter.add(egui::epaint::PathShape::closed_line(
                        screen_pts,
                        Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)),
                    ));
                }

                for (idx, elem) in elements.iter().enumerate() {
                    let anchor_local = match elem {
                        crate::core::path::PathElement::MoveTo(p)
                        | crate::core::path::PathElement::LineTo(p) => *p,
                        crate::core::path::PathElement::CurveTo(seg) => seg.end,
                        crate::core::path::PathElement::ClosePath => continue,
                    };

                    let (awx, awy) = obj
                        .transform
                        .transform_point(anchor_local.x, anchor_local.y);
                    let sp = Pos2::new(
                        origin.x + awx as f32 * state.zoom,
                        origin.y + awy as f32 * state.zoom,
                    );
                    let is_active =
                        active_obj_id == Some(id.as_str()) && active_node_idx == Some(idx);

                    let anchor_rect = Rect::from_center_size(sp, Vec2::splat(6.0));
                    if is_active {
                        // Selected anchor: Solid blue square with white outline
                        painter.rect_filled(anchor_rect, 0.0_f32, Color32::from_rgb(20, 115, 230));
                        painter.rect_stroke(
                            anchor_rect,
                            0.0_f32,
                            Stroke::new(1.0_f32, Color32::WHITE),
                            StrokeKind::Outside,
                        );

                        let handle_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230));

                        // If this element is CurveTo, draw incoming control handle (control2)
                        if let crate::core::path::PathElement::CurveTo(seg) = elem {
                            let (c2_wx, c2_wy) = obj
                                .transform
                                .transform_point(seg.control2.x, seg.control2.y);
                            let c2_sp = Pos2::new(
                                origin.x + c2_wx as f32 * state.zoom,
                                origin.y + c2_wy as f32 * state.zoom,
                            );
                            painter.line_segment([sp, c2_sp], handle_stroke);
                            let is_c2_active = matches!(active_target, Some(super::NodeTarget::Control2(i)) if i == idx);
                            let c2_col = if is_c2_active {
                                Color32::from_rgb(255, 120, 0)
                            } else {
                                Color32::from_rgb(20, 115, 230)
                            };
                            painter.circle_filled(c2_sp, 3.5_f32, c2_col);
                            painter.circle_stroke(
                                c2_sp,
                                3.5_f32,
                                Stroke::new(1.0_f32, Color32::WHITE),
                            );
                        }

                        // If the next element is CurveTo, draw outgoing control handle (control1)
                        if let Some(crate::core::path::PathElement::CurveTo(next_seg)) =
                            elements.get(idx + 1)
                        {
                            let (c1_wx, c1_wy) = obj
                                .transform
                                .transform_point(next_seg.control1.x, next_seg.control1.y);
                            let c1_sp = Pos2::new(
                                origin.x + c1_wx as f32 * state.zoom,
                                origin.y + c1_wy as f32 * state.zoom,
                            );
                            painter.line_segment([sp, c1_sp], handle_stroke);
                            let is_c1_active = matches!(active_target, Some(super::NodeTarget::Control1(i)) if i == idx + 1);
                            let c1_col = if is_c1_active {
                                Color32::from_rgb(255, 120, 0)
                            } else {
                                Color32::from_rgb(20, 115, 230)
                            };
                            painter.circle_filled(c1_sp, 3.5_f32, c1_col);
                            painter.circle_stroke(
                                c1_sp,
                                3.5_f32,
                                Stroke::new(1.0_f32, Color32::WHITE),
                            );
                        }

                        // Coordinate Tooltip badge
                        let badge_pos = Pos2::new(sp.x + 12.0, sp.y - 24.0);
                        let badge_rect = Rect::from_min_size(badge_pos, Vec2::new(96.0, 24.0));
                        painter.rect_filled(
                            badge_rect,
                            3.0,
                            Color32::from_rgba_unmultiplied(240, 240, 240, 240),
                        );
                        painter.rect_stroke(
                            badge_rect,
                            3.0,
                            Stroke::new(1.0_f32, Color32::from_rgb(160, 160, 160)),
                            StrokeKind::Outside,
                        );
                        painter.text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("X: {:.1} pt\nY: {:.1} pt", anchor_local.x, anchor_local.y),
                            egui::FontId::proportional(8.5),
                            Color32::from_rgb(30, 30, 30),
                        );
                    } else {
                        // Unselected anchor: Hollow white square with blue outline
                        painter.rect_filled(anchor_rect, 0.0_f32, Color32::WHITE);
                        painter.rect_stroke(
                            anchor_rect,
                            0.0_f32,
                            Stroke::new(1.2_f32, Color32::from_rgb(20, 115, 230)),
                            StrokeKind::Outside,
                        );
                    }
                }
            }
        }
    }

    pub(super) fn draw_rect_preview(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
        drag: &DragState,
    ) {
        let p1 = Pos2::new(
            origin.x + drag.start_world.0 as f32 * state.zoom,
            origin.y + drag.start_world.1 as f32 * state.zoom,
        );
        let p2 = Pos2::new(
            origin.x + drag.current_world.0 as f32 * state.zoom,
            origin.y + drag.current_world.1 as f32 * state.zoom,
        );
        let rect = Rect::from_two_pos(p1, p2);
        painter.rect_filled(
            rect,
            0.0_f32,
            Color32::from_rgba_unmultiplied(0, 120, 255, 40),
        );
        painter.rect_stroke(
            rect,
            0.0_f32,
            Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255)),
            StrokeKind::Outside,
        );
    }

    pub(super) fn draw_ellipse_preview(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
        drag: &DragState,
    ) {
        let p1 = Pos2::new(
            origin.x + drag.start_world.0 as f32 * state.zoom,
            origin.y + drag.start_world.1 as f32 * state.zoom,
        );
        let p2 = Pos2::new(
            origin.x + drag.current_world.0 as f32 * state.zoom,
            origin.y + drag.current_world.1 as f32 * state.zoom,
        );
        let rect = Rect::from_two_pos(p1, p2);
        painter.circle_filled(
            rect.center(),
            rect.width() / 2.0_f32,
            Color32::from_rgba_unmultiplied(0, 120, 255, 40),
        );
        painter.circle_stroke(
            rect.center(),
            rect.width() / 2.0_f32,
            Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255)),
        );
    }

    pub(super) fn draw_star_preview(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
        drag: &DragState,
    ) {
        let dx = drag.current_world.0 - drag.start_world.0;
        let dy = drag.current_world.1 - drag.start_world.1;
        let outer_r = (dx * dx + dy * dy).sqrt();
        let inner_r = outer_r * state.star_inner_ratio;
        let path = PathData::from_star(
            state.star_points,
            inner_r,
            outer_radius_to_f64(outer_r),
            drag.start_world.0,
            drag.start_world.1,
        );
        let screen_pts: Vec<Pos2> = path
            .to_polygon(1)
            .iter()
            .map(|p| {
                Pos2::new(
                    origin.x + p.x as f32 * state.zoom,
                    origin.y + p.y as f32 * state.zoom,
                )
            })
            .collect();
        painter.add(egui::epaint::PathShape::closed_line(
            screen_pts,
            Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255)),
        ));
    }

    pub(super) fn draw_polygon_preview(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
        drag: &DragState,
    ) {
        let dx = drag.current_world.0 - drag.start_world.0;
        let dy = drag.current_world.1 - drag.start_world.1;
        let radius = (dx * dx + dy * dy).sqrt();
        let path = PathData::from_polygon(
            state.polygon_sides,
            radius,
            drag.start_world.0,
            drag.start_world.1,
        );
        let screen_pts: Vec<Pos2> = path
            .to_polygon(1)
            .iter()
            .map(|p| {
                Pos2::new(
                    origin.x + p.x as f32 * state.zoom,
                    origin.y + p.y as f32 * state.zoom,
                )
            })
            .collect();
        painter.add(egui::epaint::PathShape::closed_line(
            screen_pts,
            Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255)),
        ));
    }

    pub(super) fn draw_line_preview(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
        drag: &DragState,
    ) {
        let p1 = Pos2::new(
            origin.x + drag.start_world.0 as f32 * state.zoom,
            origin.y + drag.start_world.1 as f32 * state.zoom,
        );
        let p2 = Pos2::new(
            origin.x + drag.current_world.0 as f32 * state.zoom,
            origin.y + drag.current_world.1 as f32 * state.zoom,
        );
        painter.line_segment(
            [p1, p2],
            Stroke::new(2.0_f32, Color32::from_rgb(0, 120, 255)),
        );
    }

    pub(super) fn draw_pencil_preview(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
        drag: &DragState,
    ) {
        if drag.pencil_points.len() >= 2 {
            let pts: Vec<Pos2> = drag
                .pencil_points
                .iter()
                .map(|p| {
                    Pos2::new(
                        origin.x + p.x as f32 * state.zoom,
                        origin.y + p.y as f32 * state.zoom,
                    )
                })
                .collect();
            painter.add(egui::epaint::PathShape::line(
                pts,
                Stroke::new(2.0_f32, Color32::from_rgb(0, 120, 255)),
            ));
        }
    }

    pub(super) fn draw_brush_preview(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
        drag: &DragState,
    ) {
        if drag.pencil_points.len() >= 2 {
            let pts: Vec<Pos2> = drag
                .pencil_points
                .iter()
                .map(|p| {
                    Pos2::new(
                        origin.x + p.x as f32 * state.zoom,
                        origin.y + p.y as f32 * state.zoom,
                    )
                })
                .collect();
            let fill_c = state.fill_color;
            let color = Color32::from_rgba_unmultiplied(
                (fill_c[0] * 255.0) as u8,
                (fill_c[1] * 255.0) as u8,
                (fill_c[2] * 255.0) as u8,
                (fill_c[3] * 255.0) as u8,
            );
            painter.add(egui::epaint::PathShape::line(
                pts,
                Stroke::new(4.0_f32, color),
            ));
        }
    }

    pub(super) fn draw_eraser_preview(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
        drag: &DragState,
    ) {
        if drag.pencil_points.len() >= 2 {
            let pts: Vec<Pos2> = drag
                .pencil_points
                .iter()
                .map(|p| {
                    Pos2::new(
                        origin.x + p.x as f32 * state.zoom,
                        origin.y + p.y as f32 * state.zoom,
                    )
                })
                .collect();
            painter.add(egui::epaint::PathShape::line(
                pts,
                Stroke::new(3.0_f32, Color32::from_rgba_unmultiplied(255, 80, 80, 180)),
            ));
        }
    }

    pub(super) fn draw_marquee(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
        drag: &DragState,
    ) {
        let p1 = Pos2::new(
            origin.x + drag.start_world.0 as f32 * state.zoom,
            origin.y + drag.start_world.1 as f32 * state.zoom,
        );
        let p2 = Pos2::new(
            origin.x + drag.current_world.0 as f32 * state.zoom,
            origin.y + drag.current_world.1 as f32 * state.zoom,
        );
        let rect = Rect::from_two_pos(p1, p2);
        painter.rect_filled(
            rect,
            0.0_f32,
            Color32::from_rgba_unmultiplied(0, 120, 255, 30),
        );
        painter.rect_stroke(
            rect,
            0.0_f32,
            Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 255)),
            StrokeKind::Outside,
        );
    }

    pub(super) fn draw_pen_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState) {
        let w2s = |x: f64, y: f64| -> Pos2 {
            Pos2::new(
                origin.x + x as f32 * state.zoom,
                origin.y + y as f32 * state.zoom,
            )
        };
        let mut screen_pts = Vec::new();

        for p in &self.pen_state.points {
            let sp = w2s(p.anchor.x, p.anchor.y);
            screen_pts.push(sp);

            // Draw bezier handle lines
            if let Some(ho) = p.handle_out {
                let hsp = w2s(ho.x, ho.y);
                painter.line_segment(
                    [sp, hsp],
                    Stroke::new(1.0_f32, Color32::from_rgb(100, 160, 240)),
                );
                painter.circle_filled(hsp, 4.0_f32, Color32::from_rgb(20, 115, 230));
                painter.circle_stroke(hsp, 4.0_f32, Stroke::new(1.0_f32, Color32::WHITE));
            }
            if let Some(hi) = p.handle_in {
                let hsp = w2s(hi.x, hi.y);
                painter.line_segment(
                    [sp, hsp],
                    Stroke::new(1.0_f32, Color32::from_rgb(100, 160, 240)),
                );
                painter.circle_filled(hsp, 4.0_f32, Color32::from_rgb(20, 115, 230));
                painter.circle_stroke(hsp, 4.0_f32, Stroke::new(1.0_f32, Color32::WHITE));
            }

            // Anchor point square (Illustrator style: white fill, blue border)
            let a_rect = Rect::from_center_size(sp, Vec2::splat(6.0));
            painter.rect_filled(a_rect, 0.0_f32, Color32::WHITE);
            painter.rect_stroke(
                a_rect,
                0.0_f32,
                Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)),
                StrokeKind::Outside,
            );
        }

        // Live Illustrator Rubberband Line from last anchor to mouse hover
        if let Some((hx, hy)) = self.pen_state.hover_pos {
            let hp = w2s(hx, hy);
            if let Some(last_sp) = screen_pts.last() {
                // Dashed line from last anchor to cursor
                painter.line_segment(
                    [*last_sp, hp],
                    Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230)),
                );
            }
            // Mouse cursor crosshair dot
            painter.circle_filled(hp, 3.0_f32, Color32::from_rgb(20, 115, 230));
            painter.circle_stroke(hp, 3.0_f32, Stroke::new(1.0_f32, Color32::WHITE));
        }

        // Draw the path segments as actual bezier curves
        if screen_pts.len() >= 2 {
            // Simple polyline for straight segments; curves handled by segment drawing
            for i in 1..self.pen_state.points.len() {
                let prev = &self.pen_state.points[i - 1];
                let curr = &self.pen_state.points[i];
                let p0 = w2s(prev.anchor.x, prev.anchor.y);
                let p3 = w2s(curr.anchor.x, curr.anchor.y);
                let c0 = prev.handle_out.map(|h| w2s(h.x, h.y)).unwrap_or(p0);
                let c1 = curr.handle_in.map(|h| w2s(h.x, h.y)).unwrap_or(p3);

                if c0 == p0 && c1 == p3 {
                    painter.line_segment(
                        [p0, p3],
                        Stroke::new(1.5_f32, Color32::from_rgb(20, 115, 230)),
                    );
                } else {
                    // Approximate bezier with segments
                    let steps = 32;
                    let mut prev_pt = p0;
                    for step in 1..=steps {
                        let t = step as f32 / steps as f32;
                        let mt = 1.0 - t;
                        let x = mt * mt * mt * p0.x
                            + 3.0 * mt * mt * t * c0.x
                            + 3.0 * mt * t * t * c1.x
                            + t * t * t * p3.x;
                        let y = mt * mt * mt * p0.y
                            + 3.0 * mt * mt * t * c0.y
                            + 3.0 * mt * t * t * c1.y
                            + t * t * t * p3.y;
                        let next_pt = Pos2::new(x, y);
                        painter.line_segment(
                            [prev_pt, next_pt],
                            Stroke::new(1.5_f32, Color32::from_rgb(20, 115, 230)),
                        );
                        prev_pt = next_pt;
                    }
                }
            }
        }
    }

    pub(super) fn draw_diff_overlays(
        &self,
        painter: &egui::Painter,
        diff: &crate::core::diff::SemanticDiff,
        origin: Pos2,
        state: &AppState,
    ) {
        use crate::core::diff::ObjectDiffStatus;
        for obj_diff in &diff.objects {
            if let Some((_, obj)) = state
                .document
                .all_objects()
                .find(|(_, o)| o.id == obj_diff.id)
            {
                let local_path = obj.to_path_data();
                let pts = local_path.to_polygon(16);
                let (min_x, min_y, max_x, max_y) = if pts.is_empty() {
                    (
                        obj.transform.x,
                        obj.transform.y,
                        obj.transform.x + 50.0,
                        obj.transform.y + 50.0,
                    )
                } else {
                    let mut b = (
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    );
                    for p in &pts {
                        let (wx, wy) = obj.transform.transform_point(p.x, p.y);
                        b.0 = b.0.min(wx);
                        b.1 = b.1.min(wy);
                        b.2 = b.2.max(wx);
                        b.3 = b.3.max(wy);
                    }
                    b
                };

                let screen_min = Pos2::new(
                    origin.x + min_x as f32 * state.zoom - 4.0,
                    origin.y + min_y as f32 * state.zoom - 4.0,
                );
                let screen_max = Pos2::new(
                    origin.x + max_x as f32 * state.zoom + 4.0,
                    origin.y + max_y as f32 * state.zoom + 4.0,
                );
                let rect = Rect::from_min_max(screen_min, screen_max);

                let (border_col, fill_col, badge) = match &obj_diff.status {
                    ObjectDiffStatus::Added => (
                        Color32::from_rgb(46, 204, 113),
                        Color32::from_rgba_unmultiplied(46, 204, 113, 35),
                        format!("＋ 追加: #{}", &obj_diff.id[..8.min(obj_diff.id.len())]),
                    ),
                    ObjectDiffStatus::Modified { .. } => (
                        Color32::from_rgb(241, 196, 15),
                        Color32::from_rgba_unmultiplied(241, 196, 15, 35),
                        format!("✎ 変更: #{}", &obj_diff.id[..8.min(obj_diff.id.len())]),
                    ),
                    ObjectDiffStatus::Removed => (
                        Color32::from_rgb(231, 76, 60),
                        Color32::from_rgba_unmultiplied(231, 76, 60, 35),
                        format!("ー 削除: #{}", &obj_diff.id[..8.min(obj_diff.id.len())]),
                    ),
                };

                painter.rect_filled(rect, 2.0_f32, fill_col);
                painter.rect_stroke(
                    rect,
                    2.0_f32,
                    Stroke::new(1.5_f32, border_col),
                    StrokeKind::Outside,
                );

                // Badge text above the bounding box
                let badge_pos = Pos2::new(rect.min.x, rect.min.y - 14.0_f32);
                painter.rect_filled(
                    Rect::from_min_size(badge_pos, Vec2::new(badge.len() as f32 * 6.5 + 8.0, 13.0)),
                    2.0_f32,
                    Color32::from_rgba_unmultiplied(20, 20, 25, 220),
                );
                painter.text(
                    Pos2::new(badge_pos.x + 4.0, badge_pos.y + 1.0),
                    egui::Align2::LEFT_TOP,
                    badge,
                    egui::FontId::proportional(10.0),
                    border_col,
                );
            }
        }
    }
}

fn outer_radius_to_f64(r: f64) -> f64 {
    r
}
