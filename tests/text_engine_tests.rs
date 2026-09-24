//! Text engine: kinsoku wrapping, width estimates, faux italic, kerning.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{
    char_advance_estimate, compute_wrapped_lines, layout_text, Object, TextArea, TextStyle,
};
use irasu_illustrator::core::font::FontRegistry;

fn style(size: f64, max_w: f64) -> TextStyle {
    TextStyle {
        font_family: "sans-serif".to_string(),
        font_size: size,
        font_weight: 400,
        font_style: irasu_illustrator::core::document::FontStyle::Normal,
        letter_spacing: 0.0,
        text_anchor: irasu_illustrator::core::document::TextAnchor::Start,
        line_height: None,
        max_width: Some(max_w),
        word_wrap: true,
        variations: Vec::new(),
        vertical: false,
        ligatures: true,
    }
}

fn first_chars(lines: &[String]) -> Vec<char> {
    lines.iter().filter_map(|l| l.chars().next()).collect()
}

fn last_chars(lines: &[String]) -> Vec<char> {
    lines.iter().filter_map(|l| l.chars().last()).collect()
}

#[test]
fn test_fullwidth_advances_full_em() {
    assert!((char_advance_estimate('あ') - 1.0).abs() < 1e-9);
    assert!((char_advance_estimate('ア') - 1.0).abs() < 1e-9);
    assert!((char_advance_estimate('漢') - 1.0).abs() < 1e-9);
    assert!((char_advance_estimate('한') - 1.0).abs() < 1e-9);
    assert!((char_advance_estimate('A') - 0.6).abs() < 1e-9);
    assert!((char_advance_estimate('ｱ') - 0.6).abs() < 1e-9, "halfwidth kana stays narrow");
}

#[test]
fn test_kinsoku_no_line_starts_with_closing_mark() {
    // 10px font: CJK = 10px each. Width 25px fits 2 chars; without kinsoku
    // the 、 would start line 2.
    let lines = compute_wrapped_lines("あいう、えお", &style(10.0, 25.0), 25.0);
    assert!(!lines.is_empty());
    for (i, l) in lines.iter().enumerate() {
        let first = l.chars().next().unwrap_or(' ');
        assert!(
            first != '、' && first != '。',
            "line {i} starts with a closing mark: {lines:?}"
        );
    }
    assert_eq!(lines.concat(), "あいう、えお", "no text lost");
}

#[test]
fn test_kinsoku_no_line_ends_with_opening_bracket() {
    let lines = compute_wrapped_lines("あいう「えお", &style(10.0, 35.0), 35.0);
    for (i, l) in lines.iter().enumerate() {
        let last = l.chars().last().unwrap_or(' ');
        assert!(last != '「', "line {i} ends with an opener: {lines:?}");
    }
    assert_eq!(lines.concat(), "あいう「えお");
}

#[test]
fn test_kinsoku_prolonged_mark_never_starts_line() {
    // "アー" must stay glued even at 1-char width.
    let lines = compute_wrapped_lines("アーカイブ", &style(10.0, 10.0), 10.0);
    let starts = first_chars(&lines);
    assert!(!starts.contains(&'ー'), "lines: {lines:?}");
    assert_eq!(lines.concat(), "アーカイブ");
}

#[test]
fn test_kinsoku_small_kana_never_start_line() {
    let lines = compute_wrapped_lines("きゃっきゃ", &style(10.0, 20.0), 20.0);
    for c in first_chars(&lines) {
        assert!(c != 'ゃ' && c != 'っ', "lines: {lines:?}");
    }
    assert_eq!(lines.concat(), "きゃっきゃ");
}

#[test]
fn test_wrap_respects_letter_spacing() {
    let mut st = style(10.0, 100.0);
    st.letter_spacing = 0.0;
    let plain = compute_wrapped_lines("AAAAAA", &st, 100.0);
    assert_eq!(plain.len(), 1, "6 latin chars fit without spacing");
    st.letter_spacing = 10.0;
    let spaced = compute_wrapped_lines("AAAAAAA", &st, 100.0);
    assert!(spaced.len() > 1, "spacing forces a break: {spaced:?}");
    assert_eq!(spaced.concat().replace(' ', ""), "AAAAAAA");
}

#[test]
fn test_wrap_terminates_on_degenerate_width() {
    let lines = compute_wrapped_lines("あいうえお", &style(10.0, 0.5), 0.5);
    assert_eq!(lines.concat(), "あいうえお");
    assert!(lines.len() <= 5, "at most one char per line: {lines:?}");
}

#[test]
fn test_needs_synthetic_style_logic() {
    let reg = FontRegistry::global();
    // Normal never synthesizes.
    assert!(!reg.needs_synthetic_style(
        "NoSuchFamily-XYZ",
        400,
        irasu_illustrator::core::document::FontStyle::Normal
    ));
    // A missing family has no italic face by definition.
    assert!(reg.needs_synthetic_style(
        "NoSuchFamily-XYZ",
        400,
        irasu_illustrator::core::document::FontStyle::Italic
    ));
    assert!(reg.needs_synthetic_style(
        "NoSuchFamily-XYZ",
        400,
        irasu_illustrator::core::document::FontStyle::Oblique
    ));
    // Bundled Inter ships upright-only: italic must synthesize.
    assert!(reg.needs_synthetic_style(
        "Inter",
        400,
        irasu_illustrator::core::document::FontStyle::Italic
    ));
    // A family with a real slanted face needs no synthesis (when present).
    if reg.is_family_available("Helvetica") {
        assert!(!reg.needs_synthetic_style(
            "Helvetica",
            400,
            irasu_illustrator::core::document::FontStyle::Italic
        ));
    }
}

