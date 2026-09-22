use serde::{Deserialize, Serialize};

/// Figma-style layout grid attached to an artboard: columns, rows, or a
/// square cell grid. Stored per-artboard; `show = false` keeps the config
/// without rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutGridKind {
    Columns,
    Rows,
    Grid,
}

impl Default for LayoutGridKind {
    fn default() -> Self {
        Self::Columns
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutGrid {
    pub show: bool,
    pub kind: LayoutGridKind,
    /// Column / row count (ignored for square `Grid`).
    pub count: u32,
    /// Gutter between columns/rows.
    pub gutter: f64,
    /// Outer margin (columns/rows).
    pub margin: f64,
    /// Square cell size (`Grid` only).
    pub size: f64,
    /// Overlay opacity 0..1.
    pub opacity: f64,
}

impl Default for LayoutGrid {
    fn default() -> Self {
        Self {
            show: true,
            kind: LayoutGridKind::Columns,
            count: 12,
            gutter: 20.0,
            margin: 40.0,
            size: 8.0,
            opacity: 0.15,
        }
    }
}

impl LayoutGrid {
    /// Column (or row) bands inside `extent`: `(start, end)` per track.
    pub fn tracks(&self, extent: f64) -> Vec<(f64, f64)> {
        let n = self.count.max(1) as f64;
        let inner = extent - self.margin * 2.0 - self.gutter * (n - 1.0);
        if inner <= 0.0 {
            return Vec::new();
        }
        let track = inner / n;
        let mut out = Vec::with_capacity(self.count.max(1) as usize);
        let mut pos = self.margin;
        for _ in 0..self.count.max(1) {
            out.push((pos, pos + track));
            pos += track + self.gutter;
        }
        out
    }

    /// Normalize non-finite / negative inputs (untrusted project JSON).
    pub fn normalize(&mut self) {
        if !self.gutter.is_finite() || self.gutter < 0.0 {
            self.gutter = 0.0;
        }
        if !self.margin.is_finite() || self.margin < 0.0 {
            self.margin = 0.0;
        }
        if !self.size.is_finite() || self.size <= 0.0 {
            self.size = 8.0;
        }
        if !self.opacity.is_finite() {
            self.opacity = 0.15;
        }
        self.opacity = self.opacity.clamp(0.0, 1.0);
        if self.count == 0 {
            self.count = 1;
        }
    }
}
