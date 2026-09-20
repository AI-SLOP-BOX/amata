//! Image fills: SVG export geometry per tile mode.
//!
//! Covers three previously-broken behaviours:
//!  * `preserveAspectRatio` received bare `slice`/`repeat` values, which are
//!    not valid SVG — viewers fell back to the default (`xMidYMid meet`).
//!  * `Fit` and `Contain` emitted the same output, so they were
//!    indistinguishable.
//!  * `<pattern>` had no `width`/`height`, which per spec means a zero-size
//!    tile (nothing paints).
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::path::{FillStyle, ImageTileMode};
use irasu_illustrator::io::svg::export_svg;

fn tiny_png() -> Vec<u8> {
    let mut buf = Vec::new();
    let img = image::RgbaImage::from_fn(4, 2, |x, y| {
        image::Rgba([(x * 40) as u8, (y * 40) as u8, 128, 255])
    });
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .unwrap();
    buf
}

fn doc_with_image_fill(mode: ImageTileMode) -> Document {
    let mut doc = Document::default();
    doc.width = 200.0;
    doc.height = 100.0;
    let (_, _, placed) =
        irasu_illustrator::io::raster::decode_placed_image(&tiny_png()).unwrap();
    let mut src = Object::new_image("Src", 0.0, 0.0, 4.0, 2.0, placed);
    src.id = "src-img".to_string();
    doc.add_object(src);
    let mut rect = Object::new_rect("R", 10.0, 10.0, 80.0, 40.0, 0.0);
    rect.fill = Some(FillStyle::image_fill("src-img", mode));
    doc.add_object(rect);
    doc
}

fn pattern_tag(svg: &str) -> &str {
    let start = svg.find("<pattern").expect("pattern must be emitted");
    let end = svg[start..].find('>').expect("pattern tag closes") + start + 1;
    &svg[start..end]
}

#[test]
fn test_cover_uses_slice_with_bbox_tile() {
    let svg = export_svg(&doc_with_image_fill(ImageTileMode::Cover));
    assert!(svg.contains("preserveAspectRatio=\"xMidYMid slice\""));
    // Single tile exactly covering the 80x40 shape bbox at (10,10).
    let tag = pattern_tag(&svg);
    assert!(tag.contains("width=\"80.000\""), "tile width: {tag}");
    assert!(tag.contains("height=\"40.000\""), "tile height: {tag}");
}

#[test]
fn test_contain_uses_meet_and_differs_from_cover() {
    let svg = export_svg(&doc_with_image_fill(ImageTileMode::Contain));
    assert!(svg.contains("preserveAspectRatio=\"xMidYMid meet\""));
    assert_ne!(
        svg,
        export_svg(&doc_with_image_fill(ImageTileMode::Cover)),
        "Contain and Cover must not emit identical output"
    );
}

#[test]
fn test_fit_stretches_with_none_and_differs_from_contain() {
    let svg = export_svg(&doc_with_image_fill(ImageTileMode::Fit));
    assert!(svg.contains("preserveAspectRatio=\"none\""));
    assert_ne!(
        svg,
        export_svg(&doc_with_image_fill(ImageTileMode::Contain)),
        "Fit (stretch) and Contain (meet) must not emit identical output"
    );
}

#[test]
fn test_tile_repeats_at_natural_size() {
    let svg = export_svg(&doc_with_image_fill(ImageTileMode::Tile));
    let tag = pattern_tag(&svg);
    assert!(tag.contains("width=\"4.000\""), "tile width: {tag}");
    assert!(tag.contains("height=\"2.000\""), "tile height: {tag}");
}

#[test]
fn test_no_bare_invalid_preserve_aspect_ratio() {
    for mode in [
        ImageTileMode::Cover,
        ImageTileMode::Contain,
        ImageTileMode::Fit,
        ImageTileMode::Tile,
    ] {
        let svg = export_svg(&doc_with_image_fill(mode));
        for bad in ["=\"slice\"", "=\"repeat\"", "=\"meet\"", "=\"cover\"", "=\"contain\""] {
            assert!(
                !svg.contains(bad),
                "{mode:?} must not emit bare {bad}: {}",
                pattern_tag(&svg)
            );
        }
    }
}

#[test]
fn test_missing_image_falls_back_to_none_without_dangling_url() {
    let mut doc = Document::default();
    let mut rect = Object::new_rect("R", 10.0, 10.0, 80.0, 40.0, 0.0);
    rect.fill = Some(FillStyle::image_fill("no-such-image", ImageTileMode::Cover));
    doc.add_object(rect);
    let svg = export_svg(&doc);
    assert!(!svg.contains("url(#img_fill"), "no dangling paint-server ref");
    assert!(svg.contains("fill=\"none\""));
}