#[test]
fn test_faux_italic_shears_outlines() {
    // Deterministic oblique for the logo workflow: the bundled Inter ships
    // upright-only, so outlining an italic run must shear it (ascender tops
    // lean right). Shear is a slant, not a translation: the left edge must
    // not jump.
    use irasu_illustrator::core::document::FontStyle as FS;
    let upright = TextStyle::new("Inter", 100.0);
    let mut slanted = TextStyle::new("Inter", 100.0);
    slanted.font_style = FS::Italic;
    let to_bb = |st: &TextStyle| {
        irasu_illustrator::core::text_path::text_to_outline_path_with_style("Ag", st)
            .bounding_box()
            .expect("outline bbox")
    };
    let (mn_u, mx_u) = to_bb(&upright);
    let (mn_i, mx_i) = to_bb(&slanted);
    assert!(
        mx_i.x > mx_u.x + 5.0,
        "ascender tops lean right: {} -> {}",
        mx_u.x,
        mx_i.x
    );
    assert!(
        (mn_i.x - mn_u.x).abs() < 6.0,
        "left edge stays put: {} -> {}",
        mn_u.x,
        mn_i.x
    );
}

#[test]
fn test_normal_text_has_no_skew() {
    use irasu_illustrator::core::document::Document;
    let mut doc = Document::default();
    doc.add_object(Object::new_text("T", "plain", 0.0, 0.0, 40.0));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(!svg.contains("skewX"), "upright text must not skew");
}

#[test]
fn test_kerning_applies_between_glyphs() {
    // Self-adapting: find the strongest kerned uppercase pair in the
    // resolved sans face and verify the outline honors it exactly.
    // Derivation: the pair's right ink edge = adv(A) + kern + right(V),
    // so kern = x1(pair) − x1(V alone) − adv(A). Bearings cancel out.
    // Skips gracefully on kern-less setups.
    let reg = FontRegistry::global();
    let db = reg.database();
    let id = match db.query(&fontdb::Query {
        families: &[fontdb::Family::SansSerif],
        weight: fontdb::Weight(400),
        style: fontdb::Style::Normal,
        stretch: fontdb::Stretch::Normal,
    }) {
        Some(id) => id,
        None => return,
    };
    let (path, index) = match db.face_source(id) {
        Some((fontdb::Source::File(p), i)) => (p, i),
        _ => return,
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => return,
    };
    let face = match ttf_parser::Face::parse(&data, index) {
        Ok(f) => f,
        Err(_) => return,
    };
    let kern = match face.tables().kern {
        Some(k) => k,
        None => return,
    };
    let upm = face.units_per_em() as f64;
    let mut best: Option<(char, char, f64)> = None;
    for a in b'A'..=b'Z' {
        for b in b'A'..=b'Z' {
            let (ga, gb) = (face.glyph_index(a as char), face.glyph_index(b as char));
            if let (Some(ga), Some(gb)) = (ga, gb) {
                for sub in kern.subtables {
                    if !sub.horizontal {
                        continue;
                    }
                    if let Some(k) = sub.glyphs_kerning(ga, gb) {
                        let em = k as f64 / upm;
                        if best.map(|(_, _, e)| em.abs() > e.abs()).unwrap_or(true) {
                            best = Some((a as char, b as char, em));
                        }
                    }
                }
            }
        }
    }
    let (ca, cb, kern_em) = match best {
        Some(p) if p.2.abs() > 1e-9 => p,
        _ => return,
    };
    let family = db
        .face(id)
        .and_then(|f| f.families.first().map(|(n, _)| n.clone()))
        .unwrap_or_else(|| "sans-serif".to_string());
    let mut st = TextStyle::new(family, 100.0);
    st.font_style = irasu_illustrator::core::document::FontStyle::Normal;
    let outline = |s: &str| irasu_illustrator::core::text_path::text_to_outline_path_with_style(s, &st);
    let x1 = |s: &str| outline(s).bounding_box().map(|(_, mx)| mx.x);
    let pair_str: String = [ca, cb].iter().collect();
    let (x1_pair, x1_b, x0_pair, x0_a) = match (
        x1(&pair_str),
        x1(&cb.to_string()),
        outline(&pair_str).bounding_box(),
        outline(&ca.to_string()).bounding_box(),
    ) {
        (Some(a), Some(b), Some((mn_p, _)), Some((mn_a, _))) => (a, b, mn_p.x, mn_a.x),
        _ => return,
    };
    // Left ink edges coincide (pen starts at 0 in both).
    assert!((x0_pair - x0_a).abs() < 1.0, "left edges align");
    let adv_a = face
        .glyph_index(ca)
        .and_then(|g| face.glyph_hor_advance(g))
        .unwrap_or(face.units_per_em()) as f64
        / upm
        * 100.0;
    let kern_got = x1_pair - x1_b - adv_a;
    let kern_want = kern_em * 100.0;
    assert!(
        (kern_got - kern_want).abs() < 2.0,
        "pair {pair_str}: kern {kern_got:.1} != table {kern_want:.1}"
    );
}

