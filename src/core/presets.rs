use super::document::Object;
use super::path::{AnchorPoint, FillStyle, PathData, StrokeStyle};

pub struct PresetLibrary;

impl PresetLibrary {
    /// Create a heart shape
    pub fn heart(name: &str, cx: f64, cy: f64, size: f64) -> Object {
        let s = size * 0.5;
        let mut path = PathData::new();

        let top_center = AnchorPoint::new(cx, cy - s * 0.4);
        let bottom_tip = AnchorPoint::new(cx, cy + s);

        path.push_move_to(top_center.x, top_center.y);

        // Right lobe
        path.push_curve_to(
            AnchorPoint::new(cx + s * 0.8, cy - s * 1.2),
            AnchorPoint::new(cx + s * 1.3, cy + s * 0.2),
            bottom_tip,
        );

        // Left lobe
        path.push_curve_to(
            AnchorPoint::new(cx - s * 1.3, cy + s * 0.2),
            AnchorPoint::new(cx - s * 0.8, cy - s * 1.2),
            top_center,
        );

        path.close();

        let mut obj = Object::new_path(name, path);
        obj.fill = Some(FillStyle::solid([0.95, 0.2, 0.4, 1.0]));
        obj
    }

    /// Create a right-pointing block arrow
    pub fn arrow(name: &str, cx: f64, cy: f64, length: f64, thickness: f64) -> Object {
        let half_l = length * 0.5;
        let half_t = thickness * 0.5;
        let head_l = length * 0.4;
        let head_w = thickness * 1.5;

        let points = vec![
            AnchorPoint::new(cx - half_l, cy - half_t),
            AnchorPoint::new(cx + half_l - head_l, cy - half_t),
            AnchorPoint::new(cx + half_l - head_l, cy - head_w),
            AnchorPoint::new(cx + half_l, cy),
            AnchorPoint::new(cx + half_l - head_l, cy + head_w),
            AnchorPoint::new(cx + half_l - head_l, cy + half_t),
            AnchorPoint::new(cx - half_l, cy + half_t),
        ];

        let path = PathData::from_polygon_points(&points, true);
        let mut obj = Object::new_path(name, path);
        obj.fill = Some(FillStyle::solid([0.1, 0.6, 0.95, 1.0]));
        obj
    }

    /// Create a mechanical gear / cog
    pub fn gear(name: &str, cx: f64, cy: f64, teeth: usize, inner_r: f64, outer_r: f64) -> Object {
        let teeth = teeth.max(4);
        let total_pts = teeth * 4;
        let mut points = Vec::with_capacity(total_pts);

        for i in 0..total_pts {
            let angle = (i as f64 / total_pts as f64) * std::f64::consts::TAU;
            let r = match i % 4 {
                0 | 1 => outer_r,
                _ => inner_r,
            };
            points.push(AnchorPoint::new(cx + angle.cos() * r, cy + angle.sin() * r));
        }

        let path = PathData::from_polygon_points(&points, true);
        let mut obj = Object::new_path(name, path);
        obj.fill = Some(FillStyle::solid([0.5, 0.55, 0.6, 1.0]));
        obj.stroke = Some(StrokeStyle {
            color: [0.2, 0.25, 0.3, 1.0],
            width: 2.0,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        obj
    }

    /// Create a speech bubble with tail
    pub fn speech_bubble(name: &str, cx: f64, cy: f64, w: f64, h: f64) -> Object {
        let half_w = w * 0.5;
        let half_h = h * 0.5;

        let points = vec![
            AnchorPoint::new(cx - half_w, cy - half_h),
            AnchorPoint::new(cx + half_w, cy - half_h),
            AnchorPoint::new(cx + half_w, cy + half_h),
            AnchorPoint::new(cx - half_w * 0.2, cy + half_h),
            AnchorPoint::new(cx - half_w * 0.6, cy + half_h + h * 0.35),
            AnchorPoint::new(cx - half_w * 0.5, cy + half_h),
            AnchorPoint::new(cx - half_w, cy + half_h),
        ];

        let path = PathData::from_polygon_points(&points, true);
        let mut obj = Object::new_path(name, path);
        obj.fill = Some(FillStyle::solid([1.0, 0.95, 0.8, 1.0]));
        obj.stroke = Some(StrokeStyle {
            color: [0.2, 0.2, 0.2, 1.0],
            width: 2.5,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        obj
    }

    /// Create a sci-fi VFX energy portal / hexagonal ring
    pub fn vfx_portal(name: &str, cx: f64, cy: f64, radius: f64) -> Object {
        let sides = 6;
        let mut outer_pts = Vec::new();
        let mut inner_pts = Vec::new();

        for i in 0..sides {
            let angle = (i as f64 / sides as f64) * std::f64::consts::TAU;
            outer_pts.push(AnchorPoint::new(cx + angle.cos() * radius, cy + angle.sin() * radius));
            inner_pts.push(AnchorPoint::new(cx + angle.cos() * (radius * 0.75), cy + angle.sin() * (radius * 0.75)));
        }

        let mut path = PathData::from_polygon_points(&outer_pts, true);
        let mut inner_path = PathData::from_polygon_points(&inner_pts, true);
        path.elements.append(&mut inner_path.elements);

        let mut obj = Object::new_path(name, path);
        obj.fill = Some(FillStyle::solid([0.0, 0.85, 1.0, 0.85]));
        obj.stroke = Some(StrokeStyle {
            color: [0.8, 1.0, 1.0, 1.0],
            width: 3.0,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        obj
    }
}
