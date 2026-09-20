use super::CanvasWidget;
use crate::core::document::{BlendMode, Object, ObjectType};
use crate::core::path::{
    FillStyle, FillType, ImageFill, ImageTileMode, LinearGradient, PathData, PatternFill,
    RadialGradient,
};
use crate::core::state::AppState;
use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};

/// Approximate a blend mode by adjusting the source color against the
/// assumed background.  Only the most common vector blend modes are
/// handled; everything else falls back to Normal (opaque overlay).
fn apply_blend_approx(src: [f32; 4], bg: [f32; 4], mode: BlendMode) -> [f32; 4] {
    let sa = src[3];
    let ba = bg[3];
    let out_a = sa + ba * (1.0 - sa);
    if out_a <= 0.0 {
        return [0.0; 4];
    }
    let fn3 = |s: f32, b: f32| -> f32 { s * sa + b * ba * (1.0 - sa) / out_a };
    let sr = src[0];
    let sg = src[1];
    let sb = src[2];
    let br = bg[0];
    let bg_ = bg[1];
    let bb = bg[2];
    let (r, g, b) = match mode {
        BlendMode::Multiply => (sr * br, sg * bg_, sb * bb),
        BlendMode::Screen => (1.0 - (1.0 - sr) * (1.0 - br), 1.0 - (1.0 - sg) * (1.0 - bg_), 1.0 - (1.0 - sb) * (1.0 - bb)),
        BlendMode::Overlay => {
            let f = |s: f32, d: f32| -> f32 {
                if d < 0.5 { 2.0 * s * d } else { 1.0 - 2.0 * (1.0 - s) * (1.0 - d) }
            };
            (f(sr, br), f(sg, bg_), f(sb, bb))
        }
        BlendMode::Darken => (sr.min(br), sg.min(bg_), sb.min(bb)),
        BlendMode::Lighten => (sr.max(br), sg.max(bg_), sb.max(bb)),
        BlendMode::Difference => ((sr - br).abs(), (sg - bg_).abs(), (sb - bb).abs()),
        BlendMode::Exclusion => {
            (sr + br - 2.0 * sr * br, sg + bg_ - 2.0 * sg * bg_, sb + bb - 2.0 * sb * bb)
        }
        _ => (sr, sg, sb),
    };
    [fn3(r, br), fn3(g, bg_), fn3(b, bb), out_a]
}

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
        ancestor_opacity: f32,
    ) {
        let opacity = (obj.opacity * ancestor_opacity).clamp(0.0_f32, 1.0_f32);
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

        // Approximate non-Normal blend modes by blending the fill against
        // the white artboard background.  This is a preview-only heuristic;
        // SVG export uses the real CSS mix-blend-mode attribute.
        let fill_color = fill_color.map(|fc| {
            if obj.blend_mode != BlendMode::Normal {
                let bg = [1.0, 1.0, 1.0, 1.0]; // artboard white
                let blended = apply_blend_approx(
                    [fc.r() as f32 / 255.0, fc.g() as f32 / 255.0, fc.b() as f32 / 255.0, fc.a() as f32 / 255.0],
                    bg,
                    obj.blend_mode,
                );
                Color32::from_rgba_unmultiplied(
                    (blended[0] * 255.0) as u8,
                    (blended[1] * 255.0) as u8,
                    (blended[2] * 255.0) as u8,
                    (blended[3] * 255.0) as u8,
                )
            } else {
                fc
            }
        });

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
            let sh_c = sh.color;
            let sh_alpha = sh.opacity * opacity;
            let poly = obj.to_path_data().to_polygon(16);
            if poly.len() >= 3 {
                // Multi-layer shadow: draw several offset copies at
                // decreasing opacity and increasing spread to approximate
                // a Gaussian blur (similar to the glow code below).
                let layers = 5u32;
                for layer in 0..layers {
                    let t = layer as f32 / layers as f32;
                    let spread = sh.blur_radius as f32 * t * 0.5;
                    let layer_alpha = sh_alpha * (1.0 - t * 0.7);
                    let c = Color32::from_rgba_unmultiplied(
                        (sh_c[0] * 255.0) as u8,
                        (sh_c[1] * 255.0) as u8,
                        (sh_c[2] * 255.0) as u8,
                        (layer_alpha * 255.0) as u8,
                    );
                    let sh_to_screen = |lx: f64, ly: f64| -> Pos2 {
                        let (wx, wy) = affine_apply(
                            &composed,
                            lx + sh.offset_x * (1.0 + spread as f64 * 0.1),
                            ly + sh.offset_y * (1.0 + spread as f64 * 0.1),
                        );
                        Pos2::new(
                            origin.x + (wx as f32 * state.zoom),
                            origin.y + (wy as f32 * state.zoom),
                        )
                    };
                    let sh_pts: Vec<Pos2> = poly.iter().map(|p| sh_to_screen(p.x, p.y)).collect();
                    painter.add(egui::epaint::PathShape::convex_polygon(
                        sh_pts,
                        c,
                        Stroke::NONE,
                    ));
                }
            }
        }

        if let Some(ref gl) = obj.glow {
            let base_c = gl.color;
            let poly = obj.to_path_data().to_polygon(16);
            if poly.len() >= 3 {
                // Multi-tiered outer bloom with Gaussian-like alpha falloff
                for tier in (1..=6).rev() {
                    let tf = tier as f32 / 6.0;
                    let spread = (gl.radius as f32 * tf) * state.zoom;
                    // Gaussian-like falloff: alpha drops as exp(-x^2)
                    let tier_alpha = (base_c[3] * gl.intensity * opacity
                        * (0.25 * (-tf * tf * 2.0).exp())
                        * 255.0) as u8;
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
                    // Variable-width profile: stroke becomes a filled ribbon
                    // honouring per-position multipliers.  Uses the raw
                    // StrokeStyle (document units), not the zoomed egui stroke.
                    let ribbon_profile = obj
                        .width_profile
                        .as_ref()
                        .zip(obj.stroke.as_ref())
                        .filter(|(_, s)| s.width > 0.0);
                    for sp in &subpaths {
                        if sp.len() >= 2 {
                            if let Some((prof, stroke_style)) = ribbon_profile {
                                let ribbon = crate::core::offset::variable_width_outline(
                                    sp,
                                    prof,
                                    stroke_style.width,
                                    path.closed,
                                );
                                if ribbon.len() >= 3 {
                                    let screen_pts: Vec<Pos2> = ribbon
                                        .iter()
                                        .map(|p| to_screen(p.x, p.y))
                                        .collect();
                                    // Ribbon carries stroke color as fill.
                                    let c = stroke_style.color;
                                    let rc = Color32::from_rgba_unmultiplied(
                                        (c[0] * 255.0) as u8,
                                        (c[1] * 255.0) as u8,
                                        (c[2] * 255.0) as u8,
                                        ((c[3] * opacity) * 255.0) as u8,
                                    );
                                    painter.add(egui::epaint::PathShape::convex_polygon(
                                        screen_pts,
                                        rc,
                                        Stroke::NONE,
                                    ));
                                }
                            } else {
                                let screen_pts: Vec<Pos2> =
                                    sp.iter().map(|p| to_screen(p.x, p.y)).collect();
                                painter.add(egui::epaint::PathShape::line(screen_pts, stroke));
                            }
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
            ObjectType::Image { width, height, .. } => {
                if let Some(tex) = self.image_textures.get(&obj.id) {
                    // UV-mapped quad so rotation/skew stay exact.
                    let corners = [
                        to_screen(0.0, 0.0),
                        to_screen(*width, 0.0),
                        to_screen(*width, *height),
                        to_screen(0.0, *height),
                    ];
                    let uvs = [
                        Pos2::new(0.0, 0.0),
                        Pos2::new(1.0, 0.0),
                        Pos2::new(1.0, 1.0),
                        Pos2::new(0.0, 1.0),
                    ];
                    let tint = Color32::from_rgba_unmultiplied(
                        255,
                        255,
                        255,
                        (opacity * 255.0).round().clamp(0.0, 255.0) as u8,
                    );
                    let mut mesh = egui::epaint::Mesh {
                        texture_id: tex.id(),
                        ..Default::default()
                    };
                    for (pos, uv) in corners.into_iter().zip(uvs) {
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos,
                            uv,
                            color: tint,
                        });
                    }
                    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
                    painter.add(mesh);
                } else {
                    // Bytes missing or undecodable: visible placeholder.
                    let a = to_screen(0.0, 0.0);
                    let b = to_screen(*width, *height);
                    let r = egui::Rect::from_min_max(
                        Pos2::new(a.x.min(b.x), a.y.min(b.y)),
                        Pos2::new(a.x.max(b.x), a.y.max(b.y)),
                    );
                    painter.rect_stroke(
                        r,
                        0.0,
                        Stroke::new(1.0_f32, Color32::from_rgb(200, 80, 80)),
                        egui::StrokeKind::Outside,
                    );
                }
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
                // Line height from style (default 1.2em)
                let line_height = (style.effective_line_height() * state.zoom as f64) as f32;
                // Use word-wrapped lines when enabled
                let lines: Vec<String> = if style.word_wrap {
                    if let Some(max_w) = style.max_width {
                        crate::core::document::object::compute_wrapped_lines(text, style, max_w)
                    } else {
                        text.split('\n').map(String::from).collect()
                    }
                } else {
                    text.split('\n').map(String::from).collect()
                };
                let lines: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
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
                    self.draw_object(painter, child, origin, state, &composed, ancestor_opacity);
                }
            }
            ObjectType::ClippingMask { children } => {
                for child in children {
                    self.draw_object(painter, child, origin, state, &composed, ancestor_opacity);
                }
            }
            ObjectType::Use { href, .. } => {
                let symbol_id = href.trim_start_matches('#');
                if let Some(sym) = state.document.symbol_by_id(symbol_id) {
                    let mut instance_obj = sym.object.clone();
                    instance_obj.transform.x += obj.transform.x;
                    instance_obj.transform.y += obj.transform.y;
                    self.draw_object(
                        painter,
                        &instance_obj,
                        origin,
                        state,
                        parent,
                        ancestor_opacity,
                    );
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
                FillType::Image(img) => {
                    self.draw_image_fill(painter, obj, img, opacity, origin, state, parent);
                }
                FillType::Solid(_) => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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

    /// Draw a referenced raster image clipped to the object's shape.
    ///
    /// This is the canvas counterpart of the SVG `<pattern>` image fill, so
    /// both agree on [`ImageTileMode`] semantics: Cover/Contain center the
    /// uniformly-scaled image (overflow cropped / letterboxed), Fit
    /// stretches it to the bbox, Tile repeats it at natural pixel size
    /// anchored at the bbox origin. Shape triangulation (holes included)
    /// provides the clip; per-vertex UVs come from an affine world-space
    /// map, so rect-clipped fragments stay exact.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_image_fill(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        img: &ImageFill,
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

        if state.document.find_object(&img.image_id).is_none() {
            self.draw_missing_image_placeholder(painter, obj, &to_screen);
            return;
        }
        // 1px = 1 world unit, same convention as placed Image objects.
        let (tex_id, iw, ih) = match self.image_textures.get(&img.image_id) {
            Some(tex) => {
                let [w, h] = tex.size();
                (tex.id(), w as f64, h as f64)
            }
            None => {
                self.draw_missing_image_placeholder(painter, obj, &to_screen);
                return;
            }
        };
        if iw <= 0.0 || ih <= 0.0 {
            return;
        }
        let wpoly: Vec<(f64, f64)> = obj
            .to_path_data()
            .to_polygon(16)
            .iter()
            .map(|p| affine_apply(&composed, p.x, p.y))
            .collect();
        let (bx, by, bw, bh) = match world_bbox(&wpoly) {
            Some(v) => v,
            None => return,
        };
        if !(bw > 0.0) || !(bh > 0.0) {
            return;
        }
        let tris: Vec<[(f64, f64); 3]> = obj
            .to_path_data()
            .to_triangles(16)
            .iter()
            .map(|t| {
                [
                    affine_apply(&composed, t[0].x, t[0].y),
                    affine_apply(&composed, t[1].x, t[1].y),
                    affine_apply(&composed, t[2].x, t[2].y),
                ]
            })
            .collect();
        if tris.is_empty() {
            return;
        }

        let tint = Color32::from_rgba_unmultiplied(
            255,
            255,
            255,
            (opacity * 255.0).round().clamp(0.0, 255.0) as u8,
        );
        let mut mesh = egui::epaint::Mesh {
            texture_id: tex_id,
            ..Default::default()
        };
        match img.tile_mode {
            ImageTileMode::Cover | ImageTileMode::Contain => {
                let cover = img.tile_mode == ImageTileMode::Cover;
                let (ox, oy, dw, dh) = cover_contain_placement(bw, bh, iw, ih, cover);
                let (ox, oy) = (bx + ox, by + oy);
                let uv = |x: f64, y: f64| (((x - ox) / dw) as f32, ((y - oy) / dh) as f32);
                for t in &tris {
                    // Cover always spans the bbox; Contain letterboxes, so
                    // fragments outside the fitted rect must be cut away
                    // (egui clamps UVs — without this the bands would smear
                    // edge pixels instead of staying transparent).
                    let clipped = if cover {
                        t.to_vec()
                    } else {
                        clip_tri_to_rect(t, (ox, oy, dw, dh))
                    };
                    push_textured_fan(&mut mesh, &clipped, &uv, &to_screen, tint);
                }
            }
            ImageTileMode::Fit => {
                let uv = |x: f64, y: f64| (((x - bx) / bw) as f32, ((y - by) / bh) as f32);
                for t in &tris {
                    push_textured_fan(&mut mesh, t, &uv, &to_screen, tint);
                }
            }
            ImageTileMode::Tile => {
                let nx = (bw / iw).ceil() as usize;
                let ny = (bh / ih).ceil() as usize;
                let too_many =
                    nx == 0 || ny == 0 || nx.checked_mul(ny).unwrap_or(usize::MAX) > MAX_IMAGE_FILL_TILES;
                if too_many {
                    // Pathological tiling (tiny tile, huge shape): degrade to
                    // a single Cover placement instead of stalling the frame.
                    let (ox, oy, dw, dh) = cover_contain_placement(bw, bh, iw, ih, true);
                    let (ox, oy) = (bx + ox, by + oy);
                    let uv = |x: f64, y: f64| (((x - ox) / dw) as f32, ((y - oy) / dh) as f32);
                    for t in &tris {
                        push_textured_fan(&mut mesh, t, &uv, &to_screen, tint);
                    }
                } else {
                    for j in 0..ny {
                        for i in 0..nx {
                            let (tx, ty) = (bx + i as f64 * iw, by + j as f64 * ih);
                            let uv = |x: f64, y: f64| {
                                (((x - tx) / iw) as f32, ((y - ty) / ih) as f32)
                            };
                            for t in &tris {
                                let clipped = clip_tri_to_rect(t, (tx, ty, iw, ih));
                                push_textured_fan(&mut mesh, &clipped, &uv, &to_screen, tint);
                            }
                        }
                    }
                }
            }
        }
        if !mesh.indices.is_empty() {
            painter.add(mesh);
        }
    }

    /// Red outline for image fills whose source is missing or undecodable
    /// (mirrors the placeholder style of unrenderable Image objects).
    fn draw_missing_image_placeholder(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        to_screen: &dyn Fn(f64, f64) -> Pos2,
    ) {
        let pts: Vec<Pos2> = obj
            .to_path_data()
            .to_polygon(16)
            .iter()
            .map(|p| to_screen(p.x, p.y))
            .collect();
        if pts.len() >= 2 {
            painter.add(egui::epaint::PathShape::closed_line(
                pts,
                Stroke::new(1.0_f32, Color32::from_rgb(200, 80, 80)),
            ));
        }
    }
}

/// Upper bound on tiles rasterized for [`ImageTileMode::Tile`] previews.
const MAX_IMAGE_FILL_TILES: usize = 2048;

/// World-space bbox `(min_x, min_y, w, h)` of a point cloud.
fn world_bbox(pts: &[(f64, f64)]) -> Option<(f64, f64, f64, f64)> {
    if pts.is_empty() {
        return None;
    }
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for (x, y) in pts {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x);
        max_y = max_y.max(*y);
    }
    Some((min_x, min_y, max_x - min_x, max_y - min_y))
}

/// Image placement `(ox, oy, dw, dh)` relative to the shape bbox origin for
/// Cover (`cover = true`, uniform scale until the bbox is fully covered) and
/// Contain (`cover = false`, whole image fits inside), both centered —
/// mirroring SVG `xMidYMid slice` / `xMidYMid meet`.
fn cover_contain_placement(bw: f64, bh: f64, iw: f64, ih: f64, cover: bool) -> (f64, f64, f64, f64) {
    let s = if cover {
        (bw / iw).max(bh / ih)
    } else {
        (bw / iw).min(bh / ih)
    };
    let (dw, dh) = (iw * s, ih * s);
    (-(dw - bw) / 2.0, -(dh - bh) / 2.0, dw, dh)
}

/// Clip a triangle to an axis-aligned rect `(x, y, w, h)` (Sutherland–Hodgman).
/// Returns the convex intersection polygon (empty when disjoint). New
/// vertices keep world coordinates, so the caller's affine UV map stays
/// exact with no interpolation bookkeeping.
fn clip_tri_to_rect(tri: &[(f64, f64); 3], rect: (f64, f64, f64, f64)) -> Vec<(f64, f64)> {
    let (rx, ry, rw, rh) = rect;
    if rw <= 0.0 || rh <= 0.0 {
        return Vec::new();
    }
    let (x0, y0, x1, y1) = (rx, ry, rx + rw, ry + rh);
    let mut poly = vec![tri[0], tri[1], tri[2]];
    // (axis, bound, keep_greater): left, right, top, bottom.
    for (axis, bound, keep_greater) in
        [(0u8, x0, true), (0, x1, false), (1, y0, true), (1, y1, false)]
    {
        if poly.is_empty() {
            break;
        }
        let coord = |p: (f64, f64)| if axis == 0 { p.0 } else { p.1 };
        let mut out: Vec<(f64, f64)> = Vec::new();
        for i in 0..poly.len() {
            let cur = poly[i];
            let prev = poly[(i + poly.len() - 1) % poly.len()];
            let cur_in = if keep_greater {
                coord(cur) >= bound
            } else {
                coord(cur) <= bound
            };
            let prev_in = if keep_greater {
                coord(prev) >= bound
            } else {
                coord(prev) <= bound
            };
            if cur_in {
                if !prev_in {
                    out.push(edge_intersect(prev, cur, axis, bound));
                }
                out.push(cur);
            } else if prev_in {
                out.push(edge_intersect(prev, cur, axis, bound));
            }
        }
        poly = out;
    }
    poly
}

/// Intersection of segment `a→b` with the line `axis = bound`.
fn edge_intersect(a: (f64, f64), b: (f64, f64), axis: u8, bound: f64) -> (f64, f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    if axis == 0 {
        let t = if dx.abs() > 1e-12 {
            ((bound - a.0) / dx).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (bound, a.1 + dy * t)
    } else {
        let t = if dy.abs() > 1e-12 {
            ((bound - a.1) / dy).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a.0 + dx * t, bound)
    }
}

/// Append a convex world-space polygon to a textured mesh, mapping each
/// vertex through `uv` (fan triangulation from vertex 0).
fn push_textured_fan(
    mesh: &mut egui::epaint::Mesh,
    poly: &[(f64, f64)],
    uv: &dyn Fn(f64, f64) -> (f32, f32),
    to_screen: &dyn Fn(f64, f64) -> Pos2,
    tint: Color32,
) {
    if poly.len() < 3 {
        return;
    }
    let base = mesh.vertices.len() as u32;
    for (x, y) in poly {
        let (u, v) = uv(*x, *y);
        mesh.vertices.push(egui::epaint::Vertex {
            pos: to_screen(*x, *y),
            uv: Pos2::new(u, v),
            color: tint,
        });
    }
    for i in 1..poly.len() - 1 {
        mesh.indices.extend([base, base + i as u32, base + i as u32 + 1]);
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
        FillType::Linear(_) | FillType::Radial(_) | FillType::Pattern(_) | FillType::Image(_) => return None,
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
#[allow(clippy::too_many_arguments)]
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
            assert!((-1.0..=1.0).contains(&d));
            assert_eq!(d, band_dither(k));
        }
        assert_ne!(band_dither(3), band_dither(4));
    }
}

#[cfg(test)]
mod image_fill_clip_tests {
    use super::*;

    fn area(poly: &[(f64, f64)]) -> f64 {
        if poly.len() < 3 {
            return 0.0;
        }
        let mut a = 0.0;
        for i in 0..poly.len() {
            let (px, py) = poly[i];
            let (qx, qy) = poly[(i + 1) % poly.len()];
            a += px * qy - qx * py;
        }
        (a * 0.5).abs()
    }

    #[test]
    fn test_clip_fully_inside_keeps_triangle() {
        let tri = [(2.0, 2.0), (4.0, 2.0), (3.0, 4.0)];
        let out = clip_tri_to_rect(&tri, (0.0, 0.0, 10.0, 10.0));
        assert_eq!(out.len(), 3);
        assert!((area(&out) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_clip_fully_outside_empties() {
        let tri = [(20.0, 20.0), (24.0, 20.0), (22.0, 24.0)];
        assert!(clip_tri_to_rect(&tri, (0.0, 0.0, 10.0, 10.0)).is_empty());
    }

    #[test]
    fn test_clip_partial_halves_right_triangle() {
        // Right triangle legs on the axes; clip to x <= 5 keeps the
        // quad (0,0),(5,0),(5,5),(0,10): full area 50 minus the cut
        // triangle (5,0),(10,0),(5,5) of area 12.5.
        let tri = [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)];
        let out = clip_tri_to_rect(&tri, (0.0, 0.0, 5.0, 10.0));
        assert!((area(&out) - 37.5).abs() < 1e-6, "area {}", area(&out));
        for (x, y) in &out {
            assert!(*x >= -1e-9 && *x <= 5.0 + 1e-9 && *y >= -1e-9 && *y <= 10.0 + 1e-9);
        }
    }

    #[test]
    fn test_clip_degenerate_rect_empties() {
        let tri = [(2.0, 2.0), (4.0, 2.0), (3.0, 4.0)];
        assert!(clip_tri_to_rect(&tri, (0.0, 0.0, 0.0, 10.0)).is_empty());
    }

    #[test]
    fn test_cover_contain_placement_centering() {
        // Same aspect: both collapse to an exact fit.
        let (ox, oy, dw, dh) = cover_contain_placement(200.0, 100.0, 4.0, 2.0, true);
        assert!(ox.abs() < 1e-9 && oy.abs() < 1e-9);
        assert!((dw - 200.0).abs() < 1e-9 && (dh - 100.0).abs() < 1e-9);
        // Wide 8x2 image into a 200x100 bbox.
        let (ox, oy, dw, dh) = cover_contain_placement(200.0, 100.0, 8.0, 2.0, true);
        assert!((dw - 400.0).abs() < 1e-9 && (dh - 100.0).abs() < 1e-9);
        assert!((ox + 100.0).abs() < 1e-9 && oy.abs() < 1e-9, "cover centers overflow");
        let (ox, oy, dw, dh) = cover_contain_placement(200.0, 100.0, 8.0, 2.0, false);
        assert!((dw - 200.0).abs() < 1e-9 && (dh - 50.0).abs() < 1e-9);
        assert!(ox.abs() < 1e-9 && (oy - 25.0).abs() < 1e-9, "contain centers bands");
    }

    #[test]
    fn test_push_textured_fan_emits_indexed_mesh() {
        let mut mesh = egui::epaint::Mesh::default();
        let quad = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        push_textured_fan(
            &mut mesh,
            &quad,
            &|x, y| (x as f32 / 10.0, y as f32 / 10.0),
            &|x, y| Pos2::new(x as f32, y as f32),
            Color32::WHITE,
        );
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(mesh.vertices[2].uv, Pos2::new(1.0, 1.0));
        // Degenerate input appends nothing.
        push_textured_fan(
            &mut mesh,
            &quad[..2],
            &|x, y| (x as f32, y as f32),
            &|x, y| Pos2::new(x as f32, y as f32),
            Color32::WHITE,
        );
        assert_eq!(mesh.vertices.len(), 4);
    }
}
