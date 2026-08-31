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

pub fn apply_polygon_boolean(
    subject: &[AnchorPoint],
    clip: &[AnchorPoint],
    op: BooleanOp,
) -> Vec<Vec<AnchorPoint>> {
    if subject.is_empty() {
        return match op {
            BooleanOp::Union | BooleanOp::Exclude => {
                if clip.is_empty() {
                    vec![]
                } else {
                    vec![clip.to_vec()]
                }
            }
            _ => vec![],
        };
    }
    if clip.is_empty() {
        return match op {
            BooleanOp::Union | BooleanOp::Subtract | BooleanOp::Exclude => vec![subject.to_vec()],
            BooleanOp::Intersect => vec![],
        };
    }

    match op {
        BooleanOp::Union => polygon_union(subject, clip),
        BooleanOp::Intersect => polygon_intersect(subject, clip),
        BooleanOp::Subtract => polygon_subtract(subject, clip),
        BooleanOp::Exclude => polygon_exclude(subject, clip),
    }
}

fn polygon_intersect(subject: &[AnchorPoint], clip: &[AnchorPoint]) -> Vec<Vec<AnchorPoint>> {
    if subject.len() < 3 || clip.len() < 3 {
        return vec![];
    }

    let is_ccw = signed_polygon_area(clip) >= 0.0;
    let mut output = subject.to_vec();

    for i in 0..clip.len() {
        if output.is_empty() {
            break;
        }
        let p1 = clip[i];
        let p2 = clip[(i + 1) % clip.len()];

        let input = output;
        output = Vec::new();

        let mut s = *input.last().unwrap();
        for e in &input {
            let e_in = is_inside_edge(*e, p1, p2, is_ccw);
            let s_in = is_inside_edge(s, p1, p2, is_ccw);

            if e_in {
                if !s_in {
                    if let Some(inter) = line_intersection_edge(s, *e, p1, p2) {
                        output.push(inter);
                    }
                }
                output.push(*e);
            } else if s_in {
                if let Some(inter) = line_intersection_edge(s, *e, p1, p2) {
                    output.push(inter);
                }
            }
            s = *e;
        }
    }

    let mut clean: Vec<AnchorPoint> = Vec::new();
    for p in output {
        if let Some(last) = clean.last() {
            if p.distance(*last) > 1e-4 {
                clean.push(p);
            }
        } else {
            clean.push(p);
        }
    }

    if clean.len() >= 3 {
        vec![clean]
    } else {
        vec![]
    }
}

fn polygon_subtract(subject: &[AnchorPoint], clip: &[AnchorPoint]) -> Vec<Vec<AnchorPoint>> {
    if subject.is_empty() {
        return vec![];
    }
    if clip.is_empty() {
        return vec![subject.to_vec()];
    }

    let inter = polygon_intersect(subject, clip);
    if inter.is_empty() {
        if point_in_polygon(subject[0].x, subject[0].y, clip) {
            return vec![];
        }
        return vec![subject.to_vec()];
    }

    let mut all_inside = true;
    for pt in subject {
        if !point_in_polygon(pt.x, pt.y, clip) {
            all_inside = false;
            break;
        }
    }
    if all_inside {
        return vec![];
    }

    let mut enriched = Vec::new();
    let n_s = subject.len();
    for i in 0..n_s {
        let s1 = subject[i];
        let s2 = subject[(i + 1) % n_s];
        enriched.push(s1);

        let mut inters: Vec<(f64, AnchorPoint)> = Vec::new();
        let n_c = clip.len();
        for j in 0..n_c {
            let c1 = clip[j];
            let c2 = clip[(j + 1) % n_c];
            if let Some(pt) = line_segment_intersection(s1, s2, c1, c2) {
                let dist = s1.distance(pt);
                inters.push((dist, pt));
            }
        }
        inters.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, pt) in inters {
            enriched.push(pt);
        }
    }

    let mut remaining = Vec::new();
    for pt in enriched {
        if !point_in_polygon(pt.x, pt.y, clip)
            && remaining.last() != Some(&pt) {
                remaining.push(pt);
            }
    }

    let clip_is_inner_hole = clip.iter().all(|pt| point_in_polygon(pt.x, pt.y, subject));
    if clip_is_inner_hole {
        let mut bridged = Vec::new();
        bridged.extend_from_slice(subject);
        let mut reversed_clip = clip.to_vec();
        reversed_clip.reverse();
        bridged.push(reversed_clip[0]);
        bridged.extend(reversed_clip);
        return vec![bridged];
    }

    if remaining.len() >= 3 {
        vec![remaining]
    } else {
        vec![subject.to_vec()]
    }
}

