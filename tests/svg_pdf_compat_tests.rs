//! Real-file compat expansion: SVG parse→export round-trips,
//! PDF-import malicious-input guards, and .ai PDF-compat skip reports.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, ObjectType};
use irasu_illustrator::core::path::{FillStyle, FillType};
use irasu_illustrator::io::pdf_import::{parse_ai_bytes, parse_pdf_bytes};
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};

// ---------- minimal PDF builders (mirrors pdf_import_tests) ----------

fn assemble(objs: Vec<Vec<u8>>, info: Option<usize>) -> Vec<u8> {
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objs.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    let mut trailer = format!("trailer\n<< /Size {} /Root 1 0 R", objs.len() + 1);
    if let Some(n) = info {
        trailer.push_str(&format!(" /Info {n} 0 R"));
    }
    out.extend_from_slice(format!("{trailer} >>\nstartxref\n{xref_at}\n%%EOF").as_bytes());
    out
}

fn stream_obj(dict_prefix: &[u8], data: &[u8]) -> Vec<u8> {
    let mut s = dict_prefix.to_vec();
    s.extend_from_slice(format!(" /Length {} >>\nstream\n", data.len()).as_bytes());
    s.extend_from_slice(data);
    s.extend_from_slice(b"\nendstream");
    s
}

fn page_pdf(content_stream: Vec<u8>, page_dict_extra: &str, extra_objs: Vec<Vec<u8>>) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] {page_dict_extra} /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
        )
        .into_bytes(),
        content_stream,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    objs.extend(extra_objs);
    assemble(objs, None)
}

fn simple_content_pdf(content: &[u8]) -> Vec<u8> {
    page_pdf(stream_obj(b"<<", content), "", vec![])
}

fn image_pdf(content: &[u8], image_dict_prefix: &[u8], samples: &[u8]) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>"
            .to_vec(),
        stream_obj(b"<<", content),
    ];
    let mut img = image_dict_prefix.to_vec();
    img.extend_from_slice(format!(" /Length {} >>\nstream\n", samples.len()).as_bytes());
    img.extend_from_slice(samples);
    img.extend_from_slice(b"\nendstream");
    objs.push(img);
    assemble(objs, None)
}

// ---------- SVG round-trips ----------

#[test]
fn svg_roundtrip_viewbox_origin_shift() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="10 20 200 100">
  <rect x="10" y="20" width="50" height="30" fill="#ff0000" />
</svg>"##;
    let doc = parse_svg_document(svg);
    assert!((doc.width - 200.0).abs() < 1e-6, "w={}", doc.width);
    assert!((doc.height - 100.0).abs() < 1e-6, "h={}", doc.height);
    let obj = doc.all_objects().next().expect("rect").1;
    let (mn, mx) = obj.bounding_box().expect("bbox");
    assert!(
        mn.x.abs() < 1e-6 && mn.y.abs() < 1e-6,
        "origin shift {mn:?}"
    );
    assert!(((mx.x - 50.0).abs() < 1e-6) && ((mx.y - 30.0).abs() < 1e-6));
    // Export normalizes viewBox to 0-origin; geometry must survive.
    let back = parse_svg_document(&export_svg(&doc));
    assert!((back.width - 200.0).abs() < 1e-6);
    let obj2 = back.all_objects().next().expect("rect").1;
    let (mn2, mx2) = obj2.bounding_box().expect("bbox");
    assert!((mn2.x - mn.x).abs() < 1e-3 && (mx2.x - mx.x).abs() < 1e-3);
    assert!((mn2.y - mn.y).abs() < 1e-3 && (mx2.y - mx.y).abs() < 1e-3);
}

#[test]
fn svg_roundtrip_viewbox_explicit_size_wins() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 300" width="400" height="300">
  <rect x="0" y="0" width="400" height="300" fill="#00ff00" />
</svg>"##;
    let doc = parse_svg_document(svg);
    assert_eq!((doc.width, doc.height), (400.0, 300.0));
    let out = export_svg(&doc);
    assert!(out.contains("viewBox=\"0 0 400 300\""), "{out}");
    let back = parse_svg_document(&out);
    assert_eq!((back.width, back.height), (400.0, 300.0));
}

