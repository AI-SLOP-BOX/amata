//! Polygon-accurate clipping for clipping masks.
//!
//! SVG hands a clip away for free — the exporter already emits a real
//! `<clipPath>`.  On the canvas every shape is its own mesh, so the clip has
//! to be applied to the geometry: [`egui::Painter::with_clip_rect`] takes care
//! of the bounding box (and is *exact* for the axis-aligned rectangle mask,
//! which is the common case), while fills and strokes additionally run through
//! [`ClipRegion`] so a circular mask actually cuts along the circle.

use crate::core::boolean::{apply_polygon_boolean, BooleanOp};
use crate::core::mesh3d::triangulate_polygon;
use crate::core::path::AnchorPoint;
use egui::{Pos2, Rect};

/// How far a point may sit off the box before a mask stops counting as a
/// rectangle, in screen points.
const RECT_EPS: f32 = 1e-4;

/// A screen-space region that clipping-mask content is cut to.
///
/// An empty or degenerate mask is represented by `None` at the call site
/// rather than by an empty region: nothing is visible, so nothing is drawn.
#[derive(Debug, Clone)]
pub struct ClipRegion {
    poly: Vec<Pos2>,
    rect: Rect,
    /// `poly` is an axis-aligned rectangle exactly covering `rect`, so the
    /// bounding box alone already decides containment.
    is_rect: bool,
}

impl ClipRegion {
    /// Build a region from a mask polygon already in screen coordinates.
    ///
    /// Returns `None` when the polygon encloses no area — such a mask clips
    /// everything away, and callers should simply not draw.
    pub fn new(poly: Vec<Pos2>) -> Option<Self> {
        if poly.len() < 3 || shoelace(&poly).abs() < 1e-6 {
            return None;
        }
        let rect = bounds(&poly);
        let is_rect = is_axis_aligned_rect(&poly, rect);
        Some(Self {
            poly,
            rect,
            is_rect,
        })
    }

    /// Bounding box of the region — what the painter's clip rect is set to.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Narrow this region to `other`, for a clipping mask nested inside
    /// another one.  `None` means the two do not overlap, i.e. everything
    /// inside the inner mask is invisible.
    pub fn intersect(&self, other: &[Pos2]) -> Option<Self> {
        if other.len() < 3 || !self.rect.intersects(bounds(other)) {
            return None;
        }
        let out = apply_polygon_boolean(&to_aps(&self.poly), &to_aps(other), BooleanOp::Intersect);
        // Two simple polygons intersect in one polygon in practice; when the
        // result does split, keep the largest piece rather than dropping all
        // but an arbitrary one.
        let best = out.into_iter().max_by_key(|r| r.len())?;
        Self::new(best.iter().map(to_pos2).collect())
    }

    /// Cut a screen-space polygon against the region.
    pub fn clip_polygon(&self, poly: &[Pos2]) -> Vec<Vec<Pos2>> {
        if poly.len() < 3 {
            return Vec::new();
        }
        let r = bounds(poly);
        if !r.intersects(self.rect) {
            return Vec::new();
        }
        if self.is_rect && self.rect.contains_rect(r) {
            return vec![poly.to_vec()];
        }
        apply_polygon_boolean(&to_aps(poly), &to_aps(&self.poly), BooleanOp::Intersect)
            .into_iter()
            .filter(|p| p.len() >= 3)
            .map(|p| p.iter().map(to_pos2).collect())
            .collect()
    }

    /// Cut a screen-space triangle, returning the triangles that survive.
    ///
    /// Untouched triangles come back as themselves so the usual path pays
    /// nothing for the clip.
    pub fn clip_triangle(&self, tri: [Pos2; 3]) -> Vec<[Pos2; 3]> {
        let r = bounds(&tri);
        if !r.intersects(self.rect) {
            return Vec::new();
        }
        if self.is_rect && self.rect.contains_rect(r) {
            return vec![tri];
        }
        let mut out = Vec::new();
        for poly in self.clip_polygon(&tri) {
            let aps = to_aps(&poly);
            for t in triangulate_polygon(&aps) {
                out.push([poly[t[0]], poly[t[1]], poly[t[2]]]);
            }
        }
        out
    }
}

