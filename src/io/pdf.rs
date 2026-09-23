use crate::core::document::{Document, Object, ObjectType};
use crate::core::path::{FillStyle, PathData, PathElement};
use std::fmt::Write;

fn affine_mul(m1: &[f64; 6], m2: &[f64; 6]) -> [f64; 6] {
    [
        m1[0] * m2[0] + m1[2] * m2[1],
        m1[1] * m2[0] + m1[3] * m2[1],
        m1[0] * m2[2] + m1[2] * m2[3],
        m1[1] * m2[2] + m1[3] * m2[3],
        m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
        m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
    ]
}

fn emit_filled_path(stream_content: &mut String, path: &PathData) {
    if path.elements.is_empty() {
        return;
    }

    let _ = writeln!(stream_content, "q");

    let has_fill = if let Some(fill) = &path.fill {
        let [r, g, b, _] = fill.color;
        let _ = writeln!(stream_content, "{:.3} {:.3} {:.3} rg", r, g, b);
        true
    } else {
        false
    };

    let has_stroke = if let Some(stroke) = &path.stroke {
        let [r, g, b, _] = stroke.color;
        let _ = writeln!(stream_content, "{:.3} {:.3} {:.3} RG", r, g, b);
        let _ = writeln!(stream_content, "{:.2} w", stroke.width);
        true
    } else {
        false
    };

    for elem in &path.elements {
        match elem {
            PathElement::MoveTo(p) => {
                let _ = writeln!(stream_content, "{:.2} {:.2} m", p.x, p.y);
            }
            PathElement::LineTo(p) => {
                let _ = writeln!(stream_content, "{:.2} {:.2} l", p.x, p.y);
            }
            PathElement::CurveTo(seg) => {
                let _ = writeln!(
                    stream_content,
                    "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c",
                    seg.control1.x,
                    seg.control1.y,
                    seg.control2.x,
                    seg.control2.y,
                    seg.end.x,
                    seg.end.y
                );
            }
            PathElement::ClosePath => {
                let _ = writeln!(stream_content, "h");
            }
        }
    }

    if path.closed {
        let _ = writeln!(stream_content, "h");
    }

    match (has_fill, has_stroke) {
        (true, true) => {
            let _ = writeln!(stream_content, "B");
        }
        (true, false) => {
            let _ = writeln!(stream_content, "f");
        }
        (false, true) => {
            let _ = writeln!(stream_content, "S");
        }
        (false, false) => {
            let _ = writeln!(stream_content, "n");
        }
    }

    let _ = writeln!(stream_content, "Q");
}

fn render_obj_pdf(obj: &Object, parent: &[f64; 6], stream_content: &mut String) {
    if !obj.visible {
        return;
    }
    let world = affine_mul(parent, &obj.transform.matrix());

    match &obj.object_type {
        // Recurse so nested children keep their own fills, strokes and text.
        // (The previous flattening via to_path_data() painted whole groups
        // with the default fill and reduced text to its bounding box.)
        ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
            for child in children {
                render_obj_pdf(child, &world, stream_content);
            }
        }
        ObjectType::Text { text, style, .. } => {
            let mut path =
                crate::core::text_path::text_to_outline_path_with_style(text, style);
            path.transform(&world);
            // Object fill wins (canvas renders obj.fill); path fill is the
            // fallback. (PathData::new defaults to black, so testing
            // is_none() here would ignore every object-level fill edit.)
            if let Some(f) = obj.fill.clone().or(path.fill.clone()) {
                path.fill = Some(f);
            }
            // Outlines carry no stroke; keep an explicit text stroke if set.
            if path.stroke.is_none() {
                path.stroke = obj.stroke.clone();
            }
            emit_filled_path(stream_content, &path);
        }
        ObjectType::GradientMesh(m) => {
            // Bake flat quads (same approach as the SVG exporter: mesh
            // shadings don't exist in the PDF imaging model as such).
            // Corners go through `world` only — applying obj.transform on top
            // of world double-transformed every mesh.
            for (corners, color) in m.quads(6) {
                let mut path = PathData::new();
                for (i, p) in corners.iter().enumerate() {
                    let x = world[0] * p.x + world[2] * p.y + world[4];
                    let y = world[1] * p.x + world[3] * p.y + world[5];
                    if i == 0 {
                        path.push_move_to(x, y);
                    } else {
                        path.push_line_to(x, y);
                    }
                }
                path.elements.push(PathElement::ClosePath);
                path.fill = Some(FillStyle::solid(color));
                emit_filled_path(stream_content, &path);
            }
        }
        _ => {
            let mut path = obj.to_path_data();
            path.transform(&world);
            if let Some(f) = obj.fill.clone().or(path.fill.clone()) {
                path.fill = Some(f);
            }
            if path.stroke.is_none() {
                path.stroke = obj.stroke.clone();
            }
            emit_filled_path(stream_content, &path);
        }
    }
}

/// Export Document into pure standards-compliant Vector PDF format
pub fn export_pdf(doc: &Document) -> Vec<u8> {
    export_pdf_with_profile(doc, None)
}

/// Emit a PDF, optionally recording the requested ICC profile name as a
/// PDF comment (real ICC embedding needs profile blobs we do not vendor).
pub fn export_pdf_with_profile(doc: &Document, color_profile: Option<&str>) -> Vec<u8> {
    let w = doc.width.max(10.0);
    let h = doc.height.max(10.0);

    let mut stream_content = String::new();

    // Flip Y axis so origin (0,0) matches vector canvas top-left
    let _ = writeln!(stream_content, "q");
    let _ = writeln!(stream_content, "1 0 0 -1 0 {:.2} cm", h);

    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }

        for obj in &layer.objects {
            render_obj_pdf(obj, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], &mut stream_content);
        }
    }

    let _ = writeln!(stream_content, "Q");

    // Assemble complete PDF file
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    if let Some(profile) = color_profile.filter(|p| !p.trim().is_empty()) {
        // PDF comments start with `%`; keep them on one line.
        let sanitized: String = profile.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
        pdf.extend_from_slice(format!("% color-profile: {sanitized}\n").as_bytes());
    }

    let mut offsets = Vec::new();

    // 1 0 obj: Catalog
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // 2 0 obj: Pages
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    // 3 0 obj: Page
    offsets.push(pdf.len());
    let page_obj = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Contents 4 0 R >>\nendobj\n",
        w, h
    );
    pdf.extend_from_slice(page_obj.as_bytes());

    // 4 0 obj: Content Stream
    offsets.push(pdf.len());
    let stream_bytes = stream_content.as_bytes();
    let stream_obj = format!(
        "4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        stream_bytes.len(),
        stream_content
    );
    pdf.extend_from_slice(stream_obj.as_bytes());

    // XRef Table
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for off in offsets {
        let entry = format!("{:010} 00000 n \n", off);
        pdf.extend_from_slice(entry.as_bytes());
    }

    // Trailer
    let trailer = format!(
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        xref_offset
    );
    pdf.extend_from_slice(trailer.as_bytes());

    pdf
}
