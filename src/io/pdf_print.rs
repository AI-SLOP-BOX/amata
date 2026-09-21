//! Print PDF export: CMYK/separations, images, shadings, clip, opacity,
//! bleed boxes, crop marks and PDF/X markers.
//!
//! The legacy [`crate::io::pdf::export_pdf`] stays byte-stable for existing
//! flows; this is the press path. CMYK conversion is the documented
//! naive-UCR model (no ICC engine vendored) with total-ink reporting.

use crate::core::document::{ColorMode, Document, Object, ObjectType};
use crate::core::path::{
    FillRule, FillStyle, FillType, LinearGradient, PathData, PathElement, RadialGradient,
    StrokeCap, StrokeJoin,
};
use crate::core::print::{limit_ink, rgb_to_cmyk_ink, total_ink, SpotColor, MAX_TOTAL_INK};
use std::collections::HashMap;
use std::fmt::Write as _;

/// Print export switches.
pub struct PrintPdfOptions {
    /// Crop + registration marks in the slug area.
    pub marks: bool,
    /// Bleed override in points (`None` = document bleed).
    pub bleed: Option<f64>,
    /// PDF/X-1a markers + OutputIntent (Japan Color 2001 Coated).
    pub pdfx: bool,
}

impl Default for PrintPdfOptions {
    fn default() -> Self {
        Self {
            marks: true,
            bleed: None,
            pdfx: true,
        }
    }
}

fn f2(v: f64) -> String {
    format!("{v:.2}")
}

fn f3(v: f32) -> String {
    format!("{v:.3}")
}

fn mat_mul(m1: &[f64; 6], m2: &[f64; 6]) -> [f64; 6] {
    [
        m1[0] * m2[0] + m1[2] * m2[1],
        m1[1] * m2[0] + m1[3] * m2[1],
        m1[0] * m2[2] + m1[2] * m2[3],
        m1[1] * m2[2] + m1[3] * m2[3],
        m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
        m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
    ]
}

/// Resolved paint color in the export space.
enum Paint {
    Rgb([f32; 4]),
    Cmyk([f32; 4]),
    Spot(String, f32),
}

struct Ctx<'a> {
    doc: &'a Document,
    cmyk: bool,
    spots: HashMap<String, SpotColor>,
    gs: HashMap<String, String>,
    gs_next: usize,
    patterns: Vec<String>,
    images: Vec<ImageObj>,
    warnings: Vec<String>,
    warned: std::collections::HashSet<&'static str>,
}

struct ImageObj {
    jpeg: Vec<u8>,
    mask: Option<Vec<u8>>,
    w: u32,
    h: u32,
}

impl<'a> Ctx<'a> {
    fn warn(&mut self, key: &'static str, msg: impl Into<String>) {
        if self.warned.insert(key) {
            self.warnings.push(msg.into());
        }
    }

    fn spot_tint_name(&mut self, spot: &str) -> String {
        format!("SP_{}", pdf_name_escape(spot))
    }

    /// `/CSn cs tint scn` (fill) paint directive for a resolved color.
    fn fill_paint(&mut self, color: [f32; 4], spot: &Option<String>) -> String {
        match self.resolve(color, spot) {
            Paint::Rgb(c) => format!("{} {} {} rg", f3(c[0]), f3(c[1]), f3(c[2])),
            Paint::Cmyk(c) => format!("{} {} {} {} k", f3(c[0]), f3(c[1]), f3(c[2]), f3(c[3])),
            Paint::Spot(name, tint) => {
                let cs = self.spot_tint_name(&name);
                format!("/{cs} cs {} scn", f3(tint))
            }
        }
    }

    fn stroke_paint(&mut self, color: [f32; 4], spot: &Option<String>) -> String {
        match self.resolve(color, spot) {
            Paint::Rgb(c) => format!("{} {} {} RG", f3(c[0]), f3(c[1]), f3(c[2])),
            Paint::Cmyk(c) => format!("{} {} {} {} K", f3(c[0]), f3(c[1]), f3(c[2]), f3(c[3])),
            Paint::Spot(name, tint) => {
                let cs = self.spot_tint_name(&name);
                format!("/{cs} CS {} SCN", f3(tint))
            }
        }
    }

