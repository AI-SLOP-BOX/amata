use super::document::Object;
use super::path::{AnchorPoint, FillStyle, PathData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientMeshPreset {
    Sunset,
    Cyberpunk,
    Aurora,
    Gold,
}

pub fn generate_gradient_mesh(
    preset: GradientMeshPreset,
    width: f64,
    height: f64,
    rows: usize,
    cols: usize,
) -> Vec<Object> {
    // (r-1)*(c-1) feeds with_capacity; bound it against overflow/OOM.
    let r_count = rows.clamp(2, 512);
    let c_count = cols.clamp(2, 512);
    let mut objects = Vec::with_capacity((r_count - 1) * (c_count - 1));

    // Define corner colors based on preset
    let (c_tl, c_tr, c_bl, c_br) = match preset {
        GradientMeshPreset::Sunset => (
            [1.0, 0.2, 0.4, 0.95],
            [1.0, 0.6, 0.1, 0.95],
            [0.4, 0.1, 0.6, 0.95],
            [0.1, 0.05, 0.3, 0.95],
        ),
        GradientMeshPreset::Cyberpunk => (
            [0.0, 0.9, 1.0, 0.95],
            [1.0, 0.1, 0.6, 0.95],
            [0.1, 0.0, 0.4, 0.95],
            [0.0, 1.0, 0.5, 0.95],
        ),
        GradientMeshPreset::Aurora => (
            [0.1, 0.9, 0.5, 0.95],
            [0.1, 0.5, 0.9, 0.95],
            [0.5, 0.1, 0.8, 0.95],
            [0.05, 0.2, 0.4, 0.95],
        ),
        GradientMeshPreset::Gold => (
            [1.0, 0.9, 0.5, 0.95],
            [0.9, 0.6, 0.2, 0.95],
            [0.6, 0.4, 0.1, 0.95],
            [0.3, 0.2, 0.05, 0.95],
        ),
    };

    let sample_color = |u: f64, v: f64| -> [f32; 4] {
        let u_f = u as f32;
        let v_f = v as f32;
        let mut out = [0.0; 4];
        for i in 0..4 {
            let top = c_tl[i] + u_f * (c_tr[i] - c_tl[i]);
            let bot = c_bl[i] + u_f * (c_br[i] - c_bl[i]);
            out[i] = top + v_f * (bot - top);
        }
        out
    };

    let sub_steps = 4; // Sub-division for smooth vector color patches
    let cell_w = width / (c_count - 1) as f64;
    let cell_h = height / (r_count - 1) as f64;

    for r in 0..r_count - 1 {
        for c in 0..c_count - 1 {
            let x0 = c as f64 * cell_w;
            let y0 = r as f64 * cell_h;

            for sr in 0..sub_steps {
                for sc in 0..sub_steps {
                    let u0 = (c as f64 + sc as f64 / sub_steps as f64) / (c_count - 1) as f64;
                    let v0 = (r as f64 + sr as f64 / sub_steps as f64) / (r_count - 1) as f64;
                    let u1 = (c as f64 + (sc + 1) as f64 / sub_steps as f64) / (c_count - 1) as f64;
                    let v1 = (r as f64 + (sr + 1) as f64 / sub_steps as f64) / (r_count - 1) as f64;

                    let px0 = x0 + (sc as f64 / sub_steps as f64) * cell_w;
                    let py0 = y0 + (sr as f64 / sub_steps as f64) * cell_h;
                    let px1 = x0 + ((sc + 1) as f64 / sub_steps as f64) * cell_w;
                    let py1 = y0 + ((sr + 1) as f64 / sub_steps as f64) * cell_h;

                    let pts = vec![
                        AnchorPoint::new(px0, py0),
                        AnchorPoint::new(px1, py0),
                        AnchorPoint::new(px1, py1),
                        AnchorPoint::new(px0, py1),
                    ];

                    let color = sample_color((u0 + u1) * 0.5, (v0 + v1) * 0.5);
                    let mut path = PathData::from_polygon_points(&pts, true);
                    path.fill = Some(FillStyle::solid(color));
                    path.stroke = None;

                    let obj = Object::new_path(
                        &format!("Mesh Patch [{},{}]-({},{})", r, c, sr, sc),
                        path,
                    );
                    objects.push(obj);
                }
            }
        }
    }

    objects
}

/// Maximum nodes per side (16×16 nodes is plenty; subdivision multiplies
/// the triangle count from there).
pub const MAX_MESH_DIM: usize = 16;
/// Clamp for per-patch subdivision in renderers/exporters.
pub const MAX_MESH_SUBDIV: usize = 12;

/// One mesh node: position plus its color.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeshNode {
    pub x: f64,
    pub y: f64,
    pub color: [f32; 4],
}

/// A real gradient mesh: a rows×cols grid of colored nodes forming smooth
/// bicubic Hermite patches (Catmull-Rom auto-tangents, bilinear color).
/// Positions are local coordinates; the object's transform places them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshGradient {
    /// Node rows (vertical, ≥ 2).
    pub rows: usize,
    /// Node columns (horizontal, ≥ 2).
    pub cols: usize,
    /// Row-major nodes, `rows * cols` entries.
    pub nodes: Vec<MeshNode>,
}

