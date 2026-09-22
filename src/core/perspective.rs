//! One/two-point perspective grid: vanishing points, horizon, fan rays
//! and ray snapping (Illustrator's perspective grid, lightweight edition:
//! no draw-plane widgets, just guides + snap).

use serde::{Deserialize, Serialize};

/// Perspective guide set stored on the document (`None` = never set up).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerspectiveGrid {
    /// Draw the grid overlay.
    pub show: bool,
    /// Snap dragged/created points to rays.
    pub snap: bool,
    /// Two vanishing points (false = one-point, left VP only).
    pub two_point: bool,
    /// World y of the horizon line.
    pub horizon_y: f64,
    /// Left (or single) vanishing point, world coords.
    pub left_vp: (f64, f64),
    /// Right vanishing point, world coords.
    pub right_vp: (f64, f64),
    /// Fan density per vanishing point.
    pub rays: usize,
}

impl PerspectiveGrid {
    /// Sensible default for a document: horizon at 35% height, VPs parked
    /// outside the canvas left/right.
    pub fn default_for_doc(w: f64, h: f64) -> Self {
        Self {
            show: true,
            snap: false,
            two_point: true,
            horizon_y: h * 0.35,
            left_vp: (-w * 0.5, h * 0.35),
            right_vp: (w * 1.5, h * 0.35),
            rays: 12,
        }
    }

    pub fn normalize(&mut self) {
        self.rays = self.rays.clamp(2, 64);
        for v in [
            &mut self.horizon_y,
            &mut self.left_vp.0,
            &mut self.left_vp.1,
            &mut self.right_vp.0,
            &mut self.right_vp.1,
        ] {
            if !v.is_finite() {
                *v = 0.0;
            }
        }
    }

    fn vps(&self) -> Vec<(f64, f64)> {
        if self.two_point {
            vec![self.left_vp, self.right_vp]
        } else {
            vec![self.left_vp]
        }
    }

    /// Fan ray angles (radians) from a VP across a canvas box.
    /// Falls back to a full circle when the VP sits inside the box.
    pub fn fan_angles(&self, vp: (f64, f64), canvas: (f64, f64, f64, f64)) -> Vec<f64> {
        let (x0, y0, x1, y1) = canvas;
        let corners = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)];
        let mut angs: Vec<f64> = corners
            .iter()
            .map(|(x, y)| (y - vp.1).atan2(x - vp.0))
            .collect();
        angs.sort_by(|a, b| a.total_cmp(b));
        // Largest angular gap = outside the canvas; span the complement.
        let n = angs.len();
        let mut best_gap = 0.0;
        let mut best_i = 0;
        for i in 0..n {
            let a = angs[i];
            let b = if i + 1 < n { angs[i + 1] } else { angs[0] + 2.0 * std::f64::consts::PI };
            if b - a > best_gap {
                best_gap = b - a;
                best_i = i;
            }
        }
        let span = 2.0 * std::f64::consts::PI - best_gap;
        let start = angs[(best_i + 1) % n];
        let rays = self.rays.max(2);
        if span >= std::f64::consts::PI * 1.5 {
            // VP inside (or hugging) the canvas: full circle.
            (0..rays).map(|i| start + i as f64 * 2.0 * std::f64::consts::PI / rays as f64).collect()
        } else {
            (0..rays)
                .map(|i| start + i as f64 * span / (rays - 1) as f64)
                .collect()
        }
    }

    /// Ray segments (world coords) for overlay drawing, extended well past
    /// the canvas so viewport clipping has something to cut.
    pub fn ray_segments(&self, canvas: (f64, f64, f64, f64)) -> Vec<((f64, f64), (f64, f64))> {
        let (x0, y0, x1, y1) = canvas;
        let diag = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt().max(1.0);
        let mut segs = Vec::new();
        for vp in self.vps() {
            for a in self.fan_angles(vp, canvas) {
                let (dx, dy) = (a.cos(), a.sin());
                segs.push((
                    (vp.0 - dx * diag * 2.0, vp.1 - dy * diag * 2.0),
                    (vp.0 + dx * diag * 2.0, vp.1 + dy * diag * 2.0),
                ));
            }
        }
        segs
    }

    /// Snap a world point to the nearest ray/horizon within `threshold`
    /// (world units). Returns the snapped point or `None`.
    pub fn snap_point(
        &self,
        x: f64,
        y: f64,
        canvas: (f64, f64, f64, f64),
        threshold: f64,
    ) -> Option<(f64, f64)> {
        if !self.snap {
            return None;
        }
        // Candidate lines as (point, direction).
        let mut lines: Vec<((f64, f64), (f64, f64))> = vec![((0.0, self.horizon_y), (1.0, 0.0))];
        for vp in self.vps() {
            for a in self.fan_angles(vp, canvas) {
                lines.push((vp, (a.cos(), a.sin())));
            }
        }
        let mut best: Option<(f64, f64)> = None;
        let mut best_d = threshold;
        for ((px, py), (dx, dy)) in lines {
            let len2 = dx * dx + dy * dy;
            if len2 <= 1e-12 {
                continue;
            }
            let t = ((x - px) * dx + (y - py) * dy) / len2;
            let (qx, qy) = (px + dx * t, py + dy * t);
            let d = ((x - qx).powi(2) + (y - qy).powi(2)).sqrt();
            if d < best_d {
                best_d = d;
                best = Some((qx, qy));
            }
        }
        best
    }
}

