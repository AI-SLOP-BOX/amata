use crate::core::document::{
    Document, FontStyle, Object, ObjectType, TextAnchor, TextStyle, Transform,
};
use crate::core::path::{
    AnchorPoint, BezierSegment, FillStyle, FillType, PathData, PathElement, StrokeStyle,
};
use std::collections::HashMap;

pub fn parse_svg_path_data(d: &str) -> Result<Vec<PathElement>, String> {
    let mut elements = Vec::new();
    let mut curr_pos = AnchorPoint::new(0.0, 0.0);
    let mut start_pos = AnchorPoint::new(0.0, 0.0);

    let mut tokens = Vec::new();
    let mut curr_token = String::new();

    let chars: Vec<char> = d.chars().collect();
    let mut ci = 0;
    while ci < chars.len() {
        let c = chars[ci];
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            if !curr_token.trim().is_empty() {
                tokens.push(curr_token.trim().to_string());
                curr_token.clear();
            }
            tokens.push(c.to_string());
            ci += 1;
        } else if c == '-' {
            if !curr_token.trim().is_empty()
                && !curr_token.ends_with('e')
                && !curr_token.ends_with('E')
            {
                tokens.push(curr_token.trim().to_string());
                curr_token.clear();
            }
            curr_token.push(c);
            ci += 1;
        } else if c.is_whitespace() || c == ',' {
            if !curr_token.trim().is_empty() {
                tokens.push(curr_token.trim().to_string());
                curr_token.clear();
            }
            ci += 1;
        } else {
            curr_token.push(c);
            ci += 1;
        }
    }
    if !curr_token.trim().is_empty() {
        tokens.push(curr_token.trim().to_string());
    }

    let mut i = 0;
    let mut current_cmd = String::new();

    while i < tokens.len() {
        if tokens[i]
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() && c != 'e' && c != 'E')
            .unwrap_or(false)
        {
            current_cmd = tokens[i].clone();
            i += 1;
        }

        match current_cmd.as_str() {
            "M" | "m" => {
                let is_rel = current_cmd == "m";
                if i + 1 < tokens.len() {
                    let x: f64 = tokens[i].parse().map_err(|e| format!("M.x: {e}"))?;
                    let y: f64 = tokens[i + 1].parse().map_err(|e| format!("M.y: {e}"))?;
                    curr_pos = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(x, y)
                    };
                    start_pos = curr_pos;
                    elements.push(PathElement::MoveTo(curr_pos));
                    i += 2;
                    // Subsequent coordinates following M/m are treated as L/l
                    current_cmd = if is_rel {
                        "l".to_string()
                    } else {
                        "L".to_string()
                    };
                } else {
                    break;
                }
            }
            "L" | "l" => {
                let is_rel = current_cmd == "l";
                if i + 1 < tokens.len() {
                    let x: f64 = tokens[i].parse().map_err(|e| format!("L.x: {e}"))?;
                    let y: f64 = tokens[i + 1].parse().map_err(|e| format!("L.y: {e}"))?;
                    curr_pos = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(x, y)
                    };
                    elements.push(PathElement::LineTo(curr_pos));
                    i += 2;
                } else {
                    break;
                }
            }
            "H" | "h" => {
                let is_rel = current_cmd == "h";
                if i < tokens.len() {
                    let x: f64 = tokens[i].parse().map_err(|e| format!("H.x: {e}"))?;
                    curr_pos = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y)
                    } else {
                        AnchorPoint::new(x, curr_pos.y)
                    };
                    elements.push(PathElement::LineTo(curr_pos));
                    i += 1;
                } else {
                    break;
                }
            }
            "V" | "v" => {
                let is_rel = current_cmd == "v";
                if i < tokens.len() {
                    let y: f64 = tokens[i].parse().map_err(|e| format!("V.y: {e}"))?;
                    curr_pos = if is_rel {
                        AnchorPoint::new(curr_pos.x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(curr_pos.x, y)
                    };
                    elements.push(PathElement::LineTo(curr_pos));
                    i += 1;
                } else {
                    break;
                }
            }
            "C" | "c" => {
                let is_rel = current_cmd == "c";
                if i + 5 < tokens.len() {
                    let x1: f64 = tokens[i].parse().map_err(|e| format!("C.x1: {e}"))?;
                    let y1: f64 = tokens[i + 1].parse().map_err(|e| format!("C.y1: {e}"))?;
                    let x2: f64 = tokens[i + 2].parse().map_err(|e| format!("C.x2: {e}"))?;
                    let y2: f64 = tokens[i + 3].parse().map_err(|e| format!("C.y2: {e}"))?;
                    let x: f64 = tokens[i + 4].parse().map_err(|e| format!("C.x: {e}"))?;
                    let y: f64 = tokens[i + 5].parse().map_err(|e| format!("C.y: {e}"))?;

                    let c0 = if is_rel {
                        AnchorPoint::new(curr_pos.x + x1, curr_pos.y + y1)
                    } else {
                        AnchorPoint::new(x1, y1)
                    };
                    let c1 = if is_rel {
                        AnchorPoint::new(curr_pos.x + x2, curr_pos.y + y2)
                    } else {
                        AnchorPoint::new(x2, y2)
                    };
                    let dest = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(x, y)
                    };

                    elements.push(PathElement::CurveTo(BezierSegment::cubic(
                        curr_pos, c0, c1, dest,
                    )));
                    curr_pos = dest;
                    i += 6;
                } else {
                    break;
                }
            }
            "S" | "s" => {
                let is_rel = current_cmd == "s";
                if i + 3 < tokens.len() {
                    let x2: f64 = tokens[i].parse().map_err(|e| format!("S.x2: {e}"))?;
                    let y2: f64 = tokens[i + 1].parse().map_err(|e| format!("S.y2: {e}"))?;
                    let x: f64 = tokens[i + 2].parse().map_err(|e| format!("S.x: {e}"))?;
                    let y: f64 = tokens[i + 3].parse().map_err(|e| format!("S.y: {e}"))?;

                    let c0 = if let Some(PathElement::CurveTo(prev_seg)) = elements.last() {
                        AnchorPoint::new(
                            2.0 * curr_pos.x - prev_seg.control2.x,
                            2.0 * curr_pos.y - prev_seg.control2.y,
                        )
                    } else {
                        curr_pos
                    };

                    let c1 = if is_rel {
                        AnchorPoint::new(curr_pos.x + x2, curr_pos.y + y2)
                    } else {
                        AnchorPoint::new(x2, y2)
                    };
                    let dest = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(x, y)
                    };

                    elements.push(PathElement::CurveTo(BezierSegment::cubic(
                        curr_pos, c0, c1, dest,
                    )));
                    curr_pos = dest;
                    i += 4;
                } else {
                    break;
                }
            }
            "Q" | "q" => {
                let is_rel = current_cmd == "q";
                if i + 3 < tokens.len() {
                    let x1: f64 = tokens[i].parse().map_err(|e| format!("Q.x1: {e}"))?;
                    let y1: f64 = tokens[i + 1].parse().map_err(|e| format!("Q.y1: {e}"))?;
                    let x: f64 = tokens[i + 2].parse().map_err(|e| format!("Q.x: {e}"))?;
                    let y: f64 = tokens[i + 3].parse().map_err(|e| format!("Q.y: {e}"))?;

                    let qp = if is_rel {
                        AnchorPoint::new(curr_pos.x + x1, curr_pos.y + y1)
                    } else {
                        AnchorPoint::new(x1, y1)
                    };
                    let dest = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(x, y)
                    };

                    let c0 = AnchorPoint::new(
                        curr_pos.x + (2.0 / 3.0) * (qp.x - curr_pos.x),
                        curr_pos.y + (2.0 / 3.0) * (qp.y - curr_pos.y),
                    );
                    let c1 = AnchorPoint::new(
                        dest.x + (2.0 / 3.0) * (qp.x - dest.x),
                        dest.y + (2.0 / 3.0) * (qp.y - dest.y),
                    );

                    elements.push(PathElement::CurveTo(BezierSegment::cubic(
                        curr_pos, c0, c1, dest,
                    )));
                    curr_pos = dest;
                    i += 4;
                } else {
                    break;
                }
            }
            "T" | "t" => {
                let is_rel = current_cmd == "t";
                if i + 1 < tokens.len() {
                    let x: f64 = tokens[i].parse().map_err(|e| format!("T.x: {e}"))?;
                    let y: f64 = tokens[i + 1].parse().map_err(|e| format!("T.y: {e}"))?;

                    let dest = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(x, y)
                    };

                    let qp = if let Some(PathElement::CurveTo(prev_seg)) = elements.last() {
                        let approx_qp_x = (3.0 * prev_seg.control2.x - prev_seg.end.x) / 2.0;
                        let approx_qp_y = (3.0 * prev_seg.control2.y - prev_seg.end.y) / 2.0;
                        AnchorPoint::new(
                            2.0 * curr_pos.x - approx_qp_x,
                            2.0 * curr_pos.y - approx_qp_y,
                        )
                    } else {
                        curr_pos
                    };

                    let c0 = AnchorPoint::new(
                        curr_pos.x + (2.0 / 3.0) * (qp.x - curr_pos.x),
                        curr_pos.y + (2.0 / 3.0) * (qp.y - curr_pos.y),
                    );
                    let c1 = AnchorPoint::new(
                        dest.x + (2.0 / 3.0) * (qp.x - dest.x),
                        dest.y + (2.0 / 3.0) * (qp.y - dest.y),
                    );

                    elements.push(PathElement::CurveTo(BezierSegment::cubic(
                        curr_pos, c0, c1, dest,
                    )));
                    curr_pos = dest;
                    i += 2;
                } else {
                    break;
                }
            }
            "A" | "a" => {
                let is_rel = current_cmd == "a";
                if i + 6 < tokens.len() {
                    let rx: f64 = tokens[i].parse::<f64>().unwrap_or(0.0).abs();
                    let ry: f64 = tokens[i + 1].parse::<f64>().unwrap_or(0.0).abs();
                    let _rot: f64 = tokens[i + 2].parse().unwrap_or(0.0);
                    let _large_arc: f64 = tokens[i + 3].parse().unwrap_or(0.0);
                    let _sweep: f64 = tokens[i + 4].parse().unwrap_or(0.0);
                    let x: f64 = tokens[i + 5].parse().map_err(|e| format!("A.x: {e}"))?;
                    let y: f64 = tokens[i + 6].parse().map_err(|e| format!("A.y: {e}"))?;

                    let dest = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(x, y)
                    };

                    if rx == 0.0 || ry == 0.0 {
                        elements.push(PathElement::LineTo(dest));
                    } else {
                        let mid_x = (curr_pos.x + dest.x) / 2.0;
                        let mid_y = (curr_pos.y + dest.y) / 2.0;
                        let c0 = AnchorPoint::new(
                            curr_pos.x + (mid_x - curr_pos.x) * 0.55,
                            curr_pos.y + (mid_y - curr_pos.y) * 0.55,
                        );
                        let c1 = AnchorPoint::new(
                            dest.x - (dest.x - mid_x) * 0.55,
                            dest.y - (dest.y - mid_y) * 0.55,
                        );
                        elements.push(PathElement::CurveTo(BezierSegment::cubic(
                            curr_pos, c0, c1, dest,
                        )));
                    }
                    curr_pos = dest;
                    i += 7;
                } else {
                    break;
                }
            }
            "Z" | "z" => {
                elements.push(PathElement::ClosePath);
                curr_pos = start_pos;
                current_cmd.clear();
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(elements)
}

