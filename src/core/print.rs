//! Print production model: spot colors, ink math and preflight.
//!
//! The working color everywhere stays RGB (`[f32; 4]`); print attributes
//! (overprint flags, spot references) ride on fills/strokes and resolve at
//! export time. CMYK conversion is the documented naive-UCR model (no ICC
//! engine is vendored); values are deterministic and ink-limited, which is
//! what matters for predictable plates.

use crate::core::document::{ColorMode, Document};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

/// A spot (special) color from the document's library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpotColor {
    /// Plate name as it appears in separations (e.g. "Spot Reflex Blue").
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
        SpotColor::new("Spot Reflex Blue", [1.0, 0.72, 0.0, 0.04]),
        SpotColor::new("Spot Warm Red", [0.0, 0.91, 0.76, 0.0]),
        SpotColor::new("Spot Green", [0.95, 0.0, 1.0, 0.0]),
    ]
}

/// Minimal built-in Pantone kit: the solid-coated formulas that show up
/// in most Japanese print work. Values are the *published process
/// fallbacks* (CMYK, 0..=1), not ICC-verified ink measurements — the kit
/// exists so a spot plate carries a plausible ink and preflight has a
/// total-area figure to check. Metallics, pastels/neons and coated/USC
/// or U variants are deliberately out of scope (they cannot be
/// represented as CMYK anyway); unknown names return `None` and the UI
/// falls back to the current fill.
const PANTONE_KIT: &[(&str, [f32; 4])] = &[
    ("Pantone Yellow C", [0.0, 0.09, 1.0, 0.0]),
    ("Pantone Yellow 012 C", [0.0, 0.22, 1.0, 0.0]),
    ("Pantone Orange 021 C", [0.0, 0.62, 1.0, 0.0]),
    ("Pantone Warm Red C", [0.0, 0.88, 1.0, 0.0]),
    ("Pantone Red 032 C", [0.0, 1.0, 0.88, 0.0]),
    ("Pantone 185 C", [0.0, 1.0, 0.9, 0.0]),
    ("Pantone Rubine Red C", [0.0, 1.0, 0.56, 0.09]),
    ("Pantone Rhodamine Red C", [0.0, 1.0, 0.36, 0.0]),
    ("Pantone Purple C", [0.44, 1.0, 0.0, 0.12]),
    ("Pantone Violet C", [0.69, 1.0, 0.0, 0.0]),
    ("Pantone Blue 072 C", [0.9, 0.55, 0.0, 0.0]),
    ("Pantone Reflex Blue C", [1.0, 0.18, 0.0, 0.04]),
    ("Pantone Process Blue C", [1.0, 0.59, 0.0, 0.0]),
    ("Pantone Green C", [1.0, 0.0, 0.65, 0.34]),
    ("Pantone 354 C", [0.68, 0.0, 1.0, 0.0]),
    ("Pantone 802 C", [0.35, 1.0, 0.33, 0.02]),
    ("Pantone 805 C", [0.24, 1.0, 0.45, 0.1]),
    ("Pantone Black 2 C", [0.0, 0.0, 0.0, 0.7]),
    ("Pantone Cool Gray 1 C", [0.0, 0.0, 0.0, 0.12]),
];

/// Match key for spot names: lowercase alphanumerics only, so
/// "Pantone 185 C", "PANTONE  185c" and "pantone_185_C" all agree.
fn spot_match_key(name: &str) -> String {
    name.chars()
        .flat_map(char::to_lowercase)
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

/// Look up a built-in Pantone formula by name. Accepts the kit spelling
/// ("Pantone Reflex Blue C"), the bare formula ("Reflex Blue C", "185 C")
/// and any punctuation casing. Returns the CMYK process fallback.
pub fn pantone_cmyk(query: &str) -> Option<[f32; 4]> {
    let key = spot_match_key(query);
    if key.is_empty() {
        return None;
    }
    let short = key.strip_prefix("pantone").unwrap_or(&key);
    PANTONE_KIT
        .iter()
        .find(|(name, _)| {
            let k = spot_match_key(name);
            k == key || k.strip_prefix("pantone") == Some(short)
        })
        .map(|(_, cmyk)| *cmyk)
}

/// The kit's own spelling for a query (so registered plates read like
/// "Pantone 185 C" even when the user typed "185C"), or `None`.
pub fn pantone_display_name(query: &str) -> Option<&'static str> {
    let key = spot_match_key(query);
    if key.is_empty() {
        return None;
    }
    let short = key.strip_prefix("pantone").unwrap_or(&key);
    PANTONE_KIT
        .iter()
        .find(|(name, _)| {
            let k = spot_match_key(name);
            k == key || k.strip_prefix("pantone") == Some(short)
        })
        .map(|(name, _)| *name)
}

