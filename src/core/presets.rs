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
            outer_pts.push(AnchorPoint::new(
                cx + angle.cos() * radius,
                cy + angle.sin() * radius,
            ));
            inner_pts.push(AnchorPoint::new(
                cx + angle.cos() * (radius * 0.75),
                cy + angle.sin() * (radius * 0.75),
            ));
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

    /// Search Magnifying Glass Icon
    pub fn search_icon(name: &str, cx: f64, cy: f64, radius: f64) -> Object {
        let mut path = PathData::from_ellipse(
            cx - radius * 0.3,
            cy - radius * 0.3,
            radius * 0.6,
            radius * 0.6,
        );
        path.fill = None;
        path.stroke = Some(StrokeStyle {
            color: [0.2, 0.2, 0.25, 1.0],
            width: 3.0,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        // Handle line
        path.push_move_to(cx + radius * 0.2, cy + radius * 0.2);
        path.push_line_to(cx + radius * 0.85, cy + radius * 0.85);

        let mut obj = Object::new_path(name, path);
        obj.fill = None;
        obj.stroke = Some(StrokeStyle {
            color: [0.2, 0.2, 0.25, 1.0],
            width: 3.5,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        obj
    }

    /// User Profile Avatar Icon
    pub fn user_avatar(name: &str, cx: f64, cy: f64, size: f64) -> Object {
        let r = size * 0.5;
        let mut head = PathData::from_ellipse(cx, cy - r * 0.35, r * 0.35, r * 0.35);
        // Body shoulder curve
        head.push_move_to(cx - r * 0.7, cy + r * 0.8);
        head.push_curve_to(
            AnchorPoint::new(cx - r * 0.7, cy + r * 0.2),
            AnchorPoint::new(cx + r * 0.7, cy + r * 0.2),
            AnchorPoint::new(cx + r * 0.7, cy + r * 0.8),
        );
        head.close();

        let mut obj = Object::new_path(name, head);
        obj.fill = Some(FillStyle::solid([0.2, 0.5, 0.9, 1.0]));
        obj.stroke = None;
        obj
    }

    /// Cloud Icon
    pub fn cloud(name: &str, cx: f64, cy: f64, width: f64) -> Object {
        let w = width * 0.5;
        let h = width * 0.3;
        let mut path = PathData::new();

        path.push_move_to(cx - w * 0.6, cy + h * 0.5);
        path.push_curve_to(
            AnchorPoint::new(cx - w * 0.9, cy + h * 0.3),
            AnchorPoint::new(cx - w * 0.9, cy - h * 0.2),
            AnchorPoint::new(cx - w * 0.5, cy - h * 0.3),
        );
        path.push_curve_to(
            AnchorPoint::new(cx - w * 0.4, cy - h * 0.9),
            AnchorPoint::new(cx + w * 0.2, cy - h * 0.9),
            AnchorPoint::new(cx + w * 0.3, cy - h * 0.3),
        );
        path.push_curve_to(
            AnchorPoint::new(cx + w * 0.8, cy - h * 0.2),
            AnchorPoint::new(cx + w * 0.9, cy + h * 0.4),
            AnchorPoint::new(cx + w * 0.6, cy + h * 0.5),
        );
        path.close();

        let mut obj = Object::new_path(name, path);
        obj.fill = Some(FillStyle::solid([0.3, 0.7, 1.0, 0.9]));
        obj
    }

    /// Shopping Cart Icon
    pub fn shopping_cart(name: &str, cx: f64, cy: f64, size: f64) -> Object {
        let s = size * 0.5;
        let mut path = PathData::new();
        // Basket outline
        path.push_move_to(cx - s * 0.8, cy - s * 0.6);
        path.push_line_to(cx - s * 0.5, cy - s * 0.6);
        path.push_line_to(cx - s * 0.3, cy + s * 0.2);
        path.push_line_to(cx + s * 0.6, cy + s * 0.2);
        path.push_line_to(cx + s * 0.8, cy - s * 0.4);
        path.push_line_to(cx - s * 0.4, cy - s * 0.4);

        let mut obj = Object::new_path(name, path);
        obj.fill = None;
        obj.stroke = Some(StrokeStyle {
            color: [0.15, 0.15, 0.2, 1.0],
            width: 3.0,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        obj
    }

    /// Ribbon Badge Banner
    pub fn ribbon_badge(name: &str, cx: f64, cy: f64, w: f64, h: f64) -> Object {
        let hw = w * 0.5;
        let hh = h * 0.5;
        let points = vec![
            AnchorPoint::new(cx - hw, cy - hh),
            AnchorPoint::new(cx + hw, cy - hh),
            AnchorPoint::new(cx + hw - hh * 0.5, cy),
            AnchorPoint::new(cx + hw, cy + hh),
            AnchorPoint::new(cx - hw, cy + hh),
            AnchorPoint::new(cx - hw + hh * 0.5, cy),
        ];
        let path = PathData::from_polygon_points(&points, true);
        let mut obj = Object::new_path(name, path);
        obj.fill = Some(FillStyle::solid([1.0, 0.75, 0.1, 1.0]));
        obj.stroke = Some(StrokeStyle {
            color: [0.85, 0.5, 0.0, 1.0],
            width: 2.0,
            dash_pattern: None,
            ..StrokeStyle::default()
        });
        obj
    }
}
