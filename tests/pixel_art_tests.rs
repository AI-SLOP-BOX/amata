//! Pixel-art (dot絵) pipeline: model, document integration, SVG export.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::pixel::{PixelArt, TRANSPARENT};
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};

fn sample_art() -> PixelArt {
    let mut p = PixelArt::new(8, 8, PixelArt::pico8_palette());
    // Red diagonal (palette index 8 = 0xFF004D).
    p.stroke_line(0, 0, 7, 7, 8);
    p
}

#[test]
fn test_new_pixel_art_object_basics() {
    let obj = Object::new_pixel_art("Dot", 10.0, 20.0, sample_art());
    assert!((obj.transform.x - 10.0).abs() < 1e-9);
    assert!((obj.transform.y - 20.0).abs() < 1e-9);
    assert!(obj.hit_test(10.0, 20.0), "top-left corner hits");
    assert!(obj.hit_test(17.9, 27.9), "inside the 8x8 grid hits");
    assert!(!obj.hit_test(18.1, 20.0), "outside misses");
    let (mn, mx) = obj.bounding_box().expect("bbox");
    assert!((mx.x - mn.x - 8.0).abs() < 1e-9);
    assert!((mx.y - mn.y - 8.0).abs() < 1e-9);
}

#[test]
fn test_pixel_serialization_round_trip() {
    let mut doc = Document::default();
    doc.add_object(Object::new_pixel_art("Dot", 0.0, 0.0, sample_art()));
    let json = serde_json::to_string(&doc).unwrap();
    let doc2: Document = serde_json::from_str(&json).unwrap();
    let found = doc2
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| matches!(o.object_type, ObjectType::PixelArt(_)))
        .expect("pixel object survives JSON");
    if let ObjectType::PixelArt(p) = &found.object_type {
        assert_eq!((p.width, p.height), (8, 8));
        assert_eq!(p.palette.len(), 16);
        assert_eq!(p.get(3, 3), 8, "diagonal survives");
        assert_eq!(p.get(0, 7), TRANSPARENT, "empty stays empty");
    }
}

#[test]
fn test_pixel_svg_export_embeds_crisp_image() {
    let mut doc = Document::default();
    doc.width = 64.0;
    doc.height = 64.0;
    doc.add_object(Object::new_pixel_art("Dot", 4.0, 4.0, sample_art()));
    let svg = export_svg(&doc);
    assert!(svg.contains("<image"), "pixel art exports as <image>");
    assert!(svg.contains("data:image/png;base64,"), "self-contained");
    assert!(
        svg.contains("image-rendering=\"pixelated\""),
        "integer-scale raster exports must stay Nearest"
    );
    assert!(svg.contains("width=\"8\" height=\"8\""), "exact grid size");
}

#[test]
fn test_pixel_svg_reimports_as_image() {
    // No information loss: the importer already handles embedded PNGs, so
    // a pixel layer round-trips as a placed image (documented behaviour).
    let mut doc = Document::default();
    doc.add_object(Object::new_pixel_art("Dot", 4.0, 4.0, sample_art()));
    let svg = export_svg(&doc);
    let doc2 = parse_svg_document(&svg);
    let img = doc2
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| matches!(o.object_type, ObjectType::Image { .. }))
        .expect("reimports as an image object");
    if let ObjectType::Image {
        width,
        height,
        png_bytes,
    } = &img.object_type
    {
        assert!((*width - 8.0).abs() < 1e-9);
        assert!((*height - 8.0).abs() < 1e-9);
        let decoded = image::load_from_memory(png_bytes).unwrap().to_rgba8();
        // The red diagonal pixel decodes back (alpha intact).
        let px = decoded.get_pixel(3, 3).0;
        assert!(px[0] > 200 && px[3] == 255, "diagonal pixel: {px:?}");
        assert_eq!(decoded.get_pixel(0, 7).0[3], 0, "empty stays transparent");
    }
}

#[test]
fn test_pixel_png_export_keeps_hard_edges() {
    use irasu_illustrator::io::raster::export_png;
    let mut doc = Document::default();
    doc.width = 8.0;
    doc.height = 8.0;
    doc.add_object(Object::new_pixel_art("Dot", 0.0, 0.0, sample_art()));
    // 4x integer upscale: Nearest filtering must keep blocky dots.
    let png = export_png(&doc, 4.0, true).expect("PNG export");
    let img = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!((img.width(), img.height()), (32, 32));
    // A full 4x4 block deep inside the diagonal is uniformly red.
    for (x, y) in [(13, 13), (14, 14), (15, 15)] {
        let px = img.get_pixel(x, y).0;
        assert!(px[0] > 200 && px[1] < 80 && px[3] == 255, "block pixel {x},{y}: {px:?}");
    }
    // Off-diagonal stays transparent.
    assert_eq!(img.get_pixel(2, 28).0[3], 0);
}

#[test]
fn test_pixel_checksum_detects_edits() {
    let mut p = sample_art();
    let before = p.checksum();
    p.set(0, 7, 1);
    assert_ne!(p.checksum(), before);
}
