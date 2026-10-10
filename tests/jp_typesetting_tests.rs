//! 日本語組版の3機能テスト:
//! 1) 縦組グリフ回転 (canvas/svg/pdf 共通のアウトライン基盤)
//! 2) 絵文字 per-glyph フォールバック
//! 3) 和欧間スペース自動挿入
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{
    burasage_hang, has_ja_latin_boundary, is_fullwidth_char, is_ja_latin_boundary,
    is_vert_rotated_char, ja_latin_gap, ja_latin_gap_em, layout_text, parse_ruby,
    split_ja_latin_segments, tatechuyoko_run, tatechuyoko_scale, text_advance_estimate, Object,
    TextStyle,
};
use irasu_illustrator::core::text_path::try_text_to_outline_path_with_style;

fn style(size: f64) -> TextStyle {
    TextStyle::new("Inter", size)
}

fn bbox_of(p: &irasu_illustrator::core::path::PathData) -> (f64, f64) {
    let (mn, mx) = p.bounding_box().unwrap();
    (mx.x - mn.x, mx.y - mn.y)
}

#[test]
fn auto_spacing_boundary_detection() {
    assert!(is_ja_latin_boundary('あ', 'a'));
    assert!(is_ja_latin_boundary('漢', 'Z'));
    assert!(is_ja_latin_boundary('A', '。'));
    assert!(!is_ja_latin_boundary('あ', 'い'));
    assert!(!is_ja_latin_boundary('a', 'b'));
    assert!(has_ja_latin_boundary("あA"));
    assert!(!has_ja_latin_boundary("あいう"));
    assert!(!has_ja_latin_boundary("ABC"));
}

#[test]
fn auto_spacing_advance_adds_quarter_em() {
    let st = style(24.0);
    let plain_a = text_advance_estimate("あ", &st);
    let mixed = text_advance_estimate("あA", &st);
    let sep = text_advance_estimate("A", &st);
    assert!(
        (mixed - (plain_a + sep) - ja_latin_gap(24.0)).abs() < 1e-6,
        "expected exactly 1/4em gap: {mixed} vs {} + {}",
        plain_a + sep,
        ja_latin_gap(24.0)
    );
    // Gap can be toggled off.
    let mut off = style(24.0);
    off.auto_spacing = false;
    assert!((text_advance_estimate("あA", &off) - (plain_a + sep)).abs() < 1e-6);
}

#[test]
fn split_segments_marks_gap_before_latin() {
    let st = style(10.0);
    let segs = split_ja_latin_segments("ああAあ", &st);
    assert_eq!(segs.len(), 3);
    assert_eq!(segs[0].0, "ああ");
    assert!(!segs[0].1);
    assert_eq!(segs[1].0, "A");
    assert!(segs[1].1);
    assert_eq!(segs[2].0, "あ");
    assert!(segs[2].1);
}

#[test]
fn outline_width_includes_auto_gap() {
    let st = style(24.0);
    let full = try_text_to_outline_path_with_style("あA", &st).expect("outline");
    let cjk = try_text_to_outline_path_with_style("あ", &st).expect("outline");
    let lat = try_text_to_outline_path_with_style("A", &st).expect("outline");
    let (fw, _) = bbox_of(&full);
    let (cw, _) = bbox_of(&cjk);
    let (lw, _) = bbox_of(&lat);
    assert!(
        fw >= cw + lw + 0.15 * 24.0,
        "outline of あA should carry the boundary gap: {fw} vs {cw}+{lw}"
    );
}

#[test]
fn emoji_run_keeps_primary_face_and_fallbacks() {
    let st = style(24.0);
    // Inter has no emoji: previously the whole run degraded to mock blocks.
    // Now "AB" must keep real outlines and only the emoji falls back.
    let mixed = try_text_to_outline_path_with_style("AB😀", &st);
    let plain = try_text_to_outline_path_with_style("AB", &st);
    let (mixed, plain) = match (mixed, plain) {
        (Some(m), Some(p)) => (m, p),
        _ => panic!("fallback should produce outlines"),
    };
    assert!(
        !mixed.elements.is_empty(),
        "primary-face part of the run must keep outlines"
    );
    assert!(
        mixed.elements.len() >= plain.elements.len(),
        "AB outlines preserved (fallback adds, never replaces): {} vs {}",
        mixed.elements.len(),
        plain.elements.len()
    );
}

