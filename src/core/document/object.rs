use super::super::effects::{DropShadow, GlowEffect};
use crate::core::path::{AnchorPoint, FillStyle, PathData, StrokeStyle};
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
            Self::Normal,
            Self::Multiply,
            Self::Screen,
            Self::Overlay,
            Self::Darken,
            Self::Lighten,
            Self::ColorDodge,
            Self::ColorBurn,
            Self::HardLight,
            Self::SoftLight,
            Self::Difference,
            Self::Exclusion,
            Self::Hue,
            Self::Saturation,
            Self::Color,
            Self::Luminosity,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

impl FontStyle {
    pub fn as_svg_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Italic => "italic",
            Self::Oblique => "oblique",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TextAnchor {
    #[default]
    Start,
    Middle,
    End,
}

impl TextAnchor {
    pub fn as_svg_str(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Middle => "middle",
            Self::End => "end",
        }
    }
}

fn default_font_family() -> String {
    "Inter, sans-serif".to_string()
}

fn default_font_size() -> f64 {
    24.0
}

fn default_font_weight() -> u16 {
    400
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    #[serde(default = "default_font_weight")]
    pub font_weight: u16,
    #[serde(default)]
    pub font_style: FontStyle,
    #[serde(default)]
    pub letter_spacing: f64,
    #[serde(default)]
    pub text_anchor: TextAnchor,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: default_font_size(),
            font_weight: default_font_weight(),
            font_style: FontStyle::Normal,
            letter_spacing: 0.0,
            text_anchor: TextAnchor::Start,
        }
    }
}

