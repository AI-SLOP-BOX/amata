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
    if !font_size.is_finite() || !start_offset.is_finite() {
        return Vec::new();
    }
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
        // Newlines have no on-path glyph: placing them drew tofu blocks.
        if ch == '\n' || ch == '\r' {
            continue;
        }
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

/// Flattened polygon of `path` plus cumulative arc lengths and total length.
fn poly_lengths(path: &PathData) -> (Vec<AnchorPoint>, Vec<f64>, f64) {
    let poly = path.to_polygon(24);
    let mut lengths = Vec::with_capacity(poly.len());
    let mut total = 0.0;
    lengths.push(0.0);
    for i in 0..poly.len().saturating_sub(1) {
        total += poly[i].distance(poly[i + 1]);
        lengths.push(total);
    }
    (poly, lengths, total)
}

/// Total arc length of `path` (24 subdivisions per curve), used to clamp
/// Text-on-Path `start_offset`.
pub fn path_total_length(path: &PathData) -> f64 {
    poly_lengths(path).2
}

/// Point at arc-length `dist` along `path`, clamped to `[0, total]`.
/// Returns `None` for degenerate (empty / zero-length) paths.
pub fn arc_point_at(path: &PathData, dist: f64) -> Option<AnchorPoint> {
    let (poly, lengths, total) = poly_lengths(path);
    if poly.len() < 2 || total <= 1e-6 {
        return None;
    }
    let dist = dist.clamp(0.0, total);
    for i in 0..poly.len() - 1 {
        let (d0, d1) = (lengths[i], lengths[i + 1]);
        if dist >= d0 && dist <= d1 {
            let seg_len = (d1 - d0).max(1e-6);
            let t = (dist - d0) / seg_len;
            let (p0, p1) = (poly[i], poly[i + 1]);
            return Some(AnchorPoint::new(
                p0.x + t * (p1.x - p0.x),
                p0.y + t * (p1.y - p0.y),
            ));
        }
    }
    poly.last().copied()
}

