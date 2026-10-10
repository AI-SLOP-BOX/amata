//! Print pipeline: press-PDF structure and preflight logic.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{ColorMode, Document, Object};
use irasu_illustrator::core::path::{
    FillRule, FillStyle, FillType, GradientStop, ImageFill, ImageTileMode, LinearGradient,
    StrokeStyle,
};
use irasu_illustrator::core::print::{mm_to_pt, preflight, PreflightLevel};
use irasu_illustrator::io::pdf_print::{export_pdf_print, validate_pdfx, PrintPdfOptions};

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
        spot: Some("Spot Reflex Blue".to_string()),
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
    let (_, _, placed) = irasu_illustrator::io::raster::decode_placed_image(&tiny_png()).unwrap();
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
    for key in [
        &b"MediaBox"[..],
        &b"TrimBox"[..],
        &b"BleedBox"[..],
        &b"CropBox"[..],
    ] {
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
    let catalog = doc
        .get_object(doc.trailer.get(b"Root").unwrap().as_reference().unwrap())
        .unwrap();
    let catalog = match catalog {
        lopdf::Object::Dictionary(d) => d.clone(),
        _ => panic!("catalog"),
    };
    assert!(catalog.get(b"GTS_PDFXVersion").is_ok());
    assert!(catalog.get(b"OutputIntents").is_ok());
}

/// Resolve the single OutputIntent's `/DestOutputProfile` and decompress it,
/// returning the declared channel count alongside the ICC bytes.
fn output_intent_profile(doc: &lopdf::Document) -> (i64, Vec<u8>) {
    let catalog = doc
        .get_object(doc.trailer.get(b"Root").unwrap().as_reference().unwrap())
        .unwrap();
    let catalog = match catalog {
        lopdf::Object::Dictionary(d) => d.clone(),
        _ => panic!("catalog"),
    };
    let intents = match catalog.get(b"OutputIntents").unwrap() {
        lopdf::Object::Array(a) => a.clone(),
        _ => panic!("output intents array"),
    };
    assert_eq!(intents.len(), 1, "exactly one OutputIntent");
    let intent = match &intents[0] {
        lopdf::Object::Dictionary(d) => d.clone(),
        _ => panic!("output intent dict"),
    };
    let dest = intent
        .get(b"DestOutputProfile")
        .expect("PDF/X intent carries an embedded profile")
        .as_reference()
        .unwrap();
    let profile = match doc.get_object(dest).unwrap() {
        lopdf::Object::Stream(s) => s.clone(),
        _ => panic!("profile is a stream"),
    };
    let n = match profile.dict.get(b"N").unwrap() {
        lopdf::Object::Integer(n) => *n,
        _ => panic!("/N is an integer"),
    };
    (
        n,
        profile
            .decompressed_content()
            .expect("profile decompresses"),
    )
}

#[test]
fn test_pdfx_embeds_the_dest_output_profile() {
    let (pdf, _) = export_pdf_print(&press_doc(), &PrintPdfOptions::default());
    let doc = parse(&pdf);
    let (n, icc) = output_intent_profile(&doc);
    assert_eq!(n, 4, "CMYK document tags a four-channel profile");
    assert!(icc.len() > 128, "profile payload present");
    assert_eq!(&icc[36..40], b"acsp", "ICC magic number");
    assert_eq!(&icc[16..20], b"CMYK", "profile space matches the plates");
}

#[test]
fn test_rgb_output_intent_is_tagged_srgb() {
    let mut doc = Document::default();
    doc.width = 100.0;
    doc.height = 100.0;
    let (pdf, _) = export_pdf_print(&doc, &PrintPdfOptions::default());
    let parsed = parse(&pdf);
    let (n, icc) = output_intent_profile(&parsed);
    assert_eq!(n, 3, "RGB document tags a three-channel profile");
    assert_eq!(&icc[36..40], b"acsp", "ICC magic number");
    assert_eq!(&icc[16..20], b"RGB ", "profile space is RGB");
    // …and the intent name must agree with the bytes behind it.
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("(sRGB IEC61966-2.1)"), "condition names sRGB");
    assert!(!text.contains("DeviceCMYK"), "no CMYK in RGB export");
}

