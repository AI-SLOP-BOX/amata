use super::attrs::*;
use super::tokenize::*;
use super::util::*;
use crate::core::document::{
    Document, FontStyle, Object, ObjectType, TextAnchor, TextStyle,
};
use crate::core::path::{
    AnchorPoint, BezierSegment, FillStyle, FillType, PathData, PathElement, StrokeStyle,
};
use std::collections::HashMap;

/// SVG elliptical arc (endpoint → center form) approximated by cubic segments.
/// `large_arc` / `sweep` select which of the four candidate arcs to draw.
fn svg_arc_to_cubics(
    p0: AnchorPoint,
    p1: AnchorPoint,
    rx: f64,
    ry: f64,
    large_arc: bool,
    sweep: bool,
) -> Vec<BezierSegment> {
    if (p0.x - p1.x).abs() < 1e-12 && (p0.y - p1.y).abs() < 1e-12 {
        return Vec::new();
    }
    let (rx, ry) = (rx.abs().max(1e-12), ry.abs().max(1e-12));

    // F.6.5.1 — scale up radii if too small for the chord.
    let x1p = (p0.x - p1.x) / 2.0;
    let y1p = (p0.y - p1.y) / 2.0;
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    let (rx, ry) = if lambda > 1.0 {
        let s = lambda.sqrt();
        (rx * s, ry * s)
    } else {
        (rx, ry)
    };

    // F.6.5.2 — center in the ellipse coordinate system, then world.
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let num = (rx * rx) * (ry * ry) - (rx * rx) * (y1p * y1p) - (ry * ry) * (x1p * x1p);
    let den = (rx * rx) * (y1p * y1p) + (ry * ry) * (x1p * x1p);
    let coef = if den == 0.0 {
        0.0
    } else {
        sign * (num.max(0.0) / den).sqrt()
    };
    let cxp = coef * (rx * y1p) / ry;
    let cyp = coef * -(ry * x1p) / rx;
    let cx = cxp + (p0.x + p1.x) / 2.0;
    let cy = cyp + (p0.y + p1.y) / 2.0;

    let theta1 = ((y1p - cyp) / ry).atan2((x1p - cxp) / rx);
    let dtheta_raw = ((-y1p - cyp) / ry).atan2((-x1p - cxp) / rx) - theta1;
    let dtheta = if !sweep && dtheta_raw > 0.0 {
        dtheta_raw - std::f64::consts::TAU
    } else if sweep && dtheta_raw < 0.0 {
        dtheta_raw + std::f64::consts::TAU
    } else {
        dtheta_raw
    };

    // Approximate each ≤90° slice with one cubic (α ≈ 4/3·tan(Δθ/4)).
    let n = ((dtheta.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let delta = dtheta / n as f64;
    let alpha = 4.0 / 3.0 * (delta / 4.0).tan();

    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let th1 = theta1 + delta * k as f64;
        let th2 = th1 + delta;
        let (c1s, s1) = (th1.cos(), th1.sin());
        let (c2s, s2) = (th2.cos(), th2.sin());

        // Clamp the true endpoints on the first/last slice so the curve
        // lands exactly on p0 / p1 despite floating-point drift.
        let start = if k == 0 {
            p0
        } else {
            AnchorPoint::new(cx + rx * c1s, cy + ry * s1)
        };
        let end = if k + 1 == n {
            p1
        } else {
            AnchorPoint::new(cx + rx * c2s, cy + ry * s2)
        };

        // Tangent d(P)/dθ = (-rx·sinθ, ry·cosθ); scale by α for the handles.
        let c1 = AnchorPoint::new(start.x + alpha * (-rx * s1), start.y + alpha * (ry * c1s));
        let c2 = AnchorPoint::new(end.x - alpha * (-rx * s2), end.y - alpha * (ry * c2s));
        out.push(BezierSegment::cubic(start, c1, c2, end));
    }
    out
}

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
                    let large_arc: f64 = tokens[i + 3].parse().unwrap_or(0.0);
                    let sweep: f64 = tokens[i + 4].parse().unwrap_or(0.0);
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
                        // Endpoint → center parameterization (SVG F.6.5),
                        // then approximate with cubic Beziers. Honors
                        // large-arc / sweep instead of always midpoint-cubic.
                        let arc = svg_arc_to_cubics(
                            curr_pos,
                            dest,
                            rx,
                            ry,
                            large_arc != 0.0,
                            sweep != 0.0,
                        );
                        for c in arc {
                            elements.push(PathElement::CurveTo(c));
                        }
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

/// Validate SVG text with the real SVG parser (usvg) before converting.
/// Returns Err for broken XML / invalid SVG so callers can refuse a
/// document swap with feedback instead of silently loading an empty canvas.
/// Scope note: usvg also normalizes and may reject exotic-but-tolerable
/// input, so this gate is used at interactive replacement points (Open,
/// external-change notice) — never in batch/CLI paths, which stay lenient.
pub fn try_parse_svg_document(svg_text: &str) -> Result<Document, String> {
    let options = resvg::usvg::Options::default();
    resvg::usvg::Tree::from_str(svg_text, &options)
        .map_err(|e| format!("Invalid SVG: {e}"))?;
    Ok(parse_svg_document(svg_text))
}

pub fn parse_svg_document(svg_text: &str) -> Document {
    let mut doc = Document {
        name: "SVG Import".to_string(),
        ..Default::default()
    };
    let mut obj_count = 0;

    let tags = tokenize_svg_tags(svg_text);

    // 1. Scan and parse <defs> resources (Gradients, Symbols, clipPaths)
    let mut gradients: HashMap<String, FillStyle> = HashMap::new();
    let mut clip_paths: HashMap<String, Object> = HashMap::new();
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
        } else if trimmed.starts_with("<clipPath") {
            let clip_id = extract_attr_str(trimmed, "id").unwrap_or("").to_string();
            if trimmed.ends_with("/>") {
                if !clip_id.is_empty() {
                    clip_paths.insert(
                        clip_id.clone(),
                        Object::new_path(
                            &format!("Mask {clip_id}"),
                            PathData::new(),
                        ),
                    );
                }
            } else {
                let mut child_tags = Vec::new();
                let mut j = i + 1;
                while j < tags.len() && !tags[j].trim().starts_with("</clipPath") {
                    child_tags.push(tags[j].clone());
                    j += 1;
                }
                if !clip_id.is_empty() {
                    let inner_svg = format!("<svg>{}</svg>", child_tags.join("\n"));
                    let inner_doc = parse_svg_document(&inner_svg);
                    let mut objs: Vec<Object> = inner_doc
                        .all_objects()
                        .map(|(_, o)| o.clone())
                        .collect();
                    // clipPath's own transform applies on top of children.
                    let cm = parse_svg_transform(trimmed);
                    if !is_affine_identity(&cm) {
                        for o in &mut objs {
                            let mut path = o.to_path_data();
                            path.transform(&o.transform.matrix());
                            path.transform(&cm);
                            *o = Object::new_path(&o.name, path);
                        }
                    }
                    let mask = merge_mask_objects(&clip_id, objs);
                    clip_paths.insert(clip_id, mask);
                }
                i = j;
            }
        }
        i += 1;
    }

    // 2. Parse Elements & Nested Groups
    let mut group_stack: Vec<[f64; 6]> = vec![affine_identity()];
    // Accumulated group opacity (SVG groups compose opacity
    // multiplicatively; previously dropped on import).
    let mut group_opacity: Vec<f32> = vec![1.0];
    let mut in_defs = false;
    // Skip <clipPath>…</clipPath> content in the drawable pass (defs or not).
    let mut in_clip_path = false;
    // Clip groups: parallel to every <g>, true when that <g> opened a
    // collector frame for `clip-path="url(#…)"`.
    let mut g_opens_clip: Vec<bool> = Vec::new();
    // Open clip collector frames (innermost last): id, matrix at open,
    // objects collected until the matching </g>.
    let mut clip_frames: Vec<(String, [f64; 6], Vec<Object>)> = Vec::new();

    for tag in &tags {
        let trimmed = tag.trim();

        // defs may be self-closing (<defs/>) — don't wait for </defs>.
        if trimmed.starts_with("<defs") {
            in_defs = !trimmed.ends_with("/>");
            continue;
        }
        if trimmed.starts_with("</defs>") || trimmed.starts_with("</defs ") {
            in_defs = false;
            continue;
        }
        if in_defs {
            continue;
        }

        // clipPath definitions are collected in pass 1; skip their markup
        // here so child shapes are not imported as drawable objects.
        if trimmed.starts_with("<clipPath") {
            in_clip_path = !trimmed.ends_with("/>");
            continue;
        }
        if trimmed.starts_with("</clipPath") {
            in_clip_path = false;
            continue;
        }
        if in_clip_path {
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
                // Prefer viewBox size whenever present; width/height on the
                // root (if any) wins so physical units still apply.
                // Previously this only ran for the default 1920×1080 doc and
                // ignored min-x/min-y (origin offset).
                if parts.len() == 4 && parts[2] > 0.0 && parts[3] > 0.0 {
                    let vb_w = parts[2];
                    let vb_h = parts[3];
                    let has_w = extract_attr_f64(trimmed, "width").is_some();
                    let has_h = extract_attr_f64(trimmed, "height").is_some();
                    if !has_w {
                        doc.width = vb_w;
                    }
                    if !has_h {
                        doc.height = vb_h;
                    }
                    // Shift content so viewBox origin (min-x, min-y) maps to 0,0.
                    if (parts[0].abs() > 1e-9 || parts[1].abs() > 1e-9)
                        && group_stack.len() == 1
                    {
                        let tx = -parts[0];
                        let ty = -parts[1];
                        if tx != 0.0 || ty != 0.0 {
                            group_stack[0] = [1.0, 0.0, 0.0, 1.0, tx, ty];
                        }
                    }
                }
            }
            continue;
        }

        // Group opening
        if trimmed.starts_with("<g") {
            let local = parse_svg_transform(trimmed);
            let current = group_stack.last().copied().unwrap_or(affine_identity());
            let full = affine_multiply(&current, &local);
            group_stack.push(full);
            let parent_op = group_opacity.last().copied().unwrap_or(1.0);
            let local_op = extract_opacity(trimmed).unwrap_or(1.0);
            group_opacity.push(parent_op * local_op);
            let clip_id = extract_clip_path_id(trimmed);
            let opens = clip_id
                .as_ref()
                .filter(|id| clip_paths.contains_key(*id))
                .cloned();
            if let Some(id) = opens {
                clip_frames.push((id, full, Vec::new()));
                g_opens_clip.push(true);
            } else {
                g_opens_clip.push(false);
            }
            continue;
        }

        // Group closing
        if trimmed.starts_with("</g>") || trimmed.starts_with("</g ") {
            let opened_clip = g_opens_clip.pop().unwrap_or(false);
            if opened_clip {
                if let Some((clip_id, matrix, mut objs)) = clip_frames.pop() {
                    if let Some(mask) = clip_paths.get(&clip_id) {
                        let mut mask_path = mask.to_path_data();
                        mask_path.transform(&mask.transform.matrix());
                        mask_path.transform(&matrix);
                        let mut children = vec![Object::new_path("Mask", mask_path)];
                        children.append(&mut objs);
                        let mut clipping = Object::new_rect(
                            "Clipping Mask",
                            0.0,
                            0.0,
                            100.0,
                            100.0,
                            0.0,
                        );
                        clipping.object_type = ObjectType::ClippingMask { children };
                        clipping.name = "Clipping Mask".into();
                        // Emit into the parent clip frame (or the document).
                        if let Some((_, _, parent)) = clip_frames.last_mut() {
                            parent.push(clipping);
                        } else {
                            doc.add_object(clipping);
                        }
                    } else if let Some((_, _, parent)) = clip_frames.last_mut() {
                        // Missing mask definition: keep content unclipped.
                        parent.append(&mut objs);
                    } else {
                        for o in objs {
                            doc.add_object(o);
                        }
                    }
                }
            }
            if group_stack.len() > 1 {
                group_stack.pop();
            }
            if group_opacity.len() > 1 {
                group_opacity.pop();
            }
            continue;
        }

        let group_m = group_stack.last().copied().unwrap_or(affine_identity());
        let group_op = group_opacity.last().copied().unwrap_or(1.0);
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
                        obj.opacity = group_op
                            * extract_opacity(trimmed).unwrap_or(1.0);
                        let obj = wrap_element_clip(obj, trimmed, &clip_paths, &total_m);
                        emit_import_object(&mut doc, &mut clip_frames, obj);
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
                    let obj = wrap_element_clip(obj, trimmed, &clip_paths, &total_m);
                        emit_import_object(&mut doc, &mut clip_frames, obj);
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
                obj.opacity = group_op * extract_opacity(trimmed).unwrap_or(1.0);
                let obj = wrap_element_clip(obj, trimmed, &clip_paths, &total_m);
                        emit_import_object(&mut doc, &mut clip_frames, obj);
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
                obj.opacity = group_op * extract_opacity(trimmed).unwrap_or(1.0);
                let obj = wrap_element_clip(obj, trimmed, &clip_paths, &total_m);
                        emit_import_object(&mut doc, &mut clip_frames, obj);
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
                } else {
                    // SVG default for <line>/<polyline> is a black 1px stroke;
                    // without this the line imports invisible (no fill either).
                    obj.stroke = Some(StrokeStyle::default());
                }
                obj.opacity = group_op * extract_opacity(trimmed).unwrap_or(1.0);
                let obj = wrap_element_clip(obj, trimmed, &clip_paths, &total_m);
                        emit_import_object(&mut doc, &mut clip_frames, obj);
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
                .unwrap_or_else(|| {
                    if trimmed.contains("data-tbold=\"1\"") {
                        700
                    } else {
                        400
                    }
                });
            let font_style = extract_prop_str(trimmed, "font-style")
                .map(|s| parse_font_style(&s))
                .unwrap_or_else(|| {
                    if trimmed.contains("data-titalic=\"1\"") {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    }
                });
            let letter_spacing = extract_prop_str(trimmed, "letter-spacing")
                .map(|ls| parse_letter_spacing(&ls))
                .unwrap_or(0.0);
            let text_anchor = extract_prop_str(trimmed, "text-anchor")
                .map(|ta| parse_text_anchor(&ta))
                .unwrap_or(TextAnchor::Start);
            let variations = extract_prop_str(trimmed, "font-variation-settings")
                .map(|s| TextStyle::parse_variation_settings_css(&s))
                .unwrap_or_default();
            let vertical = extract_prop_str(trimmed, "writing-mode")
                .map(|m| m.contains("vertical"))
                .unwrap_or(false)
                || extract_attr_str(trimmed, "writing-mode")
                    .map(|m| m.contains("vertical"))
                    .unwrap_or(false);
            let ligatures = extract_prop_str(trimmed, "font-variant-ligatures")
                .map(|v| !v.contains("none"))
                .unwrap_or(true);
            let next_frame = extract_attr_str(trimmed, "data-text-thread").map(|s| s.to_string());
            let next_frame_from_attr = next_frame.clone();

            let style = TextStyle {
                font_family,
                font_size,
                font_weight,
                font_style,
                letter_spacing,
                text_anchor,
                variations,
                vertical,
                ligatures,
                ..Default::default()
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
                // Area-type box round-trip (`data-text-area="x y w h"`).
                if let Some(area_str) = extract_attr_str(trimmed, "data-text-area") {
                    let parts: Vec<f64> = area_str
                        .split(|c: char| c.is_whitespace() || c == ',')
                        .filter(|s| !s.is_empty())
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    if parts.len() == 4
                        && parts.iter().all(|v| v.is_finite())
                        && parts[2] > 0.0
                        && parts[3] > 0.0
                    {
                        if let ObjectType::Text { area, next_frame, .. } = &mut obj.object_type {
                            *area = Some(crate::core::document::TextArea::new(
                                parts[0], parts[1], parts[2], parts[3],
                            ));
                            *next_frame = next_frame_from_attr;
                        }
                    }
                }
                obj.id = extract_attr_str(trimmed, "id")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("auto_text_{obj_count}"));
                if let Some(fill) = extract_fill(trimmed, &gradients) {
                    obj.fill = Some(fill);
                }
                obj.opacity = group_op * extract_opacity(trimmed).unwrap_or(1.0);
                let obj = wrap_element_clip(obj, trimmed, &clip_paths, &total_m);
                        emit_import_object(&mut doc, &mut clip_frames, obj);
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
                obj.opacity = group_op * extract_opacity(trimmed).unwrap_or(1.0);
                let obj = wrap_element_clip(obj, trimmed, &clip_paths, &total_m);
                        emit_import_object(&mut doc, &mut clip_frames, obj);
            }
        } else if trimmed.starts_with("<image") {
            // Embedded data URIs only; external file references cannot be
            // resolved without the source document's location.
            let href = extract_attr_str(trimmed, "href")
                .or_else(|| extract_attr_str(trimmed, "xlink:href"))
                .unwrap_or("");
            if let Some(b64) = href.strip_prefix("data:image/png;base64,") {
                if let Some(png) = base64_decode(b64.trim()) {
                    let x = extract_attr_f64(trimmed, "x").unwrap_or(0.0);
                    let y = extract_attr_f64(trimmed, "y").unwrap_or(0.0);
                    let w = extract_attr_f64(trimmed, "width").unwrap_or(0.0);
                    let h = extract_attr_f64(trimmed, "height").unwrap_or(0.0);
                    if w > 0.0 && h > 0.0 && !png.is_empty() {
                        obj_count += 1;
                        let mut obj = Object::new_image(
                            &format!("Image {obj_count}"),
                            x,
                            y,
                            w,
                            h,
                            png,
                        );
                        if has_linear {
                            obj.transform = affine_to_transform(&total_m, x, y);
                        } else {
                            obj.transform.x = x + total_tx;
                            obj.transform.y = y + total_ty;
                        }
                        obj.id = extract_attr_str(trimmed, "id")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("auto_image_{obj_count}"));
                        obj.opacity = group_op * extract_opacity(trimmed).unwrap_or(1.0);
                        let obj = wrap_element_clip(obj, trimmed, &clip_paths, &total_m);
                        emit_import_object(&mut doc, &mut clip_frames, obj);
                    }
                }
            }
        }
    }

    doc
}