    fn resolve(&mut self, color: [f32; 4], spot: &Option<String>) -> Paint {
        if let Some(name) = spot {
            if self.spots.contains_key(name) {
                return Paint::Spot(name.clone(), 1.0);
            }
            self.warn("spot-missing", format!("特色「{name}」がライブラリにないためプロセス色で出力"));
        }
        if self.cmyk {
            let ink = limit_ink(rgb_to_cmyk_ink(color[0], color[1], color[2]), 4.0);
            // Report-only cap at TAC: export stays faithful, preflight warns.
            let t = total_ink(ink);
            if t > MAX_TOTAL_INK {
                self.warn("tac", "インキ総量320%超の色があります（プリフライト参照）".to_string());
            }
            Paint::Cmyk(ink)
        } else {
            Paint::Rgb(color)
        }
    }

    /// ExtGState for overprint and/or transparency; empty when unneeded.
    fn gs_for(&mut self, overprint: bool, alpha: f32) -> String {
        let mut key = String::new();
        if overprint {
            key.push_str("OP");
        }
        let a = alpha.clamp(0.0, 1.0);
        if a < 0.999 {
            key.push_str(&format!("A{a:.2}"));
        }
        if key.is_empty() {
            return String::new();
        }
        if let Some(n) = self.gs.get(&key) {
            return format!("/{n} gs");
        }
        self.gs_next += 1;
        let name = format!("GS{}", self.gs_next);
        self.gs.insert(key, name.clone());
        format!("/{name} gs")
    }

    fn gs_dict(&self) -> String {
        let mut s = String::from("<< ");
        let mut names: Vec<(&String, &String)> = self.gs.iter().collect();
        names.sort_by(|a, b| a.1.cmp(b.1));
        for (key, name) in names {
            let mut dict = String::from("<< /Type /ExtGState");
            if key.contains("OP") {
                dict.push_str(" /OP true /op true /OPM 1");
            }
            if let Some(pos) = key.find('A') {
                if let Ok(a) = key[pos + 1..].parse::<f32>() {
                    dict.push_str(&format!(" /CA {} /ca {}", f3(a), f3(a)));
                }
            }
            dict.push_str(" >>");
            s.push_str(&format!("/{name} {dict} "));
        }
        s.push_str(">>");
        s
    }
}

fn cap_str(c: &StrokeCap) -> &'static str {
    match c {
        StrokeCap::Butt => "0",
        StrokeCap::Round => "1",
        StrokeCap::Square => "2",
    }
}

fn join_str(j: &StrokeJoin) -> &'static str {
    match j {
        StrokeJoin::Miter => "0",
        StrokeJoin::Round => "1",
        StrokeJoin::Bevel => "2",
    }
}