#[test]
fn vertical_outline_turns_horizontal_runs_on_their_side() {
    let mut st = style(24.0);
    st.vertical = true;
    let vert = try_text_to_outline_path_with_style("AAB", &st).expect("vertical outline");
    let (vw, vh) = bbox_of(&vert);
    let horiz = style(24.0);
    let hor = try_text_to_outline_path_with_style("AAB", &horiz).expect("outline");
    let (hw, hh) = bbox_of(&hor);
    // Vertical "AAB" is taller than it is wide; horizontal is wider.
    assert!(
        vh > vw * 1.5,
        "vertical text should stack glyphs down Y: {vw}x{vh}"
    );
    assert!(hw > hh, "horizontal text spans X: {hw}x{hh}");
}

#[test]
fn vertical_svg_emits_rotated_halfwidth_glyphs() {
    let mut doc = irasu_illustrator::core::document::Document::default();
    let mut st = style(24.0);
    st.vertical = true;
    doc.add_object(Object::new_text_with_style("T", "Aあ", 10.0, 10.0, st));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(
        svg.contains("data-text-vertical=\"1\""),
        "vertical flag round-trips: {svg}"
    );
    assert!(
        svg.contains("rotate(90"),
        "halfwidth glyph is rotated 90° CW in the export: {svg}"
    );
    assert!(
        !svg.contains("writing-mode="),
        "no double-rotation from CSS writing mode: {svg}"
    );
}

#[test]
fn horizontal_svg_injects_dx_gap_at_ja_latin_boundary() {
    let mut doc = irasu_illustrator::core::document::Document::default();
    doc.add_object(Object::new_text_with_style(
        "T",
        "あA",
        10.0,
        10.0,
        style(10.0),
    ));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(
        svg.contains("dx=\"2.5\""),
        "1/4em (=2.5) gap emitted as dx: {svg}"
    );
}

#[test]
fn mixed_auto_spacing_round_trips_through_layout() {
    // Wrapping measures the injected gap: "あ" (1em) + gap (0.25em) + "A"
    // (0.6em) = 1.85em > 1.8em, so the pair at 10pt wraps after あ.
    let st = style(10.0);
    let mut capped = st.clone();
    capped.word_wrap = true;
    capped.max_width = Some(18.0);
    let lines = irasu_illustrator::core::document::layout_text("あA", &capped, None);
    assert_eq!(
        lines.lines,
        vec!["あ".to_string(), "A".to_string()]
            .as_slice()
            .join("\n")
            .split('\n')
            .map(String::from)
            .collect::<Vec<_>>(),
        "the boundary gap must count toward the wrap width"
    );
}

#[test]
fn fullwidth_detection_covers_kana_and_emoji() {
    assert!(is_fullwidth_char('あ'));
    assert!(is_fullwidth_char('ア'));
    assert!(is_fullwidth_char('漢'));
    assert!(is_fullwidth_char('😀'));
    assert!(!is_fullwidth_char('A'));
    assert!(!is_fullwidth_char('a'));
}

// --- 和欧間量指定 (auto_spacing_em) ---

#[test]
fn auto_spacing_em_scales_gap() {
    let mut half = style(24.0);
    half.auto_spacing_em = 0.5;
    let mixed = text_advance_estimate("あA", &half);
    let plain = text_advance_estimate("あ", &half);
    let a = text_advance_estimate("A", &half);
    assert!(
        (mixed - plain - a - ja_latin_gap_em(24.0, 0.5)).abs() < 1e-6,
        "half-em gap: {mixed} vs {}",
        plain + a + ja_latin_gap_em(24.0, 0.5)
    );
    // 0 keeps the boundary but removes the gap.
    let mut zero = style(24.0);
    zero.auto_spacing_em = 0.0;
    assert!(
        (text_advance_estimate("あA", &zero) - (plain + a)).abs() < 1e-6,
        "0em gap is still a boundary but no extra advance"
    );
    // Serde default keeps 1/4em on old documents.
    let json = r#"{"font_family":"Inter","font_size":12}"#;
    let restored: TextStyle = serde_json::from_str(json).expect("style json");
    assert!(restored.auto_spacing);
    assert_eq!(restored.auto_spacing_em, 0.25);
}

// --- 縦組み禁則 (kinsoku) ---

#[test]
fn vertical_kinsoku_binds_punctuation_to_previous_column() {
    // 行頭禁則: "、" never starts a column — it rides on the column above
    // (追い込み). At 1.5em columns every char gets its own column, but
    // "、" stays attached to う instead of opening a fourth one.
    let mut st = style(10.0);
    st.vertical = true;
    st.word_wrap = true;
    st.max_width = Some(15.0);
    // ぶら下げあり（既定）: 、 の Ink が半文字分はみ出すまで詰められる。
    let layout = layout_text("あいう、あ", &st, None);
    assert_eq!(
        layout.lines,
        vec!["あ".to_string(), "いう、".to_string(), "あ".to_string()],
        "hanging punctuation lets the column pack one glyph more"
    );
    // ぶら下げなし: 追い込みだけなので 1 文字ごとに折る（旧挙動）。
    st.burasage = false;
    let plain = layout_text("あいう、あ", &st, None);
    assert_eq!(
        plain.lines,
        vec![
            "あ".to_string(),
            "い".to_string(),
            "う、".to_string(),
            "あ".to_string()
        ],
        "without ぶら下け the closing mark costs its full em"
    );
}