/// Naive-UCR RGB → CMYK. Deterministic; matches the pickers.
///
/// This is the *model*, not the export path: [`crate::core::icc::rgb_to_cmyk`]
/// runs the ICC transform first and lands here whenever the C engine is
/// unavailable, so ink math keeps one documented definition either way.
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

/// Pixel budget for per-pixel ink scans of image fills. Anything larger is
/// skipped by the ink math — and refused loudly by the exporter — so
/// preflight reports it as unevaluated instead of silently passing.
pub const MAX_INK_SCAN_PX: u64 = 16_777_216;

/// One plate of a separations preview. `Composite` is the normal view;
/// anything else renders only that plate's ink as black on white paper.
#[derive(Debug, Clone, PartialEq)]
pub enum PreviewPlate {
    Composite,
    Cyan,
    Magenta,
    Yellow,
    Black,
    Spot(String),
}

impl PreviewPlate {
    /// Process channel index for C/M/Y/K, `None` for composite/spot.
    pub fn channel(&self) -> Option<usize> {
        match self {
            PreviewPlate::Composite | PreviewPlate::Spot(_) => None,
            PreviewPlate::Cyan => Some(0),
            PreviewPlate::Magenta => Some(1),
            PreviewPlate::Yellow => Some(2),
            PreviewPlate::Black => Some(3),
        }
    }

    pub fn label(&self) -> String {
        match self {
            PreviewPlate::Composite => "コンポジット".to_string(),
            PreviewPlate::Cyan => "C版".to_string(),
            PreviewPlate::Magenta => "M版".to_string(),
            PreviewPlate::Yellow => "Y版".to_string(),
            PreviewPlate::Black => "K版".to_string(),
            PreviewPlate::Spot(name) => format!("特色「{name}」"),
        }
    }
}