/// Export a press PDF. Returns bytes plus non-fatal warnings.
pub fn export_pdf_print(doc: &Document, opts: &PrintPdfOptions) -> (Vec<u8>, Vec<String>) {
    let bleed = opts.bleed.unwrap_or(doc.bleed).max(0.0);
    let (tw, th) = (doc.width.max(1.0), doc.height.max(1.0));
    let slug = if opts.marks { 18.0 } else { 0.0 };
    let (mw, mh) = (tw + 2.0 * (bleed + slug), th + 2.0 * (bleed + slug));
    // Media origin offset: trim box sits at (bleed+slug) inside media.
    let ox = bleed + slug;
    let oy = bleed + slug;

    let mut ctx = Ctx {
        doc,
        cmyk: doc.color_mode == ColorMode::Cmyk,
        spots: doc.spots.iter().map(|s| (s.name.clone(), s.clone())).collect(),
        gs: HashMap::new(),
        gs_next: 0,
        patterns: Vec::new(),
        images: Vec::new(),
        warnings: Vec::new(),
        warned: std::collections::HashSet::new(),
    };

    let mut content = String::new();
    let _ = writeln!(content, "q");
    // Media space: origin top-left like the canvas, y-down.
    let _ = writeln!(content, "1 0 0 -1 0 {} cm", f2(mh));
    // Shift so trim (0,0) lands inside media.
    let _ = writeln!(content, "1 0 0 1 {} {} cm", f2(ox), f2(-oy));

    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        for obj in &layer.objects {
            render_obj(&mut ctx, obj, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], &mut content);
        }
    }
    if opts.marks {
        render_marks(&mut ctx, tw, th, ox, oy, &mut content);
    }
    let _ = writeln!(content, "Q");

    // ---- assemble objects ----
    // 1 catalog, 2 pages, 3 page, 4 content, then resources/images/info.
    let mut pdf: Vec<u8> = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets: Vec<usize> = Vec::new();
    let mut obj_no: usize = 0;
    let mut begin = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, obj_no: &mut usize| -> usize {
        *obj_no += 1;
        offsets.push(pdf.len());
        *obj_no
    };

    // Spot color spaces (Separation, exponential tint ramp to the CMYK
    // alt). Only spaces actually referenced by fills/strokes are emitted,
    // so RGB documents stay free of DeviceCMYK.
    let mut used: Vec<String> = Vec::new();
    {
        let mut seen = std::collections::HashSet::new();
        let mut collect = |spot: &Option<String>| {
            if let Some(name) = spot {
                if ctx.spots.contains_key(name) && seen.insert(name.clone()) {
                    used.push(name.clone());
                }
            }
        };
        for layer in &doc.layers {
            for obj in &layer.objects {
                collect(&obj.fill.as_ref().and_then(|f| f.spot.clone()));
                collect(&obj.stroke.as_ref().and_then(|s| s.spot.clone()));
            }
        }
    }
    used.sort();
    let mut spot_cs = String::new();
    {
        for name in &used {
            let alt = ctx.spots[name].cmyk;
            // Only emit spaces actually referenced? Cheap: emit all (few).
            let cs = ctx.spot_tint_name(name);
            let esc_name = pdf_name_escape(name);
            spot_cs.push_str(&format!(
                "/{cs} [/Separation /{esc_name} /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [{} {} {} {}] /N 1 >>] ",
                f3(alt[0]), f3(alt[1]), f3(alt[2]), f3(alt[3])
            ));
        }
    }

    // Patterns (gradient shadings collected during render).
    let mut pattern_res = String::new();
    for (i, pat) in ctx.patterns.iter().enumerate() {
        pattern_res.push_str(&format!("/P{} {} ", i + 1, pat));
    }
    // Images.
    let mut image_res = String::new();
    for i in 0..ctx.images.len() {
        image_res.push_str(&format!("/Im{} {} 0 R ", i + 1, 5 + i));
    }

    let resources = format!(
        "<< /ExtGState {} /Pattern << {}>> /XObject << {}>> /ColorSpace << {}>> >>",
        ctx.gs_dict(),
        pattern_res,
        image_res,
        spot_cs
    );

    // 1 catalog (+ PDF/X + OutputIntent).
    let n1 = begin(&mut pdf, &mut offsets, &mut obj_no);
    debug_assert_eq!(n1, 1);
    let mut catalog = String::from("<< /Type /Catalog /Pages 2 0 R");
    if opts.pdfx {
        catalog.push_str(" /GTS_PDFXVersion (PDF/X-1a:2001)");
        catalog.push_str(&format!(
            " /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier (Japan Color 2001 Coated) /RegistryName (http://www.color.org) /Info (press default) >>]"
        ));
        catalog.push_str(" /Trapped /False");
    }
    catalog.push_str(" >>");
    pdf.extend_from_slice(format!("1 0 obj\n{catalog}\nendobj\n").as_bytes());

    // 2 pages.
    let n2 = begin(&mut pdf, &mut offsets, &mut obj_no);
    debug_assert_eq!(n2, 2);
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    // 3 page with boxes.
    let n3 = begin(&mut pdf, &mut offsets, &mut obj_no);
    debug_assert_eq!(n3, 3);
    pdf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /CropBox [0 0 {} {}] /BleedBox [{} {} {} {}] /TrimBox [{} {} {} {}] /Contents 4 0 R /Resources {} >>\nendobj\n",
            f2(mw), f2(mh), f2(mw), f2(mh),
            f2(slug), f2(slug), f2(mw - slug), f2(mh - slug),
            f2(ox), f2(oy), f2(ox + tw), f2(oy + th),
            resources
        )
        .as_bytes(),
    );

    // 4 content.
    let n4 = begin(&mut pdf, &mut offsets, &mut obj_no);
    debug_assert_eq!(n4, 4);
    let cbytes = content.as_bytes();
    pdf.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", cbytes.len()).as_bytes(),
    );
    pdf.extend_from_slice(cbytes);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    // 5.. images (+masks inline as further objects).
    // Object layout: 1 catalog, 2 pages, 3 page, 4 content, 5.. images,
    // then one mask object per masked image, then info.
    let n_images = ctx.images.len();
    // Pre-compress masks so declared /Length values are exact.
    let mut mask_data: Vec<Option<Vec<u8>>> = Vec::with_capacity(n_images);
    for img in ctx.images.iter() {
        mask_data.push(img.mask.as_ref().map(|m| flate_compress(m)));
    }
    let mut mask_no_of: Vec<Option<usize>> = vec![None; n_images];
    let mut next_no = 5 + n_images;
    for (i, m) in mask_data.iter().enumerate() {
        if m.is_some() {
            mask_no_of[i] = Some(next_no);
            next_no += 1;
        }
    }
    for (i, img) in ctx.images.iter().enumerate() {
        offsets.push(pdf.len());
        debug_assert_eq!(offsets.len(), 4 + i + 1);
        let smask = match mask_no_of[i] {
            Some(mn) => format!(" /SMask {mn} 0 R"),
            None => String::new(),
        };
        pdf.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode{smask} /Length {} >>\nstream\n",
                5 + i, img.w, img.h, img.jpeg.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(&img.jpeg);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
    }
    // Masks (8-bit gray Flate).
    for (i, m) in mask_data.iter().enumerate() {
        if let Some(data) = m {
            offsets.push(pdf.len());
            let img = &ctx.images[i];
            pdf.extend_from_slice(
                format!(
                    "{} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n",
                    mask_no_of[i].unwrap(), img.w, img.h, data.len()
                )
                .as_bytes(),
            );
            pdf.extend_from_slice(data);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
        }
    }

    // Info dict.
    let info_no = next_no;
    offsets.push(pdf.len());
    let info_no = next_no;
    pdf.extend_from_slice(
        format!(
            "{info_no} 0 obj\n<< /Title ({}) /Creator (Amata) /Producer (Amata print export) /CreationDate (D:20260101000000+09'00') >>\nendobj\n",
            sanitize(&ctx.doc.name)
        )
        .as_bytes(),
    );

    // xref.
    let total = next_no + 1; // Size counts object 0.
    let xref_at = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {total}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {total} /Root 1 0 R /Info {info_no} 0 R >>\nstartxref\n{xref_at}\n%%EOF\n").as_bytes(),
    );

    // Sanity: object-number bookkeeping above assumed images start at 5.
    // (debug_asserts guard the layout in debug builds.)
    (pdf, ctx.warnings)
}

