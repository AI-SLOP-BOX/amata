//! Stroke width has to ride the transform the exporters bake geometry with.
//!
//! The canvas builds a stroke in object-local units and maps it through the
//! object transform, so a 2× object shows a 2× heavy line. The exporters bake
//! that same transform into the path data; writing the saved width verbatim
//! would export a fraction of the on-screen weight.

use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::path::{PathData, StrokeStyle};
use irasu_illustrator::io::pdf::export_pdf;
use irasu_illustrator::io::pdf_print::{export_pdf_print, PrintPdfOptions};
use irasu_illustrator::io::svg::export_svg;

const WIDTH: f64 = 2.0;

/// Every `w` (line width) operator of a content stream, in order.
fn width_ops(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim_end)
        .filter(|line| line.ends_with(" w"))
        .collect()
}

/// A stroked rectangle, scaled uniformly by `scale`.
fn stroked_rect(scale: f64) -> Document {
    let mut doc = Document {
        width: 400.0,
        height: 400.0,
        ..Default::default()
    };
    let mut rect = Object::new_rect("R", 20.0, 20.0, 120.0, 80.0, 0.0);
    rect.stroke = Some(StrokeStyle {
        width: WIDTH,
        ..Default::default()
    });
    rect.transform.scale_x = scale;
    rect.transform.scale_y = scale;
    doc.add_object(rect);
    doc
}

/// Same, with a dash pattern so the spacing is checked alongside the width.
fn dashed_rect(scale: f64) -> Document {
    let mut doc = stroked_rect(scale);
    doc.layers[0].objects[0]
        .stroke
        .as_mut()
        .unwrap()
        .dash_pattern = Some(vec![4.0, 2.0]);
    doc
}

/// A stroked line exported as a baked `<path d="...">` (no `transform=`).
fn stroked_path(scale: f64, dashed: bool) -> Document {
    let mut doc = Document {
        width: 400.0,
        height: 400.0,
        ..Default::default()
    };
    let mut line = PathData::new();
    line.push_move_to(0.0, 0.0);
    line.push_line_to(100.0, 0.0);
    let mut obj = Object::new_path("P", line);
    obj.stroke = Some(StrokeStyle {
        width: WIDTH,
        dash_pattern: if dashed { Some(vec![4.0, 2.0]) } else { None },
        ..Default::default()
    });
    obj.transform.scale_x = scale;
    obj.transform.scale_y = scale;
    doc.add_object(obj);
    doc
}

/// The page's content streams as text: Flate-encoded ones are decompressed,
/// and embedded binary payloads (ICC profiles, images) are skipped so only
/// the operators of the page content are returned.
fn press_streams(pdf: &[u8]) -> String {
    let parsed = lopdf::Document::load_mem(pdf).expect("press PDF parses");
    let mut out = String::new();
    for (id, obj) in &parsed.objects {
        let lopdf::Object::Stream(stream) = obj else {
            continue;
        };
        let bytes = if stream.dict.get(b"Filter").is_ok() {
            match stream.decompressed_content() {
                Ok(bytes) => bytes,
                Err(err) => {
                    panic!("content stream {id:?} does not decode: {err}");
                }
            }
        } else {
            // `decompressed_content` errors without a /Filter; the press path
            // leaves short content streams uncompressed.
            stream.content.clone()
        };
        let text = String::from_utf8_lossy(&bytes);
        if text.contains('\u{FFFD}') {
            continue;
        }
        out.push_str(&text);
    }
    out
}

#[test]
fn legacy_pdf_scales_the_stroke_with_the_baked_geometry() {
    let scaled = String::from_utf8_lossy(&export_pdf(&stroked_rect(2.0))).into_owned();
    assert_eq!(
        width_ops(&scaled),
        vec!["4.00 w"],
        "a 2× object must stroke at 4.00 pt"
    );

    let plain = String::from_utf8_lossy(&export_pdf(&stroked_rect(1.0))).into_owned();
    assert_eq!(
        width_ops(&plain),
        vec!["2.00 w"],
        "an unscaled object keeps its stored width"
    );
}

#[test]
fn print_pdf_scales_the_stroke_with_the_baked_geometry() {
    let opts = PrintPdfOptions {
        marks: false,
        bleed: Some(0.0),
        pdfx: false,
        outline_text: false,
    };
    let (pdf, _) = export_pdf_print(&stroked_rect(2.0), &opts);
    assert_eq!(
        width_ops(&press_streams(&pdf)),
        vec!["4.00 w"],
        "the press path bakes world geometry too"
    );
}

#[test]
fn print_pdf_scales_the_dash_spacing_with_the_baked_geometry() {
    let opts = PrintPdfOptions {
        marks: false,
        bleed: Some(0.0),
        pdfx: false,
        outline_text: false,
    };
    let (pdf, _) = export_pdf_print(&dashed_rect(2.0), &opts);
    let streams = press_streams(&pdf);
    assert!(
        streams.contains("[8.00 4.00] 0 d"),
        "dashes live in the baked space: {streams}"
    );
}

#[test]
fn svg_baked_paths_scale_the_stroke_and_dashes() {
    let svg = export_svg(&stroked_path(2.0, false));
    assert!(
        svg.contains("stroke-width=\"4\""),
        "baked `d` carries the 2× transform: {svg}"
    );

    let dashed = export_svg(&stroked_path(2.0, true));
    assert!(
        dashed.contains("stroke-dasharray=\"8 4\""),
        "dashes ride the same transform: {dashed}"
    );
}

#[test]
fn svg_transform_attr_branches_leave_the_stroke_alone() {
    // `<rect>` keeps `transform=`, which already scales the stroke — the
    // stored width must reach the file untouched.
    let svg = export_svg(&stroked_rect(2.0));
    assert!(
        svg.contains("stroke-width=\"2\""),
        "the transform scales it, not the attribute: {svg}"
    );
    assert!(svg.contains("transform=\"matrix("), "{svg}");
}
