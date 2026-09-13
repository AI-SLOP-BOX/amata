use crate::core::document::Document;

/// Export the vector document to PNG byte buffer at given scale factor.
/// `scale = 1.0` produces 1:1 pixel dimensions matching document width/height.
/// `transparent = true` keeps transparent canvas background.
pub fn export_png(doc: &Document, scale: f32, transparent: bool) -> Result<Vec<u8>, String> {
    let scale = scale.clamp(0.1, 16.0);
    let svg_data = crate::io::svg::export_svg(doc);
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let rtree = resvg::usvg::Tree::from_str(&svg_data, &opt)
        .map_err(|e| format!("Failed to parse SVG for raster export: {}", e))?;

    let base_size = rtree.size();
    let target_width = ((base_size.width() * scale).round().max(1.0) as u32).min(16384);
    let target_height = ((base_size.height() * scale).round().max(1.0) as u32).min(16384);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(target_width, target_height)
        .ok_or_else(|| "Failed to allocate pixmap buffer".to_string())?;

    if !transparent {
        pixmap.fill(resvg::tiny_skia::Color::WHITE);
    }

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&rtree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| format!("Failed to encode PNG: {}", e))
}

/// Export the vector document to JPEG byte buffer at given scale factor.
pub fn export_jpeg(doc: &Document, scale: f32) -> Result<Vec<u8>, String> {
    let png_bytes = export_png(doc, scale, false)?;
    let img = image::load_from_memory(&png_bytes)
        .map_err(|e| format!("Failed to load image for JPEG encoding: {}", e))?;

    let mut jpeg_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut jpeg_bytes);
    img.to_rgb8()
        .write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to encode JPEG: {}", e))?;

    Ok(jpeg_bytes)
}