fn sanitize(s: &str) -> String {
    // Literal-string escaping for the Info dict.
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_ascii() => out.push(c),
            _ => out.push('?'),
        }
    }
    out
}

/// PDF name object escaping: `#` must be followed by two hex digits.
fn pdf_name_escape(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'_' | b'.') {
            out.push(b as char);
        } else {
            out.push_str(&format!("#{b:02X}"));
        }
    }
    if out.is_empty() {
        out.push_str("Spot");
    }
    out
}

fn flate_compress(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    let _ = enc.write_all(data);
    enc.finish().unwrap_or_default()
}

fn render_obj(ctx: &mut Ctx, obj: &Object, parent: &[f64; 6], out: &mut String) {
    if !obj.visible {
        return;
    }
    let world = mat_mul(parent, &obj.transform.matrix());
    match &obj.object_type {
        ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
            // Masks: first child clips the rest.
            if matches!(obj.object_type, ObjectType::ClippingMask { .. }) && !children.is_empty() {
                let mut clip = children[0].to_path_data();
                clip.transform(&children[0].transform.matrix());
                let _ = writeln!(out, "q");
                emit_path_geom(&clip, &world, out);
                let rule = children[0]
                    .fill
                    .as_ref()
                    .map(|f| f.rule == FillRule::EvenOdd)
                    .unwrap_or(false);
                let _ = writeln!(out, "{} n", if rule { "W*" } else { "W" });
                for child in &children[1..] {
                    render_obj(ctx, child, &world, out);
                }
                let _ = writeln!(out, "Q");
                return;
            }
            for child in children {
                render_obj(ctx, child, &world, out);
            }
        }
        ObjectType::Text { text, style, area, .. } => {
            // Deterministic press text: outlines (fonts never missing).
            let layout = crate::core::document::layout_text(text, style, *area);
            let line_h = style.effective_line_height();
            for (li, line) in layout.lines.iter().take(layout.visible).enumerate() {
                // Anchor per line from real outline width.
                let ol = crate::core::text_path::text_to_outline_path_with_style(line, style);
                let lw = ol
                    .bounding_box()
                    .map(|(mn, mx)| mx.x - mn.x)
                    .unwrap_or(0.0);
                let (ox, _) = layout.origin;
                let ax = match style.text_anchor {
                    crate::core::document::TextAnchor::Start => ox,
                    crate::core::document::TextAnchor::Middle => {
                        area.map(|a| a.x + a.width / 2.0).unwrap_or(ox) - lw / 2.0
                    }
                    crate::core::document::TextAnchor::End => {
                        area.map(|a| a.x + a.width).unwrap_or(ox) - lw
                    }
                };
                let mut moved = ol;
                moved.transform(&[1.0, 0.0, 0.0, 1.0, ax, layout.origin.1 + li as f64 * line_h]);
                moved.transform(&world);
                emit_painted_path(ctx, obj, &moved, out);
            }
        }
        ObjectType::Image { width, height, png_bytes } => {
            emit_image(ctx, obj, *width, *height, png_bytes, &world, out);
        }
        ObjectType::GradientMesh(m) => {
            // Flat quads through the regular painter (CMYK/spots/opacity
            // handled per quad like any solid fill).
            let tm = obj.transform.matrix();
            for (corners, color) in m.quads(6) {
                let mut path = PathData::new();
                for (i, p) in corners.iter().enumerate() {
                    let x = tm[0] * p.x + tm[2] * p.y + tm[4];
                    let y = tm[1] * p.x + tm[3] * p.y + tm[5];
                    if i == 0 {
                        path.push_move_to(x, y);
                    } else {
                        path.push_line_to(x, y);
                    }
                }
                path.elements.push(PathElement::ClosePath);
                path.fill = Some(FillStyle::solid(color));
                path.transform(&world);
                emit_painted_path(ctx, obj, &path, out);
            }
        }
        _ => {
            let mut path = obj.to_path_data();
            path.transform(&world);
            // Object fill wins over the PathData default (matches canvas).
            if let Some(f) = obj.fill.clone().or(path.fill.clone()) {
                path.fill = Some(f);
            }
            if path.stroke.is_none() {
                path.stroke = obj.stroke.clone();
            }
            emit_painted_path(ctx, obj, &path, out);
        }
    }
}

