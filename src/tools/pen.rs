use crate::core::document::Object;
use crate::core::path::{AnchorPoint, FillStyle, PathData, StrokeStyle};

pub struct PenState {
    pub points: Vec<PenPoint>,
    pub is_drawing: bool,
    pub last_point: Option<(f64, f64)>,
    pub hover_pos: Option<(f64, f64)>,
    /// While the user drags after placing an anchor, we pull bezier handles out
    pub dragging_handle: bool,
}

#[derive(Debug, Clone)]
pub struct PenPoint {
    pub anchor: AnchorPoint,
    pub handle_in: Option<AnchorPoint>,
    pub handle_out: Option<AnchorPoint>,
}

impl PenState {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            is_drawing: false,
            last_point: None,
            hover_pos: None,
            dragging_handle: false,
        }
    }

    pub fn start_path(&mut self, x: f64, y: f64) {
        self.points.clear();
        self.points.push(PenPoint {
            anchor: AnchorPoint::new(x, y),
            handle_in: None,
            handle_out: None,
        });
        self.is_drawing = true;
        self.last_point = Some((x, y));
        self.dragging_handle = true;
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        if !self.is_drawing {
            self.start_path(x, y);
            return;
        }

        self.points.push(PenPoint {
            anchor: AnchorPoint::new(x, y),
            handle_in: None,
            handle_out: None,
        });
        self.last_point = Some((x, y));
        self.dragging_handle = true;
    }

    /// Called while dragging after placing an anchor: set smooth bezier handles
    pub fn drag_handle(&mut self, hx: f64, hy: f64) {
        if let Some(last) = self.points.last_mut() {
            // handle_out goes to where the mouse dragged
            last.handle_out = Some(AnchorPoint::new(hx, hy));
            // handle_in is the mirror (smooth node)
            let ax = last.anchor.x;
            let ay = last.anchor.y;
            last.handle_in = Some(AnchorPoint::new(2.0 * ax - hx, 2.0 * ay - hy));
        }
    }

    pub fn end_drag(&mut self) {
        self.dragging_handle = false;
    }

    pub fn update_hover(&mut self, x: f64, y: f64) {
        self.hover_pos = Some((x, y));
        if self.is_drawing && !self.points.is_empty() {
            self.last_point = Some((x, y));
        }
    }

    pub fn finish_path(
        &mut self,
        fill_color: [f32; 4],
        stroke_color: [f32; 4],
        stroke_width: f64,
    ) -> Option<Object> {
        if self.points.len() >= 2 {
            let mut path = PathData::new();

            for (i, pt) in self.points.iter().enumerate() {
                if i == 0 {
                    path.push_move_to(pt.anchor.x, pt.anchor.y);
                } else {
                    let prev = &self.points[i - 1];
                    let has_handles = prev.handle_out.is_some() || pt.handle_in.is_some();

                    if has_handles {
                        let c0 = prev.handle_out.unwrap_or(prev.anchor);
                        let c1 = pt.handle_in.unwrap_or(pt.anchor);
                        path.push_curve_to(c0, c1, pt.anchor);
                    } else {
                        path.push_line_to(pt.anchor.x, pt.anchor.y);
                    }
                }
            }

            let first = &self.points[0];
            let last = &self.points[self.points.len() - 1];
            if first.anchor.x == last.anchor.x && first.anchor.y == last.anchor.y {
                path.close();
            }

            path.fill = Some(FillStyle::solid(fill_color));
            path.stroke = Some(StrokeStyle {
                color: stroke_color,
                width: stroke_width,
                dash_pattern: None,
                ..StrokeStyle::default()
            });

            let obj = Object::new_path("Path", path);
            self.points.clear();
            self.is_drawing = false;
            self.last_point = None;
            self.dragging_handle = false;
            Some(obj)
        } else {
            self.cancel();
            None
        }
    }

    pub fn cancel(&mut self) {
        self.points.clear();
        self.is_drawing = false;
        self.last_point = None;
        self.dragging_handle = false;
    }

    pub fn preview_points(&self) -> Vec<AnchorPoint> {
        self.points.iter().map(|p| p.anchor).collect()
    }
}

impl Default for PenState {
    fn default() -> Self {
        Self::new()
    }
}