#[test]
fn test_no_output_intent_without_pdfx_flag() {
    let (pdf, _) = export_pdf_print(
        &press_doc(),
        &PrintPdfOptions {
            marks: false,
            bleed: Some(0.0),
            pdfx: false,
            outline_text: false,
        },
    );
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        !text.contains("/OutputIntents"),
        "no intent when PDF/X is off"
    );
    assert!(!text.contains("/DestOutputProfile"), "no profile either");
    parse(&pdf);
}

#[test]
fn test_press_pdf_separations_overprint_images_shadings() {
    let (pdf, warnings) = export_pdf_print(&press_doc(), &PrintPdfOptions::default());
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Separation"), "spot color space");
    assert!(text.contains("Reflex"), "spot name survives: {warnings:?}");
    assert!(text.contains("/OP true"), "overprint ExtGState");
    assert!(
        text.contains("/ShadingType 2"),
        "axial shading for gradients"
    );
    assert!(
        !text.contains("/SMask"),
        "alpha composited onto paper, no masks"
    );
    assert!(
        text.contains("/DCTDecode") || text.contains("/FlateDecode"),
        "image XObject"
    );
    assert!(text.contains("DeviceCMYK"), "CMYK mode output");
    // And it all still parses.
    parse(&pdf);
}

#[test]
fn test_pdfx_validator_flags_live_transparency() {
    // A translucent vector object under PDF/X-1a must come out flattened
    // (opaque raster), not live transparency: no violations, parseable.
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.width = 200.0;
    doc.height = 200.0;
    doc.bleed = irasu_illustrator::core::print::mm_to_pt(3.0);
    let mut ghost = Object::new_rect("G", 20.0, 20.0, 60.0, 60.0, 0.0);
    ghost.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
    ghost.opacity = 0.5;
    doc.add_object(ghost);
    let (pdf, warnings) = export_pdf_print(&doc, &PrintPdfOptions::default());
    assert!(
        !warnings.iter().any(|w| w.contains("PDF/X-1a違反")),
        "flattened file is X-1a clean: {warnings:?}"
    );
    assert!(
        warnings.iter().any(|w| w.contains("フラット化")),
        "flattening reported: {warnings:?}"
    );
    let text = String::from_utf8_lossy(&pdf);
    assert!(!text.contains("/SMask"), "no soft masks");
    // Placed RGB images convert to opaque CMYK (no SMask, no RGB plates).
    let (pdf, warnings) = export_pdf_print(&press_doc(), &PrintPdfOptions::default());
    assert!(
        !warnings.iter().any(|w| w.contains("PDF/X-1a違反")),
        "press doc converts clean: {warnings:?}"
    );
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/DeviceCMYK"), "CMYK plates");
    assert!(!text.contains("/SMask"), "alpha composited, no masks");
    parse(&pdf);
}

#[test]
fn test_pdfx_flattens_transparency_to_clean_plates() {
    // A translucent object under PDF/X-1a must come out as opaque raster,
    // not live transparency: no violations, parseable, region placed.
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.width = 200.0;
    doc.height = 200.0;
    doc.bleed = irasu_illustrator::core::print::mm_to_pt(3.0);
    let mut ghost = Object::new_rect("G", 20.0, 20.0, 60.0, 60.0, 0.0);
    ghost.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
    ghost.opacity = 0.5;
    doc.add_object(ghost);
    let (pdf, warnings) = export_pdf_print(&doc, &PrintPdfOptions::default());
    assert!(
        !warnings.iter().any(|w| w.contains("PDF/X-1a違反")),
        "flattened file is X-1a clean: {warnings:?}"
    );
    assert!(
        warnings.iter().any(|w| w.contains("フラット化")),
        "flattening reported: {warnings:?}"
    );
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Fm1 Do"), "region placed");
    assert!(!text.contains("/SMask"), "no soft masks");
    assert!(!text.contains("/CA"), "no live alpha states");
    parse(&pdf);
}