fn emit_path_geom(path: &PathData, world: &[f64; 6], out: &mut String) {
    for el in &path.elements {
        match el {
            PathElement::MoveTo(p) => {
                let (x, y) = apply_world(world, p.x, p.y);
                let _ = writeln!(out, "{} {} m", f2(x), f2(y));
            }
            PathElement::LineTo(p) => {
                let (x, y) = apply_world(world, p.x, p.y);
                let _ = writeln!(out, "{} {} l", f2(x), f2(y));
            }
            PathElement::CurveTo(seg) => {
                // Control points through the same affine map (exact).
                let m = world;
                let t = |x: f64, y: f64| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
                let (c1x, c1y) = t(seg.control1.x, seg.control1.y);
                let (c2x, c2y) = t(seg.control2.x, seg.control2.y);
                let (ex, ey) = t(seg.end.x, seg.end.y);
                let _ = writeln!(
                    out,
                    "{} {} {} {} {} {} c",
                    f2(c1x), f2(c1y), f2(c2x), f2(c2y), f2(ex), f2(ey)
                );
            }
            PathElement::ClosePath => {
                let _ = writeln!(out, "h");
            }
        }
    }
}

fn apply_world(world: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    (
        world[0] * x + world[2] * y + world[4],
        world[1] * x + world[3] * y + world[5],
    )
}

/// Paint a baked path with fill/stroke/gradient/opacity/overprint.
fn emit_painted_path(ctx: &mut Ctx, obj: &Object, path: &PathData, out: &mut String) {
    if path.elements.is_empty() {
        return;
    }
    let _ = writeln!(out, "q");
    // Gradient fills become shading patterns.
    if let Some(fill) = path.fill.as_ref() {
        match &fill.fill_type {
            FillType::Linear(g) => {
                emit_shading_fill(ctx, path, g, out);
                let _ = writeln!(out, "Q");
                // Stroke still paints on top when set.
                if path.stroke.is_some() {
                    let _ = writeln!(out, "q");
                    emit_stroke_only(ctx, obj, path, out);
                    let _ = writeln!(out, "Q");
                }
                return;
            }
            FillType::Radial(g) => {
                emit_radial_shading_fill(ctx, path, g, out);
                let _ = writeln!(out, "Q");
                if path.stroke.is_some() {
                    let _ = writeln!(out, "q");
                    emit_stroke_only(ctx, obj, path, out);
                    let _ = writeln!(out, "Q");
                }
                return;
            }
            _ => {}
        }
    }
    if let Some(fill) = path.fill.as_ref() {
        let g = ctx.gs_for(fill.overprint, obj.opacity * fill.color[3]);
        if !g.is_empty() {
            let _ = writeln!(out, "{g}");
        }
        // Spot fills route through Separation spaces.
        let p = ctx.fill_paint(fill.color, &fill.spot);
        let _ = writeln!(out, "{p}");
    }
    let has_stroke = path.stroke.is_some();
    if let Some(stroke) = path.stroke.as_ref() {
        let g = ctx.gs_for(stroke.overprint, obj.opacity * stroke.color[3]);
        if !g.is_empty() {
            let _ = writeln!(out, "{g}");
        }
        let p = ctx.stroke_paint(stroke.color, &stroke.spot);
        let _ = writeln!(out, "{p}");
        let _ = writeln!(out, "{} w", f2(stroke.width.max(0.05)));
        let _ = writeln!(out, "{} J", cap_str(&stroke.cap));
        let _ = writeln!(out, "{} j", join_str(&stroke.join));
        if let Some(dash) = stroke.dash_pattern.as_ref().filter(|d| !d.is_empty()) {
            let pat = dash.iter().map(|v| f2(*v)).collect::<Vec<_>>().join(" ");
            let _ = writeln!(out, "[{pat}] 0 d");
        }
    }
    emit_path_geom(path, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], out);
    let rule = path
        .fill
        .as_ref()
        .map(|f| f.rule == FillRule::EvenOdd)
        .unwrap_or(false);
    let op = match (path.fill.is_some(), has_stroke) {
        (true, true) => if rule { "B*" } else { "B" },
        (true, false) => if rule { "f*" } else { "f" },
        (false, true) => "S",
        (false, false) => "n",
    };
    let _ = writeln!(out, "{op}");
    let _ = writeln!(out, "Q");
}

