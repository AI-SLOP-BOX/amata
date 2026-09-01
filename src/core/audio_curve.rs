use super::path::{AnchorPoint, PathData, StrokeStyle};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaveformType {
    Sine,
    Sawtooth,
    Square,
    Triangle,
    Harmonics,
    FM,
}

pub fn generate_audio_waveform(
    wave_type: WaveformType,
    freq: f64,
    harmonics: usize,
    width: f64,
    height: f64,
    samples: usize,
) -> PathData {
    let mut pts = Vec::with_capacity(samples);
    let cy = height * 0.5;
    let amp = height * 0.4;
    let sample_count = samples.max(32);

    for i in 0..sample_count {
        let t = i as f64 / (sample_count - 1) as f64;
        let phase = t * freq * TAU;

        let val = match wave_type {
            WaveformType::Sine => phase.sin(),
            WaveformType::Sawtooth => {
                let p = (phase / TAU).fract();
                2.0 * p - 1.0
            }
            WaveformType::Square => {
                if phase.sin() >= 0.0 { 1.0 } else { -1.0 }
            }
            WaveformType::Triangle => {
                let p = (phase / TAU).fract();
                if p < 0.5 {
                    4.0 * p - 1.0
                } else {
                    3.0 - 4.0 * p
                }
            }
            WaveformType::Harmonics => {
                let mut sum = 0.0;
                let mut norm = 0.0;
                for h in 1..=harmonics.max(1) {
                    let weight = 1.0 / h as f64;
                    sum += (phase * h as f64).sin() * weight;
                    norm += weight;
                }
                sum / norm.max(1e-6)
            }
            WaveformType::FM => {
                let modulator = (phase * 3.0).sin() * 2.5;
                (phase + modulator).sin()
            }
        };

        let px = t * width;
        let py = cy - val * amp;
        pts.push(AnchorPoint::new(px, py));
    }

    let mut path = PathData::from_polygon_points(&pts, false);
    path.fill = None;
    path.stroke = Some(StrokeStyle {
        color: match wave_type {
            WaveformType::Sine => [0.1, 0.8, 1.0, 1.0],
            WaveformType::Sawtooth => [1.0, 0.4, 0.1, 1.0],
            WaveformType::Square => [0.2, 1.0, 0.5, 1.0],
            WaveformType::Triangle => [1.0, 0.8, 0.1, 1.0],
            WaveformType::Harmonics => [0.8, 0.2, 1.0, 1.0],
            WaveformType::FM => [1.0, 0.1, 0.6, 1.0],
        },
        width: 2.0,
        dash_pattern: None,
    });

    path
}