#[test]
fn test_embedded_text_stays_selectable() {
    // Resolvable font + solid fill => embedded CIDFontType2 with ToUnicode,
    // not outlines: text remains selectable in the press PDF.
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.width = 200.0;
    doc.height = 100.0;
    doc.bleed = irasu_illustrator::core::print::mm_to_pt(3.0);
    let mut txt = Object::new_text("T", "Hello", 10.0, 40.0, 24.0);
    txt.fill = Some(FillStyle::solid([0.0, 0.0, 0.0, 1.0]));
    doc.add_object(txt);
    let (pdf, warnings) = export_pdf_print(&doc, &PrintPdfOptions::default());
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        text.contains("/CIDFontType2"),
        "embedded font: {warnings:?}"
    );
    assert!(text.contains("/ToUnicode"), "searchable");
    assert!(text.contains("] TJ"), "positioned text");
    parse(&pdf);
}

#[test]
fn print_outline_text_option_forces_glyph_paths() {
    // 印刷所フレンドリーな「テキスト全部アウトライン化」。ON だと
    // 埋め込みフォントが消え、ToUnicode も出ない（= テキスト検索不可）。
    use irasu_illustrator::core::document::TextStyle;
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.width = 200.0;
    doc.height = 100.0;
    doc.bleed = mm_to_pt(3.0);
    let mut style = TextStyle::default();
    style.font_family = "Noto Sans JP".into();
    doc.add_object(Object::new_text_with_style("T", "和a", 10.0, 40.0, style));
    let embedded = export_pdf_print(
        &doc,
        &PrintPdfOptions {
            pdfx: true,
            outline_text: false,
            ..Default::default()
        },
    );
    let outlined = export_pdf_print(
        &doc,
        &PrintPdfOptions {
            pdfx: true,
            outline_text: true,
            ..Default::default()
        },
    );
    let (a, _) = embedded;
    let (b, _) = outlined;
    let sa = String::from_utf8_lossy(&a);
    let sb = String::from_utf8_lossy(&b);
    assert!(
        sa.contains("/CIDFontType2"),
        "default embeds the face: {}",
        sa.len()
    );
    assert!(
        !sb.contains("/CIDFontType2"),
        "outline_text drops the embedded font: {}",
        sb.len()
    );
    // The outlined copy still draws the glyphs (as path objects).
    assert!(sb.contains(" cm") && sb.contains(" c"), "paths emitted");
    parse(&b);
}

#[test]
fn pdfx4_output_self_check_passes() {
    // Exported PDF/X-4 must satisfy the requirements the claims make:
    // OutputIntent present, boxes consistent, no RGB-only plates and no
    // live transparency left in the page content.
    let mut doc = press_doc();
    doc.color_mode = ColorMode::Cmyk;
    doc.width = 400.0;
    doc.height = 200.0;
    doc.bleed = mm_to_pt(3.0);
    let (pdf, warnings) = export_pdf_print(
        &doc,
        &PrintPdfOptions {
            pdfx: true,
            ..Default::default()
        },
    );
    assert!(warnings.is_empty(), "clean doc exports clean: {warnings:?}");
    let parsed = parse(&pdf);
    // OutputIntent in the catalog.
    let catalog = parsed
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .and_then(|id| parsed.get_dictionary(id).ok())
        .cloned()
        .expect("catalog");
    assert!(
        catalog.has(b"OutputIntents"),
        "PDF/X requires an OutputIntents array: {:?}",
        catalog
    );
    // No ExtGState with /SMask (live transparency).
    let mut smask = 0;
    for obj in parsed.objects.values() {
        if let lopdf::Object::Dictionary(d) = obj {
            if d.has(b"SMask") {
                smask += 1;
            }
        }
    }
    assert_eq!(smask, 0, "PDF/X-4 content carries no soft masks");
    // Boxes: TrimBox inside BleedBox inside MediaBox.
    let issues = validate_pdfx(&pdf, true);
    assert!(issues.is_empty(), "PDF/X-4 self-check clean: {issues:?}");
    let (sanity, _) = export_pdf_print(
        &press_doc(),
        &PrintPdfOptions {
            pdfx: true,
            ..Default::default()
        },
    );
    assert!(sanity.starts_with(b"%PDF"));
}

