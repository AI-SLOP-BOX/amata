use crate::core::path::{AnchorPoint, BezierSegment, FillStyle, FillType, PathData, PathElement, StrokeStyle};
use crate::core::document::{Document, Object, ObjectType, Transform};

pub fn parse_svg_path_data(d: &str) -> Result<Vec<PathElement>, String> {
    let mut elements = Vec::new();
    let mut curr_pos = AnchorPoint::new(0.0, 0.0);

    let mut tokens = Vec::new();
    let mut curr_token = String::new();

    for c in d.chars() {
        if c.is_alphabetic() {
            if !curr_token.trim().is_empty() {
                tokens.push(curr_token.trim().to_string());
                curr_token.clear();
            }
            tokens.push(c.to_string());
        } else if c.is_whitespace() || c == ',' {
            if !curr_token.trim().is_empty() {
                tokens.push(curr_token.trim().to_string());
                curr_token.clear();
            }
        } else {
            curr_token.push(c);
        }
    }
    if !curr_token.trim().is_empty() {
        tokens.push(curr_token.trim().to_string());
    }

    let mut i = 0;
    while i < tokens.len() {
        let cmd = &tokens[i];
        match cmd.as_str() {
            "M" | "m" => {
                let is_rel = cmd == "m";
                if i + 2 < tokens.len() {
                    let x: f64 = tokens[i + 1].parse().map_err(|e| format!("M.x: {e}"))?;
                    let y: f64 = tokens[i + 2].parse().map_err(|e| format!("M.y: {e}"))?;
                    curr_pos = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(x, y)
                    };
                    elements.push(PathElement::MoveTo(curr_pos));
                    i += 3;
                } else {
                    break;
                }
            }
            "L" | "l" => {
                let is_rel = cmd == "l";
                if i + 2 < tokens.len() {
                    let x: f64 = tokens[i + 1].parse().map_err(|e| format!("L.x: {e}"))?;
                    let y: f64 = tokens[i + 2].parse().map_err(|e| format!("L.y: {e}"))?;
                    curr_pos = if is_rel {
                        AnchorPoint::new(curr_pos.x + x, curr_pos.y + y)
                    } else {
                        AnchorPoint::new(x, y)
                    };
                    elements.push(PathElement::LineTo(curr_pos));
                    i += 3;
                } else {
                    break;
                }
            }
            "C" | "c" => {
                let is_rel = cmd == "c";
                if i + 6 < tokens.len() {
                    let x1: f64 = tokens[i + 1].parse().map_err(|e| format!("C.x1: {e}"))?;
                    let y1: f64 = tokens[i + 2].parse().map_err(|e| format!("C.y1: {e}"))?;
                    let x2: f64 = tokens[i + 3].parse().map_err(|e| format!("C.x2: {e}"))?;
                    let y2: f64 = tokens[i + 4].parse().map_err(|e| format!("C.y2: {e}"))?;
                    let x: f64 = tokens[i + 5].parse().map_err(|e| format!("C.x: {e}"))?;
                    let y: f64 = tokens[i + 6].parse().map_err(|e| format!("C.y: {e}"))?;

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

                    elements.push(PathElement::CurveTo(BezierSegment::cubic(curr_pos, c0, c1, dest)));
                    curr_pos = dest;
                    i += 7;
                } else {
                    break;
                }
            }
            "Z" | "z" => {
                elements.push(PathElement::ClosePath);
                i += 1;
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
                Some([
                    r as f32 / 15.0,
                    g as f32 / 15.0,
                    b as f32 / 15.0,
                    1.0,
                ])
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some([
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    1.0,
                ])
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

    for line in svg_text.lines() {
        if line.contains("<path") {
            if let Some(d_start) = line.find("d=\"") {
                let rest = &line[d_start + 3..];
                if let Some(d_end) = rest.find('"') {
                    let d = &rest[..d_end];
                    if let Ok(elements) = parse_svg_path_data(d) {
                        if !elements.is_empty() {
                            let mut path = PathData::new();
                            path.elements = elements;
                            path.closed = path.elements.iter().any(|e| matches!(e, PathElement::ClosePath));

                            let fill = extract_fill(line);
                            let stroke = extract_stroke(line);
                            path.fill = fill;
                            path.stroke = stroke;

                            obj_count += 1;
                            doc.add_object(Object::new_path(&format!("Path {obj_count}"), path));
                        }
                    }
                }
            }
        } else if line.contains("<rect") {
            if let (Some(x), Some(y), Some(w), Some(h)) = (
                extract_attr_f64(line, "x"),
                extract_attr_f64(line, "y"),
                extract_attr_f64(line, "width"),
                extract_attr_f64(line, "height"),
            ) {
                let rx = extract_attr_f64(line, "rx").unwrap_or(0.0);
                obj_count += 1;
                let mut obj = Object::new_rect(&format!("Rect {obj_count}"), x, y, w, h, rx);
                if let Some(fill) = extract_fill(line) {
                    obj.fill = Some(fill);
                }
                if let Some(stroke) = extract_stroke(line) {
                    obj.stroke = Some(stroke);
                }
                doc.add_object(obj);
            }
        } else if line.contains("<circle") || line.contains("<ellipse") {
            if let (Some(cx), Some(cy), Some(r1), r2) = (
                extract_attr_f64(line, "cx"),
                extract_attr_f64(line, "cy"),
                extract_attr_f64(line, "rx").or_else(|| extract_attr_f64(line, "r")),
                extract_attr_f64(line, "ry").or_else(|| extract_attr_f64(line, "r")),
            ) {
                let ry = r2.unwrap_or(r1);
                obj_count += 1;
                let mut obj = Object::new_ellipse(&format!("Ellipse {obj_count}"), cx, cy, r1, ry);
                if let Some(fill) = extract_fill(line) {
                    obj.fill = Some(fill);
                }
                if let Some(stroke) = extract_stroke(line) {
                    obj.stroke = Some(stroke);
                }
                doc.add_object(obj);
            }
        } else if line.contains("<line") {
            if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                extract_attr_f64(line, "x1"),
                extract_attr_f64(line, "y1"),
                extract_attr_f64(line, "x2"),
                extract_attr_f64(line, "y2"),
            ) {
                obj_count += 1;
                let mut obj = Object::new_line(&format!("Line {obj_count}"), x1, y1, x2, y2);
                if let Some(stroke) = extract_stroke(line) {
                    obj.stroke = Some(stroke);
                }
                doc.add_object(obj);
            }
        } else if line.contains("<text") {
            if let (Some(x), Some(y)) = (extract_attr_f64(line, "x"), extract_attr_f64(line, "y")) {
                let font_size = extract_attr_f64(line, "font-size").unwrap_or(24.0);
                if let Some(start) = line.find('>') {
                    if let Some(end) = line[start + 1..].find("</text>") {
                        let text_content = &line[start + 1..start + 1 + end];
                        obj_count += 1;
                        let mut obj = Object::new_text(&format!("Text {obj_count}"), text_content, x, y, font_size);
                        if let Some(fill) = extract_fill(line) {
                            obj.fill = Some(fill);
                        }
                        doc.add_object(obj);
                    }
                }
            }
        }
    }

    doc
}

fn extract_fill(line: &str) -> Option<FillStyle> {
    if let Some(start) = line.find("fill=\"") {
        let rest = &line[start + 6..];
        if let Some(end) = rest.find('"') {
            let color_str = &rest[..end];
            return parse_svg_color(color_str).map(FillStyle::solid);
        }
    }
    None
}

fn extract_stroke(line: &str) -> Option<StrokeStyle> {
    let color = if let Some(start) = line.find("stroke=\"") {
        let rest = &line[start + 8..];
        if let Some(end) = rest.find('"') {
            parse_svg_color(&rest[..end])
        } else {
            None
        }
    } else {
        None
    };

    let width = extract_attr_f64(line, "stroke-width").unwrap_or(1.0);

    if color.is_some() || width > 0.0 {
        Some(StrokeStyle {
            color: color.unwrap_or([0.0, 0.0, 0.0, 1.0]),
            width,
            dash_pattern: None,
            ..StrokeStyle::default()
        })
    } else {
        None
    }
}

fn extract_attr_f64(line: &str, attr: &str) -> Option<f64> {
    let pattern = format!("{attr}=\"");
    if let Some(start) = line.find(&pattern) {
        let rest = &line[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return rest[..end].parse().ok();
        }
    }
    None
}

pub fn export_svg(doc: &Document) -> String {
    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
"#,
        doc.width, doc.height, doc.width, doc.height
    );

    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        for obj in &layer.objects {
            if !obj.visible {
                continue;
            }
            let opacity_str = if (obj.opacity - 1.0).abs() > 1e-3 {
                format!(" opacity=\"{:.2}\"", obj.opacity)
            } else {
                String::new()
            };

            match &obj.object_type {
                ObjectType::Path(path) => {
                    svg.push_str(&path_to_svg(path, &obj.transform, &opacity_str));
                }
                ObjectType::Rectangle { width, height, corner_radius } => {
                    let fill = obj.fill.as_ref().map(color_to_svg).unwrap_or_else(|| " fill=\"none\"".to_string());
                    let stroke = obj
                        .stroke
                        .as_ref()
                        .map(|s| format!(" stroke=\"{}\" stroke-width=\"{}\"", color_to_svg_str(&s.color), s.width))
                        .unwrap_or_default();
                    let rx_str = if *corner_radius > 0.0 {
                        format!(" rx=\"{corner_radius}\" ry=\"{corner_radius}\"")
                    } else {
                        String::new()
                    };
                    svg.push_str(&format!(
                        "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{rx_str}{fill}{stroke}{opacity_str} />\n",
                        obj.transform.x, obj.transform.y, width, height,
                    ));
                }
                ObjectType::Ellipse { rx, ry } => {
                    let fill = obj.fill.as_ref().map(color_to_svg).unwrap_or_else(|| " fill=\"none\"".to_string());
                    let stroke = obj
                        .stroke
                        .as_ref()
                        .map(|s| format!(" stroke=\"{}\" stroke-width=\"{}\"", color_to_svg_str(&s.color), s.width))
                        .unwrap_or_default();
                    svg.push_str(&format!(
                        "  <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"{fill}{stroke}{opacity_str} />\n",
                        obj.transform.x, obj.transform.y, rx, ry,
                    ));
                }
                ObjectType::Line { x2, y2 } => {
                    let stroke = obj
                        .stroke
                        .as_ref()
                        .map(|s| format!(" stroke=\"{}\" stroke-width=\"{}\"", color_to_svg_str(&s.color), s.width))
                        .unwrap_or_else(|| " stroke=\"#000000\" stroke-width=\"1\"".to_string());
                    svg.push_str(&format!(
                        "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"{stroke}{opacity_str} />\n",
                        obj.transform.x, obj.transform.y, obj.transform.x + x2, obj.transform.y + y2,
                    ));
                }
                ObjectType::Text { text, font_size } => {
                    let fill = obj.fill.as_ref().map(color_to_svg).unwrap_or_else(|| " fill=\"#000000\"".to_string());
                    svg.push_str(&format!(
                        "  <text x=\"{}\" y=\"{}\" font-size=\"{}\"{fill}{opacity_str}>{}</text>\n",
                        obj.transform.x, obj.transform.y, font_size, text
                    ));
                }
                ObjectType::Star { .. } | ObjectType::Polygon { .. } | ObjectType::Group(_) | ObjectType::ClippingMask { .. } => {
                    let path = obj.to_path_data();
                    svg.push_str(&path_to_svg(&path, &obj.transform, &opacity_str));
                }
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn path_to_svg(path: &PathData, transform: &Transform, opacity_str: &str) -> String {
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

    let fill = path.fill.as_ref().map(color_to_svg).unwrap_or_else(|| " fill=\"none\"".to_string());
    let stroke = path
        .stroke
        .as_ref()
        .map(|s| format!(" stroke=\"{}\" stroke-width=\"{}\"", color_to_svg_str(&s.color), s.width))
        .unwrap_or_default();

    format!("  <path d=\"{d}\"{fill}{stroke}{opacity_str} />\n")
}

fn color_to_svg(fill: &FillStyle) -> String {
    match &fill.fill_type {
        FillType::Solid(c) => format!(" fill=\"{}\"", color_to_svg_str(c)),
        FillType::Linear(_) => format!(" fill=\"{}\"", color_to_svg_str(&fill.color)),
        FillType::Radial(_) => format!(" fill=\"{}\"", color_to_svg_str(&fill.color)),
        FillType::Pattern(_) => String::new(),
    }
}

fn color_to_svg_str(c: &[f32; 4]) -> String {
    let r = (c[0] * 255.0) as u8;
    let g = (c[1] * 255.0) as u8;
    let b = (c[2] * 255.0) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}