fn is_affine_identity(m: &[f64; 6]) -> bool {
    (m[0] - 1.0).abs() < 1e-12
        && m[1].abs() < 1e-12
        && m[2].abs() < 1e-12
        && (m[3] - 1.0).abs() < 1e-12
        && m[4].abs() < 1e-12
        && m[5].abs() < 1e-12
}

/// Combine clipPath child shapes into one mask object (single path when possible).
fn merge_mask_objects(clip_id: &str, mut objs: Vec<Object>) -> Object {
    if objs.is_empty() {
        return Object::new_path(&format!("Mask {clip_id}"), PathData::new());
    }
    if objs.len() == 1 {
        return objs.pop().unwrap();
    }
    // Multi-shape clipPath: merge geometry into one path (SVG union semantics
    // approximated by concatenating subpaths).
    let mut merged = PathData::new();
    for o in &objs {
        let mut p = o.to_path_data();
        p.transform(&o.transform.matrix());
        if !merged.elements.is_empty() {
            merged.elements.push(PathElement::MoveTo(
                p.elements
                    .iter()
                    .find_map(|e| match e {
                        PathElement::MoveTo(pt) | PathElement::LineTo(pt) => Some(*pt),
                        PathElement::CurveTo(seg) => Some(seg.start),
                        PathElement::ClosePath => None,
                    })
                    .unwrap_or(AnchorPoint::new(0.0, 0.0)),
            ));
        }
        merged.elements.extend(p.elements);
    }
    merged.closed = objs.iter().any(|o| {
        o.to_path_data()
            .elements
            .iter()
            .any(|e| matches!(e, PathElement::ClosePath))
    });
    Object::new_path(&format!("Mask {clip_id}"), merged)
}

