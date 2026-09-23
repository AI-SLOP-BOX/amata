use crate::core::document::{Document, FontStyle, Object, ObjectType, TextAnchor, Transform};
use crate::core::effects::color_adjust_matrix;
use image::ImageEncoder;use crate::core::path::{
    FillType, PathData, PathElement, StrokeStyle,
};
use super::util::*;

/// Emit a `<pattern>` containing an embedded `<image>` for use as an image fill.
/// Returns the full `<pattern>...</pattern>` block (not the fill attribute),
/// or `None` when there is nothing paintable (missing image object, empty
/// bytes, undecodable PNG, degenerate shape bbox) — callers must fall back
/// to `fill="none"` instead of emitting a dangling `url(#...)` reference.
///
/// Geometry follows [`ImageTileMode`]: Cover/Contain/Fit use a single tile
/// exactly covering the shape bbox in the object's local coordinates
/// (`patternUnits="userSpaceOnUse"` resolves inside the referencing
/// element's transform, so local space is correct), differing only in
/// `preserveAspectRatio` (`xMidYMid slice` / `xMidYMid meet` / `none`).
/// Tile repeats at natural pixel size anchored at the bbox origin.
fn embed_image_for_pattern(
    doc: &Document,
    obj: &Object,
    img: &crate::core::path::ImageFill,
    pattern_id: &str,
) -> Option<String> {
    // Deep lookup: image objects nested in groups are valid fill sources.
    let image_obj = doc.find_object(&img.image_id)?;
    let png_bytes = match &image_obj.object_type {
        ObjectType::Image { png_bytes, .. } => png_bytes,
        _ => return None,
    };
    if png_bytes.is_empty() {
        return None;
    }
    let (iw, ih) = image::load_from_memory(png_bytes)
        .ok()
        .map(|d| (d.width() as f64, d.height() as f64))
        .filter(|(w, h)| *w > 0.0 && *h > 0.0)?;
    let (bb_min, bb_max) = obj.to_path_data().bounding_box()?;
    let (bx, by) = (bb_min.x, bb_min.y);
    let (bw, bh) = (bb_max.x - bb_min.x, bb_max.y - bb_min.y);
    if !(bw > 0.0) || !(bh > 0.0) {
        return None;
    }
    let data_uri = format!("data:image/png;base64,{}", base64_encode(png_bytes));
    // (tile x/y/w/h, image x/y/w/h, preserveAspectRatio)
    let (tile, place, par) = match img.tile_mode {
        crate::core::path::style::ImageTileMode::Cover => {
            ((bx, by, bw, bh), (bx, by, bw, bh), "xMidYMid slice")
        }
        crate::core::path::style::ImageTileMode::Contain => {
            ((bx, by, bw, bh), (bx, by, bw, bh), "xMidYMid meet")
        }
        crate::core::path::style::ImageTileMode::Fit => {
            ((bx, by, bw, bh), (bx, by, bw, bh), "none")
        }
        crate::core::path::style::ImageTileMode::Tile => {
            ((bx, by, iw, ih), (bx, by, iw, ih), "none")
        }
    };
    Some(format!(
        "  <pattern id=\"{}\" patternUnits=\"userSpaceOnUse\" x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\"><image href=\"{}\" x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" preserveAspectRatio=\"{}\" /></pattern>\n",
        pattern_id,
        tile.0, tile.1, tile.2, tile.3,
        data_uri,
        place.0, place.1, place.2, place.3,
        par
    ))
}

pub fn export_svg(doc: &Document) -> String {
    export_svg_with_profile(doc, None)
}