#[test]
fn test_boundary_text_still_embeds_with_tj_gap() {
    // 和欧境界を持つテキストも、TJディスプレースメントで1/4emを表現して
    // 埋め込みフォント経路を維持する（以前はアウトライン化されていた）。
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.width = 200.0;
    doc.height = 100.0;
    doc.bleed = irasu_illustrator::core::print::mm_to_pt(3.0);
    let mut style = irasu_illustrator::core::document::TextStyle::default();
    style.font_family = "Noto Sans JP".into();
    doc.add_object(Object::new_text_with_style("T", "和a", 10.0, 40.0, style));
    let (pdf, warnings) = export_pdf_print(&doc, &PrintPdfOptions::default());
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        text.contains("/CIDFontType2"),
        "boundary text keeps embedded font: {warnings:?}"
    );
    // Content stream is Flate-compressed; decode it and confirm TJ
    // displacements (including the 和欧 gap) are present.
    let doc_pdf = parse(&pdf);
    let mut saw_tj = false;
    for obj in doc_pdf.objects.values() {
        if let lopdf::Object::Stream(st) = obj {
            let mut s = st.clone();
            let body = if s.decompress().is_ok() {
                String::from_utf8_lossy(&s.content).into_owned()
            } else {
                String::from_utf8_lossy(&st.content).into_owned()
            };
            if body.contains("] TJ") {
                saw_tj = true;
            }
        }
    }
    assert!(saw_tj, "positioned text stream present");
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
        &PrintPdfOptions {
            marks: false,
            bleed: Some(0.0),
            pdfx: false,
            outline_text: false,
        },
    );
    assert!(with_marks.len() > plain.len(), "marks add content");
    // Registration targets are drawn as crosshairs + a cubic-circle ring
    // (PDF has no `arc` operator; the old invalid `… arc S` was replaced).
    let marks = String::from_utf8_lossy(&with_marks);
    assert!(marks.contains("c S"), "registration ring uses cubic curves");
    assert!(marks.contains("m"), "crop/registration crosshairs");
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
fn test_preflight_flags_press_killers() {
    use irasu_illustrator::core::document::TextStyle;
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.bleed = irasu_illustrator::core::print::mm_to_pt(3.0);
    // Hairline stroke.
    let mut thin = Object::new_rect("T", 0.0, 0.0, 10.0, 10.0, 0.0);
    thin.stroke = Some(irasu_illustrator::core::path::StrokeStyle {
        color: [0.0, 0.0, 0.0, 1.0],
        width: 0.1,
        ..Default::default()
    });
    doc.add_object(thin);
    // Transparent object.
    let mut ghost = Object::new_rect("G", 20.0, 0.0, 10.0, 10.0, 0.0);
    ghost.opacity = 0.5;
    doc.add_object(ghost);
    // Missing font.
    let txt = Object::new_text_with_style(
        "X",
        "hi",
        0.0,
        0.0,
        TextStyle::new("No Such Family XYZ", 12.0),
    );
    doc.add_object(txt);
    // RGB image.
    doc.add_object(Object::new_image("I", 0.0, 0.0, 8.0, 8.0, tiny_png()));
    let issues = preflight(&doc);
    let has = |level: PreflightLevel, check: &str| {
        issues.iter().any(|i| i.level == level && i.check == check)
    };
    assert!(has(PreflightLevel::Warn, "ヘアライン"));
    assert!(has(PreflightLevel::Warn, "透明・効果"));
    assert!(has(PreflightLevel::Fail, "未インストールフォント"));
    assert!(has(PreflightLevel::Warn, "RGB画像"));
}

