use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    LinearDodge,
    LinearBurn,
    VividLight,
    LinearLight,
    PinLight,
    HardMix,
    Divide,
    Subtract,
}

/// Blend foreground (src) RGBA over background (dst) RGBA using specified Photoshop blend mode
pub fn blend_colors(src: [f32; 4], dst: [f32; 4], mode: BlendMode) -> [f32; 4] {
    let [sr, sg, sb, sa] = src;
    let [dr, dg, db, da] = dst;

    if sa <= 0.0 {
        return dst;
    }
    if da <= 0.0 {
        return src;
    }

    let blend_ch = |s: f32, d: f32| -> f32 {
        match mode {
            BlendMode::Normal => s,
            BlendMode::Multiply => s * d,
            BlendMode::Screen => 1.0 - (1.0 - s) * (1.0 - d),
            BlendMode::Overlay => {
                if d < 0.5 {
                    2.0 * s * d
                } else {
                    1.0 - 2.0 * (1.0 - s) * (1.0 - d)
                }
            }
            BlendMode::Darken => s.min(d),
            BlendMode::Lighten => s.max(d),
            BlendMode::ColorDodge => {
                if s >= 1.0 { 1.0 } else { (d / (1.0 - s).max(1e-5)).min(1.0) }
            }
            BlendMode::ColorBurn => {
                if s <= 0.0 { 0.0 } else { (1.0 - (1.0 - d) / s.max(1e-5)).max(0.0) }
            }
            BlendMode::HardLight => {
                if s < 0.5 {
                    2.0 * s * d
                } else {
                    1.0 - 2.0 * (1.0 - s) * (1.0 - d)
                }
            }
            BlendMode::SoftLight => {
                if s < 0.5 {
                    d - (1.0 - 2.0 * s) * d * (1.0 - d)
                } else {
                    let g = if d <= 0.25 {
                        ((16.0 * d - 12.0) * d + 4.0) * d
                    } else {
                        d.sqrt()
                    };
                    d + (2.0 * s - 1.0) * (g - d)
                }
            }
            BlendMode::Difference => (d - s).abs(),
            BlendMode::Exclusion => d + s - 2.0 * d * s,
            BlendMode::LinearDodge => (d + s).min(1.0),
            BlendMode::LinearBurn => (d + s - 1.0).max(0.0),
            BlendMode::VividLight => {
                if s < 0.5 {
                    if s <= 0.0 { 0.0 } else { (1.0 - (1.0 - d) / (2.0 * s).max(1e-5)).max(0.0) }
                } else {
                    let s2 = 2.0 * (s - 0.5);
                    if s2 >= 1.0 { 1.0 } else { (d / (1.0 - s2).max(1e-5)).min(1.0) }
                }
            }
            BlendMode::LinearLight => (d + 2.0 * s - 1.0).clamp(0.0, 1.0),
            BlendMode::PinLight => {
                if s < 0.5 {
                    d.min(2.0 * s)
                } else {
                    d.max(2.0 * (s - 0.5))
                }
            }
            BlendMode::HardMix => {
                if s + d >= 1.0 { 1.0 } else { 0.0 }
            }
            BlendMode::Divide => (d / s.max(1e-5)).min(1.0),
            BlendMode::Subtract => (d - s).max(0.0),
            BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => {
                // Approximate fallback for non-separable modes
                s
            }
        }
    };

    let br = blend_ch(sr, dr);
    let bg = blend_ch(sg, dg);
    let bb = blend_ch(sb, db);

    // Alpha composite (Over operator)
    let out_a = sa + da * (1.0 - sa);
    let out_r = (br * sa + dr * da * (1.0 - sa)) / out_a.max(1e-6);
    let out_g = (bg * sa + dg * da * (1.0 - sa)) / out_a.max(1e-6);
    let out_b = (bb * sa + db * da * (1.0 - sa)) / out_a.max(1e-6);

    [out_r.clamp(0.0, 1.0), out_g.clamp(0.0, 1.0), out_b.clamp(0.0, 1.0), out_a.clamp(0.0, 1.0)]
}