/// Project a local-space point onto `path`; returns the arc length of the
/// closest point on the flattened curve. Drives the Text-on-Path slide handle.
pub fn project_to_arc_length(path: &PathData, lx: f64, ly: f64) -> Option<f64> {
    let (poly, lengths, total) = poly_lengths(path);
    if poly.len() < 2 || total <= 1e-6 {
        return None;
    }
    let p = AnchorPoint::new(lx, ly);
    let mut best_dist = f64::MAX;
    let mut best_s = 0.0;
    for i in 0..poly.len() - 1 {
        let (a, b) = (poly[i], poly[i + 1]);
        let seg_len = b.distance(a).max(1e-12);
        let t = (((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / (seg_len * seg_len))
            .clamp(0.0, 1.0);
        let q = AnchorPoint::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
        let d = p.distance(q);
        if d < best_dist {
            best_dist = d;
            best_s = lengths[i] + t * seg_len;
        }
    }
    Some(best_s)
}

/// [`place_text_along_path`] honouring `letter_spacing` (advance per glyph =
/// `font_size * 0.6 + letter_spacing`). Used by the live Text-on-Path object.
pub fn place_text_along_path_styled(
    path: &PathData,
    text: &str,
    font_size: f64,
    letter_spacing: f64,
    start_offset: f64,
) -> Vec<GlyphPlacement> {
    if !font_size.is_finite() || !start_offset.is_finite() {
        return Vec::new();
    }
    let letter_spacing = if letter_spacing.is_finite() {
        letter_spacing
    } else {
        0.0
    };
    let poly = path.to_polygon(24);
    if poly.len() < 2 || text.is_empty() {
        return Vec::new();
    }
    let mut lengths = vec![0.0];
    let mut total_len = 0.0;
    for i in 0..poly.len() - 1 {
        total_len += poly[i].distance(poly[i + 1]);
        lengths.push(total_len);
    }
    if total_len <= 1e-6 {
        return Vec::new();
    }
    let mut placements = Vec::new();
    let mut current_dist = start_offset;
    for ch in text.chars() {
        // Newlines have no on-path glyph: placing them drew tofu blocks.
        if ch == '\n' || ch == '\r' {
            continue;
        }
        if current_dist > total_len {
            break;
        }
        for i in 0..poly.len() - 1 {
            let (d0, d1) = (lengths[i], lengths[i + 1]);
            if current_dist >= d0 && current_dist <= d1 {
                let seg_len = (d1 - d0).max(1e-6);
                let seg_t = (current_dist - d0) / seg_len;
                let p0 = poly[i];
                let p1 = poly[i + 1];
                let pt =
                    AnchorPoint::new(p0.x + seg_t * (p1.x - p0.x), p0.y + seg_t * (p1.y - p0.y));
                let angle = (p1.y - p0.y).atan2(p1.x - p0.x);
                placements.push(GlyphPlacement {
                    char_value: ch,
                    position: pt,
                    rotation_rad: angle,
                });
                break;
            }
        }
        current_dist += font_size * 0.6 + letter_spacing;
    }
    placements
}

/// Live outline generation for `ObjectType::TextOnPath`: styled placement plus
/// optional 180° flip (`side == Bottom`) around each glyph's arc anchor
/// (linear part negated, translation mirrored: `t' = 2p - t`).
pub fn text_on_path_outlines(
    path: &PathData,
    text: &str,
    style: &crate::core::document::TextStyle,
    start_offset: f64,
    side: crate::core::document::TextPathSide,
) -> PathData {
    use crate::core::document::TextPathSide;
    let font_size = style.font_size;
    let placements =
        place_text_along_path_styled(path, text, font_size, style.letter_spacing, start_offset);
    let mut combined = PathData::new();
    let char_width = font_size * 0.6;
    let flip = side == TextPathSide::Bottom;

    for p in placements {
        if p.char_value == ' ' {
            continue;
        }
        let mut glyph = get_glyph_outline_path(p.char_value);
        let cos = p.rotation_rad.cos();
        let sin = p.rotation_rad.sin();
        let mut matrix = [
            cos * char_width,
            sin * char_width,
            -sin * font_size,
            cos * font_size,
            p.position.x - sin * (font_size * 0.5),
            p.position.y + cos * (font_size * 0.5) - font_size,
        ];
        if flip {
            matrix[0] = -matrix[0];
            matrix[1] = -matrix[1];
            matrix[2] = -matrix[2];
            matrix[3] = -matrix[3];
            matrix[4] = 2.0 * p.position.x - matrix[4];
            matrix[5] = 2.0 * p.position.y - matrix[5];
        }
        glyph.transform(&matrix);
        combined.elements.extend(glyph.elements);
    }
    combined
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

/// OutlineBuilder for ttf-parser that converts TrueType/OpenType contours into Amata PathData.
struct PathOutlineBuilder {
    path: PathData,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

impl ttf_parser::OutlineBuilder for PathOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let px = self.offset_x + (x as f64) * self.scale;
        let py = self.offset_y - (y as f64) * self.scale; // In TTF, Y is up, in SVG Y is down
        self.path.push_move_to(px, py);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let px = self.offset_x + (x as f64) * self.scale;
        let py = self.offset_y - (y as f64) * self.scale;
        self.path.push_line_to(px, py);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let cx = self.offset_x + (x1 as f64) * self.scale;
        let cy = self.offset_y - (y1 as f64) * self.scale;
        let px = self.offset_x + (x as f64) * self.scale;
        let py = self.offset_y - (y as f64) * self.scale;
        self.path.push_quad_curve_to(cx, cy, px, py);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let c1x = self.offset_x + (x1 as f64) * self.scale;
        let c1y = self.offset_y - (y1 as f64) * self.scale;
        let c2x = self.offset_x + (x2 as f64) * self.scale;
        let c2y = self.offset_y - (y2 as f64) * self.scale;
        let px = self.offset_x + (x as f64) * self.scale;
        let py = self.offset_y - (y as f64) * self.scale;
        self.path.push_cubic_curve_to(c1x, c1y, c2x, c2y, px, py);
    }

    fn close(&mut self) {
        self.path.close();
    }
}

/// One HarfBuzz-shaped glyph: final glyph id plus advances/offsets in
/// font units (y-up, like the outline builder's glyph space).
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub gid: u32,
    pub x_advance: f32,
    pub y_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    /// Byte index of the source cluster (for space/.notdef decisions).
    pub cluster: u32,
}

/// Shape a run with HarfBuzz: full GSUB/GPOS (ligatures, kerning, mark
/// positioning, complex scripts) instead of the manual cmap + legacy
/// `kern`-table walk below. Returns `None` when shaping is unavailable so
/// callers fall back to the manual path.
pub fn shape_run_hb(
    data: &[u8],
    index: u32,
    text: &str,
    features: &[([u8; 4], u32)],
    variations: &[crate::core::document::VariationSetting],
) -> Option<Vec<ShapedGlyph>> {
    shape_run_hb_dir(data, index, text, false, features, variations)
}

/// Directional shaping: `vertical` selects HarfBuzz's top-to-bottom
/// direction (real 縦組み with `vert`/`vrt2` glyph forms) instead of
/// horizontal. [`shape_run_hb`] stays the horizontal entry point.
pub fn shape_run_hb_dir(
    data: &[u8],
    index: u32,
    text: &str,
    vertical: bool,
    features: &[([u8; 4], u32)],
    variations: &[crate::core::document::VariationSetting],
) -> Option<Vec<ShapedGlyph>> {
    let font = read_fonts::FontRef::from_index(data, index).ok()?;
    let shaper_data = harfrust::ShaperData::new(&font);
    // Variable-font axes: without the instance the shaper (and the
    // outlines below) would silently use the default master while the
    // manual fallback path applies coordinates — same text, two widths.
    let vars: Vec<(read_fonts::types::Tag, f32)> = variations
        .iter()
        .filter_map(|v| {
            // Non-finite coordinates would reach the instance as NaN
            // advances (see `TextStyle::set_variation`); drop them here too
            // for faces built directly from a deserialized style.
            if !v.value.is_finite() {
                log::warn!("dropping non-finite variation axis {:?}", v.axis);
                return None;
            }
            // OpenType axis tags are exactly 4 bytes and case-sensitive
            // (e.g. `wght`, `opsz`, `WIDT`, digits allowed). Reject only
            // non-tag shapes; never truncate silently.
            let bytes = v.axis.as_bytes();
            if bytes.len() != 4
                || !bytes
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() && *b != b' ')
            {
                log::warn!("dropping invalid variation axis {:?}", v.axis);
                return None;
            }
            let mut tag = [0u8; 4];
            tag.copy_from_slice(bytes);
            Some((read_fonts::types::Tag::new(&tag), v.value as f32))
        })
        .collect();
    let instance;
    let mut builder = shaper_data.shaper(&font);
    if !vars.is_empty() {
        instance = harfrust::ShaperInstance::from_variations(&font, vars);
        builder = builder.instance(Some(&instance));
    }
    let shaper = builder.build();
    let mut buffer = harfrust::UnicodeBuffer::new();
    buffer.push_str(text);
    if vertical {
        buffer.set_direction(harfrust::Direction::TopToBottom);
    } else {
        buffer.guess_segment_properties();
    }
    let no_liga;
    let features: &[harfrust::Feature] = if features.is_empty() {
        &[]
    } else {
        no_liga = features
            .iter()
            .map(|(tag, value)| {
                harfrust::Feature::new(read_fonts::types::Tag::new(tag), *value, ..)
            })
            .collect::<Vec<_>>();
        &no_liga
    };
    let shaped = shaper.shape(buffer, harfrust::ShapeOptions::new().features(features));
    let infos = shaped.glyph_infos();
    let pos = shaped.glyph_positions();
    if infos.len() != pos.len() {
        return None;
    }
    Some(
        infos
            .iter()
            .zip(pos.iter())
            .map(|(i, p)| ShapedGlyph {
                gid: i.glyph_id,
                x_advance: p.x_advance as f32,
                y_advance: p.y_advance as f32,
                x_offset: p.x_offset as f32,
                y_offset: p.y_offset as f32,
                cluster: i.cluster,
            })
            .collect(),
    )
}

/// Faux italic/oblique shear factor: tan(12°). Matches the SVG export
/// `skewX(-12)` so outlined logos and exported text slant identically.
pub const FAUX_ITALIC_SHEAR: f64 = 0.2126;

/// Convert a text string into an outlined vector PathData with proper kerning and size,
/// using the requested TextStyle and real font glyph outlines via ttf-parser.
/// Falls back to mock block glyphs when no face resolves.
pub fn text_to_outline_path_with_style(
    text: &str,
    style: &crate::core::document::TextStyle,
) -> PathData {
    // Mock fallbacks must not show ruby markup: strip it here too.
    let (base, _) = crate::core::document::parse_ruby(text);
    try_text_to_outline_path_with_style(text, style)
        .unwrap_or_else(|| mock_text_outline(&base, style))
}