fn emit_stroke_only(ctx: &mut Ctx, obj: &Object, path: &PathData, out: &mut String) {
    if let Some(stroke) = path.stroke.as_ref() {
        let g = ctx.gs_for(stroke.overprint, obj.opacity * stroke.color[3]);
        if !g.is_empty() {
            let _ = writeln!(out, "{g}");
        }
        let p = ctx.stroke_paint(stroke.color, &stroke.spot);
        let _ = writeln!(out, "{p}");
        let _ = writeln!(out, "{} w", f2(stroke.width.max(0.05)));
        emit_path_geom(path, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], out);
        let _ = writeln!(out, "S");
    }
}

fn gradient_colors(
    ctx: &mut Ctx,
    stops: &[crate::core::path::GradientStop],
) -> Vec<(f32, [f32; 4])> {
    let mut out: Vec<(f32, [f32; 4])> = stops
        .iter()
        .map(|s| {
            let c = if ctx.cmyk {
                let ink = crate::core::print::rgb_to_cmyk_ink(s.color[0], s.color[1], s.color[2]);
                [ink[0], ink[1], ink[2], ink[3]]
            } else {
                [s.color[0], s.color[1], s.color[2], 1.0]
            };
            (s.offset.clamp(0.0, 1.0), c)
        })
        .collect();
    // Stitching requires ascending knots.
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

fn shading_function(colors: &[(f32, [f32; 4])]) -> String {
    // Stitching (Type 3) over exponential (Type 2) segments.
    if colors.len() < 2 {
        let c = colors.first().map(|(_, c)| *c).unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let v: Vec<String> = c.iter().map(|v| f3(*v)).collect();
        return format!(
            "<< /FunctionType 2 /Domain [0 1] /C0 [{}] /C1 [{}] /N 1 >>",
            v.join(" "),
            v.join(" ")
        );
    }
    let mut funcs = String::new();
    let mut encode = String::new();
    for w in colors.windows(2) {
        let v0: Vec<String> = w[0].1.iter().map(|v| f3(*v)).collect();
        let v1: Vec<String> = w[1].1.iter().map(|v| f3(*v)).collect();
        funcs.push_str(&format!(
            "<< /FunctionType 2 /Domain [0 1] /C0 [{}] /C1 [{}] /N 1 >> ",
            v0.join(" "),
            v1.join(" ")
        ));
        encode.push_str("0 1 ");
    }
    // Bounds holds the N-1 interior knots.
    format!(
        "<< /FunctionType 3 /Domain [0 1] /Functions [{}] /Bounds [{}] /Encode [{}] >>",
        funcs,
        colors[1..colors.len() - 1].iter().map(|(o, _)| f3(*o)).collect::<Vec<_>>().join(" "),
        encode
    )
}

fn emit_shading_fill(
    ctx: &mut Ctx,
    path: &PathData,
    grad: &LinearGradient,
    out: &mut String,
) {
    // Device-space bbox of the baked path drives objectBoundingBox mapping.
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for el in &path.elements {
        match el {
            PathElement::MoveTo(p) | PathElement::LineTo(p) => {
                xs.push(p.x);
                ys.push(p.y);
            }
            PathElement::CurveTo(seg) => {
                xs.extend([seg.control1.x, seg.control2.x, seg.end.x]);
                ys.extend([seg.control1.y, seg.control2.y, seg.end.y]);
            }
            PathElement::ClosePath => {}
        }
    }
    if xs.is_empty() {
        return;
    }
    let (bx0, bx1) = (
        xs.iter().cloned().fold(f64::MAX, f64::min),
        xs.iter().cloned().fold(f64::MIN, f64::max),
    );
    let (by0, by1) = (
        ys.iter().cloned().fold(f64::MAX, f64::min),
        ys.iter().cloned().fold(f64::MIN, f64::max),
    );
    let (bw, bh) = ((bx1 - bx0).max(1.0), (by1 - by0).max(1.0));
    let colors = gradient_colors(ctx, &grad.stops);
    if colors.len() < 2 {
        return;
    }
    let space = if ctx.cmyk { "/DeviceCMYK" } else { "/DeviceRGB" };
    let x1 = bx0 + grad.start_x as f64 * bw;
    let y1 = by0 + grad.start_y as f64 * bh;
    let x2 = bx0 + grad.end_x as f64 * bw;
    let y2 = by0 + grad.end_y as f64 * bh;
    let func = shading_function(&colors);
    ctx.patterns.push(format!(
        "<< /PatternType 2 /Shading << /ShadingType 2 /ColorSpace {space} /Coords [{} {} {} {}] /Function {func} /Extend [true true] >> >>",
        f2(x1), f2(y1), f2(x2), f2(y2)
    ));
    let n = ctx.patterns.len();
    // Uncolored shading pattern + path clip so paint stays inside.
    let rule = path
        .fill
        .as_ref()
        .map(|f| f.rule == FillRule::EvenOdd)
        .unwrap_or(false);
    emit_path_geom(path, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], out);
    let _ = writeln!(out, "{}", if rule { "W* n" } else { "W n" });
    let _ = writeln!(out, "/Pattern cs /P{n} scn");
    emit_path_geom(path, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], out);
    let _ = writeln!(out, "{}", if rule { "f*" } else { "f" });
}