#[test]
fn test_preflight_includes_pattern_and_cropped_image_fills_in_ink_check() {
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    let mut pattern = Object::new_rect("Pattern", 0.0, 0.0, 20.0, 20.0, 0.0);
    pattern.fill = Some(FillStyle {
        color: [1.0, 0.0, 0.0, 1.0],
        fill_type: FillType::Pattern(Default::default()),
        ..FillStyle::default()
    });
    doc.add_object(pattern);
    let pattern_issues = preflight(&doc);
    let pattern_ink = pattern_issues
        .iter()
        .find(|issue| issue.check == "インキ総量")
        .unwrap();
    assert!(
        pattern_ink.detail.contains("パターン塗り"),
        "{pattern_ink:?}"
    );

    let mut source = Object::new_image("Ink source", 0.0, 30.0, 8.0, 8.0, tiny_png());
    source.id = "ink-source".to_string();
    let source_id = source.id.clone();
    doc.add_object(source);
    let mut image = Object::new_rect("Image fill", 30.0, 0.0, 20.0, 20.0, 0.0);
    image.fill = Some(FillStyle {
        fill_type: FillType::Image(irasu_illustrator::core::path::ImageFill {
            image_id: source_id,
            tile_mode: irasu_illustrator::core::path::ImageTileMode::Cover,
            crop_rect: Some([0.0, 0.0, 0.5, 1.0]),
        }),
        ..FillStyle::default()
    });
    doc.add_object(image);

    let issues = preflight(&doc);
    let ink = issues
        .iter()
        .find(|issue| issue.check == "インキ総量")
        .unwrap();
    assert!(ink.detail.contains("画像塗り"), "{ink:?}");
}

#[test]
fn test_print_pdf_renders_pattern_and_clipped_image_fills() {
    use irasu_illustrator::core::path::{ImageFill, ImageTileMode, PatternFill};

    let mut doc = Document::default();
    doc.width = 100.0;
    doc.height = 60.0;
    let mut source = Object::new_image("Source", 0.0, 0.0, 8.0, 8.0, tiny_png());
    source.id = "image-fill-source".to_string();
    let source_id = source.id.clone();
    doc.add_object(source);

    let mut patterned = Object::new_rect("Pattern fill", 10.0, 10.0, 20.0, 20.0, 0.0);
    patterned.fill = Some(FillStyle {
        color: [0.1, 0.3, 0.8, 1.0],
        fill_type: FillType::Pattern(PatternFill {
            tile_width: 5.0,
            tile_height: 5.0,
            ..Default::default()
        }),
        ..FillStyle::default()
    });
    doc.add_object(patterned);

    let mut image_filled = Object::new_rect("Image fill", 40.0, 10.0, 30.0, 20.0, 0.0);
    image_filled.fill = Some(FillStyle {
        fill_type: FillType::Image(ImageFill {
            image_id: source_id,
            tile_mode: ImageTileMode::Cover,
            crop_rect: Some([0.0, 0.0, 0.5, 1.0]),
        }),
        ..FillStyle::default()
    });
    doc.add_object(image_filled);

    let (pdf_bytes, warnings) = export_pdf_print(
        &doc,
        &PrintPdfOptions {
            marks: false,
            bleed: Some(0.0),
            pdfx: false,
            outline_text: false,
        },
    );
    assert!(
        !warnings.iter().any(|w| w.contains("単色近似")),
        "{warnings:?}"
    );
    let parsed = parse(&pdf_bytes);
    let page_id = *parsed.get_pages().values().next().unwrap();
    let content = String::from_utf8(parsed.get_page_content(page_id).unwrap()).unwrap();
    assert!(content.contains("W n"), "fills are clipped to their paths");
    assert!(
        content.contains(" l S"),
        "pattern motifs are emitted as clipped vector strokes"
    );
    assert!(
        content.matches(" Do").count() >= 2,
        "image object and image fill are painted"
    );
    let has_cropped_image = parsed.objects.values().any(|object| match object {
        lopdf::Object::Stream(stream) => {
            matches!(stream.dict.get(b"Width"), Ok(lopdf::Object::Integer(4)))
        }
        _ => false,
    });
    assert!(
        has_cropped_image,
        "crop_rect must crop the embedded image XObject"
    );
}