#[test]
fn burasage_hang_amounts_match_the_convention() {
    // 半角ぶら下げ（句読点）と全角ぶら下げ（閉じ括弧）。
    assert!((burasage_hang('、') - 0.5).abs() < 1e-9);
    assert!((burasage_hang('。') - 0.5).abs() < 1e-9);
    assert!((burasage_hang('…') - 0.5).abs() < 1e-9);
    assert!((burasage_hang('」') - 1.0).abs() < 1e-9);
    assert!((burasage_hang('）') - 1.0).abs() < 1e-9);
    assert_eq!(burasage_hang('あ'), 0.0);
    assert_eq!(burasage_hang('（'), 0.0);

    // 横組み: 和欧間(0.25em)込みで あ(10) + A(6) + gap(2.5) + 。(10) + gap(2.5)
    // = 31px。max_width=30 だと off は 31>30 で改行、on は 。 の Ink 0.5em=5px を
    // 許すので 31<=35 で同居する。
    let mut st = style(10.0);
    st.word_wrap = true;
    st.max_width = Some(30.0);
    st.burasage = false;
    let off = layout_text("あA。", &st, None);
    st.burasage = true;
    let on = layout_text("あA。", &st, None);
    // off: 追い込みで 。 は残るが A の手前で折れる（欧文の禁則境界）
    assert_eq!(off.lines, vec!["あ".to_string(), "A。".to_string()]);
    // on: 。 の Ink 分だけ予算が増えるので A まで同一行に乗る
    assert_eq!(on.lines, vec!["あA。".to_string()]);
}

#[test]
fn vertical_kinsoku_pushes_opening_bracket_down() {
    // 行末禁則: an opening bracket never ends a column — it drops to the
    // next column (追い出し). あ「いう at 2.1em: 「 moves down with いう.
    let mut st = style(10.0);
    st.vertical = true;
    st.word_wrap = true;
    st.max_width = Some(21.0);
    let layout = layout_text("あ「いう", &st, None);
    assert_eq!(
        layout.lines,
        vec!["あ".to_string(), "「い".to_string(), "う".to_string()],
        "opening bracket is pushed to the next column"
    );
}

// --- 縦中横 (tate-chū-yoko) ---

#[test]
fn tatechuyoko_run_detects_short_digit_runs() {
    assert_eq!(tatechuyoko_run(&['1', '2'], 0), Some(2));
    assert_eq!(tatechuyoko_run(&['1', '2', '3'], 0), Some(3));
    assert_eq!(tatechuyoko_run(&['1', '2', '3', '4'], 0), None);
    // …and a long run never fragments either.
    assert_eq!(tatechuyoko_run(&['1', '2', '3', '4'], 1), None);
    assert_eq!(tatechuyoko_run(&['あ', '1', '2'], 0), None);
    assert_eq!(tatechuyoko_run(&['あ', '1', '2'], 1), Some(2));
    assert_eq!(tatechuyoko_scale(2), 0.5);
    assert_eq!(tatechuyoko_scale(3), 1.0 / 3.0);
}

#[test]
fn vertical_layout_gives_tatechuyoko_unit_one_em_cell() {
    // 2–3 digit runs occupy exactly one em cell (no 0.6em-per-digit math).
    let mut st = style(10.0);
    st.vertical = true;
    st.word_wrap = true;
    st.max_width = Some(21.0); // 2.1em
    let two = layout_text("令和12", &st, None);
    assert_eq!(
        two.lines,
        vec!["令和".to_string(), "12".to_string()],
        "the digit pair wraps as a single em cell"
    );
    let three = layout_text("令和123", &st, None);
    assert_eq!(
        three.lines,
        vec!["令和".to_string(), "123".to_string()],
        "a 3-digit run still fits one em cell"
    );
}

#[test]
fn tatechuyoko_outline_sets_digits_inline() {
    let mut st = style(20.0);
    st.vertical = true;
    let unit = try_text_to_outline_path_with_style("12", &st).expect("unit outline");
    let (w, h) = bbox_of(&unit);
    assert!(
        w < 20.0 * 0.9,
        "the pair is compressed into the em cell: {w}x{h}"
    );
    assert!(h < 20.0, "one em cell tall: {w}x{h}");
    // Four digits keep the rotated path and stack several cells.
    let rotated = try_text_to_outline_path_with_style("1234", &st).expect("rotated outline");
    let (_, rh) = bbox_of(&rotated);
    assert!(
        rh > 30.0,
        "4 digits stay rotated (no fragment into a 3-digit unit): h={rh}"
    );
}

