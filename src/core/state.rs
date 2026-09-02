use super::document::Document;
use super::history::UndoManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Node,
    Pen,
    Pencil,
    Rectangle,
    Ellipse,
    Star,
    Polygon,
    Line,
    Text,
    Eyedropper,
    Hand,
    Brush,
    Eraser,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Selection",
            Tool::Node => "Direct Select",
            Tool::Pen => "Pen",
            Tool::Pencil => "Pencil (Smooth)",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Star => "Star",
            Tool::Polygon => "Polygon",
            Tool::Line => "Line Segment",
            Tool::Text => "Type",
            Tool::Eyedropper => "Eyedropper",
            Tool::Hand => "Hand",
            Tool::Brush => "Brush",
            Tool::Eraser => "Eraser",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Tool::Select => "⬚",
            Tool::Node => "◆",
            Tool::Pen => "✒",
            Tool::Pencil => "✎",
            Tool::Rectangle => "▭",
            Tool::Ellipse => "◯",
            Tool::Star => "★",
            Tool::Polygon => "⬡",
            Tool::Line => "╱",
            Tool::Text => "𝐓",
            Tool::Eyedropper => "💧",
            Tool::Hand => "✋",
            Tool::Brush => "🖌",
            Tool::Eraser => "🧹",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            Tool::Select => "V",
            Tool::Node => "A",
            Tool::Pen => "P",
            Tool::Pencil => "N",
            Tool::Rectangle => "U",
            Tool::Ellipse => "O",
            Tool::Star => "S",
            Tool::Polygon => "G",
            Tool::Line => "L",
            Tool::Text => "T",
            Tool::Eyedropper => "I",
            Tool::Hand => "H",
            Tool::Brush => "B",
            Tool::Eraser => "E",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleCorner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Top,
    Bottom,
    Left,
    Right,
    RotateTopRight,
}

