//! Minimal PDF importer: vector paths, text and raster images.
//!
//! Scope is deliberately "light": single content-stream feature set that
//! covers exporter/office-suite output (our own exporter round-trips
//! exactly). Known gaps, each recorded as an import warning instead of a
//! failure:
//! * shadings/patterns (`sh`, pattern color spaces) and clipping (`W`)
//! * Type 3 / CID-keyed text shaping (extracted as WinAnsi-ish runs)
//! * 1-bit fax, CMYK/JPX images and soft masks (`SMask`)
//! * page `/Rotate` other than 0
//!
//! Pages stack vertically (page 2 below page 1, …) so multi-page documents
//! stay in one canvas.

use crate::core::document::{Document, Object, TextStyle};
use crate::core::path::{
    AnchorPoint, FillRule, FillStyle, PathData, PathElement, StrokeCap, StrokeJoin, StrokeStyle,
};
use lopdf::content::{Content, Operation};

/// Imported document plus non-fatal warnings (shown as one toast).
pub fn parse_pdf_bytes(bytes: &[u8]) -> Result<(Document, Vec<String>), String> {
    match lopdf::Document::load_mem(bytes) {
        Ok(doc) => {
            let mut imp = Importer::new(&doc);
            imp.run()?;
            return Ok((imp.out, imp.warnings));
        }
        Err(first) => {
            // Lenient fallback: hand-made tools often emit slightly-off
            // xref tables. Rebuild one by scanning object offsets and retry
            // (incremental-style append, so nothing original is touched).
            if let Some(repaired) = repair_xref(bytes) {
                if let Ok(doc) = lopdf::Document::load_mem(&repaired) {
                    let mut imp = Importer::new(&doc);
                    imp.run()?;
                    imp.warnings.insert(
                        0,
                        "相互参照テーブルを修復して開きました".to_string(),
                    );
                    return Ok((imp.out, imp.warnings));
                }
            }
            return Err(format!("PDFを開けません: {first}"));
        }
    }
}

