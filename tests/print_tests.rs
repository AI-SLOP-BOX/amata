//! Print pipeline: press-PDF structure and preflight logic.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{ColorMode, Document, Object};
use irasu_illustrator::core::path::{FillRule, FillStyle, FillType, LinearGradient};
use irasu_illustrator::core::print::{preflight, PreflightLevel};
use irasu_illustrator::io::pdf_print::{export_pdf_print, PrintPdfOptions};

fn tiny_png() -> Vec<u8> {
    let mut buf = Vec::new();
    let img = image::RgbaImage::from_fn(8, 8, |x, y| {
        if x == y {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 0, 255, 128])
        }
    });
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .unwrap();
    buf
}

fn press_doc() -> Document {
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.width = 200.0;
    doc.height = 100.0;
    doc.bleed = irasu_illustrator::core::print::mm_to_pt(3.0);
    // Process red rect with overprint.
    let mut rect = Object::new_rect("R", 10.0, 10.0, 80.0, 40.0, 0.0);
    rect.fill = Some(FillStyle {
        color: [1.0, 0.0, 0.0, 1.0],
        fill_type: FillType::Solid([1.0, 0.0, 0.0, 1.0]),
        rule: FillRule::NonZero,
        overprint: true,
        spot: None,
    });
    doc.add_object(rect);
    // Spot circle.
    let mut circ = Object::new_rect("S", 100.0, 10.0, 40.0, 40.0, 20.0);
    circ.fill = Some(FillStyle {
        color: [0.0, 0.0, 0.0, 1.0],
        fill_type: FillType::Solid([0.0, 0.0, 0.0, 1.0]),
        rule: FillRule::NonZero,
        overprint: false,
        spot: Some("PANTONE Reflex Blue C".to_string()),
    });
    doc.add_object(circ);
    // Gradient bar.
    let mut grad = Object::new_rect("G", 10.0, 60.0, 80.0, 20.0, 0.0);
    grad.fill = Some(FillStyle::linear_gradient(LinearGradient {
        start_x: 0.0,
        start_y: 0.5,
        end_x: 1.0,
        end_y: 0.5,
        stops: vec![
            irasu_illustrator::core::path::GradientStop {
                offset: 0.0,
                color: [0.0, 0.0, 0.0, 1.0],
            },
            irasu_illustrator::core::path::GradientStop {
                offset: 1.0,
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ],
    }));
    doc.add_object(grad);
    // Placed image with alpha (exercises SMask).
    let (_, _, placed) =
        irasu_illustrator::io::raster::decode_placed_image(&tiny_png()).unwrap();
    doc.add_object(Object::new_image("Img", 150.0, 10.0, 8.0, 8.0, placed));
    doc
}

fn parse(bytes: &[u8]) -> lopdf::Document {
    lopdf::Document::load_mem(bytes).expect("press PDF parses")
}

#[test]
fn test_press_pdf_boxes_and_output_intent() {
    let (pdf, _) = export_pdf_print(&press_doc(), &PrintPdfOptions::default());
    let doc = parse(&pdf);
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 1);
    let page = doc.get_object(*pages.values().next().unwrap()).unwrap();
    let page = match page {
        lopdf::Object::Dictionary(d) => d.clone(),
        _ => panic!("page dict"),
    };
    for key in [&b"MediaBox"[..], &b"TrimBox"[..], &b"BleedBox"[..], &b"CropBox"[..]] {
        assert!(page.get(key).is_ok(), "{key:?} present");
    }
    let media = match page.get(b"MediaBox").unwrap() {
        lopdf::Object::Array(a) => a.clone(),
        _ => panic!("mediabox"),
    };
    let num = |o: &lopdf::Object| match o {
        lopdf::Object::Integer(i) => *i as f64,
        lopdf::Object::Real(f) => *f as f64,
        _ => -1.0,
    };
    // Media = trim + bleed + slug on every side.
    assert!(num(&media[2]) > 200.0 && num(&media[3]) > 100.0);
    // Catalog carries PDF/X markers.
    let catalog = doc.get_object(doc.trailer.get(b"Root").unwrap().as_reference().unwrap()).unwrap();
    let catalog = match catalog {
        lopdf::Object::Dictionary(d) => d.clone(),
        _ => panic!("catalog"),
    };
    assert!(catalog.get(b"GTS_PDFXVersion").is_ok());
    assert!(catalog.get(b"OutputIntents").is_ok());
}

