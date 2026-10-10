//! PDF import: hand-built fixture + exporter round-trip.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::path::FillStyle;
use irasu_illustrator::io::pdf_import::{parse_ai_bytes, parse_pdf_bytes};

/// Assemble a minimal one-page PDF with correct xref offsets.
fn build_pdf(content: &[u8]) -> Vec<u8> {
    build_pdf_with_info(content, None)
}

/// Same, plus an Info dict (used to mimic Illustrator-saved files).
fn build_pdf_with_info(content: &[u8], info: Option<&[u8]>) -> Vec<u8> {
    build_pdf_with_page_and_info(content, b"", info)
}

fn build_pdf_with_page_and_info(content: &[u8], page_attrs: &[u8], info: Option<&[u8]>) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objs.push(
        [
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] ".as_slice(),
            page_attrs,
            b" /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        ]
        .concat(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objs.push(stream);
    objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
    if let Some(info_body) = info {
        objs.push(info_body.to_vec());
    }
    assemble_pdf(objs, info.is_some())
}

fn assemble_pdf(objs: Vec<Vec<u8>>, has_info_last: bool) -> Vec<u8> {
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
    if has_info_last {
        trailer.push_str(&format!(" /Info {} 0 R", objs.len()));
    }
    out.extend_from_slice(format!("{trailer} >>\nstartxref\n{xref_at}\n%%EOF").as_bytes());
    out
}

/// Minimal page painting one raw-sample image XObject.
fn build_image_pdf(
    content: &[u8],
    image_dict: &[u8],
    samples: &[u8],
    extra_objs: &[&[u8]],
) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objs.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objs.push(stream);
    let mut img = image_dict.to_vec();
    img.extend_from_slice(format!(" /Length {} >>\nstream\n", samples.len()).as_bytes());
    img.extend_from_slice(samples);
    img.extend_from_slice(b"\nendstream");
    objs.push(img);
    objs.extend(extra_objs.iter().map(|b| b.to_vec()));
    assemble_pdf(objs, false)
}

/// Minimal page with an `/ExtGState` resource.
fn build_gs_pdf(content: &[u8], gs_dict: &[u8]) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objs.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /ExtGState << /GS1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objs.push(stream);
    objs.push(gs_dict.to_vec());
    assemble_pdf(objs, false)
}

/// Minimal Type0 (CID) page with an explicit `/ToUnicode` CMap.
fn build_cid_pdf(content: &[u8], cmap: &[u8]) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objs.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objs.push(stream);
    objs.push(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /TestCID /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >>"
            .to_vec(),
    );
    objs.push(
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /TestCID /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> >>"
            .to_vec(),
    );
    let mut cmap_stream = format!("<< /Length {} >>\nstream\n", cmap.len()).into_bytes();
    cmap_stream.extend_from_slice(cmap);
    cmap_stream.extend_from_slice(b"\nendstream");
    objs.push(cmap_stream);
    assemble_pdf(objs, false)
}

/// Same Type0 setup but without `/ToUnicode` (missing-mapping path).
fn build_cid_pdf_without_tounicode(content: &[u8]) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objs.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objs.push(stream);
    objs.push(
        b"<< /Type /Font /Subtype /Type0 /BaseFont /TestCID /Encoding /Identity-H /DescendantFonts [6 0 R] >>"
            .to_vec(),
    );
    objs.push(
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /TestCID /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> >>"
            .to_vec(),
    );
    assemble_pdf(objs, false)
}

