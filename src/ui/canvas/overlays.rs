use super::{CanvasWidget, DragState, HANDLE_HIT_RADIUS, RULER_WIDTH};
use crate::core::document::{Object, ObjectType};
use crate::core::path::PathData;
use crate::core::state::AppState;
use egui::{Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

/// `(fill, stroke)` for points and handles — `point_handle_color_mode`.
/// The default matches what the renderer has always drawn (white fill with an
/// Adobe-blue border); the high-contrast variant is black on yellow.
fn handle_palette(state: &AppState) -> (Color32, Color32) {
    if state.prefs.high_contrast_handles() {
        (Color32::from_rgb(255, 242, 0), Color32::BLACK)
    } else {
        (Color32::WHITE, Color32::from_rgb(20, 115, 230))
    }
}

impl CanvasWidget {
    /// Perspective guide overlay: horizon, fan rays clipped to the view,
    /// and VP markers. No-op unless a grid exists and `show` is set.
    pub(super) fn draw_perspective(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        _origin: Pos2,
        state: &AppState,
    ) {
        let grid = match state.document.perspective.as_ref() {
            Some(g) if g.show => g,
            _ => return,
        };
        // Visible world rect for clipping.
        let w0 = state.screen_to_world(rect.min.x, rect.min.y);
        let w1 = state.screen_to_world(rect.max.x, rect.max.y);
        let view = (w0.0, w0.1, w1.0, w1.1);
        let canvas = (0.0, 0.0, state.document.width, state.document.height);
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = state.world_to_screen(wx, wy);
            Pos2::new(sx, sy)
        };
        let ray_stroke = Stroke::new(0.75_f32, Color32::from_rgba_unmultiplied(90, 160, 220, 110));
        for (a, b) in grid.ray_segments(canvas) {
            if let Some(((x0, y0), (x1, y1))) =
                crate::core::perspective::clip_seg_to_rect(a, b, view)
            {
                painter.line_segment([to_screen(x0, y0), to_screen(x1, y1)], ray_stroke);
            }
        }
        // Horizon across the view.
        let hz = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(120, 200, 255, 160));
        if let Some(((x0, y0), (x1, y1))) = crate::core::perspective::clip_seg_to_rect(
            (view.0 - 10.0, grid.horizon_y),
            (view.2 + 10.0, grid.horizon_y),
            view,
        ) {
            painter.line_segment([to_screen(x0, y0), to_screen(x1, y1)], hz);
        }
        // VP diamonds (may sit outside the view; draw when visible).
        for (vx, vy) in grid.two_point
            .then(|| vec![grid.left_vp, grid.right_vp])
            .unwrap_or_else(|| vec![grid.left_vp])
        {
            let (sx, sy) = state.world_to_screen(vx, vy);
            let p = Pos2::new(sx, sy);
            if rect.contains(p) {
                painter.rect_filled(
                    Rect::from_center_size(p, Vec2::splat(9.0)),
                    1.0,
                    Color32::from_rgb(255, 170, 40),
                );
                painter.rect_stroke(
                    Rect::from_center_size(p, Vec2::splat(9.0)),
                    1.0,
                    Stroke::new(1.0_f32, Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }

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
        let unit = state.prefs.ruler_unit;

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
        // Active unit in the origin box, the way Illustrator labels it.
        painter.text(
            rect.min + Vec2::splat(RULER_WIDTH * 0.5),
            egui::Align2::CENTER_CENTER,
            unit.suffix(),
            FontId::proportional(7.0),
            tick_major_color,
        );

        // Scale steps. The zoom ladder still picks a *rough* spacing in
        // document px; the display unit then snaps it to a readable number
        // of its own (100 px → 20 mm), so every label stays whole.
        let rough: f32 = if state.zoom > 3.0_f32 {
            20.0_f32
        } else if state.zoom > 1.5_f32 {
            50.0_f32
        } else if state.zoom < 0.3_f32 {
            500.0_f32
        } else {
            100.0_f32
        };
        // `nice_step` is the identity for px, so the default unit keeps the
        // exact 20/50/100/500 ladder it has always drawn.
        let step: f32 = unit.nice_step(rough as f64) as f32;
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
                    unit.format(val as f64),
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
                    unit.format(val_y as f64),
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

            let others: Vec<(
                crate::core::path::AnchorPoint,
                crate::core::path::AnchorPoint,
            )> = state
                .document
                .all_objects()
                .filter(|(_, o)| !state.selected_ids.contains(&o.id) && o.visible)
                .filter_map(|(_, o)| o.bounding_box())
                .collect();

            // Alignment lines: selection edges/centers vs other edges/centers.
            for (o_min, o_max) in &others {
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

            // Equal-spacing (distribution) bars: the selection squeezed
            // between two neighbours with matching gaps (Figma-style bars
            // plus numeric gap labels).
            let mut h_left: Vec<(f64, f64)> = Vec::new(); // (gap, neighbour right edge x)
            let mut h_right: Vec<(f64, f64)> = Vec::new(); // (gap, neighbour left edge x)
            let mut v_top: Vec<(f64, f64)> = Vec::new(); // (gap, neighbour bottom edge y)
            let mut v_bottom: Vec<(f64, f64)> = Vec::new(); // (gap, neighbour top edge y)
            for (o_min, o_max) in &others {
                // Horizontal distribution: neighbours must overlap the
                // selection vertically to count as "left" / "right".
                if o_min.y < s_max.y && o_max.y > s_min.y {
                    let gap_l = s_min.x - o_max.x;
                    if gap_l >= -tol {
                        h_left.push((gap_l, o_max.x));
                    }
                    let gap_r = o_min.x - s_max.x;
                    if gap_r >= -tol {
                        h_right.push((gap_r, o_min.x));
                    }
                }
                // Vertical distribution: overlap horizontally.
                if o_min.x < s_max.x && o_max.x > s_min.x {
                    let gap_t = s_min.y - o_max.y;
                    if gap_t >= -tol {
                        v_top.push((gap_t, o_max.y));
                    }
                    let gap_b = o_min.y - s_max.y;
                    if gap_b >= -tol {
                        v_bottom.push((gap_b, o_min.y));
                    }
                }
            }

            let to_screen = |x: f64, y: f64| {
                Pos2::new(
                    origin.x + x as f32 * state.zoom,
                    origin.y + y as f32 * state.zoom,
                )
            };
            let bar_stroke = Stroke::new(2.0_f32, guide_stroke.color);

            if let Some((li, ri, _)) =
                crate::core::smart_guides::best_distribution(&h_left, &h_right, tol)
            {
                let (gap_l, x_l) = h_left[li];
                let (gap_r, x_r) = h_right[ri];
                // Left gap bar: neighbour's right edge → selection's left edge.
                painter.line_segment([to_screen(x_l, s_cy), to_screen(s_min.x, s_cy)], bar_stroke);
                measure_badge(
                    painter,
                    to_screen((x_l + s_min.x) * 0.5, s_cy),
                    &fmt_measure(gap_l, state),
                    guide_stroke.color,
                );
                // Right gap bar: selection's right edge → neighbour's left edge.
                painter.line_segment([to_screen(s_max.x, s_cy), to_screen(x_r, s_cy)], bar_stroke);
                measure_badge(
                    painter,
                    to_screen((s_max.x + x_r) * 0.5, s_cy),
                    &fmt_measure(gap_r, state),
                    guide_stroke.color,
                );
            }

            if let Some((ti, bi, _)) =
                crate::core::smart_guides::best_distribution(&v_top, &v_bottom, tol)
            {
                let (gap_t, y_t) = v_top[ti];
                let (gap_b, y_b) = v_bottom[bi];
                // Top gap bar: neighbour's bottom edge → selection's top edge.
                painter.line_segment([to_screen(s_cx, y_t), to_screen(s_cx, s_min.y)], bar_stroke);
                measure_badge(
                    painter,
                    to_screen(s_cx, (y_t + s_min.y) * 0.5),
                    &fmt_measure(gap_t, state),
                    guide_stroke.color,
                );
                // Bottom gap bar: selection's bottom edge → neighbour's top edge.
                painter.line_segment([to_screen(s_cx, s_max.y), to_screen(s_cx, y_b)], bar_stroke);
                measure_badge(
                    painter,
                    to_screen(s_cx, (s_max.y + y_b) * 0.5),
                    &fmt_measure(gap_b, state),
                    guide_stroke.color,
                );
            }
        }
    }

    /// Per-cell grid over every visible pixel-art object, plus a hover-cell
    /// highlight while a pixel tool is active. Skipped below ~7 screen px
    /// per cell (sub-pixel lines would just shimmer).
    pub(super) fn draw_pixel_grid(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
    ) {
        if !state.pixel_show_grid {
            return;
        }
        let pixel_tool = matches!(
            state.current_tool,
            crate::core::state::Tool::PixelPencil
                | crate::core::state::Tool::PixelEraser
                | crate::core::state::Tool::PixelBucket
        );
        let mut stack: Vec<&Object> = state
            .document
            .layers
            .iter()
            .flat_map(|l| l.objects.iter())
            .collect();
        while let Some(obj) = stack.pop() {
            match &obj.object_type {
                ObjectType::Group(children)
                | ObjectType::ClippingMask { children } => {
                    stack.extend(children.iter());
                }
                ObjectType::PixelArt(p) => {
                    if !obj.visible {
                        continue;
                    }
                    let m = obj.transform.matrix();
                    // Screen length of one local unit (uniform-scale approx).
                    let ux = (m[0] * m[0] + m[1] * m[1]).sqrt() as f32 * state.zoom;
                    if ux < 7.0 {
                        continue;
                    }
                    let to_screen = |lx: f64, ly: f64| -> Pos2 {
                        let sx = m[0] * lx + m[2] * ly + m[4];
                        let sy = m[1] * lx + m[3] * ly + m[5];
                        Pos2::new(
                            origin.x + sx as f32 * state.zoom,
                            origin.y + sy as f32 * state.zoom,
                        )
                    };
                    let thin = Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 26),
                    );
                    for i in 0..=p.width {
                        painter.line_segment(
                            [to_screen(i as f64, 0.0), to_screen(i as f64, p.height as f64)],
                            thin,
                        );
                    }
                    for j in 0..=p.height {
                        painter.line_segment(
                            [to_screen(0.0, j as f64), to_screen(p.width as f64, j as f64)],
                            thin,
                        );
                    }
                    // Hover cell while painting.
                    if pixel_tool {
                        if let Some((wx, wy)) = state.cursor_world {
                            let (lx, ly) = obj.transform.inverse_transform_point(wx, wy);
                            if lx >= 0.0
                                && ly >= 0.0
                                && lx < p.width as f64
                                && ly < p.height as f64
                            {
                                let (cx, cy) = (lx.floor(), ly.floor());
                                let corners = [
                                    to_screen(cx, cy),
                                    to_screen(cx + 1.0, cy),
                                    to_screen(cx + 1.0, cy + 1.0),
                                    to_screen(cx, cy + 1.0),
                                ];
                                painter.add(egui::epaint::PathShape::closed_line(
                                    corners.into_iter().collect(),
                                    Stroke::new(1.5_f32, Color32::from_rgb(120, 200, 255)),
                                ));
                            }
                        }
                    }
                }
                _ => {}
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
        // A hidden bounding box hides the whole widget (Illustrator's
        // *View ▸ Hide Bounding Box*): `hit_test_handles` bails out on the
        // same flag, so no invisible handle stays clickable.
        if !state.prefs.show_bounding_box {
            return;
        }
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

            let (point_fill, point_stroke) = handle_palette(state);
            let sel_stroke =
                Stroke::new(state.prefs.selection_line_width, Color32::from_rgb(20, 115, 230));
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

            // 8 handles (size + palette come from the preferences)
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
                let h_rect = Rect::from_center_size(p, Vec2::splat(state.prefs.handle_size));
                painter.rect_filled(h_rect, 0.0_f32, point_fill);
                painter.rect_stroke(
                    h_rect,
                    0.0_f32,
                    Stroke::new(1.0_f32, point_stroke),
                    StrokeKind::Outside,
                );
            }

            // Top Rotation handle circle above right_top
            let rot_p = Pos2::new(rect.right_top().x + 12.0_f32, rect.right_top().y - 12.0_f32);
            painter.circle_filled(rot_p, 4.0_f32, point_stroke);
            painter.circle_stroke(rot_p, 4.0_f32, Stroke::new(1.0_f32, point_fill));

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

            // Live corner-radius widgets (Rectangle only): circular handles
            // inset from each corner so a sharp (r = 0) corner stays grabbable
            // without colliding with the square resize handles.
            if let ObjectType::Rectangle {
                width,
                height,
                corner_radius,
            } = &obj.object_type
            {
                if width.abs() >= 1.0 && height.abs() >= 1.0 {
                    let min_inset = f64::from(HANDLE_HIT_RADIUS)
                        / f64::from(state.zoom).max(1e-6);
                    let max_r = width.abs().min(height.abs()) * 0.5;
                    let inset = corner_radius.clamp(min_inset, max_r);
                    let local_corners = [
                        (inset, inset),
                        (width - inset, inset),
                        (width - inset, height - inset),
                        (inset, height - inset),
                    ];
                    for (lx, ly) in local_corners {
                        let (wx, wy) = obj.transform.transform_point(lx, ly);
                        let sp = Pos2::new(
                            origin.x + wx as f32 * state.zoom,
                            origin.y + wy as f32 * state.zoom,
                        );
                        painter.circle_filled(sp, 5.0_f32, Color32::from_rgb(255, 150, 0));
                        painter.circle_stroke(sp, 5.0_f32, Stroke::new(1.5_f32, Color32::WHITE));
                    }
                }
            }
        }
    }

    pub(super) fn draw_node_edit(&self, painter: &egui::Painter, origin: Pos2, state: &AppState) {
        let (point_fill, point_stroke) = handle_palette(state);
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
                        Stroke::new(
                            state.prefs.selection_line_width,
                            Color32::from_rgb(20, 115, 230),
                        ),
                    ));
                }

                // The anchors themselves (with their handles and badge) are a
                // display preference; `hit_test_nodes` bails out on the same
                // flag so nothing invisible stays clickable.
                if !state.prefs.show_anchor_points {
                    continue;
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

                    let anchor_rect =
                        Rect::from_center_size(sp, Vec2::splat(state.prefs.anchor_point_size));
                    if is_active {
                        // Selected anchor: solid square in the palette's stroke
                        // colour, outlined with its fill colour.
                        painter.rect_filled(anchor_rect, 0.0_f32, point_stroke);
                        painter.rect_stroke(
                            anchor_rect,
                            0.0_f32,
                            Stroke::new(1.0_f32, point_fill),
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
                                point_stroke
                            };
                            painter.circle_filled(c2_sp, 3.5_f32, c2_col);
                            painter.circle_stroke(
                                c2_sp,
                                3.5_f32,
                                Stroke::new(1.0_f32, point_fill),
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
                                point_stroke
                            };
                            painter.circle_filled(c1_sp, 3.5_f32, c1_col);
                            painter.circle_stroke(
                                c1_sp,
                                3.5_f32,
                                Stroke::new(1.0_f32, point_fill),
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
                        // Unselected anchor: hollow square in the palette
                        painter.rect_filled(anchor_rect, 0.0_f32, point_fill);
                        painter.rect_stroke(
                            anchor_rect,
                            0.0_f32,
                            Stroke::new(1.2_f32, point_stroke),
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

    /// Figma-style measurement overlay (Inspect): while Alt is held, show
    /// W/H size badges for the selection and distance lines from its bbox
    /// edges to the nearest overlapping object / artboard edges.
    pub(super) fn draw_measurements(
        &self,
        painter: &egui::Painter,
        _rect: Rect,
        origin: Pos2,
        state: &AppState,
        alt_down: bool,
    ) {
        if !alt_down || state.selected_ids.is_empty() {
            return;
        }
        let mut s_min: Option<(f64, f64)> = None;
        let mut s_max: Option<(f64, f64)> = None;
        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                if let Some((mn, mx)) = obj.bounding_box() {
                    s_min = Some(match s_min {
                        None => (mn.x, mn.y),
                        Some(c) => (c.0.min(mn.x), c.1.min(mn.y)),
                    });
                    s_max = Some(match s_max {
                        None => (mx.x, mx.y),
                        Some(c) => (c.0.max(mx.x), c.1.max(mx.y)),
                    });
                }
            }
        }
        let (Some(s_min), Some(s_max)) = (s_min, s_max) else {
            return;
        };
        let w2s = |wx: f64, wy: f64| -> Pos2 {
            Pos2::new(origin.x + wx as f32 * state.zoom, origin.y + wy as f32 * state.zoom)
        };
        let mcol = Color32::from_rgb(242, 72, 34);
        let stroke = Stroke::new(1.0_f32, mcol);

        // Size badges: W under the bbox, H to its right.
        let w = s_max.0 - s_min.0;
        let h = s_max.1 - s_min.1;
        let cb = w2s((s_min.0 + s_max.0) * 0.5, s_max.1);
        measure_badge(
            painter,
            Pos2::new(cb.x, cb.y + 10.0),
            &fmt_measure(w, state),
            mcol,
        );
        let cr = w2s(s_max.0, (s_min.1 + s_max.1) * 0.5);
        measure_badge(
            painter,
            Pos2::new(cr.x + 10.0, cr.y),
            &fmt_measure(h, state),
            mcol,
        );

        // Candidate targets: other visible objects + artboard rects.
        let mut cands: Vec<((f64, f64), (f64, f64))> = Vec::new();
        for (_, other) in state.document.all_objects() {
            if state.selected_ids.contains(&other.id) || !other.visible {
                continue;
            }
            if let Some((mn, mx)) = other.bounding_box() {
                cands.push(((mn.x, mn.y), (mx.x, mx.y)));
            }
        }
        for ab in state.document.effective_artboards().iter() {
            cands.push(((ab.x, ab.y), (ab.x + ab.width, ab.y + ab.height)));
        }

        // Nearest gap on each side where the perpendicular ranges overlap.
        let mut left: Option<(f64, f64, f64)> = None;
        let mut right: Option<(f64, f64, f64)> = None;
        let mut top: Option<(f64, f64, f64)> = None;
        let mut bottom: Option<(f64, f64, f64)> = None;
        for &(cmin, cmax) in &cands {
            let oy0 = s_min.1.max(cmin.1);
            let oy1 = s_max.1.min(cmax.1);
            if oy1 > oy0 + 0.01 {
                if cmax.0 <= s_min.0 {
                    let gap = s_min.0 - cmax.0;
                    if gap > 0.5 && left.map_or(true, |(g, _, _)| gap < g) {
                        left = Some((gap, oy0, oy1));
                    }
                }
                if cmin.0 >= s_max.0 {
                    let gap = cmin.0 - s_max.0;
                    if gap > 0.5 && right.map_or(true, |(g, _, _)| gap < g) {
                        right = Some((gap, oy0, oy1));
                    }
                }
            }
            let ox0 = s_min.0.max(cmin.0);
            let ox1 = s_max.0.min(cmax.0);
            if ox1 > ox0 + 0.01 {
                if cmax.1 <= s_min.1 {
                    let gap = s_min.1 - cmax.1;
                    if gap > 0.5 && top.map_or(true, |(g, _, _)| gap < g) {
                        top = Some((gap, ox0, ox1));
                    }
                }
                if cmin.1 >= s_max.1 {
                    let gap = cmin.1 - s_max.1;
                    if gap > 0.5 && bottom.map_or(true, |(g, _, _)| gap < g) {
                        bottom = Some((gap, ox0, ox1));
                    }
                }
            }
        }

        // Horizontal (left/right) dimension lines at the y-overlap midpoint.
        for (side, info) in [
            ("left", left),
            ("right", right),
        ] {
            let Some((gap, y0, y1)) = info else { continue };
            let my = (y0 + y1) * 0.5;
            let (x0, x1) = if side == "left" {
                (s_min.0 - gap, s_min.0)
            } else {
                (s_max.0, s_max.0 + gap)
            };
            let a = w2s(x0, my);
            let b = w2s(x1, my);
            painter.line_segment([a, b], stroke);
            for p in [a, b] {
                painter.line_segment([Pos2::new(p.x, p.y - 3.5), Pos2::new(p.x, p.y + 3.5)], stroke);
            }
            measure_badge(
                painter,
                Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5 - 9.0),
                &fmt_measure(gap, state),
                mcol,
            );
        }

        // Vertical (top/bottom) dimension lines at the x-overlap midpoint.
        for (side, info) in [("top", top), ("bottom", bottom)] {
            let Some((gap, x0, x1)) = info else { continue };
            let mx = (x0 + x1) * 0.5;
            let (y0w, y1w) = if side == "top" {
                (s_min.1 - gap, s_min.1)
            } else {
                (s_max.1, s_max.1 + gap)
            };
            let a = w2s(mx, y0w);
            let b = w2s(mx, y1w);
            painter.line_segment([a, b], stroke);
            for p in [a, b] {
                painter.line_segment([Pos2::new(p.x - 3.5, p.y), Pos2::new(p.x + 3.5, p.y)], stroke);
            }
            measure_badge(
                painter,
                Pos2::new((a.x + b.x) * 0.5 + 9.0, (a.y + b.y) * 0.5),
                &fmt_measure(gap, state),
                mcol,
            );
        }
    }

    /// Figma-style layout grid overlay on artboards that own one.
    /// Drawn after content so it reads as an editing aid, not artwork.
    pub(super) fn draw_layout_grid(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        state: &AppState,
    ) {
        use crate::core::layout_grid::LayoutGridKind;
        for ab in state.document.effective_artboards().iter() {
            let Some(grid) = ab.layout_grid.as_ref().filter(|g| g.show) else {
                continue;
            };
            let to_screen = |wx: f64, wy: f64| -> Pos2 {
                Pos2::new(origin.x + wx as f32 * state.zoom, origin.y + wy as f32 * state.zoom)
            };
            let alpha = (grid.opacity * 255.0) as u8;
            match grid.kind {
                LayoutGridKind::Columns | LayoutGridKind::Rows => {
                    let along = if grid.kind == LayoutGridKind::Columns {
                        ab.width
                    } else {
                        ab.height
                    };
                    let col = if grid.kind == LayoutGridKind::Columns {
                        Color32::from_rgba_unmultiplied(242, 72, 34, alpha)
                    } else {
                        Color32::from_rgba_unmultiplied(160, 48, 240, alpha)
                    };
                    for (t0, t1) in grid.tracks(along) {
                        let r = if grid.kind == LayoutGridKind::Columns {
                            Rect::from_min_max(
                                to_screen(ab.x + t0, ab.y),
                                to_screen(ab.x + t1, ab.y + ab.height),
                            )
                        } else {
                            Rect::from_min_max(
                                to_screen(ab.x, ab.y + t0),
                                to_screen(ab.x + ab.width, ab.y + t1),
                            )
                        };
                        painter.rect_filled(r, 0.0_f32, col);
                    }
                }
                LayoutGridKind::Grid => {
                    let cell = grid.size as f32 * state.zoom;
                    if cell < 4.0_f32 {
                        continue;
                    }
                    let rect = Rect::from_min_max(
                        to_screen(ab.x, ab.y),
                        to_screen(ab.x + ab.width, ab.y + ab.height),
                    );
                    let line = Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(242, 72, 34, alpha.max(80)),
                    );
                    let mut x = rect.min.x;
                    while x <= rect.max.x {
                        painter.line_segment(
                            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                            line,
                        );
                        x += cell;
                    }
                    let mut y = rect.min.y;
                    while y <= rect.max.y {
                        painter.line_segment(
                            [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                            line,
                        );
                        y += cell;
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

/// Format a document-px length for an on-canvas badge, in the display unit
/// the ruler is using (they must agree — a gap bar next to a mm ruler
/// reading px would be nonsense).
fn fmt_measure(v: f64, state: &AppState) -> String {
    state.prefs.ruler_unit.format(v)
}

fn measure_badge(painter: &egui::Painter, center: Pos2, text: &str, color: Color32) {
    let bw = text.chars().count() as f32 * 6.5 + 10.0;
    let r = Rect::from_center_size(center, Vec2::new(bw, 15.0));
    painter.rect_filled(r, 2.0, color);
    painter.text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(10.0),
        Color32::WHITE,
    );
}