fn emit_radial_shading_fill(
    ctx: &mut Ctx,
    path: &PathData,
    grad: &RadialGradient,
    out: &mut String,
) {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for el in &path.elements {
        match el {
            PathElement::MoveTo(p) | PathElement::LineTo(p) => {
                xs.push(p.x);
                ys.push(p.y);
            }
            PathElement::CurveTo(seg) => {
                xs.extend([seg.control1.x, seg.control2.x, seg.end.x]);
                ys.extend([seg.control1.y, seg.control2.y, seg.end.y]);
            }
            PathElement::ClosePath => {}
        }
    }
    if xs.is_empty() {
        return;
    }
    let (bx0, bx1) = (
        xs.iter().cloned().fold(f64::MAX, f64::min),
        xs.iter().cloned().fold(f64::MIN, f64::max),
    );
    let (by0, by1) = (
        ys.iter().cloned().fold(f64::MAX, f64::min),
        ys.iter().cloned().fold(f64::MIN, f64::max),
    );
    let (bw, bh) = ((bx1 - bx0).max(1.0), (by1 - by0).max(1.0));
    let colors = gradient_colors(ctx, &grad.stops);
    if colors.len() < 2 {
        return;
    }
    let space = if ctx.cmyk { "/DeviceCMYK" } else { "/DeviceRGB" };
    // Circular approximation of the (possibly elliptical) model: outer
    // center at the declared center, inner pinhole at the focus.
    let cx = bx0 + grad.center_x as f64 * bw;
    let cy = by0 + grad.center_y as f64 * bh;
    let fx = bx0 + grad.focus_x as f64 * bw;
    let fy = by0 + grad.focus_y as f64 * bh;
    let r = grad.radius as f64 * bw.max(bh);
    let func = shading_function(&colors);
    ctx.patterns.push(format!(
        "<< /PatternType 2 /Shading << /ShadingType 3 /ColorSpace {space} /Coords [{} {} 0 {} {} {}] /Function {func} /Extend [true true] >> >>",
        f2(fx), f2(fy), f2(cx), f2(cy), f2(r)
    ));
    let n = ctx.patterns.len();
    let rule = path
        .fill
        .as_ref()
        .map(|f| f.rule == FillRule::EvenOdd)
        .unwrap_or(false);
    emit_path_geom(path, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], out);
    let _ = writeln!(out, "{}", if rule { "W* n" } else { "W n" });
    let _ = writeln!(out, "/Pattern cs /P{n} scn");
    emit_path_geom(path, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], out);
    let _ = writeln!(out, "{}", if rule { "f*" } else { "f" });
}

