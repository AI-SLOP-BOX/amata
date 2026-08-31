use serde::{Deserialize, Serialize};

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
            u3 * self.start.x + 3.0 * u2 * t * self.control1.x + 3.0 * u * t2 * self.control2.x + t3 * self.end.x,
            u3 * self.start.y + 3.0 * u2 * t * self.control1.y + 3.0 * u * t2 * self.control2.y + t3 * self.end.y,
        )
    }

    pub fn to_kurbo(self) -> kurbo::CubicBez {
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
                GradientStop { offset: 0.0, color: [0.2, 0.6, 1.0, 1.0] },
                GradientStop { offset: 1.0, color: [0.8, 0.2, 0.9, 1.0] },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FillType {
    Solid([f32; 4]),
    Linear(LinearGradient),
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
        let first_color = gradient.stops.first().map(|s| s.color).unwrap_or([0.0, 0.0, 0.0, 1.0]);
        Self {
            color: first_color,
            fill_type: FillType::Linear(gradient),
            rule: FillRule::NonZero,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrokeStyle {
    pub color: [f32; 4],
    pub width: f64,
    pub dash_pattern: Option<Vec<f64>>,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 1.0],
            width: 1.0,
            dash_pattern: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathData {
    pub elements: Vec<PathElement>,
    pub fill: Option<FillStyle>,
    pub stroke: Option<StrokeStyle>,
    pub closed: bool,
}

impl Default for PathData {
    fn default() -> Self {
        Self::new()
    }
}

impl PathData {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            fill: Some(FillStyle::default()),
            stroke: None,
            closed: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn push_move_to(&mut self, x: f64, y: f64) {
        self.elements.push(PathElement::MoveTo(AnchorPoint::new(x, y)));
    }

    pub fn push_line_to(&mut self, x: f64, y: f64) {
        self.elements.push(PathElement::LineTo(AnchorPoint::new(x, y)));
    }

    pub fn push_curve_to(&mut self, c1: AnchorPoint, c2: AnchorPoint, end: AnchorPoint) {
        let start = self.last_point().unwrap_or(AnchorPoint::new(0.0, 0.0));
        self.elements
            .push(PathElement::CurveTo(BezierSegment::cubic(start, c1, c2, end)));
    }

    pub fn close(&mut self) {
        self.closed = true;
        self.elements.push(PathElement::ClosePath);
    }

    pub fn last_point(&self) -> Option<AnchorPoint> {
        for elem in self.elements.iter().rev() {
            match elem {
                PathElement::MoveTo(p) | PathElement::LineTo(p) => return Some(*p),
                PathElement::CurveTo(seg) => return Some(seg.end),
                PathElement::ClosePath => continue,
            }
        }
        None
    }

    pub fn to_polygon(&self, segments_per_edge: u32) -> Vec<AnchorPoint> {
        let mut points = Vec::new();

        for elem in &self.elements {
            match elem {
                PathElement::MoveTo(p) => {
                    points.push(*p);
                }
                PathElement::LineTo(p) => {
                    points.push(*p);
                }
                PathElement::CurveTo(seg) => {
                    let n = segments_per_edge.max(1);
                    for i in 1..=n {
                        let t = i as f64 / n as f64;
                        let pt = seg.eval(t);
                        points.push(pt);
                    }
                }
                PathElement::ClosePath => {
                    if let Some(first) = points.first().copied() {
                        if points.len() > 1 && points.last() != Some(&first) {
                            points.push(first);
                        }
                    }
                }
            }
        }
        points
    }

    pub fn bounding_box(&self) -> Option<(AnchorPoint, AnchorPoint)> {
        let poly = self.to_polygon(8);
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
        Some((AnchorPoint::new(min_x, min_y), AnchorPoint::new(max_x, max_y)))
    }

    pub fn transform(&mut self, matrix: &[f64; 6]) {
        for elem in &mut self.elements {
            match elem {
                PathElement::MoveTo(p) | PathElement::LineTo(p) => {
                    *p = transform_point(*p, matrix);
                }
                PathElement::CurveTo(seg) => {
                    seg.start = transform_point(seg.start, matrix);
                    seg.control1 = transform_point(seg.control1, matrix);
                    seg.control2 = transform_point(seg.control2, matrix);
                    seg.end = transform_point(seg.end, matrix);
                }
                PathElement::ClosePath => {}
            }
        }
    }

    /// Construct a regular polygon with N sides centered at (cx, cy)
    pub fn from_polygon(sides: usize, radius: f64, cx: f64, cy: f64) -> Self {
        let mut path = Self::new();
        let sides = sides.max(3);
        let angle_step = std::f64::consts::TAU / sides as f64;
        let start_angle = -std::f64::consts::FRAC_PI_2; // top vertex

        for i in 0..sides {
            let angle = start_angle + i as f64 * angle_step;
            let px = cx + radius * angle.cos();
            let py = cy + radius * angle.sin();
            if i == 0 {
                path.push_move_to(px, py);
            } else {
                path.push_line_to(px, py);
            }
        }
        path.close();
        path
    }

    /// Construct a star with N points, inner and outer radii, centered at (cx, cy)
    pub fn from_star(points: usize, inner_radius: f64, outer_radius: f64, cx: f64, cy: f64) -> Self {
        let mut path = Self::new();
        let points = points.max(3);
        let total_vertices = points * 2;
        let angle_step = std::f64::consts::TAU / total_vertices as f64;
        let start_angle = -std::f64::consts::FRAC_PI_2;

        for i in 0..total_vertices {
            let r = if i % 2 == 0 { outer_radius } else { inner_radius };
            let angle = start_angle + i as f64 * angle_step;
            let px = cx + r * angle.cos();
            let py = cy + r * angle.sin();
            if i == 0 {
                path.push_move_to(px, py);
            } else {
                path.push_line_to(px, py);
            }
        }
        path.close();
        path
    }

    /// Construct a rounded or standard rectangle
    pub fn from_rect(x: f64, y: f64, w: f64, h: f64, corner_radius: f64) -> Self {
        let mut path = Self::new();
        let r = corner_radius.clamp(0.0, (w.abs().min(h.abs())) / 2.0);
        if r <= 0.0 {
            path.push_move_to(x, y);
            path.push_line_to(x + w, y);
            path.push_line_to(x + w, y + h);
            path.push_line_to(x, y + h);
            path.close();
        } else {
            // Rounded corners with cubic beziers (k = 0.5522847498)
            let k = 0.5522847498 * r;
            path.push_move_to(x + r, y);
            path.push_line_to(x + w - r, y);
            path.push_curve_to(
                AnchorPoint::new(x + w - r + k, y),
                AnchorPoint::new(x + w, y + r - k),
                AnchorPoint::new(x + w, y + r),
            );
            path.push_line_to(x + w, y + h - r);
            path.push_curve_to(
                AnchorPoint::new(x + w, y + h - r + k),
                AnchorPoint::new(x + w - r + k, y + h),
                AnchorPoint::new(x + w - r, y + h),
            );
            path.push_line_to(x + r, y + h);
            path.push_curve_to(
                AnchorPoint::new(x + r - k, y + h),
                AnchorPoint::new(x, y + h - r + k),
                AnchorPoint::new(x, y + h - r),
            );
            path.push_line_to(x, y + r);
            path.push_curve_to(
                AnchorPoint::new(x, y + r - k),
                AnchorPoint::new(x + r - k, y),
                AnchorPoint::new(x + r, y),
            );
            path.close();
        }
        path
    }

    /// Construct an ellipse with center (cx, cy) and radii (rx, ry)
    pub fn from_ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Self {
        let mut path = Self::new();
        let kx = 0.5522847498 * rx;
        let ky = 0.5522847498 * ry;
        path.push_move_to(cx, cy - ry);
        path.push_curve_to(
            AnchorPoint::new(cx + kx, cy - ry),
            AnchorPoint::new(cx + rx, cy - ky),
            AnchorPoint::new(cx + rx, cy),
        );
        path.push_curve_to(
            AnchorPoint::new(cx + rx, cy + ky),
            AnchorPoint::new(cx + kx, cy + ry),
            AnchorPoint::new(cx, cy + ry),
        );
        path.push_curve_to(
            AnchorPoint::new(cx - kx, cy + ry),
            AnchorPoint::new(cx - rx, cy + ky),
            AnchorPoint::new(cx - rx, cy),
        );
        path.push_curve_to(
            AnchorPoint::new(cx - rx, cy - ky),
            AnchorPoint::new(cx - kx, cy - ry),
            AnchorPoint::new(cx, cy - ry),
        );
        path.close();
        path
    }

    /// Construct a straight line
    pub fn from_line(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let mut path = Self::new();
        path.push_move_to(x1, y1);
        path.push_line_to(x2, y2);
        path.fill = None;
        path.stroke = Some(StrokeStyle::default());
        path
    }

    /// Convert a list of polygon points into a closed/open PathData
    pub fn from_polygon_points(poly: &[AnchorPoint], closed: bool) -> Self {
        let mut path = Self::new();
        if poly.is_empty() {
            return path;
        }
        path.push_move_to(poly[0].x, poly[0].y);
        for p in &poly[1..] {
            path.push_line_to(p.x, p.y);
        }
        if closed {
            path.close();
        }
        path
    }

    /// Smooth freehand strokes into a cubic bezier curve using Catmull-Rom spline conversion
    pub fn from_smooth_points(pts: &[AnchorPoint]) -> Self {
        let mut path = Self::new();
        if pts.is_empty() {
            return path;
        }
        if pts.len() == 1 {
            path.push_move_to(pts[0].x, pts[0].y);
            return path;
        }
        if pts.len() == 2 {
            path.push_move_to(pts[0].x, pts[0].y);
            path.push_line_to(pts[1].x, pts[1].y);
            return path;
        }

        path.push_move_to(pts[0].x, pts[0].y);
        let n = pts.len();
        for i in 0..n - 1 {
            let p0 = if i == 0 { pts[0] } else { pts[i - 1] };
            let p1 = pts[i];
            let p2 = pts[i + 1];
            let p3 = if i + 2 < n { pts[i + 2] } else { pts[i + 1] };

            let c1 = AnchorPoint::new(
                p1.x + (p2.x - p0.x) / 6.0,
                p1.y + (p2.y - p0.y) / 6.0,
            );
            let c2 = AnchorPoint::new(
                p2.x - (p3.x - p1.x) / 6.0,
                p2.y - (p3.y - p1.y) / 6.0,
            );
            path.push_curve_to(c1, c2, p2);
        }
        path
    }
}

fn transform_point(p: AnchorPoint, m: &[f64; 6]) -> AnchorPoint {
    AnchorPoint::new(
        m[0] * p.x + m[2] * p.y + m[4],
        m[1] * p.x + m[3] * p.y + m[5],
    )
}
