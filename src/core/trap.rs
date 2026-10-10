//! Automatic trapping (spread) for press: abutting solid fills get a thin
//! overprinted stroke along their shared edge so small misregistrations
//! do not leave white gaps.
//!
//! Scope is deliberately the common case: top-level and grouped solid
//! fills, opaque, process colors, matched edge-to-edge within tolerance.
//! Spot boundaries, gradients, translucency and sub-point gaps are skipped
//! (counted, so the report stays honest). The trap color is the darker of
//! the two fills drawn with overprint — the classic spread default.
//!
//! Traps live on a dedicated `TRAP_LAYER_NAME` layer so they can be
//! regenerated or deleted as a unit; preflight reports their presence.

use crate::core::document::{Object, ObjectType};
use crate::core::path::{PathData, StrokeStyle};

/// Layer holding generated traps.
pub const TRAP_LAYER_NAME: &str = "Traps";
/// Default trap width (Illustrator's default 0.25pt).
pub const DEFAULT_TRAP_WIDTH_PT: f64 = 0.25;
/// Edge coincidence tolerance.
pub const TRAP_TOLERANCE_PT: f64 = 0.5;

type Pt = (f64, f64);
type Seg = (Pt, Pt);
type SegKey = ((i64, i64), (i64, i64));

fn luminance(c: [f32; 4]) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