pub struct AppState {
    pub document: Document,
    pub undo_manager: UndoManager,
    pub current_tool: Tool,
    pub previous_tool: Tool,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub target_zoom: f32,
    pub target_pan_x: f32,
    pub target_pan_y: f32,
    pub zoom_animation_progress: f32,
    pub start_zoom: f32,
    pub start_pan_x: f32,
    pub start_pan_y: f32,
    pub selected_ids: Vec<String>,
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub show_grid: bool,
    pub grid_size: f64,
    pub snap_to_grid: bool,
    pub snap_to_objects: bool,
    pub snap_to_guides: bool,
    pub snap_to_points: bool,
    pub show_rulers: bool,
    pub show_smart_guides: bool,
    pub fill_color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub stroke_width: f64,
    pub opacity: f32,
    pub corner_radius: f64,
    pub star_points: usize,
    pub star_inner_ratio: f64,
    pub polygon_sides: usize,
    pub font_size: f64,
    pub text_input_buf: String,
    pub is_panning: bool,
    pub drag_start: Option<(f32, f32)>,
    pub clipboard: Vec<crate::core::document::Object>,
    pub timeline: super::timeline::Timeline,
    pub show_timeline: bool,
    // Symbols
    pub symbols: Vec<crate::core::document::Symbol>,
    // Width profiles
    pub width_profiles: std::collections::HashMap<String, crate::core::document::WidthProfile>,
    // Guides
    pub guides: Vec<Guide>,
    // Export
    pub export_format: String,
    pub export_width: f64,
    pub export_height: f64,
    pub export_scale: f32,
    pub export_transparent: bool,
    pub export_svg_viewbox: bool,
    pub export_svg_embed_fonts: bool,
    pub export_scope: String,
    pub export_path: Option<String>,
    pub pending_export: bool,
    // Repeat
    pub repeat_cols: usize,
    pub repeat_rows: usize,
    pub repeat_h_gap: f64,
    pub repeat_v_gap: f64,
    pub repeat_radial_count: usize,
    pub repeat_radial_radius: f64,
    pub repeat_start_angle: f64,
    // Canvas center (set each frame)
    pub canvas_center_x: f32,
    pub canvas_center_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guide {
    pub orientation: GuideOrientation,
    pub position: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuideOrientation {
    Horizontal,
    Vertical,
}

impl Default for Guide {
    fn default() -> Self {
        Self { orientation: GuideOrientation::Horizontal, position: 0.0 }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            document: Document::default(),
            undo_manager: UndoManager::new(),
            current_tool: Tool::Select,
            previous_tool: Tool::Select,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            target_zoom: 1.0,
            target_pan_x: 0.0,
            target_pan_y: 0.0,
            zoom_animation_progress: 1.0,
            start_zoom: 1.0,
            start_pan_x: 0.0,
            start_pan_y: 0.0,
            selected_ids: Vec::new(),
            canvas_width: 0.0,
            canvas_height: 0.0,
            show_grid: true,
            grid_size: 50.0,
            snap_to_grid: false,
            snap_to_objects: true,
            snap_to_guides: true,
            snap_to_points: true,
            show_rulers: true,
            show_smart_guides: true,
            fill_color: [0.2, 0.5, 0.8, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 2.0,
            opacity: 1.0,
            corner_radius: 0.0,
            star_points: 5,
            star_inner_ratio: 0.4,
            polygon_sides: 6,
            font_size: 32.0,
            text_input_buf: "Hello Illustrator".to_string(),
            is_panning: false,
            drag_start: None,
            clipboard: Vec::new(),
            timeline: super::timeline::Timeline::default(),
            show_timeline: true,
            symbols: Vec::new(),
            width_profiles: std::collections::HashMap::new(),
            guides: Vec::new(),
            export_format: "SVG".into(),
            export_width: 1920.0,
            export_height: 1080.0,
            export_scale: 1.0,
            export_transparent: false,
            export_svg_viewbox: true,
            export_svg_embed_fonts: true,
            export_scope: "All".into(),
            export_path: None,
            pending_export: false,
            repeat_cols: 3,
            repeat_rows: 3,
            repeat_h_gap: 50.0,
            repeat_v_gap: 50.0,
            repeat_radial_count: 8,
            repeat_radial_radius: 200.0,
            repeat_start_angle: 0.0,
            canvas_center_x: 0.0,
            canvas_center_y: 0.0,
        }
    }
}

impl AppState {
    pub fn screen_to_world(&self, sx: f32, sy: f32) -> (f64, f64) {
        let wx = (sx - self.canvas_center_x - self.pan_x) as f64 / self.zoom as f64;
        let wy = (sy - self.canvas_center_y - self.pan_y) as f64 / self.zoom as f64;
        (wx, wy)
    }

    pub fn world_to_screen(&self, wx: f64, wy: f64) -> (f32, f32) {
        let sx = wx as f32 * self.zoom + self.pan_x + self.canvas_center_x;
        let sy = wy as f32 * self.zoom + self.pan_y + self.canvas_center_y;
        (sx, sy)
    }

    pub fn snap(&self, x: f64, y: f64) -> (f64, f64) {
        let (mut sx, mut sy) = if self.snap_to_grid && self.grid_size > 0.0 {
            (
                (x / self.grid_size).round() * self.grid_size,
                (y / self.grid_size).round() * self.grid_size,
            )
        } else {
            (x, y)
        };

        if self.snap_to_objects {
            let objects: Vec<&crate::core::document::Object> = self.document.all_objects()
                .filter(|(_, o)| o.visible && !o.locked)
                .map(|(_, o)| o)
                .collect();
            let (osx, osy) = self.snap_to_object_edges(x, y, &objects);
            let grid_dist = ((sx - x).powi(2) + (sy - y).powi(2)).sqrt();
            let obj_dist = ((osx - x).powi(2) + (osy - y).powi(2)).sqrt();
            if obj_dist < grid_dist {
                sx = osx;
                sy = osy;
            }
        }

        (sx, sy)
    }

    /// Snaps to the nearest object edge or center within a threshold (5px in world space).
    /// Returns the snapped position, or the original position if nothing is close enough.
    pub fn snap_to_object_edges(&self, x: f64, y: f64, objects: &[&crate::core::document::Object]) -> (f64, f64) {
        let threshold = 5.0;
        let mut best_x = x;
        let mut best_y = y;
        let mut best_dist = f64::MAX;

        for obj in objects {
            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                let cx = (bb_min.x + bb_max.x) / 2.0;
                let cy = (bb_min.y + bb_max.y) / 2.0;

                let edges = [
                    (bb_min.x, cy),   // left edge center
                    (bb_max.x, cy),   // right edge center
                    (cx, bb_min.y),   // top edge center
                    (cx, bb_max.y),   // bottom edge center
                    (bb_min.x, bb_min.y), // top-left corner
                    (bb_max.x, bb_min.y), // top-right corner
                    (bb_min.x, bb_max.y), // bottom-left corner
                    (bb_max.x, bb_max.y), // bottom-right corner
                    (cx, cy),         // center
                ];

                for (ex, ey) in edges {
                    let dist = ((ex - x).powi(2) + (ey - y).powi(2)).sqrt();
                    if dist < threshold && dist < best_dist {
                        best_dist = dist;
                        best_x = ex;
                        best_y = ey;
                    }
                }
            }
        }

        (best_x, best_y)
    }

    /// Calculate and apply zoom-to-fit, setting animation targets
    pub fn zoom_to_fit(&mut self) {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for (_, obj) in self.document.all_objects() {
            if let Some((bb_min, bb_max)) = obj.bounding_box() {
                min_x = min_x.min(bb_min.x);
                min_y = min_y.min(bb_min.y);
                max_x = max_x.max(bb_max.x);
                max_y = max_y.max(bb_max.y);
            }
        }
        if min_x < max_x && min_y < max_y {
            let obj_w = max_x - min_x;
            let obj_h = max_y - min_y;
            let cw = self.canvas_width;
            let ch = self.canvas_height;
            let new_zoom = ((cw / obj_w as f32).min(ch / obj_h as f32) * 0.85).clamp(0.01, 100.0);
            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;
            self.target_zoom = new_zoom;
            self.target_pan_x = cw / 2.0 - center_x as f32 * new_zoom;
            self.target_pan_y = ch / 2.0 - center_y as f32 * new_zoom;
        } else {
            self.target_zoom = 1.0;
            self.target_pan_x = 0.0;
            self.target_pan_y = 0.0;
        }
    }
}