/// Map one resolved paint to separations-preview space.
///
/// `rgb` is the already-resolved sRGB paint (0.0..1.0 + alpha), `spot` the
/// fill/stroke's assigned spot plate (`None` = process). Returns `None`
/// when this paint puts no ink on `plate` (caller skips drawing).
///
/// Process plates skip spot-assigned paints entirely — the exporter writes
/// those to Separation plates, so they must not ghost onto CMYK either.
/// Spot plates draw only their own ink, as solid black (tints are not
/// modeled on fills/strokes). Density is the plate channel; alpha is kept
/// so translucent paints stay translucent. Overlapping same-plate paints
/// accumulate through normal alpha blending: an approximation of overprint,
/// documented as such.
pub fn plate_preview(rgb: [f32; 4], spot: Option<&str>, plate: &PreviewPlate) -> Option<[f32; 4]> {
    match plate {
        PreviewPlate::Composite => Some(rgb),
        PreviewPlate::Spot(name) => {
            if spot == Some(name.as_str()) {
                Some([0.0, 0.0, 0.0, rgb[3]])
            } else {
                None
            }
        }
        _ => {
            if spot.is_some() {
                return None;
            }
            let cmyk = rgb_to_cmyk_ink(rgb[0], rgb[1], rgb[2]);
            let density = cmyk[plate.channel().unwrap_or(3)];
            if density <= 0.001 {
                return None;
            }
            Some([0.0, 0.0, 0.0, (density * rgb[3]).clamp(0.0, 1.0)])
        }
    }
}

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
    let mut skipped_ink = 0usize;

    #[allow(clippy::too_many_arguments)]
    fn check_nested_objects(
        doc: &Document,
        objects: &[crate::core::document::Object],
        rgb_count: &mut usize,
        overprint_white: &mut usize,
        worst_ink: &mut f32,
        worst_name: &mut String,
        spot_names: &mut std::collections::BTreeSet<String>,
        skipped_ink: &mut usize,
    ) {
        for obj in objects {
            check_fill_stroke(
                doc,
                obj,
                rgb_count,
                overprint_white,
                worst_ink,
                worst_name,
                spot_names,
                skipped_ink,
            );
            match &obj.object_type {
                crate::core::document::ObjectType::Group(children)
                | crate::core::document::ObjectType::ClippingMask { children } => {
                    check_nested_objects(
                        doc,
                        children,
                        rgb_count,
                        overprint_white,
                        worst_ink,
                        worst_name,
                        spot_names,
                        skipped_ink,
                    );
                }
                _ => {}
            }
        }
    }
    for layer in &doc.layers {
        check_nested_objects(
            doc,
            &layer.objects,
            &mut rgb_count,
            &mut overprint_white,
            &mut worst_ink,
            &mut worst_name,
            &mut spot_names,
            &mut skipped_ink,
        );
    }
    // Press-readiness killers the ink math cannot see: hairlines,
    // transparency/effects (forbidden under PDF/X-1a), missing fonts and
    // RGB images. Walks into groups/masks, which the ink pass skips.
    let mut hairlines = 0usize;
    let mut transparency = 0usize;
    let mut rgb_images = 0usize;
    let mut outlined_text = 0usize;
    let mut missing_fonts: std::collections::BTreeSet<String> = Default::default();
    #[allow(clippy::too_many_arguments)]
    fn walk(
        doc: &crate::core::document::Document,
        obj: &crate::core::document::Object,
        hairlines: &mut usize,
        transparency: &mut usize,
        rgb_images: &mut usize,
        low_dpi: &mut usize,
        lowest_dpi: &mut f64,
        missing_fonts: &mut std::collections::BTreeSet<String>,
        outlined_text: &mut usize,
    ) {
        use crate::core::document::{BlendMode, ObjectType};
        use crate::core::path::FillType;
        if let Some(s) = &obj.stroke {
            if s.width < 0.25 && s.width > 0.0 {
                *hairlines += 1;
            }
        }
        // Live transparency the PDF/X gate must flatten or composite:
        // object-level flags, paint-level alpha (translucent fills/strokes
        // incl. gradient stops) and alpha pixels in placed rasters or
        // image-fill sources. Mirrors the exporter's flatten predicate, so
        // the report and the plates can never disagree. One increment per
        // object at most: misses kill presses, double counts only annoy.
        let mut paint_alpha = false;
        if let Some(f) = &obj.fill {
            if f.color[3] < 1.0 {
                paint_alpha = true;
            }
            match &f.fill_type {
                FillType::Solid(c) => {
                    if c[3] < 1.0 {
                        paint_alpha = true;
                    }
                }
                FillType::Linear(g) => {
                    if g.stops.iter().any(|s| s.color[3] < 1.0) {
                        paint_alpha = true;
                    }
                }
                FillType::Radial(g) => {
                    if g.stops.iter().any(|s| s.color[3] < 1.0) {
                        paint_alpha = true;
                    }
                }
                FillType::Image(image_fill) => {
                    let alpha = doc
                        .find_object(&image_fill.image_id)
                        .and_then(|source| match &source.object_type {
                            ObjectType::Image { png_bytes, .. } => {
                                cached_image_has_alpha(png_bytes)
                            }
                            _ => None,
                        })
                        .unwrap_or(false);
                    if alpha {
                        paint_alpha = true;
                    }
                }
                _ => {}
            }
        }
        if obj.stroke.as_ref().is_some_and(|s| s.color[3] < 1.0) {
            paint_alpha = true;
        }
        let mut raster_alpha = false;
        if let ObjectType::Image { png_bytes, .. } = &obj.object_type {
            raster_alpha = cached_image_has_alpha(png_bytes).unwrap_or(false);
        }
        if obj.opacity < 1.0
            || obj.blend_mode != BlendMode::Normal
            || obj.shadow.is_some()
            || obj.glow.is_some()
            || paint_alpha
            || raster_alpha
        {
            *transparency += 1;
        }
        match &obj.object_type {
            ObjectType::Group(children) => {
                for c in children {
                    walk(
                        doc,
                        c,
                        hairlines,
                        transparency,
                        rgb_images,
                        low_dpi,
                        lowest_dpi,
                        missing_fonts,
                        outlined_text,
                    );
                }
            }
            ObjectType::ClippingMask { children } => {
                for c in children {
                    walk(
                        doc,
                        c,
                        hairlines,
                        transparency,
                        rgb_images,
                        low_dpi,
                        lowest_dpi,
                        missing_fonts,
                        outlined_text,
                    );
                }
            }
            ObjectType::Text { style, .. } | ObjectType::TextOnPath { style, .. } => {
                let registry = crate::core::font::FontRegistry::global();
                // Comma/generic-aware like the exporter's face resolution:
                // "Inter, sans-serif" embeds via fallback, so it must not
                // Fail here while embedding fine on press.
                if !registry.is_any_family_available(&style.font_family) {
                    missing_fonts.insert(style.font_family.clone());
                } else if !matches!(&obj.object_type, ObjectType::Text { .. })
                    || text_will_outline(obj, style)
                {
                    // Installed but not embeddable (TextOnPath, vertical /
                    // variable / synthetic faces, decorated text): the
                    // exporter outlines it, so the press PDF stays correct
                    // but unsearchable — worth knowing before plating.
                    *outlined_text += 1;
                }
            }
            ObjectType::Image {
                width,
                height,
                png_bytes,
            } => {
                if let Some((pw, ph, is_rgb)) = cached_image_info(png_bytes) {
                    if is_rgb {
                        *rgb_images += 1;
                    }
                    let dpi = 72.0 * (pw as f64 / width.max(1.0)).min(ph as f64 / height.max(1.0));
                    if dpi < MIN_PRINT_DPI {
                        *low_dpi += 1;
                        *lowest_dpi = lowest_dpi.min(dpi);
                    }
                }
            }
            _ => {}
        }
    }
    for layer in &doc.layers {
        for obj in &layer.objects {
            walk(
                doc,
                obj,
                &mut hairlines,
                &mut transparency,
                &mut rgb_images,
                &mut low_dpi,
                &mut lowest_dpi,
                &mut missing_fonts,
                &mut outlined_text,
            );
        }
    }
    if hairlines > 0 {
        out.push(PreflightIssue::warn(
            "ヘアライン",
            format!("{hairlines}件の線幅が0.25pt未満 — 印刷で消える恐れ"),
        ));
    }
    if transparency > 0 {
        out.push(PreflightIssue::warn(
            "透明・効果",
            format!(
                "{transparency}件に不透明度/ブレンド/シャドウ/グロー — PDF/X-1aでは要フラット化"
            ),
        ));
    }
    if !missing_fonts.is_empty() {
        out.push(PreflightIssue::fail(
            "未インストールフォント",
            format!(
                "{} — 代替グリフで出力されます",
                missing_fonts.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        ));
    }
    if outlined_text > 0 {
        out.push(PreflightIssue::warn(
            "フォント埋込",
            format!("{outlined_text}件のテキストは埋込対象外のためアウトライン化されます"),
        ));
    }
    if rgb_images > 0 {
        out.push(PreflightIssue::warn(
            "RGB画像",
            format!("{rgb_images}件 — 書き出し時にCMYK変換されます"),
        ));
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
            format!(
                "最大{:.0}%（{}）— 320%以下推奨",
                worst_ink * 100.0,
                worst_name
            ),
        ));
    } else {
        out.push(PreflightIssue::pass(
            "インキ総量",
            if worst_name.is_empty() {
                format!("最大{:.0}%", worst_ink * 100.0)
            } else {
                format!("最大{:.0}%（{}）", worst_ink * 100.0, worst_name)
            },
        ));
    }
    if skipped_ink > 0 {
        out.push(PreflightIssue::warn(
            "インキ未評価",
            format!("{skipped_ink}件の画像塗りは大きすぎて走査できず — TAC超過を見逃す恐れ"),
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
        out.push(PreflightIssue::pass(
            "画像解像度",
            "配置画像は十分な解像度です",
        ));
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
            format!(
                "{}版: {}",
                spot_names.len(),
                spot_names.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        ));
    }
    // No trap engine: modern RIPs trap at output, and this file declares
    // /Trapped /False honestly. Surfaced here so "nothing traps" is a
    // visible statement, not an unknown.
    let trap_count: usize = doc
        .layers
        .iter()
        .filter(|l| l.name == crate::core::trap::TRAP_LAYER_NAME)
        .map(|l| l.objects.len())
        .sum();
    if trap_count > 0 {
        out.push(PreflightIssue::pass(
            "トラップ",
            format!("スプレッド{trap_count}件配置済み（特色境界は対象外）"),
        ));
    } else {
        out.push(PreflightIssue::pass(
            "トラップ",
            "アプリ側トラップなし（RIP任せ・/Trapped /False宣言）",
        ));
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn check_fill_stroke(
    doc: &Document,
    obj: &crate::core::document::Object,
    rgb_count: &mut usize,
    overprint_white: &mut usize,
    worst_ink: &mut f32,
    worst_name: &mut String,
    spot_names: &mut std::collections::BTreeSet<String>,
    skipped_ink: &mut usize,
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
        // Same conversion the exporter writes, so the report cannot disagree
        // with the plates it is describing.
        let ink = crate::core::icc::rgb_to_cmyk([c[0], c[1], c[2], 1.0]);
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
    let mut image_fill_ink = None;
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
            FillType::Pattern(_) => proc_color(f.color, f.overprint, &f.spot, "パターン塗り"),
            FillType::Image(image_fill) => {
                if let Some(source) = doc.find_object(&image_fill.image_id) {
                    if let crate::core::document::ObjectType::Image { png_bytes, .. } =
                        &source.object_type
                    {
                        match cached_image_fill_max_ink(
                            png_bytes,
                            image_fill.crop_rect,
                            f.color[3] * obj.opacity,
                        ) {
                            Some(ink) => image_fill_ink = Some(ink),
                            // Oversize / undecodable: a silent pass would
                            // under-report TAC, so count it for the
                            // インキ未評価 warning instead.
                            None => *skipped_ink += 1,
                        }
                    }
                }
            }
        }
    }
    if let Some(s) = &obj.stroke {
        proc_color(s.color, s.overprint, &s.spot, "線");
    }
    if let Some(ink) = image_fill_ink {
        // `>=`: ties name the raster. A thousands-pixel maximum that ties a
        // flat color is the likelier TAC breaker on press (quantization,
        // JPEG variance), so the conservative report names it.
        if ink >= *worst_ink && ink > 0.0 {
            *worst_ink = ink;
            *worst_name = format!("画像塗り（{}）", obj.name);
        }
    }
}