/// Extract `clip-path="url(#id)"` / `style="clip-path:url(#id)"` target id.
fn extract_clip_path_id(tag: &str) -> Option<String> {
    let raw = extract_prop_str(tag, "clip-path")?;
    let start = raw.find("url(")?;
    let rest = &raw[start + 4..];
    let end = rest.find(')')?;
    let inner = rest[..end].trim();
    let inner = inner
        .trim_start_matches("url(")
        .trim_matches(|c| c == '\'' || c == '"' || c == ' ');
    let id = inner.strip_prefix('#')?;
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

/// Place an imported object: into the innermost open clip group, else the doc.
fn emit_import_object(
    doc: &mut Document,
    clip_frames: &mut Vec<(String, [f64; 6], Vec<Object>)>,
    obj: Object,
) {
    if let Some((_, _, parent)) = clip_frames.last_mut() {
        parent.push(obj);
    } else {
        doc.add_object(obj);
    }
}

/// Wrap `obj` in a ClippingMask when the tag carries a resolvable clip-path.
fn wrap_element_clip(
    obj: Object,
    tag: &str,
    clip_paths: &HashMap<String, Object>,
    total_m: &[f64; 6],
) -> Object {
    let Some(cid) = extract_clip_path_id(tag) else {
        return obj;
    };
    let Some(mask) = clip_paths.get(&cid) else {
        return obj;
    };
    let mut mask_path = mask.to_path_data();
    mask_path.transform(&mask.transform.matrix());
    mask_path.transform(total_m);
    let children = vec![Object::new_path("Mask", mask_path), obj];
    // Reuse child list length for a stable placeholder size.
    let n = children.len();
    let mut clipping = Object::new_rect("Clipping Mask", 0.0, 0.0, 100.0, 100.0, 0.0);
    clipping.object_type = ObjectType::ClippingMask { children };
    clipping.name = format!("Clipping Mask ({n})");
    clipping
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
            let color = extract_prop_str(stop_tag, "stop-color")
                .as_deref()
                .and_then(parse_svg_color)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let opacity = extract_prop_str(stop_tag, "stop-opacity")
                .and_then(|s| s.parse::<f64>().ok())
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
            overprint: false,
            spot: None,
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
            let color = extract_prop_str(stop_tag, "stop-color")
                .as_deref()
                .and_then(parse_svg_color)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let opacity = extract_prop_str(stop_tag, "stop-opacity")
                .and_then(|s| s.parse::<f64>().ok())
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
            overprint: false,
            spot: None,
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