pub fn parse_svg_color(color_str: &str) -> Option<[f32; 4]> {
    let s = color_str.trim();
    if let Some(hex) = s.strip_prefix('#') {
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some([r as f32 / 15.0, g as f32 / 15.0, b as f32 / 15.0, 1.0])
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some([
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    a as f32 / 255.0,
                ])
            }
            _ => None,
        }
    } else if s.starts_with("rgb") {
        let inner = s.trim_start_matches("rgb").trim_start_matches('a');
        let inner = inner.trim_start_matches('(').trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 3 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            let a = if parts.len() > 3 {
                parts[3].parse::<f32>().unwrap_or(1.0)
            } else {
                1.0
            };
            Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a])
        } else {
            None
        }
    } else {
        match s {
            "black" => Some([0.0, 0.0, 0.0, 1.0]),
            "white" => Some([1.0, 1.0, 1.0, 1.0]),
            "red" => Some([1.0, 0.0, 0.0, 1.0]),
            "green" => Some([0.0, 1.0, 0.0, 1.0]),
            "blue" => Some([0.0, 0.0, 1.0, 1.0]),
            "yellow" => Some([1.0, 1.0, 0.0, 1.0]),
            "cyan" => Some([0.0, 1.0, 1.0, 1.0]),
            "magenta" => Some([1.0, 0.0, 1.0, 1.0]),
            "none" | "transparent" => None,
            _ => None,
        }
    }
}

