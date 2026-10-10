/// Draw one ruby reading at half size to the right of a vertical column,
/// centred on the base group's vertical extent. Kept out of the main
/// draw loop so the loop stays readable.
#[allow(clippy::too_many_arguments)]
fn draw_canvas_ruby(
    painter: &egui::Painter,
    font_id: egui::FontId,
    color: egui::Color32,
    reading: &str,
    col_x_local: f64,
    start_y: f64,
    end_y: f64,
    style: &crate::core::document::TextStyle,
    to_screen: &impl Fn(f64, f64) -> egui::Pos2,
) {
    let font_size = style.effective_font_size();
    let small = egui::FontId::new(font_id.size / 2.0, font_id.family.clone());
    let galley = painter.layout_no_wrap(reading.to_string(), small, color);
    let gw = galley.size().x;
    let gh = galley.size().y;
    let mid = (start_y + end_y) / 2.0;
    // Strip to the right of the base column: [font_size, font_size*1.5].
    let cx = to_screen(col_x_local + font_size * RUBY_STRIP_CENTER_EM, mid).x - gw / 2.0;
    let cy = to_screen(col_x_local, mid).y - gh / 2.0;
    painter.galley(egui::pos2(cx, cy), galley, color);
}

use super::CanvasWidget;
use crate::core::document::{
    char_advance_estimate, is_fullwidth_char, is_ja_latin_boundary, ja_latin_gap_em, BlendMode,
    Object, ObjectType, TextArea, TextStyle, RUBY_STRIP_CENTER_EM,
};
use crate::core::path::{
    sample_pattern_primitives_for_frame, AnchorPoint, FillStyle, FillType, ImageFill,
    ImageTileMode, LinearGradient, PathData, PatternFill, PatternPrimitive, RadialGradient,
    PATTERN_FRAME_BUDGET,
};
use crate::core::state::AppState;
use crate::ui::canvas::clip::{self, ClipRegion};
use egui::{Color32, FontId, Pos2, Stroke};

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

/// One cached text line: local-space triangles plus the local advance width
/// used for middle/end anchoring.
pub(super) struct CachedTextLine {
    pub tris: Vec<[AnchorPoint; 3]>,
    #[allow(dead_code)]
    pub width: f64,
    /// Local x shift aligning the line to its anchor (precomputed at bake:
    /// point text anchors on the origin, area text on the box edges/center).
    pub x_off: f64,
}

/// Baked outlines for one text object. `origin` is the first-baseline origin
/// in local coordinates (area text starts at the box's top-left em-box, point
/// text at 0,0) and `visible` is how many lines fit inside an area box —
/// the remainder overflows and is clipped on canvas and in export.
pub(super) struct CachedText {
    pub lines: Vec<CachedTextLine>,
    pub origin: (f64, f64),
    pub visible: usize,
}

impl CachedText {
    /// Lines that should actually be painted.
    pub fn drawable(&self) -> usize {
        self.visible.min(self.lines.len())
    }
}

/// Cache key for a text object's baked outlines. Any shaping input change
/// (text, face, size, spacing, wrap, **area box**) re-bakes; transforms do
/// not (they apply per-frame to the cached local triangles).
pub fn text_shape_key(text: &str, style: &TextStyle, area: Option<TextArea>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    style.font_family.hash(&mut h);
    style.font_size.to_bits().hash(&mut h);
    style.font_weight.hash(&mut h);
    (match style.font_style {
        crate::core::document::FontStyle::Normal => 0u8,
        crate::core::document::FontStyle::Italic => 1u8,
        crate::core::document::FontStyle::Oblique => 2u8,
    })
    .hash(&mut h);
    style.letter_spacing.to_bits().hash(&mut h);
    style.line_height.map(f64::to_bits).hash(&mut h);
    style.max_width.map(f64::to_bits).hash(&mut h);
    style.word_wrap.hash(&mut h);
    style.vertical.hash(&mut h);
    style.ligatures.hash(&mut h);
    // OpenType overrides reshape glyphs (palt/halt/vert/…) and 和欧間 spacing
    // moves them: both must invalidate the cached outline triangles.
    (match style.text_anchor {
        crate::core::document::TextAnchor::Start => 0u8,
        crate::core::document::TextAnchor::Middle => 1u8,
        crate::core::document::TextAnchor::End => 2u8,
    })
    .hash(&mut h);
    style.auto_spacing.hash(&mut h);
    style.auto_spacing_em.to_bits().hash(&mut h);
    style.ot_features.len().hash(&mut h);
    for f in &style.ot_features {
        f.tag.hash(&mut h);
        f.on.hash(&mut h);
    }
    // Variable-font axes reshape glyph outlines; include them in the key.
    style.variations.len().hash(&mut h);
    for v in &style.variations {
        v.axis.hash(&mut h);
        v.value.to_bits().hash(&mut h);
    }
    // Area box is a shaping input: resizing the box rewraps the text.
    area.map(|a| {
        (
            a.x.to_bits(),
            a.y.to_bits(),
            a.width.to_bits(),
            a.height.to_bits(),
        )
    })
    .hash(&mut h);
    h.finish()
}

/// Split a text object into drawable lines, mirroring the legacy egui path
/// and the SVG exporter so all three agree. Area text wraps to the box
/// width; `layout_text` also reports how many lines fit (overflow is
/// clipped on canvas and in export).
///
/// Kept as the documented line-splitting primitive (unit-tested); renderers
/// now prefer thread-aware layouts via `thread_frame_layout`.
#[allow(dead_code)]
pub(super) fn text_draw_lines(
    text: &str,
    style: &TextStyle,
    area: Option<TextArea>,
) -> Vec<String> {
    crate::core::document::layout_text(text, style, area).lines
}

