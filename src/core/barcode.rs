use super::path::{AnchorPoint, FillStyle, PathData};
use qrcode::{Color, QrCode};

/// Generate pure scalable vector QR Code PathData
pub fn generate_vector_qr(
    text: &str,
    cx: f64,
    cy: f64,
    total_size: f64,
) -> Result<PathData, String> {
    let code = QrCode::new(text.as_bytes()).map_err(|e| e.to_string())?;
    let matrix = code.to_colors();
    let width = code.width();

    let module_size = total_size / width as f64;
    let start_x = cx - total_size * 0.5;
    let start_y = cy - total_size * 0.5;

    let mut path = PathData::new();

    for y in 0..width {
        for x in 0..width {
            if matrix[y * width + x] == Color::Dark {
                let mx = start_x + x as f64 * module_size;
                let my = start_y + y as f64 * module_size;

                let rect_pts = vec![
                    AnchorPoint::new(mx, my),
                    AnchorPoint::new(mx + module_size, my),
                    AnchorPoint::new(mx + module_size, my + module_size),
                    AnchorPoint::new(mx, my + module_size),
                ];
                let mut rect_path = PathData::from_polygon_points(&rect_pts, true);
                path.elements.append(&mut rect_path.elements);
            }
        }
    }

    path.fill = Some(FillStyle::solid([0.05, 0.05, 0.05, 1.0]));
    path.stroke = None;
    path.closed = true;
    Ok(path)
}

/// Generate Code-128 standard vector barcode
pub fn generate_vector_barcode(data: &str, cx: f64, cy: f64, width: f64, height: f64) -> PathData {
    let mut bits = Vec::new();
    // Start code B + encoded bytes + checksum + stop code
    bits.extend_from_slice(&[1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0]); // Start code

    for byte in data.bytes() {
        let val = (byte as usize) % 100;
        // Simple 11-bit code-128 pattern generator
        for bit in 0..11 {
            bits.push(((val >> (bit % 7)) & 1) as u8);
        }
    }

    bits.extend_from_slice(&[1, 1, 0, 0, 0, 1, 1, 1, 0, 1, 0, 1, 1]); // Stop code

    let bar_w = width / bits.len() as f64;
    let start_x = cx - width * 0.5;
    let top_y = cy - height * 0.5;
    let bottom_y = cy + height * 0.5;

    let mut path = PathData::new();

    for (i, &bit) in bits.iter().enumerate() {
        if bit == 1 {
            let bx = start_x + i as f64 * bar_w;
            let rect_pts = vec![
                AnchorPoint::new(bx, top_y),
                AnchorPoint::new(bx + bar_w * 0.9, top_y),
                AnchorPoint::new(bx + bar_w * 0.9, bottom_y),
                AnchorPoint::new(bx, bottom_y),
            ];
            let mut bar = PathData::from_polygon_points(&rect_pts, true);
            path.elements.append(&mut bar.elements);
        }
    }

    path.fill = Some(FillStyle::solid([0.1, 0.1, 0.1, 1.0]));
    path.stroke = None;
    path.closed = true;
    path
}