fn pdf_texts(doc: &Document) -> Vec<String> {
    doc.all_objects()
        .filter_map(|(_, o)| match &o.object_type {
            ObjectType::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Minimal page with a `/Shading` resource (axial shading test).
fn build_shading_pdf(content: &[u8], shading: &[u8], func: &[u8]) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objs.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /Shading << /Sh1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objs.push(stream);
    objs.push(shading.to_vec());
    objs.push(func.to_vec());
    assemble_pdf(objs, false)
}

/// Minimal page with a stream-backed (mesh) `/Shading` resource.
fn build_mesh_pdf(content: &[u8], shading_dict_prefix: &[u8], data: &[u8]) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objs.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /Shading << /Sh1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objs.push(stream);
    let mut shading = shading_dict_prefix.to_vec();
    shading.extend_from_slice(format!(" /Length {} >>\nstream\n", data.len()).as_bytes());
    shading.extend_from_slice(data);
    shading.extend_from_slice(b"\nendstream");
    objs.push(shading);
    assemble_pdf(objs, false)
}

/// Minimal page with a Type 3 font (dict + encoding + charprocs + glyph).
fn build_type3_pdf(
    content: &[u8],
    font: &[u8],
    encoding: &[u8],
    charprocs: &[u8],
    glyph: &[u8],
) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objs.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objs.push(stream);
    objs.push(font.to_vec());
    objs.push(encoding.to_vec());
    objs.push(charprocs.to_vec());
    let mut glyph_stream = format!("<< /Length {} >>\nstream\n", glyph.len()).into_bytes();
    glyph_stream.extend_from_slice(glyph);
    glyph_stream.extend_from_slice(b"\nendstream");
    objs.push(glyph_stream);
    assemble_pdf(objs, false)
}