/// Liang-Barsky clip of a segment to an axis rect `(x0,y0,x1,y1)`.
/// Used by the overlay so rays never paint outside the canvas view.
pub fn clip_seg_to_rect(
    p0: (f64, f64),
    p1: (f64, f64),
    rect: (f64, f64, f64, f64),
) -> Option<((f64, f64), (f64, f64))> {
    let (x0, y0, x1, y1) = rect;
    let (rx0, rx1) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
    let (ry0, ry1) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
    let (mut t0, mut t1) = (0.0, 1.0);
    let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
    for (p, q) in [(-dx, p0.0 - rx0), (dx, rx1 - p0.0), (-dy, p0.1 - ry0), (dy, ry1 - p0.1)] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                if r > t1 {
                    return None;
                }
                if r > t0 {
                    t0 = r;
                }
            } else {
                if r < t0 {
                    return None;
                }
                if r < t1 {
                    t1 = r;
                }
            }
        }
    }
    Some((
        (p0.0 + dx * t0, p0.1 + dy * t0),
        (p0.0 + dx * t1, p0.1 + dy * t1),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> PerspectiveGrid {
        PerspectiveGrid::default_for_doc(400.0, 300.0)
    }

    #[test]
    fn fan_covers_canvas_corners() {
        let g = grid();
        let canvas = (0.0, 0.0, 400.0, 300.0);
        for vp in g.vps() {
            let angs = g.fan_angles(vp, canvas);
            assert_eq!(angs.len(), 12);
        }
        assert!(!g.ray_segments(canvas).is_empty());
    }

    #[test]
    fn snap_hits_rays_and_horizon() {
        let mut g = grid();
        g.snap = true;
        let canvas = (0.0, 0.0, 400.0, 300.0);
        // On the horizon already: snaps to itself.
        let p = g.snap_point(123.0, g.horizon_y, canvas, 5.0).expect("horizon");
        assert!((p.0 - 123.0).abs() < 1e-6 && (p.1 - g.horizon_y).abs() < 1e-9);
        // Far from everything: no snap.
        assert!(g.snap_point(200.0, 299.0, canvas, 0.01).is_none());
        // Near a VP ray: pulls onto it (result lies on some ray line).
        let near = g.snap_point(g.left_vp.0 + 50.0, g.left_vp.1 + 1.0, canvas, 5.0);
        assert!(near.is_some());
    }

    #[test]
    fn clip_keeps_inside_part() {
        let r = (0.0, 0.0, 10.0, 10.0);
        assert!(clip_seg_to_rect((20.0, 20.0), (30.0, 30.0), r).is_none());
        let kept = clip_seg_to_rect((-5.0, 5.0), (15.0, 5.0), r).expect("clipped");
        assert!((kept.0.0 - 0.0).abs() < 1e-9 && (kept.1.0 - 10.0).abs() < 1e-9);
    }
}