#[test]
fn test_pdfx_flattens_transparent_image_fills() {
    use irasu_illustrator::core::path::ImageTileMode;

    let mut doc = Document::default();
    doc.width = 80.0;
    doc.height = 50.0;
    let mut source = Object::new_image("Source", 0.0, 0.0, 8.0, 8.0, tiny_png());
    source.id = "transparent-image-fill-source".to_string();
    let source_id = source.id.clone();
    doc.add_object(source);
    let mut target = Object::new_rect("Transparent image fill", 30.0, 10.0, 30.0, 20.0, 0.0);
    target.fill = Some(FillStyle::image_fill(&source_id, ImageTileMode::Cover));
    doc.add_object(target);

    let (pdf, warnings) = export_pdf_print(&doc, &PrintPdfOptions::default());
    assert!(
        !warnings
            .iter()
            .any(|warning| warning.starts_with("PDF/X-1a違反")),
        "transparent image fill should be flattened before PDF/X validation: {warnings:?}"
    );
    assert!(
        !warnings.iter().any(|warning| warning.contains("単色近似")),
        "image fill must not fall back to a flat color: {warnings:?}"
    );
    assert!(irasu_illustrator::io::pdf_print::validate_pdfx(&pdf, false).is_empty());
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

fn solid_yellow_png() -> Vec<u8> {
    let mut buf = Vec::new();
    image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 0, 255]))
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .unwrap();
    buf
}

fn ink_percent(doc: &Document) -> String {
    preflight(doc)
        .iter()
        .find(|issue| issue.check == "インキ総量")
        .unwrap()
        .detail
        .split('（')
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn test_preflight_flags_paint_level_transparency() {
    // Object-level opacity is the old check; paint-level alpha (translucent
    // fills incl. gradient stops, strokes) and alpha rasters must also trip
    // the 透明・効果 gate, or the PDF/X flatten runs where preflight is silent.
    let mut doc = Document::default();
    doc.bleed = mm_to_pt(3.0);
    // 1. translucent solid fill.
    let mut a = Object::new_rect("A", 0.0, 0.0, 10.0, 10.0, 0.0);
    a.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 0.5]));
    doc.add_object(a);
    // 2. gradient with a translucent stop.
    let mut b = Object::new_rect("B", 20.0, 0.0, 10.0, 10.0, 0.0);
    b.fill = Some(FillStyle::linear_gradient(LinearGradient {
        start_x: 0.0,
        start_y: 0.5,
        end_x: 1.0,
        end_y: 0.5,
        stops: vec![
            GradientStop {
                offset: 0.0,
                color: [0.0, 0.0, 0.0, 1.0],
            },
            GradientStop {
                offset: 1.0,
                color: [0.0, 0.0, 0.0, 0.0],
            },
        ],
    }));
    doc.add_object(b);
    // 3. translucent stroke, no fill.
    let mut c = Object::new_rect("C", 40.0, 0.0, 10.0, 10.0, 0.0);
    c.fill = None;
    c.stroke = Some(StrokeStyle {
        color: [0.0, 0.0, 0.0, 0.5],
        width: 1.0,
        ..Default::default()
    });
    doc.add_object(c);
    // 4. nested inside a group.
    let mut g = Object::new_rect("G", 60.0, 0.0, 10.0, 10.0, 0.0);
    g.fill = Some(FillStyle::solid([0.0, 0.0, 1.0, 0.3]));
    doc.add_object(Object::new_group("grp", vec![g]));
    // 5. translucent pattern paint.
    let mut p = Object::new_rect("P", 80.0, 0.0, 10.0, 10.0, 0.0);
    p.fill = Some(FillStyle {
        color: [1.0, 0.0, 0.0, 0.5],
        fill_type: FillType::Pattern(Default::default()),
        ..FillStyle::default()
    });
    doc.add_object(p);
    // 6+7. alpha image fill plus its alpha source.
    let mut source = Object::new_image("Src", 0.0, 30.0, 8.0, 8.0, tiny_png());
    source.id = "alpha-src".to_string();
    doc.add_object(source);
    let mut t = Object::new_rect("T", 100.0, 0.0, 10.0, 10.0, 0.0);
    t.fill = Some(FillStyle {
        fill_type: FillType::Image(ImageFill {
            image_id: "alpha-src".to_string(),
            tile_mode: ImageTileMode::Cover,
            crop_rect: None,
        }),
        ..FillStyle::default()
    });
    doc.add_object(t);

    let issues = preflight(&doc);
    let hit = issues
        .iter()
        .find(|issue| issue.check == "透明・効果")
        .expect("paint-level transparency must trip preflight");
    assert_eq!(hit.level, PreflightLevel::Warn);
    assert!(
        hit.detail.contains("7件"),
        "every alpha carrier counts once: {hit:?}"
    );
}

