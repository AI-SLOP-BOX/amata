use crate::core::document::{ObjectType, Object};
use crate::core::path::{AnchorPoint, FillStyle, PathData, PathElement, StrokeStyle};
use crate::core::state::{AppState, HandleCorner, Tool};
use crate::tools::pen::PenState;
use crate::tools::select::SelectState;
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragMode {
    Select,
    CreateRect,
    CreateEllipse,
    CreateStar,
    CreatePolygon,
    CreateLine,
    PencilDraw,
    MoveObject,
    Resize(HandleCorner),
    Rotate,
    Pan,
    MoveNode(usize),
}

struct DragState {
    mode: DragMode,
    start_world: (f64, f64),
    current_world: (f64, f64),
    pencil_points: Vec<AnchorPoint>,
}

impl DragState {
    fn new(mode: DragMode, wx: f64, wy: f64) -> Self {
        Self {
            mode,
            start_world: (wx, wy),
            current_world: (wx, wy),
            pencil_points: vec![AnchorPoint::new(wx, wy)],
        }
    }
}

const HANDLE_SIZE: f32 = 8.0_f32;
const HANDLE_HIT_RADIUS: f32 = 12.0_f32;
const RULER_WIDTH: f32 = 20.0_f32;

pub struct CanvasWidget {
    pub pen_state: PenState,
    pub select_state: SelectState,
    drag: Option<DragState>,
    node_edit_state: NodeEditState,
}

struct NodeEditState {
    selected_anchor_idx: Option<usize>,
    selected_object_id: Option<String>,
}

impl NodeEditState {
    fn new() -> Self {
        Self {
            selected_anchor_idx: None,
            selected_object_id: None,
        }
    }
}

impl Default for CanvasWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasWidget {
    pub fn new() -> Self {
        Self {
            pen_state: PenState::new(),
            select_state: SelectState::new(),
            drag: None,
            node_edit_state: NodeEditState::new(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut AppState) {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), ui.available_height()),
            Sense::click_and_drag(),
        );

        let rect = response.rect;
        let origin = Pos2::new(
            rect.center().x + state.pan_x,
            rect.center().y + state.pan_y,
        );

        state.canvas_width = rect.width();
        state.canvas_height = rect.height();

        // Dark Canvas Background
        painter.rect_filled(rect, 0.0_f32, Color32::from_rgb(32, 33, 36));

        // Grid
        if state.show_grid {
            self.draw_grid(&painter, rect, origin, state);
        }

        // Artboard White Paper
        let artboard = Rect::from_min_size(
            origin,
            Vec2::new(
                state.document.width as f32 * state.zoom,
                state.document.height as f32 * state.zoom,
            ),
        );
        painter.rect_filled(artboard, 0.0_f32, Color32::WHITE);
        painter.rect_stroke(artboard, 0.0_f32, Stroke::new(1.5_f32, Color32::from_rgb(180, 180, 180)), StrokeKind::Inside);

        // Render Objects
        for (_, obj) in state.document.all_objects() {
            if !obj.visible {
                continue;
            }
            self.draw_object(&painter, obj, origin, state);
        }

        // Smart Guides
        if state.show_smart_guides && self.drag.is_some() {
            self.draw_smart_guides(&painter, artboard, rect);
        }

        // Drag Previews
        if let Some(ref drag) = self.drag {
            match drag.mode {
                DragMode::CreateRect => self.draw_rect_preview(&painter, origin, state, drag),
                DragMode::CreateEllipse => self.draw_ellipse_preview(&painter, origin, state, drag),
                DragMode::CreateStar => self.draw_star_preview(&painter, origin, state, drag),
                DragMode::CreatePolygon => self.draw_polygon_preview(&painter, origin, state, drag),
                DragMode::CreateLine => self.draw_line_preview(&painter, origin, state, drag),
                DragMode::PencilDraw => self.draw_pencil_preview(&painter, origin, state, drag),
                DragMode::Select => self.draw_marquee(&painter, origin, state, drag),
                _ => {}
            }
        }

        // Pen Preview
        if state.current_tool == Tool::Pen && self.pen_state.is_drawing {
            self.draw_pen_preview(&painter, origin, state);
        }