/// Emit SVG, optionally recording an ICC profile name as a comment so
/// downstream tools (and our own re-import) can recover the intent.
/// Actual ICC embedding requires profile blobs we do not vendor yet.
pub fn export_svg_with_profile(doc: &Document, color_profile: Option<&str>) -> String {
    let profile_comment = color_profile
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            let escaped = p.replace("--", "- -");
            format!("<!-- color-profile: {escaped} -->\n")
        })
        .unwrap_or_default();
    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
{profile_comment}<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
"#,
        doc.width, doc.height, doc.width, doc.height
    );

    let mut defs = String::new();
    let mut sym_defs = String::new();
    let mut id_counter = 0;

    for sym in &doc.symbols {
        sym_defs.push_str(&format!("  <symbol id=\"{}\">\n", sym.id));
        render_object_to_svg(&sym.object, doc, &mut sym_defs, &mut defs, &mut id_counter);
        sym_defs.push_str("  </symbol>\n");
    }
    defs.push_str(&sym_defs);

    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        // Layer opacity travels as a group attribute so canvas, SVG and
        // resvg-based raster export agree. (Previously it was model-only.)
        if (layer.opacity - 1.0).abs() > 1e-3 {
            let mut group = String::new();
            for obj in &layer.objects {
                render_object_to_svg(obj, doc, &mut group, &mut defs, &mut id_counter);
            }
            svg.push_str(&format!(
                "  <g opacity=\"{:.3}\">\n{}  </g>\n",
                layer.opacity, group
            ));
        } else {
            for obj in &layer.objects {
                render_object_to_svg(obj, doc, &mut svg, &mut defs, &mut id_counter);
            }
        }
    }

    if !defs.is_empty() {
        if let Some(svg_pos) = svg.find("<svg") {
            if let Some(pos) = svg[svg_pos..].find(">\n") {
                let insert_idx = svg_pos + pos + 2;
                let defs_block = format!("  <defs>\n{}  </defs>\n", defs);
                svg.insert_str(insert_idx, &defs_block);
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn path_data_to_d(path: &PathData, transform: &Transform) -> String {
    let mut d = String::new();
    let m = transform.matrix();

    for elem in &path.elements {
        match elem {
            PathElement::MoveTo(p) => {
                let tp = crate::core::geometry::transform_point_if_needed(p, &m);
                d.push_str(&format!("M {} {} ", tp.x, tp.y));
            }
            PathElement::LineTo(p) => {
                let tp = crate::core::geometry::transform_point_if_needed(p, &m);
                d.push_str(&format!("L {} {} ", tp.x, tp.y));
            }
            PathElement::CurveTo(seg) => {
                let sc1 = crate::core::geometry::transform_point_if_needed(&seg.control1, &m);
                let sc2 = crate::core::geometry::transform_point_if_needed(&seg.control2, &m);
                let se = crate::core::geometry::transform_point_if_needed(&seg.end, &m);
                d.push_str(&format!(
                    "C {} {} {} {} {} {} ",
                    sc1.x, sc1.y, sc2.x, sc2.y, se.x, se.y
                ));
            }
            PathElement::ClosePath => {
                d.push_str("Z ");
            }
        }
    }
    d
}

#[allow(dead_code)]
fn path_to_svg_with_fill(
    path: &PathData,
    transform: &Transform,
    obj_fill_attr: &str,
    obj_stroke: Option<&StrokeStyle>,
    effect_attr: &str,
) -> String {
    let d = path_data_to_d(path, transform);
    let fill = obj_fill_attr;
    let stroke = obj_stroke
        .or(path.stroke.as_ref())
        .map(|s| {
            let dash_str = s
                .dash_pattern
                .as_ref()
                .map(|dp| {
                    format!(
                        " stroke-dasharray=\"{}\"",
                        dp.iter()
                            .map(|n| n.to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                })
                .unwrap_or_default();
            format!(
                " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                color_to_svg_str(&s.color),
                s.width
            )
        })
        .unwrap_or_default();

    format!("  <path d=\"{d}\"{fill}{stroke}{effect_attr} />\n")
}

fn transform_has_linear_part(t: &Transform) -> bool {
    t.rotation.abs() > 1e-9
        || (t.scale_x - 1.0).abs() > 1e-9
        || (t.scale_y - 1.0).abs() > 1e-9
        || t.skew_x.abs() > 1e-9
        || t.skew_y.abs() > 1e-9
}

fn svg_transform_attr(t: &Transform) -> String {
    if !transform_has_linear_part(t) {
        // Pure translation still needs a transform on containers (<g>,
        // clip wrappers): they have no x/y attributes of their own, so a
        // moved group used to export at the origin.
        if t.x.abs() > 1e-9 || t.y.abs() > 1e-9 {
            return format!(" transform=\"translate({} {})\"", t.x, t.y);
        }
        return String::new();
    }
    let m = t.matrix();
    format!(
        " transform=\"matrix({} {} {} {} {} {})\"",
        m[0], m[1], m[2], m[3], m[4], m[5]
    )
}

fn render_object_to_svg(
    obj: &Object,
    doc: &Document,
    svg: &mut String,
    defs: &mut String,
    counter: &mut usize,
) {
    if !obj.visible {
        return;
    }

    // Generate SVG Filter if object has shadow, glow, blur, or color adjust
    let mut filter_attr = String::new();
    let blur_radius = obj.appearance.has_blur().unwrap_or(0.0);
    let color_matrix = obj
        .appearance
        .has_color_adjust()
        .and_then(color_adjust_matrix);
    let wants_filter = obj.shadow.is_some()
        || obj.glow.is_some()
        || (blur_radius.is_finite() && blur_radius > 0.0)
        || color_matrix.is_some();
    if wants_filter {
        *counter += 1;
        let fid = format!("filter_{}", *counter);
        let mut filter_content = String::new();

        if let Some(ref sh) = obj.shadow {
            let color_hex = color_to_svg_str(&sh.color);
            let opac = sh.opacity;
            filter_content.push_str(&format!(
                r#"    <feDropShadow dx="{}" dy="{}" stdDeviation="{}" flood-color="{}" flood-opacity="{:.2}" />
"#,
                sh.offset_x, sh.offset_y, sh.blur_radius, color_hex, opac
            ));
        }

        if let Some(ref gl) = obj.glow {
            let color_hex = color_to_svg_str(&gl.color);
            let opac = gl.intensity;
            filter_content.push_str(&format!(
                r#"    <feGaussianBlur in="SourceAlpha" stdDeviation="{}" result="blur" />
    <feFlood flood-color="{}" flood-opacity="{:.2}" result="color" />
    <feComposite in="color" in2="blur" operator="in" result="glow" />
    <feMerge>
      <feMergeNode in="glow" />
      <feMergeNode in="SourceGraphic" />
    </feMerge>
"#,
                gl.radius, color_hex, opac
            ));
        }

        if blur_radius.is_finite() && blur_radius > 0.0 {
            // Illustrator "Effect > Blur > Gaussian Blur" on the object's own
            // artwork: blur the source graphic, then paint it back. resvg
            // honors feGaussianBlur; canvas preview approximates in egui.
            filter_content.push_str(&format!(
                "    <feGaussianBlur stdDeviation=\"{:.3}\" />\n",
                blur_radius
            ));
        }

        if let Some(matrix) = color_matrix {
            // One matrix carries brightness/contrast/saturation/hue-rotate
            // (see `color_adjust_matrix` for the exact composition).
            let values = matrix
                .iter()
                .map(|v| {
                    if v.is_finite() {
                        format!("{v:.5}")
                    } else {
                        "0".to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            filter_content.push_str(&format!(
                "    <feColorMatrix type=\"matrix\" values=\"{values}\" />\n"
            ));
        }

        defs.push_str(&format!(
            "  <filter id=\"{}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\">\n{}  </filter>\n",
            fid, filter_content
        ));
        filter_attr = format!(" filter=\"url(#{fid})\"");
    }

    let opacity_str = if (obj.opacity - 1.0).abs() > 1e-3 {
        format!(" opacity=\"{:.2}\"", obj.opacity)
    } else {
        String::new()
    };
    // Blend modes travel as CSS mix-blend-mode so resvg-based raster
    // export honours them. (Canvas egui rendering has no blend-mode
    // support and still shows Normal.)
    let blend_str = obj
        .blend_mode
        .as_svg_str()
        .map(|m| format!(" mix-blend-mode=\"{m}\""))
        .unwrap_or_default();
    let effect_attr = format!("{opacity_str}{filter_attr}{blend_str}");

    // Handle fill: solid, linear, or radial gradient
    let fill_attr = if let Some(ref fill) = obj.fill {
        match &fill.fill_type {
            FillType::Solid(c) => format!(" fill=\"{}\"", color_to_svg_str(c)),
            FillType::Linear(grad) => {
                *counter += 1;
                let gid = format!("grad_{}", *counter);
                let mut stops_str = String::new();
                for s in &grad.stops {
                    let opac_s = if (s.color[3] - 1.0).abs() > 1e-3 {
                        format!(" stop-opacity=\"{:.2}\"", s.color[3])
                    } else {
                        String::new()
                    };
                    stops_str.push_str(&format!(
                        "    <stop offset=\"{:.1}%\" stop-color=\"{}\"{opac_s} />\n",
                        s.offset * 100.0,
                        color_to_svg_str(&s.color)
                    ));
                }
                defs.push_str(&format!(
                    "  <linearGradient id=\"{}\" x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\">\n{}  </linearGradient>\n",
                    gid, grad.start_x, grad.start_y, grad.end_x, grad.end_y, stops_str
                ));
                format!(" fill=\"url(#{gid})\"")
            }
            FillType::Radial(grad) => {
                *counter += 1;
                let gid = format!("rad_grad_{}", *counter);
                let mut stops_str = String::new();
                for s in &grad.stops {
                    let opac_s = if (s.color[3] - 1.0).abs() > 1e-3 {
                        format!(" stop-opacity=\"{:.2}\"", s.color[3])
                    } else {
                        String::new()
                    };
                    stops_str.push_str(&format!(
                        "    <stop offset=\"{:.1}%\" stop-color=\"{}\"{opac_s} />\n",
                        s.offset * 100.0,
                        color_to_svg_str(&s.color)
                    ));
                }
                defs.push_str(&format!(
                    "  <radialGradient id=\"{}\" cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fx=\"{:.2}\" fy=\"{:.2}\">\n{}  </radialGradient>\n",
                    gid, grad.center_x, grad.center_y, grad.radius, grad.focus_x, grad.focus_y, stops_str
                ));
                format!(" fill=\"url(#{gid})\"")
            }
            FillType::Pattern(_) => String::new(),
            FillType::Image(ref img) => {
                let pattern_id = format!("img_fill_{}", *counter);
                *counter += 1;
                match embed_image_for_pattern(doc, obj, img, &pattern_id) {
                    Some(embed) => {
                        defs.push_str(&embed);
                        format!(" fill=\"url(#{pattern_id})\"")
                    }
                    // No paintable pixels (or degenerate shape): a dangling
                    // url(#...) would render inconsistently across viewers.
                    None => " fill=\"none\"".to_string(),
                }
            }
        }
    } else {
        " fill=\"none\"".to_string()
    };

    let id_attr = if !obj.id.is_empty() {
        format!(" id=\"{}\"", xml_escape(&obj.id))
    } else {
        String::new()
    };

    match &obj.object_type {
        ObjectType::Use {
            href,
            width,
            height,
        } => {
            let href_attr = {
                let raw = if href.starts_with('#') {
                    href.clone()
                } else {
                    format!("#{href}")
                };
                xml_escape(&raw)
            };
            let dim_str = match (width, height) {
                (Some(w), Some(h)) => format!(" width=\"{w}\" height=\"{h}\""),
                (Some(w), None) => format!(" width=\"{w}\""),
                (None, Some(h)) => format!(" height=\"{h}\""),
                (None, None) => String::new(),
            };
            let transform_attr = svg_transform_attr(&obj.transform);
            if transform_has_linear_part(&obj.transform) {
                svg.push_str(&format!(
                    "  <use{id_attr} href=\"{href_attr}\" x=\"0\" y=\"0\"{dim_str}{transform_attr}{effect_attr} />\n",
                ));
            } else {
                svg.push_str(&format!(
                    "  <use{id_attr} href=\"{href_attr}\" x=\"{}\" y=\"{}\"{dim_str}{effect_attr} />\n",
                    obj.transform.x, obj.transform.y
                ));
            }
        }
        ObjectType::Path(path) => {
            // Variable-width profile: bake the stroke into a filled ribbon
            // so export matches the canvas.
            let ribbon = obj
                .width_profile
                .as_ref()
                .zip(obj.stroke.as_ref().or(path.stroke.as_ref()))
                .filter(|(_, s)| s.width > 0.0)
                .map(|(prof, s)| {
                    let mut combined = crate::core::path::PathData::new();
                    for sp in path.to_subpaths(16) {
                        if sp.len() >= 2 {
                            let band = crate::core::offset::variable_width_outline(
                                &sp,
                                prof,
                                s.width,
                                path.closed,
                            );
                            if band.len() >= 3 {
                                let mut sub =
                                    crate::core::path::PathData::from_polygon_points(
                                        &band, true,
                                    );
                                combined.elements.append(&mut sub.elements);
                            }
                        }
                    }
                    combined.fill = Some(crate::core::path::FillStyle::solid(s.color));
                    combined.stroke = None;
                    combined
                });
            let d = match &ribbon {
                Some(rp) => path_data_to_d(rp, &obj.transform),
                None => path_data_to_d(path, &obj.transform),
            };
            let fill = match &ribbon {
                Some(rp) => rp
                    .fill
                    .as_ref()
                    .map(|f| format!(" fill=\"{}\"", color_to_svg_str(&f.color)))
                    .unwrap_or_default(),
                None => fill_attr.clone(),
            };
            let stroke = match &ribbon {
                Some(_) => String::new(),
                None => obj
                    .stroke
                    .as_ref()
                    .or(path.stroke.as_ref())
                    .map(|s| {
                        let dash_str = s
                            .dash_pattern
                            .as_ref()
                            .map(|dp| {
                                format!(
                                    " stroke-dasharray=\"{}\"",
                                    dp.iter()
                                        .map(|n| n.to_string())
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                )
                            })
                            .unwrap_or_default();
                        format!(
                            " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                            color_to_svg_str(&s.color),
                            s.width
                        )
                    })
                    .unwrap_or_default(),
            };

            svg.push_str(&format!(
                "  <path{id_attr} d=\"{d}\"{fill}{stroke}{effect_attr} />\n"
            ));
        }
        ObjectType::Rectangle {
            width,
            height,
            corner_radius,
        } => {
            let stroke = obj
                .stroke
                .as_ref()
                .map(|s| {
                    let dash_str = s
                        .dash_pattern
                        .as_ref()
                        .map(|dp| {
                            format!(
                                " stroke-dasharray=\"{}\"",
                                dp.iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_default();
            let rx_str = if *corner_radius > 0.0 {
                format!(" rx=\"{corner_radius}\" ry=\"{corner_radius}\"")
            } else {
                String::new()
            };
            if transform_has_linear_part(&obj.transform) {
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <rect{id_attr} x=\"0\" y=\"0\" width=\"{}\" height=\"{}\"{rx_str}{fill_attr}{stroke}{transform_attr}{effect_attr} />\n",
                    width, height,
                ));
            } else {
                svg.push_str(&format!(
                    "  <rect{id_attr} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{rx_str}{fill_attr}{stroke}{effect_attr} />\n",
                    obj.transform.x, obj.transform.y, width, height,
                ));
            }
        }
        ObjectType::Ellipse { rx, ry } => {
            let stroke = obj
                .stroke
                .as_ref()
                .map(|s| {
                    let dash_str = s
                        .dash_pattern
                        .as_ref()
                        .map(|dp| {
                            format!(
                                " stroke-dasharray=\"{}\"",
                                dp.iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_default();
            if transform_has_linear_part(&obj.transform) {
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <ellipse{id_attr} cx=\"0\" cy=\"0\" rx=\"{}\" ry=\"{}\"{fill_attr}{stroke}{transform_attr}{effect_attr} />\n",
                    rx, ry,
                ));
            } else {
                svg.push_str(&format!(
                    "  <ellipse{id_attr} cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"{fill_attr}{stroke}{effect_attr} />\n",
                    obj.transform.x, obj.transform.y, rx, ry,
                ));
            }
        }
        ObjectType::Line { x2, y2 } => {
            let stroke = obj
                .stroke
                .as_ref()
                .map(|s| {
                    let dash_str = s
                        .dash_pattern
                        .as_ref()
                        .map(|dp| {
                            format!(
                                " stroke-dasharray=\"{}\"",
                                dp.iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_else(|| " stroke=\"#000000\" stroke-width=\"1\"".to_string());
            if transform_has_linear_part(&obj.transform) {
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <line{id_attr} x1=\"0\" y1=\"0\" x2=\"{}\" y2=\"{}\"{stroke}{transform_attr}{effect_attr} />\n",
                    x2, y2,
                ));
            } else {
                svg.push_str(&format!(
                    "  <line{id_attr} x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"{stroke}{effect_attr} />\n",
                    obj.transform.x, obj.transform.y, obj.transform.x + x2, obj.transform.y + y2,
                ));
            }
        }
        ObjectType::Text {
            text,
            font_size,
            style,
            area,
            next_frame,
        } => {
            let fill = if fill_attr.is_empty() || fill_attr == " fill=\"none\"" {
                " fill=\"#000000\"".to_string()
            } else {
                fill_attr
            };
            let escaped_text = xml_escape(text);

            let font_fam = if style.font_family.is_empty() {
                "Inter, sans-serif".to_string()
            } else {
                xml_escape(&style.font_family)
            };

            let mut extra_attrs = String::new();
            // Area text round-trips through a private attribute: the box is a
            // layout input that plain `x`/`y`/tspans cannot express.
            if let Some(a) = *area {
                extra_attrs.push_str(&format!(
                    " data-text-area=\"{} {} {} {}\"",
                    a.x, a.y, a.width, a.height
                ));
            }
            if style.font_weight != 400 {
                extra_attrs.push_str(&format!(" font-weight=\"{}\"", style.font_weight));
            }
            // NOTE: no faux-oblique skew here on purpose. Viewers that lack
            // a slanted face synthesize `font-style: italic` themselves, so
            // an extra skew would double-slant there (and break the
            // import round-trip). Deterministic obliques come from the
            // outline path (`text_path` shears when no slanted face
            // exists) — the logo workflow.
            if style.font_style != FontStyle::Normal {
                extra_attrs.push_str(&format!(
                    " font-style=\"{}\"",
                    style.font_style.as_svg_str()
                ));
            }
            if style.letter_spacing != 0.0 {
                extra_attrs.push_str(&format!(" letter-spacing=\"{}\"", style.letter_spacing));
            }
            if !style.variations.is_empty() {
                // Standard CSS property; also our round-trip channel (parse
                // reads font-variation-settings back into TextStyle.variations).
                // The CSS uses double quotes (`"wght" 700`) — escape them or
                // the whole SVG becomes malformed and every exporter fails.
                extra_attrs.push_str(&format!(
                    " font-variation-settings=\"{}\"",
                    xml_escape(&style.variation_settings_css())
                ));
            }
            if style.text_anchor != TextAnchor::Start {
                extra_attrs.push_str(&format!(
                    " text-anchor=\"{}\"",
                    style.text_anchor.as_svg_str()
                ));
            }
            if style.vertical {
                extra_attrs.push_str(" writing-mode=\"vertical-rl\"");
            }
            if !style.ligatures {
                extra_attrs.push_str(" font-variant-ligatures=\"none\"");
            }
            if let Some(nf) = next_frame {
                extra_attrs.push_str(&format!(
                    " data-text-thread=\"{}\"",
                    xml_escape(nf)
                ));
            }

            // Explicit line breaks become positioned tspans (line-height advance,
            // matching canvas). The importer turns positioned tspans back
            // into `\n`, so multi-line text round-trips.
            let line_height = style.effective_line_height();
            let (tx, ty) = if transform_has_linear_part(&obj.transform) {
                (0.0, 0.0)
            } else {
                (obj.transform.x, obj.transform.y)
            };
            // Area text lays out inside its box: lines wrap to the box width,
            // the first baseline sits at the em-box origin, and lines past the
            // box bottom are clipped (drawn in a clipPath) so raster export
            // matches the canvas.
            let layout = crate::core::document::layout_text(text, style, *area);
            let emit_lines = layout.lines.clone();
            let (area_tx, area_ty) = layout.origin;
            // Text position: local layout origin, plus the object offset in
            // the non-linear (plain x/y) case.
            let (base_x, base_y) = if transform_has_linear_part(&obj.transform) {
                (area_tx, area_ty)
            } else {
                (tx + area_tx, ty + area_ty)
            };
            // Clip rect must live in the same space as the positioned text:
            // local when the object transform applies, absolute otherwise.
            let clip_id = if layout.overflow() > 0 {
                if let Some(a) = *area {
                    *counter += 1;
                    let cid = format!("text_clip_{}", *counter);
                    let (cx, cy) = if transform_has_linear_part(&obj.transform) {
                        (a.x, a.y)
                    } else {
                        (tx + a.x, ty + a.y)
                    };
                    defs.push_str(&format!(
                        "  <clipPath id=\"{}\">\n    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" />\n  </clipPath>\n",
                        cid, cx, cy, a.width, a.height
                    ));
                    Some(cid)
                } else {
                    None
                }
            } else {
                None
            };
            let body = if emit_lines.len() > 1 {
                let mut spans = String::new();
                for (i, ln) in emit_lines.iter().enumerate() {
                    if i == 0 {
                        spans.push_str(&format!(
                            "<tspan x=\"{base_x}\">{}</tspan>",
                            xml_escape(ln)
                        ));
                    } else {
                        let ratio = line_height / style.font_size;
                        let dy_str = if (ratio - 1.2).abs() < 0.001 {
                            "1.2em".to_string()
                        } else {
                            format!("{:.2}em", ratio)
                        };
                        spans.push_str(&format!(
                            "<tspan x=\"{base_x}\" dy=\"{dy_str}\">{}</tspan>",
                            xml_escape(ln)
                        ));
                    }
                }
                spans
            } else {
                escaped_text
            };
            let transform_attr = if transform_has_linear_part(&obj.transform) {
                svg_transform_attr(&obj.transform)
            } else {
                String::new()
            };
            let clip_attr = clip_id
                .as_ref()
                .map(|c| format!(" clip-path=\"url(#{c})\""))
                .unwrap_or_default();
            // The clip rect lives in local coordinates, so in the linear
            // case the object transform moves onto the wrapping `<g>` (with
            // the clip) and the text itself stays untransformed — otherwise
            // the clip would apply in outer space and misalign.
            if transform_has_linear_part(&obj.transform) && !clip_attr.is_empty() {
                svg.push_str(&format!(
                    "  <g{clip_attr}{transform_attr}>\n  <text{id_attr} x=\"{base_x}\" y=\"{base_y}\" font-size=\"{font_size}\" font-family=\"{font_fam}\"{extra_attrs}{fill}{effect_attr}>{body}</text>\n  </g>\n",
                ));
            } else if clip_attr.is_empty() {
                svg.push_str(&format!(
                    "  <text{id_attr} x=\"{base_x}\" y=\"{base_y}\" font-size=\"{font_size}\" font-family=\"{font_fam}\"{extra_attrs}{fill}{transform_attr}{effect_attr}>{body}</text>\n",
                ));
            } else {
                svg.push_str(&format!(
                    "  <g{clip_attr}>\n  <text{id_attr} x=\"{base_x}\" y=\"{base_y}\" font-size=\"{font_size}\" font-family=\"{font_fam}\"{extra_attrs}{fill}{transform_attr}{effect_attr}>{body}</text>\n  </g>\n",
                ));
            }
        }
        ObjectType::Group(children) => {
            let transform_attr = svg_transform_attr(&obj.transform);
            svg.push_str(&format!("  <g{id_attr}{transform_attr}{effect_attr}>\n"));
            for child in children {
                render_object_to_svg(child, doc, svg, defs, counter);
            }
            svg.push_str("  </g>\n");
        }
        ObjectType::ClippingMask { children } => {
            if !children.is_empty() {
                *counter += 1;
                let clip_id = format!("clip_{}", *counter);
                let mask_obj = &children[0];
                let mask_path = mask_obj.to_path_data();
                defs.push_str(&format!(
                    "  <clipPath id=\"{}\">\n    <path d=\"{}\" />\n  </clipPath>\n",
                    clip_id,
                    path_data_to_d(&mask_path, &mask_obj.transform)
                ));
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <g{id_attr} clip-path=\"url(#{clip_id})\"{transform_attr}{effect_attr}>\n"
                ));
                for child in &children[1..] {
                    render_object_to_svg(child, doc, svg, defs, counter);
                }
                svg.push_str("  </g>\n");
            }
        }
        ObjectType::Star { .. } | ObjectType::Polygon { .. } => {
            let path = obj.to_path_data();
            let d = path_data_to_d(&path, &obj.transform);
            let fill = &fill_attr;
            let stroke = obj
                .stroke
                .as_ref()
                .or(path.stroke.as_ref())
                .map(|s| {
                    let dash_str = s
                        .dash_pattern
                        .as_ref()
                        .map(|dp| {
                            format!(
                                " stroke-dasharray=\"{}\"",
                                dp.iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"{dash_str}",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_default();
            svg.push_str(&format!(
                "  <path{id_attr} d=\"{d}\"{fill}{stroke}{effect_attr} />\n"
            ));
        }
        ObjectType::PixelArt(p) => {
            // Exact grid size (no scaling), so the pixels stay crisp by
            // construction — same shape as placed Image objects, which the
            // importer already round-trips.
            let (w, h) = (p.width, p.height);
            let raw = p.to_rgba8();
            let mut png = Vec::new();
            let ok = image::codecs::png::PngEncoder::new(&mut png)
                .write_image(&raw, w, h, image::ExtendedColorType::Rgba8)
                .is_ok();
            if !ok || png.is_empty() {
                return;
            }
            let b64 = base64_encode(&png);
            // image-rendering keeps integer-scale raster exports crisp:
            // resvg maps pixelated to Nearest filtering. NOTE: usvg 0.48
            // only picks this up from inline `style`, not from the
            // presentation attribute, so both are emitted (browsers honor
            // either; the attribute is the standards-clean one).
            let rendering =
                " image-rendering=\"pixelated\" style=\"image-rendering:pixelated\"";
            if transform_has_linear_part(&obj.transform) {
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <image{id_attr} x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"none\"{rendering} href=\"data:image/png;base64,{b64}\"{transform_attr}{effect_attr} />\n",
                ));
            } else {
                svg.push_str(&format!(
                    "  <image{id_attr} x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"none\"{rendering} href=\"data:image/png;base64,{b64}\"{effect_attr} />\n",
                    obj.transform.x, obj.transform.y
                ));
            }
        }
        ObjectType::Image {
            width,
            height,
            png_bytes,
        } => {
            // Embedded data URI so the SVG stays self-contained.
            let b64 = base64_encode(png_bytes);
            if transform_has_linear_part(&obj.transform) {
                let transform_attr = svg_transform_attr(&obj.transform);
                svg.push_str(&format!(
                    "  <image{id_attr} x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" preserveAspectRatio=\"none\" href=\"data:image/png;base64,{b64}\"{transform_attr}{effect_attr} />\n",
                ));
            } else {
                svg.push_str(&format!(
                    "  <image{id_attr} x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{height}\" preserveAspectRatio=\"none\" href=\"data:image/png;base64,{b64}\"{effect_attr} />\n",
                    obj.transform.x, obj.transform.y
                ));
            }
        }
        ObjectType::GradientMesh(m) => {
            // No renderer supports meshgradient widely, so bake flat quads
            // (compatible everywhere: browsers, resvg, print paths).
            let mut subdiv = 6usize;
            let mut quads = m.quads(subdiv);
            while quads.len() > 20000 && subdiv > 1 {
                subdiv /= 2;
                quads = m.quads(subdiv);
            }
            let tm = obj.transform.matrix();
            let map = |x: f64, y: f64| -> (f64, f64) {
                (tm[0] * x + tm[2] * y + tm[4], tm[1] * x + tm[3] * y + tm[5])
            };
            // One group per mesh keeps the markup navigable.
            svg.push_str(&format!("  <g{id_attr}{effect_attr}>\n"));
            for (corners, color) in &quads {
                let pts: Vec<(f64, f64)> =
                    corners.iter().map(|p| map(p.x, p.y)).collect();
                let opac = if (color[3] - 1.0).abs() > 1e-3 {
                    format!(" fill-opacity=\"{:.3}\"", color[3].clamp(0.0, 1.0))
                } else {
                    String::new()
                };
                svg.push_str(&format!(
                    "    <path d=\"M{:.2},{:.2}L{:.2},{:.2}L{:.2},{:.2}L{:.2},{:.2}Z\" fill=\"{}\"{} />\n",
                    pts[0].0, pts[0].1, pts[1].0, pts[1].1,
                    pts[2].0, pts[2].1, pts[3].0, pts[3].1,
                    color_to_svg_str(color), opac
                ));
            }
            svg.push_str("  </g>\n");
        }
        ObjectType::TextOnPath {
            text,
            style,
            path: tp,
            start_offset,
            side,
            ..
        } => {
            // Live outlines baked once at export time.
            let outlines = crate::core::text_path::text_on_path_outlines(
                tp,
                text,
                style,
                *start_offset,
                *side,
            );
            let d = path_data_to_d(&outlines, &obj.transform);
            let stroke = obj
                .stroke
                .as_ref()
                .map(|s| {
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"",
                        color_to_svg_str(&s.color),
                        s.width
                    )
                })
                .unwrap_or_default();
            svg.push_str(&format!(
                "  <path{id_attr} d=\"{d}\"{fill_attr}{stroke}{effect_attr} />\n"
            ));
        }
        ObjectType::Envelope { .. } => {
            // Same proxy recursion as canvas: deformed source as Path.
            if let Some(proxy) = obj.envelope_proxy() {
                render_object_to_svg(&proxy, doc, svg, defs, counter);
            }
        }
    }
}

