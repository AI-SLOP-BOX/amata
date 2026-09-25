//! Painting tessellated strokes on the canvas.
//!
//! [`crate::core::stroke_tess`] builds the outline of a stroke (dashes, caps,
//! joins, miter limit, arrowheads) as rings in document coordinates; this
//! module maps them to screen space and fills them with proper anti-aliasing.
//!
//! The obvious implementation — one `PathShape::convex_polygon` per triangle,
//! the way the fill code does it — would leave a seam down every stroke.
//! epaint feathers *every* edge of every shape it is handed, so two adjacent
//! triangles overlap in a 1 px band whose combined alpha only reaches ~75 %;
//! on a 4 px stroke that reads as a hairline running along the middle of the
//! line.  Instead the interior is triangulated once onto shared vertices and
//! only the true boundary gets a feathering strip.

use crate::core::path::{AnchorPoint, PathData, StrokeStyle};
use crate::core::stroke_tess;
use egui::epaint::{Mesh, Shape};
use egui::{Color32, Pos2};

/// Push a vertex and return its index (`Mesh::colored_vertex` doesn't).
fn add_vertex(mesh: &mut Mesh, pos: Pos2, color: Color32) -> u32 {
    let idx = mesh.vertices.len() as u32;
    mesh.colored_vertex(pos, color);
    idx
}

/// Width of the anti-aliasing band, in points — the same 1 px epaint uses.
const FEATHER: f32 = 1.0;

/// Stroke colour with the object's (and its ancestors') opacity folded in.
pub fn stroke_color(style: &StrokeStyle, opacity: f32) -> Color32 {
    let c = style.color;
    Color32::from_rgba_unmultiplied(
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
        (c[3] * opacity * 255.0) as u8,
    )
}

/// Shoelace area; positive means counter-clockwise (in the usual math sense).
fn signed_area(poly: &[Pos2]) -> f32 {
    let n = poly.len();
    let mut a = 0.0_f32;
    for i in 0..n {
        let j = (i + 1) % n;
        a += poly[i].x * poly[j].y - poly[j].x * poly[i].y;
    }
    a * 0.5
}

/// Ray-cast containment test (exact vertex hits count as inside, which is all
/// we need to decide "is this ring a hole in that one").
fn contains(poly: &[Pos2], p: Pos2) -> bool {
    let n = poly.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x;
            if p.x < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Outward unit normal of the edge `a -> b` of a counter-clockwise ring.
fn edge_outward(a: Pos2, b: Pos2) -> (f32, f32) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l = (dx * dx + dy * dy).sqrt();
    if l < 1e-6 {
        (0.0, 0.0)
    } else {
        (dy / l, -dx / l)
    }
}

/// Fill a set of rings with a single feathered mesh.
///
/// Rings may be nested: a ring contained in another is treated as a hole, so
/// a closed stroke's outer/inner pair fills as an annulus rather than a blob.
pub fn fill_rings(painter: &egui::Painter, rings: &[Vec<Pos2>], color: Color32) {
    let mesh = build_mesh(rings, color);
    if !mesh.vertices.is_empty() {
        painter.add(Shape::mesh(mesh));
    }
}

