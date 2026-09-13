use super::CanvasWidget;
use crate::core::document::{Object, ObjectType};
use crate::core::path::{
    FillStyle, FillType, LinearGradient, PathData, PatternFill, RadialGradient,
};
use crate::core::state::AppState;
use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};

impl CanvasWidget {
    pub(super) fn draw_object(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        origin: Pos2,
        state: &AppState,
    ) {
        let opacity = obj.opacity.clamp(0.0_f32, 1.0_f32);
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = obj.transform.transform_point(wx, wy);
            Pos2::new(
                origin.x + sx as f32 * state.zoom,
                origin.y + sy as f32 * state.zoom,
            )
        };

        let fill_color = obj.fill.as_ref().and_then(|f| fill_type_color(f, opacity));

        let stroke_info = obj.stroke.as_ref().map(|s| {
            let c = s.color;
            let stroke_c = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0_f32) as u8,
                (c[1] * 255.0_f32) as u8,
                (c[2] * 255.0_f32) as u8,
                ((c[3] * opacity) * 255.0_f32) as u8,
            );
            Stroke::new((s.width as f32 * state.zoom).max(1.0_f32), stroke_c)
        });

        if let Some(ref sh) = obj.shadow {
            let sh_c = Color32::from_rgba_unmultiplied(
                (sh.color[0] * 255.0_f32) as u8,
                (sh.color[1] * 255.0_f32) as u8,
                (sh.color[2] * 255.0_f32) as u8,
                ((sh.color[3] * sh.opacity * opacity) * 255.0_f32) as u8,
            );
            let sh_to_screen = |lx: f64, ly: f64| -> Pos2 {
                let (wx, wy) = obj
                    .transform
                    .transform_point(lx + sh.offset_x, ly + sh.offset_y);
                Pos2::new(
                    origin.x + (wx as f32 * state.zoom),
                    origin.y + (wy as f32 * state.zoom),
                )
            };
            let poly = obj.to_path_data().to_polygon(16);
            if poly.len() >= 3 {
                let sh_pts: Vec<Pos2> = poly.iter().map(|p| sh_to_screen(p.x, p.y)).collect();
                painter.add(egui::epaint::PathShape::convex_polygon(
                    sh_pts,
                    sh_c,
                    Stroke::NONE,
                ));
            }
        }

        if let Some(ref gl) = obj.glow {
            let base_c = gl.color;
            let poly = obj.to_path_data().to_polygon(16);
            if poly.len() >= 3 {
                // Multi-tiered outer bloom
                for tier in (1..=4).rev() {
                    let spread = (gl.radius as f32 * (tier as f32 / 4.0)) * state.zoom;
                    let tier_alpha =
                        (base_c[3] * gl.intensity * opacity * (0.15 / tier as f32) * 255.0) as u8;
                    let tier_color = Color32::from_rgba_unmultiplied(
                        (base_c[0] * 255.0) as u8,
                        (base_c[1] * 255.0) as u8,
                        (base_c[2] * 255.0) as u8,
                        tier_alpha,
                    );
                    let gl_pts: Vec<Pos2> = poly.iter().map(|p| to_screen(p.x, p.y)).collect();
                    painter.add(egui::epaint::PathShape::closed_line(
                        gl_pts,
                        Stroke::new(spread * 2.0, tier_color),
                    ));
                }
            }
        }

        match &obj.object_type {
            ObjectType::Path(path) => {
                let subpaths = path.to_subpaths(16);
                let triangles = path.to_triangles(16);
                if !triangles.is_empty() {
                    if let Some(fill) = fill_color {
                        for tri in &triangles {
                            let p0 = to_screen(tri[0].x, tri[0].y);
                            let p1 = to_screen(tri[1].x, tri[1].y);
                            let p2 = to_screen(tri[2].x, tri[2].y);
                            painter.add(egui::epaint::PathShape::convex_polygon(
                                vec![p0, p1, p2],
                                fill,
                                Stroke::NONE,
                            ));
                        }
                    }
                }
                if let Some(stroke) = stroke_info {
                    for sp in subpaths {
                        if sp.len() >= 2 {
                            let screen_pts: Vec<Pos2> =
                                sp.iter().map(|p| to_screen(p.x, p.y)).collect();
                            painter.add(egui::epaint::PathShape::line(screen_pts, stroke));
                        }
                    }
                }
            }
            ObjectType::Rectangle {
                width,
                height,
                corner_radius,
            } => {
                let path = PathData::from_rect(0.0, 0.0, *width, *height, *corner_radius);
                let screen_pts: Vec<Pos2> = path
                    .to_polygon(12)
                    .iter()
                    .map(|p| to_screen(p.x, p.y))
                    .collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(
                        screen_pts.clone(),
                        fill,
                        Stroke::NONE,
                    ));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Ellipse { rx, ry } => {
                let path = PathData::from_ellipse(0.0, 0.0, *rx, *ry);
                let screen_pts: Vec<Pos2> = path
                    .to_polygon(24)
                    .iter()
                    .map(|p| to_screen(p.x, p.y))
                    .collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(
                        screen_pts.clone(),
                        fill,
                        Stroke::NONE,
                    ));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Star {
                points,
                inner_radius,
                outer_radius,
            } => {
                let path = PathData::from_star(*points, *inner_radius, *outer_radius, 0.0, 0.0);
                let screen_pts: Vec<Pos2> = path
                    .to_polygon(1)
                    .iter()
                    .map(|p| to_screen(p.x, p.y))
                    .collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(
                        screen_pts.clone(),
                        fill,
                        Stroke::NONE,
                    ));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Polygon { sides, radius } => {
                let path = PathData::from_polygon(*sides, *radius, 0.0, 0.0);
                let screen_pts: Vec<Pos2> = path
                    .to_polygon(1)
                    .iter()
                    .map(|p| to_screen(p.x, p.y))
                    .collect();
                if let Some(fill) = fill_color {
                    painter.add(egui::epaint::PathShape::convex_polygon(
                        screen_pts.clone(),
                        fill,
                        Stroke::NONE,
                    ));
                }
                if let Some(stroke) = stroke_info {
                    painter.add(egui::epaint::PathShape::closed_line(screen_pts, stroke));
                }
            }
            ObjectType::Line { x2, y2 } => {
                let p1 = to_screen(0.0, 0.0);
                let p2 = to_screen(*x2, *y2);
                let stroke = stroke_info.unwrap_or_else(|| Stroke::new(2.0_f32, Color32::BLACK));
                painter.line_segment([p1, p2], stroke);
            }
            ObjectType::Text { text, font_size } => {
                let pos = to_screen(0.0, 0.0);
                let font_id = FontId::proportional((font_size * state.zoom as f64) as f32);
                let color = fill_color.unwrap_or(Color32::BLACK);
                painter.text(pos, egui::Align2::LEFT_BOTTOM, text, font_id, color);
            }
            ObjectType::Group(children) => {
                for child in children {
                    self.draw_object(painter, child, origin, state);
                }
            }
            ObjectType::ClippingMask { children } => {
                for child in children {
                    self.draw_object(painter, child, origin, state);
                }
            }
            ObjectType::Use { href, .. } => {
                let symbol_id = href.trim_start_matches('#');
                if let Some(sym) = state.document.symbol_by_id(symbol_id) {
                    let mut instance_obj = sym.object.clone();
                    instance_obj.transform.x += obj.transform.x;
                    instance_obj.transform.y += obj.transform.y;
                    self.draw_object(painter, &instance_obj, origin, state);
                }
            }
        }

        if let Some(ref fill) = obj.fill {
            match &fill.fill_type {
                FillType::Linear(grad) => {
                    self.draw_linear_gradient(painter, obj, grad, opacity, origin, state);
                }
                FillType::Radial(grad) => {
                    self.draw_radial_gradient(painter, obj, grad, opacity, origin, state);
                }
                FillType::Pattern(pat) => {
                    self.draw_pattern_fill(painter, obj, pat, opacity, origin, state);
                }
                FillType::Solid(_) => {}
            }
        }
    }

    pub(super) fn draw_linear_gradient(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        grad: &LinearGradient,
        opacity: f32,
        origin: Pos2,
        state: &AppState,
    ) {
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = obj.transform.transform_point(wx, wy);
            Pos2::new(
                origin.x + sx as f32 * state.zoom,
                origin.y + sy as f32 * state.zoom,
            )
        };

        let path = obj.to_path_data();
        let poly = path.to_polygon(16);
        if poly.len() < 3 {
            return;
        }
        let screen_pts: Vec<Pos2> = poly.iter().map(|p| to_screen(p.x, p.y)).collect();

        let dx = grad.end_x - grad.start_x;
        let dy = grad.end_y - grad.start_y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            return;
        }

        let num_bands = 32;
        let min_y = screen_pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = screen_pts
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);
        let band_height = (max_y - min_y) / num_bands as f32;

        for i in 0..num_bands {
            let y_top = min_y + i as f32 * band_height;
            let y_bot = y_top + band_height;
            let y_mid = (y_top + y_bot) / 2.0;

            let t = ((y_mid - min_y) / (max_y - min_y)).clamp(0.0, 1.0);
            let c = sample_gradient_stops(&grad.stops, t);
            let a = c[3] * opacity;
            if a <= 0.0 {
                continue;
            }
            let band_color = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (a * 255.0) as u8,
            );

            let clipped = clip_polygon_to_y_band(&screen_pts, y_top, y_bot);
            if clipped.len() >= 3 {
                painter.add(egui::epaint::PathShape::convex_polygon(
                    clipped,
                    band_color,
                    Stroke::NONE,
                ));
            }
        }

        let stroke_info = obj.stroke.as_ref().map(|s| {
            let c = s.color;
            let stroke_c = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                ((c[3] * opacity) * 255.0) as u8,
            );
            Stroke::new((s.width as f32 * state.zoom).max(1.0_f32), stroke_c)
        });
        if let Some(stroke) = stroke_info {
            painter.add(egui::epaint::PathShape::line(screen_pts, stroke));
        }
    }

    pub(super) fn draw_radial_gradient(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        grad: &RadialGradient,
        opacity: f32,
        origin: Pos2,
        state: &AppState,
    ) {
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = obj.transform.transform_point(wx, wy);
            Pos2::new(
                origin.x + sx as f32 * state.zoom,
                origin.y + sy as f32 * state.zoom,
            )
        };

        let path = obj.to_path_data();
        let poly = path.to_polygon(16);
        if poly.len() < 3 {
            return;
        }
        let screen_pts: Vec<Pos2> = poly.iter().map(|p| to_screen(p.x, p.y)).collect();

        let min_x = screen_pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = screen_pts
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = screen_pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = screen_pts
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);

        let cx = (min_x + max_x) / 2.0;
        let cy = (min_y + max_y) / 2.0;
        let max_r = ((max_x - min_x).max(max_y - min_y)) / 2.0;
        if max_r <= 0.0 {
            return;
        }

        let num_rings = 24;
        for i in (0..num_rings).rev() {
            let t = (i as f32) / (num_rings as f32);
            let inner_r = t * max_r;
            let outer_r = ((i + 1) as f32) / (num_rings as f32) * max_r;

            let c = sample_gradient_stops(&grad.stops, t);
            let a = c[3] * opacity;
            if a <= 0.0 {
                continue;
            }
            let ring_color = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (a * 255.0) as u8,
            );

            let segments = 32;
            let mut ring_pts: Vec<Pos2> = Vec::new();
            for j in 0..=segments {
                let angle = (j as f32 / segments as f32) * std::f32::consts::TAU;
                ring_pts.push(Pos2::new(
                    cx + outer_r * angle.cos(),
                    cy + outer_r * angle.sin(),
                ));
            }
            for j in (0..=segments).rev() {
                let angle = (j as f32 / segments as f32) * std::f32::consts::TAU;
                ring_pts.push(Pos2::new(
                    cx + inner_r * angle.cos(),
                    cy + inner_r * angle.sin(),
                ));
            }

            painter.add(egui::epaint::PathShape::convex_polygon(
                ring_pts,
                ring_color,
                Stroke::NONE,
            ));
        }

        let stroke_info = obj.stroke.as_ref().map(|s| {
            let c = s.color;
            let stroke_c = Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                ((c[3] * opacity) * 255.0) as u8,
            );
            Stroke::new((s.width as f32 * state.zoom).max(1.0_f32), stroke_c)
        });
        if let Some(stroke) = stroke_info {
            painter.add(egui::epaint::PathShape::line(screen_pts, stroke));
        }
    }

    pub(super) fn draw_pattern_fill(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        pat: &PatternFill,
        opacity: f32,
        origin: Pos2,
        state: &AppState,
    ) {
        if let Some((bb_min, bb_max)) = obj.bounding_box() {
            let tile_w = (pat.tile_width * pat.scale) as f32 * state.zoom;
            let tile_h = (pat.tile_height * pat.scale) as f32 * state.zoom;
            if tile_w < 2.0 || tile_h < 2.0 {
                return;
            }

            let base_color = obj
                .fill
                .as_ref()
                .map(|f| f.color)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let a = (base_color[3] * opacity * 255.0) as u8;
            let tile_color = Color32::from_rgba_unmultiplied(
                (base_color[0] * 255.0) as u8,
                (base_color[1] * 255.0) as u8,
                (base_color[2] * 255.0) as u8,
                a,
            );

            let sx = origin.x + bb_min.x as f32 * state.zoom + (pat.offset_x as f32 * state.zoom);
            let sy = origin.y + bb_min.y as f32 * state.zoom + (pat.offset_y as f32 * state.zoom);
            let ex = origin.x + bb_max.x as f32 * state.zoom;
            let ey = origin.y + bb_max.y as f32 * state.zoom;

            let mut y = sy;
            while y < ey {
                let mut x = sx;
                while x < ex {
                    let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(tile_w, tile_h));
                    painter.rect_filled(r, 0.0, tile_color);
                    x += tile_w;
                }
                y += tile_h;
            }
        }
    }
}

