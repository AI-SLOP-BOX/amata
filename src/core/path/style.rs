use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════
// Pattern Fill: Repeating tile patterns
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PatternType {
    #[default]
    Grid,
    Hex,
    Brick,
    Dots,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternFill {
    pub pattern_type: PatternType,
    pub tile_width: f64,
    pub tile_height: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub rotation: f64,
    pub scale: f64,
}

impl Default for PatternFill {
    fn default() -> Self {
        Self {
            pattern_type: PatternType::Grid,
            tile_width: 40.0,
            tile_height: 40.0,
            offset_x: 0.0,
            offset_y: 0.0,
            rotation: 0.0,
            scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnchorPoint {
    pub x: f64,
    pub y: f64,
}

impl AnchorPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn to_kurbo(self) -> kurbo::Point {
        kurbo::Point::new(self.x, self.y)
    }

    pub fn from_kurbo(p: kurbo::Point) -> Self {
        Self { x: p.x, y: p.y }
    }

    pub fn distance(self, other: Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BezierSegment {
    pub start: AnchorPoint,
    pub control1: AnchorPoint,
    pub control2: AnchorPoint,
    pub end: AnchorPoint,
}

impl BezierSegment {
    pub fn line(start: AnchorPoint, end: AnchorPoint) -> Self {
        Self {
            start,
            control1: start,
            control2: end,
            end,
        }
    }

    pub fn cubic(start: AnchorPoint, c1: AnchorPoint, c2: AnchorPoint, end: AnchorPoint) -> Self {
        Self {
            start,
            control1: c1,
            control2: c2,
            end,
        }
    }

    pub fn is_line(&self) -> bool {
        self.control1 == self.start && self.control2 == self.end
    }

    pub fn eval(&self, t: f64) -> AnchorPoint {
        let t = t.clamp(0.0, 1.0);
        let u = 1.0 - t;
        let u2 = u * u;
        let u3 = u2 * u;
        let t2 = t * t;
        let t3 = t2 * t;

        AnchorPoint::new(
            u3 * self.start.x
                + 3.0 * u2 * t * self.control1.x
                + 3.0 * u * t2 * self.control2.x
                + t3 * self.end.x,
            u3 * self.start.y
                + 3.0 * u2 * t * self.control1.y
                + 3.0 * u * t2 * self.control2.y
                + t3 * self.end.y,
        )
    }

    pub fn to_kurbo(&self) -> kurbo::CubicBez {
        kurbo::CubicBez::new(
            self.start.to_kurbo(),
            self.control1.to_kurbo(),
            self.control2.to_kurbo(),
            self.end.to_kurbo(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathElement {
    MoveTo(AnchorPoint),
    LineTo(AnchorPoint),
    CurveTo(BezierSegment),
    ClosePath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FillRule {
    NonZero,
    #[default]
    EvenOdd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    pub offset: f32, // 0.0 to 1.0
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinearGradient {
    pub start_x: f32,
    pub start_y: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub stops: Vec<GradientStop>,
}

impl Default for LinearGradient {
    fn default() -> Self {
        Self {
            start_x: 0.0,
            start_y: 0.0,
            end_x: 1.0,
            end_y: 1.0,
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: [0.2, 0.6, 1.0, 1.0],
                },
                GradientStop {
                    offset: 1.0,
                    color: [0.8, 0.2, 0.9, 1.0],
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadialGradient {
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
    pub focus_x: f32,
    pub focus_y: f32,
    pub stops: Vec<GradientStop>,
}

impl Default for RadialGradient {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            radius: 0.5,
            focus_x: 0.5,
            focus_y: 0.5,
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                },
                GradientStop {
                    offset: 1.0,
                    color: [0.0, 0.0, 0.0, 1.0],
                },
            ],
        }
    }
}

/// How an image fill maps onto the object's shape.
///
/// The four modes mirror CSS `background-size` semantics so canvas preview,
/// SVG export, and raster export can agree on one definition:
///
/// * [`ImageTileMode::Cover`] — uniform scale until the shape bbox is fully
///   covered, centered, overflow cropped (SVG `preserveAspectRatio="xMidYMid
///   slice"`).
/// * [`ImageTileMode::Contain`] — uniform scale until the whole image fits
///   inside the bbox, centered, empty bands left transparent (SVG
///   `"xMidYMid meet"`).
/// * [`ImageTileMode::Fit`] — non-uniform stretch to the bbox exactly,
///   aspect ratio ignored (SVG `"none"`).
/// * [`ImageTileMode::Tile`] — repeat at natural pixel size (1px = 1 world
///   unit) anchored at the bbox origin (SVG `<pattern>` tiling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageTileMode {
    #[serde(rename = "cover")]
    Cover,
    #[serde(rename = "contain")]
    Contain,
    #[serde(rename = "fit")]
    Fit,
    #[serde(rename = "tile")]
    Tile,
}

impl Default for ImageTileMode {
    fn default() -> Self {
        Self::Cover
    }
}

/// An image used as a fill. The actual pixels live in the document's image
/// objects (`ObjectType::Image`); this struct only keeps a reference so a fill
/// can follow an image object that the user later edits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageFill {
    /// ID of the `ObjectType::Image` object whose PNG bytes provide the pixels.
    pub image_id: String,
    /// How the image should be mapped onto the shape.
    #[serde(default)]
    pub tile_mode: ImageTileMode,
    /// Optional crop in image pixel coordinates (0..=1 relative to pixel size).
    #[serde(default)]
    pub crop_rect: Option<[f32; 4]>,
}

impl Default for ImageFill {
    fn default() -> Self {
        Self {
            image_id: String::new(),
            tile_mode: ImageTileMode::Cover,
            crop_rect: None,
        }
    }
}

/// Eraser-style gap between tiles when `ImageTileMode::Tile` is used.
pub const DEFAULT_IMAGE_TILE_GAP: f64 = 0.0;

impl ImageFill {
    pub fn new(image_id: impl Into<String>) -> Self {
        Self {
            image_id: image_id.into(),
            ..Default::default()
        }
    }

    /// True when this image fill refers to a non-empty image.
    pub fn has_image(&self) -> bool {
        !self.image_id.is_empty()
    }
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FillType {
    Solid([f32; 4]),
    Linear(LinearGradient),
    Radial(RadialGradient),
    Pattern(PatternFill),
    Image(ImageFill),
}

impl Default for FillType {
    fn default() -> Self {
        FillType::Solid([0.0, 0.0, 0.0, 1.0])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FillStyle {
    pub color: [f32; 4],
    pub fill_type: FillType,
    pub rule: FillRule,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 1.0],
            fill_type: FillType::Solid([0.0, 0.0, 0.0, 1.0]),
            rule: FillRule::NonZero,
        }
    }
}

impl FillStyle {
    pub fn solid(color: [f32; 4]) -> Self {
        Self {
            color,
            fill_type: FillType::Solid(color),
            rule: FillRule::NonZero,
        }
    }

    pub fn linear_gradient(gradient: LinearGradient) -> Self {
        let first_color = gradient
            .stops
            .first()
            .map(|s| s.color)
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        Self {
            color: first_color,
            fill_type: FillType::Linear(gradient),
            rule: FillRule::NonZero,
        }
    }

    pub fn radial_gradient(gradient: RadialGradient) -> Self {
        let first_color = gradient
            .stops
            .first()
            .map(|s| s.color)
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        Self {
            color: first_color,
            fill_type: FillType::Radial(gradient),
            rule: FillRule::NonZero,
        }
    }

    pub fn image_fill(image_id: impl Into<String>, tile_mode: ImageTileMode) -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 0.0],
            fill_type: FillType::Image(ImageFill {
                image_id: image_id.into(),
                tile_mode,
                ..Default::default()
            }),
            rule: FillRule::NonZero,
        }
    }

    /// Best-guess alpha color for UI previews. Image fills return (0,0,0,0) so
    /// callers can distinguish "no color" from a solid fill.
    pub fn alpha_color(&self) -> [f32; 4] {
        match &self.fill_type {
            FillType::Solid(c) => *c,
            FillType::Linear(g) => g.stops.first().map(|s| s.color).unwrap_or([0.0; 4]),
            FillType::Radial(g) => g.stops.first().map(|s| s.color).unwrap_or([0.0; 4]),
            FillType::Pattern(_) => self.color,
            FillType::Image(_) => [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// True when this style should be drawn as a raster image fill.
    pub fn is_image_fill(&self) -> bool {
        matches!(self.fill_type, FillType::Image(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StrokeCap {
    #[default]
    Butt,
    Round,
    Square,
}

impl StrokeCap {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Butt => "Butt",
            Self::Round => "Round",
            Self::Square => "Square",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StrokeJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

impl StrokeJoin {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Miter => "Miter",
            Self::Round => "Round",
            Self::Bevel => "Bevel",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ArrowHead {
    #[default]
    None,
    Triangle,
    Arrow,
    Circle,
    Diamond,
    Square,
    Barbed,
}

impl ArrowHead {
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Triangle => "Triangle",
            Self::Arrow => "Arrow",
            Self::Circle => "Circle",
            Self::Diamond => "Diamond",
            Self::Square => "Square",
            Self::Barbed => "Barbed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrokeStyle {
    pub color: [f32; 4],
    pub width: f64,
    pub dash_pattern: Option<Vec<f64>>,
    pub cap: StrokeCap,
    pub join: StrokeJoin,
    pub miter_limit: f64,
    pub arrow_start: ArrowHead,
    pub arrow_end: ArrowHead,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 1.0],
            width: 1.0,
            dash_pattern: None,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            miter_limit: 4.0,
            arrow_start: ArrowHead::None,
            arrow_end: ArrowHead::None,
        }
    }
}