impl TextStyle {
    pub fn new(font_family: impl Into<String>, font_size: f64) -> Self {
        Self {
            font_family: font_family.into(),
            font_size,
            font_weight: 400,
            font_style: FontStyle::Normal,
            letter_spacing: 0.0,
            text_anchor: TextAnchor::Start,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectType {
    Path(PathData),
    Rectangle {
        width: f64,
        height: f64,
        corner_radius: f64,
    },
    Ellipse {
        rx: f64,
        ry: f64,
    },
    Star {
        points: usize,
        inner_radius: f64,
        outer_radius: f64,
    },
    Polygon {
        sides: usize,
        radius: f64,
    },
    Line {
        x2: f64,
        y2: f64,
    },
    Text {
        text: String,
        #[serde(default = "default_font_size")]
        font_size: f64,
        #[serde(default)]
        style: TextStyle,
    },
    Group(Vec<Object>),
    ClippingMask {
        children: Vec<Object>,
    },
    Use {
        href: String,
        width: Option<f64>,
        height: Option<f64>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

        // Transformation order: Scale & Skew, then Rotate, then Translate
        // (x', y') = R * K * S * (x, y) + T
        // skewed_x = sx * x + sy * y * sin(skew_x)
        // skewed_y = sx * x * sin(skew_y) + sy * y
        // x_rot = cos * skewed_x - sin * skewed_y + tx
        // y_rot = sin * skewed_x + cos * skewed_y + ty
        let k0 = self.scale_x;
        let k1 = self.scale_x * skew_y_rad.sin();
        let k2 = self.scale_y * skew_x_rad.sin();
        let k3 = self.scale_y;

        let m0 = cos * k0 - sin * k1;
        let m1 = sin * k0 + cos * k1;
        let m2 = cos * k2 - sin * k3;
        let m3 = sin * k2 + cos * k3;

        [m0, m1, m2, m3, self.x, self.y]
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
        let sx = if self.scale_x.abs() > 1e-6 {
            rx / self.scale_x
        } else {
            0.0
        };
        let sy = if self.scale_y.abs() > 1e-6 {
            ry / self.scale_y
        } else {
            0.0
        };
        (sx, sy)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
            object_type: ObjectType::Rectangle {
                width: w,
                height: h,
                corner_radius,
            },
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
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
            transform: Transform {
                x: cx,
                y: cy,
                ..Default::default()
            },
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

    pub fn new_star(
        name: &str,
        cx: f64,
        cy: f64,
        points: usize,
        inner_radius: f64,
        outer_radius: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Star {
                points,
                inner_radius,
                outer_radius,
            },
            transform: Transform {
                x: cx,
                y: cy,
                ..Default::default()
            },
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
            transform: Transform {
                x: cx,
                y: cy,
                ..Default::default()
            },
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
            object_type: ObjectType::Line {
                x2: x2 - x1,
                y2: y2 - y1,
            },
            transform: Transform {
                x: x1,
                y: y1,
                ..Default::default()
            },
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
        Self::new_text_with_style(
            name,
            text,
            x,
            y,
            TextStyle::new("Inter, sans-serif", font_size),
        )
    }

    pub fn new_text_with_style(name: &str, text: &str, x: f64, y: f64, style: TextStyle) -> Self {
        let font_size = style.font_size;
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Text {
                text: text.to_string(),
                font_size,
                style,
            },
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
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

    pub fn new_use(
        name: &str,
        href: &str,
        x: f64,
        y: f64,
        width: Option<f64>,
        height: Option<f64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            object_type: ObjectType::Use {
                href: href.to_string(),
                width,
                height,
            },
            transform: Transform {
                x,
                y,
                ..Default::default()
            },
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

    /// Combine multiple objects into a single Compound Path (holes are created where subpaths overlap using EvenOdd rule)
    pub fn make_compound_path(objects: &[Object]) -> Option<Self> {
        if objects.is_empty() {
            return None;
        }

        let base = &objects[0];
        let mut combined_path = PathData::new();
        combined_path.fill = base.fill.clone().or_else(|| Some(FillStyle::default()));
        if let Some(ref mut fill) = combined_path.fill {
            fill.rule = crate::core::path::FillRule::EvenOdd;
        }
        combined_path.stroke = base.stroke.clone();

        for obj in objects {
            let mut p = obj.to_path_data();
            p.transform(&obj.transform.matrix());
            combined_path.elements.extend(p.elements);
        }

        let mut compound = Self::new_path("Compound Path", combined_path);
        compound.shadow = base.shadow.clone();
        compound.glow = base.glow.clone();
        compound.opacity = base.opacity;
        compound.blend_mode = base.blend_mode;
        Some(compound)
    }

    /// Release a Compound Path into its component independent subpath objects
    pub fn release_compound_path(&self) -> Vec<Self> {
        if let ObjectType::Path(ref path) = self.object_type {
            let subpaths = path.to_subpaths(16);
            if subpaths.len() <= 1 {
                return vec![self.clone()];
            }

            let mut released = Vec::new();
            for (idx, sp) in subpaths.into_iter().enumerate() {
                let mut p = PathData::from_polygon_points(&sp, true);
                p.fill = self.fill.clone();
                p.stroke = self.stroke.clone();
                let mut obj = Self::new_path(&format!("{}_part_{}", self.name, idx + 1), p);
                obj.transform = self.transform.clone();
                obj.shadow = self.shadow.clone();
                obj.glow = self.glow.clone();
                obj.opacity = self.opacity;
                obj.blend_mode = self.blend_mode;
                released.push(obj);
            }
            released
        } else {
            vec![self.clone()]
        }
    }

    /// Converts this object to its canonical local PathData representation
    pub fn to_path_data(&self) -> PathData {
        match &self.object_type {
            ObjectType::Path(path) => path.clone(),
            ObjectType::Rectangle {
                width,
                height,
                corner_radius,
            } => PathData::from_rect(0.0, 0.0, *width, *height, *corner_radius),
            ObjectType::Ellipse { rx, ry } => PathData::from_ellipse(0.0, 0.0, *rx, *ry),
            ObjectType::Star {
                points,
                inner_radius,
                outer_radius,
            } => PathData::from_star(*points, *inner_radius, *outer_radius, 0.0, 0.0),
            ObjectType::Polygon { sides, radius } => {
                PathData::from_polygon(*sides, *radius, 0.0, 0.0)
            }
            ObjectType::Line { x2, y2 } => PathData::from_line(0.0, 0.0, *x2, *y2),
            ObjectType::Text {
                text, font_size, ..
            } => {
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
            ObjectType::Use { width, height, .. } => {
                let w = width.unwrap_or(100.0);
                let h = height.unwrap_or(100.0);
                PathData::from_rect(0.0, 0.0, w, h, 0.0)
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
                let subpaths = path.to_subpaths(8);
                let even_odd = path
                    .fill
                    .as_ref()
                    .map(|f| matches!(f.rule, crate::core::path::FillRule::EvenOdd))
                    .unwrap_or(true);
                crate::core::geometry::point_in_subpaths(lx, ly, &subpaths, even_odd)
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
                crate::core::geometry::point_in_polygon(lx, ly, &poly)
            }
            ObjectType::Line { x2, y2 } => {
                let dist = crate::core::geometry::distance_to_segment(
                    AnchorPoint::new(lx, ly),
                    AnchorPoint::new(0.0, 0.0),
                    AnchorPoint::new(*x2, *y2),
                );
                let stroke_w = self.stroke.as_ref().map(|s| s.width).unwrap_or(2.0);
                dist <= (stroke_w / 2.0).max(4.0)
            }
            ObjectType::Text {
                text, font_size, ..
            } => {
                let width = text.chars().count() as f64 * font_size * 0.6;
                let height = *font_size;
                lx >= 0.0 && lx <= width && ly >= -height && ly <= 0.0
            }
            ObjectType::Group(children) => children.iter().any(|c| c.hit_test(lx, ly)),
            ObjectType::ClippingMask { children } => children.iter().any(|c| c.hit_test(lx, ly)),
            ObjectType::Use { width, height, .. } => {
                let w = width.unwrap_or(100.0);
                let h = height.unwrap_or(100.0);
                lx >= 0.0 && lx <= w && ly >= 0.0 && ly <= h
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
            Some((
                AnchorPoint::new(min_x, min_y),
                AnchorPoint::new(max_x, max_y),
            ))
        } else {
            None
        }
    }
}