#[test]
fn test_preflight_flags_unembeddable_text() {
    use irasu_illustrator::core::document::TextStyle;
    // Vertical writing with the default (installed) family: the exporter
    // outlines it instead of embedding, so preflight must say so.
    let mut doc = Document::default();
    doc.bleed = mm_to_pt(3.0);
    let mut style = TextStyle::default();
    style.vertical = true;
    doc.add_object(Object::new_text_with_style("V", "縦書き", 0.0, 0.0, style));
    let issues = preflight(&doc);
    assert!(
        issues
            .iter()
            .any(|i| i.level == PreflightLevel::Warn && i.check == "フォント埋込"),
        "outline fallback is reported: {issues:?}"
    );
    // Plain text with the same family embeds: no such warning.
    let mut clean = Document::default();
    clean.bleed = mm_to_pt(3.0);
    clean.add_object(Object::new_text("T", "Hello", 10.0, 40.0, 24.0));
    let issues = preflight(&clean);
    assert!(
        issues.iter().all(|i| i.check != "フォント埋込"),
        "embeddable text stays silent: {issues:?}"
    );
}

#[test]
fn test_image_fill_ink_matches_solid_fill_conversion() {
    // The image-fill ink scan must use the same RGB→CMYK conversion as solid
    // fills: an opaque yellow raster and a yellow flat must report the same
    // worst TAC, or plates and report disagree at the 320% boundary.
    let mut solid_doc = Document::default();
    solid_doc.color_mode = ColorMode::Cmyk;
    let mut r = Object::new_rect("R", 0.0, 0.0, 20.0, 20.0, 0.0);
    r.fill = Some(FillStyle::solid([1.0, 1.0, 0.0, 1.0]));
    solid_doc.add_object(r);

    let mut img_doc = Document::default();
    img_doc.color_mode = ColorMode::Cmyk;
    let mut source = Object::new_image("Src", 0.0, 30.0, 8.0, 8.0, solid_yellow_png());
    source.id = "yellow-src".to_string();
    img_doc.add_object(source);
    let mut t = Object::new_rect("T", 30.0, 0.0, 20.0, 20.0, 0.0);
    t.fill = Some(FillStyle {
        fill_type: FillType::Image(ImageFill {
            image_id: "yellow-src".to_string(),
            tile_mode: ImageTileMode::Cover,
            crop_rect: None,
        }),
        ..FillStyle::default()
    });
    img_doc.add_object(t);

    assert_eq!(ink_percent(&solid_doc), ink_percent(&img_doc));
}