#[test]
fn pdf_import_honors_crop_box_and_quarter_turn_rotation() {
    let pdf = build_pdf_with_page_and_info(
        b"1 0 0 rg 20 30 10 15 re f\n",
        b"/CropBox [10 20 110 70] /Rotate 90",
        None,
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("rotated cropped page imports");
    assert!(
        warnings.is_empty(),
        "unexpected import warning: {warnings:?}"
    );
    assert_eq!((doc.width, doc.height), (50.0, 100.0));
    let object = doc.all_objects().next().unwrap().1;
    let (min, max) = object.bounding_box().unwrap();
    // Crop origin is removed, then clockwise rotation maps PDF (20,30)..
    // (30,45) to canvas x=10..25, y=10..20.
    assert!((min.x - 10.0).abs() < 1e-6 && (max.x - 25.0).abs() < 1e-6);
    assert!((min.y - 10.0).abs() < 1e-6 && (max.y - 20.0).abs() < 1e-6);
}

#[test]
fn pdf_import_preserves_clip_paths_and_q_q_state() {
    let pdf = build_pdf(b"q 20 20 30 30 re W n 0 0 100 100 re f Q 0 0 100 100 re f\n");
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("clipped page imports");
    assert!(
        warnings.is_empty(),
        "unexpected import warning: {warnings:?}"
    );
    let objects: Vec<_> = doc.all_objects().map(|(_, object)| object).collect();
    assert_eq!(objects.len(), 2);
    let ObjectType::ClippingMask { children } = &objects[0].object_type else {
        panic!("first path should retain its active PDF clip");
    };
    assert_eq!(children.len(), 2, "mask shape followed by clipped artwork");
    assert!(matches!(children[0].object_type, ObjectType::Path(_)));
    assert!(matches!(children[1].object_type, ObjectType::Path(_)));
    assert!(
        matches!(objects[1].object_type, ObjectType::Path(_)),
        "Q restores unclipped state"
    );

    let (mask_min, mask_max) = children[0].bounding_box().unwrap();
    assert!((mask_min.x - 20.0).abs() < 1e-6);
    assert!((mask_max.x - 50.0).abs() < 1e-6);
}

#[test]
fn pdf_import_keeps_even_odd_clip_rule() {
    let pdf = build_pdf(b"0 0 80 80 re W* n 0 0 100 100 re f\n");
    let (doc, _) = parse_pdf_bytes(&pdf).expect("even-odd clip imports");
    let clipped = doc.all_objects().next().unwrap().1;
    let ObjectType::ClippingMask { children } = &clipped.object_type else {
        panic!("expected clipping mask wrapper");
    };
    let mask = children[0].fill.as_ref().unwrap();
    assert_eq!(mask.rule, irasu_illustrator::core::path::FillRule::EvenOdd);
}
#[test]
fn test_ai_import_reads_pdf_compatible_part() {
    let content = b"1 0 0 rg\n10 10 50 30 re f\n";
    let info = b"<< /Creator (Adobe Illustrator(R) 28.0) /Producer (Adobe PDF library) >>";
    let ai = build_pdf_with_info(content, Some(info));
    let (doc, warnings) = parse_ai_bytes(&ai).expect("ai import");
    assert_eq!(doc.all_objects().count(), 1);
    assert!(
        warnings.first().is_some_and(|w| w.contains("Illustrator")),
        "skip report first: {warnings:?}"
    );
    assert!(
        warnings.first().unwrap().contains("28.0"),
        "version surfaced: {warnings:?}"
    );
}

#[test]
fn test_ai_rejects_non_pdf_bytes() {
    assert!(parse_ai_bytes(b"definitely not a pdf").is_err());
    assert!(parse_ai_bytes(b"").is_err());
    assert!(parse_ai_bytes(b"%PDF").is_err());
}

#[test]
fn test_ai_without_creator_still_imports() {
    let content = b"0 0 1 RG\n2 w\n0 0 m 200 100 l S\n";
    let ai = build_pdf_with_info(content, None);
    let (doc, warnings) = parse_ai_bytes(&ai).expect("ai import");
    assert_eq!(doc.all_objects().count(), 1);
    assert!(
        warnings.first().is_some_and(|w| w.contains("スキップ")),
        "generic skip report: {warnings:?}"
    );
}

#[test]
fn test_import_paths_text_and_colors() {
    let mut content = b"1 0 0 rg\n10 10 50 30 re f\n0 0 1 RG\n2 w\n0 0 m 200 100 l S\nBT /F1 12 Tf 20 80 Td (Hello) Tj ET\n".to_vec();
    // WinAnsi high byte: (H\xe9llo) -> Héllo.
    content.extend_from_slice(b"BT /F1 12 Tf 20 60 Td (H\xe9llo) Tj ET\n");
    let pdf = build_pdf(&content);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert!((doc.width - 200.0).abs() < 1e-6);
    assert!((doc.height - 100.0).abs() < 1e-6);

    let objs: Vec<_> = doc.all_objects().map(|(_, o)| o).collect();
    assert_eq!(objs.len(), 4, "rect + line + 2 texts");

    // Red filled rect: PDF (10,10)-(60,40) -> canvas y-flip (60..90).
    let rect = objs
        .iter()
        .find(|o| o.name.starts_with("PDF Path"))
        .expect("rect");
    let (mn, mx) = rect.bounding_box().expect("bbox");
    assert!((mn.x - 10.0).abs() < 1e-6 && (mx.x - 60.0).abs() < 1e-6);
    assert!((mn.y - 60.0).abs() < 1e-6 && (mx.y - 90.0).abs() < 1e-6);
    let fill = rect.fill.as_ref().expect("fill");
    assert!(
        fill.color[0] > 0.9 && fill.color[1] < 0.1,
        "red fill: {:?}",
        fill.color
    );

    // Blue stroked diagonal.
    let line = objs
        .iter()
        .filter(|o| o.name.starts_with("PDF Path"))
        .nth(1)
        .expect("line");
    let stroke = line.stroke.as_ref().expect("stroke");
    assert!(stroke.color[2] > 0.9, "blue stroke");
    assert!((stroke.width - 2.0).abs() < 1e-6);

    // Text runs with positions, size and WinAnsi decoding.
    let texts: Vec<&&Object> = objs
        .iter()
        .filter(|o| matches!(o.object_type, ObjectType::Text { .. }))
        .collect();
    assert_eq!(texts.len(), 2);
    if let ObjectType::Text { text, style, .. } = &texts[0].object_type {
        assert_eq!(text, "Hello");
        assert!((style.font_size - 12.0).abs() < 1e-6);
        assert_eq!(style.font_family, "Helvetica");
    } else {
        unreachable!();
    }
    assert!((texts[0].transform.x - 20.0).abs() < 1e-6);
    assert!(
        (texts[0].transform.y - 20.0).abs() < 1e-6,
        "y-flipped baseline"
    );
    if let ObjectType::Text { text, .. } = &texts[1].object_type {
        assert_eq!(text, "Héllo", "WinAnsi 0xE9 decodes");
    } else {
        unreachable!();
    }
}

#[test]
fn test_import_rejects_garbage() {
    assert!(parse_pdf_bytes(b"definitely not a pdf").is_err());
    assert!(parse_pdf_bytes(b"").is_err());
}

#[test]
fn test_broken_xref_is_repaired() {
    // startxref zeroed: direct load fails, object scan salvages it.
    let mut pdf = build_pdf(b"0 0 m 10 10 l S\n");
    if let Some(pos) = pdf.windows(9).rposition(|w| w == b"startxref") {
        let num_start = pos + 10;
        let num_end = pdf[num_start..].iter().position(|&b| b == b'\n').unwrap() + num_start;
        for b in pdf[num_start..num_end].iter_mut() {
            if b.is_ascii_digit() {
                *b = b'0';
            }
        }
    }
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("repaired import");
    assert_eq!(doc.all_objects().count(), 1);
    assert!(
        warnings.iter().any(|w| w.contains("修復")),
        "repair reported"
    );
    // Whole xref section dropped instead: same salvage path.
    let mut cut = build_pdf(b"0 0 m 10 10 l S\n");
    if let Some(xref) = cut.windows(5).position(|w| w == b"xref\n") {
        cut.truncate(xref);
        cut.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n0\n%%EOF\n");
    }
    let (doc2, _) = parse_pdf_bytes(&cut).expect("repaired import");
    assert_eq!(doc2.all_objects().count(), 1);
}

#[test]
fn test_exporter_round_trip() {
    // Our own exporter output must reimport without errors.
    let mut doc = Document::default();
    doc.width = 300.0;
    doc.height = 200.0;
    let mut rect = Object::new_rect("R", 10.0, 10.0, 80.0, 40.0, 0.0);
    rect.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
    doc.add_object(rect);
    doc.add_object(Object::new_text("T", "Hi", 5.0, 50.0, 20.0));
    let pdf = irasu_illustrator::io::pdf::export_pdf(&doc);
    assert!(pdf.starts_with(b"%PDF"));
    let (back, warnings) = parse_pdf_bytes(&pdf).expect("round-trip import");
    assert!((back.width - 300.0).abs() < 1e-6);
    assert!((back.height - 200.0).abs() < 1e-6);
    let n = back.all_objects().count();
    assert!(
        n >= 2,
        "rect + text survive, got {n} (warnings: {warnings:?})"
    );
    // Red rect keeps its fill through the round trip.
    let red = back
        .all_objects()
        .map(|(_, o)| o)
        .filter(|o| matches!(o.object_type, ObjectType::Path(_)))
        .filter_map(|o| o.fill.clone())
        .any(|f| f.color[0] > 0.9);
    assert!(red, "red fill survives");
}

#[test]
fn pdf_import_decodes_cid_via_tounicode_bfchar() {
    let cmap = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n2 beginbfchar\n<0000> <3042>\n<0001> <30A2>\nendbfchar\nendcmap\n";
    let pdf = build_cid_pdf(b"BT /F1 12 Tf 20 80 Td <00000001> Tj ET\n", cmap);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("cid import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(pdf_texts(&doc), vec!["\u{3042}\u{30A2}".to_string()]);
}

#[test]
fn pdf_import_decodes_cid_via_tounicode_bfrange() {
    let cmap = b"begincmap\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n1 beginbfrange\n<0002> <0004> <0041>\nendbfrange\n1 beginbfrange\n<0010> <0011> [<3042> <3044>] \nendbfrange\nendcmap\n";
    let pdf = build_cid_pdf(
        b"BT /F1 12 Tf 20 80 Td <00020003000400100011> Tj ET\n",
        cmap,
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("cid bfrange import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(pdf_texts(&doc), vec!["ABC\u{3042}\u{3044}".to_string()]);
}

#[test]
fn pdf_import_marks_unmapped_and_missing_tounicode_cids() {
    // CID 0x0009 has no entry: placeholder + explicit warning.
    let cmap = b"begincmap\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n1 beginbfchar\n<0000> <0041>\nendbfchar\nendcmap\n";
    let pdf = build_cid_pdf(b"BT /F1 12 Tf 20 80 Td <00000009> Tj ET\n", cmap);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("partial cid import");
    assert_eq!(pdf_texts(&doc), vec!["A\u{FFFD}".to_string()]);
    assert!(
        warnings.iter().any(|w| w.contains("ToUnicode")),
        "{warnings:?}"
    );

    // No /ToUnicode at all: every CID becomes a placeholder with a warning.
    let bare = build_cid_pdf_without_tounicode(b"BT /F1 12 Tf 20 80 Td <00000001> Tj ET\n");
    let (doc2, warnings2) = parse_pdf_bytes(&bare).expect("bare cid import");
    assert_eq!(pdf_texts(&doc2), vec!["\u{FFFD}\u{FFFD}".to_string()]);
    assert!(
        warnings2.iter().any(|w| w.contains("ToUnicode")),
        "{warnings2:?}"
    );
}

#[test]
fn pdf_import_paints_axial_shading_as_gradient() {
    use irasu_illustrator::core::path::FillType;
    let shading = b"<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [10 80 60 80] /BBox [10 70 60 90] /Function 6 0 R /Extend [true true] >>";
    let func = b"<< /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >>";
    let pdf = build_shading_pdf(b"/Sh1 sh\n", shading, func);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("shading import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(doc.all_objects().count(), 1);
    let obj = doc.all_objects().next().unwrap().1;
    let (mn, mx) = obj.bounding_box().unwrap();
    // BBox [10 70 60 90] through the page y-flip lands at y 10..30.
    assert!((mn.x - 10.0).abs() < 1e-6 && (mx.x - 60.0).abs() < 1e-6);
    assert!((mn.y - 10.0).abs() < 1e-6 && (mx.y - 30.0).abs() < 1e-6);
    let fill = obj.fill.as_ref().expect("gradient fill");
    let FillType::Linear(grad) = &fill.fill_type else {
        panic!("axial shading becomes a linear gradient");
    };
    assert_eq!(grad.stops.len(), 16);
    assert!(grad.stops.first().unwrap().color[0] > 0.9);
    assert!(grad.stops.last().unwrap().color[2] > 0.9);
    assert!((grad.start_x - 0.0).abs() < 1e-6);
    assert!((grad.end_x - 1.0).abs() < 1e-6);
}

#[test]
fn pdf_import_paints_radial_shading_as_gradient() {
    use irasu_illustrator::core::document::ObjectType;
    use irasu_illustrator::core::path::FillType;
    let shading = b"<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [35 95 0 35 95 25] /BBox [10 70 60 120] /Function 6 0 R /Extend [true true] >>";
    let func = b"<< /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >>";
    let pdf = build_shading_pdf(b"/Sh1 sh\n", shading, func);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("radial shading import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    // Square paint clipped to the /BBox: mask first, gradient content second.
    assert_eq!(doc.all_objects().count(), 1);
    let obj = doc.all_objects().next().unwrap().1;
    let ObjectType::ClippingMask { children } = &obj.object_type else {
        panic!("radial square paint must carry its /BBox clip");
    };
    assert_eq!(children.len(), 2);
    let content = &children[1];
    let (mn, mx) = content.bounding_box().unwrap();
    assert!((mn.x - 10.0).abs() < 1e-6 && (mx.x - 60.0).abs() < 1e-6);
    assert!(
        (mx.x - mn.x - (mx.y - mn.y)).abs() < 1e-6,
        "square paint keeps circles circular"
    );
    let fill = content.fill.as_ref().expect("gradient fill");
    let FillType::Radial(grad) = &fill.fill_type else {
        panic!("radial shading becomes a radial gradient");
    };
    assert_eq!(grad.stops.len(), 16);
    assert!((grad.center_x - 0.5).abs() < 1e-6);
    assert!((grad.radius - 0.5).abs() < 1e-6);
    assert!(grad.stops.first().unwrap().color[0] > 0.9);
    assert!(grad.stops.last().unwrap().color[2] > 0.9);
}

#[test]
fn pdf_import_skips_unsupported_shading_explicitly() {
    let shading = b"<< /ShadingType 4 /ColorSpace /DeviceRGB /BBox [10 70 60 90] >>";
    let func = b"<< /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >>";
    let pdf = build_shading_pdf(b"/Sh1 sh\n", shading, func);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("mesh shading import");
    assert_eq!(doc.all_objects().count(), 0);
    assert!(
        warnings.iter().any(|w| w.contains("シェーディング")),
        "{warnings:?}"
    );
}

#[test]
fn pdf_import_restores_icc_based_image_via_alternate() {
    use irasu_illustrator::core::document::ObjectType;
    // 2x1 raw RGB samples tagged with an sRGB ICC profile.
    let samples = [255, 0, 0, 0, 255, 0];
    let pdf = build_image_pdf(
        b"q 2 0 0 2 10 80 cm /Im1 Do Q\n",
        b"<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace [/ICCBased 6 0 R] /BitsPerComponent 8",
        &samples,
        &[b"<< /N 3 /Alternate /DeviceRGB /Length 0 >>\nstream\n\nendstream"],
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("icc image import");
    assert_eq!(doc.all_objects().count(), 1, "warnings: {warnings:?}");
    assert!(
        !warnings.iter().any(|w| w.contains("未対応色空間")),
        "ICCBased must not skip: {warnings:?}"
    );
    let obj = doc.all_objects().next().unwrap().1;
    let ObjectType::Image { png_bytes, .. } = &obj.object_type else {
        panic!("expected placed image");
    };
    let img = image::load_from_memory(png_bytes).unwrap().to_rgba8();
    assert_eq!(&img.get_pixel(0, 0).0[..3], &[255u8, 0u8, 0u8]);
    assert_eq!(&img.get_pixel(1, 0).0[..3], &[0u8, 255u8, 0u8]);
}

#[test]
fn pdf_import_restores_lab_image() {
    use irasu_illustrator::core::document::ObjectType;
    // 2x1 Lab samples with the default decode (L 0..100, a/b -100..100).
    let samples = [255, 128, 128, 0, 128, 128];
    let pdf = build_image_pdf(
        b"q 2 0 0 2 10 80 cm /Im1 Do Q\n",
        b"<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace [/Lab << /WhitePoint [0.9642 1 0.8251] >>] /BitsPerComponent 8",
        &samples,
        &[],
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("lab image import");
    assert_eq!(doc.all_objects().count(), 1, "warnings: {warnings:?}");
    assert!(
        !warnings.iter().any(|w| w.contains("未対応色空間")),
        "Lab must not skip: {warnings:?}"
    );
    let obj = doc.all_objects().next().unwrap().1;
    let ObjectType::Image { png_bytes, .. } = &obj.object_type else {
        panic!("expected placed image");
    };
    let img = image::load_from_memory(png_bytes).unwrap().to_rgba8();
    assert!(img.get_pixel(0, 0).0[..3].iter().all(|v| *v > 240));
    assert!(img.get_pixel(1, 0).0[..3].iter().all(|v| *v < 15));
}

fn fax_image_pdf(samples: &[u8], parms: &str, w: u32, h: u32) -> Vec<u8> {
    build_image_pdf(
        b"q 8 0 0 8 10 80 cm /Im1 Do Q\n",
        format!(
            "<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 1 /Filter /CCITTFaxDecode /DecodeParms << {parms} >>"
        )
        .as_bytes(),
        samples,
        &[],
    )
}

fn placed_gray_pixels(doc: &Document) -> Vec<u8> {
    let obj = doc.all_objects().next().unwrap().1;
    let ObjectType::Image { png_bytes, .. } = &obj.object_type else {
        panic!("expected placed image");
    };
    let img = image::load_from_memory(png_bytes).unwrap().to_luma8();
    img.pixels().map(|p| p[0]).collect()
}

#[test]
fn pdf_import_decodes_g4_fax_images() {
    // 8x2 all-white G4: two vertical(0) codes (`1` each); height bounds the
    // read so no EOFB is needed.
    let pdf = fax_image_pdf(&[0xC0], "/K -1 /Columns 8 /Rows 2", 8, 2);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("g4 import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(doc.all_objects().count(), 1);
    assert!(placed_gray_pixels(&doc).iter().all(|v| *v == 255));

    // 8x1 all-black G4: Horizontal mode (`001`) + white run 0
    // (`00110101`) + black run 8 (`000101`).
    let pdf = fax_image_pdf(&[0x26, 0xA2, 0x80], "/K -1 /Columns 8 /Rows 1", 8, 1);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("g4 black import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert!(placed_gray_pixels(&doc).iter().all(|v| *v == 0));
}

#[test]
fn pdf_import_decodes_g3_1d_and_honors_black_is_1() {
    // EOL + white run 8 (`10011`) + EOL, then the 5-EOL return-to-control.
    let eol = "000000000001";
    let bits = format!("{eol}10011{eol}{}{}{}{}{}", eol, eol, eol, eol, eol);
    let mut data = vec![0u8; bits.len().div_ceil(8)];
    for (i, bit) in bits.bytes().enumerate() {
        if bit == b'1' {
            data[i / 8] |= 0x80 >> (i % 8);
        }
    }
    let pdf = fax_image_pdf(&data, "/K 0 /Columns 8 /Rows 1", 8, 1);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("g3 import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert!(placed_gray_pixels(&doc).iter().all(|v| *v == 255));

    // Same all-white encoding with BlackIs1 reads back black.
    let pdf = fax_image_pdf(&[0xC0], "/K -1 /Columns 8 /Rows 2 /BlackIs1 true", 8, 2);
    let (doc, _) = parse_pdf_bytes(&pdf).expect("blackis1 import");
    assert!(placed_gray_pixels(&doc).iter().all(|v| *v == 0));
}

#[test]
fn pdf_import_skips_mixed_2d_fax_explicitly() {
    let pdf = fax_image_pdf(&[0xC0], "/K 2 /Columns 8 /Rows 2", 8, 2);
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("k>0 import");
    assert_eq!(doc.all_objects().count(), 0);
    assert!(warnings.iter().any(|w| w.contains("FAX")), "{warnings:?}");
}

#[test]
fn pdf_import_executes_type3_glyphs_as_paths() {
    use irasu_illustrator::core::document::ObjectType;
    let pdf = build_type3_pdf(
        b"BT /F1 24 Tf 20 80 Td (A) Tj ET\n",
        b"<< /Type /Font /Subtype /Type3 /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 65 /LastChar 65 /Widths [600] /Encoding 6 0 R /CharProcs 7 0 R >>",
        b"<< /Type /Encoding /Differences [65 /Aglyph] >>",
        b"<< /Aglyph 8 0 R >>",
        b"0 0 m 0 500 l 500 500 l 500 0 l h f\n",
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("type3 import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(doc.all_objects().count(), 1);
    let obj = doc.all_objects().next().unwrap().1;
    assert!(
        matches!(obj.object_type, ObjectType::Path(_)),
        "glyph becomes a path"
    );
    // 500-unit square at size 24 lands on (20,8)..(32,20) after the y-flip.
    let (mn, mx) = obj.bounding_box().unwrap();
    assert!((mn.x - 20.0).abs() < 1e-6 && (mx.x - 32.0).abs() < 1e-6);
    assert!((mn.y - 8.0).abs() < 1e-6 && (mx.y - 20.0).abs() < 1e-6);
}

#[test]
fn pdf_import_falls_back_for_unresolvable_type3() {
    let pdf = build_type3_pdf(
        b"BT /F1 24 Tf 20 80 Td (A) Tj ET\n",
        b"<< /Type /Font /Subtype /Type3 /FontMatrix [0.001 0 0 0.001 0 0] /Encoding /WinAnsiEncoding >>",
        b"<< /Type /Encoding /Differences [65 /Aglyph] >>",
        b"<< /Aglyph 8 0 R >>",
        b"0 0 m 0 500 l 500 500 l 500 0 l h f\n",
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("type3 fallback import");
    assert!(warnings.iter().any(|w| w.contains("Type3")), "{warnings:?}");
    assert_eq!(doc.all_objects().count(), 1);
}

#[test]
fn pdf_import_applies_extgstate_alpha_blend_and_overprint() {
    use irasu_illustrator::core::document::BlendMode;
    let pdf = build_gs_pdf(
        b"/GS1 gs\n1 0 0 rg\n10 10 50 30 re f\n",
        b"<< /Type /ExtGState /ca 0.5 /CA 0.25 /BM /Multiply /OP true >>",
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("extgstate import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(doc.all_objects().count(), 1);
    let obj = doc.all_objects().next().unwrap().1;
    let fill = obj.fill.as_ref().expect("fill");
    assert!((fill.color[3] - 0.5).abs() < 1e-6);
    assert!(fill.overprint);
    assert_eq!(obj.blend_mode, BlendMode::Multiply);
}

#[test]
fn pdf_import_flags_pattern_paint_explicitly() {
    let pdf = build_pdf(b"/P1 scn\n10 10 50 30 re f\n");
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("pattern paint import");
    assert!(
        warnings.iter().any(|w| w.contains("パターン")),
        "{warnings:?}"
    );
    // The rect after the pattern operator still imports (single-color); the
    // warning records that its pattern paint was dropped.
    assert_eq!(doc.all_objects().count(), 1);
}

#[test]
fn pdf_import_restores_separation_image_via_tint() {
    use irasu_illustrator::core::document::ObjectType;
    // 2x1 spot tints (0 and 1) mapped through an exponential tint
    // transform: 0 → uninked white, 1 → CMYK(0,1,1,0) red.
    let samples = [0, 255];
    let pdf = build_image_pdf(
        b"q 2 0 0 2 10 80 cm /Im1 Do Q\n",
        b"<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace [/Separation /SpotA /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 1 0] /N 1 >>] /BitsPerComponent 8",
        &samples,
        &[],
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("separation image import");
    assert_eq!(doc.all_objects().count(), 1, "warnings: {warnings:?}");
    assert!(
        !warnings.iter().any(|w| w.contains("未対応色空間")),
        "Separation must not skip: {warnings:?}"
    );
    let obj = doc.all_objects().next().unwrap().1;
    let ObjectType::Image { png_bytes, .. } = &obj.object_type else {
        panic!("expected placed image");
    };
    let img = image::load_from_memory(png_bytes).unwrap().to_rgba8();
    assert_eq!(&img.get_pixel(0, 0).0[..3], &[255u8, 255u8, 255u8]);
    assert_eq!(&img.get_pixel(1, 0).0[..3], &[255u8, 0u8, 0u8]);
}

#[test]
fn pdf_import_paints_freeform_gouraud_mesh() {
    // One RGB triangle: red (1,1), green (7,1), blue (4,7); 8-bit fields.
    let data: Vec<u8> = vec![
        0, 1, 1, 255, 0, 0, //
        1, 7, 1, 0, 255, 0, //
        2, 4, 7, 0, 0, 255,
    ];
    let pdf = build_mesh_pdf(
        b"/Sh1 sh\n",
        b"<< /ShadingType 4 /ColorSpace /DeviceRGB /BitsPerFlag 8 /BitsPerCoordinate 8 /BitsPerComponent 8 /Decode [0 8 0 8 0 1 0 1 0 1] /BBox [0 0 8 8]",
        &data,
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("mesh import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(doc.all_objects().count(), 1);
    let obj = doc.all_objects().next().unwrap().1;
    let fill = obj.fill.as_ref().expect("flat fill");
    // Vertex average: (85, 85, 85) / 255.
    for (channel, expected) in fill.color.iter().zip([85.0 / 255.0; 3]) {
        assert!((channel - expected).abs() < 0.01, "{:?}", fill.color);
    }
}

#[test]
fn pdf_import_triangulates_lattice_mesh() {
    // 2x2 lattice: red, green / blue, white.
    let data: Vec<u8> = vec![
        0, 1, 1, 255, 0, 0, //
        0, 7, 1, 0, 255, 0, //
        0, 1, 7, 0, 0, 255, //
        0, 7, 7, 255, 255, 255,
    ];
    let pdf = build_mesh_pdf(
        b"/Sh1 sh\n",
        b"<< /ShadingType 5 /ColorSpace /DeviceRGB /BitsPerFlag 8 /BitsPerCoordinate 8 /BitsPerComponent 8 /VerticesPerRow 2 /Decode [0 8 0 8 0 1 0 1 0 1] /BBox [0 0 8 8]",
        &data,
    );
    let (doc, warnings) = parse_pdf_bytes(&pdf).expect("lattice import");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(doc.all_objects().count(), 2);
}
