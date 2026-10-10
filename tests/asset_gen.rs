//! 自作テンプレート（`assets/templates/*.amata`）の生成器。
//! 内容はコード側が正なので、レイアウトを変えたら再実行する:
//! `cargo test --test asset_gen -- --ignored --nocapture`
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{
    Artboard, Document, FontStyle, Object, ObjectType, TextArea, TextStyle,
};
use irasu_illustrator::core::path::{FillStyle, StrokeStyle};

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

/// Thin rule (border line) helper.
fn line(name: &str, x1: f64, y1: f64, x2: f64, y2: f64, width: f64) -> Object {
    let mut o = Object::new_line(name, x1, y1, x2, y2);
    o.stroke = Some(StrokeStyle {
        width,
        color: [0.0, 0.0, 0.0, 1.0],
        ..Default::default()
    });
    o
}

/// Rect outline (no fill) helper for table cells / borders.
fn cell(name: &str, x: f64, y: f64, w: f64, h: f64) -> Object {
    let mut o = Object::new_rect(name, x, y, w, h, 0.0);
    o.fill = None;
    o.stroke = Some(StrokeStyle {
        width: 1.0,
        color: [0.0, 0.0, 0.0, 1.0],
        ..Default::default()
    });
    o
}

fn verify(doc: &Document) {
    // Templates must round-trip through the real project loader.
    let json = serde_json::to_string_pretty(doc).expect("serialize");
    let back: Document = serde_json::from_str(&json).expect("round-trip");
    assert_eq!(
        serde_json::to_string(&back).unwrap(),
        serde_json::to_string(&doc).unwrap()
    );
}

