use super::document::Object;
use super::path::{AnchorPoint, PathData};

#[derive(Debug, Clone)]
pub struct GlyphPlacement {
    pub char_value: char,
    pub position: AnchorPoint,
    pub rotation_rad: f64,
}

/// Place characters of `text` along the curve defined by `path`
pub fn place_text_along_path(
    path: &PathData,
    text: &str,
    font_size: f64,
    start_offset: f64,
) -> Vec<GlyphPlacement> {
    let poly = path.to_polygon(24);
    if poly.len() < 2 || text.is_empty() {
        return Vec::new();
    }

    // Cumulative length calculation
    let mut lengths = vec![0.0];
    let mut total_len = 0.0;
    for i in 0..poly.len() - 1 {
        let seg_len = poly[i].distance(poly[i + 1]);
        total_len += seg_len;
        lengths.push(total_len);
    }

    if total_len <= 1e-6 {
        return Vec::new();
    }

    let char_width = font_size * 0.6; // Average glyph spacing
    let mut placements = Vec::new();
    let mut current_dist = start_offset;

    for ch in text.chars() {
        if current_dist > total_len {
            break;
        }

        // Find segment at current_dist
        for i in 0..poly.len() - 1 {
            let d0 = lengths[i];
            let d1 = lengths[i + 1];
            if current_dist >= d0 && current_dist <= d1 {
                let seg_len = (d1 - d0).max(1e-6);
                let seg_t = (current_dist - d0) / seg_len;
                let p0 = poly[i];
                let p1 = poly[i + 1];

                let pt =
                    AnchorPoint::new(p0.x + seg_t * (p1.x - p0.x), p0.y + seg_t * (p1.y - p0.y));

                let dx = p1.x - p0.x;
                let dy = p1.y - p0.y;
                let angle = dy.atan2(dx);

                placements.push(GlyphPlacement {
                    char_value: ch,
                    position: pt,
                    rotation_rad: angle,
                });
                break;
            }
        }

        current_dist += char_width;
    }

    placements
}