pub fn parse_svg_document(svg_text: &str) -> Document {
    let mut doc = Document {
        name: "SVG Import".to_string(),
        ..Default::default()
    };
    let mut obj_count = 0;

    let tags = tokenize_svg_tags(svg_text);

    // 1. Scan and parse <defs> resources (Gradients)
    let mut gradients: HashMap<String, FillStyle> = HashMap::new();
    let mut i = 0;
    while i < tags.len() {
        let tag = &tags[i];
        let trimmed = tag.trim();

        if trimmed.starts_with("<linearGradient") {
            let mut child_tags = Vec::new();
            let mut j = i + 1;
            while j < tags.len() && !tags[j].trim().starts_with("</linearGradient>") {
                child_tags.push(tags[j].clone());
                j += 1;
            }
            let (id, fill) = parse_linear_gradient_tag(trimmed, &child_tags);
            if !id.is_empty() {
                gradients.insert(id, fill);
            }
            i = j;
        } else if trimmed.starts_with("<radialGradient") {
            let mut child_tags = Vec::new();
            let mut j = i + 1;
            while j < tags.len() && !tags[j].trim().starts_with("</radialGradient>") {
                child_tags.push(tags[j].clone());
                j += 1;
            }
            let (id, fill) = parse_radial_gradient_tag(trimmed, &child_tags);
            if !id.is_empty() {
                gradients.insert(id, fill);
            }
            i = j;
        } else if trimmed.starts_with("<symbol") {
            let sym_id = extract_attr_str(trimmed, "id").unwrap_or("").to_string();
            let mut child_tags = Vec::new();
            let mut j = i + 1;
            while j < tags.len() && !tags[j].trim().starts_with("</symbol>") {
                child_tags.push(tags[j].clone());
                j += 1;
            }
            if !sym_id.is_empty() {
                let inner_svg = format!("<svg>{}</svg>", child_tags.join("\n"));
                let inner_doc = parse_svg_document(&inner_svg);
                let inner_objects: Vec<_> =
                    inner_doc.all_objects().map(|(_, o)| o.clone()).collect();
                let master = if inner_objects.len() == 1 {
                    inner_objects.into_iter().next().unwrap()
                } else {
                    Object::new_group(&format!("Symbol {sym_id}"), inner_objects)
                };
                doc.add_symbol(crate::core::document::Symbol {
                    id: sym_id.clone(),
                    name: format!("Symbol {sym_id}"),
                    object: master,
                    use_count: 0,
                });
            }
            i = j;
        }
        i += 1;
    }

    // 2. Parse Elements & Nested Groups
    let mut group_stack: Vec<[f64; 6]> = vec![affine_identity()];
    let mut in_defs = false;

    for tag in &tags {
        let trimmed = tag.trim();

        if trimmed.starts_with("<defs") {
            in_defs = true;
            continue;
        }
        if trimmed.starts_with("</defs>") || trimmed.starts_with("</defs ") {
            in_defs = false;
            continue;
        }
        if in_defs {
            continue;
        }

        // Handle SVG root dimensions
        if trimmed.starts_with("<svg") {
            if let Some(w) = extract_attr_f64(trimmed, "width") {
                if w > 0.0 {
                    doc.width = w;
                }
            }
            if let Some(h) = extract_attr_f64(trimmed, "height") {
                if h > 0.0 {
                    doc.height = h;
                }
            }
            if let Some(vb) = extract_attr_str(trimmed, "viewBox") {
                let parts: Vec<f64> = vb
                    .split(|c: char| c.is_whitespace() || c == ',')
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if parts.len() == 4 && parts[2] > 0.0 && parts[3] > 0.0 {
                    if doc.width == 1920.0 && doc.height == 1080.0 {
                        doc.width = parts[2];
                        doc.height = parts[3];
                    }
                }
            }
            continue;
        }

        // Group opening
        if trimmed.starts_with("<g") {
            let local = parse_svg_transform(trimmed);
            let current = group_stack.last().copied().unwrap_or(affine_identity());
            group_stack.push(affine_multiply(&current, &local));
            continue;
        }

        // Group closing
        if trimmed.starts_with("</g>") || trimmed.starts_with("</g ") {
            if group_stack.len() > 1 {
                group_stack.pop();
            }
            continue;
        }

        let group_m = group_stack.last().copied().unwrap_or(affine_identity());
        let elem_m = parse_svg_transform(trimmed);
        let total_m = affine_multiply(&group_m, &elem_m);
        let (total_tx, total_ty) = (total_m[4], total_m[5]);
        let has_linear = (total_m[0] - 1.0).abs() > 1e-9
            || total_m[1].abs() > 1e-9
            || total_m[2].abs() > 1e-9
            || (total_m[3] - 1.0).abs() > 1e-9;

        if trimmed.starts_with("<path") {
            if let Some(d) = extract_attr_str(trimmed, "d") {
                if let Ok(mut elements) = parse_svg_path_data(d) {
                    if !elements.is_empty() {
                        if has_linear || total_tx != 0.0 || total_ty != 0.0 {
                            for elem in &mut elements {
                                match elem {
                                    PathElement::MoveTo(p) | PathElement::LineTo(p) => {
                                        let (nx, ny) = affine_apply(&total_m, p.x, p.y);
                                        p.x = nx;
                                        p.y = ny;
                                    }
                                    PathElement::CurveTo(seg) => {
                                        let (sx, sy) =
                                            affine_apply(&total_m, seg.start.x, seg.start.y);
                                        let (c1x, c1y) =
                                            affine_apply(&total_m, seg.control1.x, seg.control1.y);
                                        let (c2x, c2y) =
                                            affine_apply(&total_m, seg.control2.x, seg.control2.y);
                                        let (ex, ey) =
                                            affine_apply(&total_m, seg.end.x, seg.end.y);
                                        seg.start.x = sx;
                                        seg.start.y = sy;
                                        seg.control1.x = c1x;
                                        seg.control1.y = c1y;
                                        seg.control2.x = c2x;
                                        seg.control2.y = c2y;
                                        seg.end.x = ex;
                                        seg.end.y = ey;
                                    }
                                    PathElement::ClosePath => {}
                                }
                            }
                        }

                        let mut path = PathData::new();
                        path.elements = elements;
                        path.closed = path
                            .elements
                            .iter()
                            .any(|e| matches!(e, PathElement::ClosePath));
                        path.fill = extract_fill(trimmed, &gradients);
                        path.stroke = extract_stroke(trimmed);

                        obj_count += 1;
                        let mut obj = Object::new_path(&format!("Path {obj_count}"), path);
                        obj.id = extract_attr_str(trimmed, "id")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("auto_path_{obj_count}"));
                        if let Some(op) = extract_opacity(trimmed) {
                            obj.opacity = op;
                        }
                        doc.add_object(obj);
                    }
                }
            }
        } else if trimmed.starts_with("<polygon") || trimmed.starts_with("<polyline") {
            let is_polygon = trimmed.starts_with("<polygon");
            if let Some(points_str) = extract_attr_str(trimmed, "points") {
                let pts = parse_svg_points(points_str);
                if !pts.is_empty() {
                    let mut elements = Vec::new();
                    let (sx0, sy0) = affine_apply(&total_m, pts[0].x, pts[0].y);
                    elements.push(PathElement::MoveTo(AnchorPoint::new(sx0, sy0)));
                    for pt in &pts[1..] {
                        let (sx, sy) = affine_apply(&total_m, pt.x, pt.y);
                        elements.push(PathElement::LineTo(AnchorPoint::new(sx, sy)));
                    }
                    if is_polygon {
                        elements.push(PathElement::ClosePath);
                    }

                    let mut path = PathData::new();
                    path.elements = elements;
                    path.closed = is_polygon;
                    path.fill = extract_fill(trimmed, &gradients);
                    path.stroke = extract_stroke(trimmed);

                    obj_count += 1;
                    let label = if is_polygon {
                        format!("Polygon {obj_count}")
                    } else {
                        format!("Polyline {obj_count}")
                    };
                    let mut obj = Object::new_path(&label, path);
                    let prefix = if is_polygon { "polygon" } else { "polyline" };
                    obj.id = extract_attr_str(trimmed, "id")
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("auto_{prefix}_{obj_count}"));
                    if let Some(op) = extract_opacity(trimmed) {
                        obj.opacity = op;
                    }
                    doc.add_object(obj);
                }
            }
        } else if trimmed.starts_with("<rect") {
            if let (Some(x), Some(y), Some(w), Some(h)) = (
                extract_attr_f64(trimmed, "x"),
                extract_attr_f64(trimmed, "y"),
                extract_attr_f64(trimmed, "width"),
                extract_attr_f64(trimmed, "height"),
            ) {
                let rx = extract_attr_f64(trimmed, "rx").unwrap_or(0.0);
                obj_count += 1;
                let mut obj = Object::new_rect(&format!("Rect {obj_count}"), x, y, w, h, rx);
                if has_linear {
                    obj.transform = affine_to_transform(&total_m, x, y);
                } else {
                    obj.transform.x = x + total_tx;
                    obj.transform.y = y + total_ty;
                }
                obj.id = extract_attr_str(trimmed, "id")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("auto_rect_{obj_count}"));
                if let Some(fill) = extract_fill(trimmed, &gradients) {
                    obj.fill = Some(fill);
                }
                if let Some(stroke) = extract_stroke(trimmed) {
                    obj.stroke = Some(stroke);
                }
                if let Some(op) = extract_opacity(trimmed) {
                    obj.opacity = op;
                }
                doc.add_object(obj);
            }
        } else if trimmed.starts_with("<circle") || trimmed.starts_with("<ellipse") {
            if let (Some(cx), Some(cy), Some(r1), r2) = (
                extract_attr_f64(trimmed, "cx"),
                extract_attr_f64(trimmed, "cy"),
                extract_attr_f64(trimmed, "rx").or_else(|| extract_attr_f64(trimmed, "r")),
                extract_attr_f64(trimmed, "ry").or_else(|| extract_attr_f64(trimmed, "r")),
            ) {
                let ry = r2.unwrap_or(r1);
                obj_count += 1;
                let mut obj =
                    Object::new_ellipse(&format!("Ellipse {obj_count}"), cx, cy, r1, ry);
                if has_linear {
                    obj.transform = affine_to_transform(&total_m, cx, cy);
                } else {
                    obj.transform.x = cx + total_tx;
                    obj.transform.y = cy + total_ty;
                }
                obj.id = extract_attr_str(trimmed, "id")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("auto_ellipse_{obj_count}"));
                if let Some(fill) = extract_fill(trimmed, &gradients) {
                    obj.fill = Some(fill);
                }
                if let Some(stroke) = extract_stroke(trimmed) {
                    obj.stroke = Some(stroke);
                }
                if let Some(op) = extract_opacity(trimmed) {
                    obj.opacity = op;
                }
                doc.add_object(obj);
            }
        } else if trimmed.starts_with("<line") {
            if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                extract_attr_f64(trimmed, "x1"),
                extract_attr_f64(trimmed, "y1"),
                extract_attr_f64(trimmed, "x2"),
                extract_attr_f64(trimmed, "y2"),
            ) {
                obj_count += 1;
                let mut obj = if has_linear {
                    let mut o =
                        Object::new_line(&format!("Line {obj_count}"), 0.0, 0.0, x2 - x1, y2 - y1);
                    o.transform = affine_to_transform(&total_m, x1, y1);
                    o
                } else {
                    Object::new_line(
                        &format!("Line {obj_count}"),
                        x1 + total_tx,
                        y1 + total_ty,
                        x2 + total_tx,
                        y2 + total_ty,
                    )
                };
                obj.id = extract_attr_str(trimmed, "id")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("auto_line_{obj_count}"));
                if let Some(stroke) = extract_stroke(trimmed) {
                    obj.stroke = Some(stroke);
                }
                if let Some(op) = extract_opacity(trimmed) {
                    obj.opacity = op;
                }
                doc.add_object(obj);
            }
        } else if trimmed.starts_with("<text") {
            let x = extract_attr_f64(trimmed, "x").unwrap_or(0.0);
            let y = extract_attr_f64(trimmed, "y").unwrap_or(0.0);
            let font_size = extract_prop_str(trimmed, "font-size")
                .and_then(|s| {
                    s.trim()
                        .trim_end_matches("px")
                        .trim_end_matches("pt")
                        .parse::<f64>()
                        .ok()
                })
                .unwrap_or(24.0);
            let font_family = extract_prop_str(trimmed, "font-family")
                .unwrap_or_else(|| "Inter, sans-serif".to_string());
            let font_weight = extract_prop_str(trimmed, "font-weight")
                .map(|w| parse_font_weight(&w))
                .unwrap_or(400);
            let font_style = extract_prop_str(trimmed, "font-style")
                .map(|s| parse_font_style(&s))
                .unwrap_or(FontStyle::Normal);
            let letter_spacing = extract_prop_str(trimmed, "letter-spacing")
                .map(|ls| parse_letter_spacing(&ls))
                .unwrap_or(0.0);
            let text_anchor = extract_prop_str(trimmed, "text-anchor")
                .map(|ta| parse_text_anchor(&ta))
                .unwrap_or(TextAnchor::Start);

            let style = TextStyle {
                font_family,
                font_size,
                font_weight,
                font_style,
                letter_spacing,
                text_anchor,
            };

            let text_content = if let Some(start) = trimmed.find('>') {
                let rest = &trimmed[start + 1..];
                let raw_content = if let Some(end) = rest.find("</text>") {
                    rest[..end].trim()
                } else {
                    rest.trim()
                };
                xml_unescape(raw_content)
            } else {
                String::new()
            };
            if !text_content.is_empty() {
                obj_count += 1;
                let mut obj = Object::new_text_with_style(
                    &format!("Text {obj_count}"),
                    &text_content,
                    x,
                    y,
                    style,
                );
                if has_linear {
                    obj.transform = affine_to_transform(&total_m, x, y);
                } else {
                    obj.transform.x = x + total_tx;
                    obj.transform.y = y + total_ty;
                }
                obj.id = extract_attr_str(trimmed, "id")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("auto_text_{obj_count}"));
                if let Some(fill) = extract_fill(trimmed, &gradients) {
                    obj.fill = Some(fill);
                }
                if let Some(op) = extract_opacity(trimmed) {
                    obj.opacity = op;
                }
                doc.add_object(obj);
            }
        } else if trimmed.starts_with("<use") {
            let href = extract_attr_str(trimmed, "href")
                .or_else(|| extract_attr_str(trimmed, "xlink:href"))
                .unwrap_or("")
                .to_string();
            if !href.is_empty() {
                let clean_href = href.trim_start_matches('#');
                let x = extract_attr_f64(trimmed, "x").unwrap_or(0.0);
                let y = extract_attr_f64(trimmed, "y").unwrap_or(0.0);
                let w = extract_attr_f64(trimmed, "width");
                let h = extract_attr_f64(trimmed, "height");
                obj_count += 1;
                let mut obj =
                    Object::new_use(&format!("Use {obj_count}"), clean_href, x, y, w, h);
                if has_linear {
                    obj.transform = affine_to_transform(&total_m, x, y);
                } else {
                    obj.transform.x = x + total_tx;
                    obj.transform.y = y + total_ty;
                }
                obj.id = extract_attr_str(trimmed, "id")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("auto_use_{obj_count}"));
                if let Some(op) = extract_opacity(trimmed) {
                    obj.opacity = op;
                }
                doc.add_object(obj);
            }
        }
    }

    doc
}