#[test]
#[ignore = "manual: writes assets/templates/*.amata"]
fn generate_templates() {
    // 1. A4縦組みポスター (A4 = 595×842pt @72dpi → 2480×3508px @300dpi/3.46)
    //    作業単位は px。A4比率で 1240×1754 px。
    let mut poster = Document::default();
    poster.name = "A4縦組みポスター".into();
    poster.width = 1240.0;
    poster.height = 1754.0;
    poster.artboards = vec![Artboard::new("A4", 0.0, 0.0, 1240.0, 1754.0)];
    poster.spots = irasu_illustrator::core::print::default_spots();
    {
        // 背色
        let mut bg = Object::new_rect("背景", 0.0, 0.0, 1240.0, 1754.0, 0.0);
        bg.fill = Some(FillStyle::solid([0.96, 0.94, 0.90, 1.0]));
        poster.add_object(bg);
        // 本文（縦組み・3段）
        let mut body = TextStyle::new("Noto Sans JP", 34.0);
        body.vertical = true;
        body.word_wrap = true;
        body.line_height = Some(1.8);
        body.max_width = Some(340.0);
        poster.add_object(text_obj(
            "本文",
            "｜組版《くみはん》の手本\n縦組みでも崩れない、\n和欧間12pt入りの\nサンプル_textです。",
            120.0,
            160.0,
            body.clone(),
            Some(TextArea::new(120.0, 160.0, 380.0, 1400.0)),
        ));
        // 見出し
        let mut title = TextStyle::new("Noto Sans JP", 96.0);
        title.font_weight = 700;
        poster.add_object(text_obj(
            "見出し",
            "季刊 組",
            760.0,
            220.0,
            title.clone(),
            Some(TextArea::new(760.0, 220.0, 360.0, 560.0)),
        ));
        let mut sub = TextStyle::new("Noto Sans JP", 30.0);
        sub.font_style = FontStyle::Italic;
        poster.add_object(text_obj(
            "リード",
            "季刊号｜第《だい》十二巻\n二〇二六年・秋",
            760.0,
            900.0,
            sub.clone(),
            Some(TextArea::new(760.0, 900.0, 360.0, 200.0)),
        ));
    }
    verify(&poster);
    std::fs::write(
        "assets/templates/a4-poster-vertical.amata",
        serde_json::to_string_pretty(&poster).unwrap(),
    )
    .unwrap();

    // 2. 名刺 91×55mm @300dpi ≈ 1075×650px
    let mut card = Document::default();
    card.name = "名刺".into();
    card.width = 1075.0;
    card.height = 650.0;
    card.artboards = vec![Artboard::new("名刺", 0.0, 0.0, 1075.0, 650.0)];
    card.bleed = 0.0;
    {
        let mut name = TextStyle::new("Noto Sans JP", 54.0);
        name.font_weight = 700;
        card.add_object(text_obj(
            "氏名",
            "｜amata《アマタ》 花子",
            80.0,
            150.0,
            name.clone(),
            None,
        ));
        let role = TextStyle::new("Inter", 30.0);
        card.add_object(text_obj(
            "肩書",
            "Graphic Designer / 組版係",
            80.0,
            260.0,
            role.clone(),
            None,
        ));
        let contact = TextStyle::new("Inter", 26.0);
        card.add_object(text_obj(
            "連絡先",
            "hello@example.com\n+81 90-0000-0000",
            80.0,
            400.0,
            contact.clone(),
            None,
        ));
        // アクセント
        let mut bar = Object::new_rect("アクセント", 80.0, 520.0, 240.0, 14.0, 0.0);
        bar.fill = Some(FillStyle::solid([0.75, 0.55, 0.20, 1.0]));
        card.add_object(bar);
    }
    verify(&card);
    std::fs::write(
        "assets/templates/business-card.amata",
        serde_json::to_string_pretty(&card).unwrap(),
    )
    .unwrap();

    // 3. A4横2段チラシ
    let mut flyer = Document::default();
    flyer.name = "A4横2段チラシ".into();
    flyer.width = 1754.0;
    flyer.height = 1240.0;
    flyer.artboards = vec![Artboard::new("A4横", 0.0, 0.0, 1754.0, 1240.0)];
    {
        let mut head = TextStyle::new("Noto Sans JP", 64.0);
        head.font_weight = 700;
        flyer.add_object(text_obj(
            "タイトル",
            "秋の｜作品《さくひん》展",
            120.0,
            100.0,
            head.clone(),
            None,
        ));
        for (i, gutter) in [(0, "第一"), (1, "第二")] {
            let mut body = TextStyle::new("Noto Sans JP", 28.0);
            body.word_wrap = true;
            body.line_height = Some(1.7);
            flyer.add_object(text_obj(
                &format!("カラム{i}"),
                &format!(
                    "｜{gutter}《{gutter}》の部\n\n組みのテストです。\n和欧間A・12・Bが\n読みやすい並びに\nなります。"
                ),
                120.0 + i as f64 * 860.0,
                300.0,
                body.clone(),
                Some(TextArea::new(120.0 + i as f64 * 860.0, 300.0, 760.0, 800.0)),
            ));
        }
    }
    verify(&flyer);
    std::fs::write(
        "assets/templates/flyer-2col.amata",
        serde_json::to_string_pretty(&flyer).unwrap(),
    )
    .unwrap();

    // 4. 請求書 A4（表組み・罫線・和文均等）
    let mut invoice = Document::default();
    invoice.name = "請求書".into();
    invoice.width = 1240.0;
    invoice.height = 1754.0;
    invoice.artboards = vec![Artboard::new("A4", 0.0, 0.0, 1240.0, 1754.0)];
    invoice.color_mode = irasu_illustrator::core::document::ColorMode::Cmyk;
    invoice.spots = irasu_illustrator::core::print::default_spots();
    {
        let mut title = TextStyle::new("Noto Sans JP", 72.0);
        title.font_weight = 700;
        invoice.add_object(text_obj("請求書", "請 求 書", 420.0, 160.0, title, None));
        let no = TextStyle::new("Noto Sans JP", 24.0);
        invoice.add_object(text_obj("番号", "No. 2026-1010", 840.0, 200.0, no, None));
        // 料品表（罫線）
        let rows = ["品名", "数量", "単価", "金額"];
        let cols = [420.0, 120.0, 200.0, 240.0];
        let x = 100.0;
        let mut y = 420.0;
        for (r, name) in rows.iter().enumerate() {
            let mut cx = x;
            for (c, w) in cols.iter().enumerate() {
                invoice.add_object(cell(&format!("cell_{r}_{c}"), cx, y, *w, 60.0));
                let mut s = TextStyle::new("Noto Sans JP", 22.0);
                if r == 0 {
                    s.font_weight = 700;
                }
                let anchor = if c >= 2 {
                    irasu_illustrator::core::document::TextAnchor::End
                } else {
                    irasu_illustrator::core::document::TextAnchor::Start
                };
                s.text_anchor = anchor;
                let label = if r == 0 {
                    name.to_string()
                } else if c == 0 {
                    "組版データ作成".to_string()
                } else {
                    "1".to_string()
                };
                let tx = if c >= 2 { cx + w - 12.0 } else { cx + 12.0 };
                invoice.add_object(text_obj(
                    &format!("t_{r}_{c}"),
                    &label,
                    tx,
                    y + 42.0,
                    s,
                    None,
                ));
                cx += w;
            }
            y += 60.0;
        }
        // 合計欄
        invoice.add_object(cell("total", 100.0, y, 980.0, 90.0));
        let mut tl = TextStyle::new("Noto Sans JP", 28.0);
        tl.font_weight = 700;
        tl.text_anchor = irasu_illustrator::core::document::TextAnchor::End;
        invoice.add_object(text_obj(
            "合計",
            "合計  ¥120,000-",
            1056.0,
            y + 60.0,
            tl,
            None,
        ));
        // 印影
        invoice.add_object(Object::new_ellipse("印影", 980.0, 260.0, 70.0, 70.0));
    }
    verify(&invoice);
    std::fs::write(
        "assets/templates/invoice-a4.amata",
        serde_json::to_string_pretty(&invoice).unwrap(),
    )
    .unwrap();

    // 5. 年賀状（縦組み・本文ルビ）
    let mut nenga = Document::default();
    nenga.name = "年賀状".into();
    nenga.width = 1000.0; // ハガキ比率 (100×148mm)
    nenga.height = 1480.0;
    nenga.artboards = vec![Artboard::new("ハガキ", 0.0, 0.0, 1000.0, 1480.0)];
    nenga.spots = irasu_illustrator::core::print::default_spots();
    {
        // 枠
        let mut frame = Object::new_rect("枠", 60.0, 60.0, 880.0, 1360.0, 0.0);
        frame.fill = None;
        frame.stroke = Some(StrokeStyle {
            width: 6.0,
            color: [0.65, 0.1, 0.1, 1.0],
            ..Default::default()
        });
        nenga.add_object(frame);
        let mut body = TextStyle::new("Noto Sans JP", 44.0);
        body.vertical = true;
        body.word_wrap = true;
        body.line_height = Some(2.0);
        nenga.add_object(text_obj(
            "本文",
            "｜新年《しんねん》おめでとうございます。\n旧年中は大変お世話になりました。\n｜本年《ほとねん》もどうぞ宜しくお願いいたします。",
            180.0,
            180.0,
            body.clone(),
            Some(TextArea::new(180.0, 180.0, 660.0, 1120.0)),
        ));
        let mut year = TextStyle::new("Noto Sans JP", 120.0);
        year.font_weight = 700;
        year.vertical = true;
        nenga.add_object(text_obj("年", "二〇二六年", 760.0, 200.0, year, None));
        // 干支（午×午）×簡単な円
        nenga.add_object(Object::new_ellipse(
            "干支モチーフ",
            520.0,
            1300.0,
            90.0,
            90.0,
        ));
    }
    verify(&nenga);
    std::fs::write(
        "assets/templates/newyear-card.amata",
        serde_json::to_string_pretty(&nenga).unwrap(),
    )
    .unwrap();

    // 6. SNS縦（1080×1920, 9:16）
    let mut story = Document::default();
    story.name = "SNS縦 (9:16)".into();
    story.width = 1080.0;
    story.height = 1920.0;
    story.artboards = vec![Artboard::new("9:16", 0.0, 0.0, 1080.0, 1920.0)];
    {
        let mut bg = Object::new_rect("背景", 0.0, 0.0, 1080.0, 1920.0, 0.0);
        bg.fill = Some(FillStyle::solid([0.07, 0.09, 0.16, 1.0]));
        story.add_object(bg);
        let mut head = TextStyle::new("Noto Sans JP", 110.0);
        head.font_weight = 700;
        story.add_object(text_obj(
            "見出し",
            "｜新《しん》機能\nOpenType",
            90.0,
            220.0,
            head,
            Some(TextArea::new(90.0, 220.0, 900.0, 420.0)),
        ));
        let sub = TextStyle::new("Noto Sans JP", 52.0);
        story.add_object(text_obj(
            "リード",
            "約物の半角化・縦組グリフ・\n和欧間の自動挿入に対応。",
            90.0,
            720.0,
            sub,
            Some(TextArea::new(90.0, 720.0, 900.0, 240.0)),
        ));
        // 3つの機能カード（横組みルビ付き）
        let mut card_head = TextStyle::new("Noto Sans JP", 60.0);
        card_head.font_weight = 700;
        for (i, (t, d)) in [
            ("palt", "約物を半角に"),
            ("vert", "縦組グリフ"),
            ("ruby", "ルビ注"),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 1100.0 + i as f64 * 250.0;
            let mut c = Object::new_rect(&format!("card_{i}"), 90.0, y, 900.0, 200.0, 24.0);
            c.fill = Some(FillStyle::solid([0.13, 0.16, 0.26, 1.0]));
            story.add_object(c);
            story.add_object(text_obj(
                &format!("card_t_{i}"),
                &format!("｜{t}《タグ》"),
                140.0,
                y + 70.0,
                card_head.clone(),
                None,
            ));
            story.add_object(text_obj(
                &format!("card_d_{i}"),
                d,
                560.0,
                y + 70.0,
                TextStyle::new("Noto Sans JP", 48.0),
                None,
            ));
        }
    }
    verify(&story);
    std::fs::write(
        "assets/templates/story-9x16.amata",
        serde_json::to_string_pretty(&story).unwrap(),
    )
    .unwrap();

    // 7. 方眼ノート A4（方眼＋しおり）
    let mut note = Document::default();
    note.name = "方眼ノート".into();
    note.width = 1240.0;
    note.height = 1754.0;
    note.artboards = vec![Artboard::new("A4", 0.0, 0.0, 1240.0, 1754.0)];
    {
        // 方眼 5mm=20px 相当
        let step = 62.0;
        let mut i = 0;
        let mut x = 0.0;
        while x <= 1240.0 {
            note.add_object(line(&format!("gx_{i}"), x, 0.0, x, 1754.0, 0.5));
            x += step;
            i += 1;
        }
        let mut j = 0;
        let mut y = 0.0;
        while y <= 1754.0 {
            note.add_object(line(&format!("gy_{j}"), 0.0, y, 1240.0, y, 0.5));
            y += step;
            j += 1;
        }
        // ヘッダ（透かし）
        let mut head = TextStyle::new("Noto Sans JP", 40.0);
        head.font_weight = 700;
        note.add_object(text_obj(
            "ヘッダ",
            "｜設計《せっけい》メモ",
            80.0,
            90.0,
            head,
            Some(TextArea::new(80.0, 90.0, 600.0, 120.0)),
        ));
        note.add_object(line("ヘッダ線", 80.0, 190.0, 620.0, 190.0, 2.0));
    }
    verify(&note);
    std::fs::write(
        "assets/templates/notebook-grid.amata",
        serde_json::to_string_pretty(&note).unwrap(),
    )
    .unwrap();

    eprintln!("wrote 7 templates under assets/templates/");
}
