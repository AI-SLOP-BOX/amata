use super::document::Object;
use super::geometry::{line_segment_intersection, point_in_polygon, signed_polygon_area};
use super::path::{AnchorPoint, PathData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Intersect,
    Subtract,
    Exclude,
}

impl BooleanOp {
    pub fn name(&self) -> &'static str {
        match self {
            BooleanOp::Union => "Unite",
            BooleanOp::Subtract => "Minus Front",
            BooleanOp::Intersect => "Intersect",
            BooleanOp::Exclude => "Exclude",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            BooleanOp::Union => "⧉",
            BooleanOp::Subtract => "⊟",
            BooleanOp::Intersect => "⨅",
            BooleanOp::Exclude => "⨁",
        }
    }
}

/// Execute a boolean operation across multiple objects
pub fn execute_pathfinder(objects: &[&Object], op: BooleanOp) -> Option<Object> {
    if objects.len() < 2 {
        return None;
    }

    let mut current_polys = vec![objects[0].to_world_polygon()];
    let base_fill = objects[0].fill.clone();
    let base_stroke = objects[0].stroke.clone();

    for obj in &objects[1..] {
        let clip_poly = obj.to_world_polygon();
        let mut next_polys = Vec::new();

        for subj_poly in current_polys {
            let res = apply_polygon_boolean(&subj_poly, &clip_poly, op);
            next_polys.extend(res);
        }
        current_polys = next_polys;
        if current_polys.is_empty() && op == BooleanOp::Intersect {
            break;
        }
    }

    if current_polys.is_empty() {
        return None;
    }

    let mut combined_path = PathData::new();
    for poly in current_polys {
        if poly.len() >= 3 {
            let mut sub_path = PathData::from_polygon_points(&poly, true);
            combined_path.elements.append(&mut sub_path.elements);
        }
    }

    if combined_path.is_empty() {
        return None;
    }

    combined_path.fill = base_fill;
    combined_path.stroke = base_stroke;
    let mut obj = Object::new_path(&format!("Pathfinder ({})", op.name()), combined_path);
    obj.transform = Default::default(); // already in world coordinates
    Some(obj)
}

/// Drop consecutive duplicates and closing duplicates; degenerate input
/// (fewer than 3 distinct points) cannot participate in boolean ops.
fn clean_polygon(poly: &[AnchorPoint]) -> Vec<AnchorPoint> {
    let mut out = Vec::with_capacity(poly.len());
    for p in poly {
        if let Some(last) = out.last() {
            if p.distance(*last) < 1e-4 {
                continue;
            }
        }
        out.push(*p);
    }
    if out.len() >= 2 && out[0].distance(out[out.len() - 1]) < 1e-4 {
        out.pop();
    }
    out
}

fn dedup_ring(poly: Vec<AnchorPoint>) -> Vec<AnchorPoint> {
    let mut out = Vec::with_capacity(poly.len());
    for p in poly {
        if let Some(last) = out.last() {
            if p.distance(*last) < 1e-4 {
                continue;
            }
        }
        out.push(p);
    }
    if out.len() >= 2 && out[0].distance(out[out.len() - 1]) < 1e-4 {
        out.pop();
    }
    out
}