// --- ルビ (ruby) ---

#[test]
fn ruby_markup_parses_both_notations() {
    let (base, anns) = parse_ruby("｜漢字《かんじ》");
    assert_eq!(base, "漢字");
    assert_eq!(anns.len(), 1);
    assert_eq!(anns[0].start, 0);
    assert_eq!(anns[0].len, 2);
    assert_eq!(anns[0].reading, "かんじ");
    // len == annotation length, and no base char is dropped.
    let (base2, anns2) = parse_ruby("｜良質《りょうしつ》な品");
    assert_eq!(base2, "良質な品");
    assert_eq!(
        (anns2[0].start, anns2[0].len, anns2[0].reading.as_str()),
        (0, 2, "りょうしつ")
    );
    let (base3, anns3) = parse_ruby("字(よみ)");
    assert_eq!(base3, "字");
    assert_eq!((anns3[0].start, anns3[0].len), (0, 1));
    // Unbalanced markup stays literal text.
    let (base4, anns4) = parse_ruby("字(よみ");
    assert_eq!(base4, "字(よみ");
    assert!(anns4.is_empty());
    let (base5, anns5) = parse_ruby("単(たん)｜連結《れんけつ》語");
    assert_eq!(base5, "単連結語");
    assert_eq!(anns5.len(), 2);
    assert_eq!((anns5[1].start, anns5[1].len), (1, 2));
}

#[test]
fn ruby_markup_takes_no_vertical_advance() {
    // The wrap must see only base chars: 漢字《かんじ》字 wraps after
    // 2 base chars (2em), not after the 4-char notated string.
    let mut st = style(10.0);
    st.vertical = true;
    st.word_wrap = true;
    st.max_width = Some(21.0); // 2.1em
    let layout = layout_text("｜漢字《かんじ》字", &st, None);
    assert_eq!(
        layout.lines,
        vec!["｜漢字《かんじ》".to_string(), "字".to_string()],
        "readings must not consume column cells: {:?}",
        layout.lines,
    );
    // Without the 2-char group the notation would wrap much earlier.
    let plain = layout_text("字", &st, None);
    assert_eq!(plain.lines, vec!["字".to_string()]);
}

#[test]
fn vertical_ruby_outline_draws_reading_beside_the_column() {
    // Reading sits to the right of the base column: ink spans past x=font_size.
    let mut st = TextStyle::new("Noto Sans JP", 24.0);
    st.vertical = true;
    let with_ruby = try_text_to_outline_path_with_style("｜漢《かん》", &st).expect("outline");
    let (rw, _) = bbox_of(&with_ruby);
    let with_ruby2 = try_text_to_outline_path_with_style("｜漢字《かんじ》", &st)
        .expect("outline with 2-char base");
    let (rw2, rh2) = bbox_of(&with_ruby2);
    assert!(rw > 24.0, "the reading lives right of the column: {rw}");
    // A 2-char base group stacks 2 em tall and still carries the reading.
    assert!(rh2 > 40.0, "base stack: {rh2}");
    assert!(rw2 > 24.0, "the 2-char group keeps its reading: {rw2}");
}

#[test]
fn vertical_svg_emits_ruby_reading() {
    let mut doc = irasu_illustrator::core::document::Document::default();
    let mut st = style(24.0);
    st.font_family = "Noto Sans JP".into();
    st.vertical = true;
    doc.add_object(Object::new_text_with_style(
        "T",
        "｜漢字《かんじ》",
        10.0,
        10.0,
        st,
    ));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(
        svg.contains("font-size=\"50%\""),
        "half-size reading: {svg}"
    );
    assert!(svg.contains(">かんじ</tspan>"), "reading emitted: {svg}");
    assert!(
        !svg.contains("《") && !svg.contains("｜"),
        "markup never reaches the export: {svg}"
    );
}

