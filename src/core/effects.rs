use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropShadow {
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur_radius: f64,
    pub color: [f32; 4],
    pub opacity: f32,
}

impl Default for DropShadow {
    fn default() -> Self {
        Self {
            offset_x: 6.0,
            offset_y: 6.0,
            blur_radius: 8.0,
            color: [0.0, 0.0, 0.0, 1.0],
            opacity: 0.4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlowEffect {
    pub radius: f64,
    pub color: [f32; 4],
    pub intensity: f32,
}

impl Default for GlowEffect {
    fn default() -> Self {
        Self {
            radius: 12.0,
            color: [0.0, 0.8, 1.0, 1.0],
            intensity: 0.6,
        }
    }
}

/// Gaussian Blur filter effect
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlurEffect {
    pub radius: f64,
}

impl Default for BlurEffect {
    fn default() -> Self {
        Self { radius: 5.0 }
    }
}

/// Color matrix adjust effect (Brightness, Contrast, Saturation)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorAdjustEffect {
    pub brightness: f32, // -1.0 to 1.0 (default 0.0)
    pub contrast: f32,   // 0.0 to 3.0 (default 1.0)
    pub saturation: f32, // 0.0 to 3.0 (default 1.0)
    pub hue_rotate: f32, // degrees: 0 to 360 (default 0.0)
}

impl Default for ColorAdjustEffect {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue_rotate: 0.0,
        }
    }
}

/// A comprehensive stackable appearance effect
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VectorEffect {
    DropShadow(DropShadow),
    Glow(GlowEffect),
    Blur(BlurEffect),
    ColorAdjust(ColorAdjustEffect),
}

/// Appearance effect stack supporting non-destructive multiple effects
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppearanceStack {
    pub effects: Vec<VectorEffect>,
}

impl AppearanceStack {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    pub fn push(&mut self, effect: VectorEffect) {
        self.effects.push(effect);
    }

