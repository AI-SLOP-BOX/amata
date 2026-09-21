use crate::core::document::{Document, Object, ObjectType};
use crate::core::path::{FillType, GradientStop, ImageFill};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Field-level changes on an object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldDiff {
    pub field: String,
    #[serde(rename = "before", alias = "old_value")]
    pub old_value: String,
    #[serde(rename = "after", alias = "new_value")]
    pub new_value: String,
}

/// Status of an object in a semantic diff
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ObjectDiffStatus {
    Added,
    Removed,
    Modified { changes: Vec<FieldDiff> },
}

/// A semantic difference record for an object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectDiff {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub status: ObjectDiffStatus,
}

/// Symbol component change record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolDiff {
    pub id: String,
    pub name: String,
    pub status: ObjectDiffStatus,
}

/// Overall semantic diff report between two Documents
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SemanticDiff {
    pub summary: DiffSummary,
    #[serde(alias = "changes")]
    pub objects: Vec<ObjectDiff>,
    pub symbols: Vec<SymbolDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DiffSummary {
    #[serde(rename = "added", alias = "added_count")]
    pub added_count: usize,
    #[serde(rename = "removed", alias = "removed_count")]
    pub removed_count: usize,
    #[serde(rename = "modified", alias = "modified_count")]
    pub modified_count: usize,
    #[serde(rename = "unchanged", alias = "unchanged_count")]
    pub unchanged_count: usize,
}

impl SemanticDiff {
    pub fn is_empty(&self) -> bool {
        self.summary.added_count == 0
            && self.summary.removed_count == 0
            && self.summary.modified_count == 0
            && self.symbols.is_empty()
    }

    pub fn count_text_changes(&self) -> usize {
        self.objects
            .iter()
            .filter(|o| {
                if let ObjectDiffStatus::Modified { changes } = &o.status {
                    changes
                        .iter()
                        .any(|c| c.field == "text" || c.field == "font-size")
                } else {
                    false
                }
            })
            .count()
    }

    pub fn count_geometry_changes(&self) -> usize {
        self.objects
            .iter()
            .filter(|o| {
                if let ObjectDiffStatus::Modified { changes } = &o.status {
                    changes.iter().any(|c| {
                        c.field == "geometry" || c.field == "node-count" || c.field == "size"
                    })
                } else {
                    false
                }
            })
            .count()
    }

    pub fn count_style_changes(&self) -> usize {
        self.objects
            .iter()
            .filter(|o| {
                if let ObjectDiffStatus::Modified { changes } = &o.status {
                    changes
                        .iter()
                        .any(|c| c.field == "fill" || c.field == "stroke" || c.field == "opacity")
                } else {
                    false
                }
            })
            .count()
    }

    pub fn count_transform_changes(&self) -> usize {
        self.objects
            .iter()
            .filter(|o| {
                if let ObjectDiffStatus::Modified { changes } = &o.status {
                    changes.iter().any(|c| {
                        c.field == "position" || c.field == "scale" || c.field == "rotation"
                    })
                } else {
                    false
                }
            })
            .count()
    }

    /// Format for human-readable CLI output
    pub fn format_text(&self) -> String {
        let mut out = String::new();

        if self.is_empty() {
            return "No semantic differences detected. Documents are functionally identical.\n"
                .to_string();
        }

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut modified = Vec::new();

        for obj in &self.objects {
            match &obj.status {
                ObjectDiffStatus::Added => added.push(obj),
                ObjectDiffStatus::Removed => removed.push(obj),
                ObjectDiffStatus::Modified { .. } => modified.push(obj),
            }
        }

        if !modified.is_empty() {
            out.push_str("Modified:\n");
            for obj in modified {
                let id_display = if !obj.id.is_empty() {
                    format!("#{}", obj.id)
                } else {
                    format!("\"{}\"", obj.name)
                };
                out.push_str(&format!("  {id_display} ({})\n", obj.object_type));
                if let ObjectDiffStatus::Modified { changes } = &obj.status {
                    for c in changes {
                        out.push_str(&format!(
                            "    {}: {} -> {}\n",
                            c.field, c.old_value, c.new_value
                        ));
                    }
                }
            }
            out.push('\n');
        }

        if !added.is_empty() {
            out.push_str("Added:\n");
            for obj in added {
                let id_display = if !obj.id.is_empty() {
                    format!("#{}", obj.id)
                } else {
                    format!("\"{}\"", obj.name)
                };
                out.push_str(&format!("  {id_display} ({})\n", obj.object_type));
            }
            out.push('\n');
        }

        if !removed.is_empty() {
            out.push_str("Removed:\n");
            for obj in removed {
                let id_display = if !obj.id.is_empty() {
                    format!("#{}", obj.id)
                } else {
                    format!("\"{}\"", obj.name)
                };
                out.push_str(&format!("  {id_display} ({})\n", obj.object_type));
            }
            out.push('\n');
        }

        if !self.symbols.is_empty() {
            out.push_str("Symbols / Components:\n");
            for sym in &self.symbols {
                match &sym.status {
                    ObjectDiffStatus::Added => {
                        out.push_str(&format!("  Added Symbol: #{}\n", sym.id))
                    }
                    ObjectDiffStatus::Removed => {
                        out.push_str(&format!("  Removed Symbol: #{}\n", sym.id))
                    }
                    ObjectDiffStatus::Modified { changes } => {
                        out.push_str(&format!("  Modified Symbol: #{}\n", sym.id));
                        for c in changes {
                            out.push_str(&format!(
                                "    {}: {} -> {}\n",
                                c.field, c.old_value, c.new_value
                            ));
                        }
                    }
                }
            }
            out.push('\n');
        }

        out.push_str(&format!(
            "Summary: {} added, {} removed, {} modified, {} unchanged\n",
            self.summary.added_count,
            self.summary.removed_count,
            self.summary.modified_count,
            self.summary.unchanged_count
        ));

        out
    }

    /// Format as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Compare two Document instances and return a SemanticDiff
pub fn compute_semantic_diff(doc_a: &Document, doc_b: &Document) -> SemanticDiff {
    let mut diff = SemanticDiff::default();

    // Flatten all objects from all layers with layer names and global positions
    let objs_a: Vec<(usize, &str, &Object)> = doc_a
        .layers
        .iter()
        .flat_map(|l| l.objects.iter().map(move |o| (l.name.as_str(), o)))
        .enumerate()
        .map(|(idx, (layer, o))| (idx, layer, o))
        .collect();

    let objs_b: Vec<(usize, &str, &Object)> = doc_b
        .layers
        .iter()
        .flat_map(|l| l.objects.iter().map(move |o| (l.name.as_str(), o)))
        .enumerate()
        .map(|(idx, (layer, o))| (idx, layer, o))
        .collect();

    // Helper: is this an explicit user/source ID or an ephemeral auto-generated index ID?
    fn is_explicit_id(id: &str) -> bool {
        !id.is_empty() && !id.starts_with("auto_")
    }

    // Match objects primarily by explicit user ID
    let mut a_by_id: HashMap<&str, (usize, &str, &Object)> = HashMap::new();
    let mut b_by_id: HashMap<&str, (usize, &str, &Object)> = HashMap::new();

    let mut a_unmatched: Vec<(usize, &str, &Object)> = Vec::new();
    let mut b_unmatched: Vec<(usize, &str, &Object)> = Vec::new();

    for item in objs_a.iter().copied() {
        if is_explicit_id(&item.2.id) {
            a_by_id.insert(&item.2.id, item);
        } else {
            a_unmatched.push(item);
        }
    }

    for item in objs_b.iter().copied() {
        if is_explicit_id(&item.2.id) {
            b_by_id.insert(&item.2.id, item);
        } else {
            b_unmatched.push(item);
        }
    }

    /// (index, id, object) pairs matched between the two documents.
    type MatchedPair<'a> = ((usize, &'a str, &'a Object), (usize, &'a str, &'a Object));
    let mut matched_pairs: Vec<MatchedPair<'_>> = Vec::new();
    let mut added_objs: Vec<&Object> = Vec::new();
    let mut removed_objs: Vec<&Object> = Vec::new();

    // 1. Match by Explicit ID
    let mut b_ids_used: HashSet<&str> = HashSet::new();
    for (id, meta_a) in &a_by_id {
        if let Some(meta_b) = b_by_id.get(id) {
            matched_pairs.push((*meta_a, *meta_b));
            b_ids_used.insert(id);
        } else {
            // If ID was removed/modified by external tool, don't drop immediately; let fallback matcher evaluate
            a_unmatched.push(*meta_a);
        }
    }

    for (id, meta_b) in &b_by_id {
        if !b_ids_used.contains(id) {
            b_unmatched.push(*meta_b);
        }
    }

    // 2. Fallback stable matching for unmatched objects by structural/geometric similarity
    // Pass 2A: Prioritize EXACT matches (score < 0.01) first to prevent greedy priority inversion
    let mut b_matched_indices: HashSet<usize> = HashSet::new();
    let mut a_still_unmatched: Vec<(usize, &str, &Object)> = Vec::new();

    for meta_a in a_unmatched {
        let obj_a = meta_a.2;
        let type_a = object_type_name(obj_a);
        let mut exact_match: Option<usize> = None;

        for (idx, meta_b) in b_unmatched.iter().enumerate() {
            if b_matched_indices.contains(&idx) {
                continue;
            }
            let obj_b = meta_b.2;
            if object_type_name(obj_b) == type_a {
                let dist = (obj_a.transform.x - obj_b.transform.x)
                    .hypot(obj_a.transform.y - obj_b.transform.y);
                let dim_score = match (&obj_a.object_type, &obj_b.object_type) {
                    (
                        ObjectType::Rectangle {
                            width: w1,
                            height: h1,
                            ..
                        },
                        ObjectType::Rectangle {
                            width: w2,
                            height: h2,
                            ..
                        },
                    ) => (w1 - w2).abs() + (h1 - h2).abs(),
                    (
                        ObjectType::Ellipse { rx: rx1, ry: ry1 },
                        ObjectType::Ellipse { rx: rx2, ry: ry2 },
                    ) => (rx1 - rx2).abs() + (ry1 - ry2).abs(),
                    (ObjectType::Text { text: t1, .. }, ObjectType::Text { text: t2, .. }) => {
                        if t1 == t2 {
                            0.0
                        } else {
                            30.0
                        }
                    }
                    (ObjectType::Path(p1), ObjectType::Path(p2)) => {
                        (p1.elements.len() as f64 - p2.elements.len() as f64).abs() * 5.0
                    }
                    _ => 0.0,
                };
                if dist + dim_score < 0.01 {
                    exact_match = Some(idx);
                    break;
                }
            }
        }

        if let Some(idx) = exact_match {
            matched_pairs.push((meta_a, b_unmatched[idx]));
            b_matched_indices.insert(idx);
        } else {
            a_still_unmatched.push(meta_a);
        }
    }

    // Pass 2B: Proximity / similarity match for remaining genuinely modified objects
    for meta_a in a_still_unmatched {
        let obj_a = meta_a.2;
        let mut best_match: Option<(usize, f64)> = None;
        let type_a = object_type_name(obj_a);

        for (idx, meta_b) in b_unmatched.iter().enumerate() {
            if b_matched_indices.contains(&idx) {
                continue;
            }
            let obj_b = meta_b.2;
            if object_type_name(obj_b) == type_a {
                let dist = (obj_a.transform.x - obj_b.transform.x)
                    .hypot(obj_a.transform.y - obj_b.transform.y);
                let dim_score = match (&obj_a.object_type, &obj_b.object_type) {
                    (
                        ObjectType::Rectangle {
                            width: w1,
                            height: h1,
                            ..
                        },
                        ObjectType::Rectangle {
                            width: w2,
                            height: h2,
                            ..
                        },
                    ) => (w1 - w2).abs() + (h1 - h2).abs(),
                    (
                        ObjectType::Ellipse { rx: rx1, ry: ry1 },
                        ObjectType::Ellipse { rx: rx2, ry: ry2 },
                    ) => (rx1 - rx2).abs() + (ry1 - ry2).abs(),
                    (ObjectType::Text { text: t1, .. }, ObjectType::Text { text: t2, .. }) => {
                        if t1 == t2 {
                            0.0
                        } else {
                            30.0
                        }
                    }
                    (ObjectType::Path(p1), ObjectType::Path(p2)) => {
                        (p1.elements.len() as f64 - p2.elements.len() as f64).abs() * 5.0
                    }
                    _ => 0.0,
                };
                let score = dist + dim_score;
                if best_match.as_ref().is_none_or(|(_, d)| score < *d) {
                    best_match = Some((idx, score));
                }
            }
        }

        if let Some((idx, score)) = best_match {
            if score < 300.0 {
                matched_pairs.push((meta_a, b_unmatched[idx]));
                b_matched_indices.insert(idx);
                continue;
            }
        }
        removed_objs.push(obj_a);
    }

    for (idx, meta_b) in b_unmatched.iter().enumerate() {
        if !b_matched_indices.contains(&idx) {
            added_objs.push(meta_b.2);
        }
    }

    // Determine relative z-order ranks among matched objects with O(1) hash map lookup
    let matched_set_a: HashSet<*const Object> = matched_pairs
        .iter()
        .map(|(a, _)| a.2 as *const Object)
        .collect();
    let matched_set_b: HashSet<*const Object> = matched_pairs
        .iter()
        .map(|(_, b)| b.2 as *const Object)
        .collect();

    let order_a: Vec<*const Object> = objs_a
        .iter()
        .map(|(_, _, o)| *o as *const Object)
        .filter(|p| matched_set_a.contains(p))
        .collect();
    let order_b: Vec<*const Object> = objs_b
        .iter()
        .map(|(_, _, o)| *o as *const Object)
        .filter(|p| matched_set_b.contains(p))
        .collect();

    let rank_map_a: HashMap<*const Object, usize> =
        order_a.iter().enumerate().map(|(i, p)| (*p, i)).collect();
    let rank_map_b: HashMap<*const Object, usize> =
        order_b.iter().enumerate().map(|(i, p)| (*p, i)).collect();

    // 3. Inspect differences in matched pairs
    for (meta_a, meta_b) in matched_pairs {
        let (_idx_a, layer_a, obj_a) = meta_a;
        let (_idx_b, layer_b, obj_b) = meta_b;

        let mut changes = compare_objects(obj_a, obj_b);

        if layer_a != layer_b {
            changes.push(FieldDiff {
                field: "group membership".to_string(),
                old_value: layer_a.to_string(),
                new_value: layer_b.to_string(),
            });
        }

        let rank_a = rank_map_a.get(&(obj_a as *const Object)).copied();
        let rank_b = rank_map_b.get(&(obj_b as *const Object)).copied();
        if rank_a != rank_b {
            changes.push(FieldDiff {
                field: "z-order".to_string(),
                old_value: format!("index {}", rank_a.unwrap_or(0)),
                new_value: format!("index {}", rank_b.unwrap_or(0)),
            });
        }

        if changes.is_empty() {
            diff.summary.unchanged_count += 1;
        } else {
            diff.summary.modified_count += 1;
            diff.objects.push(ObjectDiff {
                id: if !obj_b.id.is_empty() {
                    obj_b.id.clone()
                } else {
                    obj_a.id.clone()
                },
                name: obj_b.name.clone(),
                object_type: object_type_name(obj_b).to_string(),
                status: ObjectDiffStatus::Modified { changes },
            });
        }
    }

    // 4. Record additions
    for obj in added_objs {
        diff.summary.added_count += 1;
        diff.objects.push(ObjectDiff {
            id: obj.id.clone(),
            name: obj.name.clone(),
            object_type: object_type_name(obj).to_string(),
            status: ObjectDiffStatus::Added,
        });
    }

    // 5. Record removals
    for obj in removed_objs {
        diff.summary.removed_count += 1;
        diff.objects.push(ObjectDiff {
            id: obj.id.clone(),
            name: obj.name.clone(),
            object_type: object_type_name(obj).to_string(),
            status: ObjectDiffStatus::Removed,
        });
    }

    // 6. Compare Symbols / Components
    let mut a_syms: HashMap<&str, &crate::core::document::Symbol> = HashMap::new();
    let mut b_syms: HashMap<&str, &crate::core::document::Symbol> = HashMap::new();
    for s in &doc_a.symbols {
        a_syms.insert(&s.id, s);
    }
    for s in &doc_b.symbols {
        b_syms.insert(&s.id, s);
    }

    for (id, s_a) in &a_syms {
        if let Some(s_b) = b_syms.get(id) {
            let changes = compare_objects(&s_a.object, &s_b.object);
            if !changes.is_empty() {
                diff.symbols.push(SymbolDiff {
                    id: id.to_string(),
                    name: s_b.name.clone(),
                    status: ObjectDiffStatus::Modified { changes },
                });
            }
        } else {
            diff.symbols.push(SymbolDiff {
                id: id.to_string(),
                name: s_a.name.clone(),
                status: ObjectDiffStatus::Removed,
            });
        }
    }

    for (id, s_b) in &b_syms {
        if !a_syms.contains_key(id) {
            diff.symbols.push(SymbolDiff {
                id: id.to_string(),
                name: s_b.name.clone(),
                status: ObjectDiffStatus::Added,
            });
        }
    }

    diff
}

fn object_type_name(obj: &Object) -> &'static str {
    match &obj.object_type {
        ObjectType::Path(_) => "Path",
        ObjectType::Rectangle { .. } => "Rectangle",
        ObjectType::Ellipse { .. } => "Ellipse",
        ObjectType::Line { .. } => "Line",
        ObjectType::Text { .. } => "Text",
        ObjectType::Group(_) => "Group",
        ObjectType::ClippingMask { .. } => "ClippingMask",
        ObjectType::Star { .. } => "Star",
        ObjectType::Polygon { .. } => "Polygon",
        ObjectType::Use { .. } => "UseInstance",
        ObjectType::Image { .. } => "Image",
        ObjectType::PixelArt(_) => "PixelArt",
    }
}

fn compare_objects(a: &Object, b: &Object) -> Vec<FieldDiff> {
    let mut changes = Vec::new();

    // 1. Transform / Position / Scale / Rotation
    if (a.transform.x - b.transform.x).abs() > 0.01 || (a.transform.y - b.transform.y).abs() > 0.01
    {
        changes.push(FieldDiff {
            field: "position".to_string(),
            old_value: format!("({:.1}, {:.1})", a.transform.x, a.transform.y),
            new_value: format!("({:.1}, {:.1})", b.transform.x, b.transform.y),
        });
    }

    if (a.transform.scale_x - b.transform.scale_x).abs() > 0.01
        || (a.transform.scale_y - b.transform.scale_y).abs() > 0.01
    {
        changes.push(FieldDiff {
            field: "scale".to_string(),
            old_value: format!("({:.2}, {:.2})", a.transform.scale_x, a.transform.scale_y),
            new_value: format!("({:.2}, {:.2})", b.transform.scale_x, b.transform.scale_y),
        });
    }

    if (a.transform.rotation - b.transform.rotation).abs() > 0.01 {
        changes.push(FieldDiff {
            field: "rotation".to_string(),
            old_value: format!("{:.1}°", a.transform.rotation.to_degrees()),
            new_value: format!("{:.1}°", b.transform.rotation.to_degrees()),
        });
    }

    // 2. Opacity
    if (a.opacity - b.opacity).abs() > 0.01 {
        changes.push(FieldDiff {
            field: "opacity".to_string(),
            old_value: format!("{:.2}", a.opacity),
            new_value: format!("{:.2}", b.opacity),
        });
    }

    // 3. Fill
    let fill_a = format_fill(&a.fill);
    let fill_b = format_fill(&b.fill);
    if fill_a != fill_b {
        changes.push(FieldDiff {
            field: "fill".to_string(),
            old_value: fill_a,
            new_value: fill_b,
        });
    }

    // 4. Stroke
    let stroke_a = format_stroke(&a.stroke);
    let stroke_b = format_stroke(&b.stroke);
    if stroke_a != stroke_b {
        changes.push(FieldDiff {
            field: "stroke".to_string(),
            old_value: stroke_a,
            new_value: stroke_b,
        });
    }

    // 5. Type-specific checks (Text, Path geometry, Rectangle dimensions, Use instance)
    match (&a.object_type, &b.object_type) {
        (
            ObjectType::Text {
                text: t_a,
                font_size: s_a,
                style: style_a,
                area: area_a,
            },
            ObjectType::Text {
                text: t_b,
                font_size: s_b,
                style: style_b,
                area: area_b,
            },
        ) => {
            if t_a != t_b {
                changes.push(FieldDiff {
                    field: "text".to_string(),
                    old_value: format!("\"{t_a}\""),
                    new_value: format!("\"{t_b}\""),
                });
            }
            if (s_a - s_b).abs() > 0.1 {
                changes.push(FieldDiff {
                    field: "font-size".to_string(),
                    old_value: format!("{s_a:.0}"),
                    new_value: format!("{s_b:.0}"),
                });
            }
            if style_a.font_family != style_b.font_family {
                changes.push(FieldDiff {
                    field: "font-family".to_string(),
                    old_value: style_a.font_family.clone(),
                    new_value: style_b.font_family.clone(),
                });
            }
            if style_a.font_weight != style_b.font_weight {
                changes.push(FieldDiff {
                    field: "font-weight".to_string(),
                    old_value: style_a.font_weight.to_string(),
                    new_value: style_b.font_weight.to_string(),
                });
            }
            if style_a.font_style != style_b.font_style {
                changes.push(FieldDiff {
                    field: "font-style".to_string(),
                    old_value: style_a.font_style.as_svg_str().to_string(),
                    new_value: style_b.font_style.as_svg_str().to_string(),
                });
            }
            if (style_a.letter_spacing - style_b.letter_spacing).abs() > 0.01 {
                changes.push(FieldDiff {
                    field: "letter-spacing".to_string(),
                    old_value: format!("{:.1}", style_a.letter_spacing),
                    new_value: format!("{:.1}", style_b.letter_spacing),
                });
            }
            if style_a.text_anchor != style_b.text_anchor {
                changes.push(FieldDiff {
                    field: "text-anchor".to_string(),
                    old_value: style_a.text_anchor.as_svg_str().to_string(),
                    new_value: style_b.text_anchor.as_svg_str().to_string(),
                });
            }
            if area_a != area_b {
                let fmt = |a: &Option<crate::core::document::TextArea>| match a {
                    Some(r) => format!("{:.0}x{:.0}@{:.0},{:.0}", r.width, r.height, r.x, r.y),
                    None => "point".to_string(),
                };
                changes.push(FieldDiff {
                    field: "text-area".to_string(),
                    old_value: fmt(area_a),
                    new_value: fmt(area_b),
                });
            }
        }
        (ObjectType::Path(p_a), ObjectType::Path(p_b)) => {
            if p_a.elements.len() != p_b.elements.len() {
                changes.push(FieldDiff {
                    field: "node-count".to_string(),
                    old_value: p_a.elements.len().to_string(),
                    new_value: p_b.elements.len().to_string(),
                });
            } else {
                // Check if geometry coordinates changed
                let poly_a = p_a.to_polygon(8);
                let poly_b = p_b.to_polygon(8);
                let bbox_a = path_bbox(&poly_a);
                let bbox_b = path_bbox(&poly_b);
                if (bbox_a.0 - bbox_b.0).abs() > 0.5
                    || (bbox_a.1 - bbox_b.1).abs() > 0.5
                    || (bbox_a.2 - bbox_b.2).abs() > 0.5
                    || (bbox_a.3 - bbox_b.3).abs() > 0.5
                {
                    changes.push(FieldDiff {
                        field: "geometry".to_string(),
                        old_value: format!("{:.0}x{:.0}", bbox_a.2, bbox_a.3),
                        new_value: format!("{:.0}x{:.0}", bbox_b.2, bbox_b.3),
                    });
                }
            }
        }
        (
            ObjectType::Rectangle {
                width: w_a,
                height: h_a,
                corner_radius: r_a,
            },
            ObjectType::Rectangle {
                width: w_b,
                height: h_b,
                corner_radius: r_b,
            },
        ) => {
            if (w_a - w_b).abs() > 0.1 || (h_a - h_b).abs() > 0.1 {
                changes.push(FieldDiff {
                    field: "size".to_string(),
                    old_value: format!("{w_a:.1}x{h_a:.1}"),
                    new_value: format!("{w_b:.1}x{h_b:.1}"),
                });
            }
            if (r_a - r_b).abs() > 0.1 {
                changes.push(FieldDiff {
                    field: "corner-radius".to_string(),
                    old_value: format!("{r_a:.1}"),
                    new_value: format!("{r_b:.1}"),
                });
            }
        }
        (
            ObjectType::Use {
                href: href_a,
                width: w_a,
                height: h_a,
            },
            ObjectType::Use {
                href: href_b,
                width: w_b,
                height: h_b,
            },
        ) => {
            if href_a != href_b {
                changes.push(FieldDiff {
                    field: "component-href".to_string(),
                    old_value: href_a.clone(),
                    new_value: href_b.clone(),
                });
            }
            if w_a != w_b || h_a != h_b {
                changes.push(FieldDiff {
                    field: "instance-size".to_string(),
                    old_value: format!("{:?}x{:?}", w_a, h_a),
                    new_value: format!("{:?}x{:?}", w_b, h_b),
                });
            }
        }
        (
            ObjectType::Image {
                width: w_a,
                height: h_a,
                png_bytes: b_a,
            },
            ObjectType::Image {
                width: w_b,
                height: h_b,
                png_bytes: b_b,
            },
        ) => {
            if (w_a - w_b).abs() > 1e-6 || (h_a - h_b).abs() > 1e-6 {
                changes.push(FieldDiff {
                    field: "image-size".to_string(),
                    old_value: format!("{w_a:.1}x{h_a:.1}"),
                    new_value: format!("{w_b:.1}x{h_b:.1}"),
                });
            }
            if b_a.len() != b_b.len() {
                changes.push(FieldDiff {
                    field: "image-content".to_string(),
                    old_value: format!("{} bytes", b_a.len()),
                    new_value: format!("{} bytes", b_b.len()),
                });
            }
        }
        (ObjectType::Group(c_a), ObjectType::Group(c_b)) => {
            if c_a.len() != c_b.len() {
                changes.push(FieldDiff {
                    field: "group-children-count".to_string(),
                    old_value: c_a.len().to_string(),
                    new_value: c_b.len().to_string(),
                });
            }
        }
        _ => {
            let name_a = object_type_name(a);
            let name_b = object_type_name(b);
            if name_a != name_b {
                changes.push(FieldDiff {
                    field: "object-type".to_string(),
                    old_value: name_a.to_string(),
                    new_value: name_b.to_string(),
                });
            }
        }
    }

    changes
}

fn format_fill(fill: &Option<crate::core::path::FillStyle>) -> String {
    match fill {
        None => "none".to_string(),
        Some(f) => match &f.fill_type {
            FillType::Solid(c) => format_color(c),
            FillType::Linear(lg) => {
                format!(
                    "linear-gradient({} stops: {})",
                    lg.stops.len(),
                    format_stops(&lg.stops)
                )
            }
            FillType::Radial(rg) => {
                format!(
                    "radial-gradient({} stops: {})",
                    rg.stops.len(),
                    format_stops(&rg.stops)
                )
            }
            FillType::Pattern(_) => "pattern".to_string(),
            FillType::Image(ImageFill { image_id, .. }) => {
                format!("image-fill({})", image_id)
            },
        },
    }
}

fn format_stops(stops: &[GradientStop]) -> String {
    stops
        .iter()
        .map(|s| format!("{:.0}%:{}", s.offset * 100.0, format_color(&s.color)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_stroke(stroke: &Option<crate::core::path::StrokeStyle>) -> String {
    match stroke {
        None => "none".to_string(),
        Some(s) => format!("{} (width: {:.1})", format_color(&s.color), s.width),
    }
}

fn format_color(c: &[f32; 4]) -> String {
    let r = (c[0] * 255.0) as u8;
    let g = (c[1] * 255.0) as u8;
    let b = (c[2] * 255.0) as u8;
    if (c[3] - 1.0).abs() < 1e-3 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("rgba({r},{g},{b},{:.2})", c[3])
    }
}

fn path_bbox(pts: &[crate::core::path::AnchorPoint]) -> (f64, f64, f64, f64) {
    if pts.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in pts {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }

    (
        min_x,
        min_y,
        (max_x - min_x).max(0.0),
        (max_y - min_y).max(0.0),
    )
}