fn polygon_union(subject: &[AnchorPoint], clip: &[AnchorPoint]) -> Vec<Vec<AnchorPoint>> {
    if subject.is_empty() && clip.is_empty() {
        return vec![];
    }
    if subject.is_empty() {
        return vec![clip.to_vec()];
    }
    if clip.is_empty() {
        return vec![subject.to_vec()];
    }

    let inter = polygon_intersect(subject, clip);
    if inter.is_empty() {
        return vec![subject.to_vec(), clip.to_vec()];
    }

    let sub_in_clip = subject.iter().all(|p| point_in_polygon(p.x, p.y, clip));
    if sub_in_clip {
        return vec![clip.to_vec()];
    }
    let clip_in_sub = clip.iter().all(|p| point_in_polygon(p.x, p.y, subject));
    if clip_in_sub {
        return vec![subject.to_vec()];
    }

    let mut boundary: Vec<AnchorPoint> = Vec::new();
    for pt in subject {
        if !point_in_polygon(pt.x, pt.y, clip) {
            boundary.push(*pt);
        }
    }
    for pt in clip {
        if !point_in_polygon(pt.x, pt.y, subject) {
            boundary.push(*pt);
        }
    }

    let n_s = subject.len();
    let n_c = clip.len();
    for i in 0..n_s {
        let s1 = subject[i];
        let s2 = subject[(i + 1) % n_s];
        for j in 0..n_c {
            let c1 = clip[j];
            let c2 = clip[(j + 1) % n_c];
            if let Some(pt) = line_segment_intersection(s1, s2, c1, c2) {
                boundary.push(pt);
            }
        }
    }

    if boundary.len() < 3 {
        return vec![subject.to_vec(), clip.to_vec()];
    }

    let centroid = super::geometry::polygon_centroid(&boundary);
    boundary.sort_by(|a, b| {
        let angle_a = (a.y - centroid.y).atan2(a.x - centroid.x);
        let angle_b = (b.y - centroid.y).atan2(b.x - centroid.x);
        angle_a.total_cmp(&angle_b)
    });

    let mut dedup: Vec<AnchorPoint> = Vec::new();
    for p in boundary {
        if let Some(last) = dedup.last() {
            if p.distance(*last) > 1e-4 {
                dedup.push(p);
            }
        } else {
            dedup.push(p);
        }
    }

    if dedup.len() >= 3 {
        vec![dedup]
    } else {
        vec![subject.to_vec(), clip.to_vec()]
    }
}

fn polygon_exclude(subject: &[AnchorPoint], clip: &[AnchorPoint]) -> Vec<Vec<AnchorPoint>> {
    let sub_a = polygon_subtract(subject, clip);
    let sub_b = polygon_subtract(clip, subject);
    let mut result = Vec::new();
    result.extend(sub_a);
    result.extend(sub_b);
    result
}

fn is_inside_edge(p: AnchorPoint, p1: AnchorPoint, p2: AnchorPoint, is_ccw: bool) -> bool {
    let cross = (p2.x - p1.x) * (p.y - p1.y) - (p2.y - p1.y) * (p.x - p1.x);
    if is_ccw {
        cross >= -1e-4
    } else {
        cross <= 1e-4
    }
}

fn line_intersection_edge(
    a1: AnchorPoint,
    a2: AnchorPoint,
    b1: AnchorPoint,
    b2: AnchorPoint,
) -> Option<AnchorPoint> {
    let d = (b2.y - b1.y) * (a2.x - a1.x) - (b2.x - b1.x) * (a2.y - a1.y);
    if d.abs() < 1e-6 {
        return None;
    }
    let ua = ((b2.x - b1.x) * (a1.y - b1.y) - (b2.y - b1.y) * (a1.x - b1.x)) / d;
    Some(AnchorPoint::new(
        a1.x + ua * (a2.x - a1.x),
        a1.y + ua * (a2.y - a1.y),
    ))
}