    pub fn clear(&mut self) {
        self.effects.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn has_blur(&self) -> Option<f64> {
        self.effects.iter().find_map(|e| match e {
            VectorEffect::Blur(b) => Some(b.radius),
            _ => None,
        })
    }

    pub fn has_shadow(&self) -> Option<&DropShadow> {
        self.effects.iter().find_map(|e| match e {
            VectorEffect::DropShadow(s) => Some(s),
            _ => None,
        })
    }

    pub fn has_glow(&self) -> Option<&GlowEffect> {
        self.effects.iter().find_map(|e| match e {
            VectorEffect::Glow(g) => Some(g),
            _ => None,
        })
    }

    pub fn has_color_adjust(&self) -> Option<&ColorAdjustEffect> {
        self.effects.iter().find_map(|e| match e {
            VectorEffect::ColorAdjust(c) => Some(c),
            _ => None,
        })
    }

    /// Replace the first Blur/ColorAdjust entry if one exists, otherwise push.
    /// Illustrator's effect dialog edits the *same* effect row, it never
    /// stacks a second Blur on every slider tick.
    pub fn upsert(&mut self, effect: VectorEffect) {
        let same_kind = |e: &VectorEffect| {
            matches!(
                (e, &effect),
                (VectorEffect::Blur(_), VectorEffect::Blur(_))
                    | (VectorEffect::ColorAdjust(_), VectorEffect::ColorAdjust(_))
                    | (VectorEffect::DropShadow(_), VectorEffect::DropShadow(_))
                    | (VectorEffect::Glow(_), VectorEffect::Glow(_))
            )
        };
        if let Some(pos) = self.effects.iter().position(same_kind) {
            self.effects[pos] = effect;
        } else {
            self.effects.push(effect);
        }
    }

    /// Remove every effect of the same kind as `probe` (the payload is
    /// ignored, only the variant matters).
    pub fn remove_kind(&mut self, probe: &VectorEffect) {
        let same_kind = |e: &VectorEffect| {
            matches!(
                (e, probe),
                (VectorEffect::Blur(_), VectorEffect::Blur(_))
                    | (VectorEffect::ColorAdjust(_), VectorEffect::ColorAdjust(_))
                    | (VectorEffect::DropShadow(_), VectorEffect::DropShadow(_))
                    | (VectorEffect::Glow(_), VectorEffect::Glow(_))
            )
        };
        self.effects.retain(|e| !same_kind(e));
    }
}

/// Compose the brightness / contrast / saturation / hue-rotate adjustment of
/// an object into a single SVG `feColorMatrix` row-major 4x5 matrix.
///
/// Order (Illustrator "Edit Colors" behaves the same way): hue-rotate, then
/// saturation, then contrast-as-scale-around-mid-gray, then brightness offset.
/// Everything is computed in premultiplied-1.0 RGB space; the 5th column is
/// the additive offset applied after the 4x4 linear part.
///
/// Returns `None` when the adjustment is the identity (SVG round-trips of
/// no-op effects must not sprout `<filter>` blocks).
pub fn color_adjust_matrix(adj: &ColorAdjustEffect) -> Option<[f32; 20]> {
    let hue = adj.hue_rotate.to_radians();
    let sat = adj.saturation;
    let contrast = adj.contrast;
    let bright = adj.brightness;

    if !hue.is_finite()
        || !sat.is_finite()
        || !contrast.is_finite()
        || !bright.is_finite()
        || (hue.abs() < 1e-6
            && (sat - 1.0).abs() < 1e-6
            && (contrast - 1.0).abs() < 1e-6
            && bright.abs() < 1e-6)
    {
        return None;
    }

    let clamp_f = |v: f32| {
        if v.is_finite() {
            v.clamp(-8.0, 8.0)
        } else {
            1.0
        }
    };
    let sat = clamp_f(sat);
    let contrast = clamp_f(contrast);
    let bright = if bright.is_finite() { bright.clamp(-1.0, 1.0) } else { 0.0 };

    // --- Hue-rotate (SVG 1.1 feColorMatrix "hueRotate" definition) ---
    // | a00 a01 a02 |   | 0.213+0.787c-0.213s, 0.715-0.715c-0.715s, 0.072-0.072c+0.928s |
    // | a10 a11 a12 | = | 0.213-0.213c+0.143s, 0.715+0.285c+0.140s, 0.072-0.072c-0.283s |
    // | a20 a21 a22 |   | 0.213-0.213c-0.787s, 0.715-0.715c+0.715s, 0.072+0.928c+0.072s |
    let (sin_h, cos_h) = hue.sin_cos();
    let hue_m = [
        [
            0.213 + 0.787 * cos_h - 0.213 * sin_h,
            0.715 - 0.715 * cos_h - 0.715 * sin_h,
            0.072 - 0.072 * cos_h + 0.928 * sin_h,
        ],
        [
            0.213 - 0.213 * cos_h + 0.143 * sin_h,
            0.715 + 0.285 * cos_h + 0.140 * sin_h,
            0.072 - 0.072 * cos_h - 0.283 * sin_h,
        ],
        [
            0.213 - 0.213 * cos_h - 0.787 * sin_h,
            0.715 - 0.715 * cos_h + 0.715 * sin_h,
            0.072 + 0.928 * cos_h + 0.072 * sin_h,
        ],
    ];

    // --- Saturate (SVG 1.1 feColorMatrix "saturate" definition) ---
    // Exact composition: out = Saturate(H * in) with
    //   Saturate(y)[r] = L(y) + s * (y[r] - L(y)),  L(y) = lum · y.
    // Expanding L(y) through the hue matrix gives the closed form
    //   out[r][c] = s * H[r][c] + (1 - s) * K[c],
    // where K[c] = Σ_k lum[k] * H[k][c] is the luminance of output row r's
    // gray target expressed in input channels (identical for every row, since
    // all three rows blend toward the *same* gray value L(y)).
    let lum = [0.213, 0.715, 0.072];
    let mut k_col = [0.0f32; 3];
    for col in 0..3 {
        k_col[col] = lum[0] * hue_m[0][col] + lum[1] * hue_m[1][col] + lum[2] * hue_m[2][col];
    }
    let mut m = [[0.0f32; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            m[row][col] = sat * hue_m[row][col] + (1.0 - sat) * k_col[col];
        }
    }

    // --- Contrast (scale around mid-gray) + brightness (additive) ---
    let offset = 0.5 * (1.0 - contrast) + bright;
    let mut out = [0.0f32; 20];
    for row in 0..3 {
        for col in 0..3 {
            out[row * 5 + col] = m[row][col] * contrast;
        }
        out[row * 5 + 3] = 0.0;
        out[row * 5 + 4] = offset;
    }
    // Alpha row: untouched (SVG alpha stays out of the color math).
    out[15] = 0.0;
    out[16] = 0.0;
    out[17] = 0.0;
    out[18] = 1.0;
    out[19] = 0.0;
    Some(out)
}

/// Apply a [`color_adjust_matrix`] to one straight-alpha RGBA color (0..1).
///
/// SVG runs the matrix per pixel; the canvas runs it per color, which is the
/// same answer for flat artwork and cheap enough to do every frame.  Both
/// paths therefore show the identical adjustment instead of two
/// implementations drifting apart.
pub fn apply_color_adjust_matrix(m: &[f32; 20], rgba: [f32; 4]) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for (row, v) in out.iter_mut().enumerate() {
        let base = row * 5;
        *v = m[base] * rgba[0]
            + m[base + 1] * rgba[1]
            + m[base + 2] * rgba[2]
            + m[base + 3] * rgba[3]
            + m[base + 4];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_adjust() -> ColorAdjustEffect {
        ColorAdjustEffect {
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue_rotate: 0.0,
        }
    }

    #[test]
    fn identity_adjustment_emits_no_matrix() {
        assert!(color_adjust_matrix(&identity_adjust()).is_none());
    }

    #[test]
    fn brightness_offset_applies_and_leaves_alpha_alone() {
        let adj = ColorAdjustEffect {
            brightness: 0.5,
            ..identity_adjust()
        };
        let m = color_adjust_matrix(&adj).expect("non-identity adjustment emits a matrix");
        let out = apply_color_adjust_matrix(&m, [0.2, 0.4, 0.6, 1.0]);
        assert!((out[0] - 0.7).abs() < 1e-5, "{out:?}");
        assert!((out[1] - 0.9).abs() < 1e-5, "{out:?}");
        assert!((out[2] - 1.1).abs() < 1e-5, "{out:?}");
        assert_eq!(out[3], 1.0, "alpha row must be untouched");
    }

    #[test]
    fn saturation_reaches_gray_without_touching_alpha() {
        let adj = ColorAdjustEffect {
            saturation: 0.0,
            ..identity_adjust()
        };
        let m = color_adjust_matrix(&adj).expect("non-identity adjustment emits a matrix");
        let out = apply_color_adjust_matrix(&m, [1.0, 0.0, 0.0, 0.5]);
        let lum = 0.213;
        assert!((out[0] - lum).abs() < 1e-5, "{out:?}");
        assert!((out[1] - lum).abs() < 1e-5, "{out:?}");
        assert!((out[2] - lum).abs() < 1e-5, "{out:?}");
        assert_eq!(out[3], 0.5, "alpha row must be untouched");
    }
}