pub fn apply_polygon_boolean(
    subject: &[AnchorPoint],
    clip: &[AnchorPoint],
    op: BooleanOp,
) -> Vec<Vec<AnchorPoint>> {
    let subject = clean_polygon(subject);
    let clip = clean_polygon(clip);
    if subject.len() < 3 {
        return match op {
            BooleanOp::Union | BooleanOp::Exclude => {
                if clip.len() >= 3 {
                    vec![clip]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        };
    }
    if clip.len() < 3 {
        return match op {
            BooleanOp::Union | BooleanOp::Subtract | BooleanOp::Exclude => vec![subject],
            BooleanOp::Intersect => vec![],
        };
    }

    match op {
        BooleanOp::Union => gh_boolean(&subject, &clip, GhOp::Union),
        BooleanOp::Intersect => gh_boolean(&subject, &clip, GhOp::Intersect),
        BooleanOp::Subtract => gh_boolean(&subject, &clip, GhOp::Subtract),
        BooleanOp::Exclude => {
            let mut result = gh_boolean(&subject, &clip, GhOp::Subtract);
            result.extend(gh_boolean(&clip, &subject, GhOp::Subtract));
            result
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GhOp {
    Intersect,
    Union,
    Subtract,
}

#[derive(Clone)]
struct GhNode {
    p: AnchorPoint,
    is_x: bool,
    /// Index of the twin crossing in the other polygon's node list.
    other: usize,
    visited: bool,
}

fn gh_nodes(poly: &[AnchorPoint]) -> Vec<GhNode> {
    poly.iter()
        .map(|&p| GhNode {
            p,
            is_x: false,
            other: usize::MAX,
            visited: false,
        })
        .collect()
}

/// Strict-interior edge crossing parameter, or None for parallel,
/// collinear-overlapping and endpoint touches (handled as non-crossing).
fn gh_cross_param(a1: AnchorPoint, a2: AnchorPoint, pt: AnchorPoint) -> f64 {
    let dx = a2.x - a1.x;
    let dy = a2.y - a1.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return 0.5;
    }
    (((pt.x - a1.x) * dx + (pt.y - a1.y) * dy) / len_sq).clamp(0.0, 1.0)
}

/// Distance from `p` to segment AB plus the projection parameter.
fn gh_seg_dist_t(p: AnchorPoint, a: AnchorPoint, b: AnchorPoint) -> (f64, f64) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return (p.distance(a), 0.0);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    (
        AnchorPoint::new(a.x + t * dx, a.y + t * dy).distance(p),
        t,
    )
}

/// A polygon is convex when all turns share a sign (collinear runs OK).
fn is_convex(poly: &[AnchorPoint]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut sign = 0i32;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let c = poly[(i + 2) % n];
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross.abs() < 1e-9 {
            continue;
        }
        let s = if cross > 0.0 { 1 } else { -1 };
        if sign == 0 {
            sign = s;
        } else if sign != s {
            return false;
        }
    }
    true
}

/// Sutherland–Hodgman half-plane clip. Exact for convex clips (shared
/// boundary segments and touches included via tolerance); never used with
/// concave clips, where it silently produces garbage.
fn sutherland_clip(subject: &[AnchorPoint], convex_clip: &[AnchorPoint]) -> Vec<AnchorPoint> {
    let is_ccw = signed_polygon_area(convex_clip) >= 0.0;
    let mut output = subject.to_vec();
    for i in 0..convex_clip.len() {
        if output.is_empty() {
            break;
        }
        let p1 = convex_clip[i];
        let p2 = convex_clip[(i + 1) % convex_clip.len()];
        let input = std::mem::take(&mut output);
        let mut s = *input.last().unwrap();
        for e in &input {
            let cross_e = (p2.x - p1.x) * (e.y - p1.y) - (p2.y - p1.y) * (e.x - p1.x);
            let cross_s = (p2.x - p1.x) * (s.y - p1.y) - (p2.y - p1.y) * (s.x - p1.x);
            let e_in = if is_ccw {
                cross_e >= -1e-4
            } else {
                cross_e <= 1e-4
            };
            let s_in = if is_ccw {
                cross_s >= -1e-4
            } else {
                cross_s <= 1e-4
            };
            if e_in {
                if !s_in {
                    if let Some(pt) = sh_edge_intersection(s, *e, p1, p2) {
                        output.push(pt);
                    }
                }
                output.push(*e);
            } else if s_in {
                if let Some(pt) = sh_edge_intersection(s, *e, p1, p2) {
                    output.push(pt);
                }
            }
            s = *e;
        }
    }
    dedup_ring(output)
}

fn sh_edge_intersection(
    a1: AnchorPoint,
    a2: AnchorPoint,
    b1: AnchorPoint,
    b2: AnchorPoint,
) -> Option<AnchorPoint> {
    let d = (b2.y - b1.y) * (a2.x - a1.x) - (b2.x - b1.x) * (a2.y - a1.y);
    if d.abs() < 1e-9 {
        return None;
    }
    let ua = ((b2.x - b1.x) * (a1.y - b1.y) - (b2.y - b1.y) * (a1.x - b1.x)) / d;
    Some(AnchorPoint::new(
        a1.x + ua * (a2.x - a1.x),
        a1.y + ua * (a2.y - a1.y),
    ))
}

/// Build crossing-linked node lists. Besides proper edge crossings this
/// registers vertex-edge touches: shared corners (coincident vertices) and
/// T-junctions (a vertex strictly inside the other polygon's edge), which
/// carry collinear shared boundary segments. Returns None only when the
/// boundaries are fully disjoint. The flag reports proper crossings.
fn gh_link(
    subject: &[AnchorPoint],
    clip: &[AnchorPoint],
) -> Option<(Vec<GhNode>, Vec<GhNode>, bool)> {
    let ns = subject.len();
    let nc = clip.len();
    // Per-edge crossing records: (param, point).
    let mut s_hits: Vec<Vec<(f64, AnchorPoint)>> = vec![Vec::new(); ns];
    let mut c_hits: Vec<Vec<(f64, AnchorPoint)>> = vec![Vec::new(); nc];
    let mut found = false;
    let mut proper = false;
    for i in 0..ns {
        let a1 = subject[i];
        let a2 = subject[(i + 1) % ns];
        for j in 0..nc {
            let b1 = clip[j];
            let b2 = clip[(j + 1) % nc];
            if let Some(pt) = line_segment_intersection(a1, a2, b1, b2) {
                found = true;
                proper = true;
                s_hits[i].push((gh_cross_param(a1, a2, pt), pt));
                c_hits[j].push((gh_cross_param(b1, b2, pt), pt));
            }
        }
    }
    // Vertex marks: vertices promoted to crossings.
    let mut s_mark = vec![false; ns];
    let mut c_mark = vec![false; nc];
    // Coincident vertex pairs (shared corners), linked directly.
    for i in 0..ns {
        for j in 0..nc {
            if subject[i].distance(clip[j]) < 1e-4 {
                found = true;
                s_mark[i] = true;
                c_mark[j] = true;
            }
        }
    }
    // T-junctions: subject vertex strictly inside a clip edge.
    for i in 0..ns {
        if s_mark[i] {
            continue;
        }
        for j in 0..nc {
            let (d, t) = gh_seg_dist_t(subject[i], clip[j], clip[(j + 1) % nc]);
            if d < 1e-6 && t > 1e-3 && t < 1.0 - 1e-3 {
                found = true;
                s_mark[i] = true;
                c_hits[j].push((t, subject[i]));
                break;
            }
        }
    }
    // Mirror: clip vertex strictly inside a subject edge.
    for j in 0..nc {
        if c_mark[j] {
            continue;
        }
        for i in 0..ns {
            let (d, t) = gh_seg_dist_t(clip[j], subject[i], subject[(i + 1) % ns]);
            if d < 1e-6 && t > 1e-3 && t < 1.0 - 1e-3 {
                found = true;
                c_mark[j] = true;
                s_hits[i].push((t, clip[j]));
                break;
            }
        }
    }
    if !found {
        return None;
    }
    let mut s_list: Vec<GhNode> = Vec::with_capacity(ns + 8);
    for i in 0..ns {
        s_list.push(GhNode {
            p: subject[i],
            is_x: s_mark[i],
            other: usize::MAX,
            visited: false,
        });
        let mut hits = std::mem::take(&mut s_hits[i]);
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut last_t = -1.0;
        for (t, pt) in hits {
            // Merge near-coincident crossings from adjacent clip edges.
            if t - last_t < 1e-6 {
                continue;
            }
            last_t = t;
            // Skip insertions colliding with an already-marked vertex.
            if s_list.iter().any(|n| n.p.distance(pt) < 1e-4) {
                continue;
            }
            s_list.push(GhNode {
                p: pt,
                is_x: true,
                other: usize::MAX,
                visited: false,
            });
        }
    }
    let mut c_list: Vec<GhNode> = Vec::with_capacity(nc + 8);
    for j in 0..nc {
        c_list.push(GhNode {
            p: clip[j],
            is_x: c_mark[j],
            other: usize::MAX,
            visited: false,
        });
        let mut hits = std::mem::take(&mut c_hits[j]);
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut last_t = -1.0;
        for (t, pt) in hits {
            if t - last_t < 1e-6 {
                continue;
            }
            last_t = t;
            if c_list.iter().any(|n| n.p.distance(pt) < 1e-4) {
                continue;
            }
            c_list.push(GhNode {
                p: pt,
                is_x: true,
                other: usize::MAX,
                visited: false,
            });
        }
    }
    // Link twins by coordinates.
    for si in 0..s_list.len() {
        if !s_list[si].is_x || s_list[si].other != usize::MAX {
            continue;
        }
        if let Some(ci) = c_list
            .iter()
            .position(|n| n.is_x && n.other == usize::MAX && n.p.distance(s_list[si].p) < 1e-4)
        {
            s_list[si].other = ci;
            c_list[ci].other = si;
        }
    }
    Some((s_list, c_list, proper))
}

fn gh_inside(pt: AnchorPoint, poly: &[AnchorPoint]) -> bool {
    point_in_polygon(pt.x, pt.y, poly)
}

/// Probe slightly ahead of `at` along `dir`; tests region membership.
///
/// The probe distance MUST exceed `point_in_polygon`'s boundary tolerance
/// (distance < 0.01 counts as inside): a shorter probe sits "on" the
/// boundary it just crossed and always reads inside, which breaks every
/// entry/exit classification and keep test.
fn gh_probe(at: AnchorPoint, dir: AnchorPoint, poly: &[AnchorPoint]) -> bool {
    let len = (dir.x * dir.x + dir.y * dir.y).sqrt().max(1e-9);
    let eps = 5e-2;
    gh_inside(
        AnchorPoint::new(at.x + dir.x / len * eps, at.y + dir.y / len * eps),
        poly,
    )
}

fn gh_edge_dir(from: AnchorPoint, to: AnchorPoint) -> AnchorPoint {
    AnchorPoint::new(to.x - from.x, to.y - from.y)
}

/// Keep predicate for a directed edge on `on_subject` under `op`.
fn gh_keep(op: GhOp, on_subject: bool, probe_inside_other: bool) -> bool {
    match op {
        GhOp::Intersect => probe_inside_other,
        GhOp::Union => !probe_inside_other,
        GhOp::Subtract => {
            if on_subject {
                !probe_inside_other
            } else {
                probe_inside_other
            }
        }
    }
}

/// Greiner–Hormann traversal. Handles concave inputs; falls back to
/// containment results when boundaries do not properly cross.
fn gh_boolean(subject: &[AnchorPoint], clip: &[AnchorPoint], op: GhOp) -> Vec<Vec<AnchorPoint>> {
    let sub_in_clip = subject.iter().all(|p| gh_inside(*p, clip));
    let clip_in_sub = clip.iter().all(|p| gh_inside(*p, subject));

    let Some((mut s_list, mut c_list, proper)) = gh_link(subject, clip) else {
        // Fully disjoint boundaries.
        return match op {
            GhOp::Intersect => {
                if sub_in_clip {
                    vec![subject.to_vec()]
                } else if clip_in_sub {
                    vec![clip.to_vec()]
                } else {
                    vec![]
                }
            }
            GhOp::Union => {
                if sub_in_clip {
                    vec![clip.to_vec()]
                } else if clip_in_sub {
                    vec![subject.to_vec()]
                } else {
                    vec![subject.to_vec(), clip.to_vec()]
                }
            }
            GhOp::Subtract => {
                if sub_in_clip {
                    vec![]
                } else {
                    vec![subject.to_vec()]
                }
            }
        };
    };
    // Touch-only boundaries (no proper crossing): containment decides,
    // except intersect against a convex clip, where half-plane clipping
    // resolves shared-boundary overlaps exactly.
    if !proper {
        if op == GhOp::Intersect && !sub_in_clip && !clip_in_sub {
            if is_convex(clip) {
                let r = dedup_ring(sutherland_clip(&subject, &clip));
                if r.len() >= 3 {
                    return vec![r];
                }
            } else if is_convex(subject) {
                let r = dedup_ring(sutherland_clip(&clip, &subject));
                if r.len() >= 3 {
                    return vec![r];
                }
            }
        }
        return match op {
            GhOp::Intersect => {
                if sub_in_clip {
                    vec![subject.to_vec()]
                } else if clip_in_sub {
                    vec![clip.to_vec()]
                } else {
                    vec![]
                }
            }
            GhOp::Union => {
                if sub_in_clip {
                    vec![clip.to_vec()]
                } else if clip_in_sub {
                    vec![subject.to_vec()]
                } else {
                    vec![subject.to_vec(), clip.to_vec()]
                }
            }
            GhOp::Subtract => {
                if sub_in_clip {
                    vec![]
                } else {
                    vec![subject.to_vec()]
                }
            }
        };
    }

    // Classify subject crossings: entry (out->in) vs exit (in->out).
    // entry[i] is valid only for crossing nodes.
    let mut is_entry = vec![false; s_list.len()];
    for (i, node) in s_list.iter().enumerate() {
        if !node.is_x {
            continue;
        }
        let prev = s_list[(i + s_list.len() - 1) % s_list.len()].p;
        let next = s_list[(i + 1) % s_list.len()].p;
        let before_in = gh_probe(node.p, gh_edge_dir(node.p, prev), clip);
        let after_in = gh_probe(node.p, gh_edge_dir(node.p, next), clip);
        is_entry[i] = !before_in && after_in;
    }

    // Start crossings on subject: entries for intersect, exits otherwise.
    // (Subtract keeps subject-outside parts, which start at exits;
    // union keeps outside parts on both polygons, also starting at exits.)
    let want_entry = op == GhOp::Intersect;
    let mut rings: Vec<Vec<AnchorPoint>> = Vec::new();
    for start in 0..s_list.len() {
        if !s_list[start].is_x || s_list[start].visited || is_entry[start] != want_entry {
            continue;
        }
        // Walk a ring.
        let mut ring = vec![s_list[start].p];
        // (on_subject, index, dir)
        let mut cur = (true, start, 1i32);
        s_list[start].visited = true;
        if s_list[start].other != usize::MAX {
            c_list[s_list[start].other].visited = true;
        }
        let mut guard = 0usize;
        let limit = 4 * (s_list.len() + c_list.len()) + 8;
        let closed = loop {
            guard += 1;
            if guard > limit {
                break false;
            }
            let (on_s, idx, dir) = cur;
            // Step to adjacent vertex in walking direction.
            let len = if on_s { s_list.len() } else { c_list.len() };
            let nxt = ((idx as i32 + dir).rem_euclid(len as i32)) as usize;
            if nxt == start && on_s {
                break true;
            }
            let (node_p, node_x, node_other) = if on_s {
                let n = &s_list[nxt];
                (n.p, n.is_x, n.other)
            } else {
                let n = &c_list[nxt];
                (n.p, n.is_x, n.other)
            };
            // Arriving at the start crossing's twin means the walk has
            // returned to its geometric origin on the other polygon:
            // close the ring (dedup drops the trailing duplicate).
            if !on_s && node_other == start {
                ring.push(node_p);
                break true;
            }
            // Already-consumed crossings are passed through without
            // re-switching; otherwise walks ping-pong and never close.
            let consumed = if on_s {
                s_list[nxt].visited
            } else {
                c_list[nxt].visited
            };
            if node_x && !consumed {
                // Mark the crossing consumed so tangent pass-throughs
                // cannot seed spurious rings later.
                if on_s {
                    s_list[nxt].visited = true;
                    if node_other != usize::MAX {
                        c_list[node_other].visited = true;
                    }
                } else {
                    c_list[nxt].visited = true;
                    if node_other != usize::MAX {
                        s_list[node_other].visited = true;
                    }
                }
                // Choose outgoing edge: prefer switching polygons; probe
                // both directions of the other polygon against this
                // polygon's interior.
                let other_len = if on_s { c_list.len() } else { s_list.len() };
                let mut switched = false;
                let mut closed_now = false;
                if node_other != usize::MAX {
                    for ndir in [1i32, -1] {
                        let ahead_idx =
                            ((node_other as i32 + ndir).rem_euclid(other_len as i32)) as usize;
                        let ahead_p = if on_s {
                            c_list[ahead_idx].p
                        } else {
                            s_list[ahead_idx].p
                        };
                        let self_poly = if on_s { subject } else { clip };
                        let probe_in =
                            gh_probe(node_p, gh_edge_dir(node_p, ahead_p), self_poly);
                        // Candidates live on the other polygon (!on_s),
                        // tested against this one.
                        if gh_keep(op, !on_s, probe_in) {
                            ring.push(node_p);
                            if !on_s && node_other == start {
                                // Switched back onto the start crossing:
                                // the ring is closed (dedup drops the
                                // trailing duplicate of ring[0]).
                                closed_now = true;
                            } else {
                                cur = (!on_s, node_other, ndir);
                                switched = true;
                            }
                            break;
                        }
                    }
                }
                if closed_now {
                    break true;
                }
                if !switched {
                    // Degenerate touch: continue on the same polygon.
                    ring.push(node_p);
                    cur = (on_s, nxt, dir);
                }
            } else {
                ring.push(node_p);
                cur = (on_s, nxt, dir);
            }
        };
        if closed {
            let clean = dedup_ring(ring);
            if clean.len() >= 3 {
                rings.push(clean);
            }
        }
    }
    if rings.is_empty() {
        // Traversal failed despite proper crossings (pathological
        // tangencies): return conservative non-garbage results.
        return match op {
            GhOp::Intersect => {
                if is_convex(clip) {
                    let r = dedup_ring(sutherland_clip(&subject, &clip));
                    if r.len() >= 3 {
                        return vec![r];
                    }
                }
                if sub_in_clip {
                    vec![subject.to_vec()]
                } else if clip_in_sub {
                    vec![clip.to_vec()]
                } else {
                    vec![]
                }
            }
            GhOp::Union => vec![subject.to_vec(), clip.to_vec()],
            GhOp::Subtract => {
                if sub_in_clip {
                    vec![]
                } else {
                    vec![subject.to_vec()]
                }
            }
        };
    }
    rings
}

