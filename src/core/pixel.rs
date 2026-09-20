//! Pixel-art (dot絵) document model.
//!
//! A [`PixelArt`] is a fixed W×H grid of palette indices. One cell maps to
//! 1×1 document units at scale 1 (the object's `transform` positions and
//! scales it like any other object), so pixel objects compose, group, and
//! export through the same pipeline as vector shapes.
//!
//! * Cell value `255` ([`TRANSPARENT`]) means erased; palette indices are
//!   `0..=254`, so palettes hold at most 255 entries (authentic 8-bit limit).
//! * Colors are linear 0..=1 RGBA, same convention as [`FillStyle`](crate::core::path::FillStyle).
//! * Grids are capped at [`MAX_PIXEL_DIM`] per side (256×256 = 64Ki cells).

use serde::{Deserialize, Serialize};

/// Cell value meaning "no pixel here".
pub const TRANSPARENT: u8 = 255;

/// Maximum grid dimension (either side). Bounds memory (64Ki cells) and
/// keeps flood-fill / texture upload costs trivial.
pub const MAX_PIXEL_DIM: u32 = 256;

/// Maximum palette entries (index 255 is reserved for transparency).
pub const MAX_PALETTE: usize = 255;

/// A pixel-art layer: grid + palette, both serializable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PixelArt {
    pub width: u32,
    pub height: u32,
    /// Row-major cells (`y * width + x`), `TRANSPARENT` = erased.
    pub cells: Vec<u8>,
    /// Palette entries; cell value `i` paints `palette[i]`.
    pub palette: Vec<[f32; 4]>,
}

impl PixelArt {
    /// Blank grid with the given palette (at least one entry is kept so
    /// index 0 is always paintable).
    pub fn new(width: u32, height: u32, mut palette: Vec<[f32; 4]>) -> Self {
        let width = width.clamp(1, MAX_PIXEL_DIM);
        let height = height.clamp(1, MAX_PIXEL_DIM);
        if palette.is_empty() {
            palette.push([0.0, 0.0, 0.0, 1.0]);
        }
        palette.truncate(MAX_PALETTE);
        let cells = vec![TRANSPARENT; (width * height) as usize];
        Self {
            width,
            height,
            cells,
            palette,
        }
    }

