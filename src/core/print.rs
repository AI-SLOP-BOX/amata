//! Print production model: spot colors, ink math and preflight.
//!
//! The working color everywhere stays RGB (`[f32; 4]`); print attributes
//! (overprint flags, spot references) ride on fills/strokes and resolve at
//! export time. CMYK conversion is the documented naive-UCR model (no ICC
//! engine is vendored); values are deterministic and ink-limited, which is
//! what matters for predictable plates.

use crate::core::document::{ColorMode, Document, ObjectType};
use serde::{Deserialize, Serialize};

/// A spot (special) color from the document's library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpotColor {
    /// Plate name as it appears in separations (e.g. "PANTONE  reflex Blue C").
    pub name: String,
    /// Process fallback for preview and composite output.
    pub cmyk: [f32; 4],
}

impl SpotColor {
    pub fn new(name: impl Into<String>, cmyk: [f32; 4]) -> Self {
        Self {
            name: name.into(),
            cmyk: [
                cmyk[0].clamp(0.0, 1.0),
                cmyk[1].clamp(0.0, 1.0),
                cmyk[2].clamp(0.0, 1.0),
                cmyk[3].clamp(0.0, 1.0),
            ],
        }
    }

    /// Screen/process preview color.
    pub fn preview_rgb(&self) -> [f32; 4] {
        let [c, m, y, k] = self.cmyk;
        [
            (1.0 - c) * (1.0 - k),
            (1.0 - m) * (1.0 - k),
            (1.0 - y) * (1.0 - k),
            1.0,
        ]
    }
}

/// A few starter spots so new documents are not empty-handed.
pub fn default_spots() -> Vec<SpotColor> {
    vec![
        SpotColor::new("PANTONE Reflex Blue C", [1.0, 0.72, 0.0, 0.04]),
        SpotColor::new("PANTONE 185 C", [0.0, 0.91, 0.76, 0.0]),
        SpotColor::new("PANTONE 355 C", [0.95, 0.0, 1.0, 0.0]),
    ]
}

/// Naive-UCR RGB → CMYK. Deterministic; matches the pickers.
pub fn rgb_to_cmyk_ink(r: f32, g: f32, b: f32) -> [f32; 4] {
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let inv = 1.0 - k;
    [
        ((inv - r) / inv).clamp(0.0, 1.0),
        ((inv - g) / inv).clamp(0.0, 1.0),
        ((inv - b) / inv).clamp(0.0, 1.0),
        k,
    ]
}

/// Total area coverage (sum of plates, 0..4).
pub fn total_ink(cmyk: [f32; 4]) -> f32 {
    cmyk[0] + cmyk[1] + cmyk[2] + cmyk[3]
}

/// Scale channels uniformly so the total stays under `max` (Japan coated
/// stock is usually profiled around 3.2–3.5; preflight warns above 3.2).
pub fn limit_ink(mut cmyk: [f32; 4], max: f32) -> [f32; 4] {
    let t = total_ink(cmyk);
    if t > max && t > 0.0 {
        let s = max / t;
        for c in &mut cmyk {
            *c *= s;
        }
    }
    cmyk
}

/// Japan print default TAC alarm threshold (320%).
pub const MAX_TOTAL_INK: f32 = 3.2;

/// Minimum raster DPI for print (warn) and hard floor (fail).
pub const MIN_PRINT_DPI: f64 = 300.0;
pub const FAIL_PRINT_DPI: f64 = 150.0;

/// Standard bleed for Japanese sheet-fed offset (3mm ≈ 8.5pt at 72dpi… in
/// practice shops ask for 3mm; our units are points so 3mm = 8.5pt).
/// Kept asmm-based helper: callers pass millimetres.
pub fn mm_to_pt(mm: f64) -> f64 {
    mm * 72.0 / 25.4
}

pub const DEFAULT_BLEED_MM: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreflightLevel {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone)]
pub struct PreflightIssue {
    pub level: PreflightLevel,
    pub check: String,
    pub detail: String,
}

impl PreflightIssue {
    pub fn pass(check: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            level: PreflightLevel::Pass,
            check: check.into(),
            detail: detail.into(),
        }
    }
    pub fn warn(check: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            level: PreflightLevel::Warn,
            check: check.into(),
            detail: detail.into(),
        }
    }
    pub fn fail(check: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            level: PreflightLevel::Fail,
            check: check.into(),
            detail: detail.into(),
        }
    }
}