impl CanvasWidget {
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_object(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        origin: Pos2,
        state: &AppState,
        parent: &[f64; 6],
        ancestor_opacity: f32,
        clip: Option<&ClipRegion>,
    ) {
        let opacity = (obj.opacity * ancestor_opacity).clamp(0.0_f32, 1.0_f32);
        // Narrow the painter as soon as a clip is in play: the mask's
        // bounding box is exact for a rectangular mask and at least bounds
        // everything the polygon itself cannot cut (text, images, gradient
        // quads).  Fills and strokes are cut by `clip` proper below.
        let clipped_painter;
        let painter = match clip {
            Some(c) => {
                clipped_painter = painter.with_clip_rect(c.rect());
                &clipped_painter
            }
            None => painter,
        };
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

        // Illustrator's colour adjustments filter the *rendered* object; the
        // canvas previews them at the color level through the very matrix the
        // SVG export emits (`core::effects::color_adjust_matrix`), so screen
        // and file cannot drift apart.  Exact for flat artwork, an
        // approximation for gradients/images — the exporter does those per
        // pixel.
        let adjust_matrix = obj
            .appearance
            .has_color_adjust()
            .and_then(crate::core::effects::color_adjust_matrix);
        let adjust = |c: [f32; 4]| -> [f32; 4] {
            match adjust_matrix {
                Some(m) => crate::core::effects::apply_color_adjust_matrix(&m, c),
                None => c,
            }
        };
        let fill_color = fill_color.map(|fc| {
            let c = adjust([
                fc.r() as f32 / 255.0,
                fc.g() as f32 / 255.0,
                fc.b() as f32 / 255.0,
                fc.a() as f32 / 255.0,
            ]);
            Color32::from_rgba_unmultiplied(
                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                (c[3].clamp(0.0, 1.0) * 255.0) as u8,
            )
        });

        // Non-Normal modes are previewed by blending the fill against the
        // white artboard.  The maths itself is delegated to the shared
        // `core::blend::blend_colors`, so the canvas agrees with the CLI's
        // `blend` command; SVG export still uses the real CSS
        // mix-blend-mode, since the artboard is only a white backdrop.
        let fill_color = fill_color.map(|fc| {
            if obj.blend_mode == BlendMode::Normal || fc.a() == 0 {
                return fc;
            }
            let bg = [1.0, 1.0, 1.0, 1.0]; // artboard white
            let blended = crate::core::blend::blend_colors(
                [
                    fc.r() as f32 / 255.0,
                    fc.g() as f32 / 255.0,
                    fc.b() as f32 / 255.0,
                    fc.a() as f32 / 255.0,
                ],
                bg,
                obj.blend_mode.to_blend(),
            );
            Color32::from_rgba_unmultiplied(
                (blended[0] * 255.0) as u8,
                (blended[1] * 255.0) as u8,
                (blended[2] * 255.0) as u8,
                (blended[3] * 255.0) as u8,
            )
        });

        // Separations preview: map the resolved paint to the active plate.
        // Preview-only; the document is untouched. Text, halo and overlays
        // downstream all read this same `fill_color`, so plates stay
        // consistent across every fill consumer.
        let fill_spot = obj.fill.as_ref().and_then(|f| f.spot.clone());
        let fill_color = fill_color.and_then(|fc| {
            let rgb = [
                fc.r() as f32 / 255.0,
                fc.g() as f32 / 255.0,
                fc.b() as f32 / 255.0,
                fc.a() as f32 / 255.0,
            ];
            crate::core::print::plate_preview(rgb, fill_spot.as_deref(), &state.preview_plate).map(
                |c| {
                    Color32::from_rgba_unmultiplied(
                        (c[0] * 255.0) as u8,
                        (c[1] * 255.0) as u8,
                        (c[2] * 255.0) as u8,
                        (c[3].clamp(0.0, 1.0) * 255.0) as u8,
                    )
                },
            )
        });

        // Same adjustment for the stroke — but only clone when an effect is
        // in play, so the common path keeps borrowing the document instead
        // of copying every stroke each frame.
        let adjusted_stroke;
        let stroke_style = match obj.stroke.as_ref() {
            Some(s) if adjust_matrix.is_some() => {
                let mut s = s.clone();
                let c = adjust(s.color);
                s.color = [
                    c[0].clamp(0.0, 1.0),
                    c[1].clamp(0.0, 1.0),
                    c[2].clamp(0.0, 1.0),
                    c[3].clamp(0.0, 1.0),
                ];
                adjusted_stroke = Some(s);
                adjusted_stroke.as_ref()
            }
            other => other,
        };

        // Effects live in object space, so their rims are scaled like every
        // other geometry — otherwise the preview drifts from the SVG export,
        // which scales the filter with the transform (and 「線幅と効果を拡大・
        // 縮小」off, which divides the stored radius by the scale change).
        let vscale = obj.visual_scale() as f32;
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
                    let spread = sh.blur_radius as f32 * t * 0.5 * vscale;
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
                    let spread = (gl.radius as f32 * tf) * state.zoom * vscale;
                    // Gaussian-like falloff: alpha drops as exp(-x^2)
                    let tier_alpha = (base_c[3]
                        * gl.intensity
                        * opacity
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

        // Gaussian blur preview.  A blur shows at the edge — the interior of
        // a shape keeps its color — so a few widening tiers of the
        // silhouette in the object's own colour read as a soft rim without
        // an offscreen blur pass.  SVG export runs the real feGaussianBlur
        // (see `color_adjust_matrix`'s sibling in `core::effects`).
        if let Some(radius) = obj.appearance.has_blur() {
            let halo_color = fill_color.or_else(|| {
                stroke_style.map(|s| {
                    Color32::from_rgba_unmultiplied(
                        (s.color[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (s.color[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (s.color[2].clamp(0.0, 1.0) * 255.0) as u8,
                        (s.color[3].clamp(0.0, 1.0) * opacity * 255.0) as u8,
                    )
                })
            });
            let poly = obj.to_path_data().to_polygon(16);
            if radius > 0.0 && poly.len() >= 3 {
                if let Some(halo) = halo_color {
                    for tier in 1..=5u32 {
                        let t = tier as f32 / 5.0;
                        let spread = radius as f32 * t * state.zoom * vscale;
                        // Gaussian-ish falloff, same shape the glow uses.
                        let tier_alpha = (halo.a() as f32 * 0.45 * (-(t * t) * 2.0).exp()) as u8;
                        let tier_color = Color32::from_rgba_unmultiplied(
                            halo.r(),
                            halo.g(),
                            halo.b(),
                            tier_alpha,
                        );
                        let halo_pts: Vec<Pos2> =
                            poly.iter().map(|p| to_screen(p.x, p.y)).collect();
                        painter.add(egui::epaint::PathShape::closed_line(
                            halo_pts,
                            Stroke::new(spread * 2.0, tier_color),
                        ));
                    }
                }
            }
        }

        match &obj.object_type {
            ObjectType::Path(path) => {
                let subpaths = path.to_stroke_subpaths(16);
                let triangles = path.to_triangles(16);
                if !triangles.is_empty() {
                    if let Some(fill) = fill_color {
                        for tri in &triangles {
                            let screen = [
                                to_screen(tri[0].x, tri[0].y),
                                to_screen(tri[1].x, tri[1].y),
                                to_screen(tri[2].x, tri[2].y),
                            ];
                            clip::paint_fill(painter, &screen, fill, clip);
                        }
                    }
                }
                if let Some(style) = stroke_style {
                    // Variable-width profile: stroke becomes a filled ribbon
                    // honouring per-position multipliers.  Uses the raw
                    // StrokeStyle (document units), not the zoomed egui stroke.
                    let ribbon_profile = obj
                        .width_profile
                        .as_ref()
                        .zip(stroke_style)
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
                                    let screen_pts: Vec<Pos2> =
                                        ribbon.iter().map(|p| to_screen(p.x, p.y)).collect();
                                    // Ribbon carries stroke color as fill.
                                    let rc = crate::ui::canvas::stroke_paint::stroke_color(
                                        stroke_style,
                                        opacity,
                                        &state.preview_plate,
                                    )
                                    .unwrap_or(Color32::TRANSPARENT);
                                    clip::paint_fill(painter, &screen_pts, rc, clip);
                                }
                            } else {
                                // Dash / cap / join / miter limit are all
                                // honoured here; egui's own Stroke can't be.
                                crate::ui::canvas::stroke_paint::paint_stroke(
                                    painter,
                                    style,
                                    sp,
                                    path.closed,
                                    opacity,
                                    &to_screen,
                                    clip,
                                    &state.preview_plate,
                                );
                            }
                        }
                    }
                }
            }
            ObjectType::TextOnPath {
                text,
                style,
                path: tp,
                start_offset,
                side,
                ..
            } => {
                let path = crate::core::text_path::text_on_path_outlines(
                    tp,
                    text,
                    style,
                    *start_offset,
                    *side,
                );
                let subpaths = path.to_stroke_subpaths(16);
                let triangles = path.to_triangles(16);
                if let Some(fill) = fill_color {
                    for tri in &triangles {
                        let screen = [
                            to_screen(tri[0].x, tri[0].y),
                            to_screen(tri[1].x, tri[1].y),
                            to_screen(tri[2].x, tri[2].y),
                        ];
                        clip::paint_fill(painter, &screen, fill, clip);
                    }
                }
                if let Some(style) = stroke_style {
                    for sp in &subpaths {
                        if sp.len() >= 2 {
                            crate::ui::canvas::stroke_paint::paint_stroke(
                                painter,
                                style,
                                sp,
                                path.closed,
                                opacity,
                                &to_screen,
                                clip,
                                &state.preview_plate,
                            );
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
                    clip::paint_fill(painter, &screen_pts, fill, clip);
                }
                if let Some(style) = stroke_style {
                    crate::ui::canvas::stroke_paint::paint_path_stroke(
                        painter,
                        &path,
                        style,
                        opacity,
                        &to_screen,
                        clip,
                        &state.preview_plate,
                    );
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
                    clip::paint_fill(painter, &screen_pts, fill, clip);
                }
                if let Some(style) = stroke_style {
                    crate::ui::canvas::stroke_paint::paint_path_stroke(
                        painter,
                        &path,
                        style,
                        opacity,
                        &to_screen,
                        clip,
                        &state.preview_plate,
                    );
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
                    clip::paint_fill(painter, &screen_pts, fill, clip);
                }
                if let Some(style) = stroke_style {
                    crate::ui::canvas::stroke_paint::paint_path_stroke(
                        painter,
                        &path,
                        style,
                        opacity,
                        &to_screen,
                        clip,
                        &state.preview_plate,
                    );
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
                    clip::paint_fill(painter, &screen_pts, fill, clip);
                }
                if let Some(style) = stroke_style {
                    crate::ui::canvas::stroke_paint::paint_path_stroke(
                        painter,
                        &path,
                        style,
                        opacity,
                        &to_screen,
                        clip,
                        &state.preview_plate,
                    );
                }
            }
            ObjectType::Line { x2, y2 } => {
                let p1 = to_screen(0.0, 0.0);
                let p2 = to_screen(*x2, *y2);
                match stroke_style {
                    Some(style) => crate::ui::canvas::stroke_paint::paint_stroke(
                        painter,
                        style,
                        &[
                            crate::core::path::AnchorPoint::new(0.0, 0.0),
                            crate::core::path::AnchorPoint::new(*x2, *y2),
                        ],
                        false,
                        opacity,
                        &to_screen,
                        clip,
                        &state.preview_plate,
                    ),
                    // Lines created without a stroke style still have to be
                    // visible: keep the plain 2 px screen-space segment.
                    None => {
                        painter.line_segment([p1, p2], Stroke::new(2.0_f32, Color32::BLACK));
                    }
                }
            }
            ObjectType::PixelArt(p) => {
                // Same UV-mapped quad as placed images; the texture itself
                // is NEAREST-filtered (see ensure_pixel_textures) so dots
                // stay crisp at any zoom.
                let (w, h) = (p.width as f64, p.height as f64);
                if let Some((tex, _)) = self.pixel_textures.get(&obj.id) {
                    let corners = [
                        to_screen(0.0, 0.0),
                        to_screen(w, 0.0),
                        to_screen(w, h),
                        to_screen(0.0, h),
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
                    let a = to_screen(0.0, 0.0);
                    let b = to_screen(w, h);
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
            ObjectType::GradientMesh(m) => {
                // Vertex-colored triangles: smooth on canvas at any zoom.
                // Fixed subdivision keeps frame cost bounded (a 16×16 mesh
                // is 225 patches × 72 tris worst case).
                let mut mesh = egui::epaint::Mesh::default();
                for (tri, cols) in m.triangulate(6) {
                    let base = mesh.vertices.len() as u32;
                    for (p, c) in tri.iter().zip(cols.iter()) {
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos: to_screen(p.x, p.y),
                            uv: Pos2::ZERO,
                            color: Color32::from_rgba_unmultiplied(
                                (c[0] * 255.0).round().clamp(0.0, 255.0) as u8,
                                (c[1] * 255.0).round().clamp(0.0, 255.0) as u8,
                                (c[2] * 255.0).round().clamp(0.0, 255.0) as u8,
                                (c[3] * opacity * 255.0).round().clamp(0.0, 255.0) as u8,
                            ),
                        });
                    }
                    mesh.indices.extend([base, base + 1, base + 2]);
                }
                if !mesh.indices.is_empty() {
                    painter.add(mesh);
                }
                // Node markers for the selected mesh (panel edits values;
                // direct dragging is a follow-up).
                if state.selected_ids.contains(&obj.id) {
                    for n in &m.nodes {
                        let p = to_screen(n.x, n.y);
                        painter.rect_filled(
                            egui::Rect::from_center_size(p, egui::Vec2::splat(7.0)),
                            1.0,
                            Color32::from_rgb(20, 115, 230),
                        );
                        painter.rect_stroke(
                            egui::Rect::from_center_size(p, egui::Vec2::splat(7.0)),
                            1.0,
                            Stroke::new(1.0_f32, Color32::WHITE),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
            }
            ObjectType::Envelope { .. } => {
                // Live deform renders through a plain-Path proxy (same
                // paint, transform and ancestry).
                if let Some(proxy) = obj.envelope_proxy() {
                    self.draw_object(
                        painter,
                        &proxy,
                        origin,
                        state,
                        parent,
                        ancestor_opacity,
                        clip,
                    );
                }
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
                area,
                ..
            } => {
                // Real typeface first (cached outline triangles); missing
                // fonts fall through to the legacy egui-font path below.
                if self.draw_real_text(painter, obj, text, style, fill_color, &to_screen, state) {
                    self.draw_fill_overlay(painter, obj, opacity, origin, state, parent);
                    return;
                }
                // The legacy duplicated size field may carry hostile values
                // from hand-edited files; fall back to the style size.
                let font_size = if font_size.is_finite() && *font_size > 0.0 {
                    *font_size
                } else {
                    style.effective_font_size()
                };
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
                // Area text wraps to the box and starts at the box origin;
                // overflow past the box bottom is dropped (same as the baked
                // outline path and the SVG exporter).
                let layout = crate::core::document::layout_text(text, style, *area);
                let lines: Vec<String> = layout.lines.clone();
                let lines: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
                let (origin_x, origin_y) = layout.origin;
                let base_pos = to_screen(origin_x, origin_y);
                // Anchor reference in local coords: point text anchors on
                // the origin, area text on the box edges/center.
                let anchor_lx = match style.text_anchor {
                    crate::core::document::TextAnchor::Start => origin_x,
                    crate::core::document::TextAnchor::Middle => {
                        area.map(|a| a.x + a.width / 2.0).unwrap_or(origin_x)
                    }
                    crate::core::document::TextAnchor::End => {
                        area.map(|a| a.x + a.width).unwrap_or(origin_x)
                    }
                };
                let mut widest: f32 = 0.0;
                // Faux-italic block shear: egui draws glyphs axis-aligned,
                // so single lines can't slant (a real oblique needs glyph
                // outlines — see text_path / SVG skewX export). Shifting
                // successive baselines still signals italic for multiline
                // text and matches the export slant direction.
                let italic = !matches!(style.font_style, crate::core::document::FontStyle::Normal);
                if style.vertical {
                    // 縦組み: columns advance right→left (each successive
                    // column one line-height to the left) and glyphs stack
                    // top to bottom. Fullwidth glyphs stand upright;
                    // halfwidth glyphs rotate 90° clockwise.
                    for (li, line) in lines.iter().take(layout.visible).enumerate() {
                        let col_x_local = origin_x - li as f64 * style.effective_line_height();
                        let mut y_local = origin_y;
                        let mut prev_sig: Option<char> = None;
                        let (base_line, ruby_anns) = crate::core::document::parse_ruby(line);
                        let chars: Vec<char> = base_line.chars().collect();
                        let mut ci = 0;
                        let mut ruby_open: Option<(f64, usize)> = None;
                        while ci < chars.len() {
                            let ch = chars[ci];
                            ci += 1;
                            if char_advance_estimate(ch) == 0.0 {
                                continue;
                            }
                            // 和欧間 auto spacing advances the cell.
                            if style.auto_spacing {
                                if let Some(p) = prev_sig {
                                    if is_ja_latin_boundary(p, ch) {
                                        y_local += ja_latin_gap_em(
                                            style.effective_font_size(),
                                            style.auto_spacing_em as f64,
                                        );
                                    }
                                }
                            }
                            // Ruby group starting at this char.
                            if let Some(ann) = ruby_anns.iter().find(|a| a.start == ci - 1) {
                                ruby_open = Some((y_local, ann.len));
                            }
                            let adv_local = char_advance_estimate(ch) * style.effective_font_size()
                                + style.effective_letter_spacing();
                            let adv_screen = adv_local * state.zoom as f64;
                            let xs = to_screen(col_x_local, y_local).x;
                            let ys = to_screen(col_x_local, y_local).y;
                            // 縦中横: 2–3 digits share one em cell, set
                            // horizontally (scaled, unrotated).
                            let unit_len = crate::core::document::tatechuyoko_run(&chars, ci - 1);
                            if let Some(n) = unit_len {
                                let unit: String = chars[ci - 1..ci - 1 + n].iter().collect();
                                ci += n - 1;
                                let adv_unit =
                                    style.effective_font_size() + style.effective_letter_spacing();
                                let small = egui::FontId::new(
                                    font_id.size / n as f32,
                                    font_id.family.clone(),
                                );
                                let galley = painter.layout_no_wrap(unit, small, color);
                                let gw = galley.size().x;
                                let gh = galley.size().y;
                                let cx = to_screen(
                                    col_x_local + style.effective_font_size() / 2.0,
                                    y_local,
                                )
                                .x - gw / 2.0;
                                let cy =
                                    to_screen(col_x_local, y_local + adv_unit / 2.0).y - gh / 2.0;
                                painter.galley(egui::pos2(cx, cy), galley, color);
                                y_local += adv_unit;
                                prev_sig = Some(chars[ci - 1]);
                                // Ruby group closing on this char.
                                if let Some((start_y, left)) = ruby_open {
                                    if left <= 1 {
                                        ruby_open = None;
                                        let ann = ruby_anns.iter().find(|a| a.start + a.len == ci);
                                        if let Some(ann) = ann {
                                            draw_canvas_ruby(
                                                painter,
                                                font_id.clone(),
                                                color,
                                                &ann.reading,
                                                col_x_local,
                                                start_y,
                                                y_local,
                                                style,
                                                &to_screen,
                                            );
                                        }
                                    } else {
                                        ruby_open = Some((start_y, left - 1));
                                    }
                                }
                                continue;
                            }
                            // vert-rotated brackets: the face's vertical
                            // form is a 90° CW rotation, so draw it that
                            // way (PDF/SVG agree; see is_vert_rotated_char).
                            if is_fullwidth_char(ch)
                                && !crate::core::document::is_vert_rotated_char(ch)
                            {
                                let galley =
                                    painter.layout_no_wrap(ch.to_string(), font_id.clone(), color);
                                painter.galley(egui::pos2(xs, ys), galley, color);
                            } else {
                                let galley =
                                    painter.layout_no_wrap(ch.to_string(), font_id.clone(), color);
                                let gw = galley.size().x;
                                let gh = galley.size().y;
                                let pos = egui::pos2(
                                    xs + (line_height - gh) / 2.0,
                                    ys + (adv_screen as f32 - gw) / 2.0,
                                );
                                let ts = egui::epaint::TextShape::new(pos, galley, color)
                                    .with_angle(std::f32::consts::FRAC_PI_2);
                                painter.add(ts);
                            }
                            y_local += adv_local;
                            prev_sig = Some(ch);
                            // Ruby group closing on this char.
                            if let Some((start_y, left)) = ruby_open {
                                if left <= 1 {
                                    ruby_open = None;
                                    let ann = ruby_anns.iter().find(|a| a.start + a.len == ci);
                                    if let Some(ann) = ann {
                                        draw_canvas_ruby(
                                            painter,
                                            font_id.clone(),
                                            color,
                                            &ann.reading,
                                            col_x_local,
                                            start_y,
                                            y_local,
                                            style,
                                            &to_screen,
                                        );
                                    }
                                } else {
                                    ruby_open = Some((start_y, left - 1));
                                }
                            }
                        }
                    }
                } else {
                    for (li, line) in lines.iter().take(layout.visible).enumerate() {
                        // Ruby markup is stripped from the drawn base; readings
                        // (if any) are set above their group in the branch below.
                        let (base_line, ruby_h) = crate::core::document::parse_ruby(line);
                        let shear_dx = if italic {
                            -(crate::core::text_path::FAUX_ITALIC_SHEAR as f32)
                                * (li as f32 * line_height)
                        } else {
                            0.0
                        };
                        // Anchor in local space first (rotation-safe), then the
                        // screen-space italic shear.
                        let baseline_ly = origin_y + li as f64 * style.effective_line_height();
                        let line_pos = egui::pos2(
                            to_screen(anchor_lx, baseline_ly).x + shear_dx,
                            base_pos.y + li as f32 * line_height,
                        );
                        if !ruby_h.is_empty() {
                            // ルビ (horizontal): measure the base char-by-char so
                            // the anchored width matches what is drawn, then set
                            // each reading above its group's centre.
                            let letter_space_screen =
                                (style.effective_letter_spacing() * state.zoom as f64) as f32;
                            let mut widths: Vec<(char, String, f32, f32)> = Vec::new();
                            let mut total_w = 0.0;
                            for ch in base_line.chars() {
                                let ch_str = ch.to_string();
                                let galley =
                                    painter.layout_no_wrap(ch_str.clone(), font_id.clone(), color);
                                let w = galley.size().x;
                                let h = galley.size().y;
                                widths.push((ch, ch_str, w, h));
                                total_w += w;
                            }
                            let mut gaps_w = 0.0f32;
                            if style.auto_spacing {
                                for pair in widths.windows(2) {
                                    if crate::core::document::is_ja_latin_boundary(
                                        pair[0].0, pair[1].0,
                                    ) {
                                        gaps_w += (crate::core::document::ja_latin_gap_em(
                                            style.effective_font_size(),
                                            style.auto_spacing_em as f64,
                                        ) * state.zoom as f64)
                                            as f32;
                                    }
                                }
                                total_w += gaps_w;
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
                            let mut group_open: Option<(f32, usize)> = None;
                            let mut groups: Vec<(f32, f32, String)> = Vec::new();
                            let mut prev_ch: Option<char> = None;
                            let mut i = 0usize;
                            while i < widths.len() {
                                let (ch, ch_str, w, h) = &widths[i];
                                if style.auto_spacing {
                                    if let Some(p) = prev_ch {
                                        if crate::core::document::is_ja_latin_boundary(p, *ch) {
                                            curr_x += (crate::core::document::ja_latin_gap_em(
                                                style.effective_font_size(),
                                                style.auto_spacing_em as f64,
                                            ) * state.zoom as f64)
                                                as f32;
                                        }
                                    }
                                }
                                // Ruby group starting here: remember its left edge.
                                if let Some(ann) = ruby_h.iter().find(|a| a.start == i) {
                                    group_open = Some((curr_x, ann.len));
                                }
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
                                prev_ch = Some(*ch);
                                i += 1;
                                if let Some((sx, left)) = group_open {
                                    if left <= 1 {
                                        group_open = None;
                                        if let Some(ann) =
                                            ruby_h.iter().find(|a| a.start + a.len == i)
                                        {
                                            groups.push((sx, curr_x, ann.reading.clone()));
                                        }
                                    } else {
                                        group_open = Some((sx, left - 1));
                                    }
                                }
                            }
                            for (sx, ex, reading) in groups {
                                let small =
                                    egui::FontId::new(font_id.size / 2.0, font_id.family.clone());
                                let galley = painter.layout_no_wrap(reading, small, color);
                                let gw = galley.size().x;
                                let cy = line_pos.y
                                    - (style.effective_font_size()
                                        * crate::core::document::RUBY_ABOVE_EM
                                        * state.zoom as f64)
                                        as f32;
                                painter.galley(
                                    egui::pos2((sx + ex) / 2.0 - gw / 2.0, cy),
                                    galley,
                                    color,
                                );
                            }
                        } else if style.effective_letter_spacing() != 0.0 {
                            let letter_space_screen =
                                (style.effective_letter_spacing() * state.zoom as f64) as f32;
                            // Pre-measure so text-anchor (middle/end) applies to the whole run,
                            // matching exported SVG behavior.
                            let mut widths: Vec<(char, String, f32, f32)> = Vec::new();
                            let mut total_w = 0.0;
                            for ch in base_line.chars() {
                                let ch_str = ch.to_string();
                                let galley =
                                    painter.layout_no_wrap(ch_str.clone(), font_id.clone(), color);
                                let w = galley.size().x;
                                let h = galley.size().y;
                                widths.push((ch, ch_str, w, h));
                                total_w += w;
                            }
                            let mut gaps_w = 0.0f32;
                            if style.auto_spacing {
                                for pair in widths.windows(2) {
                                    if crate::core::document::is_ja_latin_boundary(
                                        pair[0].0, pair[1].0,
                                    ) {
                                        gaps_w += (crate::core::document::ja_latin_gap_em(
                                            style.effective_font_size(),
                                            style.auto_spacing_em as f64,
                                        ) * state.zoom as f64)
                                            as f32;
                                    }
                                }
                                total_w += gaps_w;
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
                            let mut prev_ch: Option<char> = None;
                            for (ch, ch_str, w, h) in &widths {
                                if style.auto_spacing {
                                    if let Some(p) = prev_ch {
                                        if crate::core::document::is_ja_latin_boundary(p, *ch) {
                                            curr_x += (crate::core::document::ja_latin_gap_em(
                                                style.effective_font_size(),
                                                style.auto_spacing_em as f64,
                                            ) * state.zoom as f64)
                                                as f32;
                                        }
                                    }
                                }
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
                                prev_ch = Some(*ch);
                            }
                        } else if style.auto_spacing
                            && crate::core::document::has_ja_latin_boundary(&base_line)
                        {
                            // 和欧間 auto-spacing: draw each script-neighbourhood
                            // segment separately, injecting the ~1/4em gap between.
                            let segs =
                                crate::core::document::split_ja_latin_segments(&base_line, style);
                            let mut total_w = 0.0f32;
                            let mut galleys = Vec::new();
                            let mut gaps = Vec::new();
                            for (seg, gap) in &segs {
                                let g = painter.layout_no_wrap(seg.clone(), font_id.clone(), color);
                                total_w += g.size().x;
                                gaps.push(if *gap {
                                    (ja_latin_gap_em(
                                        style.effective_font_size(),
                                        style.auto_spacing_em as f64,
                                    ) * state.zoom as f64)
                                        as f32
                                } else {
                                    0.0
                                });
                                galleys.push(g);
                            }
                            // Anchor across the whole run.
                            total_w += gaps.iter().sum::<f32>();
                            let start_x = match style.text_anchor {
                                crate::core::document::TextAnchor::Start => line_pos.x,
                                crate::core::document::TextAnchor::Middle => {
                                    line_pos.x - total_w / 2.0
                                }
                                crate::core::document::TextAnchor::End => line_pos.x - total_w,
                            };
                            widest = widest.max(total_w);
                            let mut curr_x = start_x;
                            for (idx, g) in galleys.iter().enumerate() {
                                if idx > 0 {
                                    curr_x += gaps[idx];
                                }
                                let h = g.size().y;
                                painter.galley(
                                    egui::pos2(curr_x, line_pos.y - h),
                                    g.clone(),
                                    color,
                                );
                                curr_x += g.size().x;
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
                }

                if !is_avail && state.selected_ids.contains(&obj.id) {
                    let text_h = line_height * lines.len() as f32;
                    let text_rect = egui::Rect::from_min_size(
                        egui::pos2(base_pos.x, base_pos.y - scaled_size),
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
                    self.draw_object(
                        painter,
                        child,
                        origin,
                        state,
                        &composed,
                        ancestor_opacity,
                        clip,
                    );
                }
            }
            ObjectType::ClippingMask { children } => {
                // children[0] is the mask (see ClippingMaskPanel), the rest is
                // the content it cuts.  Flatten the mask at the current zoom
                // so the cut lands where the SVG exporter's <clipPath> does.
                if let Some((mask, content)) = children.split_first() {
                    let mask_composed = affine_mul(&composed, &mask.transform.matrix());
                    let mask_to_screen = |wx: f64, wy: f64| -> Pos2 {
                        let (sx, sy) = affine_apply(&mask_composed, wx, wy);
                        Pos2::new(
                            origin.x + sx as f32 * state.zoom,
                            origin.y + sy as f32 * state.zoom,
                        )
                    };
                    let mask_poly: Vec<Pos2> = mask
                        .to_path_data()
                        .to_polygon(24)
                        .iter()
                        .map(|p| mask_to_screen(p.x, p.y))
                        .collect();
                    // A mask inside another mask narrows to the overlap;
                    // `None` from either side means nothing is left visible.
                    let region = match clip {
                        Some(outer) => outer.intersect(&mask_poly),
                        None => ClipRegion::new(mask_poly),
                    };
                    if let Some(region) = region {
                        for child in content {
                            self.draw_object(
                                painter,
                                child,
                                origin,
                                state,
                                &composed,
                                ancestor_opacity,
                                Some(&region),
                            );
                        }
                    }
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
                        clip,
                    );
                }
            }
        }

        self.draw_fill_overlay(painter, obj, opacity, origin, state, parent);
    }

    /// Gradient / pattern / image fills paint on top of the base fill
    /// (extracted so early-returning arms, e.g. real text, keep the exact
    /// same compositing as the fall-through path).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_fill_overlay(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        opacity: f32,
        origin: Pos2,
        state: &AppState,
        parent: &[f64; 6],
    ) {
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
            let s_min = screen_pts
                .iter()
                .map(|p| proj(*p))
                .fold(f32::INFINITY, f32::min);
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

        // The gradient fill paints over the base stroke, so it has to be
        // redrawn on top — with the same dash/cap/join/arrowhead treatment
        // the base pass uses, otherwise gradient-filled objects would show a
        // different line than solid-filled ones.
        if let Some(style) = obj.stroke.as_ref() {
            crate::ui::canvas::stroke_paint::paint_path_stroke(
                painter,
                &path,
                style,
                opacity,
                &to_screen,
                None,
                &state.preview_plate,
            );
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
            if piece.len() < 3 && t0 == 0.0 && pos_in_polygon(fx, fy, &screen_pts) {
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

        // The gradient fill paints over the base stroke, so it has to be
        // redrawn on top — with the same dash/cap/join/arrowhead treatment
        // the base pass uses, otherwise gradient-filled objects would show a
        // different line than solid-filled ones.
        if let Some(style) = obj.stroke.as_ref() {
            crate::ui::canvas::stroke_paint::paint_path_stroke(
                painter,
                &path,
                style,
                opacity,
                &to_screen,
                None,
                &state.preview_plate,
            );
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
            // Tiny tiles at this zoom would be sub-pixel noise: skip work.
            let tile_w_screen = (pat.tile_width * pat.scale) as f32 * state.zoom;
            let tile_h_screen = (pat.tile_height * pat.scale) as f32 * state.zoom;
            if tile_w_screen < 2.0 || tile_h_screen < 2.0 {
                return;
            }
            let bounds = (bb_min.x, bb_min.y, bb_max.x, bb_max.y);
            let Some(primitives) = crate::core::path::pattern_primitives(pat, bounds) else {
                return;
            };
            // Frame budget: stride-sample so heavy patterns degrade to a
            // sparser motif instead of stalling interaction. Print output is
            // unaffected (it uses the full list).
            let primitives = sample_pattern_primitives_for_frame(&primitives, PATTERN_FRAME_BUDGET);
            let screen_clip = painter.clip_rect();
            let to_screen_pattern = |p: AnchorPoint| {
                Pos2::new(
                    origin.x + p.x as f32 * state.zoom,
                    origin.y + p.y as f32 * state.zoom,
                )
            };
            let polygon: Vec<AnchorPoint> = wpoly
                .iter()
                .map(|(x, y)| AnchorPoint::new(*x, *y))
                .collect();
            let base_color = obj
                .fill
                .as_ref()
                .map(|f| f.color)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let color = Color32::from_rgba_unmultiplied(
                (base_color[0] * 255.0) as u8,
                (base_color[1] * 255.0) as u8,
                (base_color[2] * 255.0) as u8,
                (base_color[3] * opacity * 255.0).round().clamp(0.0, 255.0) as u8,
            );
            let stroke = Stroke::new(0.8_f32, color);
            let inside = |x: f64, y: f64| crate::core::geometry::point_in_polygon(x, y, &polygon);
            for primitive in primitives {
                match primitive {
                    PatternPrimitive::Line(a, b) => {
                        let sa = to_screen_pattern(a);
                        let sb = to_screen_pattern(b);
                        // Viewport cull before expensive shape clipping.
                        let line_rect = egui::Rect::from_min_max(
                            Pos2::new(sa.x.min(sb.x), sa.y.min(sb.y)),
                            Pos2::new(sa.x.max(sb.x), sa.y.max(sb.y)),
                        );
                        if !screen_clip.intersects(line_rect) {
                            continue;
                        }
                        let length = sa.distance(sb);
                        let steps = (length / 8.0).ceil().clamp(1.0, 128.0) as usize;
                        let mut run_start = None;
                        for step in 0..=steps {
                            let t = step as f64 / steps as f64;
                            let x = a.x + (b.x - a.x) * t;
                            let y = a.y + (b.y - a.y) * t;
                            let in_shape = step < steps
                                && inside(
                                    x + (b.x - a.x) / steps as f64 * 0.5,
                                    y + (b.y - a.y) / steps as f64 * 0.5,
                                );
                            if in_shape && run_start.is_none() {
                                run_start = Some(step);
                            } else if !in_shape {
                                if let Some(start) = run_start.take() {
                                    let t0 = start as f64 / steps as f64;
                                    let t1 = step as f64 / steps as f64;
                                    let p0 = AnchorPoint::new(
                                        a.x + (b.x - a.x) * t0,
                                        a.y + (b.y - a.y) * t0,
                                    );
                                    let p1 = AnchorPoint::new(
                                        a.x + (b.x - a.x) * t1,
                                        a.y + (b.y - a.y) * t1,
                                    );
                                    painter.line_segment(
                                        [to_screen_pattern(p0), to_screen_pattern(p1)],
                                        stroke,
                                    );
                                }
                            }
                        }
                    }
                    PatternPrimitive::Dot(center, radius) => {
                        let p = to_screen_pattern(center);
                        if !screen_clip.contains(p) {
                            continue;
                        }
                        // Center-point test: the old 8-perimeter `all()`
                        // erased edge dots entirely and made them pop while
                        // panning. Center coverage is stable and cheap.
                        if inside(center.x, center.y) {
                            painter.circle_filled(p, (radius as f32 * state.zoom).max(0.5), color);
                        }
                    }
                }
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
        let (tex_id, source_w, source_h) = match self.image_textures.get(&img.image_id) {
            Some(tex) => {
                let [w, h] = tex.size();
                (tex.id(), w as f64, h as f64)
            }
            None => {
                self.draw_missing_image_placeholder(painter, obj, &to_screen);
                return;
            }
        };
        let [crop_x, crop_y, crop_w, crop_h] = img.crop_rect.unwrap_or([0.0, 0.0, 1.0, 1.0]);
        if ![crop_x, crop_y, crop_w, crop_h]
            .iter()
            .all(|value| value.is_finite())
        {
            return;
        }
        let crop_x = crop_x.clamp(0.0, 1.0);
        let crop_y = crop_y.clamp(0.0, 1.0);
        let crop_right = (crop_x + crop_w).clamp(0.0, 1.0);
        let crop_bottom = (crop_y + crop_h).clamp(0.0, 1.0);
        let (crop_w, crop_h) = (crop_right - crop_x, crop_bottom - crop_y);
        if crop_w <= 0.0 || crop_h <= 0.0 {
            return;
        }
        let (iw, ih) = (source_w * crop_w as f64, source_h * crop_h as f64);
        if iw <= 0.0 || ih <= 0.0 {
            return;
        }
        let crop_uv = |u: f32, v: f32| (crop_x + u * crop_w, crop_y + v * crop_h);
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
        // `> 0.0` is false for NaN, so this also rejects a degenerate box.
        if !(bw > 0.0 && bh > 0.0) {
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
                let uv = |x: f64, y: f64| crop_uv(((x - ox) / dw) as f32, ((y - oy) / dh) as f32);
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
                let uv = |x: f64, y: f64| crop_uv(((x - bx) / bw) as f32, ((y - by) / bh) as f32);
                for t in &tris {
                    push_textured_fan(&mut mesh, t, &uv, &to_screen, tint);
                }
            }
            ImageTileMode::Tile => {
                let nx = (bw / iw).ceil() as usize;
                let ny = (bh / ih).ceil() as usize;
                let too_many = nx == 0 || ny == 0 || nx.saturating_mul(ny) > MAX_IMAGE_FILL_TILES;
                if too_many {
                    // Pathological tiling (tiny tile, huge shape): degrade to
                    // a single Cover placement instead of stalling the frame.
                    let (ox, oy, dw, dh) = cover_contain_placement(bw, bh, iw, ih, true);
                    let (ox, oy) = (bx + ox, by + oy);
                    let uv =
                        |x: f64, y: f64| crop_uv(((x - ox) / dw) as f32, ((y - oy) / dh) as f32);
                    for t in &tris {
                        push_textured_fan(&mut mesh, t, &uv, &to_screen, tint);
                    }
                } else {
                    for j in 0..ny {
                        for i in 0..nx {
                            let (tx, ty) = (bx + i as f64 * iw, by + j as f64 * ih);
                            let uv = |x: f64, y: f64| {
                                crop_uv(((x - tx) / iw) as f32, ((y - ty) / ih) as f32)
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

    /// Draw text with the document's real typeface via cached outline
    /// triangles (kerning, letter-spacing and faux-italic included).
    /// Returns false when no cache entry holds real outlines (missing font),
    /// in which case the caller falls back to the legacy egui-font path.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_real_text(
        &self,
        painter: &egui::Painter,
        obj: &Object,
        _text: &str,
        style: &TextStyle,
        fill: Option<Color32>,
        to_screen: &dyn Fn(f64, f64) -> Pos2,
        state: &AppState,
    ) -> bool {
        let Some((_, cached)) = self.text_meshes.get(&obj.id) else {
            return false;
        };
        let Some(cached) = cached else {
            return false;
        };
        // `fill` already carries opacity (same as the legacy path).
        let color = fill.unwrap_or(Color32::BLACK);
        let line_h = style.effective_line_height();
        let bold = style.font_weight >= 650;
        // Match the legacy faux-bold weight (screen-space doubling).
        let scaled_size = (style.effective_font_size() * state.zoom as f64) as f32;
        let bold_dx = (scaled_size * 0.035).clamp(0.5, 1.5);
        // Area text starts at the box's em-box origin and clips lines that
        // overflow the bottom; point text keeps origin (0,0).
        let (origin_x, origin_y) = cached.origin;
        for (li, cl) in cached.lines.iter().take(cached.drawable()).enumerate() {
            if cl.tris.is_empty() {
                continue;
            }
            // Horizontal text stacks lines down the Y axis; vertical text
            // stacks columns along X (per-line col_x offset already in
            // `cl.x_off`), so every column starts at the same Y.
            let base_y = if style.vertical {
                origin_y
            } else {
                origin_y + li as f64 * line_h
            };
            let mut mesh = egui::epaint::Mesh::default();
            for tri in &cl.tris {
                let base = mesh.vertices.len() as u32;
                for p in tri {
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: to_screen(p.x + cl.x_off + origin_x, p.y + base_y),
                        uv: Pos2::ZERO,
                        color,
                    });
                }
                mesh.indices.extend([base, base + 1, base + 2]);
            }
            if mesh.indices.is_empty() {
                continue;
            }
            // Faux-bold: overprint shifted by a fraction of a pixel, same
            // recipe as the legacy galley path.
            if bold {
                let mut emboldened = mesh.clone();
                for v in &mut emboldened.vertices {
                    v.pos.x += bold_dx;
                }
                painter.add(emboldened);
            }
            painter.add(mesh);
        }
        true
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
fn cover_contain_placement(
    bw: f64,
    bh: f64,
    iw: f64,
    ih: f64,
    cover: bool,
) -> (f64, f64, f64, f64) {
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
    for (axis, bound, keep_greater) in [
        (0u8, x0, true),
        (0, x1, false),
        (1, y0, true),
        (1, y1, false),
    ] {
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
        mesh.indices
            .extend([base, base + i as u32, base + i as u32 + 1]);
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
        FillType::Linear(_) | FillType::Radial(_) | FillType::Pattern(_) | FillType::Image(_) => {
            return None
        }
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
    let lerp = |a: Pos2, b: Pos2, s: f32| Pos2::new(a.x + s * (b.x - a.x), a.y + s * (b.y - a.y));
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
            let mut xs =
                edge_ellipse_crossings(curr.x, curr.y, next.x, next.y, cx, cy, rx, ry, target);
            xs.sort_by(|a, b| a.total_cmp(b));
            if let Some(&s) = xs.first() {
                out.push(lerp(curr, next, s));
            }
        } else if !cin && nin {
            let target = if tn > tc { t0 } else { t1 };
            let mut xs =
                edge_ellipse_crossings(curr.x, curr.y, next.x, next.y, cx, cy, rx, ry, target);
            xs.sort_by(|a, b| a.total_cmp(b));
            if let Some(&s) = xs.last() {
                out.push(lerp(curr, next, s));
            }
        } else {
            // Both outside: the edge may still cut through the band.
            let mut xs = edge_ellipse_crossings(curr.x, curr.y, next.x, next.y, cx, cy, rx, ry, t0);
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
                &poly,
                5.0,
                5.0,
                6.0,
                6.0,
                i as f32 * 0.25,
                (i + 1) as f32 * 0.25,
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
        assert!(
            (ox + 100.0).abs() < 1e-9 && oy.abs() < 1e-9,
            "cover centers overflow"
        );
        let (ox, oy, dw, dh) = cover_contain_placement(200.0, 100.0, 8.0, 2.0, false);
        assert!((dw - 200.0).abs() < 1e-9 && (dh - 50.0).abs() < 1e-9);
        assert!(
            ox.abs() < 1e-9 && (oy - 25.0).abs() < 1e-9,
            "contain centers bands"
        );
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

#[cfg(test)]
mod real_text_tests {
    use super::*;

    fn text_state(body: &str) -> (AppState, String) {
        let mut state = AppState::default();
        let obj = Object::new_text("T", body, 10.0, 20.0, 40.0);
        let id = obj.id.clone();
        state.document.add_object(obj);
        (state, id)
    }

    #[test]
    fn cache_bakes_real_outlines_and_invalidates_on_edit() {
        let (mut state, id) = text_state("Ag");
        let mut w = CanvasWidget::new();
        w.ensure_text_meshes(&state);
        let (k1, n_tris, width, min_lx) = {
            let (k, cached) = w.text_meshes.get(&id).expect("text cached");
            let cached = cached.as_ref().expect("bundled Inter resolves");
            let clines = &cached.lines;
            assert_eq!(clines.len(), 1);
            assert!(!clines[0].tris.is_empty(), "glyphs triangulate");
            assert!(clines[0].width > 0.0);
            let min_lx = clines[0]
                .tris
                .iter()
                .flat_map(|t| t.iter().map(|p| p.x))
                .fold(f64::MAX, f64::min);
            (*k, clines[0].tris.len(), clines[0].width, min_lx)
        };
        assert!(n_tris > 0 && width > 0.0);
        // Triangles live in local space (origin-anchored): the object sits
        // at x=10, so world ink would start at >= 10.
        assert!(min_lx < 10.0, "local space: {min_lx}");

        if let Some(o) = state.document.find_object_mut(&id) {
            if let ObjectType::Text { text, .. } = &mut o.object_type {
                *text = "AgA".to_string();
            }
        }
        w.ensure_text_meshes(&state);
        let k2 = w.text_meshes.get(&id).unwrap().0;
        assert_ne!(k1, k2, "edit re-bakes");
        w.ensure_text_meshes(&state);
        assert_eq!(k2, w.text_meshes.get(&id).unwrap().0, "no-op keeps cache");
    }

    #[test]
    fn cjk_in_latin_face_falls_back() {
        // Bundled Inter has no kana: per-glyph fallback shapes the kana
        // with a CJK-capable face instead of the all-or-nothing ransom
        // note of mock blocks.
        let mut state = AppState::default();
        let style = TextStyle::new("Inter", 40.0);
        let obj = Object::new_text_with_style("T", "あ", 0.0, 0.0, style);
        let id = obj.id.clone();
        state.document.add_object(obj);
        let mut w = CanvasWidget::new();
        w.ensure_text_meshes(&state);
        let (_, lines) = w.text_meshes.get(&id).expect("text cached");
        assert!(
            lines.is_some(),
            "kana outlines now come from the fallback face"
        );
    }

    #[test]
    fn removed_objects_leave_the_cache() {
        let (mut state, id) = text_state("bye");
        let mut w = CanvasWidget::new();
        w.ensure_text_meshes(&state);
        assert!(w.text_meshes.contains_key(&id));
        state.document.layers[0].objects.clear();
        w.ensure_text_meshes(&state);
        assert!(!w.text_meshes.contains_key(&id));
    }

    #[test]
    fn draw_lines_mirror_wrap_settings() {
        let plain = TextStyle::new("Inter", 10.0);
        assert_eq!(text_draw_lines("a\nb", &plain, None), vec!["a", "b"]);
        let mut wrapped = TextStyle::new("Inter", 10.0);
        wrapped.word_wrap = true;
        wrapped.max_width = Some(5.0);
        let lines = text_draw_lines("AAAA", &wrapped, None);
        assert!(lines.len() > 1, "narrow width wraps: {lines:?}");
        assert_eq!(lines.concat(), "AAAA");
    }
}
