use super::effects::{DropShadow, GlowEffect};
use super::path::{AnchorPoint, FillStyle, PathData, StrokeStyle};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::Darken => "Darken",
            Self::Lighten => "Lighten",
            Self::ColorDodge => "Color Dodge",
            Self::ColorBurn => "Color Burn",
            Self::HardLight => "Hard Light",
            Self::SoftLight => "Soft Light",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
        }
    }

    pub fn all() -> &'static [BlendMode] {
        &[
            Self::Normal, Self::Multiply, Self::Screen, Self::Overlay,
            Self::Darken, Self::Lighten, Self::ColorDodge, Self::ColorBurn,
            Self::HardLight, Self::SoftLight, Self::Difference, Self::Exclusion,
            Self::Hue, Self::Saturation, Self::Color, Self::Luminosity,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    Path(PathData),
    Rectangle { width: f64, height: f64, corner_radius: f64 },
    Ellipse { rx: f64, ry: f64 },
    Star { points: usize, inner_radius: f64, outer_radius: f64 },
    Polygon { sides: usize, radius: f64 },
    Line { x2: f64, y2: f64 },
    Text { text: String, font_size: f64 },
    Group(Vec<Object>),
    ClippingMask { children: Vec<Object> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub skew_x: f64,
    pub skew_y: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            skew_x: 0.0,
            skew_y: 0.0,
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> [f64; 6] {
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let skew_x_rad = self.skew_x.to_radians();
        let skew_y_rad = self.skew_y.to_radians();
        [
            cos * self.scale_x + skew_x_rad.sin() * self.scale_x,
            sin * self.scale_x + skew_x_rad.cos() * self.scale_x,
            -sin * self.scale_y + skew_y_rad.sin() * self.scale_y,
            cos * self.scale_y + skew_y_rad.cos() * self.scale_y,
            self.x,
            self.y,
        ]
    }

    pub fn transform_point(&self, px: f64, py: f64) -> (f64, f64) {
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let sx = px * self.scale_x;
        let sy = py * self.scale_y;
        let skew_x_rad = self.skew_x.to_radians();
        let skew_y_rad = self.skew_y.to_radians();
        let skewed_x = sx + sy * skew_x_rad.sin();
        let skewed_y = sx * skew_y_rad.sin() + sy;
        (
            cos * skewed_x - sin * skewed_y + self.x,
            sin * skewed_x + cos * skewed_y + self.y,
        )
    }

    pub fn inverse_transform_point(&self, wx: f64, wy: f64) -> (f64, f64) {
        let dx = wx - self.x;
        let dy = wy - self.y;
        let cos = (-self.rotation).cos();
        let sin = (-self.rotation).sin();
        let rx = cos * dx - sin * dy;
        let ry = sin * dx + cos * dy;
        let sx = if self.scale_x.abs() > 1e-6 { rx / self.scale_x } else { 0.0 };
        let sy = if self.scale_y.abs() > 1e-6 { ry / self.scale_y } else { 0.0 };
        (sx, sy)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub id: String,
    pub name: String,
    pub object_type: ObjectType,
    pub transform: Transform,
    pub fill: Option<FillStyle>,
    pub stroke: Option<StrokeStyle>,
    pub shadow: Option<DropShadow>,
    pub glow: Option<GlowEffect>,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub visible: bool,
    pub locked: bool,
}

impl Object {
    pub fn new_path(name: &str, path: PathData) -> Self {
        let fill = path.fill.clone();
        let stroke = path.stroke.clone();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Path(path),
            transform: Transform::default(),
            fill,
            stroke,
            shadow: None,
            glow: None,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_rect(name: &str, x: f64, y: f64, w: f64, h: f64, corner_radius: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Rectangle { width: w, height: h, corner_radius },
            transform: Transform { x, y, ..Default::default() },
            fill: Some(FillStyle::default()),
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_ellipse(name: &str, cx: f64, cy: f64, rx: f64, ry: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Ellipse { rx, ry },
            transform: Transform { x: cx, y: cy, ..Default::default() },
            fill: Some(FillStyle::default()),
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_star(name: &str, cx: f64, cy: f64, points: usize, inner_radius: f64, outer_radius: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Star { points, inner_radius, outer_radius },
            transform: Transform { x: cx, y: cy, ..Default::default() },
            fill: Some(FillStyle::default()),
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_polygon(name: &str, cx: f64, cy: f64, sides: usize, radius: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Polygon { sides, radius },
            transform: Transform { x: cx, y: cy, ..Default::default() },
            fill: Some(FillStyle::default()),
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_line(name: &str, x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Line { x2: x2 - x1, y2: y2 - y1 },
            transform: Transform { x: x1, y: y1, ..Default::default() },
            fill: None,
            stroke: Some(StrokeStyle::default()),
            shadow: None,
            glow: None,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_text(name: &str, text: &str, x: f64, y: f64, font_size: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Text { text: text.to_string(), font_size },
            transform: Transform { x, y, ..Default::default() },
            fill: Some(FillStyle::default()),
            stroke: None,
            shadow: None,
            glow: None,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    pub fn new_group(name: &str, objects: Vec<Object>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Group(objects),
            transform: Transform::default(),
            fill: None,
            stroke: None,
            shadow: None,
            glow: None,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            visible: true,
            locked: false,
        }
    }

    /// Converts this object to its canonical local PathData representation
    pub fn to_path_data(&self) -> PathData {
        match &self.object_type {
            ObjectType::Path(path) => path.clone(),
            ObjectType::Rectangle { width, height, corner_radius } => {
                PathData::from_rect(0.0, 0.0, *width, *height, *corner_radius)
            }
            ObjectType::Ellipse { rx, ry } => {
                PathData::from_ellipse(0.0, 0.0, *rx, *ry)
            }
            ObjectType::Star { points, inner_radius, outer_radius } => {
                PathData::from_star(*points, *inner_radius, *outer_radius, 0.0, 0.0)
            }
            ObjectType::Polygon { sides, radius } => {
                PathData::from_polygon(*sides, *radius, 0.0, 0.0)
            }
            ObjectType::Line { x2, y2 } => {
                PathData::from_line(0.0, 0.0, *x2, *y2)
            }
            ObjectType::Text { text, font_size } => {
                // Approximate bounding rect as a path
                let width = text.chars().count() as f64 * font_size * 0.6;
                let height = *font_size;
                PathData::from_rect(0.0, -height, width, height, 0.0)
            }
            ObjectType::Group(children) => {
                let mut combined = PathData::new();
                for child in children {
                    let mut child_path = child.to_path_data();
                    child_path.transform(&child.transform.matrix());
                    combined.elements.extend(child_path.elements);
                }
                combined
            }
            ObjectType::ClippingMask { children } => {
                let mut combined = PathData::new();
                for child in children {
                    let mut child_path = child.to_path_data();
                    child_path.transform(&child.transform.matrix());
                    combined.elements.extend(child_path.elements);
                }
                combined
            }
        }
    }

    pub fn world_path(&self) -> Option<PathData> {
        let mut path = self.to_path_data();
        path.transform(&self.transform.matrix());
        path.fill = self.fill.clone();
        path.stroke = self.stroke.clone();
        Some(path)
    }

    /// Convert object to world polygon points for hit testing and Boolean operations
    pub fn to_world_polygon(&self) -> Vec<AnchorPoint> {
        let mut path = self.to_path_data();
        path.transform(&self.transform.matrix());
        path.to_polygon(16)
    }

    pub fn hit_test(&self, px: f64, py: f64) -> bool {
        let (lx, ly) = self.transform.inverse_transform_point(px, py);

        match &self.object_type {
            ObjectType::Path(path) => {
                let poly = path.to_polygon(8);
                super::geometry::point_in_polygon(lx, ly, &poly)
            }
            ObjectType::Rectangle { width, height, .. } => {
                lx >= 0.0 && lx <= *width && ly >= 0.0 && ly <= *height
            }
            ObjectType::Ellipse { rx, ry } => {
                if *rx <= 0.0 || *ry <= 0.0 {
                    return false;
                }
                let dx = lx / rx;
                let dy = ly / ry;
                dx * dx + dy * dy <= 1.0
            }
            ObjectType::Star { .. } | ObjectType::Polygon { .. } => {
                let poly = self.to_path_data().to_polygon(8);
                super::geometry::point_in_polygon(lx, ly, &poly)
            }
            ObjectType::Line { x2, y2 } => {
                let dist = super::geometry::distance_to_segment(
                    AnchorPoint::new(lx, ly),
                    AnchorPoint::new(0.0, 0.0),
                    AnchorPoint::new(*x2, *y2),
                );
                let stroke_w = self.stroke.as_ref().map(|s| s.width).unwrap_or(2.0);
                dist <= (stroke_w / 2.0).max(4.0)
            }
            ObjectType::Text { text, font_size } => {
                let width = text.chars().count() as f64 * font_size * 0.6;
                let height = *font_size;
                lx >= 0.0 && lx <= width && ly >= -height && ly <= 0.0
            }
            ObjectType::Group(children) => {
                children.iter().any(|c| c.hit_test(lx, ly))
            }
            ObjectType::ClippingMask { children } => {
                children.iter().any(|c| c.hit_test(lx, ly))
            }
        }
    }

    pub fn bounding_box(&self) -> Option<(AnchorPoint, AnchorPoint)> {
        let poly = self.to_world_polygon();
        if poly.is_empty() {
            return None;
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for p in &poly {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
        if min_x <= max_x && min_y <= max_y {
            Some((AnchorPoint::new(min_x, min_y), AnchorPoint::new(max_x, max_y)))
        } else {
            None
        }
    }
}

impl Layer {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            objects: Vec::new(),
            visible: true,
            locked: false,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub objects: Vec<Object>,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub name: String,
    pub layers: Vec<Layer>,
    pub active_layer_idx: usize,
    pub width: f64,
    pub height: f64,
}

impl Default for Document {
    fn default() -> Self {
        let mut doc = Self {
            name: "Untitled".to_string(),
            layers: Vec::new(),
            active_layer_idx: 0,
            width: 1920.0,
            height: 1080.0,
        };
        doc.layers.push(Layer::new("Layer 1"));
        doc
    }
}

impl Document {
    pub fn active_layer(&self) -> &Layer {
        &self.layers[self.active_layer_idx]
    }

    pub fn active_layer_mut(&mut self) -> &mut Layer {
        &mut self.layers[self.active_layer_idx]
    }

    pub fn add_object(&mut self, obj: Object) {
        self.active_layer_mut().objects.push(obj);
    }

    pub fn all_objects(&self) -> impl DoubleEndedIterator<Item = (usize, &Object)> {
        self.layers.iter().enumerate().flat_map(|(i, layer)| {
            layer.objects.iter().map(move |obj| (i, obj))
        })
    }

    pub fn all_objects_mut(&mut self) -> impl Iterator<Item = (usize, &mut Object)> {
        self.layers.iter_mut().enumerate().flat_map(|(i, layer)| {
            layer.objects.iter_mut().map(move |obj| (i, obj))
        })
    }

    pub fn object_by_id(&self, id: &str) -> Option<(usize, &Object)> {
        self.all_objects().find(|(_, o)| o.id == id)
    }

    pub fn remove_object(&mut self, id: &str) -> Option<Object> {
        for layer in &mut self.layers {
            if let Some(pos) = layer.objects.iter().position(|o| o.id == id) {
                return Some(layer.objects.remove(pos));
            }
        }
        None
    }
}

// ═══════════════════════════════════════════════════════════════════
// Symbol: Reusable object definition
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: String,
    pub name: String,
    pub object: Object,
    pub use_count: usize,
}

impl Symbol {
    pub fn new(name: &str, object: Object) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object,
            use_count: 0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Width Point: Variable stroke width
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidthPoint {
    pub position: f64,
    pub width: f64,
    pub side: WidthSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WidthSide {
    Left,
    Right,
    #[default]
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidthProfile {
    pub points: Vec<WidthPoint>,
}

impl Default for WidthProfile {
    fn default() -> Self {
        Self {
            points: vec![
                WidthPoint { position: 0.0, width: 1.0, side: WidthSide::Both },
                WidthPoint { position: 1.0, width: 1.0, side: WidthSide::Both },
            ],
        }
    }
}
