use super::document::Document;
use super::history::UndoManager;

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
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub selected_ids: Vec<String>,
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub show_grid: bool,
    pub grid_size: f64,
    pub snap_to_grid: bool,
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            document: Document::default(),
            undo_manager: UndoManager::new(),
            current_tool: Tool::Select,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            selected_ids: Vec::new(),
            canvas_width: 0.0,
            canvas_height: 0.0,
            show_grid: true,
            grid_size: 50.0,
            snap_to_grid: false,
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
        }
    }
}

impl AppState {
    pub fn screen_to_world(&self, sx: f32, sy: f32) -> (f64, f64) {
        let wx = (sx - self.pan_x) as f64 / self.zoom as f64;
        let wy = (sy - self.pan_y) as f64 / self.zoom as f64;
        (wx, wy)
    }

    pub fn world_to_screen(&self, wx: f64, wy: f64) -> (f32, f32) {
        let sx = wx as f32 * self.zoom + self.pan_x;
        let sy = wy as f32 * self.zoom + self.pan_y;
        (sx, sy)
    }

    pub fn snap(&self, x: f64, y: f64) -> (f64, f64) {
        if self.snap_to_grid && self.grid_size > 0.0 {
            (
                (x / self.grid_size).round() * self.grid_size,
                (y / self.grid_size).round() * self.grid_size,
            )
        } else {
            (x, y)
        }
    }
}