    pub fn cell_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Repair invariants after deserialization (old files, hand edits).
    pub fn normalize(&mut self) {
        self.width = self.width.clamp(1, MAX_PIXEL_DIM);
        self.height = self.height.clamp(1, MAX_PIXEL_DIM);
        if self.palette.is_empty() {
            self.palette.push([0.0, 0.0, 0.0, 1.0]);
        }
        self.palette.truncate(MAX_PALETTE);
        self.cells.resize(self.cell_count(), TRANSPARENT);
        let n = self.palette.len() as u8;
        for c in &mut self.cells {
            // 255 stays transparent; anything past the palette is erased.
            if *c != TRANSPARENT && *c >= n {
                *c = TRANSPARENT;
            }
        }
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    pub fn get(&self, x: i32, y: i32) -> u8 {
        if !self.in_bounds(x, y) {
            return TRANSPARENT;
        }
        self.cells[(y as u32 * self.width + x as u32) as usize]
    }

    /// Paint one cell. Returns true when the grid actually changed.
    pub fn set(&mut self, x: i32, y: i32, index: u8) -> bool {
        if !self.in_bounds(x, y) {
            return false;
        }
        if index != TRANSPARENT && (index as usize) >= self.palette.len() {
            return false;
        }
        let cell = &mut self.cells[(y as u32 * self.width + x as u32) as usize];
        if *cell == index {
            return false;
        }
        *cell = index;
        true
    }

    /// Bresenham stroke between two cells (pencil drags). Returns the number
    /// of cells changed.
    pub fn stroke_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, index: u8) -> usize {
        let mut changed = 0;
        let (mut x, mut y) = (x0, y0);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if self.set(x, y, index) {
                changed += 1;
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        changed
    }

    /// Flood fill the connected region of the seed cell's value.
    /// Returns the number of cells changed (0 when the seed already holds
    /// `index`, or the seed is out of bounds). Scanline-based, so deep
    /// recursion is impossible; visited set is the grid itself.
    pub fn flood_fill(&mut self, sx: i32, sy: i32, index: u8) -> usize {
        if !self.in_bounds(sx, sy) {
            return 0;
        }
        if index != TRANSPARENT && (index as usize) >= self.palette.len() {
            return 0;
        }
        let target = self.get(sx, sy);
        if target == index {
            return 0;
        }
        let w = self.width as i32;
        let h = self.height as i32;
        let mut changed = 0;
        let mut stack = vec![(sx, sy)];
        while let Some((x, y)) = stack.pop() {
            if !self.in_bounds(x, y) || self.get(x, y) != target {
                continue;
            }
            // Expand horizontally, then push the rows above/below.
            let mut left = x;
            while left - 1 >= 0 && self.get(left - 1, y) == target {
                left -= 1;
            }
            let mut right = x;
            while right + 1 < w && self.get(right + 1, y) == target {
                right += 1;
            }
            for cx in left..=right {
                self.cells[(y as u32 * self.width + cx as u32) as usize] = index;
                changed += 1;
            }
            for ny in [y - 1, y + 1] {
                if ny < 0 || ny >= h {
                    continue;
                }
                let mut cx = left;
                while cx <= right {
                    if self.get(cx, ny) == target {
                        stack.push((cx, ny));
                        while cx <= right && self.get(cx, ny) == target {
                            cx += 1;
                        }
                    } else {
                        cx += 1;
                    }
                }
            }
        }
        changed
    }

    /// Number of opaque cells (for empty-canvas detection).
    pub fn painted_count(&self) -> usize {
        self.cells.iter().filter(|c| **c != TRANSPARENT).count()
    }

    /// RGBA8 raster, row-major. Transparent cells become (0,0,0,0).
    pub fn to_rgba8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.cell_count() * 4);
        for c in &self.cells {
            let px = if *c == TRANSPARENT {
                [0, 0, 0, 0]
            } else {
                match self.palette.get(*c as usize) {
                    Some(col) => [
                        (col[0] * 255.0).round().clamp(0.0, 255.0) as u8,
                        (col[1] * 255.0).round().clamp(0.0, 255.0) as u8,
                        (col[2] * 255.0).round().clamp(0.0, 255.0) as u8,
                        (col[3] * 255.0).round().clamp(0.0, 255.0) as u8,
                    ],
                    None => [0, 0, 0, 0],
                }
            };
            out.extend_from_slice(&px);
        }
        out
    }

    /// Cheap change detector for the canvas texture cache (FNV-1a over
    /// cells + palette). Rebuilding the texture only on mismatch keeps
    /// per-frame cost near zero.
    pub fn checksum(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce4842225;
        for c in &self.cells {
            h ^= *c as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        for col in &self.palette {
            for f in col {
                h ^= f.to_bits() as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        }
        h ^= ((self.width as u64) << 32) | self.height as u64;
        h = h.wrapping_mul(0x100000001b3);
        h
    }

    /// Append a color, returning its index (or the existing index when an
    /// identical entry exists). `None` when the palette is full.
    pub fn push_color(&mut self, color: [f32; 4]) -> Option<u8> {
        if let Some(i) = self.palette.iter().position(|c| *c == color) {
            return Some(i as u8);
        }
        if self.palette.len() >= MAX_PALETTE {
            return None;
        }
        self.palette.push(color);
        Some((self.palette.len() - 1) as u8)
    }

    /// Closest palette entry (RGB euclidean) to `color`, ignoring fully
    /// transparent entries. Used by the eyedropper on foreign rasters.
    pub fn nearest_index(&self, color: [f32; 4]) -> Option<u8> {
        let mut best: Option<(u8, f32)> = None;
        for (i, c) in self.palette.iter().enumerate() {
            if c[3] <= 0.0 {
                continue;
            }
            let d = (c[0] - color[0]).powi(2) + (c[1] - color[1]).powi(2) + (c[2] - color[2]).powi(2);
            if best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((i as u8, d));
            }
        }
        best.map(|(i, _)| i)
    }

    /// The classic PICO-8 16-color palette (opaque). A safe default that
    /// every dot絵 tool recognizes.
    pub fn pico8_palette() -> Vec<[f32; 4]> {
        const HEX: [u32; 16] = [
            0x000000, 0x1D2B53, 0x7E2553, 0x008751, 0xAB5236, 0x5F574F, 0xC2C3C7, 0xFFF1E8,
            0xFF004D, 0xFFA300, 0xFFEC27, 0x00E436, 0x29ADFF, 0x83769C, 0xFF77A8, 0xFFCCAA,
        ];
        HEX.iter()
            .map(|h| {
                [
                    ((h >> 16) & 0xFF) as f32 / 255.0,
                    ((h >> 8) & 0xFF) as f32 / 255.0,
                    (h & 0xFF) as f32 / 255.0,
                    1.0,
                ]
            })
            .collect()
    }

    /// 16-step grayscale ramp (opaque) for sketching values.
    pub fn gray_palette() -> Vec<[f32; 4]> {
        (0..16)
            .map(|i| {
                let v = i as f32 / 15.0;
                [v, v, v, 1.0]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bresenham_paints_connected_line() {
        let mut p = PixelArt::new(8, 8, vec![[1.0, 0.0, 0.0, 1.0]]);
        assert_eq!(p.stroke_line(0, 0, 7, 0, 0), 8);
        assert_eq!(p.stroke_line(0, 0, 7, 0, 0), 0, "repaint is a no-op");
        assert_eq!(p.stroke_line(0, 0, 0, 7, 0), 7, "corner shared");
    }

    #[test]
    fn flood_fill_stays_inside_bounds() {
        let mut p = PixelArt::new(
            4,
            4,
            vec![[1.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]],
        );
        // Border of color 0, hollow interior.
        for x in 0..4 {
            p.set(x, 0, 0);
            p.set(x, 3, 0);
            p.set(0, x, 0);
            p.set(3, x, 0);
        }
        assert_eq!(p.flood_fill(1, 1, 1), 4, "only the hollow interior");
        assert_eq!(p.get(0, 0), 0, "border untouched");
        assert_eq!(p.flood_fill(9, 9, 1), 0, "out of bounds");
        assert_eq!(p.flood_fill(1, 1, 1), 0, "same value is a no-op");
    }

    #[test]
    fn transparent_cells_decode_as_zero_alpha() {
        let mut p = PixelArt::new(2, 1, vec![[1.0, 0.0, 0.0, 1.0]]);
        p.set(0, 0, 0);
        let px = p.to_rgba8();
        assert_eq!(&px[0..4], &[255, 0, 0, 255]);
        assert_eq!(&px[4..8], &[0, 0, 0, 0]);
    }

    #[test]
    fn normalize_repairs_foreign_data() {
        let mut p = PixelArt {
            width: 9999,
            height: 2,
            cells: vec![0, 7, TRANSPARENT],
            palette: vec![],
        };
        p.normalize();
        assert_eq!((p.width, p.height), (MAX_PIXEL_DIM, 2));
        assert_eq!(p.cells.len(), p.cell_count());
        assert_eq!(p.palette.len(), 1);
        assert_eq!(p.cells[1], TRANSPARENT, "out-of-palette index erased");
    }
}