#[test]
fn horizontal_ruby_outline_draws_reading_above() {
    // Text outline paths use the app's y-down convention: ink above the
    // baseline is NEGATIVE y. The reading must therefore extend to a more
    // negative y than the base run (a sign flip here renders it below).
    let st = TextStyle::new("Noto Sans JP", 20.0);
    let with_ruby =
        try_text_to_outline_path_with_style("｜漢字《かんじ》", &st).expect("ruby outline");
    let base_only = try_text_to_outline_path_with_style("漢字", &st).expect("base outline");
    let (_, ruby_top) = with_ruby
        .bounding_box()
        .map(|(mn, _)| (0.0, mn.y))
        .unwrap_or((0.0, 0.0));
    let (_, base_top) = base_only
        .bounding_box()
        .map(|(mn, _)| (0.0, mn.y))
        .unwrap_or((0.0, 0.0));
    assert!(
        ruby_top < base_top - 4.0,
        "reading sits above the base run: top={ruby_top} vs base top={base_top}"
    );
    // Base ink stays where it was (only height grows).
    let (rw, rh) = bbox_of(&with_ruby);
    let (bw, bh) = bbox_of(&base_only);
    assert!(
        (rw - bw).abs() < 8.0,
        "width stays base-driven: {rw} vs {bw}"
    );
    assert!(rh > bh + 4.0, "reading adds ink: {rw}x{rh} vs {bw}x{bh}");
}

#[test]
fn horizontal_svg_emits_ruby_reading() {
    let mut doc = irasu_illustrator::core::document::Document::default();
    let mut st = style(20.0);
    st.font_family = "Noto Sans JP".into();
    doc.add_object(Object::new_text_with_style(
        "T",
        "｜漢字《かんじ》",
        10.0,
        100.0,
        st,
    ));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(
        svg.contains(">かんじ</tspan>"),
        "reading emitted above the base: {svg}"
    );
    // Absolute y (a `dy` would leak into the base tspan, shifting the run).
    assert!(
        svg.contains(&format!("y=\"{}\"", 100.0 - 20.0 * 0.95)),
        "reading sits 0.95em above the baseline: {svg}"
    );
    assert!(
        !svg.contains("《") && !svg.contains("｜"),
        "markup never reaches the export: {svg}"
    );
    // The base text still renders.
    assert!(svg.contains(">漢字<"), "base run: {svg}");
}

#[test]
fn ruby_reading_scales_with_font_size() {
    // Half-size readings: a bigger base keeps the same 2:1 ink ratio.
    let small = TextStyle::new("Noto Sans JP", 10.0);
    let large = TextStyle::new("Noto Sans JP", 40.0);
    let (sw, sh) =
        bbox_of(&try_text_to_outline_path_with_style("｜漢《かん》", &small).expect("small"));
    let (lw, lh) =
        bbox_of(&try_text_to_outline_path_with_style("｜漢《かん》", &large).expect("large"));
    // The reading strip adds a fixed *em* multiple of height either way.
    assert!(
        (lh / lh - sh / sh).abs() < 1e-9,
        "ratios are font-size independent: {sh} vs {lh}"
    );
    assert!(
        sh < lh && sw < lw,
        "outline scales up: {sw}x{sh} vs {lw}x{lh}"
    );
}

// --- OpenType features ---

#[test]
fn noto_face_reports_japanese_features() {
    use irasu_illustrator::core::font::FontRegistry;
    let tags = FontRegistry::global().face_open_type_features(
        "Noto Sans JP",
        400,
        TextStyle::new("Inter", 12.0).font_style,
    );
    let names: Vec<&str> = tags.iter().map(|(t, _)| t.as_str()).collect();
    for want in ["palt", "vert", "vrt2", "kern", "halt", "jp78"] {
        assert!(
            names.contains(&want),
            "Noto Sans JP should expose {want}: {names:?}"
        );
    }
    // Latin-only faces: no Japanese layout features, but kern is there.
    let inter = FontRegistry::global().face_open_type_features(
        "Inter",
        400,
        TextStyle::new("Inter", 12.0).font_style,
    );
    let inter_names: Vec<&str> = inter.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        !inter_names.contains(&"palt"),
        "Inter has no 約物半角 feature: {inter_names:?}"
    );
}

#[test]
fn palt_halves_punctuation_advance_when_forced_on() {
    // HarfBuzz leaves `palt` off by default; forcing it on must visibly
    // narrow fullwidth punctuation (measured through the public style API).
    let mut on = TextStyle::new("Noto Sans JP", 24.0);
    on.set_ot_feature("palt", true);
    let mut off = TextStyle::new("Noto Sans JP", 24.0);
    off.set_ot_feature("palt", false);

    let on_path = try_text_to_outline_path_with_style("、、、", &on).expect("palt on outline");
    let off_path = try_text_to_outline_path_with_style("、、、", &off).expect("palt off outline");
    let (w_on, _) = bbox_of(&on_path);
    let (w_off, _) = bbox_of(&off_path);
    assert!(
        w_on < w_off * 0.6,
        "palt halves the punctuation run: {w_on} vs {w_off}"
    );
    // Pairs reach the shaper: liga stays controlled by `ligatures`.
    let pairs = on.ot_feature_pairs();
    let palt = pairs.iter().find(|(t, _)| t == b"palt").map(|(_, v)| *v);
    assert_eq!(palt, Some(1), "explicit override wins: {pairs:?}");
    assert!(pairs.iter().any(|(t, v)| t == b"liga" && *v == 1));
}