/// Real-face outlines only: `None` when no font face resolves, or when the
/// face lacks any glyph in the run (all-or-nothing, so the caller can fall
/// back to a coherent renderer — e.g. CJK text in a Latin-only face falls
/// back to the UI cascade's Noto instead of a ransom note of mock blocks).
/// Single-line: the caller splits/wraps multi-line text itself so per-line
/// bboxes stay available for anchoring.
pub fn try_text_to_outline_path_with_style(
    text: &str,
    style: &crate::core::document::TextStyle,
) -> Option<PathData> {
    use crate::core::document::char_advance_estimate;
    // Normalize CRLF/CR/U+2028/29 first: a stray `\r` maps to no glyph, and
    // the all-or-nothing fallback below would tofu the entire run for it.
    let normalized = crate::core::document::normalize_text(text);
    let text = normalized.as_ref();
    let font_size = style.effective_font_size();
    let letter_spacing = style.effective_letter_spacing();
    let registry = super::font::FontRegistry::global();
    // Faux italic only when no real italic/oblique face exists; otherwise
    // the resolved face already slants and shearing would double it.
    let faux_italic = !matches!(style.font_style, crate::core::document::FontStyle::Normal)
        && registry.needs_synthetic_style(&style.font_family, style.font_weight, style.font_style);

    // 縦組み: lay out as a column — upright fullwidth glyphs, everything
    // else rotated 90° CW — so the cached-outline canvas path and the PDF
    // printer get correct vertical stacking for free.
    if style.vertical {
        return vertical_column_outline(text, style, font_size, letter_spacing);
    }

    // ルビ (horizontal): readings ride above the base run. The base run
    // keeps the full outline path (kerning, per-glyph fallback faces);
    // readings are placed from the same estimates the canvas and SVG use.
    let (base, anns) = crate::core::document::parse_ruby(text);
    if !anns.is_empty() {
        return horizontal_ruby_outline(&base, &anns, style, font_size, letter_spacing);
    }

    // Attempt to extract real glyph outlines from the resolved font face
    let extracted: Option<Option<PathData>> = registry.query_face_data(
        &style.font_family,
        style.font_weight,
        style.font_style,
        |data, index| {
            if let Ok(mut face) = ttf_parser::Face::parse(data, index) {
                // Variable-font coordinates must land on the face before any
                // outline_glyph / glyph_hor_advance query, or the deltas never
                // apply (static faces ignore unknown tags harmlessly).
                super::font::FontRegistry::apply_variations(&mut face, &style.variations);
                let units_per_em = face.units_per_em() as f64;
                if units_per_em > 0.0 {
                    let scale = font_size / units_per_em;
                    // Per-glyph fallback segmentation: emoji / missing
                    // glyphs go through a fallback face per segment while
                    // the rest of the run keeps the primary face, instead
                    // of all-or-nothing degradation of the entire line.
                    let missing_any = text.chars().any(|ch| {
                        !(char_advance_estimate(ch) == 0.0 || ch == ' ' || ch == '\t')
                            && face.glyph_index(ch).is_none()
                    });
                    if missing_any {
                        let mut combined = PathData::new();
                        let mut current_x = 0.0;
                        let mut segs: Vec<(String, bool)> = Vec::new();
                        for ch in text.chars() {
                            let cov = char_advance_estimate(ch) == 0.0
                                || ch == ' '
                                || ch == '\t'
                                || face.glyph_index(ch).is_some();
                            match segs.last_mut() {
                                Some((s, c)) if *c == cov => s.push(ch),
                                _ => segs.push((ch.to_string(), cov)),
                            }
                        }
                        let mut prev_sig: Option<char> = None;
                        for (seg, cov) in &segs {
                            if style.auto_spacing {
                                if let Some(p) = prev_sig {
                                    if let Some(c0) =
                                        seg.chars().find(|c| char_advance_estimate(*c) != 0.0)
                                    {
                                        if crate::core::document::is_ja_latin_boundary(p, c0) {
                                            current_x += crate::core::document::ja_latin_gap_em(
                                                font_size,
                                                style.auto_spacing_em as f64,
                                            );
                                        }
                                    }
                                }
                            }
                            if *cov {
                                if let Some(mut p) = try_text_to_outline_path_with_style(seg, style)
                                {
                                    p.transform(&[1.0, 0.0, 0.0, 1.0, current_x, 0.0]);
                                    combined.elements.extend(p.elements);
                                }
                            } else {
                                let mut seg_x = current_x;
                                for ch in seg.chars() {
                                    if char_advance_estimate(ch) == 0.0 {
                                        continue;
                                    }
                                    let adv =
                                        char_advance_estimate(ch) * font_size + letter_spacing;
                                    let produced = registry
                                        .query_fallback_face_data_for(
                                            ch,
                                            style.font_weight,
                                            style.font_style,
                                            |fd, fi| {
                                                outline_with_face_data(
                                                    fd,
                                                    fi,
                                                    &ch.to_string(),
                                                    style,
                                                    seg_x,
                                                )
                                            },
                                        )
                                        .flatten();
                                    if let Some(ol) = produced {
                                        combined.elements.extend(ol.elements);
                                    }
                                    seg_x += adv;
                                }
                            }
                            current_x += crate::core::document::text_advance_estimate(seg, style);
                            for c in seg.chars() {
                                if char_advance_estimate(c) != 0.0 {
                                    prev_sig = Some(c);
                                }
                            }
                        }
                        if faux_italic {
                            combined.transform(&[1.0, 0.0, -FAUX_ITALIC_SHEAR, 1.0, 0.0, 0.0]);
                        }
                        if combined.elements.is_empty() {
                            return None;
                        }
                        return Some(combined);
                    }
                    // HarfBuzz shaping first: full GSUB/GPOS (ligatures,
                    // kerning incl. GPOS pairs, mark attachment, complex
                    // scripts). HarfBuzz positions are y-up font units, so
                    // y_offset is negated into the builder's y-down space.
                    if let Some(shaped) = shape_run_hb(
                        data,
                        index,
                        text,
                        &style.ot_feature_pairs(),
                        &style.variations,
                    ) {
                        let mut usable = true;
                        for g in &shaped {
                            // .notdef for a real character: fall back to the
                            // manual path (which returns None → mock blocks
                            // coherently) instead of outlining tofu.
                            let ch = text
                                .get(g.cluster as usize..)
                                .and_then(|s| s.chars().next());
                            if g.gid == 0 && !matches!(ch, Some(' ') | Some('\t')) {
                                usable = false;
                                break;
                            }
                        }
                        if usable {
                            let mut combined = PathData::new();
                            let mut current_x = 0.0;
                            let mut prev_sig: Option<char> = None;
                            for g in &shaped {
                                let gid = ttf_parser::GlyphId(g.gid.min(u16::MAX as u32) as u16);
                                // 和欧間: ~1/4em gap between a Japanese and a
                                // Latin significant char.
                                let ch = text
                                    .get(g.cluster as usize..)
                                    .and_then(|s| s.chars().next());
                                if style.auto_spacing {
                                    if let (Some(p), Some(c)) = (prev_sig, ch) {
                                        if crate::core::document::is_ja_latin_boundary(p, c) {
                                            current_x += crate::core::document::ja_latin_gap_em(
                                                font_size,
                                                style.auto_spacing_em as f64,
                                            );
                                        }
                                    }
                                }
                                let mut builder = PathOutlineBuilder {
                                    path: PathData::new(),
                                    scale,
                                    offset_x: current_x + g.x_offset as f64 * scale,
                                    offset_y: -g.y_offset as f64 * scale,
                                };
                                let _ = face.outline_glyph(gid, &mut builder);
                                combined.elements.extend(builder.path.elements);
                                current_x += g.x_advance as f64 * scale + letter_spacing;
                                if let Some(c) = ch {
                                    if crate::core::document::char_advance_estimate(c) != 0.0 {
                                        prev_sig = Some(c);
                                    }
                                }
                            }
                            if faux_italic {
                                combined.transform(&[1.0, 0.0, -FAUX_ITALIC_SHEAR, 1.0, 0.0, 0.0]);
                            }
                            return Some(combined);
                        }
                    }
                    let mut combined = PathData::new();
                    let mut current_x = 0.0;
                    // `kern` (old-style TrueType kerning) table, when present.
                    let kern_table = face.tables().kern;
                    let mut prev_glyph: Option<ttf_parser::GlyphId> = None;

                    // Build a glyph-id run first so GSUB ligatures can collapse
                    // sequences before outlining. `gap_before[i]` mirrors
                    // `glyphs[i]`: true when a 和欧 boundary gap belongs
                    // before that glyph.
                    let mut glyphs: Vec<ttf_parser::GlyphId> = Vec::new();
                    let mut gap_before: Vec<bool> = Vec::new();
                    let mut prev_sig: Option<char> = None;
                    for ch in text.chars() {
                        if ch == ' ' || ch == '\t' {
                            // Spaces/tabs break ligature runs and advance a fixed amount.
                            glyphs.push(ttf_parser::GlyphId(0)); // sentinel for space
                            gap_before.push(false);
                            continue;
                        }
                        if char_advance_estimate(ch) == 0.0 {
                            // Zero-width format chars (VS16/ZWJ/IVS/combining):
                            // no outline of their own and usually no cmap entry.
                            // Treating them as missing would tofu the whole run.
                            continue;
                        }
                        let gap = style.auto_spacing
                            && prev_sig
                                .map(|p| crate::core::document::is_ja_latin_boundary(p, ch))
                                .unwrap_or(false);
                        match face.glyph_index(ch) {
                            Some(g) => {
                                glyphs.push(g);
                                gap_before.push(gap);
                            }
                            None => return None,
                        }
                        prev_sig = Some(ch);
                    }

                    if style.ligatures {
                        apply_liga_substitutions(&face, &mut glyphs, &mut gap_before);
                    }

                    for (gid, gap) in glyphs.into_iter().zip(gap_before) {
                        if gid.0 == 0 && prev_glyph.is_none() && text.contains(' ') {
                            // Fall through: treat glyph 0 after space as space advance.
                        }
                        // Detect space sentinel: we pushed GlyphId(0) for ' '.
                        // Real .notdef is also 0 but we already returned None for
                        // missing cmap entries, so 0 here only means space.
                        if gid.0 == 0 {
                            current_x += (font_size * 0.3) + letter_spacing;
                            prev_glyph = None;
                            continue;
                        }
                        // 和欧間 auto-spacing.
                        if gap {
                            current_x += crate::core::document::ja_latin_gap_em(
                                font_size,
                                style.auto_spacing_em as f64,
                            );
                        }
                        if let Some(prev) = prev_glyph {
                            if let Some(kern) = &kern_table {
                                for sub in kern.subtables {
                                    if !sub.horizontal {
                                        continue;
                                    }
                                    if let Some(k) = sub.glyphs_kerning(prev, gid) {
                                        current_x += k as f64 * scale;
                                        break;
                                    }
                                }
                            }
                        }
                        prev_glyph = Some(gid);
                        let mut builder = PathOutlineBuilder {
                            path: PathData::new(),
                            scale,
                            offset_x: current_x,
                            offset_y: 0.0,
                        };
                        let _ = face.outline_glyph(gid, &mut builder);
                        combined.elements.extend(builder.path.elements);

                        let adv = face.glyph_hor_advance(gid).unwrap_or(face.units_per_em()) as f64
                            * scale;
                        current_x += adv + letter_spacing;
                    }
                    if faux_italic {
                        // Shear around the baseline origin: tops lean right.
                        // Builder output is y-down, so x' = x − k·y.
                        combined.transform(&[1.0, 0.0, -FAUX_ITALIC_SHEAR, 1.0, 0.0, 0.0]);
                    }
                    return Some(combined);
                }
            }
            None
        },
    );

    if let Some(Some(path)) = extracted {
        return Some(path);
    }

    None
}

