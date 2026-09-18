use super::CanvasWidget;
use crate::core::document::{Object, ObjectType};
use crate::core::path::{
    FillStyle, FillType, LinearGradient, PathData, PatternFill, RadialGradient,
};
use crate::core::state::AppState;
use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};

fn affine_mul(m1: &[f64; 6], m2: &[f64; 6]) -> [f64; 6] {
    [
        m1[0] * m2[0] + m1[2] * m2[1],
        m1[1] * m2[0] + m1[3] * m2[1],
        m1[0] * m2[2] + m1[2] * m2[3],
        m1[1] * m2[2] + m1[3] * m2[3],
        m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
        m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
    ]
}

fn affine_apply(m: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

pub const IDENTITY_AFFINE: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

impl CanvasWidget {
    pub(super) fn draw_object(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        origin: Pos2,
        state: &AppState,
        parent: &[f64; 6],
    ) {
        let opacity = obj.opacity.clamp(0.0_f32, 1.0_f32);
        // Compose ancestor (group) transforms: previously group transforms
        // were silently ignored, so moved/rotated groups rendered stale
        // while hit-testing and export used the new positions.
        let composed = affine_mul(parent, &obj.transform.matrix());
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = affine_apply(&composed, wx, wy);
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
                let (wx, wy) = affine_apply(
                    &composed,
                    lx + sh.offset_x,
                    ly + sh.offset_y,
                );
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
            ObjectType::Text {
                text,
                font_size,
                style,
            } => {
                let pos = to_screen(0.0, 0.0);
                let scaled_size = (font_size * state.zoom as f64) as f32;

                let align = match style.text_anchor {
                    crate::core::document::TextAnchor::Start => egui::Align2::LEFT_BOTTOM,
                    crate::core::document::TextAnchor::Middle => egui::Align2::CENTER_BOTTOM,
                    crate::core::document::TextAnchor::End => egui::Align2::RIGHT_BOTTOM,
                };

                let registry = crate::core::font::FontRegistry::global();
                let is_avail = registry.is_any_family_available(&style.font_family);
                let font_family = if style.font_family.to_lowercase().contains("mono") {
                    egui::FontFamily::Monospace
                } else {
                    egui::FontFamily::Proportional
                };

                let font_id = FontId::new(scaled_size, font_family);
                let color = fill_color.unwrap_or(Color32::BLACK);
                // Faux-bold approximation so canvas reflects font-weight instead of
                // silently rendering everything as regular.
                let bold = style.font_weight >= 650;
                let bold_dx = (scaled_size * 0.035).clamp(0.5, 1.5);
                // Explicit line breaks: one baseline per line, advancing by
                // the same 1.2em factor the SVG exporter uses for <tspan dy>.
                let line_height = scaled_size * 1.2;
                let lines: Vec<&str> = text.split('\n').collect();
                let mut widest: f32 = 0.0;
                for (li, line) in lines.iter().enumerate() {
                    let line_pos =
                        egui::pos2(pos.x, pos.y + li as f32 * line_height);
                    if style.letter_spacing != 0.0 {
                        let letter_space_screen =
                            (style.letter_spacing * state.zoom as f64) as f32;
                        // Pre-measure so text-anchor (middle/end) applies to the whole run,
                        // matching exported SVG behavior.
                        let mut widths: Vec<(String, f32, f32)> = Vec::new();
                        let mut total_w = 0.0;
                        for ch in line.chars() {
                            let ch_str = ch.to_string();
                            let galley =
                                painter.layout_no_wrap(ch_str.clone(), font_id.clone(), color);
                            let w = galley.size().x;
                            let h = galley.size().y;
                            widths.push((ch_str, w, h));
                            total_w += w;
                        }
                        if !widths.is_empty() {
                            total_w += letter_space_screen * (widths.len() as f32 - 1.0);
                        }
                        widest = widest.max(total_w);
                        let mut curr_x = match style.text_anchor {
                            crate::core::document::TextAnchor::Start => line_pos.x,
                            crate::core::document::TextAnchor::Middle => {
                                line_pos.x - total_w / 2.0
                            }
                            crate::core::document::TextAnchor::End => line_pos.x - total_w,
                        };
                        for (ch_str, w, h) in &widths {
                            // painter::galley positions from the top-left while `pos`
                            // is the text baseline; align bottoms explicitly.
                            let char_pos = egui::pos2(curr_x, line_pos.y - *h);
                            let galley =
                                painter.layout_no_wrap(ch_str.clone(), font_id.clone(), color);
                            painter.galley(char_pos, galley, color);
                            if bold {
                                let g2 = painter.layout_no_wrap(
                                    ch_str.clone(),
                                    font_id.clone(),
                                    color,
                                );
                                painter.galley(
                                    egui::pos2(curr_x + bold_dx, line_pos.y - *h),
                                    g2,
                                    color,
                                );
                            }
                            curr_x += *w + letter_space_screen;
                        }
                    } else {
                        let galley =
                            painter.layout_no_wrap(line.to_string(), font_id.clone(), color);
                        widest = widest.max(galley.size().x);
                        painter.text(line_pos, align, *line, font_id.clone(), color);
                        if bold {
                            painter.text(
                                egui::pos2(line_pos.x + bold_dx, line_pos.y),
                                align,
                                *line,
                                font_id.clone(),
                                color,
                            );
                        }
                    }
                }

                if !is_avail && state.selected_ids.contains(&obj.id) {
                    let text_h = line_height * lines.len() as f32;
                    let text_rect = egui::Rect::from_min_size(
                        egui::pos2(pos.x, pos.y - scaled_size),
                        egui::vec2(widest.max(scaled_size * 0.6), text_h),
                    );
                    painter.rect_stroke(
                        text_rect,
                        0.0,
                        egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 140, 0)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            ObjectType::Group(children) => {
                for child in children {
                    self.draw_object(painter, child, origin, state, &composed);
                }
            }
            ObjectType::ClippingMask { children } => {
                for child in children {
                    self.draw_object(painter, child, origin, state, &composed);
                }
            }
            ObjectType::Use { href, .. } => {
                let symbol_id = href.trim_start_matches('#');
                if let Some(sym) = state.document.symbol_by_id(symbol_id) {
                    let mut instance_obj = sym.object.clone();
                    instance_obj.transform.x += obj.transform.x;
                    instance_obj.transform.y += obj.transform.y;
                    self.draw_object(painter, &instance_obj, origin, state, parent);
                }
            }
        }

        if let Some(ref fill) = obj.fill {
            match &fill.fill_type {
                FillType::Linear(grad) => {
                    self.draw_linear_gradient(painter, obj, grad, opacity, origin, state, parent);
                }
                FillType::Radial(grad) => {
                    self.draw_radial_gradient(painter, obj, grad, opacity, origin, state, parent);
                }
                FillType::Pattern(pat) => {
                    self.draw_pattern_fill(painter, obj, pat, opacity, origin, state, parent);
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
        parent: &[f64; 6],
    ) {
        let composed = affine_mul(parent, &obj.transform.matrix());
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = affine_apply(&composed, wx, wy);
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
        let bw = max_x - min_x;
        let bh = max_y - min_y;
        if bw <= 0.0 || bh <= 0.0 {
            return;
        }

        // Gradient endpoints live in normalized bbox space
        // (SVG objectBoundingBox convention), so map them onto the
        // on-screen bbox. The previous code sampled purely by vertical
        // position, rendering every gradient (including the diagonal
        // default) as vertical.
        let sx0 = min_x + grad.start_x * bw;
        let sy0 = min_y + grad.start_y * bh;
        let ex = min_x + grad.end_x * bw;
        let ey = min_y + grad.end_y * bh;
        let dx = ex - sx0;
        let dy = ey - sy0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.5 {
            // Degenerate direction: flat fill with the first stop.
            let c = sample_gradient_stops(&grad.stops, 0.0);
            let a = c[3] * opacity;
            if a > 0.0 {
                painter.add(egui::epaint::PathShape::convex_polygon(
                    screen_pts.clone(),
                    Color32::from_rgba_unmultiplied(
                        (c[0] * 255.0) as u8,
                        (c[1] * 255.0) as u8,
                        (c[2] * 255.0) as u8,
                        (a * 255.0) as u8,
                    ),
                    Stroke::NONE,
                ));
            }
        } else {
            let ux = dx / len;
            let uy = dy / len;
            let proj = |p: Pos2| (p.x - sx0) * ux + (p.y - sy0) * uy;
            let s_min = screen_pts.iter().map(|p| proj(*p)).fold(f32::INFINITY, f32::min);
            let s_max = screen_pts
                .iter()
                .map(|p| proj(*p))
                .fold(f32::NEG_INFINITY, f32::max);
            let span = s_max - s_min;
            if span > 0.0 {
                // Adaptive band count (~3px per band) plus deterministic
                // ±1 LSB dithering to break up visible banding steps.
                let num_bands = ((span / 3.0).round() as usize).clamp(24, 96);
                let band_w = span / num_bands as f32;
                for i in 0..num_bands {
                    let s_top = s_min + i as f32 * band_w;
                    let s_bot = s_top + band_w;
                    let s_mid = (s_top + s_bot) / 2.0;

                    let t = (s_mid / len).clamp(0.0, 1.0);
                    let c = sample_gradient_stops(&grad.stops, t);
                    let a = c[3] * opacity;
                    if a <= 0.0 {
                        continue;
                    }
                    let dith = band_dither(i as u32);
                    let band_color = Color32::from_rgba_unmultiplied(
                        quantize_channel(c[0], dith),
                        quantize_channel(c[1], dith),
                        quantize_channel(c[2], dith),
                        (a * 255.0).round().clamp(0.0, 255.0) as u8,
                    );

                    let clipped =
                        clip_polygon_to_s_band(&screen_pts, sx0, sy0, ux, uy, s_top, s_bot);
                    if clipped.len() >= 3 {
                        painter.add(egui::epaint::PathShape::convex_polygon(
                            clipped,
                            band_color,
                            Stroke::NONE,
                        ));
                    }
                }
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
        parent: &[f64; 6],
    ) {
        let composed = affine_mul(parent, &obj.transform.matrix());
        let to_screen = |wx: f64, wy: f64| -> Pos2 {
            let (sx, sy) = affine_apply(&composed, wx, wy);
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

        // Gradient geometry in normalized bbox space (SVG
        // objectBoundingBox convention): centre, elliptical radii and the
        // focal point the rings are centred on. The previous code ignored
        // all three — always drawing circles around the bbox middle.
        let bw = max_x - min_x;
        let bh = max_y - min_y;
        if bw <= 0.0 || bh <= 0.0 {
            return;
        }
        let fx = min_x + grad.focus_x * bw;
        let fy = min_y + grad.focus_y * bh;
        let rx = grad.radius * bw;
        let ry = grad.radius * bh;
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }

        // Normalized extent over the silhouette; corners beyond r=1 reuse
        // the end stop (spreadMethod=pad, like export).
        let extent = screen_pts
            .iter()
            .map(|p| ellipse_norm_dist(p.x, p.y, fx, fy, rx, ry))
            .fold(0.0_f32, f32::max)
            .max(1e-3);
        let approx_px = extent * rx.max(ry);
        let num_rings = ((approx_px / 3.0).round() as usize).clamp(16, 64);
        for i in (0..num_rings).rev() {
            let t0 = (i as f32) / (num_rings as f32) * extent;
            let t1 = ((i + 1) as f32) / (num_rings as f32) * extent;
            let t_mid = (t0 + t1) / 2.0;

            let c = sample_gradient_stops(&grad.stops, (t_mid / extent).clamp(0.0, 1.0));
            let a = c[3] * opacity;
            if a <= 0.0 {
                continue;
            }
            let dith = band_dither(i as u32 * 2 + 1);
            let ring_color = Color32::from_rgba_unmultiplied(
                quantize_channel(c[0], dith),
                quantize_channel(c[1], dith),
                quantize_channel(c[2], dith),
                (a * 255.0).round().clamp(0.0, 255.0) as u8,
            );

            // Shape-clipped band: no disc overdraw outside the silhouette.
            let mut piece = clip_poly_to_ellipse_band(&screen_pts, fx, fy, rx, ry, t0, t1);
            if piece.len() < 3
                && t0 == 0.0
                && pos_in_polygon(fx, fy, &screen_pts)
            {
                // Innermost band around an interior focal point touches no
                // edge; without this the centre would stay transparent.
                // No band crossings were found, so the disc lies fully
                // inside the (simply-connected) silhouette.
                piece = ellipse_disc_poly(fx, fy, rx, ry, t1);
            }
            if piece.len() >= 3 {
                painter.add(egui::epaint::PathShape::convex_polygon(
                    piece,
                    ring_color,
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

    pub(super) fn draw_pattern_fill(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        pat: &PatternFill,
        opacity: f32,
        origin: Pos2,
        state: &AppState,
        parent: &[f64; 6],
    ) {
        // World-space bbox through the composed (ancestor-aware) transform.
        let composed = affine_mul(parent, &obj.transform.matrix());
        let wpoly: Vec<(f64, f64)> = obj
            .to_path_data()
            .to_polygon(16)
            .iter()
            .map(|p| affine_apply(&composed, p.x, p.y))
            .collect();
        let bb = if wpoly.is_empty() {
            None
        } else {
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;
            for (x, y) in &wpoly {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
            Some((
                crate::core::path::AnchorPoint::new(min_x, min_y),
                crate::core::path::AnchorPoint::new(max_x, max_y),
            ))
        };
        if let Some((bb_min, bb_max)) = bb {
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

#[cfg(test)]
mod gradient_clip_tests {
    use super::*;
    use egui::Pos2;

    fn poly_area(poly: &[Pos2]) -> f32 {
        if poly.len() < 3 {
            return 0.0;
        }
        let mut a = 0.0;
        for i in 0..poly.len() {
            let p = poly[i];
            let q = poly[(i + 1) % poly.len()];
            a += p.x * q.y - q.x * p.y;
        }
        (a * 0.5).abs()
    }

    fn square() -> Vec<Pos2> {
        vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(0.0, 10.0),
        ]
    }

    #[test]
    fn test_s_band_horizontal_split() {
        let sq = square();
        let left = clip_polygon_to_s_band(&sq, 0.0, 0.0, 1.0, 0.0, 0.0, 5.0);
        let right = clip_polygon_to_s_band(&sq, 0.0, 0.0, 1.0, 0.0, 5.0, 10.0);
        assert!((poly_area(&left) - 50.0).abs() < 1.0, "left half");
        assert!((poly_area(&right) - 50.0).abs() < 1.0, "right half");
    }

    #[test]
    fn test_s_band_diagonal_preserves_area() {
        let sq = square();
        let inv = std::f32::consts::FRAC_1_SQRT_2;
        let mut total = 0.0;
        let n = 8;
        for i in 0..n {
            let piece = clip_polygon_to_s_band(
                &sq,
                0.0,
                0.0,
                inv,
                inv,
                i as f32 * 20.0 / n as f32,
                (i + 1) as f32 * 20.0 / n as f32,
            );
            total += poly_area(&piece);
        }
        // Diagonal span of a 10x10 square is ~14.14; bands cover it fully.
        assert!((total - 100.0).abs() < 2.0, "total {total}");
    }

    #[test]
    fn test_ellipse_band_full_range_keeps_shape() {
        let sq = square();
        let full = clip_poly_to_ellipse_band(&sq, 5.0, 5.0, 5.0, 5.0, 0.0, 10.0);
        assert!(poly_area(&full) > 90.0, "full band keeps silhouette");
        // A small central band touches no edge of the square: correctly
        // empty from edge-walking, with the focal point inside the shape —
        // exactly the condition that triggers the disc fallback.
        let core = clip_poly_to_ellipse_band(&sq, 5.0, 5.0, 5.0, 5.0, 0.0, 0.5);
        assert!(core.len() < 3, "interior band has no edge piece");
        assert!(pos_in_polygon(5.0, 5.0, &sq));
        // A mid band crossing edges yields a real piece inside the bbox.
        let mid = clip_poly_to_ellipse_band(&sq, 5.0, 5.0, 5.0, 5.0, 0.9, 1.1);
        assert!(poly_area(&mid) > 1.0, "edge band keeps a piece");
    }

    #[test]
    fn test_ellipse_band_stays_inside_silhouette() {
        // Star-ish concave polygon: clipped pieces must not leak outside.
        let poly = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(5.0, 4.0),
            Pos2::new(0.0, 10.0),
        ];
        for i in 0..8 {
            let piece = clip_poly_to_ellipse_band(
                &poly, 5.0, 5.0, 6.0, 6.0, i as f32 * 0.25, (i + 1) as f32 * 0.25,
            );
            for p in &piece {
                // Inside bbox (silhouette test at vertex level).
                assert!(p.x >= -0.01 && p.x <= 10.01 && p.y >= -0.01 && p.y <= 10.01);
            }
        }
    }

    #[test]
    fn test_band_dither_bounded_and_stable() {
        for k in [0, 1, 7, 96, 1000] {
            let d = band_dither(k);
            assert!(d >= -1.0 && d <= 1.0);
            assert_eq!(d, band_dither(k));
        }
        assert_ne!(band_dither(3), band_dither(4));
    }
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

/// Deterministic ±1 LSB dither (in 0..255 units) keyed by band index.
/// Breaks up Mach-band steps without per-frame shimmer from RNG state.
fn band_dither(key: u32) -> f32 {
    let h = key
        .wrapping_mul(2654435761)
        .wrapping_add(40503)
        .wrapping_mul(2246822519);
    ((h >> 9) & 255) as f32 / 255.0 * 2.0 - 1.0
}

fn quantize_channel(v: f32, dither: f32) -> u8 {
    (v * 255.0 + dither).round().clamp(0.0, 255.0) as u8
}

/// Clip a polygon to the slab `s_top <= s(p) <= s_bot` where
/// `s(p) = (p - S) . u`. Used for gradient bands along an arbitrary
/// on-screen direction `(ux, uy)` through origin `(sx0, sy0)`.
fn clip_polygon_to_s_band(
    poly: &[Pos2],
    sx0: f32,
    sy0: f32,
    ux: f32,
    uy: f32,
    s_top: f32,
    s_bot: f32,
) -> Vec<Pos2> {
    if poly.len() < 3 {
        return poly.to_vec();
    }
    let s_of = |p: Pos2| (p.x - sx0) * ux + (p.y - sy0) * uy;
    let mut result = Vec::new();
    let n = poly.len();
    for i in 0..n {
        let curr = poly[i];
        let next = poly[(i + 1) % n];
        let sc = s_of(curr);
        let sn = s_of(next);
        let curr_in = sc >= s_top && sc <= s_bot;
        let next_in = sn >= s_top && sn <= s_bot;

        if curr_in && next_in {
            result.push(curr);
        } else if curr_in && !next_in {
            result.push(curr);
            if (sn - sc).abs() > f32::EPSILON {
                let bound = if sn > sc { s_bot } else { s_top };
                let t = ((bound - sc) / (sn - sc)).clamp(0.0, 1.0);
                result.push(Pos2::new(
                    curr.x + t * (next.x - curr.x),
                    curr.y + t * (next.y - curr.y),
                ));
            }
        } else if !curr_in && next_in && (sn - sc).abs() > f32::EPSILON {
            let bound = if sn > sc { s_top } else { s_bot };
            let t = ((bound - sc) / (sn - sc)).clamp(0.0, 1.0);
            result.push(Pos2::new(
                curr.x + t * (next.x - curr.x),
                curr.y + t * (next.y - curr.y),
            ));
        } else if (sn - sc).abs() > f32::EPSILON {
            // Both outside: the edge may still cut straight through the
            // slab (no vertex inside). Collect boundary crossings and keep
            // consecutive pairs whose midpoint lies in the slab.
            let mut xs: Vec<(f32, Pos2)> = Vec::new();
            for bound in [s_top, s_bot] {
                if (sc < bound && sn > bound) || (sc > bound && sn < bound) {
                    let t = ((bound - sc) / (sn - sc)).clamp(0.0, 1.0);
                    xs.push((
                        t,
                        Pos2::new(
                            curr.x + t * (next.x - curr.x),
                            curr.y + t * (next.y - curr.y),
                        ),
                    ));
                }
            }
            xs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut k = 0;
            while k + 1 < xs.len() {
                let mid_t = (xs[k].0 + xs[k + 1].0) / 2.0;
                let mid_p = Pos2::new(
                    curr.x + mid_t * (next.x - curr.x),
                    curr.y + mid_t * (next.y - curr.y),
                );
                let sm = s_of(mid_p);
                if sm >= s_top && sm <= s_bot {
                    result.push(xs[k].1);
                    result.push(xs[k + 1].1);
                }
                k += 2;
            }
        }
    }
    result
}

/// Ray-casting point-in-polygon for screen-space points.
fn pos_in_polygon(x: f32, y: f32, poly: &[Pos2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        if (a.y > y) != (b.y > y) {
            let xin = a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y);
            if x < xin {
                inside = !inside;
            }
        }
    }
    inside
}

/// Ellipse disc polygon (convex) at normalized radius `t`.
fn ellipse_disc_poly(cx: f32, cy: f32, rx: f32, ry: f32, t: f32) -> Vec<Pos2> {
    let n = 48;
    (0..n)
        .map(|j| {
            let a = (j as f32 / n as f32) * std::f32::consts::TAU;
            Pos2::new(cx + t * rx * a.cos(), cy + t * ry * a.sin())
        })
        .collect()
}

/// Normalized elliptical distance of `p` from center, in units of the
/// gradient radius (1.0 == on the gradient circle).
fn ellipse_norm_dist(px: f32, py: f32, cx: f32, cy: f32, rx: f32, ry: f32) -> f32 {
    (((px - cx) / rx).powi(2) + ((py - cy) / ry).powi(2)).sqrt()
}

/// Edge parameters `s in [0, 1]` where segment A->B crosses the ellipse of
/// normalized radius `r` around center. Solves the quadratic in `s`.
fn edge_ellipse_crossings(
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    r: f32,
) -> Vec<f32> {
    let dx = (bx - ax) / rx;
    let dy = (by - ay) / ry;
    let ex = (ax - cx) / rx;
    let ey = (ay - cy) / ry;
    let a = dx * dx + dy * dy;
    let b = 2.0 * (ex * dx + ey * dy);
    let c = ex * ex + ey * ey - r * r;
    if a.abs() < 1e-9 {
        return Vec::new();
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return Vec::new();
    }
    let sq = disc.sqrt();
    [(-b - sq) / (2.0 * a), (-b + sq) / (2.0 * a)]
        .into_iter()
        .filter(|s| *s >= 0.0 && *s <= 1.0)
        .collect()
}

/// Clip a polygon to the elliptical annulus `t0 <= t(p) <= t1`, where `t` is
/// the normalized elliptical distance from center. The result stays inside
/// the shape silhouette (unlike disc-overdraw), honouring center, radii and
/// the focal point the rings are centred on.
fn clip_poly_to_ellipse_band(
    poly: &[Pos2],
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    t0: f32,
    t1: f32,
) -> Vec<Pos2> {
    const EPS: f32 = 1e-4;
    if poly.len() < 3 {
        return Vec::new();
    }
    let t_of = |p: Pos2| ellipse_norm_dist(p.x, p.y, cx, cy, rx, ry);
    let lerp = |a: Pos2, b: Pos2, s: f32| {
        Pos2::new(a.x + s * (b.x - a.x), a.y + s * (b.y - a.y))
    };
    let mut out = Vec::new();
    let n = poly.len();
    for i in 0..n {
        let curr = poly[i];
        let next = poly[(i + 1) % n];
        let tc = t_of(curr);
        let tn = t_of(next);
        let cin = tc >= t0 - EPS && tc <= t1 + EPS;
        let nin = tn >= t0 - EPS && tn <= t1 + EPS;
        if cin && nin {
            out.push(curr);
        } else if cin && !nin {
            out.push(curr);
            // Exiting: cross whichever bound lies ahead.
            let target = if tn > tc { t1 } else { t0 };
            let mut xs = edge_ellipse_crossings(
                curr.x, curr.y, next.x, next.y, cx, cy, rx, ry, target,
            );
            xs.sort_by(|a, b| a.total_cmp(b));
            if let Some(&s) = xs.first() {
                out.push(lerp(curr, next, s));
            }
        } else if !cin && nin {
            let target = if tn > tc { t0 } else { t1 };
            let mut xs = edge_ellipse_crossings(
                curr.x, curr.y, next.x, next.y, cx, cy, rx, ry, target,
            );
            xs.sort_by(|a, b| a.total_cmp(b));
            if let Some(&s) = xs.last() {
                out.push(lerp(curr, next, s));
            }
        } else {
            // Both outside: the edge may still cut through the band.
            let mut xs = edge_ellipse_crossings(
                curr.x, curr.y, next.x, next.y, cx, cy, rx, ry, t0,
            );
            xs.extend(edge_ellipse_crossings(
                curr.x, curr.y, next.x, next.y, cx, cy, rx, ry, t1,
            ));
            xs.sort_by(|a, b| a.total_cmp(b));
            // Consecutive crossing pairs with an inside midpoint belong.
            let mut k = 0;
            while k + 1 < xs.len() {
                let mid = (xs[k] + xs[k + 1]) / 2.0;
                let tm = t_of(lerp(curr, next, mid));
                if tm >= t0 - EPS && tm <= t1 + EPS {
                    out.push(lerp(curr, next, xs[k]));
                    out.push(lerp(curr, next, xs[k + 1]));
                }
                k += 2;
            }
        }
    }
    out
}