#[test]
fn ot_feature_overrides_round_trip_and_reject_bad_tags() {
    let mut st = TextStyle::new("Inter", 12.0);
    st.set_ot_feature("palt", true);
    st.set_ot_feature("ruby", true);
    st.set_ot_feature("nope!", true); // not a valid 4-char tag
    assert_eq!(st.ot_feature_state("PALT"), Some(true));
    assert_eq!(st.ot_feature_state("ruby"), Some(true));
    assert_eq!(st.ot_feature_state("liga"), None, "defaults stay implicit");
    assert_eq!(st.ot_features.len(), 2, "malformed tags are dropped");
    st.clear_ot_feature("PALT");
    assert_eq!(st.ot_feature_state("palt"), None);

    let json = serde_json::to_string(&st).expect("style json");
    let back: TextStyle = serde_json::from_str(&json).expect("back");
    assert_eq!(back.ot_features, st.ot_features);
    // Old documents without the field keep the shaper defaults.
    let legacy = r#"{"font_family":"Inter","font_size":12}"#;
    let old: TextStyle = serde_json::from_str(legacy).expect("legacy json");
    assert!(old.ot_features.is_empty());
    assert!(old.ligatures, "legacy default keeps ligatures on");
}

#[test]
fn vertical_brackets_use_the_fonts_rotated_vert_form() {
    // 1. Real OpenType: the outline path substitutes the font's own
    //    vertical form, which for brackets is a 90° rotation.
    let mut v = TextStyle::new("Noto Sans JP", 40.0);
    v.vertical = true;
    let (vw, vh) =
        bbox_of(&try_text_to_outline_path_with_style("（", &v).expect("vertical bracket"));
    let h = TextStyle::new("Noto Sans JP", 40.0);
    let (hw, hh) =
        bbox_of(&try_text_to_outline_path_with_style("（", &h).expect("horizontal bracket"));
    assert!(
        (vw - hh).abs() < 1.0 && (vh - hw).abs() < 1.0,
        "vertical （ is the rotated form: {vw}x{vh} vs {hw}x{hh}"
    );
    // 、。 keep their (repositioned, unrotated) vertical form.
    let (cw, ch) = bbox_of(&try_text_to_outline_path_with_style("、", &v).expect("vertical comma"));
    let (hcw, hch) =
        bbox_of(&try_text_to_outline_path_with_style("、", &h).expect("horizontal comma"));
    assert!(
        (cw - hcw).abs() < 2.0 && (ch - hch).abs() < 2.0,
        "、 is not rotated, only repositioned: {cw}x{ch} vs {hcw}x{hch}"
    );
    // 2. Canvas/SVG (live-text renderers) rotate the same set.
    assert!(is_vert_rotated_char('（') && is_vert_rotated_char('」'));
    assert!(!is_vert_rotated_char('、') && !is_vert_rotated_char('ー'));
    assert!(!is_vert_rotated_char('A') && !is_vert_rotated_char('あ'));
}

#[test]
fn vertical_svg_rotates_brackets_via_vert_set() {
    let mut doc = irasu_illustrator::core::document::Document::default();
    let mut st = style(24.0);
    st.font_family = "Noto Sans JP".into();
    st.vertical = true;
    doc.add_object(Object::new_text_with_style(
        "T",
        "（あ）「い」",
        10.0,
        10.0,
        st,
    ));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    // Brackets ride the same rotate(90 …) tspan the halfwidth path uses.
    assert_eq!(
        svg.matches("transform=\"rotate(90").count(),
        4,
        "（ ） 「 」 all rotate: {svg}"
    );
    assert!(
        !svg.contains("writing-mode="),
        "still no CSS writing-mode dependency: {svg}"
    );
}

#[test]
fn vertical_feature_pairs_default_on_and_respect_overrides() {
    use irasu_illustrator::core::text_path::vertical_feature_pairs;
    let plain = TextStyle::new("Noto Sans JP", 40.0);
    let pairs = vertical_feature_pairs(&plain);
    for tag in [
        b"vert", b"vrt2", b"vkrn", b"vpal", b"valt", b"vchw", b"vrtr",
    ] {
        let v = pairs.iter().find(|(t, _)| t == tag).map(|(_, v)| *v);
        assert_eq!(
            v,
            Some(1),
            "{:?} default on: {pairs:?}",
            std::str::from_utf8(tag)
        );
    }
    let mut off = plain.clone();
    off.set_ot_feature("vert", false);
    off.set_ot_feature("vrt2", false);
    let off_pairs = vertical_feature_pairs(&off);
    assert_eq!(
        off_pairs
            .iter()
            .find(|(t, _)| t == b"vert")
            .map(|(_, v)| *v),
        Some(0),
        "explicit off wins: {off_pairs:?}"
    );
    assert_eq!(
        off_pairs
            .iter()
            .find(|(t, _)| t == b"vrt2")
            .map(|(_, v)| *v),
        Some(0)
    );
    // Non-vertical features pass straight through.
    let mut palt = plain.clone();
    palt.set_ot_feature("palt", true);
    assert_eq!(
        vertical_feature_pairs(&palt)
            .iter()
            .find(|(t, _)| t == b"palt")
            .map(|(_, v)| *v),
        Some(1)
    );
}