/// Outline a single character (or short run) with an explicit face,
/// offset in X by `offset_x`. Used for per-glyph fallback outlines.
fn outline_with_face_data(
    data: &[u8],
    index: u32,
    text: &str,
    style: &crate::core::document::TextStyle,
    offset_x: f64,
) -> Option<PathData> {
    let mut face = ttf_parser::Face::parse(data, index).ok()?;
    super::font::FontRegistry::apply_variations(&mut face, &style.variations);
    let upem = face.units_per_em() as f64;
    if upem <= 0.0 {
        return None;
    }
    let font_size = style.effective_font_size();
    let scale = font_size / upem;
    let shaped = shape_run_hb(
        data,
        index,
        text,
        &style.ot_feature_pairs(),
        &style.variations,
    )?;
    let mut combined = PathData::new();
    let mut current_x = offset_x;
    for g in &shaped {
        let ch = text
            .get(g.cluster as usize..)
            .and_then(|s| s.chars().next());
        if g.gid == 0 && !matches!(ch, Some(' ') | Some('\t')) {
            return None;
        }
        let gid = ttf_parser::GlyphId(g.gid.min(u16::MAX as u32) as u16);
        let mut builder = PathOutlineBuilder {
            path: PathData::new(),
            scale,
            offset_x: current_x + g.x_offset as f64 * scale,
            offset_y: -g.y_offset as f64 * scale,
        };
        let _ = face.outline_glyph(gid, &mut builder);
        combined.elements.extend(builder.path.elements);
        current_x += g.x_advance as f64 * scale + style.effective_letter_spacing();
    }
    Some(combined)
}

