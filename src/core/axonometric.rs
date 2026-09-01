use super::document::Object;
use super::path::{AnchorPoint, PathElement};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AxonometricMode {
    Isometric,
    Dimetric,
    Trimetric,
    Cabinet,
    Cavalier,
}

pub fn apply_axonometric_projection(obj: &Object, mode: AxonometricMode) -> Object {
    let sqrt2_inv = std::f64::consts::FRAC_1_SQRT_2;
    let [m00, m01, m10, m11] = match mode {
        AxonometricMode::Isometric => [0.866025, -0.866025, 0.5, 0.5],
        AxonometricMode::Dimetric => [0.9659, -sqrt2_inv, 0.2588, sqrt2_inv],
        AxonometricMode::Trimetric => [0.9396, -0.6427, 0.3420, 0.7660],
        AxonometricMode::Cabinet => [1.0, -0.3535, 0.0, 0.3535],
        AxonometricMode::Cavalier => [1.0, -sqrt2_inv, 0.0, sqrt2_inv],
    };

    let project_point = |p: AnchorPoint| -> AnchorPoint {
        AnchorPoint::new(p.x * m00 + p.y * m01, p.x * m10 + p.y * m11)
    };

    let mut clone = obj.clone();
    clone.id = uuid::Uuid::new_v4().to_string();
    clone.name = format!("{} ({:?})", obj.name, mode);

    let mut path = clone.to_path_data();
    path.elements.iter_mut().for_each(|elem| match elem {
        PathElement::MoveTo(p) | PathElement::LineTo(p) => {
            *p = project_point(*p);
        }
        PathElement::CurveTo(seg) => {
            seg.start = project_point(seg.start);
            seg.control1 = project_point(seg.control1);
            seg.control2 = project_point(seg.control2);
            seg.end = project_point(seg.end);
        }
        _ => {}
    });

    clone.object_type = crate::core::document::ObjectType::Path(path);
    clone
}