#[test]
fn halt_halves_punctuation_in_estimates_and_outlines() {
    // The estimate model (wrapping / measuring / SVG dx math) and the
    // HarfBuzz outline path must agree: `halt` halves the punctuation set.
    let mut on = TextStyle::new("Noto Sans JP", 40.0);
    on.set_ot_feature("halt", true);
    let mut off = TextStyle::new("Noto Sans JP", 40.0);
    off.set_ot_feature("halt", false);
    assert!(!off.halt_on() && on.halt_on());
    let full = text_advance_estimate("、", &off);
    let half = text_advance_estimate("、", &on);
    assert!(
        (half * 2.0 - full).abs() < 1e-6,
        "、 halves under halt: {full} vs {half}"
    );
    // Brackets are NOT in the halt set (Noto keeps them full width).
    assert!(
        (text_advance_estimate("（", &on) - text_advance_estimate("（", &off)).abs() < 1e-6,
        "（ stays full width"
    );
    // Wrapping sees the halved advance: 3 punctuation + 1 kanji at 1em.
    let mut wrap = on.clone();
    wrap.word_wrap = true;
    wrap.max_width = Some(1.5 * 40.0);
    let layout = layout_text("、、、あ", &wrap, None);
    assert_eq!(
        layout.lines,
        vec!["、、、".to_string(), "あ".to_string()],
        "halt changes where the column wraps: {:?}",
        layout.lines
    );
    // The outline path (HarfBuzz `halt` forms) narrows to match.
    let (w_on, _) = bbox_of(&try_text_to_outline_path_with_style("、、、", &on).expect("halt on"));
    let (w_off, _) =
        bbox_of(&try_text_to_outline_path_with_style("、、、", &off).expect("halt off"));
    assert!(w_on < w_off * 0.7, "outline narrows too: {w_on} vs {w_off}");
}

#[test]
fn vertical_outline_uses_font_vertical_metrics() {
    // 縦組み advances come from the face's `vmtx`, not the 0.6em estimate:
    // two Latin glyphs measure ~0.5em each in Noto Sans JP.
    let mut v = TextStyle::new("Noto Sans JP", 40.0);
    v.vertical = true;
    let (_, h) = bbox_of(&try_text_to_outline_path_with_style("Ab", &v).expect("vertical latin"));
    assert!(
        h > 30.0 && h < 46.0,
        "vmtx-driven column (~40 for 2 glyphs), estimate would be 48: {h}"
    );
    // Fullwidth kanji + punctuation packs tighter than 3 × 1em too.
    let (_, h3) =
        bbox_of(&try_text_to_outline_path_with_style("雨、子", &v).expect("vertical cjk"));
    assert!(h3 < 110.0, "font metrics: {h3}");
}

#[test]
fn canvas_cache_key_tracks_open_type_and_spacing() {
    use irasu_illustrator::core::document::TextStyle;
    use irasu_illustrator::ui::canvas::rendering::text_shape_key;
    let base = TextStyle::new("Noto Sans JP", 24.0);
    let mut palt = base.clone();
    palt.set_ot_feature("palt", true);
    assert_ne!(
        text_shape_key("あ", &base, None),
        text_shape_key("あ", &palt, None),
        "toggling an OT feature must rebuild the cached outlines"
    );
    let mut gap = base.clone();
    gap.auto_spacing_em = 0.5;
    assert_ne!(
        text_shape_key("あ", &base, None),
        text_shape_key("あ", &gap, None),
        "和欧間 sizing must rebuild too"
    );
    let mut ls = base.clone();
    ls.letter_spacing = 2.0;
    assert_ne!(
        text_shape_key("あ", &base, None),
        text_shape_key("あ", &ls, None)
    );
    assert_eq!(
        text_shape_key("あ", &base, None),
        text_shape_key("あ", &base.clone(), None),
        "equal styles keep the same key (no-op must not rebuild)"
    );
}