/// Run print preflight over a document. Pure (no UI), reused by the panel
/// and the export gate.
pub fn preflight(doc: &Document) -> Vec<PreflightIssue> {
    let mut out = Vec::new();
    let mut rgb_count = 0usize;
    let mut overprint_white = 0usize;
    let mut worst_ink = 0.0f32;
    let mut worst_name = String::new();
    let mut low_dpi = 0usize;
    let mut lowest_dpi = f64::MAX;
    let mut spot_names: std::collections::BTreeSet<String> = Default::default();

    for layer in &doc.layers {
        for obj in layer.objects.iter() {
            check_fill_stroke(doc, obj, &mut rgb_count, &mut overprint_white, &mut worst_ink, &mut worst_name, &mut spot_names);
        }
    }
    // Images: effective DPI assuming 1 unit = 1pt.
    for layer in &doc.layers {
        for obj in layer.objects.iter() {
            if let ObjectType::Image { width, height, png_bytes } = &obj.object_type {
                if let Ok(img) = image::load_from_memory(png_bytes) {
                    let (pw, ph) = (img.width() as f64, img.height() as f64);
                    let dw = width.max(1.0);
                    let dh = height.max(1.0);
                    let dpi = 72.0 * (pw / dw).min(ph / dh);
                    if dpi < MIN_PRINT_DPI {
                        low_dpi += 1;
                        lowest_dpi = lowest_dpi.min(dpi);
                    }
                }
            }
        }
    }

    let is_cmyk = doc.color_mode == ColorMode::Cmyk;
    if is_cmyk && rgb_count > 0 {
        out.push(PreflightIssue::warn(
            "RGBオブジェクト",
            format!("{rgb_count}件がRGBのまま — 書き出し時にCMYK変換されます"),
        ));
    } else {
        out.push(PreflightIssue::pass(
            "カラーモード",
            if is_cmyk {
                "全プロセス色はCMYKです".to_string()
            } else {
                "RGBドキュメント（印刷時はCMYK変換）".to_string()
            },
        ));
    }
    if worst_ink > MAX_TOTAL_INK {
        out.push(PreflightIssue::warn(
            "インキ総量",
            format!("最大{:.0}%（{}）— 320%以下推奨", worst_ink * 100.0, worst_name),
        ));
    } else {
        out.push(PreflightIssue::pass(
            "インキ総量",
            format!("最大{:.0}%", worst_ink * 100.0),
        ));
    }
    if overprint_white > 0 {
        out.push(PreflightIssue::fail(
            "白のオーバープリント",
            format!("{overprint_white}件 — 白が透けて消えます"),
        ));
    }
    if low_dpi > 0 {
        let lvl = if lowest_dpi < FAIL_PRINT_DPI {
            PreflightLevel::Fail
        } else {
            PreflightLevel::Warn
        };
        out.push(PreflightIssue {
            level: lvl,
            check: "画像解像度".to_string(),
            detail: format!("{low_dpi}件が300dpi未満（最低{lowest_dpi:.0}dpi）"),
        });
    } else {
        out.push(PreflightIssue::pass("画像解像度", "配置画像は十分な解像度です"));
    }
    if doc.bleed < mm_to_pt(DEFAULT_BLEED_MM) - 0.01 {
        out.push(PreflightIssue::warn(
            "塗り足し",
            format!("{:.1}mm — 3mm推奨", doc.bleed * 25.4 / 72.0),
        ));
    } else {
        out.push(PreflightIssue::pass(
            "塗り足し",
            format!("{:.1}mm", doc.bleed * 25.4 / 72.0),
        ));
    }
    if spot_names.is_empty() {
        out.push(PreflightIssue::pass("特色", "特色は使われていません"));
    } else {
        out.push(PreflightIssue::pass(
            "特色",
            format!("{}版: {}", spot_names.len(), spot_names.iter().cloned().collect::<Vec<_>>().join(", ")),
        ));
    }
    out
}

fn check_fill_stroke(
    doc: &Document,
    obj: &crate::core::document::Object,
    rgb_count: &mut usize,
    overprint_white: &mut usize,
    worst_ink: &mut f32,
    worst_name: &mut String,
    spot_names: &mut std::collections::BTreeSet<String>,
) {
    use crate::core::path::FillType;
    let is_cmyk = doc.color_mode == ColorMode::Cmyk;
    let mut proc_color = |c: [f32; 4], overprint: bool, spot: &Option<String>, label: &str| {
        if let Some(name) = spot {
            if doc.spots.iter().any(|s| &s.name == name) {
                spot_names.insert(name.clone());
                let cmyk = doc.spots.iter().find(|s| &s.name == name).unwrap().cmyk;
                let t = total_ink(cmyk);
                if t > *worst_ink {
                    *worst_ink = t;
                    *worst_name = format!("{label}（特色{name})");
                }
            } else {
                // Dangling spot reference behaves as process color.
                if is_cmyk {
                    *rgb_count += 1;
                }
            }
            return;
        }
        if is_cmyk {
            *rgb_count += 1;
        }
        let ink = rgb_to_cmyk_ink(c[0], c[1], c[2]);
        let t = total_ink(ink);
        if t > *worst_ink {
            *worst_ink = t;
            *worst_name = label.to_string();
        }
        let lum = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        if overprint && lum > 0.95 && c[3] > 0.01 {
            *overprint_white += 1;
        }
    };
    if let Some(f) = &obj.fill {
        match &f.fill_type {
            FillType::Solid(c) => proc_color(*c, f.overprint, &f.spot, "塗り"),
            FillType::Linear(g) => {
                for s in &g.stops {
                    proc_color(s.color, f.overprint, &f.spot, "グラデーション");
                }
            }
            FillType::Radial(g) => {
                for s in &g.stops {
                    proc_color(s.color, f.overprint, &f.spot, "グラデーション");
                }
            }
            _ => {}
        }
    }
    if let Some(s) = &obj.stroke {
        proc_color(s.color, s.overprint, &s.spot, "線");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ink_math_rounds_and_caps() {
        assert_eq!(rgb_to_cmyk_ink(0.0, 0.0, 0.0), [0.0, 0.0, 0.0, 1.0]);
        assert!((total_ink([1.0, 1.0, 1.0, 0.0]) - 3.0).abs() < 1e-6);
        let capped = limit_ink([1.0, 1.0, 1.0, 1.0], 3.2);
        assert!((total_ink(capped) - 3.2).abs() < 1e-4);
        assert_eq!(limit_ink([0.5, 0.0, 0.0, 0.0], 3.2), [0.5, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn spot_preview_matches_cmyk() {
        let s = SpotColor::new("Test", [1.0, 0.0, 0.0, 0.0]);
        let rgb = s.preview_rgb();
        assert!(rgb[0] < 0.01 && rgb[1] > 0.99 && rgb[2] > 0.99);
    }
}