#[test]
fn svg_roundtrip_transform_group() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">
  <g transform="translate(10 20)">
    <rect x="0" y="0" width="40" height="10" fill="#123456" />
  </g>
  <g transform="scale(2)">
    <rect x="5" y="5" width="10" height="10" fill="#654321" />
  </g>
</svg>"##;
    let doc = parse_svg_document(svg);
    assert_eq!(doc.all_objects().count(), 2);
    let mut boxes: Vec<_> = doc
        .all_objects()
        .filter_map(|(_, o)| o.bounding_box())
        .collect();
    boxes.sort_by(|a, b| a.0.x.partial_cmp(&b.0.x).unwrap());
    assert!((boxes[0].0.x - 10.0).abs() < 1e-6, "{boxes:?}");
    assert!((boxes[0].0.y - 20.0).abs() < 1e-6, "{boxes:?}");
    assert!((boxes[1].0.x - 10.0).abs() < 1e-6, "{boxes:?}");
    assert!((boxes[1].0.y - 10.0).abs() < 1e-6, "{boxes:?}");
    assert!(((boxes[1].1.x - boxes[1].0.x) - 20.0).abs() < 1e-6);
    // Round-trip preserves translated/scaled geometry.
    let back = parse_svg_document(&export_svg(&doc));
    assert_eq!(back.all_objects().count(), 2);
    let mut boxes2: Vec<_> = back
        .all_objects()
        .filter_map(|(_, o)| o.bounding_box())
        .collect();
    boxes2.sort_by(|a, b| a.0.x.partial_cmp(&b.0.x).unwrap());
    for (a, b) in boxes.iter().zip(boxes2.iter()) {
        assert!((a.0.x - b.0.x).abs() < 1e-3, "{a:?} vs {b:?}");
        assert!((a.0.y - b.0.y).abs() < 1e-3, "{a:?} vs {b:?}");
        assert!((a.1.x - b.1.x).abs() < 1e-3, "{a:?} vs {b:?}");
        assert!((a.1.y - b.1.y).abs() < 1e-3, "{a:?} vs {b:?}");
    }
}

#[test]
fn svg_roundtrip_gradient_stops_survive() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">
  <defs>
    <linearGradient id="lg" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#ff0000" />
      <stop offset="50%" stop-color="#00ff00" />
      <stop offset="100%" stop-color="#0000ff" />
    </linearGradient>
    <radialGradient id="rg" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0%" stop-color="#ffffff" />
      <stop offset="100%" stop-color="#000000" />
    </radialGradient>
  </defs>
  <rect x="0" y="0" width="50" height="50" fill="url(#lg)" />
  <circle cx="75" cy="75" r="20" fill="url(#rg)" />
</svg>"##;
    let doc = parse_svg_document(svg);
    let grads: Vec<_> = doc
        .all_objects()
        .filter_map(|(_, o)| o.fill.clone())
        .collect();
    assert_eq!(grads.len(), 2);
    let out = export_svg(&doc);
    assert!(out.contains("<linearGradient"), "{out}");
    assert!(out.contains("<radialGradient"), "{out}");
    let back = parse_svg_document(&out);
    let fills: Vec<_> = back
        .all_objects()
        .filter_map(|(_, o)| o.fill.clone())
        .collect();
    assert_eq!(fills.len(), 2, "both gradients survive");
    let mut saw_linear = false;
    let mut saw_radial = false;
    for f in &fills {
        match &f.fill_type {
            FillType::Linear(g) => {
                saw_linear = true;
                assert_eq!(g.stops.len(), 3, "stop count");
                assert!(g.stops[0].color[0] > 0.9);
                assert!(g.stops[2].color[2] > 0.9);
            }
            FillType::Radial(g) => {
                saw_radial = true;
                assert_eq!(g.stops.len(), 2);
            }
            other => panic!("unexpected fill {other:?}"),
        }
    }
    assert!(saw_linear && saw_radial);
}

#[test]
fn svg_roundtrip_text_style_survives() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 100" width="300" height="100">
  <text x="10" y="40" font-size="24" font-family="Helvetica" font-weight="700" text-anchor="middle" letter-spacing="2">Hello</text>
