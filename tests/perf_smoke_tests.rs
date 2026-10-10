//! 実データ規模のスモーク性能テスト。
//! しきい値は寛大（CIのブレ防止）で、「壊れて数秒→数十秒」を捕まえる目的。
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object};
use std::time::Instant;

fn big_doc(objects: usize, text_objects: usize) -> Document {
    let mut doc = Document::default();
    doc.width = 4000.0;
    doc.height = 3000.0;
    for i in 0..objects {
        let x = (i % 64) as f64 * 60.0;
        let y = (i / 64) as f64 * 60.0;
        let mut o = Object::new_rect(&format!("r{i}"), x, y, 40.0, 40.0, 0.0);
        o.fill = Some(irasu_illustrator::core::path::FillStyle::solid([
            (i % 8) as f32 / 8.0,
            0.4,
            0.7,
            1.0,
        ]));
        doc.add_object(o);
    }
    for i in 0..text_objects {
        let mut st = irasu_illustrator::core::document::TextStyle::new("Noto Sans JP", 24.0);
        st.vertical = i % 2 == 0;
        st.word_wrap = true;
        st.max_width = Some(300.0);
        doc.add_object(Object::new_text_with_style(
            &format!("t{i}"),
            "｜組版《くみはん》のテストです。AmataとIllustratorを1対4で比較する。",
            (i % 32) as f64 * 120.0,
            (i / 32) as f64 * 200.0,
            st,
        ));
    }
    doc
}

#[test]
fn large_document_exports_in_reasonable_time() {
    let doc = big_doc(600, 60);
    // SVG
    let t0 = Instant::now();
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    let svg_ms = t0.elapsed().as_millis();
    assert!(svg.len() > 10_000, "document produced output");
    println!("SVG export: {svg_ms}ms ({} bytes)", svg.len());
    assert!(
        svg_ms < 20_000,
        "SVG export must not take minutes: {svg_ms}ms"
    );

    // PNG (raster, through resvg)
    let t1 = Instant::now();
    let png = irasu_illustrator::io::raster::export_png(&doc, 0.25, true).expect("png");
    let png_ms = t1.elapsed().as_millis();
    println!("PNG export: {png_ms}ms ({} bytes)", png.len());
    assert!(png_ms < 30_000, "PNG export must complete: {png_ms}ms");

    // Project save/load round-trip
    let t2 = Instant::now();
    let json = serde_json::to_string(&doc).expect("json");
    let back: Document = serde_json::from_str(&json).expect("back");
    assert_eq!(back.layers[0].objects.len(), doc.layers[0].objects.len());
    let io_ms = t2.elapsed().as_millis();
    println!("project round-trip: {io_ms}ms ({} bytes)", json.len());
    assert!(io_ms < 5_000, "project IO: {io_ms}ms");
}

#[test]
fn text_heavy_document_layout_is_linear() {
    // 600個の縦組/横組テキスト。折返し計算が O(n²) になると目に見えて遅い。
    let doc = big_doc(0, 600);
    let t0 = Instant::now();
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    let ms = t0.elapsed().as_millis();
    println!("600 text objects: {ms}ms");
    assert!(svg.contains("text"), "text emitted");
    assert!(ms < 30_000, "layout must stay linear: {ms}ms");
}