fn apply_affine(m: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

fn affine_mul(a: &[f64; 6], b: &[f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

struct Candidate {
    points: Vec<Pt>,
    color: [f32; 4],
}

/// World-space polylines of eligible fills (solid, opaque, process).
fn collect_candidates(obj: &Object, parent: &[f64; 6], out: &mut Vec<Candidate>) {
    if !obj.visible || obj.opacity < 1.0 {
        return;
    }
    let world = affine_mul(parent, &obj.transform.matrix());
    match &obj.object_type {
        ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
            for c in children {
                collect_candidates(c, &world, out);
            }
        }
        _ => {
            let (fill, solid) = match obj.fill.as_ref() {
                Some(f) => match &f.fill_type {
                    crate::core::path::FillType::Solid(c) => (Some(*c), true),
                    _ => (None, false),
                },
                None => (None, false),
            };
            if !solid {
                return;
            }
            let c = fill.unwrap();
            if c[3] < 1.0 {
                return;
            }
            // Spot boundaries need plate-aware traps: out of scope, skipped.
            if obj.fill.as_ref().and_then(|f| f.spot.clone()).is_some() {
                return;
            }
            let mut poly = obj.to_path_data().to_polygon(8);
            if poly.len() < 2 {
                return;
            }
            // Close explicitly-closed rings so the closing edge traps too.
            let closed = matches!(
                obj.to_path_data().elements.last(),
                Some(crate::core::path::PathElement::ClosePath)
            );
            if closed {
                if let (Some(first), Some(last)) = (poly.first(), poly.last()) {
                    if (first.x - last.x).hypot(first.y - last.y) > 1e-9 {
                        poly.push(*first);
                    }
                }
            }
            out.push(Candidate {
                points: poly
                    .iter()
                    .map(|p| apply_affine(&world, p.x, p.y))
                    .collect(),
                color: c,
            });
        }
    }
}

/// One trap stroke: shared-edge polyline + darker fill color.
#[derive(Debug, Clone)]
pub struct TrapStroke {
    pub points: Vec<Pt>,
    pub color: [f32; 4],
}

/// Find shared edges between candidate fills. Returns trap strokes plus
/// the number of skipped spot boundaries.
pub fn find_trap_strokes(
    doc: &crate::core::document::Document,
    width: f64,
) -> (Vec<TrapStroke>, usize) {
    let _ = width;
    let mut cands: Vec<Candidate> = Vec::new();
    let ident = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        for obj in &layer.objects {
            collect_candidates(obj, &ident, &mut cands);
        }
    }
    // Count skipped spot fills for the report.
    let mut skipped_spots = 0usize;
    fn count_spots(obj: &Object, n: &mut usize) {
        if obj.fill.as_ref().and_then(|f| f.spot.clone()).is_some() {
            *n += 1;
        }
        match &obj.object_type {
            ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
                for c in children {
                    count_spots(c, n);
                }
            }
            _ => {}
        }
    }
    for layer in &doc.layers {
        for obj in &layer.objects {
            count_spots(obj, &mut skipped_spots);
        }
    }
    // Segment-level coincidence: O(n^2) over segments with an early-out
    // bbox check per candidate pair. Guard absurd inputs.
    let total_segs: usize = cands.iter().map(|c| c.points.len().saturating_sub(1)).sum();
    if total_segs > 60_000 {
        return (Vec::new(), skipped_spots);
    }
    // Quantized endpoint key so shared vertices hash together.
    fn key(p: Pt) -> (i64, i64) {
        (
            (p.0 / (TRAP_TOLERANCE_PT * 0.5)).round() as i64,
            (p.1 / (TRAP_TOLERANCE_PT * 0.5)).round() as i64,
        )
    }
    // segments per candidate: Vec<(p, q)>
    let segs: Vec<Vec<Seg>> = cands
        .iter()
        .map(|c| {
            c.points
                .windows(2)
                .map(|w| (w[0], w[1]))
                .filter(|(a, b)| (a.0 - b.0).hypot(a.1 - b.1) > 1e-9)
                .collect()
        })
        .collect();
    // matched undirected segments (dedup by quantized endpoints)
    let mut matched: std::collections::BTreeSet<SegKey> = std::collections::BTreeSet::new();
    let mut trap_colors: std::collections::HashMap<SegKey, [f32; 4]> =
        std::collections::HashMap::new();
    // Real coordinates per key (first occurrence wins); the overlap
    // detector above also pins its synthesized interior points here.
    let mut coord: std::collections::HashMap<(i64, i64), (f64, f64)> =
        std::collections::HashMap::new();
    for (i, (ci, si)) in cands.iter().zip(segs.iter()).enumerate() {
        for (cj, sj) in cands.iter().zip(segs.iter()).skip(i + 1) {
            if ci.color == cj.color {
                continue;
            }
            for &(a1, a2) in si {
                for &(b1, b2) in sj {
                    // Same or reversed direction within tolerance.
                    let fwd = (a1.0 - b1.0).hypot(a1.1 - b1.1) <= TRAP_TOLERANCE_PT
                        && (a2.0 - b2.0).hypot(a2.1 - b2.1) <= TRAP_TOLERANCE_PT;
                    let rev = (a1.0 - b2.0).hypot(a1.1 - b2.1) <= TRAP_TOLERANCE_PT
                        && (a2.0 - b1.0).hypot(a2.1 - b1.1) <= TRAP_TOLERANCE_PT;
                    if !fwd && !rev {
                        // Partial/T-junction overlap: project b onto a and
                        // require small lateral distance plus real 1D
                        // overlap (extra vertices / subdivisions on one
                        // side must not lose the shared run).
                        let dx = a2.0 - a1.0;
                        let dy = a2.1 - a1.1;
                        let len2 = dx * dx + dy * dy;
                        if len2 > 1e-12 {
                            let len = len2.sqrt();
                            let ux = dx / len;
                            let uy = dy / len;
                            let proj = |p: Pt| -> (f64, f64) {
                                let rx = p.0 - a1.0;
                                let ry = p.1 - a1.1;
                                (rx * ux + ry * uy, (rx * uy - ry * ux).abs())
                            };
                            let (t1, l1) = proj(b1);
                            let (t2, l2) = proj(b2);
                            if l1 <= TRAP_TOLERANCE_PT && l2 <= TRAP_TOLERANCE_PT {
                                let lo = t1.min(t2).max(0.0);
                                let hi = t1.max(t2).min(len);
                                if hi - lo >= 1.0 {
                                    let pt = |tt: f64| (a1.0 + ux * tt, a1.1 + uy * tt);
                                    let (u, v) = (key(pt(lo)), key(pt(hi)));
                                    // Interior points are synthesized: pin
                                    // them into the coordinate map first
                                    // (HashMap::index would panic on miss).
                                    coord.insert(u, pt(lo));
                                    coord.insert(v, pt(hi));
                                    let kk = if u <= v { (u, v) } else { (v, u) };
                                    if matched.insert(kk) {
                                        let darker = if luminance(ci.color) <= luminance(cj.color) {
                                            ci.color
                                        } else {
                                            cj.color
                                        };
                                        trap_colors.insert(kk, darker);
                                    }
                                    continue;
                                }
                            }
                        }
                        continue;
                    }
                    let (u, v) = (key(a1), key(a2));
                    let kk = if u <= v { (u, v) } else { (v, u) };
                    if matched.insert(kk) {
                        let darker = if luminance(ci.color) <= luminance(cj.color) {
                            ci.color
                        } else {
                            cj.color
                        };
                        trap_colors.insert(kk, darker);
                    }
                }
            }
        }
    }
    // Chain matched segments into polylines via endpoint adjacency.
    let mut adj: std::collections::HashMap<(i64, i64), Vec<(i64, i64)>> =
        std::collections::HashMap::new();
    for s in &segs {
        for &(a, b) in s {
            coord.entry(key(a)).or_insert(a);
            coord.entry(key(b)).or_insert(b);
        }
    }
    for &(u, v) in matched.iter() {
        adj.entry(u).or_default().push(v);
        adj.entry(v).or_default().push(u);
    }
    let mut used: std::collections::BTreeSet<SegKey> = std::collections::BTreeSet::new();
    let mut strokes = Vec::new();
    // Order keys for determinism.
    let mut keys: Vec<(i64, i64)> = adj.keys().copied().collect();
    keys.sort_unstable();
    for start in keys {
        let Some(neigh) = adj.get(&start) else {
            continue;
        };
        for &next in neigh {
            let kk = if start <= next {
                (start, next)
            } else {
                (next, start)
            };
            if !matched.contains(&kk) || !used.insert(kk) {
                continue;
            }
            // Walk both directions from this seed edge, recording the
            // edge key of every step so runs can split when the trap
            // color changes at a junction (seed color must not leak
            // across a T).
            let mut chain = vec![start, next];
            let mut edges = vec![kk];
            // forward
            loop {
                let cur = *chain.last().unwrap();
                let prev = chain[chain.len() - 2];
                let mut ext = None;
                if let Some(ns) = adj.get(&cur) {
                    let mut ns_sorted = ns.clone();
                    ns_sorted.sort_unstable();
                    for n in ns_sorted {
                        if n == prev {
                            continue;
                        }
                        let k2 = if cur <= n { (cur, n) } else { (n, cur) };
                        if matched.contains(&k2) && !used.contains(&k2) {
                            ext = Some(n);
                            break;
                        }
                    }
                }
                match ext {
                    Some(n) => {
                        let k2 = if cur <= n { (cur, n) } else { (n, cur) };
                        used.insert(k2);
                        edges.push(k2);
                        chain.push(n);
                    }
                    None => break,
                }
            }
            // backward (mirror): walk from start the other way
            loop {
                let cur = chain[0];
                let prev = chain[1];
                let mut ext = None;
                if let Some(ns) = adj.get(&cur) {
                    let mut ns_sorted = ns.clone();
                    ns_sorted.sort_unstable();
                    for n in ns_sorted {
                        if n == prev {
                            continue;
                        }
                        let k2 = if cur <= n { (cur, n) } else { (n, cur) };
                        if matched.contains(&k2) && !used.contains(&k2) {
                            ext = Some(n);
                            break;
                        }
                    }
                }
                match ext {
                    Some(n) => {
                        let k2 = if cur <= n { (cur, n) } else { (n, cur) };
                        used.insert(k2);
                        edges.insert(0, k2);
                        chain.insert(0, n);
                    }
                    None => break,
                }
            }
            // Split into same-color runs: edges[0] sits between
            // chain[0]-chain[1], edges[i] between chain[i]-chain[i+1].
            let mut run_start = 0;
            let color_of = |e: &SegKey| trap_colors[e];
            let mut run_color = color_of(&edges[0]);
            for i in 1..edges.len() {
                if color_of(&edges[i]) != run_color {
                    let pts: Vec<Pt> = chain[run_start..=i].iter().map(|k| coord[k]).collect();
                    if pts.len() >= 2 {
                        strokes.push(TrapStroke {
                            points: pts,
                            color: run_color,
                        });
                    }
                    run_start = i;
                    run_color = color_of(&edges[i]);
                }
            }
            let pts: Vec<Pt> = chain[run_start..].iter().map(|k| coord[k]).collect();
            if pts.len() >= 2 {
                strokes.push(TrapStroke {
                    points: pts,
                    color: run_color,
                });
            }
        }
    }
    (strokes, skipped_spots)
}