/// Generate a vector PathData outlining a specific ASCII/common character.
/// Coordinates are normalized to [0.0, 1.0] in width and [0.0, 1.0] in height (origin top-left).
pub fn get_glyph_outline_path(ch: char) -> PathData {
    let mut path = PathData::new();
    match ch.to_ascii_uppercase() {
        'A' => {
            // Outer triangle
            path.push_move_to(0.5, 0.0);
            path.push_line_to(1.0, 1.0);
            path.push_line_to(0.8, 1.0);
            path.push_line_to(0.65, 0.65);
            path.push_line_to(0.35, 0.65);
            path.push_line_to(0.2, 1.0);
            path.push_line_to(0.0, 1.0);
            path.close();
            // Inner counter
            path.push_move_to(0.5, 0.25);
            path.push_line_to(0.4, 0.5);
            path.push_line_to(0.6, 0.5);
            path.close();
        }
        'B' => {
            path.push_move_to(0.1, 0.0);
            path.push_cubic_curve_to(0.7, 0.0, 0.8, 0.45, 0.5, 0.5);
            path.push_cubic_curve_to(0.85, 0.55, 0.8, 1.0, 0.1, 1.0);
            path.close();
        }
        'C' => {
            path.push_move_to(0.9, 0.2);
            path.push_cubic_curve_to(0.5, 0.0, 0.1, 0.2, 0.1, 0.5);
            path.push_cubic_curve_to(0.1, 0.8, 0.5, 1.0, 0.9, 0.8);
            path.push_line_to(0.8, 0.7);
            path.push_cubic_curve_to(0.5, 0.85, 0.3, 0.7, 0.3, 0.5);
            path.push_cubic_curve_to(0.3, 0.3, 0.5, 0.15, 0.8, 0.3);
            path.close();
        }
        'D' => {
            path.push_move_to(0.1, 0.0);
            path.push_cubic_curve_to(0.9, 0.0, 0.9, 1.0, 0.1, 1.0);
            path.close();
        }
        'E' => {
            path.push_move_to(0.1, 0.0);
            path.push_line_to(0.9, 0.0);
            path.push_line_to(0.9, 0.2);
            path.push_line_to(0.3, 0.2);
            path.push_line_to(0.3, 0.4);
            path.push_line_to(0.8, 0.4);
            path.push_line_to(0.8, 0.6);
            path.push_line_to(0.3, 0.6);
            path.push_line_to(0.3, 0.8);
            path.push_line_to(0.9, 0.8);
            path.push_line_to(0.9, 1.0);
            path.push_line_to(0.1, 1.0);
            path.close();
        }
        'F' => {
            path.push_move_to(0.1, 0.0);
            path.push_line_to(0.9, 0.0);
            path.push_line_to(0.9, 0.2);
            path.push_line_to(0.3, 0.2);
            path.push_line_to(0.3, 0.45);
            path.push_line_to(0.8, 0.45);
            path.push_line_to(0.8, 0.65);
            path.push_line_to(0.3, 0.65);
            path.push_line_to(0.3, 1.0);
            path.push_line_to(0.1, 1.0);
            path.close();
        }
        'H' => {
            path.push_move_to(0.1, 0.0);
            path.push_line_to(0.3, 0.0);
            path.push_line_to(0.3, 0.4);
            path.push_line_to(0.7, 0.4);
            path.push_line_to(0.7, 0.0);
            path.push_line_to(0.9, 0.0);
            path.push_line_to(0.9, 1.0);
            path.push_line_to(0.7, 1.0);
            path.push_line_to(0.7, 0.6);
            path.push_line_to(0.3, 0.6);
            path.push_line_to(0.3, 1.0);
            path.push_line_to(0.1, 1.0);
            path.close();
        }
        'I' => {
            path.push_move_to(0.3, 0.0);
            path.push_line_to(0.7, 0.0);
            path.push_line_to(0.7, 0.2);
            path.push_line_to(0.6, 0.2);
            path.push_line_to(0.6, 0.8);
            path.push_line_to(0.7, 0.8);
            path.push_line_to(0.7, 1.0);
            path.push_line_to(0.3, 1.0);
            path.push_line_to(0.3, 0.8);
            path.push_line_to(0.4, 0.8);
            path.push_line_to(0.4, 0.2);
            path.push_line_to(0.3, 0.2);
            path.close();
        }
        'L' => {
            path.push_move_to(0.1, 0.0);
            path.push_line_to(0.3, 0.0);
            path.push_line_to(0.3, 0.8);
            path.push_line_to(0.9, 0.8);
            path.push_line_to(0.9, 1.0);
            path.push_line_to(0.1, 1.0);
            path.close();
        }
        'O' | '0' => {
            // Outer oval
            path.push_move_to(0.5, 0.0);
            path.push_cubic_curve_to(0.95, 0.0, 0.95, 1.0, 0.5, 1.0);
            path.push_cubic_curve_to(0.05, 1.0, 0.05, 0.0, 0.5, 0.0);
            path.close();
            // Inner counter
            path.push_move_to(0.5, 0.2);
            path.push_cubic_curve_to(0.25, 0.2, 0.25, 0.8, 0.5, 0.8);
            path.push_cubic_curve_to(0.75, 0.8, 0.75, 0.2, 0.5, 0.2);
            path.close();
        }
        'T' => {
            path.push_move_to(0.0, 0.0);
            path.push_line_to(1.0, 0.0);
            path.push_line_to(1.0, 0.2);
            path.push_line_to(0.6, 0.2);
            path.push_line_to(0.6, 1.0);
            path.push_line_to(0.4, 1.0);
            path.push_line_to(0.4, 0.2);
            path.push_line_to(0.0, 0.2);
            path.close();
        }
        'V' => {
            path.push_move_to(0.0, 0.0);
            path.push_line_to(0.25, 0.0);
            path.push_line_to(0.5, 0.75);
            path.push_line_to(0.75, 0.0);
            path.push_line_to(1.0, 0.0);
            path.push_line_to(0.6, 1.0);
            path.push_line_to(0.4, 1.0);
            path.close();
        }
        'X' => {
            path.push_move_to(0.1, 0.0);
            path.push_line_to(0.35, 0.0);
            path.push_line_to(0.5, 0.3);
            path.push_line_to(0.65, 0.0);
            path.push_line_to(0.9, 0.0);
            path.push_line_to(0.65, 0.5);
            path.push_line_to(0.9, 1.0);
            path.push_line_to(0.65, 1.0);
            path.push_line_to(0.5, 0.7);
            path.push_line_to(0.35, 1.0);
            path.push_line_to(0.1, 1.0);
            path.push_line_to(0.35, 0.5);
            path.close();
        }
        ' ' => {
            // Space - empty path
        }
        _ => {
            // Default generic glyph representation (beveled block)
            path.push_move_to(0.1, 0.0);
            path.push_line_to(0.9, 0.0);
            path.push_line_to(0.9, 0.9);
            path.push_line_to(0.8, 1.0);
            path.push_line_to(0.1, 1.0);
            path.close();
            path.push_move_to(0.3, 0.2);
            path.push_line_to(0.3, 0.8);
            path.push_line_to(0.7, 0.8);
            path.push_line_to(0.7, 0.2);
            path.close();
        }
    }
    path
}