#[test]
fn test_preflight_flags_unscanned_oversize_image_fill() {
    // 4200² = 17.64Mpx > 16Mpx scan budget: the ink math must warn instead
    // of silently passing a TAC blind spot. Solid color keeps the PNG small.
    let big = image::RgbaImage::from_pixel(4200, 4200, image::Rgba([10u8, 20u8, 30u8, 255u8]));
    let mut buf = Vec::new();
    big.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .unwrap();
    let mut doc = Document::default();
    doc.color_mode = ColorMode::Cmyk;
    doc.bleed = mm_to_pt(3.0);
    let mut source = Object::new_image("Big", 0.0, 0.0, 8.0, 8.0, buf);
    source.id = "big-src".to_string();
    doc.add_object(source);
    let mut t = Object::new_rect("T", 30.0, 0.0, 20.0, 20.0, 0.0);
    t.fill = Some(FillStyle {
        fill_type: FillType::Image(ImageFill {
            image_id: "big-src".to_string(),
            tile_mode: ImageTileMode::Cover,
            crop_rect: None,
        }),
        ..FillStyle::default()
    });
    doc.add_object(t);

    let issues = preflight(&doc);
    assert!(
        issues
            .iter()
            .any(|i| i.level == PreflightLevel::Warn && i.check == "インキ未評価"),
        "oversize image fill must not pass silently: {issues:?}"
    );
}

/// Assemble a minimal PDF with correct xref offsets for gate tests.
fn minimal_pdf(objects: &[&[u8]]) -> Vec<u8> {
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

#[test]
fn test_pdfx_gate_checks_output_intent_and_boxes() {
    // Our own press output is gate-clean, with boxes nested and the trim
    // matching the document size.
    let (pdf, warnings) = export_pdf_print(&press_doc(), &PrintPdfOptions::default());
    assert!(validate_pdfx(&pdf, true).is_empty(), "{warnings:?}");
    let doc = parse(&pdf);
    let page = match doc
        .get_object(*doc.get_pages().values().next().unwrap())
        .unwrap()
    {
        lopdf::Object::Dictionary(d) => d.clone(),
        _ => panic!("page dict"),
    };
    let rect = |key: &[u8]| match page.get(key).unwrap() {
        lopdf::Object::Array(a) => a.clone(),
        _ => panic!("box array"),
    };
    let num = |o: &lopdf::Object| match o {
        lopdf::Object::Integer(i) => *i as f64,
        lopdf::Object::Real(f) => *f as f64,
        _ => f64::NAN,
    };
    let w = |r: &[lopdf::Object]| num(&r[2]) - num(&r[0]);
    let h = |r: &[lopdf::Object]| num(&r[3]) - num(&r[1]);
    let (media, bleed, trim) = (rect(b"MediaBox"), rect(b"BleedBox"), rect(b"TrimBox"));
    assert!((w(&trim) - 200.0).abs() < 0.01 && (h(&trim) - 100.0).abs() < 0.01);
    assert!(w(&media) > w(&bleed) && w(&bleed) > w(&trim));

    // No OutputIntent at all: the gate refuses the X-1a claim.
    let bare = minimal_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /CropBox [0 0 200 100] /BleedBox [0 0 200 100] /TrimBox [0 0 200 100] /Contents 4 0 R >>",
        b"<< /Length 0 >>\nstream\n\nendstream",
    ]);
    let violations = validate_pdfx(&bare, true);
    assert!(
        violations.iter().any(|v| v.contains("OutputIntents")),
        "{violations:?}"
    );
    // TrimBox escaping the page: box inconsistency is flagged too.
    let bad_trim = minimal_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R /GTS_PDFXVersion (PDF/X-1a:2001) >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /CropBox [0 0 200 100] /BleedBox [0 0 200 100] /TrimBox [500 500 600 600] /Contents 4 0 R >>",
        b"<< /Length 0 >>\nstream\n\nendstream",
    ]);
    let violations = validate_pdfx(&bad_trim, true);
    assert!(
        violations.iter().any(|v| v.contains("TrimBox")),
        "{violations:?}"
    );
}