/// Horizontal ルビ: base run through the normal outline path, each
/// reading typeset at half size above its base group's centre. Positions
/// come from the same estimates the canvas and SVG exporters use, so the
/// three agree (real glyph advance is only consulted for the base run).
fn horizontal_ruby_outline(
    base: &str,
    anns: &[crate::core::document::RubyAnnotation],
    style: &crate::core::document::TextStyle,
    font_size: f64,
    letter_spacing: f64,
) -> Option<PathData> {
    use crate::core::document::{
        char_advance_estimate, is_ja_latin_boundary, ja_latin_gap_em, RUBY_ABOVE_EM, RUBY_SCALE,
    };
    // Base run: no ruby notation left, so this cannot recurse.
    let mut base_path = try_text_to_outline_path_with_style(base, style)?;
    let chars: Vec<char> = base.chars().collect();
    // Group x-extents, derived from the same advance model as layout.
    let mut groups: Vec<(f64, f64, &crate::core::document::RubyAnnotation)> = Vec::new();
    let mut x = 0.0;
    let mut prev: Option<char> = None;
    let mut open: Option<(f64, usize)> = None;
    for (i, ch) in chars.iter().enumerate() {
        if char_advance_estimate(*ch) == 0.0 {
            continue;
        }
        if style.auto_spacing {
            if let Some(p) = prev {
                if is_ja_latin_boundary(p, *ch) {
                    x += ja_latin_gap_em(font_size, style.auto_spacing_em as f64);
                }
            }
        }
        if let Some(ann) = anns.iter().find(|a| a.start == i) {
            open = Some((x, ann.len));
        }
        x += char_advance_estimate(*ch) * font_size + letter_spacing;
        if let Some((start_x, left)) = open {
            if left <= 1 {
                open = None;
                if let Some(ann) = anns.iter().find(|a| a.start + a.len == i + 1) {
                    groups.push((start_x, x, ann));
                }
            } else {
                open = Some((start_x, left - 1));
            }
        }
        prev = Some(*ch);
    }
    for (start_x, end_x, ann) in groups {
        let mut ol = match try_text_to_outline_path_with_style(&ann.reading, style) {
            Some(p) => p,
            None => continue,
        };
        if let Some((mn, mx)) = ol.bounding_box() {
            let w = (mx.x - mn.x) * RUBY_SCALE;
            let h = (mx.y - mn.y) * RUBY_SCALE;
            let cx = (start_x + end_x) / 2.0;
            // Outline paths are y-down, so "above the baseline" is negative.
            let cy = -font_size * RUBY_ABOVE_EM;
            let tx = cx - w / 2.0 - mn.x * RUBY_SCALE;
            let ty = cy - h / 2.0 - mn.y * RUBY_SCALE;
            ol.transform(&[RUBY_SCALE, 0.0, 0.0, RUBY_SCALE, tx, ty]);
            base_path.elements.extend(ol.elements);
        }
    }
    Some(base_path)
}

/// OpenType 縦組み: HarfBuzz top-to-bottom shaping with `vert`/`vrt2`,
/// so fullwidth punctuation and composed vertical forms come from the
/// font instead of our own rotation. Latin/digits that the face leaves
/// un-substituted still rotate 90° CW (Illustrator's classic look).
/// Returns `None` when no face resolves or shaping fails, so callers
/// fall back to the estimate-based column.
fn vertical_open_type_outline(
    base: &str,
    anns: &[crate::core::document::RubyAnnotation],
    style: &crate::core::document::TextStyle,
    font_size: f64,
    letter_spacing: f64,
) -> Option<PathData> {
    use crate::core::document::{
        char_advance_estimate, is_fullwidth_char, is_ja_latin_boundary, ja_latin_gap_em,
        tatechuyoko_run, tatechuyoko_scale, RubyAnnotation, RUBY_SCALE, RUBY_STRIP_CENTER_EM,
    };
    let registry = super::font::FontRegistry::global();
    registry
        .query_face_data(
            &style.font_family,
            style.font_weight,
            style.font_style,
            |data, index| {
                let face = ttf_parser::Face::parse(data, index).ok()?;
                // Varied outlines need the coordinates on the face used for
                // `outline_glyph` (the shaper instance carries its own copy).
                let mut varied = face.clone();
                super::font::FontRegistry::apply_variations(&mut varied, &style.variations);
                let upem = face.units_per_em() as f64;
                if upem <= 0.0 {
                    return None;
                }
                let scale = font_size / upem;
                let pairs = vertical_feature_pairs(style);
                let chars: Vec<char> = base.chars().collect();
                let mut combined = PathData::new();
                let mut produced = false;
                // Column y grows downward in path space (matches the estimate
                // path and the PDF y-flip pipeline).
                let mut col_y = 0.0_f64;
                let center_x = font_size / 2.0;
                let mut i = 0usize;
                let mut prev_sig: Option<char> = None;
                // Ruby group open at this char: (start col_y, chars left).
                let mut ruby_open: Option<(f64, usize, &RubyAnnotation)> = None;
                let horizontal = {
                    let mut h = style.clone();
                    h.vertical = false;
                    h
                };
                while i < chars.len() {
                    // 縦中横: 2–3 digits are set horizontally in one em cell.
                    if let Some(n) = tatechuyoko_run(&chars, i) {
                        let unit: String = chars[i..i + n].iter().collect();
                        let adv = font_size + letter_spacing;
                        if let Some(mut ol) =
                            try_text_to_outline_path_with_style(&unit, &horizontal)
                        {
                            let s = tatechuyoko_scale(n);
                            if let Some((mn, mx)) = ol.bounding_box() {
                                let w = (mx.x - mn.x) * s;
                                let h = (mx.y - mn.y) * s;
                                let tx = (font_size - w) / 2.0 - mn.x * s;
                                let ty = (col_y + (adv - h) / 2.0) - mn.y * s;
                                ol.transform(&[s, 0.0, 0.0, s, tx, ty]);
                            }
                            combined.elements.extend(ol.elements);
                            produced = true;
                        }
                        col_y += adv;
                        prev_sig = Some(chars[i + n - 1]);
                        i += n;
                        continue;
                    }
                    // Shape everything up to the next 縦中横 unit in one run,
                    // so kerning/vkrn apply across it.
                    let mut run_end = i;
                    while run_end < chars.len() && tatechuyoko_run(&chars, run_end).is_none() {
                        run_end += 1;
                    }
                    let run: String = chars[i..run_end].iter().collect();
                    let mut base_char_idx = i;
                    let shaped =
                        shape_run_hb_dir(data, index, &run, true, &pairs, &style.variations)?;
                    for g in &shaped {
                        let ch = run
                            .get(g.cluster as usize..)
                            .and_then(|s| s.chars().next())
                            .unwrap_or('\u{FFFD}');
                        if char_advance_estimate(ch) == 0.0 {
                            continue;
                        }
                        if style.auto_spacing {
                            if let Some(p) = prev_sig {
                                if is_ja_latin_boundary(p, ch) {
                                    col_y +=
                                        ja_latin_gap_em(font_size, style.auto_spacing_em as f64);
                                }
                            }
                        }
                        if let Some(ann) = anns.iter().find(|a| a.start == base_char_idx) {
                            ruby_open = Some((col_y, ann.len, ann));
                        }
                        let gid = ttf_parser::GlyphId(g.gid.min(u16::MAX as u32) as u16);
                        // The face's own vertical form (vert/vrt2 substituted)
                        // is drawn as-is; an un-substituted halfwidth glyph
                        // rotates 90° CW like the estimate path.
                        let vert_form = face.glyph_index(ch) != Some(gid);
                        let mut builder = PathOutlineBuilder {
                            path: PathData::new(),
                            scale,
                            offset_x: center_x + g.x_offset as f64 * scale,
                            offset_y: col_y - g.y_offset as f64 * scale,
                        };
                        let _ = varied.outline_glyph(gid, &mut builder);
                        let mut ol = builder.path;
                        if !vert_form && !is_fullwidth_char(ch) {
                            ol.transform(&[0.0, 1.0, -1.0, 0.0, 0.0, 0.0]);
                            if let Some((mn, mx)) = ol.bounding_box() {
                                let tx = (font_size - (mx.x - mn.x)) / 2.0 - mn.x;
                                let ty = (col_y + font_size - (mx.y - mn.y)) / 2.0 - mn.y;
                                ol.transform(&[1.0, 0.0, 0.0, 1.0, tx, ty]);
                            }
                        }
                        if !ol.elements.is_empty() {
                            combined.elements.extend(ol.elements);
                            produced = true;
                        }
                        col_y += -(g.y_advance as f64) * scale;
                        prev_sig = Some(ch);
                        // Ruby group closing here: set the reading to the right
                        // of the base column, centred on the group's extent.
                        if let Some((start_y, left, ann)) = ruby_open {
                            if left <= 1 {
                                ruby_open = None;
                                if let Some(mut ol) =
                                    try_text_to_outline_path_with_style(&ann.reading, &horizontal)
                                {
                                    if let Some((mn, mx)) = ol.bounding_box() {
                                        let w = (mx.x - mn.x) * RUBY_SCALE;
                                        let h = (mx.y - mn.y) * RUBY_SCALE;
                                        let cy = (start_y + col_y) / 2.0;
                                        let tx = font_size * RUBY_STRIP_CENTER_EM
                                            - w / 2.0
                                            - mn.x * RUBY_SCALE;
                                        let ty = cy - h / 2.0 - mn.y * RUBY_SCALE;
                                        ol.transform(&[RUBY_SCALE, 0.0, 0.0, RUBY_SCALE, tx, ty]);
                                        combined.elements.extend(ol.elements);
                                        produced = true;
                                    }
                                }
                            } else {
                                ruby_open = Some((start_y, left - 1, ann));
                            }
                        }
                        base_char_idx += 1;
                    }
                    i = run_end;
                }
                produced.then_some(combined)
            },
        )
        .flatten()
}
/// OpenType feature pairs for vertical shaping: the style's own
/// overrides first (so an explicit `vert`/`vrt2` off wins), then the
/// vertical layout features HarfBuzz leaves off unless they are listed.
pub fn vertical_feature_pairs(style: &crate::core::document::TextStyle) -> Vec<([u8; 4], u32)> {
    let mut pairs = style.ot_feature_pairs();
    for tag in [
        b"vert", b"vrt2", b"vkrn", b"vpal", b"valt", b"vchw", b"vrtr",
    ] {
        if !pairs.iter().any(|(t, _)| t == tag) {
            pairs.push((*tag, 1));
        }
    }
    pairs
}

