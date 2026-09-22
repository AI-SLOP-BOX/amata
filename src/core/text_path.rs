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
    try_text_to_outline_path_with_style(text, style)
        .unwrap_or_else(|| mock_text_outline(text, style))
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
    let font_size = style.font_size;
    let letter_spacing = style.letter_spacing;
    let registry = super::font::FontRegistry::global();
    // Faux italic only when no real italic/oblique face exists; otherwise
    // the resolved face already slants and shearing would double it.
    let faux_italic = !matches!(
        style.font_style,
        crate::core::document::FontStyle::Normal
    ) && registry.needs_synthetic_style(&style.font_family, style.font_weight, style.font_style);

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
                    let mut combined = PathData::new();
                    let mut current_x = 0.0;
                    // `kern` (old-style TrueType kerning) table, when present.
                    let kern_table = face.tables().kern.clone();
                    let mut prev_glyph: Option<ttf_parser::GlyphId> = None;

                    // Build a glyph-id run first so GSUB ligatures can collapse
                    // sequences before outlining.
                    let mut glyphs: Vec<ttf_parser::GlyphId> = Vec::new();
                    for ch in text.chars() {
                        if ch == ' ' {
                            // Spaces break ligature runs and advance a fixed amount.
                            glyphs.push(ttf_parser::GlyphId(0)); // sentinel for space
                            continue;
                        }
                        match face.glyph_index(ch) {
                            Some(g) => glyphs.push(g),
                            None => return None,
                        }
                    }

                    if style.ligatures {
                        apply_liga_substitutions(&face, &mut glyphs);
                    }

                    for gid in glyphs {
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

                        let adv = face
                            .glyph_hor_advance(gid)
                            .unwrap_or(face.units_per_em())
                            as f64
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

/// Greedy longest-match GSUB ligature substitution over a glyph-id run.
/// Walks `liga`/`dlig`/`clig`/`rlig` lookups from the face's GSUB table
/// (DFLT/latn scripts). Glyph id 0 is a space sentinel and never matches.
fn apply_liga_substitutions(face: &ttf_parser::Face<'_>, glyphs: &mut Vec<ttf_parser::GlyphId>) {
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
            i += 1;
        } else {
            i += 1;
        }
    }
}

/// Mock block-glyph fallback used when no font face resolves (keeps
/// *something* visible instead of dropping the text silently).
fn mock_text_outline(text: &str, style: &crate::core::document::TextStyle) -> PathData {
    let font_size = style.font_size;
    let letter_spacing = style.letter_spacing;
    let faux_italic = !matches!(
        style.font_style,
        crate::core::document::FontStyle::Normal
    ) && super::font::FontRegistry::global().needs_synthetic_style(
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
