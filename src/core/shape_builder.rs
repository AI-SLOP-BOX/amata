use super::boolean::{apply_polygon_boolean, BooleanOp};
use super::document::Object;
use super::geometry::{point_in_polygon, polygon_centroid};
use super::path::{AnchorPoint, PathData};

#[derive(Debug, Clone)]
pub struct ShapeFragment {
    pub polygon: Vec<AnchorPoint>,
    pub centroid: AnchorPoint,
    pub original_object_indices: Vec<usize>,
}

/// Decompose a set of overlapping objects into minimal disjoint shape fragments.
/// In Illustrator's Shape Builder Tool, when you hover over intersecting shapes,
/// each distinct non-overlapping subdivision region can be clicked to create an independent shape,
/// or dragged across to merge.
pub fn decompose_shapes_into_fragments(objects: &[&Object]) -> Vec<ShapeFragment> {
    if objects.is_empty() {
        return Vec::new();
    }
    if objects.len() == 1 {
        let poly = objects[0].to_world_polygon();
        if poly.len() >= 3 {
            let c = polygon_centroid(&poly);
            return vec![ShapeFragment {
                polygon: poly,
                centroid: c,
                original_object_indices: vec![0],
            }];
        }
        return Vec::new();
    }

    let polys: Vec<Vec<AnchorPoint>> = objects.iter().map(|o| o.to_world_polygon()).collect();
    let mut fragments: Vec<ShapeFragment> = Vec::new();

    // 1. For pairwise interactions, compute intersections and subtractions
    // Let's compute the atomic regions:
    // For shape A and shape B:
    // Region AB = A ∩ B
    // Region A_only = A - B
    // Region B_only = B - A
    let mut candidate_polys: Vec<(Vec<AnchorPoint>, Vec<usize>)> = Vec::new();

    for (i, poly) in polys.iter().enumerate() {
        if poly.len() >= 3 {
            candidate_polys.push((poly.clone(), vec![i]));
        }
    }

    // Iteratively slice candidate fragments with each other shape boundary
    for (obj_idx, other_poly) in polys.iter().enumerate() {
        if other_poly.len() < 3 {
            continue;
        }

        let mut next_candidates = Vec::new();
        for (cand_poly, parent_indices) in candidate_polys {
            if parent_indices.contains(&obj_idx) {
                next_candidates.push((cand_poly, parent_indices));
                continue;
            }

            // Check intersection with other_poly
            let inter_pieces = apply_polygon_boolean(&cand_poly, other_poly, BooleanOp::Intersect);
            let sub_pieces = apply_polygon_boolean(&cand_poly, other_poly, BooleanOp::Subtract);

            if inter_pieces.is_empty() {
                // No overlap with this other_poly
                next_candidates.push((cand_poly, parent_indices));
            } else {
                // Intersected portion belongs to both parent_indices and obj_idx
                for piece in inter_pieces {
                    if piece.len() >= 3 {
                        let mut combined_parents = parent_indices.clone();
                        if !combined_parents.contains(&obj_idx) {
                            combined_parents.push(obj_idx);
                        }
                        next_candidates.push((piece, combined_parents));
                    }
                }
                // Remaining subtracted portion belongs only to parent_indices
                for piece in sub_pieces {
                    if piece.len() >= 3 {
                        next_candidates.push((piece, parent_indices.clone()));
                    }
                }
            }
        }
        candidate_polys = next_candidates;
    }

    // Deduplicate and assemble fragments
    for (poly, parent_indices) in candidate_polys {
        if poly.len() >= 3 {
            let c = polygon_centroid(&poly);
            // Verify centroid is inside polygon
            let valid_poly = if point_in_polygon(c.x, c.y, &poly) {
                poly
            } else {
                // Use first vertex + inner offset if centroid is outside (concave shapes)
                poly
            };
            fragments.push(ShapeFragment {
                polygon: valid_poly,
                centroid: c,
                original_object_indices: parent_indices,
            });
        }
    }

    fragments
}

/// Merge selected shape fragments into unified objects (Shape Builder Join / Merge operation).
pub fn merge_fragments(fragments: &[&ShapeFragment], base_object: &Object) -> Object {
    if fragments.is_empty() {
        return base_object.clone();
    }

    let mut current = fragments[0].polygon.clone();
    for frag in &fragments[1..] {
        let united = apply_polygon_boolean(&current, &frag.polygon, BooleanOp::Union);
        if let Some(first) = united.into_iter().next() {
            current = first;
        }
    }

    let mut path = PathData::from_polygon_points(&current, true);
    path.fill = base_object.fill.clone();
    path.stroke = base_object.stroke.clone();

    let mut obj = Object::new_path(&format!("{}_Merged", base_object.name), path);
    obj.transform = Default::default();
    obj.shadow = base_object.shadow.clone();
    obj.glow = base_object.glow.clone();
    obj.opacity = base_object.opacity;
    obj.blend_mode = base_object.blend_mode;
    obj
}

/// Extract a single fragment into an independent vector Object.
pub fn fragment_to_object(fragment: &ShapeFragment, base_object: &Object) -> Object {
    let mut path = PathData::from_polygon_points(&fragment.polygon, true);
    path.fill = base_object.fill.clone();
    path.stroke = base_object.stroke.clone();

    let mut obj = Object::new_path(&format!("{}_Fragment", base_object.name), path);
    obj.transform = Default::default();
    obj.shadow = base_object.shadow.clone();
    obj.glow = base_object.glow.clone();
    obj.opacity = base_object.opacity;
    obj.blend_mode = base_object.blend_mode;
    obj
}