/// Convert a text string into an outlined vector PathData with proper kerning and size.
pub fn text_to_outline_path(text: &str, font_size: f64) -> PathData {
    let mut combined = PathData::new();
    let char_width = font_size * 0.6;
    let mut current_x = 0.0;

    for ch in text.chars() {
        if ch == ' ' {
            current_x += char_width * 0.7;
            continue;
        }

        let mut glyph = get_glyph_outline_path(ch);
        // Scale and translate glyph
        // Normal matrix: [sx, 0, 0, sy, tx, ty]
        let matrix = [
            char_width, 0.0, 0.0, font_size, current_x,
            -font_size, // top-left relative to baseline
        ];
        glyph.transform(&matrix);
        combined.elements.extend(glyph.elements);
        current_x += char_width * 1.1; // letter spacing
    }

    combined
}

/// Convert text into outlined vector path along a trajectory path (Text-on-Path Outlines)
pub fn text_on_path_to_outlines(
    path: &PathData,
    text: &str,
    font_size: f64,
    start_offset: f64,
) -> PathData {
    let placements = place_text_along_path(path, text, font_size, start_offset);
    let mut combined = PathData::new();
    let char_width = font_size * 0.6;

    for p in placements {
        if p.char_value == ' ' {
            continue;
        }

        let mut glyph = get_glyph_outline_path(p.char_value);
        // Transform glyph: center at origin, rotate by rotation_rad, then place at p.position
        // First center x: -0.5, y: -0.5
        let cos = p.rotation_rad.cos();
        let sin = p.rotation_rad.sin();
        let sx = char_width;
        let sy = font_size;

        // Combine: [cos*sx, sin*sx, -sin*sy, cos*sy, px, py]
        // Offset glyph so baseline is at the path
        let matrix = [
            cos * sx,
            sin * sx,
            -sin * sy,
            cos * sy,
            p.position.x - sin * (font_size * 0.5),
            p.position.y + cos * (font_size * 0.5) - font_size,
        ];
        glyph.transform(&matrix);
        combined.elements.extend(glyph.elements);
    }

    combined
}

/// Convert a Document's Text object into vector Path objects (Illustrator "Create Outlines" operation)
pub fn create_text_outlines(text_obj: &Object) -> Option<Object> {
    if let crate::core::document::ObjectType::Text { text, font_size } = &text_obj.object_type {
        let mut path = text_to_outline_path(text, *font_size);
        path.fill = text_obj.fill.clone();
        path.stroke = text_obj.stroke.clone();

        let mut outlined = Object::new_path(&format!("{}_Outlines", text_obj.name), path);
        outlined.transform = text_obj.transform.clone();
        outlined.shadow = text_obj.shadow.clone();
        outlined.glow = text_obj.glow.clone();
        outlined.opacity = text_obj.opacity;
        outlined.blend_mode = text_obj.blend_mode;
        Some(outlined)
    } else {
        None
    }
}