#[test]
fn test_press_pdf_separations_overprint_images_shadings() {
    let (pdf, warnings) = export_pdf_print(&press_doc(), &PrintPdfOptions::default());
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Separation"), "spot color space");
    assert!(text.contains("PANTONE"), "spot name survives: {warnings:?}");
    assert!(text.contains("/OP true"), "overprint ExtGState");
    assert!(text.contains("/ShadingType 2"), "axial shading for gradients");
    assert!(text.contains("/SMask"), "alpha mask for translucent PNG");
    assert!(text.contains("/DCTDecode"), "JPEG image XObject");
    assert!(text.contains("DeviceCMYK"), "CMYK mode output");
    // And it all still parses.
    parse(&pdf);
}

#[test]
fn test_rgb_doc_stays_rgb() {
    let mut doc = Document::default();
    doc.width = 100.0;
    doc.height = 100.0;
    let mut rect = Object::new_rect("R", 5.0, 5.0, 20.0, 20.0, 0.0);
    rect.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
    doc.add_object(rect);
    let (pdf, _) = export_pdf_print(&doc, &PrintPdfOptions::default());
    let text = String::from_utf8_lossy(&pdf);
    assert!(!text.contains("DeviceCMYK"), "no CMYK in RGB export");
    assert!(text.contains(" rg"), "RGB fill ops");
}

#[test]
fn test_marks_change_output() {
    let doc = press_doc();
    let (with_marks, _) = export_pdf_print(&doc, &PrintPdfOptions::default());
    let (plain, _) = export_pdf_print(
        &doc,
        &PrintPdfOptions { marks: false, bleed: Some(0.0), pdfx: false },
    );
    assert!(with_marks.len() > plain.len(), "marks add content");
    assert!(String::from_utf8_lossy(&with_marks).contains("arc"), "registration targets");
}

#[test]
fn test_preflight_catches_classics() {
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.bleed = 0.0;
    // White overprint (would vanish on press).
    let mut rect = Object::new_rect("W", 0.0, 0.0, 10.0, 10.0, 0.0);
    rect.fill = Some(FillStyle {
        color: [1.0, 1.0, 1.0, 1.0],
        fill_type: FillType::Solid([1.0, 1.0, 1.0, 1.0]),
        rule: FillRule::NonZero,
        overprint: true,
        spot: None,
    });
    doc.add_object(rect);
    // RGB red in a CMYK doc.
    let mut rect2 = Object::new_rect("R", 20.0, 0.0, 10.0, 10.0, 0.0);
    rect2.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
    doc.add_object(rect2);
    let issues = preflight(&doc);
    let has = |level: PreflightLevel, check: &str| {
        issues.iter().any(|i| i.level == level && i.check == check)
    };
    assert!(has(PreflightLevel::Fail, "白のオーバープリント"));
    assert!(has(PreflightLevel::Warn, "RGBオブジェクト"));
    assert!(has(PreflightLevel::Warn, "塗り足し"));
}

#[test]
fn test_preflight_clean_doc_passes() {
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.bleed = irasu_illustrator::core::print::mm_to_pt(3.0);
    let issues = preflight(&doc);
    assert!(
        !issues.iter().any(|i| i.level == PreflightLevel::Fail),
        "clean doc has no failures: {issues:?}"
    );
}

#[test]
fn test_spot_serialization_round_trip() {
    let doc = Document::default();
    assert!(!doc.spots.is_empty(), "new docs ship starter spots");
    let json = serde_json::to_string(&doc).unwrap();
    let back: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(back.spots, doc.spots);
    assert_eq!(back.bleed, doc.bleed);
}
