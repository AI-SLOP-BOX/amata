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
    } else if ext == "ai" {
        let bytes = std::fs::read(path)?;
        crate::io::pdf_import::parse_ai_bytes(&bytes)
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

/// [`save_any_document_scaled`] with text outlined to glyph paths.
/// Only SVG/PNG honour the flag; other formats ignore it.
pub fn save_any_document_scaled_outlined(
    doc: &crate::core::document::Document,
    path: &Path,
    scale: f32,
    outline_text: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "svg" => {
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
            let svg = crate::io::svg::export_svg_with_options(&scaled, false, None, outline_text);
            crate::io::atomic::atomic_write_str(path, &svg)?;
            Ok(())
        }
        "png" => {
            let bytes = crate::io::raster::export_png_with_outline(doc, scale, true, outline_text)
                .map_err(|e| e.to_string())?;
            crate::io::atomic::atomic_write_bytes(path, &bytes)?;
            Ok(())
        }
        _ => save_any_document_scaled(doc, path, scale),
    }
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
            let svg = if (scale - 1.0).abs() > f32::EPSILON && scale.is_finite() && scale > 0.0 {
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
            if !scale.is_finite() || (scale - 1.0).abs() > f32::EPSILON {
                return Err("scale is raster-only and unsupported for .pdf (use 1.0)".into());
            }
            let pdf_bytes = crate::io::pdf::export_pdf(doc);
            crate::io::atomic::atomic_write_bytes(path, &pdf_bytes)?;
            Ok(())
        }
        // PDF-compatible .ai (Illustrator opens the PDF portion; no
        // private edit data — same honest subset as the export dialog).
        // Vector formats ignore `scale` (it is a raster concept): refuse
        // a non-1.0 scale loudly instead of silently dropping it, exactly
        // like the SVG arm documents.
        "ai" => {
            if !scale.is_finite() || (scale - 1.0).abs() > f32::EPSILON {
                return Err("scale is raster-only and unsupported for .ai (use 1.0)".into());
            }
            let pdf_bytes = crate::io::pdf::export_pdf(doc);
            crate::io::atomic::atomic_write_bytes(path, &pdf_bytes)?;
            Ok(())
        }
        _ => crate::io::project::save_project(doc, path).map_err(|e| e.into()),
    }
}