/// Vertical (縦組み) column outlines: fullwidth glyphs stand upright,
/// everything else (Latin, digits, ASCII punctuation) is rotated 90° CW
/// around its cell. Used by the outline path when `style.vertical`.
///
/// The font's OpenType vertical forms (`vert`/`vrt2`) win whenever the
/// face provides them (see [`vertical_open_type_outline`]); the
/// estimate-based rotation below is the fallback for faces without
/// vertical layout (or when shaping is unavailable).
pub fn vertical_column_outline(
    text: &str,
    style: &crate::core::document::TextStyle,
    font_size: f64,
    letter_spacing: f64,
) -> Option<PathData> {
    use crate::core::document::{
        char_advance_estimate, is_fullwidth_char, is_ja_latin_boundary, ja_latin_gap_em,
        normalize_text, parse_ruby, tatechuyoko_run, tatechuyoko_scale, RubyAnnotation, RUBY_SCALE,
        RUBY_STRIP_CENTER_EM,
    };
    let normalized = normalize_text(text);
    let text = normalized.as_ref();
    // Ruby notation is stripped first; readings are drawn to the right of
    // their base group once that group's last glyph is placed.
    let (base, anns) = parse_ruby(text);
    // OpenType 縦組み (vert/vrt2): the font's own vertical forms.
    // Readings and 縦中横 are handled inside, so the fast path returns a
    // complete column.
    if let Some(path) = vertical_open_type_outline(&base, &anns, style, font_size, letter_spacing) {
        return Some(path);
    }
    let base_chars: Vec<char> = base.chars().collect();
    let mut horiz = style.clone();
    horiz.vertical = false;
    let mut combined = PathData::new();
    let mut produced = false;
    let mut prev: Option<char> = None;
    let mut y = 0.0_f64;
    let mut i = 0usize;
    let mut active: Option<(f64, usize, &RubyAnnotation)> = None;
    while i < base_chars.len() {
        let ch = base_chars[i];
        // Layout splits columns per string; '\n' would overlap columns.
        if ch == '\n' || ch == '\r' {
            prev = None;
            y += style.effective_line_height();
            i += 1;
            continue;
        }
        if char_advance_estimate(ch) == 0.0 {
            i += 1;
            continue;
        }
        let gap_y =
            style.auto_spacing && prev.map(|p| is_ja_latin_boundary(p, ch)).unwrap_or(false);
        if gap_y {
            y += ja_latin_gap_em(font_size, style.auto_spacing_em as f64);
        }
        // Ruby group opening here: remember where it starts.
        if let Some(ann) = anns.iter().find(|a| a.start == i) {
            active = Some((y, ann.len, ann));
        }
        // 縦中横: 2–3 digits share one em cell, set horizontally.
        if let Some(n) = tatechuyoko_run(&base_chars, i) {
            let unit: String = base_chars[i..i + n].iter().collect();
            let adv = font_size + letter_spacing;
            if let Some(mut ol) = try_text_to_outline_path_with_style(&unit, &horiz) {
                let scale = tatechuyoko_scale(n);
                if let Some((mn, mx)) = ol.bounding_box() {
                    let w = (mx.x - mn.x) * scale;
                    let h = (mx.y - mn.y) * scale;
                    let tx = (font_size - w) / 2.0 - mn.x * scale;
                    let ty = (y + (adv - h) / 2.0) - mn.y * scale;
                    ol.transform(&[scale, 0.0, 0.0, scale, tx, ty]);
                }
                combined.elements.extend(ol.elements);
                produced = true;
            }
            y += adv;
            prev = Some(base_chars[i + n - 1]);
            i += n;
        } else {
            let adv = char_advance_estimate(ch) * font_size + letter_spacing;
            if let Some(mut ol) = try_text_to_outline_path_with_style(&ch.to_string(), &horiz) {
                if is_fullwidth_char(ch) {
                    // Upright: centre its ink in the cell [0, adv] × [y, y+adv].
                    if let Some((mn, mx)) = ol.bounding_box() {
                        let tx = (adv - (mx.x - mn.x)) / 2.0 - mn.x;
                        let ty = (y + (adv - (mx.y - mn.y)) / 2.0) - mn.y;
                        ol.transform(&[1.0, 0.0, 0.0, 1.0, tx, ty]);
                    }
                } else {
                    // Rotate 90° CW (right) about the origin, then centre the
                    // rotated ink in the column.
                    let mut rot = ol;
                    rot.transform(&[0.0, 1.0, -1.0, 0.0, 0.0, 0.0]);
                    if let Some((mn, mx)) = rot.bounding_box() {
                        let tx = (font_size - (mx.x - mn.x)) / 2.0 - mn.x;
                        let ty = (y + (adv - (mx.y - mn.y)) / 2.0) - mn.y;
                        rot.transform(&[1.0, 0.0, 0.0, 1.0, tx, ty]);
                    }
                    ol = rot;
                }
                combined.elements.extend(ol.elements);
                produced = true;
            }
            y += adv;
            prev = Some(ch);
            i += 1;
        }
        // Ruby group closing here: set the reading to the right of the
        // base column, centred on the group's vertical extent.
        if let Some((start_y, left, ann)) = active {
            if left <= 1 {
                active = None;
                if let Some(mut ol) = try_text_to_outline_path_with_style(&ann.reading, &horiz) {
                    if let Some((mn, mx)) = ol.bounding_box() {
                        let w = (mx.x - mn.x) * RUBY_SCALE;
                        let h = (mx.y - mn.y) * RUBY_SCALE;
                        let cy = (start_y + y) / 2.0;
                        let tx = font_size * RUBY_STRIP_CENTER_EM - w / 2.0 - mn.x * RUBY_SCALE;
                        let ty = cy - h / 2.0 - mn.y * RUBY_SCALE;
                        ol.transform(&[RUBY_SCALE, 0.0, 0.0, RUBY_SCALE, tx, ty]);
                        combined.elements.extend(ol.elements);
                        produced = true;
                    }
                }
            } else {
                active = Some((start_y, left - 1, ann));
            }
        }
    }
    if produced {
        Some(combined)
    } else {
        None
    }
}