/// Rebuild a classic xref table by scanning `N G obj` offsets. Returns a
/// new byte buffer (original + appended xref/trailer) or `None` when the
/// file has nothing salvageable.
fn repair_xref(bytes: &[u8]) -> Option<Vec<u8>> {
    // Collect (number, generation, offset) for `N G obj` at line starts.
    let mut objs: Vec<(u32, u32, usize)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let line_start = i == 0 || bytes[i - 1] == b'\n' || bytes[i - 1] == b'\r';
        if line_start && bytes[i].is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            // Overlong runs (binary garbage) must not abort the scan.
            let num: u32 = match std::str::from_utf8(&bytes[i..j.min(i + 10)]) {
                Ok(s) => match s.parse() {
                    Ok(n) => n,
                    Err(_) => {
                        i += 1;
                        continue;
                    }
                },
                Err(_) => {
                    i += 1;
                    continue;
                }
            };
            if bytes.get(j) == Some(&b' ') {
                let mut k = j + 1;
                while k < bytes.len() && bytes[k].is_ascii_digit() {
                    k += 1;
                }
                let gen: u32 = match std::str::from_utf8(&bytes[j + 1..k.min(j + 11)]) {
                    Ok(s) => match s.parse() {
                        Ok(n) => n,
                        Err(_) => {
                            i += 1;
                            continue;
                        }
                    },
                    Err(_) => {
                        i += 1;
                        continue;
                    }
                };
                if bytes.get(k) == Some(&b' ')
                    && bytes.get(k + 1..k + 4) == Some(b"obj".as_slice())
                {
                    objs.push((num, gen, i));
                    i = k + 3;
                    continue;
                }
            }
        }
        i += 1;
    }
    if objs.is_empty() {
        return None;
    }
    // Keep the original trailer dictionary (has /Root).
    let trailer_pos = bytes.windows(7).rposition(|w| w == b"trailer")?;
    let dict_start = trailer_pos + 7;
    let dict_end = bytes[dict_start..]
        .windows(9)
        .position(|w| w == b"startxref")
        .map(|p| dict_start + p)?;
    let trailer_dict = std::str::from_utf8(&bytes[dict_start..dict_end]).ok()?;
    let size = objs.iter().map(|(n, _, _)| *n).max().unwrap_or(0) + 1;
    let mut entry_of = std::collections::HashMap::new();
    for (n, g, off) in objs {
        entry_of.insert(n, (off, g));
    }
    let mut out = bytes.to_vec();
    // Neutralize ALL existing %%EOF markers (same length, offsets
    // preserved): readers search forward for %%EOF, so any stale one —
    // including the original trailing marker — would shadow the appended
    // xref section and defeat the repair.
    let mut from = 0;
    while let Some(p) = out[from..].windows(5).position(|w| w == b"%%EOF") {
        out[from + p..from + p + 5].copy_from_slice(b"%%EOX");
        from += p + 5;
    }
    let xref_at = out.len() + 1; // account for the newline below
    out.extend_from_slice(format!("\nxref\n0 {size}\n").as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for n in 1..size {
        match entry_of.get(&n) {
            Some((off, gen)) => {
                out.extend_from_slice(format!("{:010} {:05} n \n", off, gen).as_bytes())
            }
            None => out.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }
    out.extend_from_slice(b"trailer\n");
    out.extend_from_slice(trailer_dict.as_bytes());
    out.extend_from_slice(format!("\nstartxref\n{xref_at}\n%%EOF\n").as_bytes());
    Some(out)
}

type Affine = [f64; 6];

fn mat_concat(m: &Affine, n: &Affine) -> Affine {
    [
        m[0] * n[0] + m[2] * n[1],
        m[1] * n[0] + m[3] * n[1],
        m[0] * n[2] + m[2] * n[3],
        m[1] * n[2] + m[3] * n[3],
        m[0] * n[4] + m[2] * n[5] + m[4],
        m[1] * n[4] + m[3] * n[5] + m[5],
    ]
}

fn mat_apply(m: &Affine, x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

fn mat_xscale(m: &Affine) -> f64 {
    (m[0] * m[0] + m[1] * m[1]).sqrt()
}

fn num(op: &lopdf::Object) -> f64 {
    match op {
        lopdf::Object::Integer(i) => *i as f64,
        lopdf::Object::Real(f) => *f as f64,
        _ => 0.0,
    }
}

fn name_str(op: &lopdf::Object) -> Option<String> {
    match op {
        lopdf::Object::Name(n) => {
            let mut s = String::from_utf8_lossy(n).into_owned();
            if s.starts_with('/') {
                s.remove(0);
            }
            Some(s)
        }
        _ => None,
    }
}

fn str_bytes(op: &lopdf::Object) -> Option<&[u8]> {
    match op {
        lopdf::Object::String(b, _) => Some(b),
        _ => None,
    }
}

/// WinAnsi (PDF default) → Unicode. Bytes 0x00–0x7F and 0xA0–0xFF map
/// 1:1 to U+0000–U+00FF; the 0x80–0x9F window uses the table below.
fn winansi_byte_to_char(b: u8) -> char {
    const EXTRA: [u32; 32] = [
        0x20AC, 0xFFFD, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160,
        0x2039, 0x0152, 0xFFFD, 0x017D, 0xFFFD, 0xFFFD, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022,
        0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0xFFFD, 0x017E, 0x0178,
    ];
    if b < 0x80 || b >= 0xA0 {
        b as char
    } else {
        char::from_u32(EXTRA[(b - 0x80) as usize]).unwrap_or('\u{FFFD}')
    }
}

fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM.
        bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .map(|u| char::from_u32(u as u32).unwrap_or('\u{FFFD}'))
            .collect()
    } else {
        bytes.iter().map(|b| winansi_byte_to_char(*b)).collect()
    }
}

#[derive(Clone)]
enum Seg {
    Move(f64, f64),
    Line(f64, f64),
    Curve(f64, f64, f64, f64, f64, f64),
    Rect(f64, f64, f64, f64),
    Close,
}

#[derive(Clone)]
struct GState {
    ctm: Affine,
    fill: [f32; 4],
    stroke: Option<StrokeStyle>,
    path: Vec<Seg>,
    // Text state.
    in_text: bool,
    font_family: String,
    font_size: f64,
    tm: Affine,
    x_ts: f64,
    char_space: f64,
    word_space: f64,
    leading: f64,
}

impl Default for GState {
    fn default() -> Self {
        Self {
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            fill: [0.0, 0.0, 0.0, 1.0],
            stroke: None,
            path: Vec::new(),
            in_text: false,
            font_family: "sans-serif".to_string(),
            font_size: 12.0,
            tm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            x_ts: 0.0,
            char_space: 0.0,
            word_space: 0.0,
            leading: 0.0,
        }
    }
}

struct Importer<'a> {
    doc: &'a lopdf::Document,
    out: Document,
    warnings: Vec<String>,
    warned: std::collections::HashSet<&'static str>,
    seq: usize,
    page_no: usize,
}