/// [`fill_rings`] without the painting step, so tests can inspect the mesh.
pub fn build_mesh(rings: &[Vec<Pos2>], color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    if color.a() == 0 {
        return mesh;
    }
    let mut polys: Vec<Vec<Pos2>> = rings.iter().filter(|r| r.len() >= 3).cloned().collect();
    if polys.is_empty() {
        return mesh;
    }
    // One orientation for everything, so "outward" is well defined.
    for p in polys.iter_mut() {
        if signed_area(p) < 0.0 {
            p.reverse();
        }
    }
    let is_hole: Vec<bool> = (0..polys.len())
        .map(|i| {
            let probe = polys[i][0];
            (0..polys.len()).any(|j| i != j && contains(&polys[j], probe))
        })
        .collect();

    // Per-ring boundary strips, plus the inset polygon the interior is
    // triangulated from.  Insetting by half the feather and then feathering
    // back out reproduces epaint's centre-on-the-boundary ramp exactly, and
    // keeps interior vertices and strip vertices at identical positions so
    // nothing shows through the join.
    let mut inset: Vec<Vec<Pos2>> = Vec::with_capacity(polys.len());
    let mut outset: Vec<Vec<Pos2>> = Vec::with_capacity(polys.len());
    let half = FEATHER * 0.5;
    for (i, poly) in polys.iter().enumerate() {
        let n = poly.len();
        let sign = if is_hole[i] { -1.0 } else { 1.0 };
        let mut inner = Vec::with_capacity(n);
        let mut outer = Vec::with_capacity(n);
        for k in 0..n {
            let prev = poly[(k + n - 1) % n];
            let cur = poly[k];
            let next = poly[(k + 1) % n];
            let (pa, ca) = (edge_outward(prev, cur), edge_outward(cur, next));
            let (mut sx, mut sy) = ((pa.0 + ca.0) * sign, (pa.1 + ca.1) * sign);
            let l = (sx * sx + sy * sy).sqrt();
            if l < 1e-6 {
                (sx, sy) = (ca.0 * sign, ca.1 * sign);
            } else {
                sx /= l;
                sy /= l;
            }
            inner.push(Pos2::new(cur.x - sx * half, cur.y - sy * half));
            outer.push(Pos2::new(cur.x + sx * half, cur.y + sy * half));
        }
        inset.push(inner);
        outset.push(outer);
    }

    // Interior: one shared vertex per boundary point, no per-triangle edges.
    let mut path = PathData::new();
    for poly in &inset {
        path.push_move_to(poly[0].x as f64, poly[0].y as f64);
        for p in &poly[1..] {
            path.push_line_to(p.x as f64, p.y as f64);
        }
    }
    for tri in path.to_triangles(1) {
        let idx = tri.map(|v| add_vertex(&mut mesh, Pos2::new(v.x as f32, v.y as f32), color));
        mesh.add_triangle(idx[0], idx[1], idx[2]);
    }

    // Boundary: colour inside, transparent outside.
    for (i, _) in polys.iter().enumerate() {
        let n = polys[i].len();
        for k in 0..n {
            let m = (k + 1) % n;
            let ik = add_vertex(&mut mesh, inset[i][k], color);
            let im = add_vertex(&mut mesh, inset[i][m], color);
            let ok = add_vertex(&mut mesh, outset[i][k], Color32::TRANSPARENT);
            let om = add_vertex(&mut mesh, outset[i][m], Color32::TRANSPARENT);
            if is_hole[i] {
                mesh.add_triangle(ik, im, ok);
                mesh.add_triangle(ok, im, om);
            } else {
                mesh.add_triangle(ik, ok, im);
                mesh.add_triangle(im, ok, om);
            }
        }
    }

    mesh
}

/// Paint one subpath's stroke — dash pattern, caps, joins, miter limit and any
/// arrowheads — into `painter`.
///
/// `to_screen` must be the same transform the object's fill uses, so the
/// stroke hugs the fill at every zoom and rotation.
pub fn paint_stroke(
    painter: &egui::Painter,
    style: &StrokeStyle,
    pts: &[AnchorPoint],
    closed: bool,
    opacity: f32,
    to_screen: &impl Fn(f64, f64) -> Pos2,
) {
    let color = stroke_color(style, opacity);
    if color.a() == 0 {
        return;
    }
    let mut rings: Vec<Vec<Pos2>> = stroke_tess::stroke_rings(pts, closed, style)
        .into_iter()
        .map(|r| r.iter().map(|p| to_screen(p.x, p.y)).collect())
        .collect();
    for head in stroke_tess::arrowhead_rings(pts, closed, style) {
        rings.push(head.iter().map(|p| to_screen(p.x, p.y)).collect());
    }
    fill_rings(painter, &rings, color);
}

