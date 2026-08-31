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

                let pt = AnchorPoint::new(
                    p0.x + seg_t * (p1.x - p0.x),
                    p0.y + seg_t * (p1.y - p0.y),
                );

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
