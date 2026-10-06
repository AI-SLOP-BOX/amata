//! Print PDF export: CMYK/separations, images, shadings, clip, opacity,
//! bleed boxes, crop marks, PDF/X markers and the embedded ICC OutputIntent.
//!
//! The legacy [`crate::io::pdf::export_pdf`] stays byte-stable for existing
//! flows; this is the press path. CMYK conversion goes through the ICC engine
//! of [`crate::core::icc`] (with the documented naive-UCR model as its
//! fallback), and PDF/X output carries total-ink reporting.

use crate::core::document::{BlendMode, ColorMode, Document, Object, ObjectType};
use crate::core::geometry::matrix_scale;
use crate::core::icc::rgb_to_cmyk;
use crate::core::path::{
    FillRule, FillStyle, FillType, LinearGradient, PathData, PathElement, RadialGradient,
    StrokeCap, StrokeJoin,
};
use crate::core::print::{limit_ink, total_ink, SpotColor, MAX_TOTAL_INK};
use std::collections::HashMap;
use std::fmt::Write as _;

/// Print export switches.
pub struct PrintPdfOptions {
    /// Crop + registration marks in the slug area.
    pub marks: bool,
    /// Bleed override in points (`None` = document bleed).
    pub bleed: Option<f64>,
    /// PDF/X-1a markers + an OutputIntent whose `/DestOutputProfile` is an
    /// embedded ICC profile (Japan Color 2001 Coated for plates, sRGB for
    /// RGB documents).
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
    /// Opaque Flate CMYK (CMYK documents only; `jpeg`/`mask` stay empty).
    cmyk_flate: Option<Vec<u8>>,
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
            let ink = limit_ink(rgb_to_cmyk([color[0], color[1], color[2], 1.0]), 4.0);
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

