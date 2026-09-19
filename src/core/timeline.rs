use super::document::Document;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimProperty {
    PositionX,
    PositionY,
    Rotation,
    ScaleX,
    ScaleY,
    Opacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EaseType {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl EaseType {
    pub fn apply(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            EaseType::Linear => t,
            EaseType::EaseIn => t * t,
            EaseType::EaseOut => t * (2.0 - t),
            EaseType::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub frame: usize,
    pub value: f64,
    pub ease: EaseType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub object_id: String,
    pub property: AnimProperty,
    pub keyframes: Vec<Keyframe>,
}

impl Track {
    pub fn new(object_id: &str, property: AnimProperty) -> Self {
        Self {
            object_id: object_id.to_string(),
            property,
            keyframes: Vec::new(),
        }
    }

    pub fn add_keyframe(&mut self, frame: usize, value: f64, ease: EaseType) {
        if let Some(pos) = self.keyframes.iter().position(|k| k.frame == frame) {
            self.keyframes[pos].value = value;
            self.keyframes[pos].ease = ease;
        } else {
            self.keyframes.push(Keyframe { frame, value, ease });
            self.keyframes.sort_by_key(|k| k.frame);
        }
    }

    pub fn eval_at(&self, frame: usize) -> Option<f64> {
        if self.keyframes.is_empty() {
            return None;
        }
        if self.keyframes.len() == 1 || frame <= self.keyframes.first().unwrap().frame {
            return Some(self.keyframes.first().unwrap().value);
        }
        if frame >= self.keyframes.last().unwrap().frame {
            return Some(self.keyframes.last().unwrap().value);
        }

        for i in 0..self.keyframes.len() - 1 {
            let k0 = &self.keyframes[i];
            let k1 = &self.keyframes[i + 1];
            if frame >= k0.frame && frame <= k1.frame {
                let range = (k1.frame - k0.frame) as f64;
                if range <= 0.0 {
                    return Some(k0.value);
                }
                let raw_t = (frame - k0.frame) as f64 / range;
                let eased_t = k0.ease.apply(raw_t);
                return Some(k0.value + eased_t * (k1.value - k0.value));
            }
        }

        Some(self.keyframes.last().unwrap().value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub fps: f64,
    pub total_frames: usize,
    pub current_frame: usize,
    pub is_playing: bool,
    pub loop_playback: bool,
    pub tracks: Vec<Track>,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            fps: 60.0,
            total_frames: 180, // 3 seconds at 60fps
            current_frame: 0,
            is_playing: false,
            loop_playback: true,
            tracks: Vec::new(),
        }
    }
}

impl Timeline {
    pub fn new(fps: f64, duration_secs: f64) -> Self {
        let total_frames = (fps * duration_secs).round() as usize;
        Self {
            fps,
            total_frames: total_frames.max(1),
            current_frame: 0,
            is_playing: false,
            loop_playback: true,
            tracks: Vec::new(),
        }
    }

    pub fn advance_frame(&mut self) {
        if self.current_frame + 1 >= self.total_frames {
            if self.loop_playback {
                self.current_frame = 0;
            } else {
                self.is_playing = false;
            }
        } else {
            self.current_frame += 1;
        }
    }

    pub fn add_or_get_track_mut(&mut self, object_id: &str, property: AnimProperty) -> &mut Track {
        if let Some(pos) = self
            .tracks
            .iter()
            .position(|t| t.object_id == object_id && t.property == property)
        {
            &mut self.tracks[pos]
        } else {
            let track = Track::new(object_id, property);
            self.tracks.push(track);
            self.tracks.last_mut().unwrap()
        }
    }

    /// Apply the evaluated animated values at the current frame to all objects in the document
    pub fn apply_to_document(&self, doc: &mut Document) {
        for track in &self.tracks {
            if let Some(val) = track.eval_at(self.current_frame) {
                if let Some(obj) = doc.find_object_mut(&track.object_id) {
                    match track.property {
                        AnimProperty::PositionX => obj.transform.x = val,
                        AnimProperty::PositionY => obj.transform.y = val,
                        AnimProperty::Rotation => obj.transform.rotation = val.to_radians(),
                        AnimProperty::ScaleX => obj.transform.scale_x = val,
                        AnimProperty::ScaleY => obj.transform.scale_y = val,
                        AnimProperty::Opacity => obj.opacity = val as f32,
                    }
                }
            }
        }
    }
}