/// Triangles to emit for `tri` under an optional clip.
pub fn clipped_triangles(clip: Option<&ClipRegion>, tri: [Pos2; 3]) -> Vec<[Pos2; 3]> {
    match clip {
        Some(c) => c.clip_triangle(tri),
        None => vec![tri],
    }
}

/// Polygons to emit for `poly` under an optional clip.
pub fn clipped_polygons(clip: Option<&ClipRegion>, poly: &[Pos2]) -> Vec<Vec<Pos2>> {
    match clip {
        Some(c) => c.clip_polygon(poly),
        None => vec![poly.to_vec()],
    }
}

/// Paint a filled polygon, cut by `clip` when one is active.
///
/// With no clip this is byte-for-byte the `convex_polygon` call the canvas
/// always used, so the ordinary path is untouched.  Under a clip the
/// intersection is triangulated rather than fanned: cutting a convex shape
/// along a mask can leave a non-convex piece.
pub fn paint_fill(
    painter: &egui::Painter,
    poly: &[Pos2],
    color: egui::Color32,
    clip: Option<&ClipRegion>,
) {
    match clip {
        None => add_convex(painter, poly, color),
        Some(c) => {
            for piece in c.clip_polygon(poly) {
                if piece.len() == 3 {
                    add_convex(painter, &piece, color);
                    continue;
                }
                for t in triangulate_polygon(&to_aps(&piece)) {
                    add_convex(painter, &[piece[t[0]], piece[t[1]], piece[t[2]]], color);
                }
            }
        }
    }
}

fn add_convex(painter: &egui::Painter, poly: &[Pos2], color: egui::Color32) {
    painter.add(egui::epaint::PathShape::convex_polygon(
        poly.to_vec(),
        color,
        egui::Stroke::NONE,
    ));
}

fn to_aps(poly: &[Pos2]) -> Vec<AnchorPoint> {
    poly.iter()
        .map(|p| AnchorPoint::new(p.x as f64, p.y as f64))
        .collect()
}

fn to_pos2(p: &AnchorPoint) -> Pos2 {
    Pos2::new(p.x as f32, p.y as f32)
}