/// Cheap mirror of the exporter's embed gate: `true` means this text will be
/// outlined, not embedded. Shaping-stage fallbacks (.notdef, >U+FFFF GIDs)
/// are unknowable without shaping, so they stay silent here; the export
/// still outlines them safely instead of emitting wrong glyphs.
fn text_will_outline(
    obj: &crate::core::document::Object,
    style: &crate::core::document::TextStyle,
) -> bool {
    if style.vertical || !style.variations.is_empty() {
        return true;
    }
    let registry = crate::core::font::FontRegistry::global();
    if registry.needs_synthetic_style(&style.font_family, style.font_weight, style.font_style) {
        return true;
    }
    if obj.opacity < 1.0
        || obj.blend_mode != crate::core::document::BlendMode::Normal
        || obj.shadow.is_some()
        || obj.glow.is_some()
        || obj.stroke.is_some()
    {
        return true;
    }
    !matches!(
        obj.fill.as_ref().map(|f| &f.fill_type),
        Some(crate::core::path::FillType::Solid(c)) if c[3] >= 1.0
    )
}

/// Whether a PNG carries any non-opaque pixel. Decoded pixels are cached by
/// source bytes; preflight runs every frame while its panel is open.
fn cached_image_has_alpha(png_bytes: &[u8]) -> Option<bool> {
    static CACHE: OnceLock<Mutex<std::collections::HashMap<u64, Option<bool>>>> = OnceLock::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    png_bytes.hash(&mut hasher);
    let key = hasher.finish();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(value) = guard.get(&key).copied() {
            return value;
        }
    }
    let value = image::load_from_memory(png_bytes)
        .ok()
        .map(|decoded| decoded.to_rgba8().pixels().any(|p| p[3] < 255));
    if let Ok(mut cache) = cache.lock() {
        if cache.len() >= 1024 {
            cache.clear();
        }
        cache.insert(key, value);
    }
    value
}

