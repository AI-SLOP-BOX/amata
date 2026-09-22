use crate::core::document::Document;

/// Hard cap on either output dimension. Guards against OOM on gigapixel
/// canvases (matches the CLI `render --width/--height` guard).
pub const MAX_EXPORT_DIM: u32 = 16384;

/// Export the vector document to PNG byte buffer at given scale factor.
/// `scale = 1.0` produces 1:1 pixel dimensions matching document width/height.
/// `transparent = true` keeps transparent canvas background.
///
/// When the requested scale would push either side past [`MAX_EXPORT_DIM`]
/// the scale is reduced uniformly instead of cropping the image (see
/// [`export_png_with_limit`]).
pub fn export_png(doc: &Document, scale: f32, transparent: bool) -> Result<Vec<u8>, String> {
    export_png_with_limit(doc, scale, transparent, MAX_EXPORT_DIM)
}

/// [`export_png`] with an explicit dimension cap.
///
/// The requested `scale` is honoured unless it would exceed `max_dim`; in that
/// case it is reduced uniformly so the whole canvas still fits. Previously the
/// pixmap was clamped to 16384 while the render transform kept the requested
/// scale, so a large canvas exported at a high zoom silently lost everything
/// past the 16384th pixel (right/bottom of the image was cropped away).
pub fn export_png_with_limit(
    doc: &Document,
    scale: f32,
    transparent: bool,
    max_dim: u32,
) -> Result<Vec<u8>, String> {
    let requested_scale = scale.clamp(0.1, 16.0);
    let max_dim = max_dim.max(1) as f32;
    let svg_data = crate::io::svg::export_svg(doc);
    let opt = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(crate::core::font::FontRegistry::global().database().clone()),
        ..Default::default()
    };
    let rtree = resvg::usvg::Tree::from_str(&svg_data, &opt)
        .map_err(|e| format!("Failed to parse SVG for raster export: {}", e))?;

    let base_size = rtree.size();
    let base_width = (base_size.width() as f64).max(1.0) as f32;
    let base_height = (base_size.height() as f64).max(1.0) as f32;
    let longest_side = base_width.max(base_height);
    let scale = if longest_side * requested_scale > max_dim {
        max_dim / longest_side
    } else {
        requested_scale
    };

    let target_width = ((base_width * scale).round().max(1.0) as u32).min(max_dim as u32);
    let target_height = ((base_height * scale).round().max(1.0) as u32).min(max_dim as u32);

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

/// Decode user-supplied raster bytes (PNG/JPEG) into a placeable image:
/// returns document-unit size plus re-encoded PNG bytes. Images are capped
/// at 2048px per side so a photo cannot blow up project files or textures.
pub fn decode_placed_image(bytes: &[u8]) -> Result<(f64, f64, Vec<u8>), String> {
    const MAX_DIM: u32 = 2048;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Image file too large (64MB limit)".to_string());
    }
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Could not decode image: {e}"))?;
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err("Image has zero size".to_string());
    }
    let scale = (MAX_DIM as f32 / w.max(h) as f32).min(1.0);
    let img = if scale < 1.0 {
        img.resize(
            ((w as f32 * scale).round().max(1.0)) as u32,
            ((h as f32 * scale).round().max(1.0)) as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let (w, h) = (img.width(), img.height());
    let mut png = Vec::new();
    img.to_rgba8()
        .write_to(
            &mut std::io::Cursor::new(&mut png),
            image::ImageFormat::Png,
        )
        .map_err(|e| format!("Failed to encode PNG: {e}"))?;
    Ok((w as f64, h as f64, png))
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

/// Export the vector document to WebP byte buffer at given scale factor.
/// Uses lossless WebP (via `image`'s WebP encoder), so alpha from
/// `transparent = true` is preserved.
pub fn export_webp(doc: &Document, scale: f32, transparent: bool) -> Result<Vec<u8>, String> {
    let png_bytes = export_png(doc, scale, transparent)?;
    let img = image::load_from_memory(&png_bytes)
        .map_err(|e| format!("Failed to load image for WebP encoding: {}", e))?;

    let mut webp_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut webp_bytes);
    img.write_to(&mut cursor, image::ImageFormat::WebP)
        .map_err(|e| format!("Failed to encode WebP: {}", e))?;

    Ok(webp_bytes)
}

/// Export the vector document to AVIF byte buffer at given scale factor.
/// Alpha from `transparent = true` is preserved.
pub fn export_avif(doc: &Document, scale: f32, transparent: bool) -> Result<Vec<u8>, String> {
    let png_bytes = export_png(doc, scale, transparent)?;
    let img = image::load_from_memory(&png_bytes)
        .map_err(|e| format!("Failed to load image for AVIF encoding: {}", e))?;

    let mut avif_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut avif_bytes);
    img.write_to(&mut cursor, image::ImageFormat::Avif)
        .map_err(|e| format!("Failed to encode AVIF: {}", e))?;

    Ok(avif_bytes)
}
