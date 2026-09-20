//! Raster export + atomic-write hardening.
//!
//! Covers two previously-silent failure modes:
//!  * `export_png` clamped the pixmap to 16384 px while keeping the requested
//!    render scale, so oversized exports came back cropped.
//!  * `atomic_write_bytes` left a half-written `.tmp` sibling behind when the
//!    write or the rename failed.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::path::FillStyle;
use irasu_illustrator::io::atomic::{atomic_write_bytes, atomic_write_str};
use irasu_illustrator::io::raster::{export_png, export_png_with_limit};

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn decoded_size(png: &[u8]) -> (u32, u32) {
    let image = image::load_from_memory(png).expect("PNG must decode");
    (image.width(), image.height())
}

#[test]
fn test_png_export_honours_scale_below_limit() {
    let mut doc = Document::default();
    doc.width = 20.0;
    doc.height = 10.0;

    let png = export_png(&doc, 3.0, true).expect("PNG export");
    assert_eq!(decoded_size(&png), (60, 30));
}

#[test]
fn test_png_export_clamps_scale_instead_of_cropping() {
    // Right half of a 100x50 canvas is red; the left half stays transparent.
    let mut doc = Document::default();
    doc.width = 100.0;
    doc.height = 50.0;
    let mut rect = Object::new_rect("RightHalf", 50.0, 0.0, 50.0, 50.0, 0.0);
    rect.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
    doc.add_object(rect);

    // 4x on a 100px canvas wants 400px; the 64px cap must scale it down to
    // 0.64x uniformly rather than render at 4x into a 64px pixmap.
    let png = export_png_with_limit(&doc, 4.0, false, 64).expect("PNG export");
    assert_eq!(decoded_size(&png), (64, 32), "aspect ratio must be preserved");

    let image = image::load_from_memory(&png).unwrap().to_rgba8();
    let right = image.get_pixel(60, 16).0;
    let left = image.get_pixel(4, 16).0;
    assert!(
        right[0] > 200 && right[1] < 60,
        "right half must still contain the red rect (got {right:?}) — the image was cropped"
    );
    assert!(
        left[0] > 200 && left[1] > 200 && left[2] > 200,
        "left half must be the white canvas background (got {left:?})"
    );
}

#[test]
fn test_atomic_write_str_replaces_file_without_leaving_tmp() {
    let dir = scratch_dir("amata_atomic_ok");
    let target = dir.join("doc.svg");

    atomic_write_str(&target, "<svg>first</svg>").unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "<svg>first</svg>");
    assert!(!dir.join("doc.svg.tmp").exists(), "no .tmp sibling after success");

    // Overwriting an existing file must work too (rename over the old one).
    atomic_write_bytes(&target, b"<svg>second</svg>").unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "<svg>second</svg>");
    assert!(!dir.join("doc.svg.tmp").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_atomic_write_failure_cleans_up_tmp_file() {
    let dir = scratch_dir("amata_atomic_fail");

    // Destination is a directory: writing the temp file succeeds but the
    // rename fails, which is exactly the path that used to leak `<x>.tmp`.
    let target = dir.join("sub");
    std::fs::create_dir_all(&target).unwrap();

    let result = atomic_write_str(&target, "payload");
    assert!(result.is_err(), "writing over a directory must fail");
    assert!(
        !dir.join("sub.tmp").exists(),
        "the temporary file must be removed when the write fails"
    );

    // Missing parent directory: fails before creating anything.
    let missing = dir.join("no-such-parent").join("doc.svg");
    assert!(atomic_write_str(&missing, "payload").is_err());
    assert!(!dir.join("no-such-parent").exists());

    let _ = std::fs::remove_dir_all(&dir);
}
