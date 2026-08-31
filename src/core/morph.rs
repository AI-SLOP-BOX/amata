use super::path::{AnchorPoint, PathData};

/// Resample a polygon to have exactly `count` evenly distributed vertices
pub fn resample_polygon(points: &[AnchorPoint], count: usize) -> Vec<AnchorPoint> {
    if points.is_empty() || count == 0 {
        return Vec::new();
    }
    if points.len() == 1 {
        return vec![points[0]; count];
    }

    let mut lengths = vec![0.0];
    let mut total_len = 0.0;
    for i in 0..points.len() - 1 {
        let seg_len = points[i].distance(points[i + 1]);
        total_len += seg_len;
        lengths.push(total_len);
    }

    if total_len <= 1e-6 {
        return vec![points[0]; count];
    }

    let mut resampled = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f64 / (count.max(2) - 1) as f64;
        let target_dist = t * total_len;

        let mut pt = points[0];
        for j in 0..points.len() - 1 {
            let d0 = lengths[j];
            let d1 = lengths[j + 1];
            if target_dist >= d0 && target_dist <= d1 {
                let seg_len = (d1 - d0).max(1e-6);
                let seg_t = (target_dist - d0) / seg_len;
                let p0 = points[j];
                let p1 = points[j + 1];
                pt = AnchorPoint::new(
                    p0.x + seg_t * (p1.x - p0.x),
                    p0.y + seg_t * (p1.y - p0.y),
                );
                break;
            }
        }
        resampled.push(pt);
    }

    resampled
}

/// Interpolate between two polygon shapes at fraction `t` (0.0 to 1.0)
pub fn morph_polygons(poly_a: &[AnchorPoint], poly_b: &[AnchorPoint], t: f64) -> Vec<AnchorPoint> {
    let count = poly_a.len().max(poly_b.len()).max(32);
    let sample_a = resample_polygon(poly_a, count);
    let sample_b = resample_polygon(poly_b, count);

    let t = t.clamp(0.0, 1.0);
    let inv_t = 1.0 - t;

    let mut morphed = Vec::with_capacity(count);
    for i in 0..count {
        let pa = sample_a[i];
        let pb = sample_b[i];
        morphed.push(AnchorPoint::new(
            inv_t * pa.x + t * pb.x,
            inv_t * pa.y + t * pb.y,
        ));
    }
    morphed
}

/// Morph two PathData structures into an interpolated PathData
pub fn morph_paths(path_a: &PathData, path_b: &PathData, t: f64) -> PathData {
    let poly_a = path_a.to_polygon(16);
    let poly_b = path_b.to_polygon(16);
    let morphed_pts = morph_polygons(&poly_a, &poly_b, t);

    let mut result = PathData::from_polygon_points(&morphed_pts, path_a.closed || path_b.closed);
    
    // Interpolate fill and stroke color if available
    let t_f32 = t as f32;
    let inv_t_f32 = 1.0_f32 - t_f32;

    if let (Some(ref fa), Some(ref fb)) = (&path_a.fill, &path_b.fill) {
        let ca = fa.color;
        let cb = fb.color;
        let c = [
            inv_t_f32 * ca[0] + t_f32 * cb[0],
            inv_t_f32 * ca[1] + t_f32 * cb[1],
            inv_t_f32 * ca[2] + t_f32 * cb[2],
            inv_t_f32 * ca[3] + t_f32 * cb[3],
        ];
        result.fill = Some(super::path::FillStyle::solid(c));
    } else {
        result.fill = path_a.fill.clone().or_else(|| path_b.fill.clone());
    }

    result.stroke = path_a.stroke.clone().or_else(|| path_b.stroke.clone());
    result
}
