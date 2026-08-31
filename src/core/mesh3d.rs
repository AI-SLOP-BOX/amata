use super::geometry::signed_polygon_area;
use super::path::AnchorPoint;

#[derive(Debug, Clone, Default)]
pub struct Mesh3D {
    pub vertices: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub faces: Vec<[usize; 3]>, // 1-based indices in OBJ format
}

impl Mesh3D {
    pub fn new() -> Self {
        Self::default()
    }

    /// Export the mesh into Wavefront OBJ format
    pub fn to_obj(&self, name: &str) -> String {
        let mut obj = String::new();
        obj.push_str(&format!("# IRASU Illustrator 3D Mesh Export: {name}\n"));
        obj.push_str(&format!("o {name}\n\n"));

        for v in &self.vertices {
            obj.push_str(&format!("v {:.4} {:.4} {:.4}\n", v[0], v[1], v[2]));
        }
        obj.push('\n');

        for n in &self.normals {
            obj.push_str(&format!("vn {:.4} {:.4} {:.4}\n", n[0], n[1], n[2]));
        }
        obj.push('\n');

        for f in &self.faces {
            obj.push_str(&format!("f {}//{} {}//{} {}//{}\n", f[0], f[0], f[1], f[1], f[2], f[2]));
        }

        obj
    }
}

/// Triangulate a simple polygon using the Ear-Clipping algorithm
pub fn triangulate_polygon(poly: &[AnchorPoint]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }

    let is_ccw = signed_polygon_area(poly) >= 0.0;
    let mut indices: Vec<usize> = (0..n).collect();
    if !is_ccw {
        indices.reverse();
    }

    let mut triangles = Vec::new();
    let count = indices.len();
    let mut iterations = 0;
    let max_iterations = count * count;

    while indices.len() > 3 && iterations < max_iterations {
        iterations += 1;
        let mut ear_to_remove = None;
        let len = indices.len();

        for i in 0..len {
            let prev = if i == 0 { len - 1 } else { i - 1 };
            let next = (i + 1) % len;

            let u = indices[prev];
            let v = indices[i];
            let w = indices[next];

            let a = poly[u];
            let b = poly[v];
            let c = poly[w];

            // Convex check
            let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            if cross <= 1e-6 {
                continue;
            }

            // Check if any other vertex is inside this triangle
            let mut inside = false;
            for j in 0..len {
                if j == prev || j == i || j == next {
                    continue;
                }
                let pt = poly[indices[j]];
                if is_point_in_triangle(pt, a, b, c) {
                    inside = true;
                    break;
                }
            }

            if !inside {
                triangles.push([u, v, w]);
                ear_to_remove = Some(i);
                break;
            }
        }

        if let Some(idx) = ear_to_remove {
            indices.remove(idx);
        } else {
            break;
        }
    }

    if indices.len() == 3 {
        triangles.push([indices[0], indices[1], indices[2]]);
    }

    triangles
}

fn is_point_in_triangle(p: AnchorPoint, a: AnchorPoint, b: AnchorPoint, c: AnchorPoint) -> bool {
    let c1 = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let c2 = (c.x - b.x) * (p.y - b.y) - (c.y - b.y) * (p.x - b.x);
    let c3 = (a.x - c.x) * (p.y - c.y) - (a.y - c.y) * (p.x - c.x);

    (c1 >= -1e-6 && c2 >= -1e-6 && c3 >= -1e-6) || (c1 <= 1e-6 && c2 <= 1e-6 && c3 <= 1e-6)
}

/// Extrude a 2D polygon into a watertight 3D solid mesh with front cap, back cap, and side walls
pub fn extrude_polygon_3d(poly: &[AnchorPoint], depth: f64, bevel: f64) -> Mesh3D {
    let n = poly.len();
    if n < 3 {
        return Mesh3D::default();
    }

    let mut mesh = Mesh3D::new();
    let half_d = depth * 0.5;

    // 1. Front Cap Vertices (z = +half_d)
    for p in poly {
        mesh.vertices.push([p.x, p.y, half_d]);
        mesh.normals.push([0.0, 0.0, 1.0]);
    }

    // 2. Back Cap Vertices (z = -half_d)
    for p in poly {
        mesh.vertices.push([p.x, p.y, -half_d]);
        mesh.normals.push([0.0, 0.0, -1.0]);
    }

    // Triangulate front and back caps
    let cap_tris = triangulate_polygon(poly);

    // Front faces (1-based index)
    for tri in &cap_tris {
        mesh.faces.push([tri[0] + 1, tri[1] + 1, tri[2] + 1]);
    }

    // Back faces (flipped winding)
    for tri in &cap_tris {
        mesh.faces.push([n + tri[2] + 1, n + tri[1] + 1, n + tri[0] + 1]);
    }

    // 3. Side Walls
    for i in 0..n {
        let next = (i + 1) % n;
        let p1 = poly[i];
        let p2 = poly[next];

        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let nx = dy / len;
        let ny = -dx / len;

        let base_idx = mesh.vertices.len() + 1;

        // 4 wall vertices with normal
        mesh.vertices.push([p1.x, p1.y, half_d]);
        mesh.normals.push([nx, ny, 0.0]);

        mesh.vertices.push([p2.x, p2.y, half_d]);
        mesh.normals.push([nx, ny, 0.0]);

        mesh.vertices.push([p2.x, p2.y, -half_d]);
        mesh.normals.push([nx, ny, 0.0]);

        mesh.vertices.push([p1.x, p1.y, -half_d]);
        mesh.normals.push([nx, ny, 0.0]);

        // Quad split into 2 triangles
        mesh.faces.push([base_idx, base_idx + 1, base_idx + 2]);
        mesh.faces.push([base_idx, base_idx + 2, base_idx + 3]);
    }

    let _ = bevel; // Bevel available for future expansion
    mesh
}
