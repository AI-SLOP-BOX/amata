//! PDF import: hand-built fixture + exporter round-trip.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::path::FillStyle;
use irasu_illustrator::io::pdf_import::parse_pdf_bytes;

/// Assemble a minimal one-page PDF with correct xref offsets.
fn build_pdf(content: &[u8]) -> Vec<u8> {
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
    objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());

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
        out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
            objs.len() + 1,
            xref_at
        )
        .as_bytes(),
    );
    out
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
    let rect = objs.iter().find(|o| o.name.starts_with("PDF Path")).expect("rect");
    let (mn, mx) = rect.bounding_box().expect("bbox");
    assert!((mn.x - 10.0).abs() < 1e-6 && (mx.x - 60.0).abs() < 1e-6);
    assert!((mn.y - 60.0).abs() < 1e-6 && (mx.y - 90.0).abs() < 1e-6);
    let fill = rect.fill.as_ref().expect("fill");
    assert!(fill.color[0] > 0.9 && fill.color[1] < 0.1, "red fill: {:?}", fill.color);

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
    assert!((texts[0].transform.y - 20.0).abs() < 1e-6, "y-flipped baseline");
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
    assert!(warnings.iter().any(|w| w.contains("修復")), "repair reported");
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
    assert!(n >= 2, "rect + text survive, got {n} (warnings: {warnings:?})");
    // Red rect keeps its fill through the round trip.
    let red = back
        .all_objects()
        .map(|(_, o)| o)
        .filter(|o| matches!(o.object_type, ObjectType::Path(_)))
        .filter_map(|o| o.fill.clone())
        .any(|f| f.color[0] > 0.9);
    assert!(red, "red fill survives");
}