</svg>"##;
    let doc = parse_svg_document(svg);
    let (text, size, weight) = doc
        .all_objects()
        .find_map(|(_, o)| match &o.object_type {
            ObjectType::Text { text, style, .. } => {
                Some((text.clone(), style.font_size, style.font_weight))
            }
            _ => None,
        })
        .expect("text");
    assert_eq!(text, "Hello");
    assert!((size - 24.0).abs() < 1e-6);
    assert_eq!(weight, 700);
    let out = export_svg(&doc);
    assert!(out.contains("Hello"), "{out}");
    assert!(out.contains("font-weight=\"700\""), "{out}");
    assert!(out.contains("text-anchor=\"middle\""), "{out}");
    let back = parse_svg_document(&out);
    let (text2, style2) = back
        .all_objects()
        .find_map(|(_, o)| match &o.object_type {
            ObjectType::Text { text, style, .. } => Some((text.clone(), style.clone())),
            _ => None,
        })
        .expect("text survives");
    assert_eq!(text2, "Hello");
    assert!((style2.font_size - 24.0).abs() < 1e-6);
    assert_eq!(style2.font_weight, 700);
    assert_eq!(
        style2.text_anchor,
        irasu_illustrator::core::document::TextAnchor::Middle
    );
    assert!((style2.letter_spacing - 2.0).abs() < 1e-6);
}

#[test]
fn svg_roundtrip_clip_path_with_gradient_and_text() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#ff0000" />
      <stop offset="100%" stop-color="#0000ff" />
    </linearGradient>
    <clipPath id="cp"><rect x="10" y="10" width="100" height="100" /></clipPath>
  </defs>
  <g clip-path="url(#cp)">
    <rect x="0" y="0" width="200" height="200" fill="url(#g)" />
    <text x="20" y="60" font-size="20" fill="#ffffff">ClipMe</text>
  </g>
</svg>"##;
    let doc = parse_svg_document(svg);
    let masks = doc
        .all_objects()
        .filter(|(_, o)| matches!(o.object_type, ObjectType::ClippingMask { .. }))
        .count();
    assert_eq!(masks, 1, "clip group imports as mask");
    let out = export_svg(&doc);
    assert!(out.contains("<clipPath"), "{out}");
    assert!(out.contains("clip-path="), "{out}");
    assert!(out.contains("ClipMe"), "{out}");
    let back = parse_svg_document(&out);
    let masks2 = back
        .all_objects()
        .filter(|(_, o)| matches!(o.object_type, ObjectType::ClippingMask { .. }))
        .count();
    assert_eq!(masks2, 1, "mask survives round-trip");
    let texts: Vec<_> = back
        .all_objects()
        .filter_map(|(_, o)| match &o.object_type {
            ObjectType::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    // Text inside a clip group nests under the mask wrapper.
    fn collect_text(o: &irasu_illustrator::core::document::Object, out: &mut Vec<String>) {
        match &o.object_type {
            ObjectType::Text { text, .. } => out.push(text.clone()),
            ObjectType::ClippingMask { children } | ObjectType::Group(children) => {
                for c in children {
                    collect_text(c, out);
                }
            }
            _ => {}
        }
    }
    let mut nested = Vec::new();
    for layer in &back.layers {
        for o in &layer.objects {
            collect_text(o, &mut nested);
        }
    }
    assert!(
        nested.iter().any(|t| t == "ClipMe"),
        "{texts:?} / {nested:?}"
    );
}

// ---------- PDF malicious-input guards ----------

#[test]
fn pdf_import_skips_jpx_images_explicitly() {
    let pdf = image_pdf(
        b"q 2 0 0 2 10 80 cm /Im1 Do Q\n",
        b"<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /JPXDecode",
        &[0, 1, 2, 3, 4, 5],
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("jpx import");
    assert_eq!(doc.all_objects().count(), 0);
    assert!(
        warnings.iter().any(|w| w.contains("JPEG2000")),
        "{warnings:?}"
    );
}

#[test]
fn pdf_import_skips_jbig2_images_explicitly() {
    let pdf = image_pdf(
        b"q 2 0 0 2 10 80 cm /Im1 Do Q\n",
        b"<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 1 /Filter /JBIG2Decode",
        &[0xFF],
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("jbig2 import");
    assert_eq!(doc.all_objects().count(), 0);
    assert!(warnings.iter().any(|w| w.contains("JBIG2")), "{warnings:?}");
}

#[test]
fn pdf_import_truncates_deep_q_nesting() {
    let mut content = vec![0u8; 0];
    for _ in 0..80 {
        content.extend_from_slice(b"q\n");
    }
    content.extend_from_slice(b"1 0 0 rg 10 10 50 30 re f\n");
    for _ in 0..80 {
        content.extend_from_slice(b"Q\n");
    }
    let pdf = simple_content_pdf(&content);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("deep q imports");
    assert_eq!(doc.all_objects().count(), 1, "{warnings:?}");
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("ネスト") || w.contains("グラフィック状態")),
        "{warnings:?}"
    );
}

#[test]
fn pdf_import_truncates_self_recursive_form() {
    // Page paints /Fm1; Form 5 0 R paints itself -> depth cap must stop it.
    let page_content = stream_obj(b"<<", b"/Fm1 Do\n");
    let form_inner = b"/Fm1 Do\n";
    let form = stream_obj(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 50 50] /Matrix [1 0 0 1 0 0] /Resources << /XObject << /Fm1 5 0 R >> >>",
        form_inner,
    );
    let objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /XObject << /Fm1 5 0 R >> >> >>"
            .to_vec(),
        page_content,
        form,
    ];
    let pdf = assemble(objs, None);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("recursive form imports");
    assert!(
        warnings.iter().any(|w| w.contains("Form")),
        "recursion reported: {warnings:?}"
    );
    let _ = doc;
}

