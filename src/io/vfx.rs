use crate::core::document::{Document, Object, ObjectType};
use crate::core::path::{AnchorPoint, PathElement};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AevfxKeyframe {
    pub frame: usize,
    pub time_sec: f64,
    pub position: [f64; 3],
    pub tangent_in: [f64; 3],
    pub tangent_out: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AevfxPathSpline {
    pub closed: bool,
    pub vertices: Vec<[f64; 2]>,
    pub in_tangents: Vec<[f64; 2]>,
    pub out_tangents: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AevfxTransform {
    pub position: [f64; 3],
    pub rotation: [f64; 3], // degrees: [pitch, yaw, roll]
    pub scale: [f64; 3],
    pub opacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AevfxMaterial {
    pub fill_color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub stroke_width: f64,
    pub metallic: f32,
    pub roughness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AevfxLayer {
    pub id: String,
    pub name: String,
    pub layer_type: String, // "Vector", "Shape", "Text", "Extrusion3D", "Mask"
    pub transform: AevfxTransform,
    pub material: AevfxMaterial,
    pub extrusion_depth: f64,
    pub bevel_radius: f64,
    pub splines: Vec<AevfxPathSpline>,
    pub text_content: Option<String>,
    pub font_size: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AevfxComp {
    pub comp_name: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration_seconds: f64,
    pub total_frames: usize,
    pub layers: Vec<AevfxLayer>,
}

/// Convert an IRASU Illustrator document into an AEVFX Studio compatible composition
pub fn doc_to_aevfx_comp(doc: &Document, fps: f64, duration_sec: f64) -> AevfxComp {
    let fps = if fps > 0.0 { fps } else { 60.0 };
    let duration_sec = if duration_sec > 0.0 { duration_sec } else { 5.0 };
    let total_frames = (fps * duration_sec).round() as usize;

    let mut vfx_layers = Vec::new();

    for (layer_idx, layer) in doc.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }

        for (obj_idx, obj) in layer.objects.iter().enumerate() {
            if !obj.visible {
                continue;
            }

            let layer_type = match &obj.object_type {
                ObjectType::Text { .. } => "Text",
                ObjectType::Path(_) | ObjectType::Star { .. } | ObjectType::Polygon { .. } => "Extrusion3D",
                ObjectType::Line { .. } => "Vector",
                _ => "Shape",
            };

            let fill_col = obj.fill.as_ref().map(|f| f.color).unwrap_or([1.0, 1.0, 1.0, 1.0]);
            let stroke_col = obj.stroke.as_ref().map(|s| s.color).unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let stroke_w = obj.stroke.as_ref().map(|s| s.width).unwrap_or(1.0);

            let (text_content, font_size) = match &obj.object_type {
                ObjectType::Text { text, font_size } => (Some(text.clone()), Some(*font_size)),
                _ => (None, None),
            };

            let splines = vec![object_to_vfx_spline(obj)];

            let vfx_layer = AevfxLayer {
                id: obj.id.clone(),
                name: format!("{}_{}_{}", layer.name, obj.name, obj_idx + 1),
                layer_type: layer_type.to_string(),
                transform: AevfxTransform {
                    position: [obj.transform.x, obj.transform.y, layer_idx as f64 * -10.0],
                    rotation: [0.0, 0.0, obj.transform.rotation.to_degrees()],
                    scale: [obj.transform.scale_x, obj.transform.scale_y, 1.0],
                    opacity: (obj.opacity * layer.opacity) as f64,
                },
                material: AevfxMaterial {
                    fill_color: fill_col,
                    stroke_color: stroke_col,
                    stroke_width: stroke_w,
                    metallic: 0.1,
                    roughness: 0.4,
                },
                extrusion_depth: 25.0, // Cinema 4D / 3D extrusion depth
                bevel_radius: 2.0,
                splines,
                text_content,
                font_size,
            };

            vfx_layers.push(vfx_layer);
        }
    }

    AevfxComp {
        comp_name: doc.name.clone(),
        width: doc.width as u32,
        height: doc.height as u32,
        fps,
        duration_seconds: duration_sec,
        total_frames,
        layers: vfx_layers,
    }
}

/// Convert an object's path into an AEVFX PathSpline
pub fn object_to_vfx_spline(obj: &Object) -> AevfxPathSpline {
    let path = obj.to_path_data();
    let mut vertices = Vec::new();
    let mut in_tangents = Vec::new();
    let mut out_tangents = Vec::new();
    let mut closed = false;

    for elem in &path.elements {
        match elem {
            PathElement::MoveTo(p) | PathElement::LineTo(p) => {
                vertices.push([p.x, p.y]);
                in_tangents.push([0.0, 0.0]);
                out_tangents.push([0.0, 0.0]);
            }
            PathElement::CurveTo(seg) => {
                // seg.control1 is out-tangent from previous point, seg.control2 is in-tangent to end point
                if let Some(last_v) = vertices.last() {
                    if let Some(last_out) = out_tangents.last_mut() {
                        *last_out = [seg.control1.x - last_v[0], seg.control1.y - last_v[1]];
                    }
                }
                vertices.push([seg.end.x, seg.end.y]);
                in_tangents.push([seg.control2.x - seg.end.x, seg.control2.y - seg.end.y]);
                out_tangents.push([0.0, 0.0]);
            }
            PathElement::ClosePath => {
                closed = true;
            }
        }
    }

    AevfxPathSpline {
        closed,
        vertices,
        in_tangents,
        out_tangents,
    }
}

/// Sample position keyframes along a path for AEVFX camera/particle/layer motion paths
pub fn object_to_motion_path_keyframes(
    obj: &Object,
    num_samples: usize,
    duration_sec: f64,
    fps: f64,
) -> Vec<AevfxKeyframe> {
    let poly = obj.to_world_polygon();
    if poly.len() < 2 {
        return Vec::new();
    }

    let samples = num_samples.max(2);
    let total_frames = (duration_sec * fps).max(1.0) as usize;
    let mut keyframes = Vec::with_capacity(samples);

    // Calculate total path perimeter length
    let mut cumulative_lengths = vec![0.0];
    let mut total_len = 0.0;
    for i in 0..poly.len() - 1 {
        let seg_len = poly[i].distance(poly[i + 1]);
        total_len += seg_len;
        cumulative_lengths.push(total_len);
    }

    if total_len <= 1e-6 {
        return Vec::new();
    }

    for s in 0..samples {
        let t = s as f64 / (samples - 1) as f64;
        let target_dist = t * total_len;
        let time_sec = t * duration_sec;
        let frame = (t * total_frames as f64).round() as usize;

        // Find segment
        let mut pt = poly[0];
        let mut tangent = [1.0, 0.0, 0.0];

        for i in 0..poly.len() - 1 {
            let d0 = cumulative_lengths[i];
            let d1 = cumulative_lengths[i + 1];
            if target_dist >= d0 && target_dist <= d1 {
                let seg_len = (d1 - d0).max(1e-6);
                let seg_t = (target_dist - d0) / seg_len;
                let p0 = poly[i];
                let p1 = poly[i + 1];
                pt = AnchorPoint::new(
                    p0.x + seg_t * (p1.x - p0.x),
                    p0.y + seg_t * (p1.y - p0.y),
                );
                let dx = p1.x - p0.x;
                let dy = p1.y - p0.y;
                let len = (dx * dx + dy * dy).sqrt().max(1e-6);
                tangent = [dx / len, dy / len, 0.0];
                break;
            }
        }

        keyframes.push(AevfxKeyframe {
            frame,
            time_sec,
            position: [pt.x, pt.y, 0.0],
            tangent_in: [-tangent[0] * 10.0, -tangent[1] * 10.0, 0.0],
            tangent_out: [tangent[0] * 10.0, tangent[1] * 10.0, 0.0],
        });
    }

    keyframes
}

/// Export all visible objects in a document as a combined 3D OBJ mesh file
pub fn export_doc_to_obj(doc: &Document, depth: f64, bevel: f64) -> String {
    let mut out = String::new();
    out.push_str(&format!("# IRASU Illustrator 3D Mesh Export: {}\n\n", doc.name));

    let mut total_verts: Vec<[f64; 3]> = Vec::new();
    let mut total_normals: Vec<[f64; 3]> = Vec::new();
    let mut total_faces: Vec<[usize; 3]> = Vec::new();

    for (layer_idx, layer) in doc.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }

        for obj in &layer.objects {
            if !obj.visible {
                continue;
            }

            let poly = obj.to_world_polygon();
            if poly.len() < 3 {
                continue;
            }

            let mesh = crate::core::mesh3d::extrude_polygon_3d(&poly, depth, bevel);
            let v_offset = total_verts.len();

            for v in &mesh.vertices {
                total_verts.push([v[0], -v[1], v[2] - (layer_idx as f64 * depth)]);
            }
            for n in &mesh.normals {
                total_normals.push([n[0], -n[1], n[2]]);
            }
            for f in &mesh.faces {
                total_faces.push([f[0] + v_offset, f[1] + v_offset, f[2] + v_offset]);
            }
        }
    }

    for v in &total_verts {
        out.push_str(&format!("v {:.4} {:.4} {:.4}\n", v[0], v[1], v[2]));
    }
    out.push('\n');

    for n in &total_normals {
        out.push_str(&format!("vn {:.4} {:.4} {:.4}\n", n[0], n[1], n[2]));
    }
    out.push('\n');

    for f in &total_faces {
        out.push_str(&format!("f {}//{} {}//{} {}//{}\n", f[0], f[0], f[1], f[1], f[2], f[2]));
    }

    out
}