/// Maximum per-pixel total ink for an image fill after alpha compositing onto
/// paper. The print panel calls preflight every frame, so cache decoded scans
/// by source bytes, crop, and effective fill opacity.
fn cached_image_fill_max_ink(
    png_bytes: &[u8],
    crop_rect: Option<[f32; 4]>,
    opacity: f32,
) -> Option<f32> {
    static CACHE: OnceLock<Mutex<std::collections::HashMap<u64, Option<f32>>>> = OnceLock::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    png_bytes.hash(&mut hasher);
    crop_rect
        .map(|rect| rect.map(f32::to_bits))
        .hash(&mut hasher);
    opacity.to_bits().hash(&mut hasher);
    let key = hasher.finish();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(value) = guard.get(&key).copied() {
            return value;
        }
    }

    let decoded = image::load_from_memory(png_bytes).ok()?.to_rgba8();
    let pixels = if let Some([x, y, width, height]) = crop_rect {
        if ![x, y, width, height].iter().all(|v| v.is_finite()) {
            return None;
        }
        let x0 = (x.clamp(0.0, 1.0) * decoded.width() as f32).floor() as u32;
        let y0 = (y.clamp(0.0, 1.0) * decoded.height() as f32).floor() as u32;
        let x1 = ((x + width).clamp(0.0, 1.0) * decoded.width() as f32).ceil() as u32;
        let y1 = ((y + height).clamp(0.0, 1.0) * decoded.height() as f32).ceil() as u32;
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        image::imageops::crop_imm(&decoded, x0, y0, x1 - x0, y1 - y0).to_image()
    } else {
        decoded
    };
    if pixels.width() == 0
        || pixels.height() == 0
        || pixels.width() as u64 * pixels.height() as u64 > MAX_INK_SCAN_PX
    {
        return None;
    }
    let opacity = opacity.clamp(0.0, 1.0);
    let max_ink = pixels
        .pixels()
        .map(|pixel| {
            let a = pixel[3] as f32 / 255.0 * opacity;
            let rgb = [
                pixel[0] as f32 / 255.0 * a + 1.0 - a,
                pixel[1] as f32 / 255.0 * a + 1.0 - a,
                pixel[2] as f32 / 255.0 * a + 1.0 - a,
            ];
            // Same conversion the exporter writes (ICC engine with the
            // naive-UCR fallback), so the report cannot disagree with the
            // plates it is describing.
            total_ink(crate::core::icc::rgb_to_cmyk([rgb[0], rgb[1], rgb[2], 1.0]))
        })
        .fold(0.0f32, f32::max);

    if let Ok(mut cache) = cache.lock() {
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(key, Some(max_ink));
    }
    Some(max_ink)
}

