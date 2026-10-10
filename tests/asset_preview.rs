//! 自作テンプレートの目視確認用レンダリング。
//! 各テンプレートを1枚のプレビューに並べて書き出す:
//! `cargo test --test asset_preview -- --ignored --nocapture`
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::io::library::built_in_templates;

/// Scale a source document's top-level objects into a tile of the preview.
/// Groups compose their children through their own transform, so scaling
/// the top-level transform scales the whole subtree.
fn push_tile(
    out: &mut irasu_illustrator::core::document::Document,
    src: &irasu_illustrator::io::library::Template,
    ox: f64,
    oy: f64,
    s: f64,
) {
    for layer in &src.document.layers {
        for obj in &layer.objects {
            let mut o = obj.clone();
            if o.transform.x.is_finite() {
                o.transform.x = ox + o.transform.x * s;
            } else {
                o.transform.x = ox;
            }
            if o.transform.y.is_finite() {
                o.transform.y = oy + o.transform.y * s;
            } else {
                o.transform.y = oy;
            }
            o.transform.scale_x *= s;
            o.transform.scale_y *= s;
            out.add_object(o);
        }
    }
}

#[test]
#[ignore = "manual: renders every bundled template to output/templates-preview.png"]
fn render_all_templates() {
    let templates = built_in_templates();
    let tile_w = 620.0_f64;
    let gap = 32.0;
    let cols = 2.0;

    let mut doc = irasu_illustrator::core::document::Document::default();
    doc.name = "テンプレートプレビュー".into();
    doc.width = tile_w * cols + gap * (cols - 1.0);

    let mut x = 0.0;
    let mut y = 0.0;
    let mut row_h = 0.0;
    for (i, t) in templates.iter().enumerate() {
        let s = tile_w / t.size.0;
        let h = t.size.1 * s;
        if i > 0 && (i as f64 % cols) == 0.0 {
            x = 0.0;
            y += row_h + gap;
            row_h = 0.0;
        }
        push_tile(&mut doc, t, x, y, s);
        x += tile_w + gap;
        row_h = row_h.max(h);
    }
    doc.height = y + row_h;

    // White sheet behind everything.
    let mut bg = irasu_illustrator::core::document::Object::new_rect(
        "bg", 0.0, 0.0, doc.width, doc.height, 0.0,
    );
    bg.fill = Some(irasu_illustrator::core::path::FillStyle::solid([
        0.94, 0.94, 0.95, 1.0,
    ]));
    doc.layers[0].objects.insert(0, bg);

    let png = irasu_illustrator::io::raster::export_png(&doc, 0.5, false).expect("png");
    std::fs::create_dir_all("output").unwrap();
    std::fs::write("output/templates-preview.png", &png).unwrap();
    eprintln!(
        "wrote output/templates-preview.png ({} bytes, {} templates, {}x{})",
        png.len(),
        templates.len(),
        doc.width,
        doc.height
    );
}
