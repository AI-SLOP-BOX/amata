use super::document::Object;
use std::fmt::Write;

pub fn generate_3d_revolve_obj(
    profile_obj: &Object,
    axis_x: f64,
    angle_deg: f64,
    segments: usize,
) -> String {
    let poly = profile_obj.to_path_data().to_polygon(24);
    if poly.len() < 2 {
        return String::from("# Empty profile\n");
    }

    let n_pts = poly.len();
    let n_segs = segments.clamp(4, 4096);
    let total_angle = angle_deg.to_radians().clamp(0.1, std::f64::consts::TAU);

    let mut obj_str = String::new();
    let _ = writeln!(obj_str, "# IRASU Illustrator 3D Revolve Lathe OBJ");
    let _ = writeln!(obj_str, "o RevolveMesh");

    // Generate 3D Vertices
    for s in 0..=n_segs {
        let theta = (s as f64 / n_segs as f64) * total_angle;
        let cos = theta.cos();
        let sin = theta.sin();

        for pt in &poly {
            let r = pt.x - axis_x;
            let x = axis_x + r * cos;
            let y = -pt.y; // Invert Y for 3D coordinate convention
            let z = r * sin;

            let _ = writeln!(obj_str, "v {:.4} {:.4} {:.4}", x, y, z);
        }
    }

    // Generate Faces
    for s in 0..n_segs {
        for p in 0..n_pts - 1 {
            let i1 = s * n_pts + p + 1;
            let i2 = s * n_pts + (p + 1) + 1;
            let i3 = (s + 1) * n_pts + (p + 1) + 1;
            let i4 = (s + 1) * n_pts + p + 1;

            let _ = writeln!(obj_str, "f {} {} {} {}", i1, i2, i3, i4);
        }
    }

    obj_str
}
