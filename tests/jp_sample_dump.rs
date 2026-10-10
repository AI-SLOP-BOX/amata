//! 日本語組版のサンプルを書き出す（目視確認用）。
//! `cargo test --test jp_sample_dump -- --ignored --nocapture`
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object, TextArea, TextStyle};
use irasu_illustrator::io::svg::export_svg;

fn add(doc: &mut Document, name: &str, text: &str, x: f64, y: f64, style: TextStyle) {
    doc.add_object(Object::new_text_with_style(name, text, x, y, style));
}

#[test]
#[ignore = "manual: writes output/jp_typesetting_sample.svg"]
fn dump_jp_typesetting_sample() {
    let mut doc = Document::default();
    doc.width = 1200.0;
    doc.height = 900.0;

    // 1. 縦組み + ルビ + 縦中横（領域テキスト）
    let mut vert = TextStyle::new("Noto Sans JP", 28.0);
    vert.vertical = true;
    vert.word_wrap = true;
    vert.max_width = Some(360.0);
    vert.line_height = Some(1.6);
    let mut t = Object::new_text_with_style(
        "縦組み",
        "｜本日《きょう》は｜良質《りょうしつ》な品を12点、ご用意しました。（涙 debut）「あ」――了――",
        60.0,
        60.0,
        vert.clone(),
    );
    if let irasu_illustrator::core::document::ObjectType::Text { area, .. } = &mut t.object_type {
        *area = Some(TextArea::new(60.0, 60.0, 420.0, 720.0));
    }
    doc.add_object(t);

    // 2. 横組み + ルビ（和欧間も入れる）
    let mut horiz = TextStyle::new("Noto Sans JP", 28.0);
    horiz.word_wrap = true;
    horiz.max_width = Some(420.0);
    add(
        &mut doc,
        "横組み",
        "｜組み《く》みのテストです。AmataとIllustratorを1対4で比較する。",
        700.0,
        60.0,
        horiz.clone(),
    );

    // 3. 和欧間の量比較（1/8・1/4・1/2em）
    for (i, (em, label)) in [(0.125_f32, "1/8em"), (0.25, "1/4em"), (0.5, "1/2em")]
        .into_iter()
        .enumerate()
    {
        let mut s = TextStyle::new("Noto Sans JP", 22.0);
        s.auto_spacing_em = em;
        add(
            &mut doc,
            &format!("和欧間{label}"),
            "あA漢B（和欧間）",
            700.0,
            320.0 + i as f64 * 40.0,
            s,
        );
    }

    // 4. 縦中横（年号・2桁/3桁）
    let mut year = TextStyle::new("Noto Sans JP", 26.0);
    year.vertical = true;
    add(
        &mut doc,
        "縦中横",
        "令和12年、1234個",
        560.0,
        60.0,
        year.clone(),
    );

    // 5. 禁則（句読点・閉じ括弧）
    let mut kin = TextStyle::new("Noto Sans JP", 24.0);
    kin.word_wrap = true;
    kin.max_width = Some(120.0);
    add(
        &mut doc,
        "禁則",
        "これは行头禁則のテストです）、「(next)」という順番になります。",
        700.0,
        520.0,
        kin,
    );

    // 6. OpenType: palt OFF vs ON（約物の半角詰め）
    for (i, on) in [false, true].into_iter().enumerate() {
        let mut s = TextStyle::new("Noto Sans JP", 26.0);
        s.set_ot_feature("palt", on);
        add(
            &mut doc,
            &format!("ot_{i}"),
            "、「。」（）ァ ィ ゥ",
            700.0,
            760.0 + i as f64 * 60.0,
            s,
        );
        // 行ラベル
        let lbl = TextStyle::new("Inter", 18.0);
        add(
            &mut doc,
            "ot_label",
            if on { "palt ON" } else { "palt OFF" },
            980.0,
            760.0 + i as f64 * 60.0,
            lbl,
        );
    }

    let svg = export_svg(&doc);
    std::fs::create_dir_all("output").unwrap();
    std::fs::write("output/jp_typesetting_sample.svg", svg).unwrap();
    eprintln!("wrote output/jp_typesetting_sample.svg");
}
