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
}