impl<'a> Importer<'a> {
    fn new(doc: &'a lopdf::Document) -> Self {
        let mut out = Document::default();
        out.name = "PDF Import".to_string();
        Self {
            doc,
            out,
            warnings: Vec::new(),
            warned: std::collections::HashSet::new(),
            seq: 0,
            page_no: 0,
        }
    }

    fn warn(&mut self, key: &'static str, msg: String) {
        if self.warned.insert(key) {
            self.warnings.push(msg);
        }
    }

    fn resolve<'b>(&'b self, obj: &'b lopdf::Object) -> &'b lopdf::Object {
        let mut cur = obj;
        for _ in 0..16 {
            match cur {
                lopdf::Object::Reference(id) => match self.doc.get_object(*id) {
                    Ok(next) => cur = next,
                    Err(_) => break,
                },
                _ => break,
            }
        }
        cur
    }

    /// Stream payload: decompressed when filtered, raw otherwise.
    fn stream_bytes(stream: &lopdf::Stream) -> Result<Vec<u8>, String> {
        if stream.dict.get(b"Filter").is_ok() {
            stream
                .decompressed_content()
                .map_err(|e| format!("展開失敗: {e}"))
        } else {
            Ok(stream.content.clone())
        }
    }

    fn run(&mut self) -> Result<(), String> {
        let pages = self.doc.get_pages();
        if pages.is_empty() {
            return Err("ページがありません".to_string());
        }
        // First pass: boxes for vertical stacking.
        struct Box_ {
            id: lopdf::ObjectId,
            w: f64,
            h: f64,
            y_off: f64,
        }
        let mut boxes = Vec::new();
        let mut total_h = 0.0;
        let mut max_w = 0.0;
        for (_, id) in pages.iter() {
            let (w, h) = self.page_size(*id);
            if w > max_w {
                max_w = w;
            }
            boxes.push(Box_ {
                id: *id,
                w,
                h,
                y_off: total_h,
            });
            total_h += h;
        }
        self.out.width = max_w.max(1.0);
        self.out.height = total_h.max(1.0);

        for b in boxes {
            self.page_no += 1;
            let suffix = if pages.len() > 1 {
                format!(" (p{})", self.page_no)
            } else {
                String::new()
            };
            // Base: PDF y-up → canvas y-down, stacked at y_off.
            let (w, h) = self.page_size(b.id);
            let base: Affine = [1.0, 0.0, 0.0, -1.0, 0.0, b.y_off + h];
            let mut st = GState::default();
            st.ctm = base;
            let resources = self.page_resources(b.id);
            let contents = self.doc.get_page_contents(b.id);
            for cid in contents {
                let obj = self.doc.get_object(cid).map_err(|e| format!("Content取得失敗: {e}"))?;
                let obj = self.resolve(obj);
                let stream = obj.as_stream().map_err(|_| "Contentがstreamではありません".to_string())?;
                // Unfiltered streams have no /Filter key (lopdf errors on
                // those) — use the raw bytes directly.
                let bytes = Self::stream_bytes(stream)?;
                let content = Content::decode(&bytes)
                    .map_err(|e| format!("Content解析失敗: {e}"))?;
                self.run_ops(&content.operations, &resources, &mut st, &suffix)?;
            }
            let _ = w;
        }
        Ok(())
    }

    fn page_dict(&self, id: lopdf::ObjectId) -> Option<lopdf::Dictionary> {
        let obj = self.doc.get_object(id).ok()?;
        let obj = self.resolve(obj);
        match obj {
            lopdf::Object::Dictionary(d) => Some(d.clone()),
            _ => None,
        }
    }

    fn page_size(&self, id: lopdf::ObjectId) -> (f64, f64) {
        let d = match self.page_dict(id) {
            Some(d) => d,
            None => return (595.0, 842.0),
        };
        let arr = d
            .get(b"MediaBox")
            .ok()
            .map(|o| self.resolve(o))
            .and_then(|o| match o {
                lopdf::Object::Array(a) => Some(a.clone()),
                _ => None,
            });
        if let Some(a) = arr {
            if a.len() == 4 {
                let w = num(&a[2]) - num(&a[0]);
                let h = num(&a[3]) - num(&a[1]);
                if w > 0.0 && h > 0.0 {
                    return (w, h);
                }
            }
        }
        (595.0, 842.0)
    }

    fn page_resources(&self, id: lopdf::ObjectId) -> Option<lopdf::Dictionary> {
        let d = self.page_dict(id)?;
        let res = d.get(b"Resources").ok()?;
        match self.resolve(res) {
            lopdf::Object::Dictionary(r) => Some(r.clone()),
            _ => None,
        }
    }

