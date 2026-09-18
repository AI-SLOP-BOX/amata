use std::path::PathBuf;

pub fn load_any_document(
    path: &PathBuf,
) -> Result<crate::core::document::Document, Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "svg" {
        let content = std::fs::read_to_string(path)?;
        Ok(crate::io::svg::parse_svg_document(&content))
    } else {
        crate::io::project::load_project(path).map_err(|e| e.into())
    }
}

pub fn save_any_document(
    doc: &crate::core::document::Document,
    path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    save_any_document_scaled(doc, path, 1.0)
}

pub fn save_any_document_scaled(
    doc: &crate::core::document::Document,
    path: &PathBuf,
    scale: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "svg" => {
            // Raster formats consume `scale` directly; for SVG the document
            // itself must be scaled or the flag would be silently ignored.
            let svg = if (scale - 1.0).abs() > f32::EPSILON
                && scale.is_finite()
                && scale > 0.0
            {
                let mut scaled = doc.clone();
                let s = scale.clamp(0.1, 16.0) as f64;
                scaled.width *= s;
                scaled.height *= s;
                for (_, obj) in scaled.all_objects_mut() {
                    obj.transform.x *= s;
                    obj.transform.y *= s;
                    obj.transform.scale_x *= s;
                    obj.transform.scale_y *= s;
                }
                crate::io::svg::export_svg(&scaled)
            } else {
                crate::io::svg::export_svg(doc)
            };
            std::fs::write(path, svg)?;
            Ok(())
        }
        "png" => {
            let png_bytes = crate::io::raster::export_png(doc, scale, true)
                .map_err(|e| format!("Raster export failed: {e}"))?;
            std::fs::write(path, png_bytes)?;
            Ok(())
        }
        "jpg" | "jpeg" => {
            let jpeg_bytes = crate::io::raster::export_jpeg(doc, scale)
                .map_err(|e| format!("Raster export failed: {e}"))?;
            std::fs::write(path, jpeg_bytes)?;
            Ok(())
        }
        "pdf" => {
            let pdf_bytes = crate::io::pdf::export_pdf(doc);
            std::fs::write(path, pdf_bytes)?;
            Ok(())
        }
        _ => crate::io::project::save_project(doc, path).map_err(|e| e.into()),
    }
}