        // Selection Bounding Box & Handles
        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                self.draw_selection(&painter, obj, origin, state);
            }
        }

        // Node Editing Handles
        if state.current_tool == Tool::Node {
            self.draw_node_edit(&painter, origin, state);
        }

        // Rulers
        if state.show_rulers {
            self.draw_rulers(&painter, rect, origin, state);
        }

        // Mouse Wheel Zoom
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0_f32 {
            let old_zoom = state.zoom;
            state.zoom = (state.zoom * (1.0_f32 + scroll * 0.001_f32)).clamp(0.01_f32, 100.0_f32);
            if let Some(screen_pos) = response.interact_pointer_pos() {
                let zoom_ratio = state.zoom / old_zoom;
                let dx = (screen_pos.x - origin.x) * (zoom_ratio - 1.0_f32);
                let dy = (screen_pos.y - origin.y) * (zoom_ratio - 1.0_f32);
                state.pan_x -= dx;
                state.pan_y -= dy;
            }
        }

        // Secondary / Middle Click Pan
        if response.secondary_clicked() && !response.dragged() {
            self.drag = Some(DragState::new(DragMode::Pan, 0.0, 0.0));
        }

        // Handle Pointer Interaction
        if let Some(screen_pos) = response.interact_pointer_pos() {
            let (wx, wy) = state.screen_to_world(screen_pos.x, screen_pos.y);

            let mut cursor = egui::CursorIcon::Default;
            if state.current_tool == Tool::Select {
                for id in &state.selected_ids {
                    if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                        if let Some(corner) = self.hit_test_handles(obj, screen_pos, origin, state) {
                            cursor = match corner {
                                HandleCorner::TopLeft | HandleCorner::BottomRight => egui::CursorIcon::ResizeNeSw,
                                HandleCorner::TopRight | HandleCorner::BottomLeft => egui::CursorIcon::ResizeNwSe,
                                HandleCorner::Top | HandleCorner::Bottom => egui::CursorIcon::ResizeVertical,
                                HandleCorner::Left | HandleCorner::Right => egui::CursorIcon::ResizeHorizontal,
                                HandleCorner::RotateTopRight => egui::CursorIcon::Crosshair,
                            };
                            break;
                        }
                    }
                }
                if cursor == egui::CursorIcon::Default
                    && self.select_state.hit_test(state, wx, wy).is_some() {
                        cursor = egui::CursorIcon::Move;
                    }
            } else if state.current_tool == Tool::Hand {
                cursor = egui::CursorIcon::Grab;
            } else if state.current_tool == Tool::Eyedropper {
                cursor = egui::CursorIcon::Crosshair;
            } else if state.current_tool == Tool::Text {
                cursor = egui::CursorIcon::Text;
            } else if state.current_tool == Tool::Node {
                if self.hit_test_nodes(state, screen_pos, origin).is_some() {
                    cursor = egui::CursorIcon::PointingHand;
                }
            } else {
                cursor = egui::CursorIcon::Crosshair;
            }
            painter.ctx().set_cursor_icon(cursor);

            if state.current_tool == Tool::Pen && self.pen_state.is_drawing {
                self.pen_state.update_hover(wx, wy);
            }

            if response.clicked() {
                let shift = painter.ctx().input(|i| i.modifiers.shift);
                self.handle_click(state, wx, wy, screen_pos, origin, shift);
            }
        }

        // Drag Start
        if response.drag_started() && self.drag.is_none() {
            if let Some(screen_pos) = response.interact_pointer_pos() {
                let (wx, wy) = state.screen_to_world(screen_pos.x, screen_pos.y);
                let (wx, wy) = state.snap(wx, wy);

                match state.current_tool {
                    Tool::Rectangle => {
                        self.drag = Some(DragState::new(DragMode::CreateRect, wx, wy));
                    }
                    Tool::Ellipse => {
                        self.drag = Some(DragState::new(DragMode::CreateEllipse, wx, wy));
                    }
                    Tool::Star => {
                        self.drag = Some(DragState::new(DragMode::CreateStar, wx, wy));
                    }
                    Tool::Polygon => {
                        self.drag = Some(DragState::new(DragMode::CreatePolygon, wx, wy));
                    }
                    Tool::Line => {
                        self.drag = Some(DragState::new(DragMode::CreateLine, wx, wy));
                    }
                    Tool::Pencil => {
                        self.drag = Some(DragState::new(DragMode::PencilDraw, wx, wy));
                    }
                    Tool::Select => {
                        let mut handled = false;
                        for id in &state.selected_ids {
                            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                                if let Some(corner) = self.hit_test_handles(obj, screen_pos, origin, state) {
                                    if corner == HandleCorner::RotateTopRight {
                                        self.drag = Some(DragState::new(DragMode::Rotate, wx, wy));
                                    } else {
                                        self.drag = Some(DragState::new(DragMode::Resize(corner), wx, wy));
                                    }
                                    handled = true;
                                    break;
                                }
                            }
                        }
                        if !handled {
                            if let Some(id) = self.select_state.hit_test(state, wx, wy) {
                                if !state.selected_ids.contains(&id) {
                                    state.selected_ids = vec![id];
                                }
                                self.select_state.start_drag(state, wx, wy);
                                self.drag = Some(DragState::new(DragMode::MoveObject, wx, wy));
                            } else {
                                state.selected_ids.clear();
                                self.drag = Some(DragState::new(DragMode::Select, wx, wy));
                            }
                        }
                    }
                    Tool::Node => {
                        if let Some((anchor_idx, obj_id)) = self.hit_test_nodes(state, screen_pos, origin) {
                            self.node_edit_state.selected_anchor_idx = Some(anchor_idx);
                            self.node_edit_state.selected_object_id = Some(obj_id);
                            self.drag = Some(DragState::new(DragMode::MoveNode(anchor_idx), wx, wy));
                        }
                    }
                    Tool::Hand => {
                        self.drag = Some(DragState::new(DragMode::Pan, 0.0, 0.0));
                    }
                    _ => {}
                }
            }
        }

        // Dragging
        if response.dragged() {
            if let Some(ref mut drag) = self.drag {
                let delta = response.drag_delta();
                if drag.mode == DragMode::Pan {
                    state.pan_x += delta.x;
                    state.pan_y += delta.y;
                } else if let Some(screen_pos) = response.interact_pointer_pos() {
                    let (wx, wy) = state.screen_to_world(screen_pos.x, screen_pos.y);
                    let (wx, wy) = state.snap(wx, wy);
                    drag.current_world = (wx, wy);

                    if drag.mode == DragMode::PencilDraw {
                        drag.pencil_points.push(AnchorPoint::new(wx, wy));
                    } else if drag.mode == DragMode::MoveObject {
                        self.select_state.update_drag(state, wx, wy);
                    } else if drag.mode == DragMode::Rotate {
                        self.update_rotate(state, wx, wy);
                    } else if let DragMode::MoveNode(idx) = drag.mode {
                        self.move_node(state, idx, wx, wy);
                    }
                }
            }
        }

        // Drag Stopped / Completed
        if response.drag_stopped() {
            if let Some(drag) = self.drag.take() {
                match drag.mode {
                    DragMode::CreateRect => {
                        let w = drag.current_world.0 - drag.start_world.0;
                        let h = drag.current_world.1 - drag.start_world.1;
                        if w.abs() > 4.0 && h.abs() > 4.0 {
                            let x = drag.start_world.0.min(drag.current_world.0);
                            let y = drag.start_world.1.min(drag.current_world.1);
                            let mut obj = Object::new_rect("Rectangle", x, y, w.abs(), h.abs(), state.corner_radius);
                            obj.fill = Some(FillStyle::solid(state.fill_color));
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
                            });
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                    DragMode::CreateEllipse => {
                        let rx = ((drag.current_world.0 - drag.start_world.0) / 2.0).abs();
                        let ry = ((drag.current_world.1 - drag.start_world.1) / 2.0).abs();
                        if rx > 2.0 && ry > 2.0 {
                            let cx = (drag.start_world.0 + drag.current_world.0) / 2.0;
                            let cy = (drag.start_world.1 + drag.current_world.1) / 2.0;
                            let mut obj = Object::new_ellipse("Ellipse", cx, cy, rx, ry);
                            obj.fill = Some(FillStyle::solid(state.fill_color));
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
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
                            let mut obj = Object::new_star("Star", drag.start_world.0, drag.start_world.1, state.star_points, inner_radius, outer_radius);
                            obj.fill = Some(FillStyle::solid(state.fill_color));
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
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
                            let mut obj = Object::new_polygon("Polygon", drag.start_world.0, drag.start_world.1, state.polygon_sides, radius);
                            obj.fill = Some(FillStyle::solid(state.fill_color));
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
                            });
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
                        }
                    }
                    DragMode::CreateLine => {
                        let dx = drag.current_world.0 - drag.start_world.0;
                        let dy = drag.current_world.1 - drag.start_world.1;
                        if (dx * dx + dy * dy).sqrt() > 2.0 {
                            let mut obj = Object::new_line("Line", drag.start_world.0, drag.start_world.1, drag.current_world.0, drag.current_world.1);
                            obj.stroke = Some(StrokeStyle {
                                color: state.stroke_color,
                                width: state.stroke_width,
                                dash_pattern: None,
                            });
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
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
                            });
                            let obj = Object::new_path("Pencil Stroke", path);
                            let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                            state.undo_manager.execute(cmd, &mut state.document);
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
                                if bb_max.x >= min_x && bb_min.x <= max_x && bb_max.y >= min_y && bb_min.y <= max_y {
                                    state.selected_ids.push(obj.id.clone());
                                }
                            }
                        }
                    }
                    DragMode::MoveObject => {
                        self.select_state.end_drag(state);
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_click(
        &mut self,
        state: &mut AppState,
        wx: f64,
        wy: f64,
        _screen_pos: Pos2,
        _origin: Pos2,
        shift: bool,
    ) {
        match state.current_tool {
            Tool::Pen => {
                if !self.pen_state.is_drawing {
                    self.pen_state.start_path(wx, wy);
                } else {
                    self.pen_state.add_point(wx, wy);
                }
            }
            Tool::Text => {
                let mut obj = Object::new_text("Text", &state.text_input_buf, wx, wy, state.font_size);
                obj.fill = Some(FillStyle::solid(state.fill_color));
                let cmd = Box::new(crate::core::history::AddObjectCommand::new(obj));
                state.undo_manager.execute(cmd, &mut state.document);
            }
            Tool::Eyedropper => {
                if let Some(id) = self.select_state.hit_test(state, wx, wy) {
                    if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| o.id == id) {
                        if let Some(ref fill) = obj.fill {
                            state.fill_color = fill.color;
                        }
                        if let Some(ref stroke) = obj.stroke {
                            state.stroke_color = stroke.color;
                            state.stroke_width = stroke.width;
                        }
                    }
                }
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
            _ => {}
        }
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect, origin: Pos2, state: &AppState) {
        let grid_px = state.grid_size as f32 * state.zoom;
        if grid_px < 5.0_f32 {
            return;
        }

        let start_x = ((rect.min.x - origin.x) / grid_px).floor() * grid_px + origin.x;
        let start_y = ((rect.min.y - origin.y) / grid_px).floor() * grid_px + origin.y;

        let grid_stroke = Stroke::new(0.5_f32, Color32::from_rgb(50, 52, 58));

        let mut x = start_x;
        while x <= rect.max.x {
            painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], grid_stroke);
            x += grid_px;
        }

        let mut y = start_y;
        while y <= rect.max.y {
            painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], grid_stroke);
            y += grid_px;
        }
    }

    fn draw_rulers(&self, painter: &egui::Painter, rect: Rect, origin: Pos2, state: &AppState) {
        let ruler_color = Color32::from_rgb(45, 46, 50);
        let tick_color = Color32::from_rgb(160, 160, 160);
        let font_id = FontId::proportional(9.0_f32);

        // Top ruler bar
        let top_ruler = Rect::from_min_size(rect.min, Vec2::new(rect.width(), RULER_WIDTH));
        painter.rect_filled(top_ruler, 0.0_f32, ruler_color);

        // Left ruler bar
        let left_ruler = Rect::from_min_size(rect.min, Vec2::new(RULER_WIDTH, rect.height()));
        painter.rect_filled(left_ruler, 0.0_f32, ruler_color);

        // Top-left corner box
        painter.rect_filled(Rect::from_min_size(rect.min, Vec2::splat(RULER_WIDTH)), 0.0_f32, Color32::from_rgb(38, 39, 42));

        // Horizontal ticks
        let step: f32 = if state.zoom > 2.0_f32 { 50.0_f32 } else if state.zoom < 0.3_f32 { 500.0_f32 } else { 100.0_f32 };
        let step_px = step * state.zoom;
        let start_val = ((rect.min.x - origin.x) / step_px).floor() * step;

        let mut val = start_val;
        while val * state.zoom + origin.x <= rect.max.x {
            let sx = origin.x + (val * state.zoom);
            if sx >= rect.min.x + RULER_WIDTH {
                painter.line_segment([Pos2::new(sx, rect.min.y + 12.0_f32), Pos2::new(sx, rect.min.y + RULER_WIDTH)], Stroke::new(1.0_f32, tick_color));
                painter.text(Pos2::new(sx + 2.0_f32, rect.min.y + 2.0_f32), egui::Align2::LEFT_TOP, format!("{val:.0}"), font_id.clone(), tick_color);
            }
            val += step;
        }

        // Vertical ticks
        let mut val_y = ((rect.min.y - origin.y) / step_px).floor() * step;
        while val_y * state.zoom + origin.y <= rect.max.y {
            let sy = origin.y + (val_y * state.zoom);
            if sy >= rect.min.y + RULER_WIDTH {
                painter.line_segment([Pos2::new(rect.min.x + 12.0_f32, sy), Pos2::new(rect.min.x + RULER_WIDTH, sy)], Stroke::new(1.0_f32, tick_color));
                painter.text(Pos2::new(rect.min.x + 2.0_f32, sy + 2.0_f32), egui::Align2::LEFT_TOP, format!("{val_y:.0}"), font_id.clone(), tick_color);
            }
            val_y += step;
        }
    }

    fn draw_smart_guides(&self, painter: &egui::Painter, artboard: Rect, canvas_rect: Rect) {
        let guide_stroke = Stroke::new(1.0_f32, Color32::from_rgb(255, 0, 128));
        let center_x = artboard.center().x;
        let center_y = artboard.center().y;

        painter.line_segment([Pos2::new(center_x, canvas_rect.min.y), Pos2::new(center_x, canvas_rect.max.y)], guide_stroke);
        painter.line_segment([Pos2::new(canvas_rect.min.x, center_y), Pos2::new(canvas_rect.max.x, center_y)], guide_stroke);
    }

    fn draw_object(&self, painter: &egui::Painter, obj: &Object, origin: Pos2, state: &AppState) {
        let opacity = obj.opacity.clamp(0.0_f32, 1.0_f32);
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = obj.transform.transform_point(wx, wy);
            Pos2::new(origin.x + sx as f32 * state.zoom, origin.y + sy as f32 * state.zoom)
        };

        let fill_color = obj.fill.as_ref().map(|f| {
            let c = f.color;
            Color32::from_rgba_unmultiplied(
                (c[0] * 255.0_f32) as u8,
                (c[1] * 255.0_f32) as u8,
                (c[2] * 255.0_f32) as u8,
                ((c[3] * opacity) * 255.0_f32) as u8,
            )
        });

        let stroke_info = obj.stroke.as_ref().map(|s| {
            let c = s.color;
            let stroke_c = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0_f32) as u8,
                (c[1] * 255.0_f32) as u8,
                (c[2] * 255.0_f32) as u8,
                ((c[3] * opacity) * 255.0_f32) as u8,
            );
            Stroke::new((s.width as f32 * state.zoom).max(1.0_f32), stroke_c)
        });

        if let Some(ref sh) = obj.shadow {
            let sh_c = Color32::from_rgba_unmultiplied(
                (sh.color[0] * 255.0_f32) as u8,
                (sh.color[1] * 255.0_f32) as u8,
                (sh.color[2] * 255.0_f32) as u8,
                ((sh.color[3] * sh.opacity * opacity) * 255.0_f32) as u8,
            );
            let sh_to_screen = |lx: f64, ly: f64| -> Pos2 {
                let (wx, wy) = obj.transform.transform_point(lx + sh.offset_x, ly + sh.offset_y);
                Pos2::new(
                    origin.x + (wx as f32 * state.zoom),
                    origin.y + (wy as f32 * state.zoom),
                )
            };
            let poly = obj.to_path_data().to_polygon(16);
            if poly.len() >= 3 {
                let sh_pts: Vec<Pos2> = poly.iter().map(|p| sh_to_screen(p.x, p.y)).collect();
                painter.add(egui::epaint::PathShape::convex_polygon(sh_pts, sh_c, Stroke::NONE));
            }
        }

        match &obj.object_type {
            ObjectType::Path(path) => {
                let poly = path.to_polygon(16);
                if poly.len() >= 3 {
                    let screen_pts: Vec<Pos2> = poly.iter().map(|p| to_screen(p.x, p.y)).collect();
                    if let Some(fill) = fill_color {
                        painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                    }
                    if let Some(stroke) = stroke_info {
                        painter.add(egui::epaint::PathShape::line(screen_pts, stroke));
                    }
                }
            }
            ObjectType::Rectangle { width, height, corner_radius } => {
                let path = PathData::from_rect(0.0, 0.0, *width, *height, *corner_radius);
                let screen_pts: Vec<Pos2> = path.to_polygon(12).iter().map(|p| to_screen(p.x, p.y)).collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Ellipse { rx, ry } => {
                let path = PathData::from_ellipse(0.0, 0.0, *rx, *ry);
                let screen_pts: Vec<Pos2> = path.to_polygon(24).iter().map(|p| to_screen(p.x, p.y)).collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Star { points, inner_radius, outer_radius } => {
                let path = PathData::from_star(*points, *inner_radius, *outer_radius, 0.0, 0.0);
                let screen_pts: Vec<Pos2> = path.to_polygon(1).iter().map(|p| to_screen(p.x, p.y)).collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Polygon { sides, radius } => {
                let path = PathData::from_polygon(*sides, *radius, 0.0, 0.0);
                let screen_pts: Vec<Pos2> = path.to_polygon(1).iter().map(|p| to_screen(p.x, p.y)).collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(screen_pts.clone(), fill, Stroke::NONE));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Line { x2, y2 } => {
                let p1 = to_screen(0.0, 0.0);
                let p2 = to_screen(*x2, *y2);
                let stroke = stroke_info.unwrap_or_else(|| Stroke::new(2.0_f32, Color32::BLACK));
                painter.line_segment([p1, p2], stroke);
            }
            ObjectType::Text { text, font_size } => {
                let pos = to_screen(0.0, 0.0);
                let font_id = FontId::proportional((font_size * state.zoom as f64) as f32);
                let color = fill_color.unwrap_or(Color32::BLACK);
                painter.text(pos, egui::Align2::LEFT_BOTTOM, text, font_id, color);
            }
            ObjectType::Group(children) => {
                for child in children {
                    self.draw_object(painter, child, origin, state);
                }
            }
        }
    }

    fn draw_selection(&self, painter: &egui::Painter, obj: &Object, origin: Pos2, state: &AppState) {
        if let Some((bb_min, bb_max)) = obj.bounding_box() {
            let min_p = Pos2::new(origin.x + bb_min.x as f32 * state.zoom, origin.y + bb_min.y as f32 * state.zoom);
            let max_p = Pos2::new(origin.x + bb_max.x as f32 * state.zoom, origin.y + bb_max.y as f32 * state.zoom);
            let rect = Rect::from_min_max(min_p, max_p);

            let sel_stroke = Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255));
            painter.rect_stroke(rect, 0.0_f32, sel_stroke, StrokeKind::Outside);

            // 8 handles
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
                painter.rect_stroke(h_rect, 0.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 255)), StrokeKind::Outside);
            }

            // Rotation handle circle above right_top
            let rot_p = Pos2::new(rect.right_top().x + 12.0_f32, rect.right_top().y - 12.0_f32);
            painter.circle_filled(rot_p, 4.0_f32, Color32::from_rgb(0, 180, 255));
            painter.circle_stroke(rot_p, 4.0_f32, Stroke::new(1.0_f32, Color32::WHITE));
        }
    }

    fn hit_test_handles(&self, obj: &Object, screen_pos: Pos2, origin: Pos2, state: &AppState) -> Option<HandleCorner> {
        if let Some((bb_min, bb_max)) = obj.bounding_box() {
            let min_p = Pos2::new(origin.x + bb_min.x as f32 * state.zoom, origin.y + bb_min.y as f32 * state.zoom);
            let max_p = Pos2::new(origin.x + bb_max.x as f32 * state.zoom, origin.y + bb_max.y as f32 * state.zoom);
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

    fn update_rotate(&self, state: &mut AppState, wx: f64, wy: f64) {
        for id in &state.selected_ids {
            for (_, obj) in state.document.all_objects_mut() {
                if &obj.id == id {
                    let cx = obj.transform.x;
                    let cy = obj.transform.y;
                    let angle = (wy - cy).atan2(wx - cx);
                    obj.transform.rotation = angle;
                }
            }
        }
    }

    fn hit_test_nodes(&self, state: &AppState, screen_pos: Pos2, origin: Pos2) -> Option<(usize, String)> {
        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                let poly = obj.to_world_polygon();
                for (idx, p) in poly.iter().enumerate() {
                    let sp = Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom);
                    if screen_pos.distance(sp) <= HANDLE_HIT_RADIUS {
                        return Some((idx, id.clone()));
                    }
                }
            }
        }
        None
    }

    fn move_node(&self, state: &mut AppState, node_idx: usize, wx: f64, wy: f64) {
        if let Some(ref obj_id) = self.node_edit_state.selected_object_id {
            for (_, obj) in state.document.all_objects_mut() {
                if &obj.id == obj_id {
                    if let ObjectType::Path(ref mut path) = obj.object_type {
                        if let Some(elem) = path.elements.get_mut(node_idx) {
                            match elem {
                                PathElement::MoveTo(p) | PathElement::LineTo(p) => {
                                    *p = AnchorPoint::new(wx, wy);
                                }
                                PathElement::CurveTo(seg) => {
                                    seg.end = AnchorPoint::new(wx, wy);
                                }
                                PathElement::ClosePath => {}
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_node_edit(&self, painter: &egui::Painter, origin: Pos2, state: &AppState) {
        for id in &state.selected_ids {
            if let Some((_, obj)) = state.document.all_objects().find(|(_, o)| &o.id == id) {
                let poly = obj.to_world_polygon();
                for p in poly {
                    let sp = Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom);
                    painter.circle_filled(sp, 4.0_f32, Color32::from_rgb(0, 120, 255));
                    painter.circle_stroke(sp, 4.0_f32, Stroke::new(1.0_f32, Color32::WHITE));
                }
            }
        }
    }

    fn draw_rect_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let p1 = Pos2::new(origin.x + drag.start_world.0 as f32 * state.zoom, origin.y + drag.start_world.1 as f32 * state.zoom);
        let p2 = Pos2::new(origin.x + drag.current_world.0 as f32 * state.zoom, origin.y + drag.current_world.1 as f32 * state.zoom);
        let rect = Rect::from_two_pos(p1, p2);
        painter.rect_filled(rect, 0.0_f32, Color32::from_rgba_unmultiplied(0, 120, 255, 40));
        painter.rect_stroke(rect, 0.0_f32, Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255)), StrokeKind::Outside);
    }

    fn draw_ellipse_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let p1 = Pos2::new(origin.x + drag.start_world.0 as f32 * state.zoom, origin.y + drag.start_world.1 as f32 * state.zoom);
        let p2 = Pos2::new(origin.x + drag.current_world.0 as f32 * state.zoom, origin.y + drag.current_world.1 as f32 * state.zoom);
        let rect = Rect::from_two_pos(p1, p2);
        painter.circle_filled(rect.center(), rect.width() / 2.0_f32, Color32::from_rgba_unmultiplied(0, 120, 255, 40));
        painter.circle_stroke(rect.center(), rect.width() / 2.0_f32, Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255)));
    }

    fn draw_star_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let dx = drag.current_world.0 - drag.start_world.0;
        let dy = drag.current_world.1 - drag.start_world.1;
        let outer_r = (dx * dx + dy * dy).sqrt();
        let inner_r = outer_r * state.star_inner_ratio;
        let path = PathData::from_star(state.star_points, inner_r, outer_radius_to_f64(outer_r), drag.start_world.0, drag.start_world.1);
        let screen_pts: Vec<Pos2> = path.to_polygon(1).iter().map(|p| Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom)).collect();
        painter.add(egui::epaint::PathShape::closed_line(screen_pts, Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255))));
    }

    fn draw_polygon_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let dx = drag.current_world.0 - drag.start_world.0;
        let dy = drag.current_world.1 - drag.start_world.1;
        let radius = (dx * dx + dy * dy).sqrt();
        let path = PathData::from_polygon(state.polygon_sides, radius, drag.start_world.0, drag.start_world.1);
        let screen_pts: Vec<Pos2> = path.to_polygon(1).iter().map(|p| Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom)).collect();
        painter.add(egui::epaint::PathShape::closed_line(screen_pts, Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255))));
    }

    fn draw_line_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let p1 = Pos2::new(origin.x + drag.start_world.0 as f32 * state.zoom, origin.y + drag.start_world.1 as f32 * state.zoom);
        let p2 = Pos2::new(origin.x + drag.current_world.0 as f32 * state.zoom, origin.y + drag.current_world.1 as f32 * state.zoom);
        painter.line_segment([p1, p2], Stroke::new(2.0_f32, Color32::from_rgb(0, 120, 255)));
    }

    fn draw_pencil_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        if drag.pencil_points.len() >= 2 {
            let pts: Vec<Pos2> = drag.pencil_points.iter().map(|p| Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom)).collect();
            painter.add(egui::epaint::PathShape::line(pts, Stroke::new(2.0_f32, Color32::from_rgb(0, 120, 255))));
        }
    }

    fn draw_marquee(&self, painter: &egui::Painter, origin: Pos2, state: &AppState, drag: &DragState) {
        let p1 = Pos2::new(origin.x + drag.start_world.0 as f32 * state.zoom, origin.y + drag.start_world.1 as f32 * state.zoom);
        let p2 = Pos2::new(origin.x + drag.current_world.0 as f32 * state.zoom, origin.y + drag.current_world.1 as f32 * state.zoom);
        let rect = Rect::from_two_pos(p1, p2);
        painter.rect_filled(rect, 0.0_f32, Color32::from_rgba_unmultiplied(0, 120, 255, 30));
        painter.rect_stroke(rect, 0.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 255)), StrokeKind::Outside);
    }

    fn draw_pen_preview(&self, painter: &egui::Painter, origin: Pos2, state: &AppState) {
        let mut screen_pts = Vec::new();
        for p in self.pen_state.preview_points() {
            let sp = Pos2::new(origin.x + p.x as f32 * state.zoom, origin.y + p.y as f32 * state.zoom);
            screen_pts.push(sp);
            painter.circle_filled(sp, 4.0_f32, Color32::WHITE);
            painter.circle_stroke(sp, 4.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(0, 120, 255)));
        }

        if let Some((lx, ly)) = self.pen_state.last_point {
            let lp = Pos2::new(origin.x + lx as f32 * state.zoom, origin.y + ly as f32 * state.zoom);
            if let Some(last_sp) = screen_pts.last() {
                painter.line_segment([*last_sp, lp], Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255)));
            }
        }

        if screen_pts.len() >= 2 {
            painter.add(egui::epaint::PathShape::line(screen_pts, Stroke::new(1.5_f32, Color32::from_rgb(0, 120, 255))));
        }
    }
}

fn outer_radius_to_f64(r: f64) -> f64 {
    r
}