#[test]
fn test_pixel_snap_rounds_to_integers() {
    use irasu_illustrator::core::state::AppState;
    let mut state = AppState::default();
    state.snap_to_grid = false;
    state.snap_to_objects = false;
    state.snap_to_guides = false;
    state.snap_to_points = false;
    assert_eq!(state.snap(10.4, 10.6), (10.4, 10.6));
    state.snap_to_pixels = true;
    assert_eq!(state.snap(10.4, 10.6), (10.0, 11.0));
}

#[test]
fn test_last_chars_helper_sanity() {
    // Guards the test helper itself against silent rot.
    let lines = vec!["あい".to_string(), "う".to_string()];
    assert_eq!(last_chars(&lines), vec!['い', 'う']);
}

#[test]
fn test_area_layout_wraps_and_overflows() {
    let st = style(10.0, 1000.0);
    let area = TextArea::new(5.0, 7.0, 25.0, 30.0);
    let layout = layout_text("あいうえお", &st, Some(area));
    // 10px CJK chars, 25px box: 2 per line -> 3 lines; baselines at 17,
    // 29, 41 against a box bottom of 37 -> 2 visible.
    assert_eq!(layout.lines.len(), 3, "wraps to box width");
    assert_eq!(layout.visible, 2, "third line overflows");
    assert_eq!(layout.overflow(), 1);
    assert_eq!(layout.origin, (5.0, 17.0), "first baseline at em-box top");
    assert_eq!(layout.lines.concat(), "あいうえお", "no text lost");
}

#[test]
fn test_area_tiny_box_hides_all() {
    let st = style(10.0, 1000.0);
    let layout = layout_text("あ", &st, Some(TextArea::new(0.0, 0.0, 50.0, 5.0)));
    assert_eq!(layout.visible, 0);
    assert_eq!(layout.overflow(), 1);
}

#[test]
fn test_point_layout_is_passthrough() {
    let st = style(10.0, 1000.0);
    let layout = layout_text("a\nb", &st, None);
    assert_eq!(layout.origin, (0.0, 0.0));
    assert_eq!(layout.visible, 2);
    assert_eq!(layout.overflow(), 0);
}

#[test]
fn test_area_export_clips_and_round_trips() {
    use irasu_illustrator::core::document::Document;
    use irasu_illustrator::io::svg::{export_svg, parse_svg_document};
    let mut doc = Document::default();
    doc.width = 400.0;
    doc.height = 300.0;
    let mut obj = Object::new_text("T", "あいうえお", 10.0, 20.0, 10.0);
    if let irasu_illustrator::core::document::ObjectType::Text { area, .. } =
        &mut obj.object_type
    {
        *area = Some(TextArea::new(0.0, 0.0, 25.0, 30.0));
    }
    doc.add_object(obj);
    let svg = export_svg(&doc);
    assert!(svg.contains("data-text-area=\"0 0 25 30\""), "box preserved");
    assert!(svg.contains("<clipPath"), "overflow is clipped, not dropped");
    // Position composes: object at (10,20), box at local (0,0).
    assert!(svg.contains("x=\"10\""), "text x includes object offset");
    let doc2 = parse_svg_document(&svg);
    let back = doc2
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| matches!(o.object_type, irasu_illustrator::core::document::ObjectType::Text { .. }))
        .expect("text reimports");
    if let irasu_illustrator::core::document::ObjectType::Text { area, text, .. } =
        &back.object_type
    {
        assert_eq!(text, "あい\nうえ\nお", "lines survive as breaks, overflow kept");
        let a = area.expect("area round-trips");
        assert_eq!((a.x, a.y, a.width, a.height), (0.0, 0.0, 25.0, 30.0));
    } else {
        unreachable!();
    }
}

#[test]
fn test_area_diff_detected() {
    use irasu_illustrator::core::document::Document;
    let mut a = Document::default();
    let mut b = Document::default();
    a.add_object(Object::new_text("T", "hi", 0.0, 0.0, 12.0));
    let mut obj = Object::new_text("T", "hi", 0.0, 0.0, 12.0);
    if let irasu_illustrator::core::document::ObjectType::Text { area, .. } =
        &mut obj.object_type
    {
        *area = Some(TextArea::new(0.0, 0.0, 100.0, 50.0));
    }
    b.add_object(obj);
    let diff = irasu_illustrator::core::diff::compute_semantic_diff(&a, &b);
    let changed: Vec<_> = diff
        .objects
        .iter()
        .filter(|d| matches!(d.status, irasu_illustrator::core::diff::ObjectDiffStatus::Modified { .. }))
        .collect();
    assert_eq!(changed.len(), 1, "boxing the text is a modification");
}