/// Pixel dimensions plus RGB flag for placed images. Preflight runs every
/// frame while its panel is open, so decoded headers are cached by source
/// bytes; per-frame DPI math stays cheap without re-decoding pixels.
type CachedImageInfo = Option<(u32, u32, bool)>;
fn cached_image_info(png_bytes: &[u8]) -> CachedImageInfo {
    static CACHE: OnceLock<Mutex<std::collections::HashMap<u64, CachedImageInfo>>> =
        OnceLock::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    png_bytes.hash(&mut hasher);
    let key = hasher.finish();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(value) = guard.get(&key).copied() {
            return value;
        }
    }
    let decoded = image::load_from_memory(png_bytes).ok()?;
    let value = Some((
        decoded.width(),
        decoded.height(),
        matches!(
            decoded.color(),
            image::ColorType::Rgb8 | image::ColorType::Rgba8
        ),
    ));
    if let Ok(mut cache) = cache.lock() {
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(key, value);
    }
    value
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

    #[test]
    fn plate_preview_maps_channels_and_spots() {
        let red = [1.0, 0.0, 0.0, 1.0];
        // Composite passes through.
        assert_eq!(
            plate_preview(red, None, &PreviewPlate::Composite),
            Some(red)
        );
        // Pure red lands on M+Y only.
        let m = plate_preview(red, None, &PreviewPlate::Magenta).unwrap();
        assert!((m[3] - 1.0).abs() < 1e-6 && m[0] == 0.0);
        assert!(plate_preview(red, None, &PreviewPlate::Cyan).is_none());
        let k = plate_preview([0.0, 0.0, 0.0, 1.0], None, &PreviewPlate::Black).unwrap();
        assert!((k[3] - 1.0).abs() < 1e-6);
        // Spot-assigned paint never ghosts onto process plates.
        assert!(plate_preview(red, Some("Gold"), &PreviewPlate::Magenta).is_none());
        assert!(plate_preview(red, Some("Gold"), &PreviewPlate::Cyan).is_none());
        // Spot plate shows only its own ink, as black.
        let gold = plate_preview(red, Some("Gold"), &PreviewPlate::Spot("Gold".into())).unwrap();
        assert_eq!(gold, [0.0, 0.0, 0.0, 1.0]);
        assert!(plate_preview(red, Some("Gold"), &PreviewPlate::Spot("Silver".into())).is_none());
        assert!(plate_preview(red, None, &PreviewPlate::Spot("Gold".into())).is_none());
        // Alpha scales density.
        let half = plate_preview([1.0, 0.0, 0.0, 0.5], None, &PreviewPlate::Magenta).unwrap();
        assert!((half[3] - 0.5).abs() < 1e-6);
    }
}
