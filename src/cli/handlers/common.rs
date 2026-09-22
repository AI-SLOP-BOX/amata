use std::path::Path;

pub fn load_any_document(
    path: &Path,
) -> Result<crate::core::document::Document, Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "svg" {
        let content = std::fs::read_to_string(path)?;
        Ok(crate::io::svg::parse_svg_document(&content))
    } else if ext == "pdf" {
        let bytes = std::fs::read(path)?;
        crate::io::pdf_import::parse_pdf_bytes(&bytes)
            .map(|(doc, _)| doc)
            .map_err(|e| e.into())
    } else {
        crate::io::project::load_project(path).map_err(|e| e.into())
    }
}

pub fn save_any_document(
    doc: &crate::core::document::Document,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    save_any_document_scaled(doc, path, 1.0)
}

pub fn save_any_document_scaled(
    doc: &crate::core::document::Document,
    path: &Path,
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
            crate::io::atomic::atomic_write_str(path, &svg)?;
            Ok(())
        }
        "png" => {
            let png_bytes = crate::io::raster::export_png(doc, scale, true)
                .map_err(|e| format!("Raster export failed: {e}"))?;
            crate::io::atomic::atomic_write_bytes(path, &png_bytes)?;
            Ok(())
        }
        "jpg" | "jpeg" => {
            let jpeg_bytes = crate::io::raster::export_jpeg(doc, scale)
                .map_err(|e| format!("Raster export failed: {e}"))?;
            crate::io::atomic::atomic_write_bytes(path, &jpeg_bytes)?;
            Ok(())
        }
        "webp" => {
            let webp_bytes = crate::io::raster::export_webp(doc, scale, true)
                .map_err(|e| format!("Raster export failed: {e}"))?;
            crate::io::atomic::atomic_write_bytes(path, &webp_bytes)?;
            Ok(())
        }
        "avif" => {
            let avif_bytes = crate::io::raster::export_avif(doc, scale, true)
                .map_err(|e| format!("Raster export failed: {e}"))?;
            crate::io::atomic::atomic_write_bytes(path, &avif_bytes)?;
            Ok(())
        }
        "pdf" => {
            let pdf_bytes = crate::io::pdf::export_pdf(doc);
            crate::io::atomic::atomic_write_bytes(path, &pdf_bytes)?;
            Ok(())
        }
        _ => crate::io::project::save_project(doc, path).map_err(|e| e.into()),
    }
}