/// [`paint_stroke`] for every subpath of a [`PathData`].
pub fn paint_path_stroke(
    painter: &egui::Painter,
    path: &PathData,
    style: &StrokeStyle,
    opacity: f32,
    to_screen: &impl Fn(f64, f64) -> Pos2,
) {
    for subpath in path.to_subpaths(16) {
        paint_stroke(painter, style, &subpath, path.closed, opacity, to_screen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hole_rings_are_detected_by_containment() {
        let outer = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(20.0, 0.0),
            Pos2::new(20.0, 20.0),
            Pos2::new(0.0, 20.0),
        ];
        let inner = vec![
            Pos2::new(5.0, 5.0),
            Pos2::new(15.0, 5.0),
            Pos2::new(15.0, 15.0),
            Pos2::new(5.0, 15.0),
        ];
        assert!(contains(&outer, Pos2::new(10.0, 10.0)));
        assert!(!contains(&outer, Pos2::new(30.0, 10.0)));
        assert!(contains(&outer, inner[0]), "inner ring starts inside outer");
        assert!(
            !contains(&inner, outer[0]),
            "outer ring corner is outside inner"
        );
        assert!(signed_area(&outer) > 0.0);
        assert!(signed_area(&inner) > 0.0);
    }

    #[test]
    fn clockwise_rings_are_normalised_before_orientation_is_used() {
        let mut cw = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(0.0, 10.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(10.0, 0.0),
        ];
        assert!(signed_area(&cw) < 0.0);
        cw.reverse();
        assert!(signed_area(&cw) > 0.0);
    }

    #[test]
    fn annulus_fills_with_a_hole_not_a_blob() {
        let outer = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(20.0, 0.0),
            Pos2::new(20.0, 20.0),
            Pos2::new(0.0, 20.0),
        ];
        let inner = vec![
            Pos2::new(8.0, 8.0),
            Pos2::new(12.0, 8.0),
            Pos2::new(12.0, 12.0),
            Pos2::new(8.0, 12.0),
        ];
        let mesh = build_mesh(&[outer, inner], Color32::BLACK);
        let total_tris = mesh.indices.len() / 3;
        // Four edges per ring, two feather triangles each.
        let interior = total_tris - 16;
        assert!(
            interior >= 6,
            "expected a bridged annulus, got {interior} interior triangles"
        );

        let v = |i: usize| mesh.vertices[mesh.indices[i] as usize].pos;
        let covers_hole = (0..interior * 3)
            .step_by(3)
            .any(|t| tri_contains(v(t), v(t + 1), v(t + 2), Pos2::new(10.0, 10.0)));
        assert!(!covers_hole, "the hole must stay empty");
    }

    #[test]
    fn disjoint_rings_are_both_filled() {
        let a = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(0.0, 10.0),
        ];
        let b: Vec<Pos2> = a.iter().map(|p| Pos2::new(p.x + 50.0, p.y)).collect();
        let mesh = build_mesh(&[a, b], Color32::BLACK);
        let interior = mesh.indices.len() / 3 - 16;
        assert_eq!(interior, 4, "two independent quads, two triangles each");
    }

    fn tri_contains(a: Pos2, b: Pos2, c: Pos2, p: Pos2) -> bool {
        let sign =
            |o: Pos2, q: Pos2, r: Pos2| (q.x - o.x) * (r.y - o.y) - (q.y - o.y) * (r.x - o.x);
        // Degenerate (zero-area) triangles are just bridge artefacts; they
        // "contain" everything under a naive sign test.
        if sign(a, b, c).abs() < 1e-6 {
            return false;
        }
        let d1 = sign(p, a, b);
        let d2 = sign(p, b, c);
        let d3 = sign(p, c, a);
        let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(neg && pos)
    }

    #[test]
    fn fill_rings_emits_interior_and_feather_geometry() {
        // A renderer-less check that a simple ring produces a non-empty mesh:
        // `fill_rings` only needs something implementing `Painter`, and the
        // egui one is available without a display.
        let ctx = egui::Context::default();
        let output = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let painter = ui.painter();
                let ring = vec![
                    Pos2::new(10.0, 10.0),
                    Pos2::new(110.0, 10.0),
                    Pos2::new(110.0, 20.0),
                    Pos2::new(10.0, 20.0),
                ];
                fill_rings(painter, &[ring], Color32::BLACK);
            });
        });
        let shapes = ctx.tessellate(output.shapes, 1.0);
        assert!(!shapes.is_empty(), "stroke produced no primitives");
        let verts: usize = shapes
            .iter()
            .filter_map(|p| match &p.primitive {
                egui::epaint::Primitive::Mesh(m) => Some(m.vertices.len()),
                _ => None,
            })
            .sum();
        assert!(
            verts > 16,
            "expected interior + feather vertices, got {verts}"
        );
    }
}