pub fn sample_gradient_stops(stops: &[crate::core::path::GradientStop], t: f32) -> [f32; 4] {
    if stops.is_empty() {
        return [0.0, 0.0, 0.0, 1.0];
    }
    if stops.len() == 1 {
        return stops[0].color;
    }
    let t = t.clamp(0.0, 1.0);
    for w in stops.windows(2) {
        if t >= w[0].offset && t <= w[1].offset {
            let span = w[1].offset - w[0].offset;
            let local_t = if span > 0.0 {
                (t - w[0].offset) / span
            } else {
                0.0
            };
            return [
                w[0].color[0] + (w[1].color[0] - w[0].color[0]) * local_t,
                w[0].color[1] + (w[1].color[1] - w[0].color[1]) * local_t,
                w[0].color[2] + (w[1].color[2] - w[0].color[2]) * local_t,
                w[0].color[3] + (w[1].color[3] - w[0].color[3]) * local_t,
            ];
        }
    }
    stops
        .last()
        .map(|s| s.color)
        .unwrap_or([0.0, 0.0, 0.0, 1.0])
}

fn fill_type_color(fill: &FillStyle, opacity: f32) -> Option<Color32> {
    let c = match &fill.fill_type {
        FillType::Solid(color) => *color,
        FillType::Linear(_) | FillType::Radial(_) | FillType::Pattern(_) => return None,
    };
    let a = c[3] * opacity;
    if a <= 0.0 {
        return None;
    }
    Some(Color32::from_rgba_unmultiplied(
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
        (a * 255.0) as u8,
    ))
}