    // Transparency flattening for PDF/X: vector objects that carry live
    // transparency are baked into opaque press-DPI raster regions (see
    // `flatten_regions`). Skipped objects must not draw twice.
    let flat = if opts.pdfx {
        flatten_regions(doc, &mut ctx)
    } else {
        Vec::new()
    };
    // Embedded-font collection: gather faces + shaped lines for text
    // eligible for Tj output (strict gate in `embed_candidate`).
    let mut faces: Vec<embed::EmbedFace> = Vec::new();
    let mut face_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut embed_lines: std::collections::HashMap<String, Vec<embed::EmbeddedLine>> =
        std::collections::HashMap::new();
    fn collect_text(
        doc: &Document,
        obj: &Object,
        faces: &mut Vec<embed::EmbedFace>,
        face_index: &mut std::collections::HashMap<String, usize>,
        embed_lines: &mut std::collections::HashMap<String, Vec<embed::EmbeddedLine>>,
    ) {
        if let ObjectType::Group(children) | ObjectType::ClippingMask { children } =
            &obj.object_type
        {
            for c in children {
                collect_text(doc, c, faces, face_index, embed_lines);
            }
            return;
        }
        let (text, style, area) = match &obj.object_type {
            ObjectType::Text {
                text, style, area, ..
            } => (text, style, *area),
            _ => return,
        };
        if !embed_eligible(obj, style) {
            return;
        }
        let key = embed::face_key(style);
        let idx = match face_index.get(&key) {
            Some(&i) => i,
            None => {
                let (_, _, face) = match embed::resolve_face(style) {
                    Some(v) => v,
                    None => return,
                };
                faces.push(face);
                let i = faces.len() - 1;
                face_index.insert(key, i);
                i
            }
        };
        // Thread-aware: linked frames flow the head story instead of
        // laying out their own (possibly stale) text.
        let layout = crate::core::document::layout_text_full(doc, &obj.id, text, style, area);
        let face = &mut faces[idx];
        let mut lines = Vec::new();
        for (li, line) in layout.lines.iter().take(layout.visible).enumerate() {
            let glyphs = match embed::shape_line(&face.data, face.index, line, style) {
                Some(g) => g,
                None => return,
            };
            embed::collect_line(face, line, &glyphs);
            lines.push(embed::EmbeddedLine {
                li,
                face_idx: idx,
                glyphs,
            });
        }
        if lines.iter().any(|l| !l.glyphs.is_empty()) {
            embed_lines.insert(obj.id.clone(), lines);
        }
    }
    for layer in &doc.layers {
        for obj in &layer.objects {
            collect_text(doc, obj, &mut faces, &mut face_index, &mut embed_lines);
        }
    }
    for (i, face) in faces.iter_mut().enumerate() {
        face.res_idx = i + 1;
    }
    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        for obj in &layer.objects {
            if flat.iter().any(|r| region_contains(r, obj)) {
                continue;
            }
            render_obj_embed(
                &mut ctx,
                obj,
                &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                &mut content,
                &faces,
                &embed_lines,
            );
        }
    }
    // Flattened regions draw as opaque press raster on top, in object
    // order (regions were merged so they never overlap each other).
    for (i, r) in flat.iter().enumerate() {
        let _ = writeln!(
            content,
            "q {} 0 0 {} {} {} cm /Fm{} Do Q",
            f2(r.w),
            f2(-r.h),
            f2(r.x),
            f2(r.y + r.h),
            i + 1
        );
    }
    // Missing families outline as generic blocks: flag them per family so
    // the export never silently substitutes glyphs.
    {
        fn collect(obj: &Object, out: &mut Vec<String>) {
            match &obj.object_type {
                ObjectType::Text { style, .. } | ObjectType::TextOnPath { style, .. } => {
                    out.push(style.font_family.clone());
                }
                ObjectType::Group(children) => {
                    for c in children {
                        collect(c, out);
                    }
                }
                ObjectType::ClippingMask { children } => {
                    for c in children {
                        collect(c, out);
                    }
                }
                _ => {}
            }
        }
        let mut families = Vec::new();
        for layer in &doc.layers {
            for obj in &layer.objects {
                collect(obj, &mut families);
            }
        }
        families.sort();
        families.dedup();
        let registry = crate::core::font::FontRegistry::global();
        for family in families {
            if !registry.is_family_available(&family) {
                // Pushed directly (`warn` dedupes by static key): every
                // missing family must surface.
                ctx.warnings.push(format!(
                    "フォント「{family}」未インストール — 代替グリフで出力されます"
                ));
            }
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
    let begin = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, obj_no: &mut usize| -> usize {
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
    // Profile bytes for the PDF/X OutputIntent, fetched before the catalog so
    // its object number can be referenced. PDF/X wants the profile *in* the
    // file: a bare `/OutputConditionIdentifier (Japan Color …)` with no
    // `/DestOutputProfile` claims a press it cannot prove.
    let icc_profile: Option<(Vec<u8>, u8, &'static str)> = if opts.pdfx {
        crate::core::icc::output_intent_icc_bytes(ctx.cmyk).map(|bytes| {
            if ctx.cmyk {
                (bytes, 4, "DeviceCMYK")
            } else {
                (bytes, 3, "DeviceRGB")
            }
        })
    } else {
        None
    };
    // Fixed layout: 1 catalog, 2 pages, 3 page, 4 content, 5 ICC profile when
    // embedded, then images — so images start at 6 or 5 accordingly.
    let img_base = if icc_profile.is_some() { 6 } else { 5 };

    // Images.
    let mut image_res = String::new();
    for i in 0..ctx.images.len() {
        image_res.push_str(&format!("/Im{} {} 0 R ", i + 1, img_base + i));
    }
    // Five objects per face (Type0, CIDFont, Descriptor, FontFile2,
    // ToUnicode) placed after the flattened regions; numbers are analytic
    // so this dict can reference them before assembly.
    let font_base = img_base
        + ctx.images.len()
        + ctx.images.iter().filter(|im| im.mask.is_some()).count()
        + flat.len();
    for (i, face) in faces.iter_mut().enumerate() {
        face.type0_no = font_base + i * 5;
    }
    // Flattened regions sit right after the masks (see assembly).
    let flat_base =
        img_base + ctx.images.len() + ctx.images.iter().filter(|im| im.mask.is_some()).count();
    for (i, _) in flat.iter().enumerate() {
        image_res.push_str(&format!("/Fm{} {} 0 R ", i + 1, flat_base + i));
    }

    let mut font_res = String::new();
    for face in &faces {
        font_res.push_str(&format!("/F{} {} 0 R ", face.res_idx, face.type0_no));
    }
    let resources = format!(
        "<< /ExtGState {} /Pattern << {}>> /XObject << {}>> /Font << {}>> /ColorSpace << {}>> >>",
        ctx.gs_dict(),
        pattern_res,
        image_res,
        font_res,
        spot_cs
    );

    // 1 catalog (+ PDF/X + OutputIntent).
    let n1 = begin(&mut pdf, &mut offsets, &mut obj_no);
    debug_assert_eq!(n1, 1);
    let mut catalog = String::from("<< /Type /Catalog /Pages 2 0 R");
    if opts.pdfx {
        catalog.push_str(" /GTS_PDFXVersion (PDF/X-1a:2001)");
        // The embedded profile is Amata's synthetic UCR model, not a
        // measured press characterization — so it must not claim
        // "Japan Color 2001 Coated" (that assertion fails certified
        // preflight). Custom identifier + honest Info instead; RIPs fall
        // back to the embedded DestOutputProfile, which is present.
        let condition = if ctx.cmyk {
            "Amata UCR Coated (naive)"
        } else {
            "sRGB IEC61966-2.1"
        };
        let info = if ctx.cmyk {
            "synthetic UCR fallback — not a measured press profile"
        } else {
            "sRGB default"
        };
        let dest = if icc_profile.is_some() {
            " /DestOutputProfile 5 0 R"
        } else {
            ""
        };
        catalog.push_str(&format!(
            " /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier ({condition}) /RegistryName (http://www.color.org) /Info ({info}){dest} >>]"
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

    // 5 ICC profile stream (the OutputIntent's /DestOutputProfile).
    if let Some((bytes, n_comp, alternate)) = icc_profile {
        let n5 = begin(&mut pdf, &mut offsets, &mut obj_no);
        debug_assert_eq!(n5, 5);
        let body = flate_compress(&bytes);
        pdf.extend_from_slice(
            format!(
                "5 0 obj\n<< /N {n_comp} /Alternate /{alternate} /Filter /FlateDecode /Length {} >>\nstream\n",
                body.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(&body);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    // 6.. images (+masks inline as further objects).
    // Object layout: 1 catalog, 2 pages, 3 page, 4 content, 5 ICC profile
    // (only when embedded), then images, then one mask object per masked
    // image, then info.
    let n_images = ctx.images.len();
    // Pre-compress masks so declared /Length values are exact.
    let mut mask_data: Vec<Option<Vec<u8>>> = Vec::with_capacity(n_images);
    for img in ctx.images.iter() {
        mask_data.push(img.mask.as_ref().map(|m| flate_compress(m)));
    }
    let mut mask_no_of: Vec<Option<usize>> = vec![None; n_images];
    let mut next_no = img_base + n_images;
    for (i, m) in mask_data.iter().enumerate() {
        if m.is_some() {
            mask_no_of[i] = Some(next_no);
            next_no += 1;
        }
    }
    for (i, img) in ctx.images.iter().enumerate() {
        offsets.push(pdf.len());
        debug_assert_eq!(offsets.len(), img_base + i);
        if let Some(data) = img.cmyk_flate.as_ref() {
            // Opaque Flate CMYK: X-1a clean by construction.
            pdf.extend_from_slice(
                format!(
                    "{} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n",
                    img_base + i, img.w, img.h, data.len()
                )
                .as_bytes(),
            );
            pdf.extend_from_slice(data);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
            continue;
        }
        let smask = match mask_no_of[i] {
            Some(mn) => format!(" /SMask {mn} 0 R"),
            None => String::new(),
        };
        pdf.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode{smask} /Length {} >>\nstream\n",
                img_base + i, img.w, img.h, img.jpeg.len()
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

    // Flattened regions: opaque press raster, Flate (never DCT/SMask,
    // so X-1a stays clean). Numbers follow the masks, matching `flat_base`.
    for (i, r) in flat.iter().enumerate() {
        offsets.push(pdf.len());
        debug_assert_eq!(offsets.len(), flat_base + i);
        let cs = if r.cmyk { "DeviceCMYK" } else { "DeviceRGB" };
        pdf.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /{cs} /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n",
                flat_base + i, r.pw, r.ph, r.data.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(&r.data);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    // Embedded fonts: Type0 + CIDFontType2 + Descriptor + FontFile2 +
    // ToUnicode per face (full font, never subset).
    for (i, face) in faces.iter().enumerate() {
        let base = font_base + i * 5;
        let w_ranges = embed::embed_width_ranges(face);
        // Type0.
        offsets.push(pdf.len());
        debug_assert_eq!(offsets.len(), base);
        pdf.extend_from_slice(
            format!(
                "{base} 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /{ps} /Encoding /Identity-H /DescendantFonts [{cid} 0 R] /ToUnicode {cmap} 0 R >>\nendobj\n",
                ps = face.ps_name,
                cid = base + 1,
                cmap = base + 4,
            )
            .as_bytes(),
        );
        // CIDFontType2 with Identity map (CID == GID).
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{cid} 0 obj\n<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{ps} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 1000 /W [{w}] /FontDescriptor {desc} 0 R /CIDToGIDMap /Identity >>\nendobj\n",
                cid = base + 1,
                ps = face.ps_name,
                w = w_ranges,
                desc = base + 2,
            )
            .as_bytes(),
        );
        // Descriptor.
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{desc} 0 obj\n<< /Type /FontDescriptor /FontName /{ps} /Flags 4 /FontBBox [{bb}] /ItalicAngle 0 /Ascent {asc} /Descent {des} /CapHeight {cap} /StemV 80 /FontFile2 {file} 0 R >>\nendobj\n",
                desc = base + 2,
                ps = face.ps_name,
                bb = face
                    .bbox_1000
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                asc = face.ascent_1000,
                des = face.descent_1000,
                cap = face.cap_1000,
                file = base + 3,
            )
            .as_bytes(),
        );
        // Full font bytes (never subset: searchability and fidelity over size).
        offsets.push(pdf.len());
        let comp = flate_compress(&face.data);
        pdf.extend_from_slice(
            format!(
                "{file} 0 obj\n<< /Length {n} /Filter /FlateDecode >>\nstream\n",
                file = base + 3,
                n = comp.len(),
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(&comp);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
        // ToUnicode.
        offsets.push(pdf.len());
        let cmap = embed::embed_tounicode_cmap(face);
        pdf.extend_from_slice(
            format!(
                "{cmap_no} 0 obj\n<< /Length {n} >>\nstream\n",
                cmap_no = base + 4,
                n = cmap.len(),
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(cmap.as_bytes());
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    // Info dict.
    offsets.push(pdf.len());
    let info_no = next_no + flat.len() + faces.len() * 5;
    pdf.extend_from_slice(
        format!(
            "{info_no} 0 obj\n<< /Title ({}) /Creator (Amata) /Producer (Amata print export) /CreationDate (D:20260101000000+09'00') >>\nendobj\n",
            sanitize(&ctx.doc.name)
        )
        .as_bytes(),
    );

    // xref.
    let total = next_no + flat.len() + faces.len() * 5 + 1; // Size counts object 0.
    let xref_at = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {total}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {total} /Root 1 0 R /Info {info_no} 0 R >>\nstartxref\n{xref_at}\n%%EOF\n").as_bytes(),
    );

    // Sanity: object-number bookkeeping above assumed images start right
    // after the content (and ICC) objects.  (debug_asserts guard the layout
    // in debug builds.)
    if opts.pdfx {
        // The GTS flag above is a claim — verify it against the bytes we
        // just wrote so a violating document can never leave here tagged
        // X-1a without a loud warning attached. (`warn` dedupes by key, so
        // push directly: every violation must surface.)
        for v in validate_pdfx(&pdf, ctx.cmyk) {
            ctx.warnings.push(format!("PDF/X-1a違反: {v}"));
        }
    }
    (pdf, ctx.warnings)
}

/// Post-export PDF/X-1a check over the generated bytes (lopdf parse).
///
/// Walks ExtGState/Image/Shading dicts (streams included) and reports the
/// X-1a kill list: sub-1.0 CA/ca, non-Normal BM, SMask, RGB in CMYK docs.
/// Nothing is rewritten: our exporter owns the content, so a violation is
/// reported, never silently papered over (rewriting alpha would change the
/// look of translucent objects).
pub fn validate_pdfx(pdf: &[u8], cmyk: bool) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(doc) = lopdf::Document::load_mem(pdf) else {
        return vec!["PDFを解析できませんでした".to_string()];
    };
    let name_is = |obj: &lopdf::Object, want: &[u8]| -> bool {
        matches!(obj, lopdf::Object::Name(n) if n.as_slice() == want)
    };
    let num = |obj: &lopdf::Object| -> Option<f64> {
        match obj {
            lopdf::Object::Real(f) => Some(*f as f64),
            lopdf::Object::Integer(i) => Some(*i as f64),
            _ => None,
        }
    };
    let mut live_alpha = 0usize;
    let mut blend = 0usize;
    let mut smask = 0usize;
    let mut rgb_images = 0usize;
    let mut rgb_shadings = 0usize;
    for obj in doc.objects.values() {
        // Image XObjects / SMask masks are streams; everything else is a
        // plain dictionary. Both carry the keys we audit.
        let dict = match obj {
            lopdf::Object::Dictionary(dict) => Some(dict),
            lopdf::Object::Stream(stream) => Some(&stream.dict),
            _ => None,
        };
        let Some(dict) = dict else {
            continue;
        };
        let is_extgs = dict
            .get(b"Type")
            .map(|t| name_is(t, b"ExtGState"))
            .unwrap_or(false)
            || dict.get(b"CA").is_ok()
            || dict.get(b"ca").is_ok()
            || dict.get(b"BM").is_ok()
            || dict.get(b"SMask").is_ok();
        if is_extgs {
            for key in [b"CA".as_slice(), b"ca".as_slice()] {
                if let Ok(v) = dict.get(key) {
                    if num(v).is_some_and(|f| f < 1.0) {
                        live_alpha += 1;
                    }
                }
            }
            if let Ok(bm) = dict.get(b"BM") {
                if !name_is(bm, b"Normal") {
                    blend += 1;
                }
            }
            if let Ok(sm) = dict.get(b"SMask") {
                if !name_is(sm, b"None") {
                    smask += 1;
                }
            }
        }
        let is_image = dict
            .get(b"Subtype")
            .map(|t| name_is(t, b"Image"))
            .unwrap_or(false);
        if is_image {
            if let Ok(cs) = dict.get(b"ColorSpace") {
                if name_is(cs, b"DeviceRGB") && cmyk {
                    rgb_images += 1;
                }
            }
            if dict.get(b"SMask").is_ok() {
                smask += 1;
            }
        }
        if dict.get(b"ShadingType").is_ok() {
            if let Ok(cs) = dict.get(b"ColorSpace") {
                if name_is(cs, b"DeviceRGB") && cmyk {
                    rgb_shadings += 1;
                }
            }
        }
    }
    if live_alpha > 0 {
        out.push(format!(
            "{live_alpha}件の半透明ExtGState — X-1aは透明禁止（要フラット化）"
        ));
    }
    if blend > 0 {
        out.push(format!("{blend}件の非Normalブレンド — X-1aは透明禁止"));
    }
    if smask > 0 {
        out.push(format!("{smask}件のソフトマスク — X-1aは透明禁止"));
    }
    if rgb_images > 0 {
        out.push(format!("{rgb_images}件のRGB画像 — CMYK文書では要変換"));
    }
    if rgb_shadings > 0 {
        out.push(format!(
            "{rgb_shadings}件のRGBシェーディング — CMYK文書では要変換"
        ));
    }
    out
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

/// One baked transparency region: opaque press raster in document coords.
struct FlatRegion {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    /// Flate-compressed raw samples (RGB or CMYK, no alpha, no mask).
    data: Vec<u8>,
    pw: u32,
    ph: u32,
    cmyk: bool,
}

/// Press raster density for flattened regions.
const FLATTEN_DPI: f64 = 300.0;
const FLATTEN_MAX_PX: f32 = 8192.0;
/// Pad around bboxes: strokes, glows and shadows paint outside the
/// geometry box, and clipping them would show.
const FLATTEN_PAD: f64 = 16.0;

fn subtree_needs_flatten(obj: &Object) -> bool {
    // Object-level transparency.
    if obj.opacity < 1.0
        || obj.blend_mode != BlendMode::Normal
        || obj.shadow.is_some()
        || obj.glow.is_some()
    {
        return true;
    }
    // Paint-level alpha: translucent fills/strokes (incl. gradient stops)
    // also emit live ExtGState alpha.
    fn paint_alpha(obj: &Object) -> bool {
        let fill_alpha = obj.fill.as_ref().map(|f| {
            let mut a = f.color[3];
            match &f.fill_type {
                crate::core::path::FillType::Linear(g) => {
                    for s in &g.stops {
                        a = a.min(s.color[3]);
                    }
                }
                crate::core::path::FillType::Radial(g) => {
                    for s in &g.stops {
                        a = a.min(s.color[3]);
                    }
                }
                _ => {}
            }
            a
        });
        let stroke_alpha = obj.stroke.as_ref().map(|s| s.color[3]);
        fill_alpha.is_some_and(|a| a < 1.0) || stroke_alpha.is_some_and(|a| a < 1.0)
    }
    if paint_alpha(obj) {
        return true;
    }
    match &obj.object_type {
        ObjectType::Group(children) | ObjectType::ClippingMask { children } => {
            children.iter().any(subtree_needs_flatten)
        }
        _ => false,
    }
}

fn obj_flat_box(obj: &Object) -> Option<(f64, f64, f64, f64)> {
    obj.bounding_box().map(|(a, b)| {
        (
            a.x.min(b.x) - FLATTEN_PAD,
            a.y.min(b.y) - FLATTEN_PAD,
            a.x.max(b.x) + FLATTEN_PAD,
            a.y.max(b.y) + FLATTEN_PAD,
        )
    })
}

fn boxes_overlap(a: &(f64, f64, f64, f64), b: &(f64, f64, f64, f64)) -> bool {
    a.0 <= b.2 && b.0 <= a.2 && a.1 <= b.3 && b.1 <= a.3
}

fn region_contains(r: &FlatRegion, obj: &Object) -> bool {
    match obj_flat_box(obj) {
        Some((x0, y0, x1, y1)) => {
            x0 >= r.x - 1e-6 && y0 >= r.y - 1e-6 && x1 <= r.x + r.w + 1e-6 && y1 <= r.y + r.h + 1e-6
        }
        None => false,
    }
}

/// Bake every transparency-carrying area into opaque press raster.
///
/// Only used for the PDF/X path (plain PDFs keep live transparency, which
/// they support natively). Merges each transparent object with everything
/// it overlaps into whole-object regions, rasterizes the page once at
/// press DPI on white, and crops per region — so overlaps composite
/// exactly as on canvas and no vector object draws twice. Returns empty
/// (with a warning) when rasterization is impossible; the caller then
/// falls back to vector output and the X-1a gate refuses the file.
fn flatten_regions(doc: &Document, ctx: &mut Ctx) -> Vec<FlatRegion> {
    let mut seeds: Vec<(f64, f64, f64, f64)> = Vec::new();
    let mut all: Vec<(f64, f64, f64, f64)> = Vec::new();
    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        for obj in &layer.objects {
            if !obj.visible {
                continue;
            }
            let Some(b) = obj_flat_box(obj) else {
                continue;
            };
            all.push(b);
            if subtree_needs_flatten(obj) {
                seeds.push(b);
            }
        }
    }
    if seeds.is_empty() {
        return Vec::new();
    }
    // Absorb everything each seed touches, to fixpoint: regions always
    // contain whole objects, so skipping vector output inside them can
    // never double-draw or drop an edge.
    let mut regions = seeds;
    loop {
        let mut grown = false;
        let mut rest: Vec<(f64, f64, f64, f64)> = Vec::new();
        for b in all.drain(..) {
            if regions.iter().any(|r| boxes_overlap(r, &b)) {
                for r in regions.iter_mut() {
                    if boxes_overlap(r, &b) {
                        r.0 = r.0.min(b.0);
                        r.1 = r.1.min(b.1);
                        r.2 = r.2.max(b.2);
                        r.3 = r.3.max(b.3);
                    }
                }
                grown = true;
            } else {
                rest.push(b);
            }
        }
        all = rest;
        // Regions that now touch each other merge as well.
        let mut merged: Vec<(f64, f64, f64, f64)> = Vec::new();
        for b in regions.drain(..) {
            if let Some(r) = merged.iter_mut().find(|r| boxes_overlap(r, &b)) {
                r.0 = r.0.min(b.0);
                r.1 = r.1.min(b.1);
                r.2 = r.2.max(b.2);
                r.3 = r.3.max(b.3);
                grown = true;
            } else {
                merged.push(b);
            }
        }
        regions = merged;
        if !grown {
            break;
        }
    }
    if regions.len() > 32 {
        ctx.warn(
            "flatten-many",
            "透明領域が多すぎるためフラット化を中止しました",
        );
        return Vec::new();
    }
    // Full-page raster once, on white (opaque ⇒ no SMask, X-1a clean).
    let longest = doc.width.max(doc.height).max(1.0) as f32;
    let mut scale = (FLATTEN_DPI / 72.0) as f32;
    if longest * scale > FLATTEN_MAX_PX {
        scale = FLATTEN_MAX_PX / longest;
    }
    if scale < 0.1 {
        ctx.warn(
            "flatten-scale",
            "用紙が大きすぎるためフラット化を中止しました",
        );
        return Vec::new();
    }
    let png =
        match crate::io::raster::export_png_with_limit(doc, scale, false, FLATTEN_MAX_PX as u32) {
            Ok(p) => p,
            Err(e) => {
                ctx.warn("flatten-raster", format!("ラスタライズに失敗: {e}"));
                return Vec::new();
            }
        };
    let img = match image::load_from_memory(&png) {
        Ok(i) => i.to_rgba8(),
        Err(_) => {
            ctx.warn("flatten-decode", "ラスタのデコードに失敗しました");
            return Vec::new();
        }
    };
    let (iw, ih) = (img.width() as f64, img.height() as f64);
    let sx = iw / doc.width.max(1.0);
    let sy = ih / doc.height.max(1.0);
    let mut out = Vec::new();
    let mut flat_count = 0usize;
    for (x0, y0, x1, y1) in regions {
        let cx0 = (x0.max(0.0) * sx).round().clamp(0.0, iw - 1.0) as u32;
        let cy0 = (y0.max(0.0) * sy).round().clamp(0.0, ih - 1.0) as u32;
        let cx1 = (x1.max(0.0) * sx).round().clamp(1.0, iw) as u32;
        let cy1 = (y1.max(0.0) * sy).round().clamp(1.0, ih) as u32;
        if cx1 <= cx0 || cy1 <= cy0 {
            continue;
        }
        let pw = cx1 - cx0;
        let ph = cy1 - cy0;
        if pw * ph > 16_777_216 {
            ctx.warn("flatten-huge", "領域が大きすぎるため一部をベクタ出力します");
            continue;
        }
        let crop = image::imageops::crop_imm(&img, cx0, cy0, pw, ph).to_image();
        let mut raw = Vec::with_capacity((pw * ph) as usize * 4);
        if ctx.cmyk {
            for p in crop.pixels() {
                let c = crate::core::print::rgb_to_cmyk_ink(
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                );
                raw.extend_from_slice(&[
                    (c[0] * 255.0) as u8,
                    (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8,
                    (c[3] * 255.0) as u8,
                ]);
            }
        } else {
            for p in crop.pixels() {
                raw.extend_from_slice(&[p[0], p[1], p[2]]);
            }
        }
        flat_count += 1;
        out.push(FlatRegion {
            x: x0.max(0.0),
            y: y0.max(0.0),
            w: (x1.max(0.0) - x0.max(0.0)).max(1.0),
            h: (y1.max(0.0) - y0.max(0.0)).max(1.0),
            data: flate_compress(&raw),
            pw,
            ph,
            cmyk: ctx.cmyk,
        });
    }
    if !out.is_empty() {
        ctx.warn(
            "flatten",
            format!(
                "透明{flat_count}領域を{:.0}dpiでフラット化ラスタライズしました",
                sx * 72.0
            ),
        );
    }
    out
}

/// Embedded-font text for press PDFs: full-font CIDFontType2 (never
/// subset) + Identity map + ToUnicode, so text stays selectable instead of
/// being outlined. Eligibility is strict — anything exotic (area wrap is
/// fine; vertical/variable/faux/slanted faces, non-solid fills, strokes,
/// translucency) keeps the old outline path, which already handles it.
/// Outlined and embedded output share the same HarfBuzz shaping, so metrics
/// agree by construction.
mod embed {

    /// One shaped line ready for Tj emission.
    pub struct EmbeddedLine {
        pub li: usize,
        pub face_idx: usize,
        pub glyphs: Vec<crate::core::text_path::ShapedGlyph>,
    }

    pub struct EmbedFace {
        pub ps_name: String,
        pub data: Vec<u8>,
        pub index: u32,
        pub upem: u32,
        pub gids: std::collections::BTreeSet<u32>,
        /// GID -> unicode scalar for ToUnicode (first cluster wins).
        pub uni: std::collections::HashMap<u32, u32>,
        pub ascent_1000: i32,
        pub descent_1000: i32,
        pub cap_1000: i32,
        pub bbox_1000: [i32; 4],
        /// Assigned object numbers at assembly.
        pub type0_no: usize,
        pub res_idx: usize,
    }

    pub fn face_key(style: &crate::core::document::TextStyle) -> String {
        format!(
            "{}|{}|{:?}",
            style.font_family, style.font_weight, style.font_style
        )
    }

    fn ps_sanitize(s: &str) -> String {
        let mut out: String = s
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if out.is_empty() {
            out = "AmataFont".to_string();
        }
        out.truncate(60);
        out
    }

    /// Resolve face bytes + metrics. `None` = caller keeps outlines.
    pub fn resolve_face(
        style: &crate::core::document::TextStyle,
    ) -> Option<(Vec<u8>, u32, EmbedFace)> {
        if style.vertical || !style.variations.is_empty() {
            return None;
        }
        let registry = crate::core::font::FontRegistry::global();
        if registry.needs_synthetic_style(&style.font_family, style.font_weight, style.font_style) {
            return None;
        }
        let (data, index) = registry.query_face_data(
            &style.font_family,
            style.font_weight,
            style.font_style,
            |d, i| (d.to_vec(), i),
        )?;
        let face = ttf_parser::Face::parse(&data, index).ok()?;
        let upem = face.units_per_em() as u32;
        if upem == 0 {
            return None;
        }
        let q = |v: i16| (v as f32 * 1000.0 / upem as f32).round() as i32;
        let (ascent, descent) = (face.ascender() as f32, face.descender() as f32);
        let cap_1000 = face
            .tables()
            .os2
            .as_ref()
            .and_then(|os2| os2.capital_height())
            .map(q)
            .unwrap_or(700);
        let (x0, y0, x1, y1) = (
            face.global_bounding_box().x_min,
            face.global_bounding_box().y_min,
            face.global_bounding_box().x_max,
            face.global_bounding_box().y_max,
        );
        let ps_name = ps_sanitize(&style.font_family);
        Some((
            data.clone(),
            index,
            EmbedFace {
                ps_name,
                data,
                index,
                upem,
                gids: Default::default(),
                uni: Default::default(),
                ascent_1000: (ascent * 1000.0 / upem as f32).round() as i32,
                descent_1000: (descent * 1000.0 / upem as f32).round() as i32,
                cap_1000,
                bbox_1000: [q(x0), q(y0), q(x1), q(y1)],
                type0_no: 0,
                res_idx: 0,
            },
        ))
    }

    /// Shape one line for embedding. Returns glyph runs with font-unit
    /// advances; `None` = outlines fallback (unshapable, vertical offsets
    /// that TJ cannot express, .notdef).
    pub fn shape_line(
        data: &[u8],
        index: u32,
        line: &str,
        style: &crate::core::document::TextStyle,
    ) -> Option<Vec<crate::core::text_path::ShapedGlyph>> {
        let shaped = crate::core::text_path::shape_run_hb(
            data,
            index,
            line,
            style.ligatures,
            &style.variations,
        )?;
        for g in &shaped {
            if g.y_offset != 0.0 {
                return None;
            }
            let ch = line.get(g.cluster as usize..)?.chars().next()?;
            if g.gid == 0 && ch != ' ' && ch != '\t' {
                return None;
            }
        }
        Some(shaped)
    }

    /// Register glyph usage from a shaped line (widths + ToUnicode map).
    pub fn collect_line(
        face: &mut EmbedFace,
        line: &str,
        shaped: &[crate::core::text_path::ShapedGlyph],
    ) {
        for g in shaped {
            face.gids.insert(g.gid);
            face.uni.entry(g.gid).or_insert_with(|| {
                line.get(g.cluster as usize..)
                    .and_then(|s| s.chars().next())
                    .map(|ch| ch as u32)
                    .unwrap_or(0xFFFD)
            });
        }
    }

    /// W array over consecutive used-GID runs (`gid [w ...]`).
    pub fn embed_width_ranges(face: &EmbedFace) -> String {
        let mut gids: Vec<u32> = face.gids.iter().copied().collect();
        gids.sort_unstable();
        let mut out = String::new();
        let mut i = 0;
        while i < gids.len() {
            let start = gids[i];
            let mut run = vec![gid_width_1000(face, start)];
            let mut j = i + 1;
            while j < gids.len() && gids[j] == gids[j - 1] + 1 {
                run.push(gid_width_1000(face, gids[j]));
                j += 1;
            }
            use std::fmt::Write as _;
            let _ = write!(
                out,
                "{} [{}] ",
                start,
                run.iter()
                    .map(|w| w.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            i = j;
        }
        out
    }

    /// ToUnicode CMap (GID-as-CID -> UTF-16BE), 100 entries per block.
    pub fn embed_tounicode_cmap(face: &EmbedFace) -> String {
        let mut entries: Vec<(u32, u32)> = face.uni.iter().map(|(g, u)| (*g, *u)).collect();
        entries.sort_unstable();
        let mut out = String::from(
            "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n/CMapName /Amata-Identity def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
        );
        use std::fmt::Write as _;
        for chunk in entries.chunks(100) {
            let _ = writeln!(out, "{} beginbfchar", chunk.len());
            for (gid, uni) in chunk {
                let mut utf16 = [0u16; 2];
                let enc = char::from_u32(*uni)
                    .unwrap_or('\u{FFFD}')
                    .encode_utf16(&mut utf16);
                let hex: String = enc.iter().map(|u| format!("{u:04X}")).collect();
                let _ = writeln!(out, "<{gid:04X}> <{hex}>");
            }
            out.push_str("endbfchar\n");
        }
        out.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
        out
    }

    pub fn gid_width_1000(face: &EmbedFace, gid: u32) -> u32 {
        let ttf = match ttf_parser::Face::parse(&face.data, face.index) {
            Ok(f) => f,
            Err(_) => return 1000,
        };
        let adv = ttf
            .glyph_hor_advance(ttf_parser::GlyphId(gid.min(0xFFFF) as u16))
            .unwrap_or(face.upem as u16);
        (adv as f32 * 1000.0 / face.upem as f32).round().max(0.0) as u32
    }
}

/// Strict gate for embedded-font text: anything exotic keeps outlines.
/// Outlines already handle gradients, strokes, translucency, vertical and
/// synthetic faces; embedding covers the common solid-fill case.
fn embed_eligible(obj: &Object, style: &crate::core::document::TextStyle) -> bool {
    if style.vertical || !style.variations.is_empty() {
        return false;
    }
    if obj.opacity < 1.0
        || obj.blend_mode != BlendMode::Normal
        || obj.shadow.is_some()
        || obj.glow.is_some()
        || obj.stroke.is_some()
    {
        return false;
    }
    matches!(
        obj.fill.as_ref().map(|f| &f.fill_type),
        Some(crate::core::path::FillType::Solid(c)) if c[3] >= 1.0
    )
}

#[allow(clippy::too_many_arguments)]
fn emit_embedded_text(
    ctx: &mut Ctx,
    obj: &Object,
    style: &crate::core::document::TextStyle,
    _area: Option<crate::core::document::TextArea>,
    world: &[f64; 6],
    faces: &[embed::EmbedFace],
    lines: &[embed::EmbeddedLine],
    layout: &crate::core::document::TextLayout,
    out: &mut String,
) {
    use crate::core::document::TextAnchor;
    let line_h = style.effective_line_height();
    let fill = match obj.fill.as_ref() {
        Some(f) => f,
        None => return,
    };
    let paint = ctx.fill_paint(fill.color, &fill.spot);
    let gs = ctx.gs_for(fill.overprint, 1.0);
    let s = style.font_size;
    let tc = if s > 0.0 {
        style.letter_spacing / s * 1000.0
    } else {
        0.0
    };
    let _ = writeln!(out, "q");
    let _ = writeln!(out, "{paint}");
    if !gs.is_empty() {
        let _ = writeln!(out, "{gs}");
    }
    for eline in lines {
        let face = &faces[eline.face_idx];
        let upem = face.upem as f64;
        if upem <= 0.0 {
            continue;
        }
        let scale = s / upem;
        let n = eline.glyphs.len();
        if n == 0 {
            continue;
        }
        // Anchor from shaped advances (same shaping as outlines).
        let mut adv_sum = 0.0;
        for g in &eline.glyphs {
            adv_sum += g.x_advance as f64;
        }
        let lw = adv_sum * scale + style.letter_spacing * (n as f64 - 1.0).max(0.0);
        let (ox, _) = layout.origin;
        let col = layout.col_of_line.get(eline.li).copied().unwrap_or(0);
        let col_x = layout.col_x.get(col).copied().unwrap_or(ox);
        let col_w = layout.col_w.get(col).copied().unwrap_or(f64::MAX);
        let col_finite = col_w.is_finite() && col_w < f64::MAX / 2.0;
        let indent = layout.line_indent.get(eline.li).copied().unwrap_or(0.0);
        let xoff = layout.line_xoff.get(eline.li).copied().unwrap_or(0.0);
        let ax = match style.text_anchor {
            TextAnchor::Start => col_x + indent + xoff,
            TextAnchor::Middle => {
                col_x
                    + indent
                    + xoff
                    + if col_finite {
                        (col_w - lw) / 2.0
                    } else {
                        -lw / 2.0
                    }
            }
            TextAnchor::End => col_x + indent + xoff + if col_finite { col_w - lw } else { -lw },
        };
        let ay = layout.origin.1 + eline.li as f64 * line_h;
        // Tm = world * T(ax,ay) * S(s,-s): content stream is y-flipped,
        // so glyphs (y-up in text space) land upright.
        let (a, b, c, d, e, f) = (world[0], world[1], world[2], world[3], world[4], world[5]);
        let ex = a * ax + c * ay + e;
        let ey = b * ax + d * ay + f;
        // TJ array with kerning corrections (shaped minus nominal).
        let mut tj = String::from("[");
        for (i, g) in eline.glyphs.iter().enumerate() {
            use std::fmt::Write as _;
            let _ = write!(tj, "<{:04X}>", g.gid.min(0xFFFF));
            if i + 1 < n {
                let nom = embed::gid_width_1000(face, g.gid) as f64 * upem / 1000.0;
                let next = &eline.glyphs[i + 1];
                let adj = (g.x_advance as f64 - nom) + (next.x_offset as f64 - g.x_offset as f64);
                let tjn = -adj * 1000.0 / upem;
                if tjn.abs() >= 0.5 {
                    let _ = write!(tj, " {} ", tjn.round() as i32);
                } else {
                    tj.push(' ');
                }
            }
        }
        tj.push_str("] TJ");
        let _ = writeln!(
            out,
            "BT /F{} {} Tf {} Tc {} {} {} {} {} {} Tm {tj} ET",
            face.res_idx,
            f2(s),
            f2(tc),
            f2(a * s),
            f2(b * s),
            f2(-c * s),
            f2(-d * s),
            f2(ex),
            f2(ey),
        );
    }
    let _ = writeln!(out, "Q");
}

fn render_obj_embed(
    ctx: &mut Ctx,
    obj: &Object,
    parent: &[f64; 6],
    out: &mut String,
    faces: &[embed::EmbedFace],
    embed_lines: &std::collections::HashMap<String, Vec<embed::EmbeddedLine>>,
) {
    if !obj.visible {
        return;
    }
    let world = mat_mul(parent, &obj.transform.matrix());
    // Paths are baked through `world`, so a stroke width kept in the saved
    // units would draw `scale` times thinner than the canvas (which widens
    // the rings with the same transform).
    let scale = matrix_scale(&world);
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
                    render_obj_embed(ctx, child, &world, out, faces, embed_lines);
                }
                let _ = writeln!(out, "Q");
                return;
            }
            for child in children {
                render_obj_embed(ctx, child, &world, out, faces, embed_lines);
            }
        }
        ObjectType::Text {
            text: _,
            style,
            area,
            ..
        } => {
            // Embedded text when eligible (selectable, compact); outlines
            // otherwise (fonts never missing).
            if let Some(lines) = embed_lines.get(&obj.id) {
                let layout = match &obj.object_type {
                    ObjectType::Text { text, .. } => crate::core::document::layout_text_full(
                        ctx.doc, &obj.id, text, style, *area,
                    ),
                    _ => return,
                };
                emit_embedded_text(ctx, obj, style, *area, &world, faces, lines, &layout, out);
                return;
            }
            // Deterministic press text: outlines (fonts never missing).
            // Thread-aware like every other text consumer.
            let layout = match &obj.object_type {
                ObjectType::Text { text, .. } => {
                    crate::core::document::layout_text_full(ctx.doc, &obj.id, text, style, *area)
                }
                _ => return,
            };
            let line_h = style.effective_line_height();
            for (li, line) in layout.lines.iter().take(layout.visible).enumerate() {
                // Anchor per line from real outline width.
                let ol = crate::core::text_path::text_to_outline_path_with_style(line, style);
                let lw = ol.bounding_box().map(|(mn, mx)| mx.x - mn.x).unwrap_or(0.0);
                let (ox, _) = layout.origin;
                let col = layout.col_of_line.get(li).copied().unwrap_or(0);
                let col_x = layout.col_x.get(col).copied().unwrap_or(ox);
                let col_w = layout.col_w.get(col).copied().unwrap_or(f64::MAX);
                let col_finite = col_w.is_finite() && col_w < f64::MAX / 2.0;
                let indent = layout.line_indent.get(li).copied().unwrap_or(0.0);
                let xoff = layout.line_xoff.get(li).copied().unwrap_or(0.0);
                let ax = match style.text_anchor {
                    crate::core::document::TextAnchor::Start => col_x + indent + xoff,
                    crate::core::document::TextAnchor::Middle => {
                        col_x
                            + indent
                            + xoff
                            + if col_finite {
                                (col_w - lw) / 2.0
                            } else {
                                -lw / 2.0
                            }
                    }
                    crate::core::document::TextAnchor::End => {
                        col_x + indent + xoff + if col_finite { col_w - lw } else { -lw }
                    }
                };
                let mut moved = ol;
                moved.transform(&[1.0, 0.0, 0.0, 1.0, ax, layout.origin.1 + li as f64 * line_h]);
                moved.transform(&world);
                emit_painted_path(ctx, obj, &moved, scale, out);
            }
        }
        ObjectType::Image { width, height, png_bytes } => {
            emit_image(ctx, obj, *width, *height, png_bytes, &world, out);
        }
        ObjectType::GradientMesh(m) => {
            // Flat quads through the regular painter (CMYK/spots/opacity
            // handled per quad like any solid fill).
            // Corners go through `world` only — applying obj.transform on top
            // of world double-transformed every mesh.
            for (corners, color) in m.quads(6) {
                let mut path = PathData::new();
                for (i, p) in corners.iter().enumerate() {
                    let x = world[0] * p.x + world[2] * p.y + world[4];
                    let y = world[1] * p.x + world[3] * p.y + world[5];
                    if i == 0 {
                        path.push_move_to(x, y);
                    } else {
                        path.push_line_to(x, y);
                    }
                }
                path.elements.push(PathElement::ClosePath);
                path.fill = Some(FillStyle::solid(color));
                emit_painted_path(ctx, obj, &path, scale, out);
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
            emit_painted_path(ctx, obj, &path, scale, out);
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
///
/// `scale` is the factor the geometry was baked through (see
/// [`crate::core::geometry::matrix_scale`]); stroke widths and dash arrays
/// ride it so the printed line matches the canvas.
fn emit_painted_path(ctx: &mut Ctx, obj: &Object, path: &PathData, scale: f64, out: &mut String) {
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
                    emit_stroke_only(ctx, obj, path, scale, out);
                    let _ = writeln!(out, "Q");
                }
                return;
            }
            FillType::Radial(g) => {
                emit_radial_shading_fill(ctx, path, g, out);
                let _ = writeln!(out, "Q");
                if path.stroke.is_some() {
                    let _ = writeln!(out, "q");
                    emit_stroke_only(ctx, obj, path, scale, out);
                    let _ = writeln!(out, "Q");
                }
                return;
            }
            // Pattern / image fills have no press equivalent here: the flat
            // `color` field below is only a placeholder. Warn loudly instead
            // of silently printing a black box.
            FillType::Pattern(_) => {
                ctx.warn(
                    "pattern-fill",
                    "パターン塗りは印刷PDFで単色近似されます — 効果を確認してください",
                );
            }
            FillType::Image(_) => {
                ctx.warn(
                    "image-fill",
                    "画像塗りは印刷PDFで単色近似されます — 効果を確認してください",
                );
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
        let _ = writeln!(out, "{} w", f2((stroke.width * scale).max(0.05)));
        let _ = writeln!(out, "{} J", cap_str(&stroke.cap));
        let _ = writeln!(out, "{} j", join_str(&stroke.join));
        if let Some(dash) = stroke.dash_pattern.as_ref().filter(|d| !d.is_empty()) {
            // Dashes are baked in the same space as the path, so they scale
            // with it exactly like the canvas tessellation does.
            let pat = dash
                .iter()
                .map(|v| f2(v * scale))
                .collect::<Vec<_>>()
                .join(" ");
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

fn emit_stroke_only(ctx: &mut Ctx, obj: &Object, path: &PathData, scale: f64, out: &mut String) {
    if let Some(stroke) = path.stroke.as_ref() {
        let g = ctx.gs_for(stroke.overprint, obj.opacity * stroke.color[3]);
        if !g.is_empty() {
            let _ = writeln!(out, "{g}");
        }
        let p = ctx.stroke_paint(stroke.color, &stroke.spot);
        let _ = writeln!(out, "{p}");
        let _ = writeln!(out, "{} w", f2((stroke.width * scale).max(0.05)));
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
                let ink = rgb_to_cmyk([s.color[0], s.color[1], s.color[2], 1.0]);
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
    // RGB JPEG + gray SMask when alpha is used. CMYK documents get opaque
    // Flate CMYK instead (DCT has no CMYK path here, and SMask would break
    // PDF/X): alpha is composited over paper white, matching the flatten
    // regions' convention.
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
    let (jpeg, mask, cmyk_flate) = if ctx.cmyk {
        let mut cmyk = Vec::with_capacity((iw * ih) as usize * 4);
        for p in img.pixels() {
            // Opaque-paper composite, then naive-UCR ink (same model as
            // preflight/flatten, so plates agree with the report).
            let a = p[3] as f32 / 255.0;
            let r = (p[0] as f32 / 255.0) * a + (1.0 - a);
            let g = (p[1] as f32 / 255.0) * a + (1.0 - a);
            let b = (p[2] as f32 / 255.0) * a + (1.0 - a);
            let c = crate::core::print::rgb_to_cmyk_ink(r, g, b);
            cmyk.extend_from_slice(&[
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (c[3] * 255.0) as u8,
            ]);
        }
        (Vec::new(), None, Some(flate_compress(&cmyk)))
    } else {
        let mut jpeg = Vec::new();
        {
            use image::codecs::jpeg::JpegEncoder;
            use image::ImageEncoder;
            let enc = JpegEncoder::new_with_quality(&mut jpeg, 90);
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
        (jpeg, mask, None)
    };
    ctx.images.push(ImageObj {
        jpeg,
        mask,
        cmyk_flate,
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
        // PDF has no `arc` operator — emit a 4-segment cubic circle instead
        // (kappa = 0.5522847498). The old `… 4.5 0 360 arc S` was invalid.
        let k = 4.5_f64 * 0.552_284_749_8;
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
            let _ = writeln!(out, "{} {} m", f2(rx), f2(ry + 4.5));
            let _ = writeln!(
                out,
                "{} {} {} {} {} {} c",
                f2(rx + k),
                f2(ry + 4.5),
                f2(rx + 4.5),
                f2(ry + k),
                f2(rx + 4.5),
                f2(ry)
            );
            let _ = writeln!(
                out,
                "{} {} {} {} {} {} c",
                f2(rx + 4.5),
                f2(ry - k),
                f2(rx + k),
                f2(ry - 4.5),
                f2(rx),
                f2(ry - 4.5)
            );
            let _ = writeln!(
                out,
                "{} {} {} {} {} {} c",
                f2(rx - k),
                f2(ry - 4.5),
                f2(rx - 4.5),
                f2(ry - k),
                f2(rx - 4.5),
                f2(ry)
            );
            let _ = writeln!(
                out,
                "{} {} {} {} {} {} c S",
                f2(rx - 4.5),
                f2(ry + k),
                f2(rx - k),
                f2(ry + 4.5),
                f2(rx),
                f2(ry + 4.5)
            );
        }
    let _ = writeln!(out, "Q");
}
