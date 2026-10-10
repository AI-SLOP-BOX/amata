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

/// Primitive geometry shared by the canvas and print-PDF pattern painters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternPrimitive {
    Line(AnchorPoint, AnchorPoint),
    Dot(AnchorPoint, f64),
}

/// Per-frame canvas budget for pattern motifs. Heavy documents can expand
/// to `MAX_PRIMITIVES` in world space; the interactive painter stride-samples
/// down to this so one object cannot stall a frame. Print-PDF output still
/// uses the full list.
pub const PATTERN_FRAME_BUDGET: usize = 2000;

/// Stride-sample primitives to `budget` while preserving overall coverage.
/// Truncation would keep only the top rows; stepping keeps the motif visible
/// everywhere when the frame budget kicks in.
pub fn sample_pattern_primitives_for_frame(
    primitives: &[PatternPrimitive],
    budget: usize,
) -> Vec<PatternPrimitive> {
    if budget == 0 || primitives.is_empty() {
        return Vec::new();
    }
    if primitives.len() <= budget {
        return primitives.to_vec();
    }
    let step = primitives.len().div_ceil(budget).max(1);
    primitives.iter().step_by(step).copied().collect()
}

/// Expand the selected pattern type into a bounded list of local-world-space
/// strokes/dots. The caller clips the result to the object's fill path.
pub fn pattern_primitives(
    pattern: &PatternFill,
    bounds: (f64, f64, f64, f64),
) -> Option<Vec<PatternPrimitive>> {
    const MAX_PRIMITIVES: usize = 100_000;
    let (x0, y0, x1, y1) = bounds;
    let (step_x, step_y) = (
        pattern.tile_width * pattern.scale,
        pattern.tile_height * pattern.scale,
    );
    if !(x0.is_finite()
        && y0.is_finite()
        && x1.is_finite()
        && y1.is_finite()
        && pattern.offset_x.is_finite()
        && pattern.offset_y.is_finite()
        && pattern.rotation.is_finite()
        && step_x.is_finite()
        && step_y.is_finite()
        && x1 > x0
        && y1 > y0
        && step_x > 0.0
        && step_y > 0.0)
    {
        return None;
    }
    let ox = x0 + pattern.offset_x;
    let oy = y0 + pattern.offset_y;
    let first_x = ox + ((x0 - ox) / step_x).floor() * step_x;
    let first_y = oy + ((y0 - oy) / step_y).floor() * step_y;
    let cols = ((x1 - first_x) / step_x).ceil().max(0.0) as usize + 1;
    let rows = ((y1 - first_y) / step_y).ceil().max(0.0) as usize + 1;
    if cols.saturating_mul(rows) > MAX_PRIMITIVES / 6 {
        return None;
    }

    let mut primitives = Vec::new();
    let mut overflowed = false;
    let mut line = |a: (f64, f64), b: (f64, f64)| {
        if overflowed {
            return;
        }
        if primitives.len() < MAX_PRIMITIVES {
            primitives.push(PatternPrimitive::Line(
                AnchorPoint::new(a.0, a.1),
                AnchorPoint::new(b.0, b.1),
            ));
        } else {
            overflowed = true;
        }
    };
    match pattern.pattern_type {
        PatternType::Grid => {
            for col in 0..cols {
                let x = first_x + col as f64 * step_x;
                line((x, y0), (x, y1));
            }
            for row in 0..rows {
                let y = first_y + row as f64 * step_y;
                line((x0, y), (x1, y));
            }
        }
        PatternType::Brick => {
            for row in 0..rows {
                let y = first_y + row as f64 * step_y;
                line((x0, y), (x1, y));
                let stagger = if row % 2 == 0 { 0.0 } else { step_x * 0.5 };
                let first_brick_x = first_x + stagger;
                let brick_cols = ((x1 - first_brick_x) / step_x).ceil().max(0.0) as usize + 1;
                for col in 0..brick_cols {
                    let x = first_brick_x + col as f64 * step_x;
                    line((x, y), (x, (y + step_y).min(y1)));
                }
            }
        }
        PatternType::Hex => {
            let radius_x = step_x * 0.5;
            let radius_y = step_y * 0.5;
            let row_step = step_y * 0.75;
            let hex_rows = ((y1 - first_y) / row_step).ceil().max(0.0) as usize + 1;
            for row in 0..hex_rows {
                let cy = first_y + row as f64 * row_step;
                let stagger = if row % 2 == 0 { 0.0 } else { radius_x };
                for col in 0..cols {
                    let cx = first_x + col as f64 * step_x + stagger;
                    let points: Vec<_> = (0..6)
                        .map(|i| {
                            let angle = std::f64::consts::FRAC_PI_3 * i as f64;
                            (cx + radius_x * angle.cos(), cy + radius_y * angle.sin())
                        })
                        .collect();
                    for i in 0..6 {
                        line(points[i], points[(i + 1) % 6]);
                    }
                }
            }
        }
        PatternType::Dots => {
            let radius = step_x.min(step_y) * 0.14;
            for row in 0..rows {
                let cy = first_y + (row as f64 + 0.5) * step_y;
                for col in 0..cols {
                    if primitives.len() >= MAX_PRIMITIVES {
                        return None;
                    }
                    let cx = first_x + (col as f64 + 0.5) * step_x;
                    primitives.push(PatternPrimitive::Dot(AnchorPoint::new(cx, cy), radius));
                }
            }
        }
    }

    let angle = pattern.rotation.to_radians();
    if overflowed {
        // Grid/Brick/Hex hit the cap mid-fill: fail whole-pattern like
        // Dots instead of returning a silently truncated motif.
        return None;
    }
    if angle.abs() > f64::EPSILON {
        let (sin, cos) = angle.sin_cos();
        let cx = (x0 + x1) * 0.5;
        let cy = (y0 + y1) * 0.5;
        let rotate = |p: AnchorPoint| {
            let dx = p.x - cx;
            let dy = p.y - cy;
            AnchorPoint::new(cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
        };
        for primitive in &mut primitives {
            match primitive {
                PatternPrimitive::Line(a, b) => {
                    *a = rotate(*a);
                    *b = rotate(*b);
                }
                PatternPrimitive::Dot(center, _) => *center = rotate(*center),
            }
        }
    }
    Some(primitives)
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

#[cfg(test)]
mod pattern_tests {
    use super::*;

    #[test]
    fn pattern_types_generate_distinct_primitives_and_rotation_moves_them() {
        let bounds = (0.0, 0.0, 60.0, 40.0);
        let primitives: Vec<_> = [
            PatternType::Grid,
            PatternType::Hex,
            PatternType::Brick,
            PatternType::Dots,
        ]
        .into_iter()
        .map(|pattern_type| {
            pattern_primitives(
                &PatternFill {
                    pattern_type,
                    tile_width: 12.0,
                    tile_height: 10.0,
                    ..Default::default()
                },
                bounds,
            )
            .unwrap()
        })
        .collect();
        for i in 0..primitives.len() {
            for j in i + 1..primitives.len() {
                assert_ne!(primitives[i], primitives[j]);
            }
        }

        let unrotated = pattern_primitives(
            &PatternFill {
                pattern_type: PatternType::Brick,
                tile_width: 12.0,
                tile_height: 10.0,
                ..Default::default()
            },
            bounds,
        )
        .unwrap();
        let rotated = pattern_primitives(
            &PatternFill {
                rotation: 30.0,
                pattern_type: PatternType::Brick,
                tile_width: 12.0,
                tile_height: 10.0,
                ..Default::default()
            },
            bounds,
        )
        .unwrap();
        assert_ne!(unrotated, rotated);
    }

    #[test]
    fn pattern_geometry_rejects_invalid_sizes_and_excessive_output() {
        assert!(pattern_primitives(
            &PatternFill {
                tile_width: 0.0,
                ..Default::default()
            },
            (0.0, 0.0, 100.0, 100.0),
        )
        .is_none());
        assert!(pattern_primitives(
            &PatternFill {
                tile_width: 0.001,
                tile_height: 0.001,
                ..Default::default()
            },
            (0.0, 0.0, 100.0, 100.0),
        )
        .is_none());
    }

    #[test]
    fn frame_sampling_preserves_coverage_within_budget() {
        let full = pattern_primitives(
            &PatternFill {
                pattern_type: PatternType::Dots,
                tile_width: 2.0,
                tile_height: 2.0,
                ..Default::default()
            },
            (0.0, 0.0, 200.0, 200.0),
        )
        .unwrap();
        assert!(full.len() > PATTERN_FRAME_BUDGET);
        let sampled = sample_pattern_primitives_for_frame(&full, PATTERN_FRAME_BUDGET);
        assert!(sampled.len() <= PATTERN_FRAME_BUDGET);
        assert!(sampled.len() > PATTERN_FRAME_BUDGET / 2);
        // Stride keeps first and last regions instead of only the top.
        assert_eq!(sampled.first(), full.first());
        assert_ne!(sampled.last().unwrap(), &full[sampled.len() - 1]);
        assert!(full
            .iter()
            .skip(full.len() / 2)
            .any(|p| sampled.contains(p)));
        assert!(sample_pattern_primitives_for_frame(&full, 0).is_empty());
        assert_eq!(
            sample_pattern_primitives_for_frame(&full[..10], PATTERN_FRAME_BUDGET).len(),
            10.min(full.len())
        );
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ImageTileMode {
    #[serde(rename = "cover")]
    #[default]
    Cover,
    #[serde(rename = "contain")]
    Contain,
    #[serde(rename = "fit")]
    Fit,
    #[serde(rename = "tile")]
    Tile,
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
    /// Print overprint (knockout when false). Honored by print PDF export
    /// and reported by preflight; canvas preview ignores it.
    #[serde(default)]
    pub overprint: bool,
    /// Spot color name from the document's spot library. `None` = process
    /// color. Resolved at export/preview time so renames stay live.
    #[serde(default)]
    pub spot: Option<String>,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 1.0],
            fill_type: FillType::Solid([0.0, 0.0, 0.0, 1.0]),
            rule: FillRule::NonZero,
            overprint: false,
            spot: None,
        }
    }
}

impl FillStyle {
    pub fn solid(color: [f32; 4]) -> Self {
        Self {
            color,
            fill_type: FillType::Solid(color),
            rule: FillRule::NonZero,
            overprint: false,
            spot: None,
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
            overprint: false,
            spot: None,
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
            overprint: false,
            spot: None,
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
            overprint: false,
            spot: None,
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
    /// Print overprint for the stroke (see `FillStyle::overprint`).
    #[serde(default)]
    pub overprint: bool,
    /// Spot color name from the document's spot library (`None` = process).
    #[serde(default)]
    pub spot: Option<String>,
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
            overprint: false,
            spot: None,
        }
    }
}