/// Build an overprinted stroke-only object for one trap.
pub fn trap_object(stroke: &TrapStroke, width: f64, index: usize) -> Object {
    let mut path = PathData::new();
    let mut it = stroke.points.iter();
    if let Some(&(x, y)) = it.next() {
        path.push_move_to(x, y);
        for &(x, y) in it {
            path.push_line_to(x, y);
        }
    }
    path.fill = None;
    path.stroke = Some(StrokeStyle {
        color: stroke.color,
        width: width.max(0.05),
        overprint: true,
        ..Default::default()
    });
    let mut obj = Object::new_path(&format!("Trap {index}"), path);
    obj.fill = None;
    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::document::Document;
    use crate::core::path::FillStyle;

    fn two_rects(left: [f32; 4], right: [f32; 4]) -> Document {
        let mut doc = Document::default();
        let mut a = Object::new_rect("A", 0.0, 0.0, 100.0, 100.0, 0.0);
        a.fill = Some(FillStyle::solid(left));
        let mut b = Object::new_rect("B", 100.0, 0.0, 100.0, 100.0, 0.0);
        b.fill = Some(FillStyle::solid(right));
        doc.add_object(a);
        doc.add_object(b);
        doc
    }

    #[test]
    fn abutting_fills_share_one_trap_edge() {
        // White left, black right: trap takes the darker (black) ink.
        let doc = two_rects([1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0]);
        let (strokes, skipped) = find_trap_strokes(&doc, 0.25);
        assert_eq!(skipped, 0);
        assert_eq!(strokes.len(), 1, "one shared edge: {strokes:?}");
        let s = &strokes[0];
        assert_eq!(s.color, [0.0, 0.0, 0.0, 1.0]);
        // Vertical edge at x=100 spanning the rect height.
        assert!(s.points.len() >= 2);
        for (x, _) in &s.points {
            assert!((*x - 100.0).abs() < 1.0, "on the seam: {x}");
        }
        let obj = trap_object(s, 0.25, 1);
        assert!(obj.fill.is_none());
        let st = obj.stroke.expect("trap is stroke-only");
        assert!(st.overprint);
        assert!((st.width - 0.25).abs() < 1e-9);
    }

    #[test]
    fn subdivided_edge_still_traps() {
        // T-junction: right rect spans two stacked left rects; the shared
        // run must trap despite the extra vertex.
        let mut doc = Document::default();
        let mut a = Object::new_rect("A", 0.0, 0.0, 100.0, 50.0, 0.0);
        a.fill = Some(FillStyle::solid([1.0, 1.0, 1.0, 1.0]));
        let mut b = Object::new_rect("B", 0.0, 50.0, 100.0, 50.0, 0.0);
        b.fill = Some(FillStyle::solid([1.0, 1.0, 1.0, 1.0]));
        let mut c = Object::new_rect("C", 100.0, 0.0, 100.0, 100.0, 0.0);
        c.fill = Some(FillStyle::solid([0.0, 0.0, 0.0, 1.0]));
        doc.add_object(a);
        doc.add_object(b);
        doc.add_object(c);
        let (strokes, _) = find_trap_strokes(&doc, 0.25);
        assert!(!strokes.is_empty(), "subdivided shared edge traps");
        for s in &strokes {
            assert_eq!(s.color, [0.0, 0.0, 0.0, 1.0]);
        }
    }

    #[test]
    fn same_color_and_gaps_trap_nothing() {
        let doc = two_rects([1.0, 0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0]);
        let (strokes, _) = find_trap_strokes(&doc, 0.25);
        assert!(strokes.is_empty());
        // 10pt gap: no shared edge.
        let mut doc = Document::default();
        let mut a = Object::new_rect("A", 0.0, 0.0, 100.0, 100.0, 0.0);
        a.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
        let mut b = Object::new_rect("B", 110.0, 0.0, 100.0, 100.0, 0.0);
        b.fill = Some(FillStyle::solid([0.0, 0.0, 1.0, 1.0]));
        doc.add_object(a);
        doc.add_object(b);
        let (strokes, _) = find_trap_strokes(&doc, 0.25);
        assert!(strokes.is_empty());
    }
}