impl MeshGradient {
    /// Regular grid over a rect with bilinear corner colors
    /// ([top-left, top-right, bottom-left, bottom-right]).
    pub fn new_rect(
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        rows: usize,
        cols: usize,
        corners: [[f32; 4]; 4],
    ) -> Self {
        let rows = rows.clamp(2, MAX_MESH_DIM);
        let cols = cols.clamp(2, MAX_MESH_DIM);
        let mut nodes = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            let v = r as f64 / (rows - 1) as f64;
            for c in 0..cols {
                let u = c as f64 / (cols - 1) as f64;
                let mut color = [0.0; 4];
                for i in 0..4 {
                    let top = corners[0][i] + u as f32 * (corners[1][i] - corners[0][i]);
                    let bot = corners[2][i] + u as f32 * (corners[3][i] - corners[2][i]);
                    color[i] = top + v as f32 * (bot - top);
                }
                nodes.push(MeshNode {
                    x: x + u * w,
                    y: y + v * h,
                    color,
                });
            }
        }
        Self { rows, cols, nodes }
    }

    /// Repair invariants after deserialization or palette edits.
    pub fn normalize(&mut self) {
        self.rows = self.rows.clamp(2, MAX_MESH_DIM);
        self.cols = self.cols.clamp(2, MAX_MESH_DIM);
        let want = self.rows * self.cols;
        let fill = self.nodes.last().copied().unwrap_or(MeshNode {
            x: 0.0,
            y: 0.0,
            color: [0.0, 0.0, 0.0, 1.0],
        });
        self.nodes.resize(want, fill);
        self.nodes.truncate(want);
    }

    pub fn node(&self, r: usize, c: usize) -> MeshNode {
        self.nodes[(r.min(self.rows - 1)) * self.cols + (c.min(self.cols - 1))]
    }

    pub fn node_mut(&mut self, r: usize, c: usize) -> &mut MeshNode {
        let r = r.min(self.rows - 1);
        let c = c.min(self.cols - 1);
        &mut self.nodes[r * self.cols + c]
    }

    /// Catmull-Rom tangent along +u / +v (finite differences, clamped ends).
    fn tangent_u(&self, r: usize, c: usize) -> (f64, f64) {
        let a = self.node(r, c.saturating_sub(1));
        let b = self.node(r, (c + 1).min(self.cols - 1));
        ((b.x - a.x) / 2.0, (b.y - a.y) / 2.0)
    }

    fn tangent_v(&self, r: usize, c: usize) -> (f64, f64) {
        let a = self.node(r.saturating_sub(1), c);
        let b = self.node((r + 1).min(self.rows - 1), c);
        ((b.x - a.x) / 2.0, (b.y - a.y) / 2.0)
    }

    fn hermite(p0: f64, p1: f64, m0: f64, m1: f64, t: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1
    }

    /// Evaluate patch (r, c) at (s, t) in 0..=1. Returns position + color.
    /// Geometry is tensor-product Hermite (smooth); color is bilinear
    /// (seamless across shared nodes).
    pub fn evaluate(&self, r: usize, c: usize, s: f64, t: f64) -> (AnchorPoint, [f32; 4]) {
        let r = r.min(self.rows - 2);
        let c = c.min(self.cols - 2);
        let s = s.clamp(0.0, 1.0);
        let t = t.clamp(0.0, 1.0);
        let p = |rr: usize, cc: usize| self.node(rr, cc);
        // Rows interpolated along u first.
        let row = |rr: usize| {
            let a = p(rr, c);
            let b = p(rr, c + 1);
            let (mux0, muy0) = self.tangent_u(rr, c);
            let (mux1, muy1) = self.tangent_u(rr, c + 1);
            (
                Self::hermite(a.x, b.x, mux0, mux1, s),
                Self::hermite(a.y, b.y, muy0, muy1, s),
            )
        };
        let (q0x, q0y) = row(r);
        let (q1x, q1y) = row(r + 1);
        // v-tangents lerped along u (twists ignored: visually negligible).
        let lerp_tan = |rr: usize| {
            let (ax, ay) = self.tangent_v(rr, c);
            let (bx, by) = self.tangent_v(rr, c + 1);
            (ax + (bx - ax) * s, ay + (by - ay) * s)
        };
        let (m0x, m0y) = lerp_tan(r);
        let (m1x, m1y) = lerp_tan(r + 1);
        let x = Self::hermite(q0x, q1x, m0x, m1x, t);
        let y = Self::hermite(q0y, q1y, m0y, m1y, t);
        // Bilinear color.
        let cs = [p(r, c).color, p(r, c + 1).color, p(r + 1, c).color, p(r + 1, c + 1).color];
        let mut color = [0.0; 4];
        for i in 0..4 {
            let top = cs[0][i] + (cs[1][i] - cs[0][i]) * s as f32;
            let bot = cs[2][i] + (cs[3][i] - cs[2][i]) * s as f32;
            color[i] = top + (bot - top) * t as f32;
        }
        (AnchorPoint::new(x, y), color)
    }

    /// Dense sample grid over the whole mesh: `(subdiv+1)` vertices per
    /// patch side plus the row stride, for triangulators and exporters.
    pub fn sample_grid(&self, subdiv: usize) -> (Vec<(AnchorPoint, [f32; 4])>, usize) {
        let subdiv = subdiv.clamp(1, MAX_MESH_SUBDIV);
        let gw = (self.cols - 1) * subdiv + 1;
        let gh = (self.rows - 1) * subdiv + 1;
        let mut verts = Vec::with_capacity(gw * gh);
        for gr in 0..gh {
            let pr = (gr / subdiv).min(self.rows - 2);
            let t = (gr % subdiv) as f64 / subdiv as f64;
            for gc in 0..gw {
                let pc = (gc / subdiv).min(self.cols - 2);
                let s = (gc % subdiv) as f64 / subdiv as f64;
                verts.push(self.evaluate(pr, pc, s, t));
            }
        }
        (verts, gw)
    }

    /// Triangle soup (local coords, per-vertex color) for canvas meshes.
    pub fn triangulate(&self, subdiv: usize) -> Vec<([AnchorPoint; 3], [[f32; 4]; 3])> {
        let (verts, gw) = self.sample_grid(subdiv);
        let gh = verts.len() / gw;
        let mut tris = Vec::new();
        for r in 0..gh.saturating_sub(1) {
            for c in 0..gw.saturating_sub(1) {
                let i00 = r * gw + c;
                let i10 = i00 + 1;
                let i01 = i00 + gw;
                let i11 = i01 + 1;
                tris.push((
                    [verts[i00].0, verts[i10].0, verts[i11].0],
                    [verts[i00].1, verts[i10].1, verts[i11].1],
                ));
                tris.push((
                    [verts[i00].0, verts[i11].0, verts[i01].0],
                    [verts[i00].1, verts[i11].1, verts[i01].1],
                ));
            }
        }
        tris
    }

    /// Flat quads (corners + center color) for SVG/PDF exporters, which
    /// cannot express vertex colors.
    pub fn quads(&self, subdiv: usize) -> Vec<([AnchorPoint; 4], [f32; 4])> {
        let (verts, gw) = self.sample_grid(subdiv);
        let gh = verts.len() / gw;
        let mut out = Vec::new();
        for r in 0..gh.saturating_sub(1) {
            for c in 0..gw.saturating_sub(1) {
                let i00 = r * gw + c;
                let i10 = i00 + 1;
                let i01 = i00 + gw;
                let i11 = i01 + 1;
                let mut col = [0.0; 4];
                for (c4, slot) in col.iter_mut().enumerate() {
                    *slot =
                        (verts[i00].1[c4] + verts[i10].1[c4] + verts[i01].1[c4] + verts[i11].1[c4])
                            / 4.0;
                }
                out.push((
                    [verts[i00].0, verts[i10].0, verts[i11].0, verts[i01].0],
                    col,
                ));
            }
        }
        out
    }

    /// Node bounding box (selection, hit-testing).
    pub fn node_bbox(&self) -> Option<(AnchorPoint, AnchorPoint)> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for n in &self.nodes {
            min_x = min_x.min(n.x);
            min_y = min_y.min(n.y);
            max_x = max_x.max(n.x);
            max_y = max_y.max(n.y);
        }
        Some((AnchorPoint::new(min_x, min_y), AnchorPoint::new(max_x, max_y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_reproduce_inputs() {
        let m = MeshGradient::new_rect(
            0.0, 0.0, 100.0, 50.0, 3, 3,
            [[1.0, 0.0, 0.0, 1.0]; 4],
        );
        assert_eq!((m.rows, m.cols), (3, 3));
        // Patch corners evaluate exactly to their nodes (Hermite property).
        let (p00, _) = m.evaluate(0, 0, 0.0, 0.0);
        assert!((p00.x).abs() < 1e-9 && (p00.y).abs() < 1e-9);
        let (p11, _) = m.evaluate(1, 1, 1.0, 1.0);
        assert!((p11.x - 100.0).abs() < 1e-9 && (p11.y - 50.0).abs() < 1e-9);
        // Shared edges agree from both sides (no cracks).
        let (a, _) = m.evaluate(0, 0, 1.0, 0.5);
        let (b, _) = m.evaluate(0, 1, 0.0, 0.5);
        assert!((a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9);
    }

    #[test]
    fn grid_counts_match_subdiv() {
        let m = MeshGradient::new_rect(0.0, 0.0, 10.0, 10.0, 2, 2, [[0.0, 0.0, 0.0, 1.0]; 4]);
        let (verts, stride) = m.sample_grid(4);
        assert_eq!((verts.len(), stride), (25, 5));
        assert_eq!(m.triangulate(4).len(), 4 * 4 * 2);
        assert_eq!(m.quads(4).len(), 16);
    }
}
