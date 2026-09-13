use crate::core::document::Document;
use crate::core::path::PathElement;
use std::fmt::Write;

/// Export Document into pure standards-compliant Vector PDF format
pub fn export_pdf(doc: &Document) -> Vec<u8> {
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
            if !obj.visible {
                continue;
            }

            let mut path = obj.to_path_data();
            path.transform(&obj.transform.matrix());

            if path.elements.is_empty() {
                continue;
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
    }

    let _ = writeln!(stream_content, "Q");

    // Assemble complete PDF file
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

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
