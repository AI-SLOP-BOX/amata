use super::path::{AnchorPoint, FillStyle, PathData, StrokeStyle};
use std::f64::consts::TAU;

pub struct FormulaCurves;

impl FormulaCurves {
    /// Archimedean Spiral
    pub fn spiral(
        cx: f64,
        cy: f64,
        turns: f64,
        initial_r: f64,
        growth: f64,
        samples: usize,
    ) -> PathData {
        let samples = samples.clamp(32, 100_000);
        let max_theta = turns * TAU;
        let mut points = Vec::with_capacity(samples);

        for i in 0..samples {
            let theta = (i as f64 / (samples - 1) as f64) * max_theta;
            let r = initial_r + growth * theta;
            let x = cx + r * theta.cos();
            let y = cy + r * theta.sin();
            points.push(AnchorPoint::new(x, y));
        }

        let mut path = PathData::from_polygon_points(&points, false);
        path.fill = None;
        path.stroke = Some(StrokeStyle {
            color: [0.0, 0.8, 1.0, 1.0],
            width: 2.5,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        path
    }

    /// Lissajous Curve (Oscilloscope Cyberpunk Waveform)
    #[allow(clippy::too_many_arguments)]
    pub fn lissajous(
        cx: f64,
        cy: f64,
        a_freq: f64,
        b_freq: f64,
        phase_delta: f64,
        width: f64,
        height: f64,
        samples: usize,
    ) -> PathData {
        let samples = samples.clamp(64, 100_000);
        let half_w = width * 0.5;
        let half_h = height * 0.5;
        let mut points = Vec::with_capacity(samples);

        for i in 0..samples {
            let t = (i as f64 / samples as f64) * TAU;
            let x = cx + half_w * (a_freq * t + phase_delta).sin();
            let y = cy + half_h * (b_freq * t).sin();
            points.push(AnchorPoint::new(x, y));
        }

        let mut path = PathData::from_polygon_points(&points, true);
        path.fill = None;
        path.stroke = Some(StrokeStyle {
            color: [0.2, 1.0, 0.4, 1.0],
            width: 2.0,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        path
    }

    /// Spirograph (Hypotrochoid Geometric Pattern)
    pub fn spirograph(
        cx: f64,
        cy: f64,
        r_outer: f64,
        r_inner: f64,
        d_pen: f64,
        cycles: usize,
        samples_per_cycle: usize,
    ) -> PathData {
        // Guard the cycles*per-cycle multiplication against overflow/OOM.
        let cycles = cycles.min(4096);
        let total_samples = cycles
            .saturating_mul(samples_per_cycle.clamp(32, 4096))
            .clamp(32, 1_000_000);
        let max_t = cycles as f64 * TAU;
        let diff = r_outer - r_inner;
        let ratio = diff / r_inner.max(1e-4);

        let mut points = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = (i as f64 / total_samples as f64) * max_t;
            let x = cx + diff * t.cos() + d_pen * (ratio * t).cos();
            let y = cy + diff * t.sin() - d_pen * (ratio * t).sin();
            points.push(AnchorPoint::new(x, y));
        }

        let mut path = PathData::from_polygon_points(&points, true);
        path.fill = None;
        path.stroke = Some(StrokeStyle {
            color: [0.95, 0.3, 0.8, 1.0],
            width: 1.5,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        path
    }

    /// Rose / Rhodonea Flower Curve
    pub fn rose_curve(cx: f64, cy: f64, petals_k: f64, radius: f64, samples: usize) -> PathData {
        let samples = samples.clamp(64, 100_000);
        let max_theta = if (petals_k - petals_k.round()).abs() < 1e-4 && (petals_k as i64) % 2 != 0
        {
            std::f64::consts::PI
        } else {
            TAU
        };

        let mut points = Vec::with_capacity(samples);

        for i in 0..samples {
            let theta = (i as f64 / samples as f64) * max_theta;
            let r = radius * (petals_k * theta).cos();
            let x = cx + r * theta.cos();
            let y = cy + r * theta.sin();
            points.push(AnchorPoint::new(x, y));
        }

        let mut path = PathData::from_polygon_points(&points, true);
        path.fill = Some(FillStyle::solid([1.0, 0.6, 0.8, 0.4]));
        path.stroke = Some(StrokeStyle {
            color: [0.9, 0.1, 0.4, 1.0],
            width: 2.0,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        path
    }
}