#[test]
fn svg_font_feature_settings_round_trip() {
    // Export → import must keep explicit OpenType overrides.
    let mut doc = irasu_illustrator::core::document::Document::default();
    let mut st = style(24.0);
    st.font_family = "Noto Sans JP".into();
    st.set_ot_feature("palt", true);
    st.set_ot_feature("jp90", true);
    doc.add_object(Object::new_text_with_style("T", "あい", 10.0, 40.0, st));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(
        svg.contains("'palt' 1") && svg.contains("'jp90' 1"),
        "exporter writes feature settings: {svg}"
    );
    let back = irasu_illustrator::io::svg::parse_svg_document(&svg);
    let text_obj = back
        .layers
        .iter()
        .flat_map(|l| &l.objects)
        .find(|o| {
            matches!(
                o.object_type,
                irasu_illustrator::core::document::ObjectType::Text { .. }
            )
        })
        .expect("text object");
    let irasu_illustrator::core::document::ObjectType::Text { style, .. } = &text_obj.object_type
    else {
        unreachable!()
    };
    assert_eq!(style.ot_feature_state("palt"), Some(true));
    assert_eq!(style.ot_feature_state("jp90"), Some(true));
    assert_eq!(style.ot_feature_state("vert"), None);
    // A hand-written CSS value with double quotes / off still parses.
    assert_eq!(
        irasu_illustrator::io::svg::parse_font_feature_settings_public(
            "\"palt\" 1, \"vert\" 0, \"liga\", \"bad\" x"
        )
        .iter()
        .map(|f| (f.tag.as_str(), f.on))
        .collect::<Vec<_>>(),
        vec![("palt", true), ("vert", false), ("liga", true)],
        "malformed items are skipped"
    );
}

fn raster_ink_width(doc: &irasu_illustrator::core::document::Document, outline: bool) -> u32 {
    let png = irasu_illustrator::io::raster::export_png_with_outline(doc, 1.0, false, outline)
        .expect("png");
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(&png).expect("decode");
    let mut min_x = u32::MAX;
    let mut max_x = 0;
    for y in 0..pixmap.height() {
        for x in 0..pixmap.width() {
            let p = pixmap.pixel(x, y).expect("px").demultiply();
            if p.red() < 240 || p.green() < 240 || p.blue() < 240 {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
    }
    max_x.saturating_sub(min_x)
}

#[test]
fn outline_text_makes_svg_and_png_honour_open_type_features() {
    // The raster engine (resvg) ignores `font-feature-settings`, so only
    // outlined text carries the font's OpenType features to SVG/PNG.
    let mut doc = irasu_illustrator::core::document::Document::default();
    doc.width = 300.0;
    doc.height = 120.0;
    let mut st = style(40.0);
    st.font_family = "Noto Sans JP".into();
    st.set_ot_feature("palt", true);
    doc.add_object(Object::new_text_with_style("T", "、、、", 10.0, 60.0, st));

    let live = irasu_illustrator::io::svg::export_svg(&doc);
    let outlined = irasu_illustrator::io::svg::export_svg_with_options(&doc, false, None, true);
    assert!(live.contains("<text"), "live text keeps <text>: {live}");
    assert!(
        !outlined.contains("<text"),
        "outline mode drops <text>: {outlined}"
    );
    assert!(
        outlined.contains("<path"),
        "outline mode emits <path>: {outlined}"
    );

    let live_w = raster_ink_width(&doc, false);
    let outline_w = raster_ink_width(&doc, true);
    assert!(
        outline_w < live_w * 3 / 4,
        "palt only reaches the raster via outlines: {outline_w} vs {live_w}"
    );
    // The registry always resolves a fallback face (sans-serif), so an
    // unknown family still outlines — with the fallback face's glyphs.
    let mut missing = irasu_illustrator::core::document::Document::default();
    missing.width = 300.0;
    missing.height = 120.0;
    let mut ghost = style(40.0);
    ghost.font_family = "NoSuchFace, NoSuchOther".into();
    missing.add_object(Object::new_text_with_style("T", "abc", 10.0, 60.0, ghost));
    let fallback = irasu_illustrator::io::svg::export_svg_with_options(&missing, false, None, true);
    assert!(
        fallback.contains("<path") && !fallback.contains("<text"),
        "unknown families outline through the fallback face: {fallback}"
    );
}

#[test]
fn vertical_svg_emits_tatechuyoko_run() {
    let mut doc = irasu_illustrator::core::document::Document::default();
    let mut st = style(20.0);
    st.vertical = true;
    doc.add_object(Object::new_text_with_style("T", "令和12", 10.0, 10.0, st));
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(
        svg.contains("font-size=\"50%\""),
        "digit pair is set at half size inline: {svg}"
    );
    assert!(svg.contains(">12</tspan>"), "the run stays together: {svg}");
}
