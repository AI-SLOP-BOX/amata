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
