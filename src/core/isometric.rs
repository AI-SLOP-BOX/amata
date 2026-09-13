use super::document::Object;
use super::path::AnchorPoint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsometricPlane {
    Top,
    Left,
    Right,
}

/// Transform 2D flat artwork into 2.5D Isometric projected artwork (SSR method: Scale, Shear, Rotate)
pub fn apply_isometric_transform(obj: &Object, plane: IsometricPlane) -> Object {
    let mut cloned = obj.clone();
    cloned.id = uuid::Uuid::new_v4().to_string();
    cloned.name = format!("{} (Iso {:?})", obj.name, plane);

    let mut path = cloned.to_path_data();

    // Standard 30-degree isometric SSR projection matrices
    let (m00, m01, m10, m11) = match plane {
        // Top: scale Y=0.866, shear X=30deg, rotate -30deg
        IsometricPlane::Top => (0.866025, -0.866025, 0.5, 0.5),
        // Left: scale Y=0.866, shear Y=-30deg, rotate -30deg
        IsometricPlane::Left => (0.866025, 0.0, -0.5, 1.0),
        // Right: scale Y=0.866, shear Y=30deg, rotate 30deg
        IsometricPlane::Right => (0.866025, 0.0, 0.5, 1.0),
    };

    let transform_pt = |p: AnchorPoint| -> AnchorPoint {
        AnchorPoint::new(p.x * m00 + p.y * m01, p.x * m10 + p.y * m11)
    };

    path.elements.iter_mut().for_each(|elem| match elem {
        crate::core::path::PathElement::MoveTo(p) | crate::core::path::PathElement::LineTo(p) => {
            *p = transform_pt(*p);
        }
        crate::core::path::PathElement::CurveTo(seg) => {
            seg.start = transform_pt(seg.start);
            seg.control1 = transform_pt(seg.control1);
            seg.control2 = transform_pt(seg.control2);
            seg.end = transform_pt(seg.end);
        }
        _ => {}
    });

    cloned.object_type = crate::core::document::ObjectType::Path(path);
    cloned
}