    fn run_ops(
        &mut self,
        ops: &[Operation],
        resources: &Option<lopdf::Dictionary>,
        st: &mut GState,
        suffix: &str,
    ) -> Result<(), String> {
        let mut stack: Vec<GState> = Vec::new();
        for op in ops {
            let o = op.operands.as_slice();
            match op.operator.as_str() {
                "q" => stack.push(st.clone()),
                "Q" => {
                    if let Some(s) = stack.pop() {
                        let path = std::mem::take(&mut st.path);
                        *st = s;
                        st.path = path;
                    }
                }
                "cm" if o.len() == 6 => {
                    let m = [num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3]), num(&o[4]), num(&o[5])];
                    st.ctm = mat_concat(&m, &st.ctm);
                }
                "w" if !o.is_empty() => {
                    let w = num(&o[0]).max(0.0) * mat_xscale(&st.ctm).max(1e-6);
                    match &mut st.stroke {
                        Some(s) => s.width = w.max(0.1),
                        None => {
                            st.stroke = Some(StrokeStyle {
                                width: w.max(0.1),
                                ..Default::default()
                            })
                        }
                    }
                }
                "J" if !o.is_empty() => {
                    let cap = match o[0].clone() {
                        lopdf::Object::Integer(1) => StrokeCap::Round,
                        lopdf::Object::Integer(2) => StrokeCap::Square,
                        _ => StrokeCap::Butt,
                    };
                    self.ensure_stroke(st).cap = cap;
                }
                "j" if !o.is_empty() => {
                    let join = match o[0].clone() {
                        lopdf::Object::Integer(1) => StrokeJoin::Round,
                        lopdf::Object::Integer(2) => StrokeJoin::Bevel,
                        _ => StrokeJoin::Miter,
                    };
                    self.ensure_stroke(st).join = join;
                }
                "d" if o.len() == 2 => {
                    if let lopdf::Object::Array(dashes) = &o[0] {
                        let pat: Vec<f64> = dashes.iter().map(num).filter(|v| *v > 0.0).collect();
                        self.ensure_stroke(st).dash_pattern = if pat.is_empty() { None } else { Some(pat) };
                    }
                }
                "G" if !o.is_empty() => {
                    let g = (num(&o[0]) as f32).clamp(0.0, 1.0);
                    self.ensure_stroke(st).color = [g, g, g, 1.0];
                }
                "g" if !o.is_empty() => {
                    let g = (num(&o[0]) as f32).clamp(0.0, 1.0);
                    st.fill = [g, g, g, 1.0];
                }
                "RG" if o.len() == 3 => {
                    let c = [
                        num(&o[0]) as f32,
                        num(&o[1]) as f32,
                        num(&o[2]) as f32,
                        1.0,
                    ];
                    self.ensure_stroke(st).color = c;
                }
                "rg" if o.len() == 3 => {
                    st.fill = [
                        num(&o[0]) as f32,
                        num(&o[1]) as f32,
                        num(&o[2]) as f32,
                        1.0,
                    ];
                }
                "K" if o.len() == 4 => {
                    self.ensure_stroke(st).color = cmyk_to_rgb(num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3]));
                }
                "k" if o.len() == 4 => {
                    st.fill = cmyk_to_rgb(num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3]));
                }
                "m" if o.len() == 2 => st.path.push(Seg::Move(num(&o[0]), num(&o[1]))),
                "l" if o.len() == 2 => st.path.push(Seg::Line(num(&o[0]), num(&o[1]))),
                "c" if o.len() == 6 => st.path.push(Seg::Curve(
                    num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3]), num(&o[4]), num(&o[5]),
                )),
                "v" if o.len() == 4 => {
                    let (x, y) = self.current_point(st);
                    st.path.push(Seg::Curve(x, y, num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3])));
                }
                "y" if o.len() == 4 => {
                    // Second control point coincides with the end point;
                    // the first one is the current point.
                    let (x, y) = self.current_point(st);
                    st.path.push(Seg::Curve(x, y, num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3])));
                }
                "h" => st.path.push(Seg::Close),
                "re" if o.len() == 4 => {
                    st.path.push(Seg::Rect(num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3])));
                }
                "n" => st.path.clear(),
                "S" | "s" => {
                    if op.operator == "s" {
                        st.path.push(Seg::Close);
                    }
                    self.emit_path(st, false, true, false, suffix);
                }
                "f" | "F" | "f*" => {
                    self.emit_path(st, true, false, op.operator == "f*", suffix);
                }
                "B" | "B*" | "b" | "b*" => {
                    if op.operator == "b" || op.operator == "b*" {
                        st.path.push(Seg::Close);
                    }
                    self.emit_path(st, true, true, op.operator.ends_with('*'), suffix);
                }
                "W" | "W*" => {
                    self.warn("clip", "クリッピングパスは無視されました".to_string());
                    st.path.clear();
                }
                "sh" => {
                    self.warn("shading", "シェーディングは無視されました".to_string());
                }
                "BT" => {
                    st.in_text = true;
                    st.tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    st.x_ts = 0.0;
                    st.leading = 0.0;
                }
                "ET" => st.in_text = false,
                "Tf" if o.len() == 2 => {
                    st.font_family =
                        self.resolve_font(resources, name_str(&o[0]).as_deref().unwrap_or(""));
                    st.font_size = num(&o[1]).abs().max(0.5);
                }
                "Tm" if o.len() == 6 => {
                    st.tm = [num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3]), num(&o[4]), num(&o[5])];
                    st.x_ts = 0.0;
                }
                "Td" if o.len() == 2 => {
                    st.tm[4] += num(&o[0]);
                    st.tm[5] += num(&o[1]);
                    st.x_ts = 0.0;
                }
                "TD" if o.len() == 2 => {
                    st.leading = -num(&o[1]);
                    st.tm[4] += num(&o[0]);
                    st.tm[5] += num(&o[1]);
                    st.x_ts = 0.0;
                }
                "T*" => {
                    st.tm[5] -= st.leading;
                    st.x_ts = 0.0;
                }
                "Tc" if !o.is_empty() => st.char_space = num(&o[0]),
                "Tw" if !o.is_empty() => st.word_space = num(&o[0]),
                "Tz" => {}
                "TL" if !o.is_empty() => st.leading = num(&o[0]),
                "Tj" if !o.is_empty() => {
                    if let Some(b) = str_bytes(&o[0]) {
                        self.emit_text(st, &decode_pdf_string(b), suffix);
                    }
                }
                "TJ" if !o.is_empty() => {
                    if let lopdf::Object::Array(items) = &o[0] {
                        for item in items {
                            if let Some(b) = str_bytes(item) {
                                self.emit_text(st, &decode_pdf_string(b), suffix);
                            } else {
                                st.x_ts -= num(item) / 1000.0 * st.font_size;
                            }
                        }
                    }
                }
                "'" if !o.is_empty() => {
                    st.tm[5] -= st.leading;
                    st.x_ts = 0.0;
                    if let Some(b) = str_bytes(&o[0]) {
                        self.emit_text(st, &decode_pdf_string(b), suffix);
                    }
                }
                "\"" if o.len() == 3 => {
                    st.word_space = num(&o[0]);
                    st.char_space = num(&o[1]);
                    st.tm[5] -= st.leading;
                    st.x_ts = 0.0;
                    if let Some(b) = str_bytes(&o[2]) {
                        self.emit_text(st, &decode_pdf_string(b), suffix);
                    }
                }
                "Do" if !o.is_empty() => {
                    if let Some(name) = name_str(&o[0]) {
                        self.emit_xobject(resources, &name, st, suffix)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn ensure_stroke<'b>(&mut self, st: &'b mut GState) -> &'b mut StrokeStyle {
        if st.stroke.is_none() {
            st.stroke = Some(StrokeStyle::default());
        }
        st.stroke.as_mut().unwrap()
    }

    fn current_point(&self, st: &GState) -> (f64, f64) {
        for seg in st.path.iter().rev() {
            match seg {
                Seg::Move(x, y) | Seg::Line(x, y) => return (*x, *y),
                Seg::Curve(_, _, _, _, x, y) => return (*x, *y),
                Seg::Rect(x, y, _, _) => return (*x, *y),
                Seg::Close => continue,
            }
        }
        (0.0, 0.0)
    }

    fn emit_path(&mut self, st: &mut GState, fill: bool, stroke: bool, even_odd: bool, suffix: &str) {
        if st.path.is_empty() {
            return;
        }
        let mut path = PathData::new();
        for seg in &st.path {
            match seg {
                Seg::Move(x, y) => {
                    let (x, y) = mat_apply(&st.ctm, *x, *y);
                    path.push_move_to(x, y);
                }
                Seg::Line(x, y) => {
                    let (x, y) = mat_apply(&st.ctm, *x, *y);
                    path.push_line_to(x, y);
                }
                Seg::Curve(x1, y1, x2, y2, x3, y3) => {
                    let (x1, y1) = mat_apply(&st.ctm, *x1, *y1);
                    let (x2, y2) = mat_apply(&st.ctm, *x2, *y2);
                    let (x3, y3) = mat_apply(&st.ctm, *x3, *y3);
                    path.elements.push(PathElement::CurveTo(
                        crate::core::path::BezierSegment {
                            start: path_current(&path).unwrap_or(AnchorPoint::new(x1, y1)),
                            control1: AnchorPoint::new(x1, y1),
                            control2: AnchorPoint::new(x2, y2),
                            end: AnchorPoint::new(x3, y3),
                        },
                    ));
                }
                Seg::Rect(x, y, w, h) => {
                    let (x1, y1) = mat_apply(&st.ctm, *x, *y);
                    let (x2, y2) = mat_apply(&st.ctm, *x + *w, *y + *h);
                    // Axis-mapped rect (rotation bakes into a general quad;
                    // keep it rectangular — rotation-heavy PDFs are rare).
                    let (lx, rx) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
                    let (ty, by) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
                    path.push_move_to(lx, ty);
                    path.push_line_to(rx, ty);
                    path.push_line_to(rx, by);
                    path.push_line_to(lx, by);
                    path.elements.push(PathElement::ClosePath);
                }
                Seg::Close => path.elements.push(PathElement::ClosePath),
            }
        }
        st.path.clear();
        if path.elements.is_empty() {
            return;
        }
        path.fill = if fill {
            Some(FillStyle {
                color: st.fill,
                fill_type: crate::core::path::FillType::Solid(st.fill),
                rule: if even_odd { FillRule::EvenOdd } else { FillRule::NonZero },
            })
        } else {
            None
        };
        path.stroke = if stroke { st.stroke.clone() } else { None };
        self.seq += 1;
        let obj = Object::new_path(&format!("PDF Path {:03}{suffix}", self.seq), path);
        // Geometry is baked to device space already.
        self.out.add_object(obj);
    }

    fn emit_text(&mut self, st: &mut GState, s: &str, suffix: &str) {
        if s.is_empty() || !st.in_text {
            return;
        }
        let ex = mat_xscale(&st.ctm).max(1e-6);
        // Pen: text-space advance mapped through Tm scale, then CTM.
        // (Standard files use translation-only Tm, so this is exact there.)
        let (ux, uy) = (
            st.tm[4] + st.x_ts * st.tm[0],
            st.tm[5] + st.x_ts * st.tm[1],
        );
        let (dx, dy) = mat_apply(&st.ctm, ux, uy);
        // Rendered size: Tf size times any Tm scaling times CTM scale.
        let size = (st.font_size * st.tm[0].abs().max(st.tm[3].abs()) * ex).max(0.5);
        let mut tstyle = TextStyle::new(&st.font_family, size);
        if st.char_space.abs() > 1e-9 {
            tstyle.letter_spacing = st.char_space * ex;
        }
        if st.leading > 0.0 && st.font_size > 0.0 {
            tstyle.line_height = Some(st.leading / st.font_size);
        }
        self.seq += 1;
        let mut obj = Object::new_text_with_style(
            &format!("PDF Text {:03}{suffix}", self.seq),
            s,
            dx,
            dy,
            tstyle,
        );
        obj.fill = Some(FillStyle::solid(st.fill));
        self.out.add_object(obj);
        // Advance in text-space units: em estimates scaled by size, plus
        // raw Tc/Tw (spec: ((w0 − Tj/1000) × fs + Tc + Tw)).
        let em: f64 = s
            .chars()
            .map(|ch| {
                let mut a = crate::core::document::char_advance_estimate(ch) * st.font_size;
                if ch == ' ' {
                    a += st.word_space;
                }
                a + st.char_space
            })
            .sum();
        st.x_ts += em;
    }

    fn resolve_font(&mut self, resources: &Option<lopdf::Dictionary>, name: &str) -> String {
        let mut family = name.to_string();
        if let Some(res) = resources {
            if let Ok(fonts) = res.get(b"Font") {
                let fonts = self.resolve(fonts);
                if let lopdf::Object::Dictionary(fd) = fonts {
                    if let Ok(fobj) = fd.get(name.as_bytes()) {
                        let fobj = self.resolve(fobj);
                        if let lopdf::Object::Dictionary(fdict) = fobj {
                            if let Ok(base) = fdict.get(b"BaseFont") {
                                if let Some(bn) = name_str(self.resolve(base)) {
                                    family = bn;
                                }
                            }
                        }
                    }
                }
            }
        }
        // Strip subset prefixes ("ABCDEF+Helvetica").
        if let Some(plus) = family.find('+') {
            if plus == 6 {
                family = family[plus + 1..].to_string();
            }
        }
        family
    }

    fn emit_xobject(
        &mut self,
        resources: &Option<lopdf::Dictionary>,
        name: &str,
        st: &mut GState,
        suffix: &str,
    ) -> Result<(), String> {
        let res = resources.as_ref().ok_or("XObject参照もリソースもありません".to_string())?;
        let xo = res
            .get(b"XObject")
            .map_err(|_| "XObjectリソースがありません".to_string())?;
        let xo = self.resolve(xo);
        let xod = match xo {
            lopdf::Object::Dictionary(d) => d.clone(),
            _ => return Err("XObject辞書ではありません".to_string()),
        };
        let entry = xod
            .get(name.as_bytes())
            .map_err(|_| format!("XObject {name} がありません"))?;
        let entry = self.resolve(entry);
        // Clone out of the document borrow: the Form recursion and image
        // decoding below need `&mut self`.
        let stream = entry
            .as_stream()
            .map_err(|_| format!("XObject {name} がstreamではありません"))?
            .clone();
        let subtype = stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|o| name_str(self.resolve(o)))
            .unwrap_or_default();
        if subtype == "Form" {
            // Recurse with concatenated matrix.
            let mat = stream
                .dict
                .get(b"Matrix")
                .ok()
                .and_then(|o| match self.resolve(o) {
                    lopdf::Object::Array(a) if a.len() == 6 => Some([
                        num(&a[0]), num(&a[1]), num(&a[2]), num(&a[3]), num(&a[4]), num(&a[5]),
                    ]),
                    _ => None,
                })
                .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
            let saved = st.ctm;
            st.ctm = mat_concat(&mat, &st.ctm);
            let sub_res = stream
                .dict
                .get(b"Resources")
                .ok()
                .and_then(|o| match self.resolve(o) {
                    lopdf::Object::Dictionary(d) => Some(d.clone()),
                    _ => None,
                })
                .or_else(|| resources.clone());
            let bytes =
                Self::stream_bytes(&stream).map_err(|e| format!("Form{e}"))?;
            let content =
                Content::decode(&bytes).map_err(|e| format!("Form解析失敗: {e}"))?;
            let r = self.run_ops(&content.operations, &sub_res, st, suffix);
            st.ctm = saved;
            return r;
        }
        if subtype != "Image" {
            self.warn("xobject", format!("未対応XObject ({subtype}) をスキップ"));
            return Ok(());
        }
        self.emit_image(&stream, name, st, suffix)
    }

    fn emit_image(
        &mut self,
        stream: &lopdf::Stream,
        name: &str,
        st: &GState,
        suffix: &str,
    ) -> Result<(), String> {
        let dict = &stream.dict;
        let get_num = |key: &[u8]| -> Option<f64> {
            dict.get(key).ok().and_then(|o| match self.resolve(o) {
                lopdf::Object::Integer(i) => Some(*i as f64),
                lopdf::Object::Real(f) => Some(*f as f64),
                _ => None,
            })
        };
        let w = get_num(b"Width").unwrap_or(0.0) as u32;
        let h = get_num(b"Height").unwrap_or(0.0) as u32;
        let bpc = get_num(b"BitsPerComponent").unwrap_or(8.0) as u32;
        if w == 0 || h == 0 || w * h > 16_777_216 {
            self.warn("image-size", format!("異常な画像サイズ ({w}x{h}) をスキップ"));
            return Ok(());
        }
        if dict.get(b"SMask").is_ok() {
            self.warn("smask", "ソフトマスク付き画像は不透明として取り込みます".to_string());
        }
        // Filter chain decides the payload kind.
        let filters: Vec<String> = dict
            .get(b"Filter")
            .ok()
            .map(|o| match self.resolve(o) {
                lopdf::Object::Name(n) => vec![String::from_utf8_lossy(n).into_owned()],
                lopdf::Object::Array(a) => a
                    .iter()
                    .filter_map(|e| match self.resolve(e) {
                        lopdf::Object::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default();
        let has = |f: &str| filters.iter().any(|x| x == f || x == &format!("/{f}"));
        if has("JPXDecode") {
            self.warn("jpx", format!("JPEG2000画像 {name} をスキップ"));
            return Ok(());
        }
        if has("CCITTFaxDecode") || has("JBIG2Decode") {
            self.warn("fax", format!("FAX画像 {name} をスキップ"));
            return Ok(());
        }
        // Colorspace.
        let cs = dict
            .get(b"ColorSpace")
            .ok()
            .map(|o| match self.resolve(o) {
                lopdf::Object::Name(n) => String::from_utf8_lossy(n).into_owned(),
                lopdf::Object::Array(a) => a
                    .first()
                    .and_then(|e| name_str(self.resolve(e)))
                    .unwrap_or_default(),
                _ => String::new(),
            })
            .unwrap_or_default();
        let channels: u32 = if cs.contains("DeviceRGB") || cs.contains("CalRGB") {
            3
        } else if cs.contains("DeviceGray") || cs.contains("CalGray") {
            1
        } else if cs.contains("DeviceCMYK") {
            self.warn("cmyk-img", format!("CMYK画像 {name} をスキップ"));
            return Ok(());
        } else {
            // Indexed / ICCBased / Separation: light version skips.
            self.warn("colorspace", format!("未対応色空間 ({cs}) の画像 {name} をスキップ"));
            return Ok(());
        };
        let png_bytes: Vec<u8> = if has("DCTDecode") {
            if filters.len() == 1 {
                // Bare JPEG (lopdf leaves DCT alone).
                stream.content.clone()
            } else {
                self.warn(
                    "jpeg-chain",
                    format!("複合フィルタのJPEG {name} をスキップ"),
                );
                return Ok(());
            }
        } else {
            let raw = stream
                .decompressed_content()
                .map_err(|e| format!("画像展開失敗: {e}"))?;
            samples_to_png(&raw, w, h, channels, bpc).ok_or_else(|| {
                self.warn("samples", format!("画像サンプル解釈失敗 ({name})"));
                "画像サンプル解釈失敗".to_string()
            })?
        };
        let (pw, ph, placed) = crate::io::raster::decode_placed_image(&png_bytes)
            .map_err(|e| format!("画像配置失敗: {e}"))?;
        // Unit-square corners through the CTM → device rect.
        let (x1, y1) = mat_apply(&st.ctm, 0.0, 0.0);
        let (x2, y2) = mat_apply(&st.ctm, 1.0, 1.0);
        let (lx, rx) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
        let (ty, by) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
        self.seq += 1;
        let obj = Object::new_image(
            &format!("PDF Image {:03}{suffix}", self.seq),
            lx,
            ty,
            (rx - lx).max(1.0),
            (by - ty).max(1.0),
            placed,
        );
        let _ = (pw, ph);
        self.out.add_object(obj);
        Ok(())
    }
}

fn cmyk_to_rgb(c: f64, m: f64, y: f64, k: f64) -> [f32; 4] {
    let (c, m, y, k) = (
        c.clamp(0.0, 1.0) as f32,
        m.clamp(0.0, 1.0) as f32,
        y.clamp(0.0, 1.0) as f32,
        k.clamp(0.0, 1.0) as f32,
    );
    [
        (1.0 - c.min(1.0)) * (1.0 - k),
        (1.0 - m.min(1.0)) * (1.0 - k),
        (1.0 - y.min(1.0)) * (1.0 - k),
        1.0,
    ]
}

fn path_current(path: &PathData) -> Option<AnchorPoint> {
    // Last explicit point (Move/Line/Curve end).
    for el in path.elements.iter().rev() {
        match el {
            PathElement::MoveTo(p) | PathElement::LineTo(p) => return Some(*p),
            PathElement::CurveTo(seg) => return Some(seg.end),
            PathElement::ClosePath => continue,
        }
    }
    None
}

/// Raw image samples (post-filter, post-predictor via lopdf) → PNG bytes.
/// Supports 8-bit gray/RGB and 1-bit gray.
fn samples_to_png(raw: &[u8], w: u32, h: u32, channels: u32, bpc: u32) -> Option<Vec<u8>> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    let px = (w as usize) * (h as usize);
    let rgb: Vec<u8> = match (channels, bpc) {
        (3, 8) if raw.len() >= px * 3 => raw[..px * 3].to_vec(),
        (1, 8) if raw.len() >= px => {
            let mut out = Vec::with_capacity(px * 3);
            for v in &raw[..px] {
                out.extend_from_slice(&[*v, *v, *v]);
            }
            out
        }
        (1, 1) if raw.len() * 8 >= px => {
            let mut out = Vec::with_capacity(px * 3);
            for i in 0..px {
                let v = if raw[i / 8] & (0x80 >> (i % 8)) != 0 { 0u8 } else { 255u8 };
                out.extend_from_slice(&[v, v, v]);
            }
            out
        }
        _ => return None,
    };
    // Opaque alpha.
    let mut rgba = Vec::with_capacity(px * 4);
    for p in rgb.chunks_exact(3) {
        rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(png)
}
