use super::document::Object;
use super::path::{AnchorPoint, FillStyle, PathData};

pub fn evolve_vector_composition(
    width: f64,
    height: f64,
    polygon_count: usize,
    generations: usize,
) -> Vec<Object> {
    let poly_count = polygon_count.clamp(5, 5000);
    let mut objects = Vec::with_capacity(poly_count);

    let pseudo_rand =
        |seed: f64| -> f64 { ((seed * 37.123 + 12.345).sin() * 43758.5453).fract().abs() };

    for i in 0..poly_count {
        let mut best_pts = Vec::new();
        let mut best_color = [0.0; 4];

        // Simulate generational mutations
        let gen_count = generations.min(100);
        for g in 0..gen_count {
            let s = (i * 1000 + g) as f64;
            let cx = pseudo_rand(s) * width;
            let cy = pseudo_rand(s + 1.0) * height;
            let rad = 20.0 + pseudo_rand(s + 2.0) * (width * 0.25);

            let pt_count = 3 + (pseudo_rand(s + 3.0) * 3.0).floor() as usize;
            let mut pts = Vec::with_capacity(pt_count);

            for p in 0..pt_count {
                let angle = (p as f64 / pt_count as f64) * std::f64::consts::TAU
                    + pseudo_rand(s + p as f64 * 4.0);
                let r = rad * (0.6 + 0.4 * pseudo_rand(s + p as f64 * 7.0));
                pts.push(AnchorPoint::new(
                    (cx + angle.cos() * r).clamp(0.0, width),
                    (cy + angle.sin() * r).clamp(0.0, height),
                ));
            }

            let hue = pseudo_rand(s + 5.0) as f32;
            let alpha = 0.2 + 0.4 * pseudo_rand(s + 6.0) as f32;
            let [r, g, b] = hsv_to_rgb(hue, 0.75, 0.85);

            best_pts = pts;
            best_color = [r, g, b, alpha];
        }

        let mut path = PathData::from_polygon_points(&best_pts, true);
        path.fill = Some(FillStyle::solid(best_color));
        path.stroke = None;

        let obj = Object::new_path(&format!("Evolved Polygon {}", i + 1), path);
        objects.push(obj);
    }

    objects
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let i = (h * 6.0).floor() as usize;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match i % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}
