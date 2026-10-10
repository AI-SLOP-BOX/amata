//! PNG エンドツーエンドのサンプル（目視確認用）。
//! テキストのアウトライン経路を通るので、PDFと同じ絵が出ます。
//! `cargo test --test png_sample_dump -- --ignored --nocapture`
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object, ObjectType, TextArea, TextStyle};
use irasu_illustrator::core::path::FillStyle;

fn text_obj(
    name: &str,
    text: &str,
    x: f64,
    y: f64,
    style: TextStyle,
    area: Option<TextArea>,
) -> Object {
    let mut o = Object::new_text_with_style(name, text, x, y, style);
    if let ObjectType::Text { area: slot, .. } = &mut o.object_type {
        *slot = area;
    }
    o
}

#[test]
#[ignore = "manual: writes output/jp_typesetting_sample.png"]
fn dump_jp_typesetting_png() {
    let mut doc = Document::default();
    doc.width = 1200.0;
    doc.height = 900.0;

    // 1. 縦組み（ルビ・縦中横・vert 形のブラケット）
    let mut vert = TextStyle::new("Noto Sans JP", 28.0);
    vert.vertical = true;
    vert.word_wrap = true;
    vert.line_height = Some(1.6);
    doc.add_object(text_obj(
        "縦組み",
        "｜本日《きょう》は｜良質《りょうしつ》な品を12点、ご用意しました。（涙 debut）「あ」――了――",
        60.0,
        60.0,
        vert.clone(),
        Some(TextArea::new(60.0, 60.0, 420.0, 720.0)),
    ));

    // 2. 横組み＋ルビ
    let mut horiz = TextStyle::new("Noto Sans JP", 28.0);
    horiz.word_wrap = true;
    horiz.max_width = Some(420.0);
    doc.add_object(text_obj(
        "横組み",
        "｜組み《く》みのテストです。AmataとIllustratorを1対4で比較する。",
        700.0,
        60.0,
        horiz.clone(),
        None,
    ));

    // 3. palt OFF / ON
    for (i, on) in [false, true].into_iter().enumerate() {
        let mut s = TextStyle::new("Noto Sans JP", 26.0);
        s.set_ot_feature("palt", on);
        doc.add_object(text_obj(
            &format!("palt_{i}"),
            "、「。」（）ァ ィ ゥ",
            700.0,
            320.0 + i as f64 * 70.0,
            s.clone(),
            None,
        ));
    }

    // 4. 和欧間の3段階
    for (i, (em, label)) in [(0.125_f32, "1/8em"), (0.25, "1/4em"), (0.5, "1/2em")]
        .into_iter()
        .enumerate()
    {
        let mut s = TextStyle::new("Noto Sans JP", 22.0);
        s.auto_spacing_em = em;
        doc.add_object(text_obj(
            &format!("和欧間{label}"),
            "あA漢B",
            700.0,
            520.0 + i as f64 * 50.0,
            s,
            None,
        ));
    }

    // 5. 図形（テキスト以外の経路も確認）
    let mut rect = Object::new_rect("rect", 560.0, 620.0, 100.0, 60.0, 8.0);
    rect.fill = Some(FillStyle::solid([0.75, 0.55, 0.2, 1.0]));
    doc.add_object(rect);

    // Outline mode: OpenType features (palt/vert) survive into the raster.
    let png = irasu_illustrator::io::raster::export_png_with_outline(&doc, 1.0, false, true)
        .expect("png");
    std::fs::create_dir_all("output").unwrap();
    std::fs::write("output/jp_typesetting_sample.png", &png).unwrap();
    eprintln!(
        "wrote output/jp_typesetting_sample.png ({} bytes)",
        png.len()
    );
}
