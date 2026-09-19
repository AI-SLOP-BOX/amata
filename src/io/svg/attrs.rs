use super::parse::parse_svg_color;
use crate::core::document::{FontStyle, TextAnchor, Transform};
use crate::core::path::{AnchorPoint, FillStyle, StrokeStyle};
use std::collections::HashMap;

pub(super) fn parse_svg_points(points_str: &str) -> Vec<AnchorPoint> {
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

pub(super) fn extract_attr_str<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
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

pub(super) fn extract_from_style<'a>(tag: &'a str, prop: &str) -> Option<&'a str> {
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

pub(super) fn extract_prop_str(tag: &str, prop: &str) -> Option<String> {
    extract_attr_str(tag, prop)
        .or_else(|| extract_from_style(tag, prop))
        .map(|s| s.to_string())
}

pub(super) fn parse_font_weight(val: &str) -> u16 {
    let lower = val.trim().to_lowercase();
    match lower.as_str() {
        "normal" => 400,
        "bold" => 700,
        "bolder" => 800,
        "lighter" => 300,
        _ => lower.parse::<u16>().unwrap_or(400),
    }
}

pub(super) fn parse_font_style(val: &str) -> FontStyle {
    let lower = val.trim().to_lowercase();
    match lower.as_str() {
        "italic" => FontStyle::Italic,
        "oblique" => FontStyle::Oblique,
        _ => FontStyle::Normal,
    }
}

pub(super) fn parse_text_anchor(val: &str) -> TextAnchor {
    let lower = val.trim().to_lowercase();
    match lower.as_str() {
        "middle" => TextAnchor::Middle,
        "end" => TextAnchor::End,
        _ => TextAnchor::Start,
    }
}

pub(super) fn parse_letter_spacing(val: &str) -> f64 {
    let cleaned = val
        .trim()
        .trim_end_matches("px")
        .trim_end_matches("pt")
        .trim();
    cleaned.parse::<f64>().unwrap_or(0.0)
}

pub(super) fn extract_fill(tag: &str, gradients: &HashMap<String, FillStyle>) -> Option<FillStyle> {
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

pub(super) fn extract_stroke(tag: &str) -> Option<StrokeStyle> {
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

pub(super) fn extract_opacity(tag: &str) -> Option<f32> {
    extract_attr_f64(tag, "opacity")
        .or_else(|| extract_from_style(tag, "opacity").and_then(|s| s.parse().ok()))
        .map(|v| v as f32)
}

pub(super) fn extract_attr_f64(tag: &str, attr: &str) -> Option<f64> {
    extract_attr_str(tag, attr).and_then(|s| s.trim_end_matches("px").parse().ok())
}

pub(super) fn parse_numbers(s: &str) -> Vec<f64> {
    s.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect()
}

pub(super) fn affine_identity() -> [f64; 6] {
    [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
}

pub(super) fn affine_multiply(m1: &[f64; 6], m2: &[f64; 6]) -> [f64; 6] {
    [
        m1[0] * m2[0] + m1[2] * m2[1],
        m1[1] * m2[0] + m1[3] * m2[1],
        m1[0] * m2[2] + m1[2] * m2[3],
        m1[1] * m2[2] + m1[3] * m2[3],
        m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
        m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
    ]
}

pub(super) fn affine_apply(m: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

pub(super) fn affine_to_transform(m: &[f64; 6], origin_x: f64, origin_y: f64) -> Transform {
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

pub(super) fn parse_svg_transform(tag: &str) -> [f64; 6] {
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