fn clip_polygon_to_y_band(poly: &[Pos2], y_top: f32, y_bot: f32) -> Vec<Pos2> {
    if poly.len() < 3 {
        return poly.to_vec();
    }
    let mut result = Vec::new();
    let n = poly.len();
    for i in 0..n {
        let curr = poly[i];
        let next = poly[(i + 1) % n];
        let curr_in = curr.y >= y_top && curr.y <= y_bot;
        let next_in = next.y >= y_top && next.y <= y_bot;

        if curr_in && next_in {
            result.push(curr);
        } else if curr_in && !next_in {
            result.push(curr);
            if (next.y - curr.y).abs() > f32::EPSILON {
                let t = if next.y > curr.y {
                    (y_bot - curr.y) / (next.y - curr.y)
                } else {
                    (y_top - curr.y) / (next.y - curr.y)
                };
                let t = t.clamp(0.0, 1.0);
                result.push(Pos2::new(
                    curr.x + t * (next.x - curr.x),
                    curr.y + t * (next.y - curr.y),
                ));
            }
        } else if !curr_in && next_in && (next.y - curr.y).abs() > f32::EPSILON {
            let t = if next.y > curr.y {
                (y_top - curr.y) / (next.y - curr.y)
            } else {
                (y_bot - curr.y) / (next.y - curr.y)
            };
            let t = t.clamp(0.0, 1.0);
            result.push(Pos2::new(
                curr.x + t * (next.x - curr.x),
                curr.y + t * (next.y - curr.y),
            ));
        }
    }
    result
}