#[test]
fn pdf_import_truncates_huge_op_stream() {
    // 2M+ trivial ops exceed MAX_OPS; import must stop with a warning, not OOM.
    let mut content = Vec::with_capacity(4_100_000);
    for _ in 0..2_000_050 {
        content.extend_from_slice(b"n\n");
    }
    let pdf = simple_content_pdf(&content);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("huge op stream imports");
    assert!(
        warnings.iter().any(|w| w.contains("演算子が多すぎ")),
        "{warnings:?}"
    );
    let _ = doc;
}

#[test]
fn pdf_import_skips_absurd_image_dimensions() {
    // 5000x5000 = 25M px > 16M cap: warn-skip, whole import still Ok.
    let pdf = image_pdf(
        b"q 2 0 0 2 10 80 cm /Im1 Do Q\n",
        b"<< /Type /XObject /Subtype /Image /Width 5000 /Height 5000 /ColorSpace /DeviceGray /BitsPerComponent 8",
        &[0x80],
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("absurd image imports");
    assert_eq!(doc.all_objects().count(), 0);
    assert!(
        warnings.iter().any(|w| w.contains("異常な画像サイズ")),
        "{warnings:?}"
    );
}

#[test]
fn pdf_import_rejects_broken_flate_gracefully() {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;
    // Declared FlateDecode but payload is not zlib: lopdf yields empty
    // output instead of an error, so the import stays Ok but must warn
    // explicitly (never a panic, never silent).
    let raw = b"1 0 0 rg 10 10 50 30 re f\n";
    let content_stream = stream_obj(b"<< /Filter /FlateDecode", raw);
    let pdf = page_pdf(content_stream, "", vec![]);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("broken flate stays graceful");
    assert_eq!(doc.all_objects().count(), 0);
    assert!(
        warnings.iter().any(|w| w.contains("展開できない")),
        "{warnings:?}"
    );

    // Highly compressible 8MB expansion stays under the 128MB cap: imports fine.
    let big: Vec<u8> = b"1 0 0 rg 10 10 50 30 re f\n".repeat(400_000);
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(&big).unwrap();
    let compressed = enc.finish().unwrap();
    assert!(compressed.len() < big.len() / 10, "actually compressible");
    let content_stream = stream_obj(b"<< /Filter /FlateDecode", &compressed);
    let pdf = page_pdf(content_stream, "", vec![]);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("flate expansion imports");
    assert!(doc.all_objects().count() >= 1, "{warnings:?}");
}

// ---------- .ai PDF-compat skip reports ----------

#[test]
fn ai_import_reports_producer_version_and_matches_pdf() {
    let content = b"1 0 0 rg\n10 10 50 30 re f\n";
    let objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        stream_obj(b"<<", content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        b"<< /Producer (Adobe Illustrator(R) 27.1) >>".to_vec(),
    ];
    let n = objs.len();
    let pdf = assemble(objs, Some(n));
    let (ai_doc, ai_warnings) = parse_ai_bytes(&pdf).expect("ai import");
    let (pdf_doc, _) = parse_pdf_bytes(&pdf).expect("pdf import");
    assert_eq!(ai_doc.all_objects().count(), pdf_doc.all_objects().count());
    assert!(
        ai_warnings.first().is_some_and(|w| w.contains("スキップ")),
        "{ai_warnings:?}"
    );
    assert!(
        ai_warnings.first().unwrap().contains("27.1"),
        "{ai_warnings:?}"
    );
}

#[test]
fn ai_import_generic_skip_without_creator_or_producer() {
    let content = b"0 0 1 RG\n2 w\n0 0 m 200 100 l S\n";
    let objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        stream_obj(b"<<", content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        b"<< /Title (No Illustrator marks here) >>".to_vec(),
    ];
    let n = objs.len();
    let pdf = assemble(objs, Some(n));
    let (doc, warnings) = parse_ai_bytes(&pdf).expect("ai import");
    assert_eq!(doc.all_objects().count(), 1);
    assert!(
        warnings.first().is_some_and(|w| w.contains("スキップ")),
        "{warnings:?}"
    );
}

#[test]
fn ai_import_rejects_truncated_or_non_pdf() {
    assert!(parse_ai_bytes(b"").is_err());
    assert!(parse_ai_bytes(b"%PDF").is_err());
    assert!(parse_ai_bytes(b"%PDF-1.4\n%garbage without xref").is_err());
}

// ---------- pdf / pdf_print compat ----------

#[test]
fn pdf_export_reimports_rect_fill_and_text() {
    let mut doc = Document::default();
    doc.width = 300.0;
    doc.height = 200.0;
    let mut rect =
        irasu_illustrator::core::document::Object::new_rect("R", 10.0, 10.0, 80.0, 40.0, 0.0);
    rect.fill = Some(FillStyle::solid([0.0, 1.0, 0.0, 1.0]));
    doc.add_object(rect);
    doc.add_object(irasu_illustrator::core::document::Object::new_text(
        "T", "Hi", 5.0, 50.0, 20.0,
    ));
    let pdf = irasu_illustrator::io::pdf::export_pdf(&doc);
    assert!(pdf.starts_with(b"%PDF"));
    let (back, _) = parse_pdf_bytes(&pdf).expect("pdf round-trip");
    assert!((back.width - 300.0).abs() < 1e-6);
    assert!((back.height - 200.0).abs() < 1e-6);
    assert!(back.all_objects().count() >= 2);
    let green = back
        .all_objects()
        .map(|(_, o)| o)
        .filter(|o| matches!(o.object_type, ObjectType::Path(_)))
        .filter_map(|o| o.fill.clone())
        .any(|f| f.color[1] > 0.9);
    assert!(green, "green fill survives");
}

#[test]
fn pdf_print_export_reimports_without_panic() {
    use irasu_illustrator::io::pdf_print::{export_pdf_print, PrintPdfOptions};
    let mut doc = Document::default();
    doc.width = 200.0;
    doc.height = 100.0;
    let mut rect =
        irasu_illustrator::core::document::Object::new_rect("R", 10.0, 10.0, 80.0, 40.0, 0.0);
    rect.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
    doc.add_object(rect);
    doc.add_object(irasu_illustrator::core::document::Object::new_text(
        "T", "Press", 10.0, 60.0, 18.0,
    ));
    let opts = PrintPdfOptions {
        marks: false,
        bleed: Some(0.0),
        pdfx: false,
        outline_text: false,
    };
    let (pdf, _) = export_pdf_print(&doc, &opts);
    assert!(pdf.starts_with(b"%PDF"));
    // Press output must at least parse as PDF and re-import vector content.
    let lopdf_doc = lopdf::Document::load_mem(&pdf).expect("press pdf parses");
    assert_eq!(lopdf_doc.get_pages().len(), 1);
    let (back, _) = parse_pdf_bytes(&pdf).expect("press re-import");
    assert!(back.all_objects().count() >= 1);
}