fn bounds(poly: &[Pos2]) -> Rect {
    let mut min = poly[0];
    let mut max = poly[0];
    for p in &poly[1..] {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    Rect::from_min_max(min, max)
}

fn shoelace(poly: &[Pos2]) -> f32 {
    let n = poly.len();
    let mut a = 0.0_f32;
    for i in 0..n {
        let j = (i + 1) % n;
        a += poly[i].x * poly[j].y - poly[j].x * poly[i].y;
    }
    a * 0.5
}

/// True when `poly` is the four corners of `rect` walked along its edges.
fn is_axis_aligned_rect(poly: &[Pos2], rect: Rect) -> bool {
    if poly.len() != 4 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return false;
    }
    for i in 0..4 {
        let a = poly[i];
        let b = poly[(i + 1) % 4];
        if (a.x - b.x).abs() > RECT_EPS && (a.y - b.y).abs() > RECT_EPS {
            return false; // diagonal edge
        }
        let on_x = (a.x - rect.min.x).abs() <= RECT_EPS || (a.x - rect.max.x).abs() <= RECT_EPS;
        let on_y = (a.y - rect.min.y).abs() <= RECT_EPS || (a.y - rect.max.y).abs() <= RECT_EPS;
        if !on_x && !on_y {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_poly(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Pos2> {
        vec![
            Pos2::new(x0, y0),
            Pos2::new(x1, y0),
            Pos2::new(x1, y1),
            Pos2::new(x0, y1),
        ]
    }

    fn area(tri: &[Pos2; 3]) -> f32 {
        ((tri[1].x - tri[0].x) * (tri[2].y - tri[0].y)
            - (tri[1].y - tri[0].y) * (tri[2].x - tri[0].x))
            * 0.5
    }

    #[test]
    fn a_rectangular_mask_is_detected_as_such() {
        let region = ClipRegion::new(rect_poly(10.0, 10.0, 110.0, 60.0)).unwrap();
        assert!(region.is_rect);
        assert_eq!(
            region.rect(),
            Rect::from_min_max(Pos2::new(10.0, 10.0), Pos2::new(110.0, 60.0))
        );
    }

    #[test]
    fn degenerate_masks_clip_everything_away() {
        assert!(ClipRegion::new(vec![]).is_none());
        assert!(
            ClipRegion::new(rect_poly(0.0, 0.0, 0.0, 0.0).into_iter().take(3).collect()).is_none()
        );
        // A zero-width sliver has no area either.
        assert!(ClipRegion::new(vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(100.0, 0.0),
            Pos2::new(100.0, 0.0),
            Pos2::new(0.0, 0.0)
        ])
        .is_none());
    }

    #[test]
    fn triangles_inside_survive_unchanged_and_outside_vanish() {
        let region = ClipRegion::new(rect_poly(0.0, 0.0, 50.0, 50.0)).unwrap();
        let inside = [
            Pos2::new(1.0, 1.0),
            Pos2::new(10.0, 1.0),
            Pos2::new(1.0, 10.0),
        ];
        assert_eq!(region.clip_triangle(inside), vec![inside]);

        let outside = [
            Pos2::new(60.0, 60.0),
            Pos2::new(70.0, 60.0),
            Pos2::new(60.0, 70.0),
        ];
        assert!(region.clip_triangle(outside).is_empty());
    }

    #[test]
    fn a_straddling_triangle_is_cut_down_to_the_mask() {
        let region = ClipRegion::new(rect_poly(0.0, 0.0, 50.0, 50.0)).unwrap();
        let tri = [
            Pos2::new(25.0, 25.0),
            Pos2::new(250.0, 25.0),
            Pos2::new(25.0, 250.0),
        ];
        let kept: Vec<[Pos2; 3]> = region.clip_triangle(tri);
        let total: f32 = kept.iter().map(area).sum();
        // The hypotenuse leaves a 50 x 50 corner triangle inside the mask,
        // of which only the part beyond x/y = 50 is cut off.
        assert!(total > 0.0, "clipped triangle must keep some area");
        assert!(
            total < area(&tri).abs(),
            "clipping must remove area, got {total} of {}",
            area(&tri).abs()
        );
        for t in &kept {
            for p in t {
                assert!(
                    p.x <= 50.0 + 1e-3 && p.y <= 50.0 + 1e-3,
                    "vertex {p:?} escaped the mask"
                );
            }
        }
    }

    #[test]
    fn a_circular_mask_cuts_along_the_circle() {
        // Octagon inscribed in a 100 pt box — not a rectangle, so the
        // polygon path has to do the work.
        let oct: Vec<Pos2> = (0..8)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 8.0 + std::f32::consts::FRAC_PI_8;
                Pos2::new(50.0 + 50.0 * a.cos(), 50.0 + 50.0 * a.sin())
            })
            .collect();
        let region = ClipRegion::new(oct).expect("octagon has area");
        assert!(!region.is_rect);

        // Reaches into the octagon's bounding box, so the boolean has to
        // decide — and the corner it holds is outside the octagon itself.
        let outside = [
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(0.0, 10.0),
        ];
        assert!(
            region.clip_triangle(outside).is_empty(),
            "the box corner lies outside the inscribed octagon"
        );

        let centre = [
            Pos2::new(48.0, 48.0),
            Pos2::new(52.0, 48.0),
            Pos2::new(48.0, 52.0),
        ];
        assert_eq!(region.clip_triangle(centre), vec![centre]);
    }

    #[test]
    fn nested_masks_narrow_to_the_overlap() {
        let outer = ClipRegion::new(rect_poly(0.0, 0.0, 100.0, 100.0)).unwrap();
        let inner = rect_poly(50.0, 50.0, 200.0, 200.0);
        let both = outer.intersect(&inner).expect("the squares overlap");
        assert_eq!(
            both.rect(),
            Rect::from_min_max(Pos2::new(50.0, 50.0), Pos2::new(100.0, 100.0))
        );

        let disjoint = rect_poly(300.0, 300.0, 400.0, 400.0);
        assert!(outer.intersect(&disjoint).is_none());
    }

    #[test]
    fn no_clip_is_a_passthrough() {
        let tri = [
            Pos2::new(0.0, 0.0),
            Pos2::new(4.0, 0.0),
            Pos2::new(0.0, 4.0),
        ];
        assert_eq!(clipped_triangles(None, tri), vec![tri]);
        assert_eq!(clipped_polygons(None, &tri), vec![tri.to_vec()]);
    }
}