/// Greedy longest-match GSUB ligature substitution over a glyph-id run.
/// Walks `liga`/`dlig`/`clig`/`rlig` lookups from the face's GSUB table
/// (DFLT/latn scripts). Glyph id 0 is a space sentinel and never matches.
fn apply_liga_substitutions(
    face: &ttf_parser::Face<'_>,
    glyphs: &mut Vec<ttf_parser::GlyphId>,
    gap_before: &mut Vec<bool>,
) {
    use ttf_parser::gsub::SubstitutionSubtable;

    let Some(gsub) = face.tables().gsub.as_ref() else {
        return;
    };

    let wanted = [
        ttf_parser::Tag::from_bytes(b"liga"),
        ttf_parser::Tag::from_bytes(b"dlig"),
    ];
    let mut lig_subs: Vec<ttf_parser::gsub::LigatureSubstitution<'_>> = Vec::new();

    let mut feature_indices: Vec<u16> = Vec::new();
    for script_tag in [
        ttf_parser::Tag::from_bytes(b"latn"),
        ttf_parser::Tag::from_bytes(b"DFLT"),
    ] {
        let Some(srec) = gsub.scripts.find(script_tag) else {
            continue;
        };
        let ls = srec
            .default_language
            .or_else(|| srec.languages.into_iter().next());
        if let Some(ls) = ls {
            feature_indices = ls.feature_indices.into_iter().collect();
            if !feature_indices.is_empty() {
                break;
            }
        }
    }
    for tag in wanted {
        if let Some(idx) = gsub.features.index(tag) {
            if !feature_indices.contains(&idx) {
                feature_indices.push(idx);
            }
        }
    }

    for fi in feature_indices {
        let Some(feat) = gsub.features.get(fi) else {
            continue;
        };
        let is_liga = wanted.iter().any(|t| gsub.features.index(*t) == Some(fi));
        if !is_liga {
            continue;
        }
        for li in feat.lookup_indices {
            let Some(lookup) = gsub.lookups.get(li) else {
                continue;
            };
            for sub in lookup.subtables.into_iter::<SubstitutionSubtable>() {
                if let SubstitutionSubtable::Ligature(lsub) = sub {
                    lig_subs.push(lsub);
                }
            }
        }
    }

    if lig_subs.is_empty() {
        return;
    }

    // Greedy left-to-right longest match.
    let mut i = 0usize;
    while i < glyphs.len() {
        if glyphs[i].0 == 0 {
            i += 1;
            continue;
        }
        let mut best: Option<(usize, ttf_parser::GlyphId)> = None; // (component_count_including_first, lig)
        'outer: for lsub in &lig_subs {
            if !lsub.coverage.contains(glyphs[i]) {
                continue;
            }
            let Some(cov_idx) = lsub.coverage.get(glyphs[i]) else {
                continue;
            };
            let Some(lset) = lsub.ligature_sets.get(cov_idx) else {
                continue;
            };
            for lig in lset {
                let comps = lig.components;
                let total = 1 + comps.len() as usize;
                if i + total > glyphs.len() {
                    continue;
                }
                let mut ok = true;
                for (k, c) in comps.into_iter().enumerate() {
                    if glyphs[i + 1 + k] != c {
                        ok = false;
                        break;
                    }
                }
                if ok && best.map(|(n, _)| total > n).unwrap_or(true) {
                    best = Some((total, lig.glyph));
                    if total == 4 {
                        break 'outer; // can't get longer than 4 for typical liga
                    }
                }
            }
        }
        if let Some((total, lig_gid)) = best {
            glyphs[i] = lig_gid;
            glyphs.drain(i + 1..i + total);
            // A 和欧 gap belongs to the merged ligature only if it was
            // the left edge of the merge — keep the first flag.
            gap_before.drain(i + 1..i + total);
            i += 1;
        } else {
            i += 1;
        }
    }
}

/// Mock block-glyph fallback used when no font face resolves (keeps
/// *something* visible instead of dropping the text silently).
fn mock_text_outline(text: &str, style: &crate::core::document::TextStyle) -> PathData {
    let normalized = crate::core::document::normalize_text(text);
    let text = normalized.as_ref();
    let font_size = style.effective_font_size();
    let letter_spacing = style.effective_letter_spacing();
    let faux_italic = !matches!(style.font_style, crate::core::document::FontStyle::Normal)
        && super::font::FontRegistry::global().needs_synthetic_style(
            &style.font_family,
            style.font_weight,
            style.font_style,
        );
    // Fallback: mock glyph outline generator
    let mut combined = PathData::new();
    let char_width = font_size * 0.6;
    let mut current_x = 0.0;

    for ch in text.chars() {
        if ch == ' ' {
            current_x += char_width * 0.7 + letter_spacing;
            continue;
        }

        let mut glyph = get_glyph_outline_path(ch);
        let matrix = [char_width, 0.0, 0.0, font_size, current_x, -font_size];
        glyph.transform(&matrix);
        combined.elements.extend(glyph.elements);
        current_x += char_width * 1.1 + letter_spacing;
    }

    if faux_italic {
        combined.transform(&[1.0, 0.0, -FAUX_ITALIC_SHEAR, 1.0, 0.0, 0.0]);
    }

    combined
}