fn emit_image(
    ctx: &mut Ctx,
    obj: &Object,
    w: f64,
    h: f64,
    png_bytes: &[u8],
    world: &[f64; 6],
    out: &mut String,
) {
    let img = match image::load_from_memory(png_bytes) {
        Ok(i) => i.to_rgba8(),
        Err(_) => {
            ctx.warn("image-decode", "配置画像のデコードに失敗しスキップしました");
            return;
        }
    };
    let (iw, ih) = (img.width(), img.height());
    if iw == 0 || ih == 0 || iw * ih > 16_777_216 {
        ctx.warn("image-size", "異常なサイズの画像をスキップしました");
        return;
    }
    // RGB JPEG + gray SMask when alpha is used.
    let mut rgb = Vec::with_capacity((iw * ih) as usize * 3);
    let mut alpha: Vec<u8> = Vec::with_capacity((iw * ih) as usize);
    let mut has_alpha = false;
    for p in img.pixels() {
        rgb.extend_from_slice(&[p[0], p[1], p[2]]);
        alpha.push(p[3]);
        if p[3] != 255 {
            has_alpha = true;
        }
    }
    let mut jpeg = Vec::new();
    {
        use image::codecs::jpeg::JpegEncoder;
        use image::ImageEncoder;
        let mut enc = JpegEncoder::new_with_quality(&mut jpeg, 90);
        if enc
            .write_image(&rgb, iw, ih, image::ExtendedColorType::Rgb8)
            .is_err()
        {
            ctx.warn("image-encode", "JPEG変換に失敗しスキップしました");
            return;
        }
    }
    let mask = if has_alpha {
        Some(flate_compress(&alpha))
    } else {
        None
    };
    ctx.images.push(ImageObj {
        jpeg,
        mask,
        w: iw,
        h: ih,
    });
    let n = ctx.images.len();
    // Axis bbox of the transformed rect (rotation bakes approximately).
    let corners = [
        apply_world(world, 0.0, 0.0),
        apply_world(world, w, 0.0),
        apply_world(world, w, h),
        apply_world(world, 0.0, h),
    ];
    let (lx, rx) = corners
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), (x, _)| (a.min(*x), b.max(*x)));
    let (ty, by) = corners
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), (_, y)| (a.min(*y), b.max(*y)));
    let (bw, bh) = ((rx - lx).max(1.0), (by - ty).max(1.0));
    let g = ctx.gs_for(false, obj.opacity);
    let _ = writeln!(out, "q");
    if !g.is_empty() {
        let _ = writeln!(out, "{g}");
    }
    // y-flipped placement so row 0 lands on top.
    let _ = writeln!(
        out,
        "{} 0 0 {} {} {} cm /Im{n} Do",
        f2(bw),
        f2(-bh),
        f2(lx),
        f2(ty + bh)
    );
    let _ = writeln!(out, "Q");
}

/// Crop + registration marks in the slug area (registration black).
fn render_marks(ctx: &mut Ctx, tw: f64, th: f64, ox: f64, oy: f64, out: &mut String) {
    let black = if ctx.cmyk {
        "0 0 0 1 k".to_string()
    } else {
        "0 0 0 rg".to_string()
    };
    let _ = writeln!(out, "q");
    let _ = writeln!(out, "{black}");
    let _ = writeln!(out, "0.25 w 0 J 0 j");
    let len = 12.0;
    let gap = 6.0;
    // Trim corners in media coords.
    let corners = [(ox, oy), (ox + tw, oy), (ox, oy + th), (ox + tw, oy + th)];
    for (cx, cy) in corners {
        let sx = if cx <= ox + 1.0 { -1.0 } else { 1.0 };
        let sy = if cy <= oy + 1.0 { -1.0 } else { 1.0 };
        // Horizontal tick.
        let _ = writeln!(
            out,
            "{} {} m {} {} l S",
            f2(cx + sx * gap),
            f2(cy),
            f2(cx + sx * (gap + len)),
            f2(cy)
        );
        // Vertical tick.
        let _ = writeln!(
            out,
            "{} {} m {} {} l S",
            f2(cx),
            f2(cy + sy * gap),
            f2(cx),
            f2(cy + sy * (gap + len))
        );
    }
    // Registration targets: left/right center in the slug.
    for (rx, ry) in [(ox - 9.0, oy + th / 2.0), (ox + tw + 9.0, oy + th / 2.0)] {
        let _ = writeln!(
            out,
            "{} {} m {} {} l S",
            f2(rx - 6.0),
            f2(ry),
            f2(rx + 6.0),
            f2(ry)
        );
        let _ = writeln!(
            out,
            "{} {} m {} {} l S",
            f2(rx),
            f2(ry - 6.0),
            f2(rx),
            f2(ry + 6.0)
        );
        let _ = writeln!(
            out,
            "{} {} 4.5 0 360 arc S",
            f2(rx),
            f2(ry)
        );
    }
    let _ = writeln!(out, "Q");
}