fn parse_linear_gradient_tag(tag: &str, child_tags: &[String]) -> (String, FillStyle) {
    let id = extract_attr_str(tag, "id").unwrap_or("").to_string();
    let x1 = parse_coord_or_percent(extract_attr_str(tag, "x1").unwrap_or("0"));
    let y1 = parse_coord_or_percent(extract_attr_str(tag, "y1").unwrap_or("0"));
    let x2 = parse_coord_or_percent(extract_attr_str(tag, "x2").unwrap_or("1"));
    let y2 = parse_coord_or_percent(extract_attr_str(tag, "y2").unwrap_or("0"));

    let mut stops = Vec::new();
    for stop_tag in child_tags {
        if stop_tag.trim().starts_with("<stop") {
            // "NaN".parse::<f32>() succeeds; sanitize so NaN never poisons
            // gradient sampling/sorting downstream.
            let raw =
                parse_coord_or_percent(extract_attr_str(stop_tag, "offset").unwrap_or("0"));
            let offset = if raw.is_finite() {
                raw.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let color = extract_attr_str(stop_tag, "stop-color")
                .and_then(parse_svg_color)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let opacity = extract_attr_f64(stop_tag, "stop-opacity")
                .map(|v| v as f32)
                .unwrap_or(1.0);
            stops.push(crate::core::path::GradientStop {
                offset,
                color: [color[0], color[1], color[2], color[3] * opacity],
            });
        }
    }
    if stops.is_empty() {
        stops.push(crate::core::path::GradientStop {
            offset: 0.0,
            color: [0.0, 0.0, 0.0, 1.0],
        });
        stops.push(crate::core::path::GradientStop {
            offset: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }

    let grad = crate::core::path::LinearGradient {
        start_x: x1,
        start_y: y1,
        end_x: x2,
        end_y: y2,
        stops,
    };
    (
        id,
        FillStyle {
            color: [0.0, 0.0, 0.0, 1.0],
            fill_type: FillType::Linear(grad),
            rule: crate::core::path::FillRule::NonZero,
        },
    )
}

fn parse_radial_gradient_tag(tag: &str, child_tags: &[String]) -> (String, FillStyle) {
    let id = extract_attr_str(tag, "id").unwrap_or("").to_string();
    let cx = parse_coord_or_percent(extract_attr_str(tag, "cx").unwrap_or("0.5"));
    let cy = parse_coord_or_percent(extract_attr_str(tag, "cy").unwrap_or("0.5"));
    let r = parse_coord_or_percent(extract_attr_str(tag, "r").unwrap_or("0.5"));
    let fx = parse_coord_or_percent(extract_attr_str(tag, "fx").unwrap_or(&cx.to_string()));
    let fy = parse_coord_or_percent(extract_attr_str(tag, "fy").unwrap_or(&cy.to_string()));

    let mut stops = Vec::new();
    for stop_tag in child_tags {
        if stop_tag.trim().starts_with("<stop") {
            let raw =
                parse_coord_or_percent(extract_attr_str(stop_tag, "offset").unwrap_or("0"));
            let offset = if raw.is_finite() {
                raw.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let color = extract_attr_str(stop_tag, "stop-color")
                .and_then(parse_svg_color)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let opacity = extract_attr_f64(stop_tag, "stop-opacity")
                .map(|v| v as f32)
                .unwrap_or(1.0);
            stops.push(crate::core::path::GradientStop {
                offset,
                color: [color[0], color[1], color[2], color[3] * opacity],
            });
        }
    }
    if stops.is_empty() {
        stops.push(crate::core::path::GradientStop {
            offset: 0.0,
            color: [0.0, 0.0, 0.0, 1.0],
        });
        stops.push(crate::core::path::GradientStop {
            offset: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }

    let grad = crate::core::path::RadialGradient {
        center_x: cx,
        center_y: cy,
        radius: r,
        focus_x: fx,
        focus_y: fy,
        stops,
    };
    (
        id,
        FillStyle {
            color: [0.0, 0.0, 0.0, 1.0],
            fill_type: FillType::Radial(grad),
            rule: crate::core::path::FillRule::NonZero,
        },
    )
}

fn parse_coord_or_percent(s: &str) -> f32 {
    let trimmed = s.trim();
    if let Some(num) = trimmed.strip_suffix('%') {
        num.parse::<f32>().unwrap_or(0.0) / 100.0
    } else {
        trimmed.parse::<f32>().unwrap_or(0.0)
    }
}

fn tokenize_svg_tags(svg_text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut in_tag = false;
    let mut current_tag = String::new();
    let mut in_quote: Option<char> = None;
    let mut in_comment = false;
    // Index of the currently open <text> element so nested markup content
    // (<tspan>, <tref>, …) is accumulated into it instead of being dropped.
    // Text goes to a side buffer (flushed into the tag on </text>) so that
    // <tspan>/<br> boundaries can inject explicit line breaks.
    let mut open_text_idx: Option<usize> = None;
    let mut open_text_buf = String::new();
    // Whether the innermost open <tspan> carries positioning (x/y/dx/dy):
    // only those (and <br>) start a new line. Plain adjacent tspans are
    // same-line continuations and must not inject breaks.
    let mut tspan_break_pending = false;

    let chars: Vec<char> = svg_text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        // Check for XML comments <!-- ... -->
        if in_quote.is_none()
            && i + 3 < chars.len()
            && chars[i] == '<'
            && chars[i + 1] == '!'
            && chars[i + 2] == '-'
            && chars[i + 3] == '-'
        {
            in_comment = true;
            i += 4;
            continue;
        }
        if in_comment {
            if i + 2 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>'
            {
                in_comment = false;
                i += 3;
            } else {
                i += 1;
            }
            continue;
        }

        if let Some(q) = in_quote {
            current_tag.push(c);
            if c == q {
                in_quote = None;
            }
            i += 1;
        } else if c == '"' || c == '\'' {
            if in_tag {
                in_quote = Some(c);
            }
            current_tag.push(c);
            i += 1;
        } else if c == '<' {
            in_tag = true;
            current_tag.clear();
            current_tag.push(c);
            i += 1;
        } else if c == '>' && in_tag {
            in_tag = false;
            current_tag.push(c);
            tags.push(current_tag.clone());
            let pushed = tags.len() - 1;
            let tag_head = tags[pushed]
                .split(|ch: char| ch.is_whitespace() || ch == '>')
                .next()
                .unwrap_or("");
            if tag_head == "<text" {
                open_text_idx = Some(pushed);
                open_text_buf.clear();
            } else if tag_head == "</text" {
                // Flush accumulated text (with tspan/br line breaks) into
                // the <text> tag so content extraction below just works.
                if let (Some(idx), buf) = (open_text_idx, std::mem::take(&mut open_text_buf))
                {
                    if let Some(tag) = tags.get_mut(idx) {
                        tag.push_str(&buf);
                    }
                }
                open_text_idx = None;
            } else if open_text_idx.is_some() {
                if tag_head == "<tspan" {
                    let tag = &tags[pushed];
                    tspan_break_pending = tag.contains("x=")
                        || tag.contains("y=")
                        || tag.contains("dx=")
                        || tag.contains("dy=");
                } else if tag_head == "</tspan" {
                    if tspan_break_pending
                        && !open_text_buf.is_empty()
                        && !open_text_buf.ends_with('\n')
                    {
                        open_text_buf.push('\n');
                    }
                    tspan_break_pending = false;
                } else if tag_head.starts_with("<br") || tag_head.starts_with("</br") {
                    if !open_text_buf.is_empty() && !open_text_buf.ends_with('\n') {
                        open_text_buf.push('\n');
                    }
                }
            }
            current_tag.clear();
            i += 1;
        } else if in_tag {
            if c == '\n' || c == '\r' || c == '\t' {
                current_tag.push(' ');
            } else {
                current_tag.push(c);
            }
            i += 1;
        } else {
            // Text content outside tags (for <text>...</text>, including
            // nested <tspan> content which belongs to the open <text>)
            if open_text_idx.is_some() {
                open_text_buf.push(c);
            } else if let Some(last) = tags.last_mut() {
                if last.starts_with("<text") && !last.contains("</text>") {
                    last.push(c);
                }
            }
            i += 1;
        }
    }
    tags
}

fn parse_svg_points(points_str: &str) -> Vec<AnchorPoint> {
    let nums: Vec<f64> = points_str
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    let mut points = Vec::new();
    for chunk in nums.chunks(2) {
        if chunk.len() == 2 {
            points.push(AnchorPoint::new(chunk[0], chunk[1]));
        }
    }
    points
}

fn extract_attr_str<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    for quote in ['"', '\''] {
        let pattern = format!("{attr}={quote}");
        let mut search_from = 0;
        while let Some(rel_start) = tag[search_from..].find(&pattern) {
            let start = search_from + rel_start;
            let is_boundary = if start == 0 {
                true
            } else {
                tag[..start]
                    .chars()
                    .last()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
            };

            if is_boundary {
                let rest = &tag[start + pattern.len()..];
                if let Some(end) = rest.find(quote) {
                    return Some(&rest[..end]);
                }
            }
            search_from = start + pattern.len();
        }
    }
    None
}

fn extract_from_style<'a>(tag: &'a str, prop: &str) -> Option<&'a str> {
    let style = extract_attr_str(tag, "style")?;
    for part in style.split(';') {
        let mut kv = part.split(':');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            if k.trim() == prop {
                return Some(v.trim());
            }
        }
    }
    None
}

fn extract_prop_str(tag: &str, prop: &str) -> Option<String> {
    extract_attr_str(tag, prop)
        .or_else(|| extract_from_style(tag, prop))
        .map(|s| s.to_string())
}

fn parse_font_weight(val: &str) -> u16 {
    let lower = val.trim().to_lowercase();
    match lower.as_str() {
        "normal" => 400,
        "bold" => 700,
        "bolder" => 800,
        "lighter" => 300,
        _ => lower.parse::<u16>().unwrap_or(400),
    }
}

fn parse_font_style(val: &str) -> FontStyle {
    let lower = val.trim().to_lowercase();
    match lower.as_str() {
        "italic" => FontStyle::Italic,
        "oblique" => FontStyle::Oblique,
        _ => FontStyle::Normal,
    }
}

fn parse_text_anchor(val: &str) -> TextAnchor {
    let lower = val.trim().to_lowercase();
    match lower.as_str() {
        "middle" => TextAnchor::Middle,
        "end" => TextAnchor::End,
        _ => TextAnchor::Start,
    }
}

fn parse_letter_spacing(val: &str) -> f64 {
    let cleaned = val
        .trim()
        .trim_end_matches("px")
        .trim_end_matches("pt")
        .trim();
    cleaned.parse::<f64>().unwrap_or(0.0)
}

fn extract_fill(tag: &str, gradients: &HashMap<String, FillStyle>) -> Option<FillStyle> {
    let color_str = extract_attr_str(tag, "fill").or_else(|| extract_from_style(tag, "fill"));

    if let Some(cs) = color_str {
        if cs == "none" || cs == "transparent" {
            return None;
        }
        if cs.starts_with("url(#") {
            let id = cs.trim_start_matches("url(#").trim_end_matches(')');
            if let Some(grad) = gradients.get(id) {
                return Some(grad.clone());
            }
        }
        return parse_svg_color(cs).map(FillStyle::solid);
    }
    None
}

fn extract_stroke(tag: &str) -> Option<StrokeStyle> {
    let color_str = extract_attr_str(tag, "stroke").or_else(|| extract_from_style(tag, "stroke"));

    let color = color_str.and_then(|cs| {
        if cs == "none" || cs == "transparent" {
            None
        } else {
            parse_svg_color(cs)
        }
    });

    let width = extract_attr_f64(tag, "stroke-width")
        .or_else(|| {
            extract_from_style(tag, "stroke-width")
                .and_then(|s| s.trim_end_matches("px").parse().ok())
        })
        .unwrap_or(1.0);

    let dash_pattern = extract_attr_str(tag, "stroke-dasharray")
        .or_else(|| extract_from_style(tag, "stroke-dasharray"))
        .map(|s| {
            s.split(|c: char| c.is_whitespace() || c == ',')
                .filter(|p| !p.is_empty())
                .filter_map(|p| p.parse::<f64>().ok())
                .collect::<Vec<f64>>()
        })
        .filter(|v| !v.is_empty());

    if color.is_some() || width > 0.0 {
        Some(StrokeStyle {
            color: color.unwrap_or([0.0, 0.0, 0.0, 1.0]),
            width,
            dash_pattern,
            ..StrokeStyle::default()
        })
    } else {
        None
    }
}

fn extract_opacity(tag: &str) -> Option<f32> {
    extract_attr_f64(tag, "opacity")
        .or_else(|| extract_from_style(tag, "opacity").and_then(|s| s.parse().ok()))
        .map(|v| v as f32)
}

fn extract_attr_f64(tag: &str, attr: &str) -> Option<f64> {
    extract_attr_str(tag, attr).and_then(|s| s.trim_end_matches("px").parse().ok())
}

fn parse_numbers(s: &str) -> Vec<f64> {
    s.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect()
}

fn affine_identity() -> [f64; 6] {
    [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
}

fn affine_multiply(m1: &[f64; 6], m2: &[f64; 6]) -> [f64; 6] {
    [
        m1[0] * m2[0] + m1[2] * m2[1],
        m1[1] * m2[0] + m1[3] * m2[1],
        m1[0] * m2[2] + m1[2] * m2[3],
        m1[1] * m2[2] + m1[3] * m2[3],
        m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
        m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
    ]
}

fn affine_apply(m: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

fn affine_to_transform(m: &[f64; 6], origin_x: f64, origin_y: f64) -> Transform {
    let (wx, wy) = affine_apply(m, origin_x, origin_y);
    let scale_x = (m[0] * m[0] + m[1] * m[1]).sqrt();
    let scale_y = (m[2] * m[2] + m[3] * m[3]).sqrt();
    let rotation = m[1].atan2(m[0]);
    Transform {
        x: wx,
        y: wy,
        rotation,
        scale_x: if scale_x > 1e-9 { scale_x } else { 1.0 },
        scale_y: if scale_y > 1e-9 { scale_y } else { 1.0 },
        skew_x: 0.0,
        skew_y: 0.0,
    }
}

fn parse_svg_transform(tag: &str) -> [f64; 6] {
    let Some(t) = extract_attr_str(tag, "transform") else {
        return affine_identity();
    };
    let mut acc = affine_identity();
    let bytes = t.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &t[i..];
        if let Some(j) = rest.find("matrix(") {
            let s = j + i + 7;
            if let Some(e) = t[s..].find(')') {
                let nums = parse_numbers(&t[s..s + e]);
                if nums.len() >= 6 {
                    let m = [nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]];
                    acc = affine_multiply(&acc, &m);
                }
                i = s + e + 1;
                continue;
            }
            break;
        } else if let Some(j) = rest.find("translate(") {
            let s = j + i + 10;
            if let Some(e) = t[s..].find(')') {
                let nums = parse_numbers(&t[s..s + e]);
                let (tx, ty) = match nums.as_slice() {
                    [x, y, ..] => (*x, *y),
                    [x] => (*x, 0.0),
                    _ => (0.0, 0.0),
                };
                acc = affine_multiply(&acc, &[1.0, 0.0, 0.0, 1.0, tx, ty]);
                i = s + e + 1;
                continue;
            }
            break;
        } else if let Some(j) = rest.find("scale(") {
            let s = j + i + 6;
            if let Some(e) = t[s..].find(')') {
                let nums = parse_numbers(&t[s..s + e]);
                let (sx, sy) = match nums.as_slice() {
                    [x, y, ..] => (*x, *y),
                    [x] => (*x, *x),
                    _ => (1.0, 1.0),
                };
                acc = affine_multiply(&acc, &[sx, 0.0, 0.0, sy, 0.0, 0.0]);
                i = s + e + 1;
                continue;
            }
            break;
        } else if let Some(j) = rest.find("rotate(") {
            let s = j + i + 7;
            if let Some(e) = t[s..].find(')') {
                let nums = parse_numbers(&t[s..s + e]);
                if let Some(angle_deg) = nums.first() {
                    let a = angle_deg.to_radians();
                    let (c, s_) = (a.cos(), a.sin());
                    let rot = [c, s_, -s_, c, 0.0, 0.0];
                    if nums.len() >= 3 {
                        let (cx, cy) = (nums[1], nums[2]);
                        let to_o = [1.0, 0.0, 0.0, 1.0, cx, cy];
                        let back = [1.0, 0.0, 0.0, 1.0, -cx, -cy];
                        let tmp = affine_multiply(&rot, &back);
                        let full = affine_multiply(&to_o, &tmp);
                        acc = affine_multiply(&acc, &full);
                    } else {
                        acc = affine_multiply(&acc, &rot);
                    }
                }
                i = s + e + 1;
                continue;
            }
            break;
        } else {
            break;
        }
    }
    acc
}

pub fn export_svg(doc: &Document) -> String {
    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
"#,
        doc.width, doc.height, doc.width, doc.height
    );

    let mut defs = String::new();
    let mut sym_defs = String::new();
    let mut id_counter = 0;

    for sym in &doc.symbols {
        sym_defs.push_str(&format!("  <symbol id=\"{}\">\n", sym.id));
        render_object_to_svg(&sym.object, &mut sym_defs, &mut defs, &mut id_counter);
        sym_defs.push_str("  </symbol>\n");
    }
    defs.push_str(&sym_defs);

    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        for obj in &layer.objects {
            render_object_to_svg(obj, &mut svg, &mut defs, &mut id_counter);
        }
    }

    if !defs.is_empty() {
        if let Some(svg_pos) = svg.find("<svg") {
            if let Some(pos) = svg[svg_pos..].find(">\n") {
                let insert_idx = svg_pos + pos + 2;
                let defs_block = format!("  <defs>\n{}  </defs>\n", defs);
                svg.insert_str(insert_idx, &defs_block);
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn path_data_to_d(path: &PathData, transform: &Transform) -> String {
    let mut d = String::new();
    let m = transform.matrix();

    for elem in &path.elements {
        match elem {
            PathElement::MoveTo(p) => {
                let tp = crate::core::geometry::transform_point_if_needed(p, &m);
                d.push_str(&format!("M {} {} ", tp.x, tp.y));
            }
            PathElement::LineTo(p) => {
                let tp = crate::core::geometry::transform_point_if_needed(p, &m);
                d.push_str(&format!("L {} {} ", tp.x, tp.y));
            }
            PathElement::CurveTo(seg) => {
                let sc1 = crate::core::geometry::transform_point_if_needed(&seg.control1, &m);
                let sc2 = crate::core::geometry::transform_point_if_needed(&seg.control2, &m);
                let se = crate::core::geometry::transform_point_if_needed(&seg.end, &m);
                d.push_str(&format!(
                    "C {} {} {} {} {} {} ",
                    sc1.x, sc1.y, sc2.x, sc2.y, se.x, se.y
                ));
            }
            PathElement::ClosePath => {
                d.push_str("Z ");
            }
        }
    }
    d
}

#[allow(dead_code)]
fn path_to_svg_with_fill(
    path: &PathData,
    transform: &Transform,
    obj_fill_attr: &str,
    obj_stroke: Option<&StrokeStyle>,
    effect_attr: &str,
) -> String {
    let d = path_data_to_d(path, transform);
    let fill = obj_fill_attr;
    let stroke = obj_stroke
        .or(path.stroke.as_ref())
        .map(|s| {
            let dash_str = s
                .dash_pattern
                .as_ref()
                .map(|dp| {
                    format!(
                        " stroke-dasharray=\"{}\"",
                        dp.iter()
                            .map(|n| n.to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                })
                .unwrap_or_default();
            format!(
                " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                color_to_svg_str(&s.color),
                s.width
            )
        })
        .unwrap_or_default();

    format!("  <path d=\"{d}\"{fill}{stroke}{effect_attr} />\n")
}

fn transform_has_linear_part(t: &Transform) -> bool {
    t.rotation.abs() > 1e-9
        || (t.scale_x - 1.0).abs() > 1e-9
        || (t.scale_y - 1.0).abs() > 1e-9
        || t.skew_x.abs() > 1e-9
        || t.skew_y.abs() > 1e-9
}

fn svg_transform_attr(t: &Transform) -> String {
    if !transform_has_linear_part(t) {
        return String::new();
    }
    let m = t.matrix();
    format!(
        " transform=\"matrix({} {} {} {} {} {})\"",
        m[0], m[1], m[2], m[3], m[4], m[5]
    )
}

fn render_object_to_svg(obj: &Object, svg: &mut String, defs: &mut String, counter: &mut usize) {
    if !obj.visible {
        return;
    }

    // Generate SVG Filter if object has shadow or glow
    let mut filter_attr = String::new();
    if obj.shadow.is_some() || obj.glow.is_some() {
        *counter += 1;
        let fid = format!("filter_{}", *counter);
        let mut filter_content = String::new();

        if let Some(ref sh) = obj.shadow {
            let color_hex = color_to_svg_str(&sh.color);
            let opac = sh.opacity;
            filter_content.push_str(&format!(
                r#"    <feDropShadow dx="{}" dy="{}" stdDeviation="{}" flood-color="{}" flood-opacity="{:.2}" />
"#,
                sh.offset_x, sh.offset_y, sh.blur_radius, color_hex, opac
            ));
        }

        if let Some(ref gl) = obj.glow {
            let color_hex = color_to_svg_str(&gl.color);
            let opac = gl.intensity;
            filter_content.push_str(&format!(
                r#"    <feGaussianBlur in="SourceAlpha" stdDeviation="{}" result="blur" />
    <feFlood flood-color="{}" flood-opacity="{:.2}" result="color" />
    <feComposite in="color" in2="blur" operator="in" result="glow" />
    <feMerge>
      <feMergeNode in="glow" />
      <feMergeNode in="SourceGraphic" />
    </feMerge>
"#,
                gl.radius, color_hex, opac
            ));
        }

        defs.push_str(&format!(
            "  <filter id=\"{}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\">\n{}  </filter>\n",
            fid, filter_content
        ));
        filter_attr = format!(" filter=\"url(#{fid})\"");
    }

    let opacity_str = if (obj.opacity - 1.0).abs() > 1e-3 {
        format!(" opacity=\"{:.2}\"", obj.opacity)
    } else {
        String::new()
    };
    let effect_attr = format!("{opacity_str}{filter_attr}");

    // Handle fill: solid, linear, or radial gradient
    let fill_attr = if let Some(ref fill) = obj.fill {
        match &fill.fill_type {
            FillType::Solid(c) => format!(" fill=\"{}\"", color_to_svg_str(c)),
            FillType::Linear(grad) => {
                *counter += 1;
                let gid = format!("grad_{}", *counter);
                let mut stops_str = String::new();
                for s in &grad.stops {
                    let opac_s = if (s.color[3] - 1.0).abs() > 1e-3 {
                        format!(" stop-opacity=\"{:.2}\"", s.color[3])
                    } else {
                        String::new()
                    };
                    stops_str.push_str(&format!(
                        "    <stop offset=\"{:.1}%\" stop-color=\"{}\"{opac_s} />\n",
                        s.offset * 100.0,
                        color_to_svg_str(&s.color)
                    ));
                }
                defs.push_str(&format!(
                    "  <linearGradient id=\"{}\" x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\">\n{}  </linearGradient>\n",
                    gid, grad.start_x, grad.start_y, grad.end_x, grad.end_y, stops_str
                ));
                format!(" fill=\"url(#{gid})\"")
            }
            FillType::Radial(grad) => {
                *counter += 1;
                let gid = format!("rad_grad_{}", *counter);
                let mut stops_str = String::new();
                for s in &grad.stops {
                    let opac_s = if (s.color[3] - 1.0).abs() > 1e-3 {
                        format!(" stop-opacity=\"{:.2}\"", s.color[3])
                    } else {
                        String::new()
                    };
                    stops_str.push_str(&format!(
                        "    <stop offset=\"{:.1}%\" stop-color=\"{}\"{opac_s} />\n",
                        s.offset * 100.0,
                        color_to_svg_str(&s.color)
                    ));
                }
                defs.push_str(&format!(
                    "  <radialGradient id=\"{}\" cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fx=\"{:.2}\" fy=\"{:.2}\">\n{}  </radialGradient>\n",
                    gid, grad.center_x, grad.center_y, grad.radius, grad.focus_x, grad.focus_y, stops_str
                ));
                format!(" fill=\"url(#{gid})\"")
            }
            FillType::Pattern(_) => String::new(),
        }
    } else {
        " fill=\"none\"".to_string()
    };

    let id_attr = if !obj.id.is_empty() {
        format!(" id=\"{}\"", obj.id)
    } else {
        String::new()
    };

    match &obj.object_type {
        ObjectType::Use {
            href,
            width,
            height,
        } => {
            let href_attr = if href.starts_with('#') {
                href.clone()
            } else {
                format!("#{href}")
            };
            let dim_str = match (width, height) {
                (Some(w), Some(h)) => format!(" width=\"{w}\" height=\"{h}\""),
                (Some(w), None) => format!(" width=\"{w}\""),
                (None, Some(h)) => format!(" height=\"{h}\""),
                (None, None) => String::new(),
            };
            let transform_attr = svg_transform_attr(&obj.transform);
            if transform_has_linear_part(&obj.transform) {
                svg.push_str(&format!(
                    "  <use{id_attr} href=\"{href_attr}\" x=\"0\" y=\"0\"{dim_str}{transform_attr}{effect_attr} />\n",
                ));
            } else {
                svg.push_str(&format!(
                    "  <use{id_attr} href=\"{href_attr}\" x=\"{}\" y=\"{}\"{dim_str}{effect_attr} />\n",
                    obj.transform.x, obj.transform.y
                ));
            }
        }
        ObjectType::Path(path) => {
            let d = path_data_to_d(path, &obj.transform);
            let fill = &fill_attr;
            let stroke = obj
                .stroke
                .as_ref()
                .or(path.stroke.as_ref())
                .map(|s| {
                    let dash_str = s
                        .dash_pattern
                        .as_ref()
                        .map(|dp| {
                            format!(
                                " stroke-dasharray=\"{}\"",
                                dp.iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_default();

            svg.push_str(&format!(
                "  <path{id_attr} d=\"{d}\"{fill}{stroke}{effect_attr} />\n"
            ));
        }
        ObjectType::Rectangle {
            width,
            height,
            corner_radius,
        } => {
            let stroke = obj
                .stroke
                .as_ref()
                .map(|s| {
                    let dash_str = s
                        .dash_pattern
                        .as_ref()
                        .map(|dp| {
                            format!(
                                " stroke-dasharray=\"{}\"",
                                dp.iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_default();
            let rx_str = if *corner_radius > 0.0 {
                format!(" rx=\"{corner_radius}\" ry=\"{corner_radius}\"")
            } else {
                String::new()
            };
            if transform_has_linear_part(&obj.transform) {
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <rect{id_attr} x=\"0\" y=\"0\" width=\"{}\" height=\"{}\"{rx_str}{fill_attr}{stroke}{transform_attr}{effect_attr} />\n",
                    width, height,
                ));
            } else {
                svg.push_str(&format!(
                    "  <rect{id_attr} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{rx_str}{fill_attr}{stroke}{effect_attr} />\n",
                    obj.transform.x, obj.transform.y, width, height,
                ));
            }
        }
        ObjectType::Ellipse { rx, ry } => {
            let stroke = obj
                .stroke
                .as_ref()
                .map(|s| {
                    let dash_str = s
                        .dash_pattern
                        .as_ref()
                        .map(|dp| {
                            format!(
                                " stroke-dasharray=\"{}\"",
                                dp.iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_default();
            if transform_has_linear_part(&obj.transform) {
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <ellipse{id_attr} cx=\"0\" cy=\"0\" rx=\"{}\" ry=\"{}\"{fill_attr}{stroke}{transform_attr}{effect_attr} />\n",
                    rx, ry,
                ));
            } else {
                svg.push_str(&format!(
                    "  <ellipse{id_attr} cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"{fill_attr}{stroke}{effect_attr} />\n",
                    obj.transform.x, obj.transform.y, rx, ry,
                ));
            }
        }
        ObjectType::Line { x2, y2 } => {
            let stroke = obj
                .stroke
                .as_ref()
                .map(|s| {
                    let dash_str = s
                        .dash_pattern
                        .as_ref()
                        .map(|dp| {
                            format!(
                                " stroke-dasharray=\"{}\"",
                                dp.iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_else(|| " stroke=\"#000000\" stroke-width=\"1\"".to_string());
            if transform_has_linear_part(&obj.transform) {
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <line{id_attr} x1=\"0\" y1=\"0\" x2=\"{}\" y2=\"{}\"{stroke}{transform_attr}{effect_attr} />\n",
                    x2, y2,
                ));
            } else {
                svg.push_str(&format!(
                    "  <line{id_attr} x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"{stroke}{effect_attr} />\n",
                    obj.transform.x, obj.transform.y, obj.transform.x + x2, obj.transform.y + y2,
                ));
            }
        }
        ObjectType::Text {
            text,
            font_size,
            style,
        } => {
            let fill = if fill_attr.is_empty() || fill_attr == " fill=\"none\"" {
                " fill=\"#000000\"".to_string()
            } else {
                fill_attr
            };
            let escaped_text = xml_escape(text);

            let font_fam = if style.font_family.is_empty() {
                "Inter, sans-serif".to_string()
            } else {
                style.font_family.clone()
            };

            let mut extra_attrs = String::new();
            if style.font_weight != 400 {
                extra_attrs.push_str(&format!(" font-weight=\"{}\"", style.font_weight));
            }
            if style.font_style != FontStyle::Normal {
                extra_attrs.push_str(&format!(
                    " font-style=\"{}\"",
                    style.font_style.as_svg_str()
                ));
            }
            if style.letter_spacing != 0.0 {
                extra_attrs.push_str(&format!(" letter-spacing=\"{}\"", style.letter_spacing));
            }
            if style.text_anchor != TextAnchor::Start {
                extra_attrs.push_str(&format!(
                    " text-anchor=\"{}\"",
                    style.text_anchor.as_svg_str()
                ));
            }

            // Explicit line breaks become positioned tspans (1.2em advance,
            // matching canvas). The importer turns positioned tspans back
            // into `\n`, so multi-line text round-trips.
            let (tx, ty) = if transform_has_linear_part(&obj.transform) {
                (0.0, 0.0)
            } else {
                (obj.transform.x, obj.transform.y)
            };
            let body = if text.contains('\n') {
                let mut spans = String::new();
                for (i, ln) in text.split('\n').enumerate() {
                    if i == 0 {
                        spans.push_str(&format!(
                            "<tspan x=\"{tx}\">{}</tspan>",
                            xml_escape(ln)
                        ));
                    } else {
                        spans.push_str(&format!(
                            "<tspan x=\"{tx}\" dy=\"1.2em\">{}</tspan>",
                            xml_escape(ln)
                        ));
                    }
                }
                spans
            } else {
                escaped_text
            };
            if transform_has_linear_part(&obj.transform) {
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <text{id_attr} x=\"{tx}\" y=\"{ty}\" font-size=\"{font_size}\" font-family=\"{font_fam}\"{extra_attrs}{fill}{transform_attr}{effect_attr}>{body}</text>\n",
                ));
            } else {
                svg.push_str(&format!(
                    "  <text{id_attr} x=\"{tx}\" y=\"{ty}\" font-size=\"{font_size}\" font-family=\"{font_fam}\"{extra_attrs}{fill}{effect_attr}>{body}</text>\n",
                ));
            }
        }
        ObjectType::Group(children) => {
            let transform_attr = svg_transform_attr(&obj.transform);
            svg.push_str(&format!("  <g{id_attr}{transform_attr}{effect_attr}>\n"));
            for child in children {
                render_object_to_svg(child, svg, defs, counter);
            }
            svg.push_str("  </g>\n");
        }
        ObjectType::ClippingMask { children } => {
            if !children.is_empty() {
                *counter += 1;
                let clip_id = format!("clip_{}", *counter);
                let mask_obj = &children[0];
                let mask_path = mask_obj.to_path_data();
                defs.push_str(&format!(
                    "  <clipPath id=\"{}\">\n    <path d=\"{}\" />\n  </clipPath>\n",
                    clip_id,
                    path_data_to_d(&mask_path, &mask_obj.transform)
                ));
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <g{id_attr} clip-path=\"url(#{clip_id})\"{transform_attr}{effect_attr}>\n"
                ));
                for child in &children[1..] {
                    render_object_to_svg(child, svg, defs, counter);
                }
                svg.push_str("  </g>\n");
            }
        }
        ObjectType::Star { .. } | ObjectType::Polygon { .. } => {
            let path = obj.to_path_data();
            let d = path_data_to_d(&path, &obj.transform);
            let fill = &fill_attr;
            let stroke = obj
                .stroke
                .as_ref()
                .or(path.stroke.as_ref())
                .map(|s| {
                    let dash_str = s
                        .dash_pattern
                        .as_ref()
                        .map(|dp| {
                            format!(
                                " stroke-dasharray=\"{}\"",
                                dp.iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_default();
            svg.push_str(&format!(
                "  <path{id_attr} d=\"{d}\"{fill}{stroke}{effect_attr} />\n"
            ));
        }
    }
}

fn color_to_svg_str(c: &[f32; 4]) -> String {
    let r = (c[0] * 255.0) as u8;
    let g = (c[1] * 255.0) as u8;
    let b = (c[2] * 255.0) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn xml_unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}