/// Convert a text string into an outlined vector PathData with default style.
pub fn text_to_outline_path(text: &str, font_size: f64) -> PathData {
    let style = crate::core::document::TextStyle::new("Inter, sans-serif", font_size);
    text_to_outline_path_with_style(text, &style)
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
        let cos = p.rotation_rad.cos();
        let sin = p.rotation_rad.sin();
        let sx = char_width;
        let sy = font_size;

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
    if let crate::core::document::ObjectType::Text { text, style, .. } = &text_obj.object_type {
        let mut path = text_to_outline_path_with_style(text, style);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harfbuzz_shapes_with_font_unit_advances() {
        let data = include_bytes!("../../assets/fonts/Inter.ttf");
        let shaped = shape_run_hb(data, 0, "fi", &[], &[]).expect("shapes");
        // Every glyph carries a positive advance in font units.
        assert!(!shaped.is_empty());
        for g in &shaped {
            assert!(g.x_advance > 0.0, "{g:?}");
        }
        let width: f32 = shaped.iter().map(|g| g.x_advance).sum();
        assert!(width > 1000.0, "two glyphs at ~1k upem: {width}");
    }

    #[test]
    fn harfbuzz_variable_instance_changes_advances() {
        // Bundled Inter is variable: wght=700 must shape wider than the
        // default master, proving axis coordinates reach the shaper.
        use crate::core::document::VariationSetting;
        let data = include_bytes!("../../assets/fonts/Inter.ttf");
        let plain = shape_run_hb(data, 0, "AV", &[], &[]).expect("shapes");
        let vars = vec![VariationSetting::new("wght", 700.0)];
        let bold = shape_run_hb(data, 0, "AV", &[], &vars).expect("shapes");
        assert_eq!(plain.len(), bold.len());
        let w_plain: f32 = plain.iter().map(|g| g.x_advance).sum();
        let w_bold: f32 = bold.iter().map(|g| g.x_advance).sum();
        assert!(w_bold > w_plain, "bold wider: {w_bold} vs {w_plain}");
    }

    #[test]
    fn variable_instance_outlines_differ_end_to_end() {
        // wght=700 outlines must differ from the default master: proves
        // instance coordinates reach shaping AND outline extraction
        // (embed keeps the outlines fallback for varied text, which is
        // the only correct output without a full gvar instancer).
        use crate::core::document::{TextStyle, VariationSetting};
        let plain = TextStyle {
            font_family: "Inter".to_string(),
            font_size: 40.0,
            ..Default::default()
        };
        let mut bold = plain.clone();
        bold.variations = vec![VariationSetting::new("wght", 700.0)];
        let words = |style: &TextStyle| text_to_outline_path_with_style("AV", style);
        let a = words(&plain);
        let b = words(&bold);
        let w = |p: &PathData| p.bounding_box().map(|(mn, mx)| mx.x - mn.x).unwrap_or(0.0);
        assert!(w(&b) > w(&a), "bold wider: {} vs {}", w(&b), w(&a));
    }

    #[test]
    fn harfbuzz_gpos_kern_differs_from_flat_advances() {
        // Inter kerns AV via GPOS (no legacy `kern` row for it): the shaped
        // advance of A must be tighter than its nominal advance.
        let data = include_bytes!("../../assets/fonts/Inter.ttf");
        let face = ttf_parser::Face::parse(data, 0).unwrap();
        let a = face.glyph_index('A').unwrap();
        let nominal = face.glyph_hor_advance(a).unwrap() as f32;
        let shaped = shape_run_hb(data, 0, "AV", &[], &[]).expect("shapes");
        assert_eq!(shaped.len(), 2);
        assert!(
            shaped[0].x_advance < nominal,
            "GPOS kern tightens A: {} vs {nominal}",
            shaped[0].x_advance
        );
    }

    #[test]
    fn non_finite_variation_coords_never_reach_advances() {
        // A NaN wght (hand-edited file) must not poison shaped advances:
        // it is dropped, so shaping matches the default master exactly.
        use crate::core::document::VariationSetting;
        let data = include_bytes!("../../assets/fonts/Inter.ttf");
        let plain = shape_run_hb(data, 0, "AV", &[], &[]).expect("shapes");
        let vars = vec![VariationSetting::new("wght", f64::NAN)];
        let shaped = shape_run_hb(data, 0, "AV", &[], &vars).expect("shapes");
        assert_eq!(shaped.len(), plain.len());
        for (g, p) in shaped.iter().zip(plain.iter()) {
            assert!(g.x_advance.is_finite(), "{g:?}");
            assert!((g.x_advance - p.x_advance).abs() < 1e-6);
        }
    }

    #[test]
    fn fallback_survives_vs16_tab_and_crlf() {
        // Manual fallback used to tofu the WHOLE run when a single char had
        // no cmap entry: VS16/ZWJ after a base char, tabs, and stray CRs.
        use crate::core::document::TextStyle;
        let style = TextStyle::new("Inter", 40.0);
        for text in ["A\u{FE0F}B", "A\tB", "A\u{200D}B"] {
            let path = text_to_outline_path_with_style(text, &style);
            let (mn, mx) = path.bounding_box().expect("real outlines, not tofu");
            assert!(mx.x > mn.x, "{text:?} outlined");
            assert!(mx.x.is_finite() && mn.x.is_finite(), "{text:?} finite bbox");
        }
        // CRLF and LF outline identically (no stray CR block glyph).
        let crlf = text_to_outline_path_with_style("A\r\nB", &style);
        let lf = text_to_outline_path_with_style("A\nB", &style);
        assert_eq!(
            crlf.elements.len(),
            lf.elements.len(),
            "CRLF normalizes before shaping"
        );
    }

    #[test]
    fn text_on_path_guards_nan_and_newlines() {
        use crate::core::path::PathData;
        let mut line = PathData::new();
        line.push_move_to(0.0, 0.0);
        line.push_line_to(500.0, 0.0);
        assert!(place_text_along_path(&line, "AB", f64::NAN, 0.0).is_empty());
        assert!(place_text_along_path(&line, "AB", 24.0, f64::NAN).is_empty());
        // Newlines are skipped, not drawn as tofu blocks.
        let placements = place_text_along_path(&line, "A\nB\rC", 24.0, 0.0);
        assert_eq!(placements.len(), 3);
        assert!(placements.iter().all(|p| p.char_value != '\n'));
        let styled = place_text_along_path_styled(&line, "AB", 24.0, f64::NAN, 0.0);
        assert_eq!(styled.len(), 2, "NaN spacing degrades to 0, not NaN");
        assert!(styled.iter().all(|p| p.position.x.is_finite()));
    }
}
