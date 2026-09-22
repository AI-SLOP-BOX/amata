use super::style::{AnchorPoint, BezierSegment, FillStyle, PathElement, StrokeStyle};
use serde::{Deserialize, Serialize};

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
        self.elements
            .push(PathElement::MoveTo(AnchorPoint::new(x, y)));
    }

    pub fn push_line_to(&mut self, x: f64, y: f64) {
        self.elements
            .push(PathElement::LineTo(AnchorPoint::new(x, y)));
    }

    pub fn push_curve_to(&mut self, c1: AnchorPoint, c2: AnchorPoint, end: AnchorPoint) {
        let start = self.last_point().unwrap_or(AnchorPoint::new(0.0, 0.0));
        self.elements
            .push(PathElement::CurveTo(BezierSegment::cubic(
                start, c1, c2, end,
            )));
    }

    pub fn push_cubic_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        self.push_curve_to(
            AnchorPoint::new(x1, y1),
            AnchorPoint::new(x2, y2),
            AnchorPoint::new(x3, y3),
        );
    }

    pub fn push_quad_curve_to(&mut self, x1: f64, y1: f64, x: f64, y: f64) {
        let p0 = self.last_point().unwrap_or(AnchorPoint::new(0.0, 0.0));
        let c1x = p0.x + (2.0 / 3.0) * (x1 - p0.x);
        let c1y = p0.y + (2.0 / 3.0) * (y1 - p0.y);
        let c2x = x + (2.0 / 3.0) * (x1 - x);
        let c2y = y + (2.0 / 3.0) * (y1 - y);
        self.push_cubic_curve_to(c1x, c1y, c2x, c2y, x, y);
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

    /// Number of anchors (MoveTo / LineTo / CurveTo endpoints; ClosePath excluded).
    pub fn anchor_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| !matches!(e, PathElement::ClosePath))
            .count()
    }

    /// Endpoint of element `idx`, if it carries an anchor.
    pub fn element_anchor(&self, idx: usize) -> Option<AnchorPoint> {
        match self.elements.get(idx)? {
            PathElement::MoveTo(p) | PathElement::LineTo(p) => Some(*p),
            PathElement::CurveTo(seg) => Some(seg.end),
            PathElement::ClosePath => None,
        }
    }

    /// Split the segment ending at `elem_idx` at parameter `t` (0..1),
    /// inserting a new anchor between the neighbors. Lines split by
    /// lerp; cubics split exactly via de Casteljau.
    pub fn split_segment(&mut self, elem_idx: usize, t: f64) -> bool {
        if elem_idx == 0 || elem_idx >= self.elements.len() {
            return false;
        }
        let t = t.clamp(0.01, 0.99);
        match self.elements[elem_idx].clone() {
            PathElement::LineTo(p) => {
                let Some(prev) = self.element_anchor(elem_idx - 1) else {
                    return false;
                };
                let m = AnchorPoint::new(
                    prev.x + (p.x - prev.x) * t,
                    prev.y + (p.y - prev.y) * t,
                );
                self.elements[elem_idx] = PathElement::LineTo(m);
                self.elements.insert(elem_idx + 1, PathElement::LineTo(p));
                true
            }
            PathElement::CurveTo(seg) => {
                let lerp = |a: AnchorPoint, b: AnchorPoint, k: f64| {
                    AnchorPoint::new(a.x + (b.x - a.x) * k, a.y + (b.y - a.y) * k)
                };
                let p01 = lerp(seg.start, seg.control1, t);
                let p12 = lerp(seg.control1, seg.control2, t);
                let p23 = lerp(seg.control2, seg.end, t);
                let p012 = lerp(p01, p12, t);
                let p123 = lerp(p12, p23, t);
                let mid = lerp(p012, p123, t);
                self.elements[elem_idx] =
                    PathElement::CurveTo(BezierSegment::cubic(seg.start, p01, p012, mid));
                self.elements.insert(
                    elem_idx + 1,
                    PathElement::CurveTo(BezierSegment::cubic(mid, p123, p23, seg.end)),
                );
                true
            }
            PathElement::MoveTo(_) | PathElement::ClosePath => false,
        }
    }

    /// Remove the anchor at `elem_idx`, reconnecting its neighbors:
    /// the following segment starts at the previous anchor (its departure
    /// handle shifts along). Refuses when fewer than three anchors would
    /// remain, or when the path would be left with a single point.
    pub fn remove_anchor(&mut self, elem_idx: usize) -> bool {
        if elem_idx >= self.elements.len() {
            return false;
        }
        if matches!(self.elements[elem_idx], PathElement::ClosePath) {
            return false;
        }
        if self.anchor_count() < 3 {
            return false;
        }

        if elem_idx == 0 {
            // Dropping the start anchor: promote the first segment's end
            // to MoveTo and discard that segment. For closed paths the
            // ClosePath now targets the promoted point automatically.
            if self.elements.len() < 2 {
                return false;
            }
            let Some(next) = self.element_anchor(1) else {
                return false;
            };
            self.elements[0] = PathElement::MoveTo(next);
            self.elements.remove(1);
            return true;
        }

        let Some(deleted) = self.element_anchor(elem_idx) else {
            return false;
        };
        self.elements.remove(elem_idx);
        if let Some(prev) = self.element_anchor(elem_idx - 1) {
            if let Some(PathElement::CurveTo(seg)) = self.elements.get_mut(elem_idx) {
                let dx = prev.x - deleted.x;
                let dy = prev.y - deleted.y;
                seg.start = prev;
                seg.control1.x += dx;
                seg.control1.y += dy;
            }
        }
        true
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

    /// Extract separate closed/open subpaths from PathElement sequences (handles holes/islands)
    pub fn to_subpaths(&self, segments_per_edge: u32) -> Vec<Vec<AnchorPoint>> {
        let mut subpaths = Vec::new();
        let mut current = Vec::new();

        for elem in &self.elements {
            match elem {
                PathElement::MoveTo(p) => {
                    if current.len() >= 2 {
                        subpaths.push(current);
                    }
                    current = vec![*p];
                }
                PathElement::LineTo(p) => {
                    current.push(*p);
                }
                PathElement::CurveTo(seg) => {
                    let n = segments_per_edge.max(1);
                    for i in 1..=n {
                        let t = i as f64 / n as f64;
                        current.push(seg.eval(t));
                    }
                }
                PathElement::ClosePath => {
                    if current.len() >= 3 {
                        // Remove trailing duplicate of first vertex if present
                        if let Some(first) = current.first().copied() {
                            if current.len() > 1 && current.last() == Some(&first) {
                                current.pop();
                            }
                        }
                        if current.len() >= 3 {
                            subpaths.push(current);
                        }
                        current = Vec::new();
                    }
                }
            }
        }
        if current.len() >= 3 {
            if let Some(first) = current.first().copied() {
                if current.last() == Some(&first) {
                    current.pop();
                }
            }
            if current.len() >= 3 {
                subpaths.push(current);
            }
        }
        subpaths
    }

    /// Triangulate path into triangle vertices (supports holes and concave geometries via Ear-Clipping and EvenOdd rules)
    pub fn to_triangles(&self, segments_per_edge: u32) -> Vec<[AnchorPoint; 3]> {
        let subpaths = self.to_subpaths(segments_per_edge);
        if subpaths.is_empty() {
            return Vec::new();
        }

        if subpaths.len() == 1 {
            let poly = &subpaths[0];
            let tris = crate::core::mesh3d::triangulate_polygon(poly);
            return tris
                .into_iter()
                .map(|[i0, i1, i2]| [poly[i0], poly[i1], poly[i2]])
                .collect();
        }

        // Multiple subpaths (Outer boundary + inner holes / multiple islands)
        // Group into outer and inner holes based on area and containment
        let mut result = Vec::new();
        for (idx, path_a) in subpaths.iter().enumerate() {
            let is_hole = subpaths.iter().enumerate().any(|(j, path_b)| {
                if idx == j || path_b.len() < 3 {
                    false
                } else {
                    crate::core::geometry::point_in_polygon(path_a[0].x, path_a[0].y, path_b)
                }
            });

            if !is_hole {
                // Find all immediate child holes inside path_a
                let mut holes: Vec<&Vec<AnchorPoint>> = Vec::new();
                for (j, path_b) in subpaths.iter().enumerate() {
                    if idx != j
                        && path_b.len() >= 3
                        && crate::core::geometry::point_in_polygon(path_b[0].x, path_b[0].y, path_a)
                    {
                        holes.push(path_b);
                    }
                }

                if holes.is_empty() {
                    let tris = crate::core::mesh3d::triangulate_polygon(path_a);
                    for [i0, i1, i2] in tris {
                        result.push([path_a[i0], path_a[i1], path_a[i2]]);
                    }
                } else {
                    // Bridge holes into a single polygon for ear clipping
                    let mut bridged = path_a.clone();
                    for hole in holes {
                        // Find closest pair between bridged and hole
                        let mut min_d = f64::MAX;
                        let mut best_b_idx = 0;
                        let mut best_h_idx = 0;
                        for (bi, bp) in bridged.iter().enumerate() {
                            for (hi, hp) in hole.iter().enumerate() {
                                let d = bp.distance(*hp);
                                if d < min_d {
                                    min_d = d;
                                    best_b_idx = bi;
                                    best_h_idx = hi;
                                }
                            }
                        }
                        // Insert hole vertices reversed at best_b_idx
                        let mut hole_cycle = Vec::new();
                        let hn = hole.len();
                        for step in 0..hn {
                            let h_idx = (best_h_idx + hn - (step % hn)) % hn;
                            hole_cycle.push(hole[h_idx]);
                        }
                        hole_cycle.push(hole[best_h_idx]);
                        hole_cycle.push(bridged[best_b_idx]);

                        bridged.splice(best_b_idx + 1..best_b_idx + 1, hole_cycle);
                    }

                    let tris = crate::core::mesh3d::triangulate_polygon(&bridged);
                    for [i0, i1, i2] in tris {
                        result.push([bridged[i0], bridged[i1], bridged[i2]]);
                    }
                }
            }
        }

        result
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
        Some((
            AnchorPoint::new(min_x, min_y),
            AnchorPoint::new(max_x, max_y),
        ))
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
    pub fn from_star(
        points: usize,
        inner_radius: f64,
        outer_radius: f64,
        cx: f64,
        cy: f64,
    ) -> Self {
        let mut path = Self::new();
        let points = points.max(3);
        let total_vertices = points * 2;
        let angle_step = std::f64::consts::TAU / total_vertices as f64;
        let start_angle = -std::f64::consts::FRAC_PI_2;

        for i in 0..total_vertices {
            let r = if i % 2 == 0 {
                outer_radius
            } else {
                inner_radius
            };
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

            let c1 = AnchorPoint::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
            let c2 = AnchorPoint::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);
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
