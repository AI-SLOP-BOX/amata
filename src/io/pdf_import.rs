//! Minimal PDF importer: vector paths, text and raster images.
//!
//! Scope is deliberately "light": single content-stream feature set that
//! covers exporter/office-suite output (our own exporter round-trips
//! exactly). Known gaps, each recorded as an import warning instead of a
//! failure:
//! * Coons/tensor-patch and pattern shadings (`sh` beyond axial/radial/
//!   Gouraud with exponential/stitching functions)
//! * Type 3 glyphs without resolvable programs (fall back to WinAnsi runs
//!   with a warning so missing vector glyphs stay explicit)
//! * CID fonts without `/ToUnicode` (a placeholder is imported with a
//!   warning so missing text stays explicit)
//! * JPEG2000, JBIG2 and mixed-2D/byte-aligned fax images; `/SMask` beyond
//!   gray 1/2/4/8-bit and JPEG falls back to opaque with a warning
//! * page `/Rotate` is limited to quarter-turn values
//!
//! Pages stack vertically (page 2 below page 1, …) so multi-page documents
//! stay in one canvas.

use crate::core::document::{BlendMode, Document, Object, ObjectType, TextStyle};
use crate::core::path::{
    AnchorPoint, FillRule, FillStyle, PathData, PathElement, StrokeCap, StrokeJoin, StrokeStyle,
};
use lopdf::content::{Content, Operation};

/// Illustrator (.ai) import: modern .ai files are PDF with Illustrator
/// private data, so the PDF-compatible content parses with the regular
/// importer. Returns the document plus warnings; the first warnings always
/// describe what was skipped (private edit data, version), so "it opened
/// but something is missing" is never silent.
pub fn parse_ai_bytes(bytes: &[u8]) -> Result<(Document, Vec<String>), String> {
    if bytes.len() < 5 || &bytes[0..5] != b"%PDF-" {
        return Err("Illustratorファイルではありません（PDF互換部がありません）".to_string());
    }
    let (doc, warnings) = parse_pdf_bytes(bytes)?;
    // Illustrator version from the Info dict, when present.
    let mut version = String::new();
    if let Ok(loaded) = load_pdf_guarded(bytes) {
        if let Ok(info) = loaded.trailer.get(b"Info") {
            let dict_opt: Option<lopdf::Dictionary> = match info {
                lopdf::Object::Dictionary(d) => Some(d.clone()),
                lopdf::Object::Reference(id) => loaded.get_object(*id).ok().and_then(|o| match o {
                    lopdf::Object::Dictionary(d) => Some(d.clone()),
                    _ => None,
                }),
                _ => None,
            };
            if let Some(dict) = dict_opt {
                for key in [b"Creator".as_slice(), b"Producer".as_slice()] {
                    if let Ok(obj) = dict.get(key) {
                        if let Ok(s) = obj.as_str() {
                            let text = String::from_utf8_lossy(s).into_owned();
                            if text.contains("Illustrator") {
                                version = text;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    let mut out = vec![if version.is_empty() {
        "Illustrator編集用データはスキップしPDF互換部を読み込み".to_string()
    } else {
        format!("{version}の編集用データをスキップしPDF互換部を読み込み")
    }];
    out.extend(warnings);
    Ok((doc, out))
}

/// Max accepted PDF input (DoS guard: the parser walks the whole file
/// into memory, and the xref repair below clones it once more).
pub const MAX_PDF_BYTES: usize = 256 * 1024 * 1024;

/// Parse a PDF on a dedicated big-stack thread.
///
/// RUSTSEC-2026-0187: lopdf recurses into nested objects, so a malicious
/// file can overflow a normal 8MB stack and abort the process (stack
/// overflow is not catchable). A 256MB thread stack absorbs adversarial
/// nesting; the input cap above bounds the memory side.
fn load_pdf_guarded(bytes: &[u8]) -> Result<lopdf::Document, String> {
    if bytes.len() > MAX_PDF_BYTES {
        return Err(format!(
            "PDFが大きすぎます（上限{}MB）",
            MAX_PDF_BYTES / 1024 / 1024
        ));
    }
    let owned = bytes.to_vec();
    std::thread::Builder::new()
        .name("amata-pdf-parse".to_string())
        .stack_size(256 * 1024 * 1024)
        .spawn(move || lopdf::Document::load_mem(&owned))
        .map_err(|e| format!("PDF解析スレッドを起動できません: {e}"))?
        .join()
        .map_err(|_| "PDFの解析中に異常終了しました（不正なファイルの可能性）".to_string())?
        .map_err(|e| format!("PDFを開けません: {e}"))
}

/// Imported document plus non-fatal warnings (shown as one toast).
pub fn parse_pdf_bytes(bytes: &[u8]) -> Result<(Document, Vec<String>), String> {
    match load_pdf_guarded(bytes) {
        Ok(doc) => {
            let mut imp = Importer::new(&doc);
            imp.run()?;
            Ok((imp.out, imp.warnings))
        }
        Err(first) => {
            // Lenient fallback: hand-made tools often emit slightly-off
            // xref tables. Rebuild one by scanning object offsets and retry
            // (incremental-style append, so nothing original is touched).
            if let Some(repaired) = repair_xref(bytes) {
                if let Ok(doc) = load_pdf_guarded(&repaired) {
                    let mut imp = Importer::new(&doc);
                    imp.run()?;
                    imp.warnings
                        .insert(0, "相互参照テーブルを修復して開きました".to_string());
                    return Ok((imp.out, imp.warnings));
                }
            }
            Err(format!("PDFを開けません: {first}"))
        }
    }
}

/// Rebuild a classic xref table by scanning `N G obj` offsets. Returns a
/// new byte buffer (original + appended xref/trailer) or `None` when the
/// file has nothing salvageable.
fn repair_xref(bytes: &[u8]) -> Option<Vec<u8>> {
    // Refuse multi-hundred-MB blobs: we clone the whole file below, and
    // a corrupt input of that size is more likely truncated garbage than
    // a salvageable PDF (OOM guard for adversarial uploads).
    const MAX_REPAIR_BYTES: usize = 64 * 1024 * 1024;
    if bytes.len() > MAX_REPAIR_BYTES {
        return None;
    }
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
                if bytes.get(k) == Some(&b' ') && bytes.get(k + 1..k + 4) == Some(b"obj".as_slice())
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
    if !(0x80..0xA0).contains(&b) {
        b as char
    } else {
        char::from_u32(EXTRA[(b - 0x80) as usize]).unwrap_or('\u{FFFD}')
    }
}

fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM.
        let (units, _) = bytes[2..].as_chunks::<2>();
        units
            .iter()
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .map(|u| char::from_u32(u as u32).unwrap_or('\u{FFFD}'))
            .collect()
    } else {
        bytes.iter().map(|b| winansi_byte_to_char(*b)).collect()
    }
}

/// Parse `<...>` hex content into raw bytes. Whitespace inside is ignored.
fn parse_cmap_hex(token: &str) -> Option<Vec<u8>> {
    let hex: String = token.chars().filter(|c| !c.is_whitespace()).collect();
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let pair = std::str::from_utf8(&bytes[i..i + 2]).ok()?;
        out.push(u8::from_str_radix(pair, 16).ok()?);
        i += 2;
    }
    Some(out)
}

fn cmap_bytes_to_code(bytes: &[u8]) -> u32 {
    let mut code = 0u32;
    for b in bytes {
        code = (code << 8) | (*b as u32);
    }
    code
}

/// Raw `/ToUnicode` destination bytes → Unicode string. Destinations are
/// normally UTF-16BE; odd-length payloads fall back to single-byte scalars
/// so malformed entries degrade to visible placeholders, not panics.
fn cmap_bytes_to_unicode(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if !bytes.len().is_multiple_of(2) {
        return bytes
            .iter()
            .map(|b| char::from_u32(*b as u32).unwrap_or('\u{FFFD}'))
            .collect();
    }
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    let mut out = String::new();
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        if (0xD800..0xDC00).contains(&u) && i + 1 < units.len() {
            let lo = units[i + 1];
            if (0xDC00..0xE000).contains(&lo) {
                let cp = 0x10000 + ((u as u32 - 0xD800) << 10) + (lo as u32 - 0xDC00);
                out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                i += 2;
                continue;
            }
        }
        out.push(char::from_u32(u as u32).unwrap_or('\u{FFFD}'));
        i += 1;
    }
    out
}

/// Collect `<hex>` tokens inside a CMap section, preserving order.
fn collect_hex_tokens(section: &str) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let bytes = section.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(end) = section[i..].find('>') {
                if let Some(parsed) = parse_cmap_hex(&section[i + 1..i + end]) {
                    out.push(parsed);
                }
                i += end + 1;
                continue;
            }
            break;
        }
        i += 1;
    }
    out
}

/// Parse a `/ToUnicode` CMap into `source code → Unicode string`.
/// Supports `bfchar` pairs and `bfrange` runs (single start or array form).
/// Oversized ranges are truncated; callers warn once via the returned flag.
fn parse_tounicode_cmap(bytes: &[u8]) -> (std::collections::HashMap<u32, String>, bool) {
    const MAX_ENTRIES: usize = 100_000;
    let mut map = std::collections::HashMap::new();
    let mut truncated = false;
    let text = String::from_utf8_lossy(bytes).into_owned();

    // bfchar: sequential <src> <dst> pairs.
    let mut search_from = 0;
    while let Some(begin) = text[search_from..].find("beginbfchar") {
        let section_start = search_from + begin;
        let Some(end) = text[section_start..].find("endbfchar") else {
            break;
        };
        let section = &text[section_start..section_start + end];
        let tokens = collect_hex_tokens(section);
        for pair in tokens.as_chunks::<2>().0 {
            if map.len() >= MAX_ENTRIES {
                truncated = true;
                break;
            }
            map.insert(
                cmap_bytes_to_code(&pair[0]),
                cmap_bytes_to_unicode(&pair[1]),
            );
        }
        search_from = section_start + end + "endbfchar".len();
    }

    // bfrange: <lo> <hi> <dst-start>  OR  <lo> <hi> [<dst> ...].
    search_from = 0;
    while let Some(begin) = text[search_from..].find("beginbfrange") {
        let section_start = search_from + begin;
        let Some(end) = text[section_start..].find("endbfrange") else {
            break;
        };
        let section = &text[section_start..section_start + end];
        // Tokenize with array brackets preserved.
        let mut tokens: Vec<CmapToken> = Vec::new();
        let sb = section.as_bytes();
        let mut i = 0;
        while i < sb.len() {
            match sb[i] {
                b'<' => {
                    if let Some(rel) = section[i..].find('>') {
                        if let Some(parsed) = parse_cmap_hex(&section[i + 1..i + rel]) {
                            tokens.push(CmapToken::Hex(parsed));
                        }
                        i += rel + 1;
                        continue;
                    }
                    break;
                }
                b'[' => {
                    tokens.push(CmapToken::ArrayStart);
                    i += 1;
                }
                b']' => {
                    tokens.push(CmapToken::ArrayEnd);
                    i += 1;
                }
                _ => i += 1,
            }
        }
        let mut t = 0;
        while t + 2 < tokens.len() {
            let (Some(lo), Some(hi)) = (
                tokens[t].as_hex().map(|b| cmap_bytes_to_code(b)),
                tokens[t + 1].as_hex().map(|b| cmap_bytes_to_code(b)),
            ) else {
                t += 1;
                continue;
            };
            if hi < lo {
                t += 1;
                continue;
            }
            let span = (hi - lo + 1) as usize;
            match &tokens[t + 2] {
                CmapToken::Hex(dst_start) => {
                    let base_units: Vec<u16> = if dst_start.len() % 2 == 0 {
                        dst_start
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|c| u16::from_be_bytes([c[0], c[1]]))
                            .collect()
                    } else {
                        Vec::new()
                    };
                    for offset in 0..span {
                        if map.len() >= MAX_ENTRIES {
                            truncated = true;
                            break;
                        }
                        let s = if base_units.is_empty() {
                            cmap_bytes_to_unicode(dst_start)
                        } else {
                            let mut units = base_units.clone();
                            let last = units.len() - 1;
                            units[last] = units[last].wrapping_add(offset as u16);
                            units
                                .iter()
                                .map(|u| char::from_u32(*u as u32).unwrap_or('\u{FFFD}'))
                                .collect()
                        };
                        map.insert(lo + offset as u32, s);
                        // Guard against multi-million spans (e.g. <0000>-<FFFF>).
                        if offset >= MAX_ENTRIES {
                            truncated = true;
                            break;
                        }
                    }
                    t += 3;
                }
                CmapToken::ArrayStart => {
                    let mut idx = 0usize;
                    t += 3;
                    while t < tokens.len() {
                        match &tokens[t] {
                            CmapToken::Hex(dst) => {
                                if idx < span && map.len() < MAX_ENTRIES {
                                    map.insert(lo + idx as u32, cmap_bytes_to_unicode(dst));
                                } else if map.len() >= MAX_ENTRIES {
                                    truncated = true;
                                }
                                idx += 1;
                                t += 1;
                            }
                            CmapToken::ArrayEnd => {
                                t += 1;
                                break;
                            }
                            CmapToken::ArrayStart => t += 1,
                        }
                    }
                }
                CmapToken::ArrayEnd => t += 1,
            }
            if truncated {
                break;
            }
        }
        search_from = section_start + end + "endbfrange".len();
    }
    (map, truncated)
}

#[derive(Debug)]
enum CmapToken {
    Hex(Vec<u8>),
    ArrayStart,
    ArrayEnd,
}

impl CmapToken {
    fn as_hex(&self) -> Option<&Vec<u8>> {
        match self {
            CmapToken::Hex(b) => Some(b),
            _ => None,
        }
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
    /// Clip paths accumulated by PDF W/W* operators. Paths are in document
    /// coordinates and therefore survive later CTM changes like PDF clips.
    clip_paths: Vec<Object>,
    pending_clip: Option<bool>,
    // Text state.
    in_text: bool,
    font_family: String,
    /// Resource name from `Tf` (re-resolved per content stream so Form
    /// sub-resources with the same name work).
    font_name: String,
    font_size: f64,
    /// Nonstroking (`ca`) and stroking (`CA`) alpha from ExtGState.
    fill_alpha: f32,
    stroke_alpha: f32,
    blend: BlendMode,
    /// Overprint flags (`OP` fill-side, `op` stroke-side) from ExtGState.
    overprint_fill: bool,
    overprint_stroke: bool,
    /// `/ToUnicode` CMap for the current font, when present. `Rc` keeps
    /// `q`-pushes cheap even for large CJK maps.
    font_tounicode: Option<std::rc::Rc<std::collections::HashMap<u32, String>>>,
    /// True for composite (`/Type0`) fonts whose text bytes are 2-byte CIDs.
    font_is_composite: bool,
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
            clip_paths: Vec::new(),
            pending_clip: None,
            in_text: false,
            font_family: "sans-serif".to_string(),
            font_name: String::new(),
            font_size: 12.0,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend: BlendMode::Normal,
            overprint_fill: false,
            overprint_stroke: false,
            font_tounicode: None,
            font_is_composite: false,
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
    /// Form XObject nesting depth — a cyclic / self-referential Form
    /// would otherwise recurse until the stack overflows.
    form_depth: usize,
    /// Type 3 glyph-program nesting depth (a glyph showing text in its own
    /// font would otherwise recurse the same way).
    glyph_depth: usize,
    /// Operations processed in this import (DoS budget).
    op_count: usize,
    /// Objects emitted in this import (million-Tj bomb guard).
    obj_count: usize,
}

/// Hard cap on Form XObject nesting (PDF 32000-1 §8.10.1 Form XObjects).
const MAX_FORM_DEPTH: usize = 32;

impl<'a> Importer<'a> {
    fn new(doc: &'a lopdf::Document) -> Self {
        let out = Document {
            name: "PDF Import".to_string(),
            ..Default::default()
        };
        Self {
            doc,
            out,
            warnings: Vec::new(),
            warned: std::collections::HashSet::new(),
            seq: 0,
            page_no: 0,
            form_depth: 0,
            glyph_depth: 0,
            op_count: 0,
            obj_count: 0,
        }
    }

    /// Capped emission: million-object content streams stop here with
    /// a warning instead of OOMing the canvas.
    fn emit_object(&mut self, obj: Object) {
        const MAX_OBJECTS: usize = 200_000;
        if self.obj_count >= MAX_OBJECTS {
            self.warn(
                "object-budget",
                "オブジェクトが多すぎるため残りをスキップしました".to_string(),
            );
            return;
        }
        self.obj_count += 1;
        self.out.add_object(obj);
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

    /// Max decompressed stream payload (Flate bomb guard: a small
    /// stream must not expand to gigabytes before anything checks).
    const MAX_STREAM_BYTES: usize = 128 * 1024 * 1024;

    /// Stream payload: decompressed when filtered, raw otherwise.
    fn stream_bytes(stream: &lopdf::Stream) -> Result<Vec<u8>, String> {
        // Cheap pre-check on the wire size: a declared /Length far above
        // the cap cannot decode into anything usable.
        if stream.content.len() > Self::MAX_STREAM_BYTES {
            return Err("ストリームが大きすぎます".to_string());
        }
        let bytes = if stream.dict.get(b"Filter").is_ok() {
            stream
                .decompressed_content()
                .map_err(|e| format!("展開失敗: {e}"))?
        } else {
            stream.content.clone()
        };
        if bytes.len() > Self::MAX_STREAM_BYTES {
            return Err("展開後のストリームが大きすぎます".to_string());
        }
        Ok(bytes)
    }

    fn run(&mut self) -> Result<(), String> {
        let pages = self.doc.get_pages();
        if pages.is_empty() {
            return Err("ページがありません".to_string());
        }
        // First pass: boxes for vertical stacking.
        struct Box_ {
            id: lopdf::ObjectId,
            y_off: f64,
            bounds: (f64, f64, f64, f64),
            rotation: i32,
        }
        let mut boxes = Vec::new();
        let mut total_h = 0.0;
        let mut max_w = 0.0;
        for id in pages.values() {
            let (bounds, rotation) = self.page_geometry(*id);
            let (x0, y0, x1, y1) = bounds;
            let (w, h) = (x1 - x0, y1 - y0);
            let (w, h) = if rotation % 180 == 0 { (w, h) } else { (h, w) };
            if w > max_w {
                max_w = w;
            }
            boxes.push(Box_ {
                id: *id,
                y_off: total_h,
                bounds,
                rotation,
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
            // Map the selected page box to canvas coordinates. PDF /Rotate is
            // clockwise; normalize malformed values to the nearest quarter turn.
            let (x0, y0, x1, y1) = b.bounds;
            let rotation = b.rotation.rem_euclid(360);
            let base: Affine = match rotation {
                90 => [0.0, 1.0, 1.0, 0.0, -y0, b.y_off - x0],
                180 => [-1.0, 0.0, 0.0, 1.0, x1, b.y_off - y0],
                270 => [0.0, -1.0, -1.0, 0.0, y1, b.y_off + x1],
                _ => [1.0, 0.0, 0.0, -1.0, -x0, b.y_off + y1],
            };
            let mut st = GState {
                ctm: base,
                ..Default::default()
            };
            let resources = self.page_resources(b.id);
            let contents = self.doc.get_page_contents(b.id);
            for cid in contents {
                let obj = self
                    .doc
                    .get_object(cid)
                    .map_err(|e| format!("Content取得失敗: {e}"))?;
                let obj = self.resolve(obj);
                let stream = obj
                    .as_stream()
                    .map_err(|_| "Contentがstreamではありません".to_string())?;
                // Unfiltered streams have no /Filter key (lopdf errors on
                // those) — use the raw bytes directly.
                let bytes = Self::stream_bytes(stream)?;
                if bytes.is_empty() && !stream.content.is_empty() {
                    // lopdf reports undecodable filter chains (e.g. broken
                    // Flate payloads) as empty output instead of an error;
                    // keep the import but say so instead of silently
                    // dropping the page content.
                    self.warn(
                        "content-decode",
                        "内容ストリームを展開できないためスキップしました".to_string(),
                    );
                    continue;
                }
                let content =
                    Content::decode(&bytes).map_err(|e| format!("Content解析失敗: {e}"))?;
                self.run_ops(&content.operations, &resources, &mut st, &suffix)?;
            }
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

    fn page_geometry(&self, id: lopdf::ObjectId) -> ((f64, f64, f64, f64), i32) {
        let d = match self.page_dict(id) {
            Some(d) => d,
            None => return ((0.0, 0.0, 595.0, 842.0), 0),
        };
        let read_box = |key: &[u8]| {
            d.get(key)
                .ok()
                .map(|o| self.resolve(o))
                .and_then(|o| match o {
                    lopdf::Object::Array(a) if a.len() == 4 => {
                        Some((num(&a[0]), num(&a[1]), num(&a[2]), num(&a[3])))
                    }
                    _ => None,
                })
        };
        // CropBox is intersected with MediaBox by PDF viewers. Use its
        // visible portion and retain non-zero page-box origins in the matrix.
        let media = read_box(b"MediaBox").unwrap_or((0.0, 0.0, 595.0, 842.0));
        let requested = read_box(b"CropBox").unwrap_or(media);
        let bounds = (
            requested.0.max(media.0),
            requested.1.max(media.1),
            requested.2.min(media.2),
            requested.3.min(media.3),
        );
        let bounds = if bounds.2 > bounds.0 && bounds.3 > bounds.1 {
            bounds
        } else {
            media
        };
        let rotation = d
            .get(b"Rotate")
            .ok()
            .map(|o| num(self.resolve(o)) as i32)
            .unwrap_or(0)
            .div_euclid(90)
            * 90;
        (bounds, rotation.rem_euclid(360))
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
        /// Max graphic-state nesting (`q` without `Q` clones full state
        /// plus path per level: quadratic memory without a cap).
        const MAX_Q_DEPTH: usize = 64;
        /// Max operations per import (million-op content streams stall the
        /// UI thread; content past the budget is skipped with a warning).
        const MAX_OPS: usize = 2_000_000;
        let mut stack: Vec<GState> = Vec::new();
        for op in ops {
            self.op_count += 1;
            if self.op_count > MAX_OPS {
                self.warn(
                    "op-budget",
                    "演算子が多すぎるため残りをスキップしました".to_string(),
                );
                break;
            }
            let o = op.operands.as_slice();
            match op.operator.as_str() {
                "q" => {
                    if stack.len() >= MAX_Q_DEPTH {
                        self.warn(
                            "q-depth",
                            "グラフィック状態のネストが深すぎるためスキップしました".to_string(),
                        );
                    } else {
                        stack.push(st.clone());
                    }
                }
                "Q" => {
                    if let Some(s) = stack.pop() {
                        let path = std::mem::take(&mut st.path);
                        *st = s;
                        st.path = path;
                    }
                }
                "cm" if o.len() == 6 => {
                    let m = [
                        num(&o[0]),
                        num(&o[1]),
                        num(&o[2]),
                        num(&o[3]),
                        num(&o[4]),
                        num(&o[5]),
                    ];
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
                        self.ensure_stroke(st).dash_pattern =
                            if pat.is_empty() { None } else { Some(pat) };
                    }
                }
                "gs" if !o.is_empty() => {
                    if let Some(gs_name) = name_str(&o[0]) {
                        self.apply_extgstate(resources, &gs_name, st);
                    }
                }
                "scn" | "SCN" => {
                    self.warn(
                        "pattern-paint",
                        "パターン指定の塗りは単色のまま表示します".to_string(),
                    );
                }
                "cs" | "CS" if !o.is_empty() => {
                    if matches!(name_str(&o[0]).as_deref(), Some("Pattern")) {
                        self.warn(
                            "pattern-paint",
                            "パターン指定の塗りは単色のまま表示します".to_string(),
                        );
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
                    let c = [num(&o[0]) as f32, num(&o[1]) as f32, num(&o[2]) as f32, 1.0];
                    self.ensure_stroke(st).color = c;
                }
                "rg" if o.len() == 3 => {
                    st.fill = [num(&o[0]) as f32, num(&o[1]) as f32, num(&o[2]) as f32, 1.0];
                }
                "K" if o.len() == 4 => {
                    self.ensure_stroke(st).color =
                        cmyk_to_rgb(num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3]));
                }
                "k" if o.len() == 4 => {
                    st.fill = cmyk_to_rgb(num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3]));
                }
                "m" if o.len() == 2 => st.path.push(Seg::Move(num(&o[0]), num(&o[1]))),
                "l" if o.len() == 2 => st.path.push(Seg::Line(num(&o[0]), num(&o[1]))),
                "c" if o.len() == 6 => st.path.push(Seg::Curve(
                    num(&o[0]),
                    num(&o[1]),
                    num(&o[2]),
                    num(&o[3]),
                    num(&o[4]),
                    num(&o[5]),
                )),
                "v" if o.len() == 4 => {
                    let (x, y) = self.current_point(st);
                    st.path.push(Seg::Curve(
                        x,
                        y,
                        num(&o[0]),
                        num(&o[1]),
                        num(&o[2]),
                        num(&o[3]),
                    ));
                }
                "y" if o.len() == 4 => {
                    // Second control point coincides with the end point;
                    // the first one is the current point.
                    let (x, y) = self.current_point(st);
                    st.path.push(Seg::Curve(
                        x,
                        y,
                        num(&o[0]),
                        num(&o[1]),
                        num(&o[2]),
                        num(&o[3]),
                    ));
                }
                "h" => st.path.push(Seg::Close),
                "re" if o.len() == 4 => {
                    st.path
                        .push(Seg::Rect(num(&o[0]), num(&o[1]), num(&o[2]), num(&o[3])));
                }
                "n" => {
                    self.apply_pending_clip(st, suffix);
                    st.path.clear();
                }
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
                    st.pending_clip = Some(op.operator == "W*");
                }
                "sh" => {
                    if let Some(name) = o.first().and_then(name_str) {
                        self.emit_shading(resources, &name, st, suffix);
                    } else {
                        self.warn("shading", "シェーディングは無視されました".to_string());
                    }
                }
                "BT" => {
                    st.in_text = true;
                    st.tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    st.x_ts = 0.0;
                    st.leading = 0.0;
                }
                "ET" => st.in_text = false,
                "Tf" if o.len() == 2 => {
                    let name = name_str(&o[0]).unwrap_or_default();
                    let (family, map, composite) = self.font_details(resources, &name);
                    st.font_family = family;
                    st.font_name = name;
                    st.font_tounicode = map;
                    st.font_is_composite = composite;
                    st.font_size = num(&o[1]).abs().max(0.5);
                }
                "Tm" if o.len() == 6 => {
                    st.tm = [
                        num(&o[0]),
                        num(&o[1]),
                        num(&o[2]),
                        num(&o[3]),
                        num(&o[4]),
                        num(&o[5]),
                    ];
                    st.x_ts = 0.0;
                }
                "Td" if o.len() == 2 => {
                    // Translation applies in text space (Tm = T*Tm), so the
                    // offsets ride the current scale, not raw user units.
                    let (tx, ty) = (num(&o[0]), num(&o[1]));
                    st.tm[4] += tx * st.tm[0] + ty * st.tm[2];
                    st.tm[5] += tx * st.tm[1] + ty * st.tm[3];
                    st.x_ts = 0.0;
                }
                "TD" if o.len() == 2 => {
                    st.leading = -num(&o[1]);
                    let (tx, ty) = (num(&o[0]), num(&o[1]));
                    st.tm[4] += tx * st.tm[0] + ty * st.tm[2];
                    st.tm[5] += tx * st.tm[1] + ty * st.tm[3];
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
                        if self.is_type3_font(resources, &st.font_name) {
                            self.emit_type3_text(b, resources, st, suffix);
                        } else {
                            let s = self.decode_text_bytes(b, st);
                            self.emit_text(st, &s, suffix);
                        }
                    }
                }
                "TJ" if !o.is_empty() => {
                    if let lopdf::Object::Array(items) = &o[0] {
                        let is_type3 = self.is_type3_font(resources, &st.font_name);
                        for item in items {
                            if let Some(b) = str_bytes(item) {
                                if is_type3 {
                                    self.emit_type3_text(b, resources, st, suffix);
                                } else {
                                    let s = self.decode_text_bytes(b, st);
                                    self.emit_text(st, &s, suffix);
                                }
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
                        if self.is_type3_font(resources, &st.font_name) {
                            self.emit_type3_text(b, resources, st, suffix);
                        } else {
                            let s = self.decode_text_bytes(b, st);
                            self.emit_text(st, &s, suffix);
                        }
                    }
                }
                "\"" if o.len() == 3 => {
                    st.word_space = num(&o[0]);
                    st.char_space = num(&o[1]);
                    st.tm[5] -= st.leading;
                    st.x_ts = 0.0;
                    if let Some(b) = str_bytes(&o[2]) {
                        if self.is_type3_font(resources, &st.font_name) {
                            self.emit_type3_text(b, resources, st, suffix);
                        } else {
                            let s = self.decode_text_bytes(b, st);
                            self.emit_text(st, &s, suffix);
                        }
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

    fn emit_path(
        &mut self,
        st: &mut GState,
        fill: bool,
        stroke: bool,
        even_odd: bool,
        suffix: &str,
    ) {
        if st.path.is_empty() {
            return;
        }
        let mut path = path_from_segments(&st.path, &st.ctm);
        let clip_mask = st
            .pending_clip
            .map(|rule| (path_from_segments(&st.path, &st.ctm), rule));
        st.path.clear();
        if path.elements.is_empty() {
            return;
        }
        path.fill = if fill {
            let mut color = st.fill;
            color[3] *= st.fill_alpha;
            Some(FillStyle {
                color,
                fill_type: crate::core::path::FillType::Solid(color),
                rule: if even_odd {
                    FillRule::EvenOdd
                } else {
                    FillRule::NonZero
                },
                overprint: st.overprint_fill,
                spot: None,
            })
        } else {
            None
        };
        path.stroke = if stroke {
            st.stroke.clone().map(|mut s| {
                s.color[3] *= st.stroke_alpha;
                s.overprint = st.overprint_stroke;
                s
            })
        } else {
            None
        };
        self.seq += 1;
        let mut obj = Object::new_path(&format!("PDF Path {:03}{suffix}", self.seq), path);
        obj.blend_mode = st.blend;
        // Geometry is baked to device space already.
        if let Some((mask_path, even_odd)) = clip_mask {
            self.push_clip_mask(st, mask_path, even_odd, suffix);
        }
        self.emit_clipped_object(obj, st);
    }

    fn emit_text(&mut self, st: &mut GState, s: &str, suffix: &str) {
        if s.is_empty() || !st.in_text {
            return;
        }
        let ex = mat_xscale(&st.ctm).max(1e-6);
        // Pen: text-space advance mapped through Tm scale, then CTM.
        // (Standard files use translation-only Tm, so this is exact there.)
        let (ux, uy) = (st.tm[4] + st.x_ts * st.tm[0], st.tm[5] + st.x_ts * st.tm[1]);
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
        obj.fill = Some({
            let mut color = st.fill;
            color[3] *= st.fill_alpha;
            let mut fill = FillStyle::solid(color);
            fill.overprint = st.overprint_fill;
            fill
        });
        obj.blend_mode = st.blend;
        self.emit_clipped_object(obj, st);
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

    fn apply_pending_clip(&mut self, st: &mut GState, suffix: &str) {
        let Some(even_odd) = st.pending_clip.take() else {
            return;
        };
        if st.path.is_empty() {
            return;
        }
        let path = path_from_segments(&st.path, &st.ctm);
        self.push_clip_mask(st, path, even_odd, suffix);
    }

    fn push_clip_mask(
        &mut self,
        st: &mut GState,
        mut path: PathData,
        even_odd: bool,
        suffix: &str,
    ) {
        if path.elements.is_empty() {
            return;
        }
        path.fill = Some(FillStyle {
            color: [0.0, 0.0, 0.0, 1.0],
            fill_type: crate::core::path::FillType::Solid([0.0, 0.0, 0.0, 1.0]),
            rule: if even_odd {
                FillRule::EvenOdd
            } else {
                FillRule::NonZero
            },
            overprint: false,
            spot: None,
        });
        path.stroke = None;
        self.seq += 1;
        let mask = Object::new_path(&format!("PDF Clip {:03}{suffix}", self.seq), path);
        st.clip_paths.push(mask);
        st.pending_clip = None;
    }

    fn emit_clipped_object(&mut self, mut obj: Object, st: &GState) {
        for mask in st.clip_paths.iter().rev() {
            obj = Object {
                object_type: ObjectType::ClippingMask {
                    children: vec![mask.clone(), obj],
                },
                ..Object::new_group("PDF Clip Group", Vec::new())
            };
        }
        self.emit_object(obj);
    }

    /// Paint axial (`ShadingType 2`) and radial (`ShadingType 3`) shadings
    /// as gradient rects. Only exponential (`FunctionType 2`) and stitching
    /// (`FunctionType 3` of type-2 segments) functions over device RGB/Gray/
    /// CMYK are sampled; anything else warns and skips so missing paint
    /// stays explicit.
    fn emit_shading(
        &mut self,
        resources: &Option<lopdf::Dictionary>,
        name: &str,
        st: &mut GState,
        suffix: &str,
    ) {
        const STOPS: usize = 16;
        let Some((shading, _data)) = self.shading_entry(resources, name) else {
            self.warn("shading", format!("シェーディング {name} が見つかりません"));
            return;
        };
        let shading_type = shading
            .get(b"ShadingType")
            .ok()
            .map(|o| num(self.resolve(o)) as i32)
            .unwrap_or(0);
        if (4..=7).contains(&shading_type) {
            self.emit_mesh_shading(resources, name, st, suffix, shading_type);
            return;
        }
        if shading_type != 2 && shading_type != 3 {
            self.warn(
                "shading",
                format!("シェーディング種別 {shading_type} は未対応のため省略"),
            );
            return;
        }
        // Output model for sampled function values.
        let space: Option<ShadingSpace> =
            match shading.get(b"ColorSpace").ok().map(|o| self.resolve(o)) {
                Some(lopdf::Object::Name(n)) => {
                    let s = String::from_utf8_lossy(n).into_owned();
                    if s.contains("DeviceRGB") || s.contains("CalRGB") {
                        Some(ShadingSpace::Rgb)
                    } else if s.contains("DeviceGray") || s.contains("CalGray") {
                        Some(ShadingSpace::Gray)
                    } else if s.contains("DeviceCMYK") {
                        Some(ShadingSpace::Cmyk)
                    } else {
                        None
                    }
                }
                Some(lopdf::Object::Array(a)) => {
                    let head = a
                        .first()
                        .and_then(|e| name_str(self.resolve(e)))
                        .unwrap_or_default();
                    if head == "Lab" {
                        match self.resolve(a.get(1).unwrap_or(&lopdf::Object::Null)) {
                            lopdf::Object::Dictionary(d) => {
                                self.lab_params_from_dict(d).map(ShadingSpace::Lab)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };
        let Some(space) = space else {
            self.warn(
                "shading",
                format!("シェーディング {name} の色空間は未対応のため省略"),
            );
            return;
        };
        let components: usize = match &space {
            ShadingSpace::Rgb | ShadingSpace::Lab(_) => 3,
            ShadingSpace::Gray => 1,
            ShadingSpace::Cmyk => 4,
        };
        let coords: Vec<f64> = match shading.get(b"Coords").ok().map(|o| self.resolve(o)) {
            Some(lopdf::Object::Array(a)) => a.iter().map(num).collect(),
            _ => {
                self.warn(
                    "shading",
                    format!("シェーディング {name} の座標が不正のため省略"),
                );
                return;
            }
        };
        if (shading_type == 2 && coords.len() != 4) || (shading_type == 3 && coords.len() != 6) {
            self.warn(
                "shading",
                format!("シェーディング {name} の座標が不正のため省略"),
            );
            return;
        }
        let domain: (f64, f64) = match shading.get(b"Domain").ok().map(|o| self.resolve(o)) {
            Some(lopdf::Object::Array(a)) if a.len() == 2 => (num(&a[0]), num(&a[1])),
            _ => (0.0, 1.0),
        };
        let Some(func) = shading
            .get(b"Function")
            .ok()
            .map(|o| self.resolve(o).clone())
        else {
            self.warn(
                "shading",
                format!("シェーディング {name} に関数がないため省略"),
            );
            return;
        };
        let mut stops = Vec::with_capacity(STOPS);
        for i in 0..STOPS {
            let t = domain.0 + (domain.1 - domain.0) * (i as f64 / (STOPS - 1) as f64);
            let Some(values) = self.eval_shading_function(&func, t, components, 0) else {
                self.warn(
                    "shading",
                    format!("シェーディング {name} の関数が未対応のため省略"),
                );
                return;
            };
            let color = shading_values_to_rgb(&values, &space);
            stops.push(crate::core::path::GradientStop {
                offset: i as f32 / (STOPS - 1) as f32,
                color,
            });
        }
        for stop in &mut stops {
            stop.color[3] *= st.fill_alpha;
        }
        // Axis endpoints (axial) or circles (radial) through the CTM.
        if shading_type == 2 {
            let (p0x, p0y) = mat_apply(&st.ctm, coords[0], coords[1]);
            let (p1x, p1y) = mat_apply(&st.ctm, coords[2], coords[3]);
            if (p0x - p1x).hypot(p0y - p1y) < 1e-9 {
                self.warn(
                    "shading",
                    format!("シェーディング {name} の軸が縮退のため省略"),
                );
                return;
            }
            let Some((lx, ty, rx, by)) = self.shading_extent(&shading, st, name) else {
                return;
            };
            let gradient = crate::core::path::LinearGradient {
                start_x: ((p0x - lx) / (rx - lx)) as f32,
                start_y: ((p0y - ty) / (by - ty)) as f32,
                end_x: ((p1x - lx) / (rx - lx)) as f32,
                end_y: ((p1y - ty) / (by - ty)) as f32,
                stops,
            };
            self.seq += 1;
            let mut obj = Object::new_rect(
                &format!("PDF Shading {:03}{suffix}", self.seq),
                lx,
                ty,
                rx - lx,
                by - ty,
                0.0,
            );
            obj.fill = Some(FillStyle::linear_gradient(gradient));
            obj.blend_mode = st.blend;
            self.emit_clipped_object(obj, st);
            return;
        }
        // Radial: outer circle maps to the gradient disc; the inner centre
        // becomes the focal point. A non-zero inner radius has no counterpart
        // in the single-disc model, so its interior keeps the first stop
        // (correct when the inner circle is a point, close otherwise).
        let sx = (st.ctm[0] * st.ctm[0] + st.ctm[1] * st.ctm[1]).sqrt();
        let sy = (st.ctm[2] * st.ctm[2] + st.ctm[3] * st.ctm[3]).sqrt();
        let uniform = sx.max(1e-12).min(sy.max(1e-12));
        if sx <= 1e-12 || sy <= 1e-12 || (sx - sy).abs() > 1e-9 * sx.max(sy).max(1.0) {
            self.warn(
                "shading",
                format!("シェーディング {name} は非等方変換のため省略"),
            );
            return;
        }
        let (c0x, c0y) = mat_apply(&st.ctm, coords[0], coords[1]);
        let (c1x, c1y) = mat_apply(&st.ctm, coords[3], coords[4]);
        let (r0, r1) = (coords[2] * uniform, coords[5] * uniform);
        if r1 <= 1e-9 {
            self.warn(
                "shading",
                format!("シェーディング {name} の半径が縮退のため省略"),
            );
            return;
        }
        let Some((lx, ty, rx, by)) = self.shading_extent(&shading, st, name) else {
            return;
        };
        // Square paint keeps document-space circles circular; the /BBox part
        // is enforced by a temporary clip so corners never overpaint.
        let (cx, cy) = ((lx + rx) * 0.5, (ty + by) * 0.5);
        let side = (rx - lx).max(by - ty);
        if side <= 0.0 {
            return;
        }
        let (slx, sty) = (cx - side * 0.5, cy - side * 0.5);
        let gradient = crate::core::path::RadialGradient {
            center_x: ((c1x - slx) / side) as f32,
            center_y: ((c1y - sty) / side) as f32,
            radius: (r1 / side) as f32,
            focus_x: ((c0x - slx) / side) as f32,
            focus_y: ((c0y - sty) / side) as f32,
            stops,
        };
        if !gradient.radius.is_finite() || gradient.radius <= 0.0 {
            self.warn(
                "shading",
                format!("シェーディング {name} の半径が不正のため省略"),
            );
            return;
        }
        if r0 > 1e-9 * r1.max(1.0) {
            self.warn(
                "shading-inner-radius",
                format!("シェーディング {name} の内径は近似表示されます"),
            );
        }
        let _ = r0;
        self.push_rect_clip(st, lx, ty, rx - lx, by - ty, suffix);
        self.seq += 1;
        let mut obj = Object::new_rect(
            &format!("PDF Shading {:03}{suffix}", self.seq),
            slx,
            sty,
            side,
            side,
            0.0,
        );
        obj.fill = Some(FillStyle::radial_gradient(gradient));
        obj.blend_mode = st.blend;
        self.emit_clipped_object(obj, st);
        st.clip_paths.pop();
    }

    /// Finite paint rect: shading /BBox intersected with the active clip.
    /// Without either the paint is unbounded, so warn and skip explicitly.
    fn shading_extent(
        &mut self,
        shading: &lopdf::Dictionary,
        st: &GState,
        name: &str,
    ) -> Option<(f64, f64, f64, f64)> {
        let mut rect: Option<(f64, f64, f64, f64)> = None;
        if let Some(lopdf::Object::Array(a)) = shading.get(b"BBox").ok().map(|o| self.resolve(o)) {
            if a.len() == 4 {
                let corners = [
                    mat_apply(&st.ctm, num(&a[0]), num(&a[1])),
                    mat_apply(&st.ctm, num(&a[2]), num(&a[1])),
                    mat_apply(&st.ctm, num(&a[0]), num(&a[3])),
                    mat_apply(&st.ctm, num(&a[2]), num(&a[3])),
                ];
                let (mut lx, mut ty, mut rx, mut by) = (
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                );
                for (x, y) in corners {
                    lx = lx.min(x);
                    ty = ty.min(y);
                    rx = rx.max(x);
                    by = by.max(y);
                }
                rect = Some((lx, ty, rx, by));
            }
        }
        if let Some(clip) = self.clip_bbox(st) {
            rect = Some(match rect {
                Some(r) => (
                    r.0.max(clip.0),
                    r.1.max(clip.1),
                    r.2.min(clip.2),
                    r.3.min(clip.3),
                ),
                None => clip,
            });
        }
        let rect = match rect {
            Some(r) => r,
            None => {
                self.warn(
                    "shading",
                    format!("シェーディング {name} の範囲が不定のため省略"),
                );
                return None;
            }
        };
        if rect.2 - rect.0 <= 0.0 || rect.3 - rect.1 <= 0.0 {
            return None;
        }
        Some(rect)
    }

    /// Push an axis-aligned rect as a temporary clip mask.
    fn push_rect_clip(&mut self, st: &mut GState, x: f64, y: f64, w: f64, h: f64, suffix: &str) {
        if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
            return;
        }
        let mut path = PathData::new();
        path.push_move_to(x, y);
        path.push_line_to(x + w, y);
        path.push_line_to(x + w, y + h);
        path.push_line_to(x, y + h);
        path.elements.push(PathElement::ClosePath);
        self.push_clip_mask(st, path, false, suffix);
    }

    /// Paint free-form/lattice Gouraud (`ShadingType 4/5`) and Coons/tensor
    /// (`ShadingType 6/7`) meshes as flat triangles. Gouraud triangles average
    /// vertex colors; patches subdivide (8×8) with boundary-true geometry and
    /// bilinear corner colors. Output stops at a triangle budget so dense
    /// meshes degrade instead of stalling.
    fn emit_mesh_shading(
        &mut self,
        resources: &Option<lopdf::Dictionary>,
        name: &str,
        st: &mut GState,
        suffix: &str,
        shading_type: i32,
    ) {
        const MAX_TRIANGLES: usize = 20000;
        let Some((shading, data)) = self.shading_entry(resources, name) else {
            self.warn("shading", format!("シェーディング {name} が見つかりません"));
            return;
        };
        let Some(data) = data else {
            self.warn(
                "shading",
                format!("シェーディング {name} に頂点データがありません"),
            );
            return;
        };
        // Color model shared with the axial/radial sampler.
        let space: Option<ShadingSpace> =
            match shading.get(b"ColorSpace").ok().map(|o| self.resolve(o)) {
                Some(lopdf::Object::Name(n)) => {
                    let s = String::from_utf8_lossy(n).into_owned();
                    if s.contains("DeviceRGB") || s.contains("CalRGB") {
                        Some(ShadingSpace::Rgb)
                    } else if s.contains("DeviceGray") || s.contains("CalGray") {
                        Some(ShadingSpace::Gray)
                    } else if s.contains("DeviceCMYK") {
                        Some(ShadingSpace::Cmyk)
                    } else {
                        None
                    }
                }
                Some(lopdf::Object::Array(a)) => {
                    let head = a
                        .first()
                        .and_then(|e| name_str(self.resolve(e)))
                        .unwrap_or_default();
                    if head == "Lab" {
                        match self.resolve(a.get(1).unwrap_or(&lopdf::Object::Null)) {
                            lopdf::Object::Dictionary(d) => {
                                self.lab_params_from_dict(d).map(ShadingSpace::Lab)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };
        let Some(space) = space else {
            self.warn(
                "shading",
                format!("シェーディング {name} の色空間は未対応のため省略"),
            );
            return;
        };
        let components: usize = match &space {
            ShadingSpace::Rgb | ShadingSpace::Lab(_) => 3,
            ShadingSpace::Gray => 1,
            ShadingSpace::Cmyk => 4,
        };
        let num_param = |key: &[u8], default: f64| -> f64 {
            shading
                .get(key)
                .ok()
                .map(|o| num(self.resolve(o)))
                .unwrap_or(default)
        };
        let flag_bits = num_param(b"BitsPerFlag", 2.0) as u32;
        let coord_bits = num_param(b"BitsPerCoordinate", 8.0) as u32;
        let comp_bits = num_param(b"BitsPerComponent", 8.0) as u32;
        if !matches!(flag_bits, 2 | 4 | 8)
            || !(1..=32).contains(&coord_bits)
            || !matches!(comp_bits, 1 | 2 | 4 | 8 | 12 | 16)
        {
            self.warn(
                "shading",
                format!("シェーディング {name} のビット指定が未対応のため省略"),
            );
            return;
        }
        // Explicit /Decode maps raw integers to shading-space values.
        let decode: Vec<f64> = match shading.get(b"Decode").ok().map(|o| self.resolve(o)) {
            Some(lopdf::Object::Array(a)) => a.iter().map(num).collect(),
            _ => vec![0.0, 1.0, 0.0, 1.0],
        };
        if decode.len() != 4 + 2 * components {
            self.warn(
                "shading",
                format!("シェーディング {name} のDecodeが不正のため省略"),
            );
            return;
        }
        // Coons/tensor patches decode through their own record loop.
        if shading_type == 6 || shading_type == 7 {
            match self.decode_patch_mesh(
                &data,
                &shading,
                &decode,
                components,
                MAX_TRIANGLES,
                name,
                shading_type,
            ) {
                Some(patch_tris) => {
                    self.emit_mesh_triangles(
                        &patch_tris,
                        &space,
                        None,
                        components,
                        st,
                        suffix,
                        name,
                        MAX_TRIANGLES,
                    );
                    return;
                }
                None => {
                    self.warn(
                        "shading",
                        format!("シェーディング {name} の頂点が読めないため省略"),
                    );
                    return;
                }
            }
        }
        let vertices = decode_mesh_vertices(
            &data,
            flag_bits,
            coord_bits,
            comp_bits,
            &decode,
            components,
            MAX_TRIANGLES * 3 + 2,
        );
        if vertices.is_empty() {
            self.warn(
                "shading",
                format!("シェーディング {name} の頂点が読めないため省略"),
            );
            return;
        }
        // Optional /BBox pre-clip in shading space (transformed below with
        // the vertices, compared in document space).
        let bbox: Option<(f64, f64, f64, f64)> =
            match shading.get(b"BBox").ok().map(|o| self.resolve(o)) {
                Some(lopdf::Object::Array(a)) if a.len() == 4 => {
                    Some((num(&a[0]), num(&a[1]), num(&a[2]), num(&a[3])))
                }
                _ => None,
            };
        // Assemble triangles: flag 0 restarts the strip, flags 1/2 continue
        // it; every third buffered vertex closes one triangle.
        let mut triangles: Vec<MeshTriangle> = Vec::new();
        if shading_type == 4 {
            let mut pending: Vec<(f64, f64, Vec<f64>)> = Vec::with_capacity(3);
            for (flag, x, y, color) in &vertices {
                if *flag == 0 {
                    pending.clear();
                }
                pending.push((*x, *y, color.clone()));
                if pending.len() == 3 {
                    triangles.push([pending[0].clone(), pending[1].clone(), pending[2].clone()]);
                    pending.drain(..1);
                    if triangles.len() >= MAX_TRIANGLES {
                        break;
                    }
                }
            }
        } else {
            let per_row = num_param(b"VerticesPerRow", 0.0) as usize;
            if per_row < 2 {
                self.warn(
                    "shading",
                    format!("シェーディング {name} の行幅が不正のため省略"),
                );
                return;
            }
            let rows = vertices.len() / per_row;
            for r in 0..rows.saturating_sub(1) {
                for c in 0..per_row.saturating_sub(1) {
                    let (a, b, cc, d) = (
                        &vertices[r * per_row + c],
                        &vertices[r * per_row + c + 1],
                        &vertices[(r + 1) * per_row + c + 1],
                        &vertices[(r + 1) * per_row + c],
                    );
                    triangles.push([
                        (a.1, a.2, a.3.clone()),
                        (b.1, b.2, b.3.clone()),
                        (d.1, d.2, d.3.clone()),
                    ]);
                    triangles.push([
                        (b.1, b.2, b.3.clone()),
                        (cc.1, cc.2, cc.3.clone()),
                        (d.1, d.2, d.3.clone()),
                    ]);
                    if triangles.len() >= MAX_TRIANGLES {
                        break;
                    }
                }
                if triangles.len() >= MAX_TRIANGLES {
                    break;
                }
            }
        }
        self.emit_mesh_triangles(
            &triangles,
            &space,
            bbox,
            components,
            st,
            suffix,
            name,
            MAX_TRIANGLES,
        );
    }

    /// Shared triangle sink for Gouraud and patch meshes: budget notice,
    /// sliver/BBox culls, per-triangle averaged fills.
    #[allow(clippy::too_many_arguments)]
    fn emit_mesh_triangles(
        &mut self,
        triangles: &[MeshTriangle],
        space: &ShadingSpace,
        bbox: Option<(f64, f64, f64, f64)>,
        components: usize,
        st: &mut GState,
        suffix: &str,
        name: &str,
        max_triangles: usize,
    ) {
        if triangles.is_empty() {
            self.warn(
                "shading",
                format!("シェーディング {name} から三角形が作れないため省略"),
            );
            return;
        }
        if triangles.len() >= max_triangles {
            self.warn(
                "mesh-budget",
                format!("メッシュが大きいため一部のみ表示します ({name})"),
            );
        }
        let mut emitted = 0usize;
        for tri in triangles {
            let pts: [(f64, f64); 3] = [
                mat_apply(&st.ctm, tri[0].0, tri[0].1),
                mat_apply(&st.ctm, tri[1].0, tri[1].1),
                mat_apply(&st.ctm, tri[2].0, tri[2].1),
            ];
            // Degenerate slivers rasterize to nothing; skip the object cost.
            let area = ((pts[1].0 - pts[0].0) * (pts[2].1 - pts[0].1)
                - (pts[2].0 - pts[0].0) * (pts[1].1 - pts[0].1))
                .abs()
                * 0.5;
            if !area.is_finite() || area <= 1e-9 {
                continue;
            }
            if let Some((x0, y0, x1, y1)) = bbox {
                // Shading-space disjoint test on the source triangle.
                let (blx, bty, brx, bby) = (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1));
                let (mut tlx, mut tty, mut trx, mut tby) = (
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                );
                for v in tri {
                    tlx = tlx.min(v.0);
                    tty = tty.min(v.1);
                    trx = trx.max(v.0);
                    tby = tby.max(v.1);
                }
                if trx < blx || tlx > brx || tby < bty || tty > bby {
                    continue;
                }
            }
            let mut avg = vec![0.0f64; components];
            for v in tri {
                for (j, acc) in avg.iter_mut().enumerate() {
                    *acc += v.2[j] / 3.0;
                }
            }
            let color = shading_values_to_rgb(&avg, space);
            let mut path = PathData::new();
            path.push_move_to(pts[0].0, pts[0].1);
            path.push_line_to(pts[1].0, pts[1].1);
            path.push_line_to(pts[2].0, pts[2].1);
            path.elements.push(PathElement::ClosePath);
            path.fill = Some(FillStyle {
                color,
                fill_type: crate::core::path::FillType::Solid(color),
                rule: FillRule::NonZero,
                overprint: st.overprint_fill,
                spot: None,
            });
            path.stroke = None;
            self.seq += 1;
            let mut obj = Object::new_path(&format!("PDF Mesh {:03}{suffix}", self.seq), path);
            obj.blend_mode = st.blend;
            self.emit_clipped_object(obj, st);
            emitted += 1;
        }
        if emitted == 0 {
            self.warn(
                "shading",
                format!("シェーディング {name} は表示範囲外のため省略"),
            );
        }
    }

    /// Shading entry: plain dicts return no data; mesh shadings (types
    /// 4–7) are streams whose content holds the vertex records.
    #[allow(clippy::too_many_lines)]
    fn shading_entry(
        &self,
        resources: &Option<lopdf::Dictionary>,
        name: &str,
    ) -> Option<(lopdf::Dictionary, Option<Vec<u8>>)> {
        let res = resources.as_ref()?;
        let all = self.resolve(res.get(b"Shading").ok()?);
        let entry = match all {
            lopdf::Object::Dictionary(map) => self.resolve(map.get(name.as_bytes()).ok()?),
            _ => return None,
        };
        match entry {
            lopdf::Object::Dictionary(d) => Some((d.clone(), None)),
            lopdf::Object::Stream(s) => {
                let dict = s.dict.clone();
                match Self::stream_bytes(s) {
                    Ok(data) => Some((dict, Some(data))),
                    Err(_) => Some((dict, None)),
                }
            }
            _ => None,
        }
    }

    /// Decode Coons (`ShadingType 6`) and tensor-product (`ShadingType 7`)
    /// patches into shading-space triangles (8×8 subdivision each).
    ///
    /// Record layout assumptions (PDF 32000 §8.7.4.5.6–7): Coons points run
    /// bottom L→R, right B→T, top R→L, left T→B with shared corners stored
    /// once; tensor points are row-major, bottom row first; corner colors
    /// run BL, BR, TR, TL. Flag-1 records continue the previous right edge
    /// (8 new points + 2 colors for Coons, 12 + 2 for tensor).
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::too_many_arguments)]
    fn decode_patch_mesh(
        &mut self,
        data: &[u8],
        shading: &lopdf::Dictionary,
        decode: &[f64],
        components: usize,
        max_triangles: usize,
        name: &str,
        shading_type: i32,
    ) -> Option<Vec<MeshTriangle>> {
        let num_param = |key: &[u8], default: f64| -> f64 {
            shading
                .get(key)
                .ok()
                .map(|o| num(self.resolve(o)))
                .unwrap_or(default)
        };
        let flag_bits = num_param(b"BitsPerFlag", 8.0) as u32;
        let coord_bits = num_param(b"BitsPerCoordinate", 8.0) as u32;
        let comp_bits = num_param(b"BitsPerComponent", 8.0) as u32;
        if !matches!(flag_bits, 2 | 4 | 8)
            || !(1..=32).contains(&coord_bits)
            || !matches!(comp_bits, 1 | 2 | 4 | 8 | 12 | 16)
        {
            self.warn(
                "shading",
                format!("シェーディング {name} のビット指定が未対応のため省略"),
            );
            return None;
        }
        let map = |raw: u64, bits: u32, lo: f64, hi: f64| -> f64 {
            lo + raw as f64 / ((1u64 << bits) - 1) as f64 * (hi - lo)
        };
        let mut bits = MeshBitReader { data, bit: 0 };
        // Previous right edge (points B→T + bottom/top colors) for flag 1.
        let mut prev_edge: Option<PatchEdge> = None;
        let mut triangles: Vec<MeshTriangle> = Vec::new();
        const SUB: usize = 8;
        loop {
            if triangles.len() >= max_triangles {
                break;
            }
            let Some(flag) = bits.read(flag_bits) else {
                break;
            };
            // Full vs continuation record sizes.
            let full_points = if shading_type == 6 { 12 } else { 16 };
            let cont_points = if shading_type == 6 { 8 } else { 12 };
            // Boundary ring (BL, BR, TR, TL corners) + corner colors, or
            // `None` when the stream ends or a flag-1 has no predecessor.
            enum Net {
                Coons {
                    bottom: [(f64, f64); 4],
                    right: [(f64, f64); 4],
                    top: [(f64, f64); 4],
                    left: [(f64, f64); 4],
                    corners: [Vec<f64>; 4],
                },
                Tensor {
                    grid: [[(f64, f64); 4]; 4],
                    corners: [Vec<f64>; 4],
                },
            }
            let read_point = |bits: &mut MeshBitReader| -> Option<(f64, f64)> {
                let rx = bits.read(coord_bits)?;
                let ry = bits.read(coord_bits)?;
                Some((
                    map(rx, coord_bits, decode[0], decode[1]),
                    map(ry, coord_bits, decode[2], decode[3]),
                ))
            };
            let read_color = |bits: &mut MeshBitReader| -> Option<Vec<f64>> {
                (0..components)
                    .map(|j| {
                        bits.read(comp_bits)
                            .map(|raw| map(raw, comp_bits, decode[4 + 2 * j], decode[5 + 2 * j]))
                    })
                    .collect()
            };
            if flag > 1 {
                self.warn(
                    "shading",
                    format!("シェーディング {name} の記録種別が不正のため省略"),
                );
                break;
            }
            if flag == 1 && prev_edge.is_none() {
                self.warn(
                    "mesh-continue",
                    format!("メッシュの継続記録に対応する直前パッチがありません ({name})"),
                );
                break;
            }
            // A truncated tail keeps already-decoded patches; only a fully
            // empty mesh reports the short stream.
            let net: Option<Net> = (|| {
                if flag == 0 {
                    let mut pts = Vec::with_capacity(full_points);
                    for _ in 0..full_points {
                        pts.push(read_point(&mut bits)?);
                    }
                    let mut cols = Vec::with_capacity(4);
                    for _ in 0..4 {
                        cols.push(read_color(&mut bits)?);
                    }
                    if shading_type == 6 {
                        let (bl, br, tr, tl) = (
                            cols[0].clone(),
                            cols[1].clone(),
                            cols[2].clone(),
                            cols[3].clone(),
                        );
                        prev_edge =
                            Some(([pts[3], pts[4], pts[5], pts[6]], [br.clone(), tr.clone()]));
                        Some(Net::Coons {
                            bottom: [pts[0], pts[1], pts[2], pts[3]],
                            right: [pts[3], pts[4], pts[5], pts[6]],
                            top: [pts[9], pts[8], pts[7], pts[6]],
                            left: [pts[0], pts[11], pts[10], pts[9]],
                            corners: [bl, br, tr, tl],
                        })
                    } else {
                        let mut grid = [[(0.0, 0.0); 4]; 4];
                        for (i, p) in pts.into_iter().enumerate() {
                            grid[i / 4][i % 4] = p;
                        }
                        let (bl, br, tr, tl) = (
                            cols[0].clone(),
                            cols[1].clone(),
                            cols[2].clone(),
                            cols[3].clone(),
                        );
                        prev_edge = Some((
                            [grid[0][3], grid[1][3], grid[2][3], grid[3][3]],
                            [br.clone(), tr.clone()],
                        ));
                        Some(Net::Tensor {
                            grid,
                            corners: [bl, br, tr, tl],
                        })
                    }
                } else {
                    let (edge, edge_cols) = prev_edge.clone()?;
                    let mut pts = Vec::with_capacity(cont_points);
                    for _ in 0..cont_points {
                        pts.push(read_point(&mut bits)?);
                    }
                    let (c_new0, c_new1) = (read_color(&mut bits)?, read_color(&mut bits)?);
                    if shading_type == 6 {
                        // Shared left edge = previous right edge; new points run
                        // bottom-controls, BR, right-controls, TR, top-controls.
                        let (bl, tl) = (edge_cols[0].clone(), edge_cols[1].clone());
                        let bottom = [edge[0], pts[0], pts[1], pts[2]];
                        let right = [pts[2], pts[3], pts[4], pts[5]];
                        let top = [edge[3], pts[7], pts[6], pts[5]];
                        prev_edge = Some((
                            [pts[2], pts[3], pts[4], pts[5]],
                            [c_new0.clone(), c_new1.clone()],
                        ));
                        Some(Net::Coons {
                            bottom,
                            right,
                            top,
                            left: edge,
                            corners: [bl, c_new0, c_new1, tl],
                        })
                    } else {
                        let mut grid = [[(0.0, 0.0); 4]; 4];
                        for r in 0..4 {
                            grid[r][0] = edge[r];
                        }
                        for (i, p) in pts.into_iter().enumerate() {
                            grid[i / 3][1 + i % 3] = p;
                        }
                        let (bl, tl) = (edge_cols[0].clone(), edge_cols[1].clone());
                        prev_edge = Some((
                            [grid[0][3], grid[1][3], grid[2][3], grid[3][3]],
                            [c_new0.clone(), c_new1.clone()],
                        ));
                        Some(Net::Tensor {
                            grid,
                            corners: [bl, c_new0, c_new1, tl],
                        })
                    }
                }
            })();
            let Some(net) = net else {
                if triangles.is_empty() {
                    self.warn(
                        "shading",
                        format!("シェーディング {name} の頂点が読めないため省略"),
                    );
                    return None;
                }
                break;
            };
            // Subdivide the (u,v) unit square; colors ride bilinearly.
            let (c_bl, c_br, c_tr, c_tl);
            let point_at = |net: &Net, u: f64, v: f64| -> (f64, f64) {
                match net {
                    Net::Coons {
                        bottom,
                        right,
                        top,
                        left,
                        ..
                    } => coons_point(bottom, right, top, left, u, v),
                    Net::Tensor { grid, .. } => {
                        let bu = bernstein(u);
                        let bv = bernstein(v);
                        let mut x = 0.0;
                        let mut y = 0.0;
                        for r in 0..4 {
                            for c in 0..4 {
                                let w = bv[r] * bu[c];
                                x += w * grid[r][c].0;
                                y += w * grid[r][c].1;
                            }
                        }
                        (x, y)
                    }
                }
            };
            let corners = match &net {
                Net::Coons { corners, .. } | Net::Tensor { corners, .. } => corners.clone(),
            };
            (c_bl, c_br, c_tr, c_tl) = (
                corners[0].clone(),
                corners[1].clone(),
                corners[2].clone(),
                corners[3].clone(),
            );
            for iv in 0..SUB {
                for iu in 0..SUB {
                    let (u0, u1) = (iu as f64 / SUB as f64, (iu + 1) as f64 / SUB as f64);
                    let (v0, v1) = (iv as f64 / SUB as f64, (iv + 1) as f64 / SUB as f64);
                    let node = |u: f64, v: f64| {
                        let (x, y) = point_at(&net, u, v);
                        (x, y, bilinear_color(&c_bl, &c_br, &c_tr, &c_tl, u, v))
                    };
                    let (a, b, c, d) = (node(u0, v0), node(u1, v0), node(u1, v1), node(u0, v1));
                    triangles.push([
                        (a.0, a.1, a.2.clone()),
                        (b.0, b.1, b.2.clone()),
                        (d.0, d.1, d.2.clone()),
                    ]);
                    triangles.push([(b.0, b.1, b.2), (c.0, c.1, c.2), (d.0, d.1, d.2)]);
                    if triangles.len() >= max_triangles {
                        break;
                    }
                }
                if triangles.len() >= max_triangles {
                    break;
                }
            }
        }
        if triangles.is_empty() {
            return None;
        }
        Some(triangles)
    }

    /// Active clip as an intersected bbox (masks are baked geometry).
    fn clip_bbox(&self, st: &GState) -> Option<(f64, f64, f64, f64)> {
        let mut out: Option<(f64, f64, f64, f64)> = None;
        for mask in &st.clip_paths {
            let (mn, mx) = mask.bounding_box()?;
            let b = (mn.x, mn.y, mx.x, mx.y);
            out = Some(match out {
                Some(r) => (r.0.max(b.0), r.1.max(b.1), r.2.min(b.2), r.3.min(b.3)),
                None => b,
            });
        }
        out
    }

    /// Evaluate a shading function at `t` (depth-guarded stitching).
    fn eval_shading_function(
        &self,
        func: &lopdf::Object,
        t: f64,
        components: usize,
        depth: usize,
    ) -> Option<Vec<f64>> {
        if depth > 8 {
            return None;
        }
        let func = self.resolve(func);
        let lopdf::Object::Dictionary(d) = func else {
            return None;
        };
        let ftype = d
            .get(b"FunctionType")
            .ok()
            .map(|o| num(self.resolve(o)) as i32)?;
        match ftype {
            2 => {
                let domain = match d.get(b"Domain").ok().map(|o| self.resolve(o)) {
                    Some(lopdf::Object::Array(a)) if a.len() == 2 => (num(&a[0]), num(&a[1])),
                    _ => (0.0, 1.0),
                };
                let span = domain.1 - domain.0;
                let x = if span.abs() < 1e-12 {
                    0.0
                } else {
                    ((t - domain.0) / span).clamp(0.0, 1.0)
                };
                let n = d
                    .get(b"N")
                    .ok()
                    .map(|o| num(self.resolve(o)))
                    .unwrap_or(1.0);
                let take = |key: &[u8], default: f64| -> Vec<f64> {
                    match d.get(key).ok().map(|o| self.resolve(o)) {
                        Some(lopdf::Object::Array(a)) => a.iter().map(num).collect(),
                        _ => vec![default; components],
                    }
                };
                let (c0, c1) = (take(b"C0", 0.0), take(b"C1", 1.0));
                if c0.len() < components || c1.len() < components {
                    return None;
                }
                Some(
                    (0..components)
                        .map(|j| c0[j] + (c1[j] - c0[j]) * x.powf(n))
                        .collect(),
                )
            }
            3 => {
                let funcs = match d.get(b"Functions").ok().map(|o| self.resolve(o)) {
                    Some(lopdf::Object::Array(a)) => a.clone(),
                    _ => return None,
                };
                if funcs.is_empty() {
                    return None;
                }
                let bounds: Vec<f64> = match d.get(b"Bounds").ok().map(|o| self.resolve(o)) {
                    Some(lopdf::Object::Array(a)) => a.iter().map(num).collect(),
                    _ => Vec::new(),
                };
                if bounds.len() + 1 != funcs.len() {
                    return None;
                }
                let domain = match d.get(b"Domain").ok().map(|o| self.resolve(o)) {
                    Some(lopdf::Object::Array(a)) if a.len() == 2 => (num(&a[0]), num(&a[1])),
                    _ => (0.0, 1.0),
                };
                let encode: Vec<f64> = match d.get(b"Encode").ok().map(|o| self.resolve(o)) {
                    Some(lopdf::Object::Array(a)) => a.iter().map(num).collect(),
                    _ => (0..funcs.len()).flat_map(|_| [0.0, 1.0]).collect(),
                };
                if encode.len() != 2 * funcs.len() {
                    return None;
                }
                let mut seg = funcs.len() - 1;
                for (i, b) in bounds.iter().enumerate() {
                    if t < *b {
                        seg = i;
                        break;
                    }
                }
                let (lo, hi) = if seg == 0 {
                    (domain.0, bounds.first().copied().unwrap_or(domain.1))
                } else if seg == funcs.len() - 1 {
                    (bounds.last().copied().unwrap_or(domain.0), domain.1)
                } else {
                    (bounds[seg - 1], bounds[seg])
                };
                let span = hi - lo;
                let mapped = if span.abs() < 1e-12 {
                    encode[2 * seg]
                } else {
                    encode[2 * seg] + (encode[2 * seg + 1] - encode[2 * seg]) * ((t - lo) / span)
                };
                self.eval_shading_function(&funcs[seg], mapped, components, depth + 1)
            }
            _ => None,
        }
    }

    /// Parse a Lab parameter dictionary (`/WhitePoint` required).
    fn lab_params_from_dict(&self, dict: &lopdf::Dictionary) -> Option<LabParams> {
        let white_arr = match dict.get(b"WhitePoint").ok().map(|o| self.resolve(o)) {
            Some(lopdf::Object::Array(a)) if a.len() == 3 => a,
            _ => return None,
        };
        let white = [num(&white_arr[0]), num(&white_arr[1]), num(&white_arr[2])];
        if white[1] <= 0.0 || !white[1].is_finite() {
            return None;
        }
        let range = match dict.get(b"Range").ok().map(|o| self.resolve(o)) {
            Some(lopdf::Object::Array(a)) if a.len() == 4 => {
                [num(&a[0]), num(&a[1]), num(&a[2]), num(&a[3])]
            }
            _ => [-100.0, 100.0, -100.0, 100.0],
        };
        Some(LabParams { white, range })
    }

    /// Image `/Decode` for Lab samples (default maps to L 0..100 and Range).
    fn image_lab_decode(&self, image_dict: &lopdf::Dictionary, params: &LabParams) -> [f64; 6] {
        let fallback = [
            0.0,
            100.0,
            params.range[0],
            params.range[1],
            params.range[2],
            params.range[3],
        ];
        match image_dict.get(b"Decode").ok().map(|o| self.resolve(o)) {
            Some(lopdf::Object::Array(a)) if a.len() == 6 => [
                num(&a[0]),
                num(&a[1]),
                num(&a[2]),
                num(&a[3]),
                num(&a[4]),
                num(&a[5]),
            ],
            _ => fallback,
        }
    }

    /// Separation tint samples → PNG bytes through the tint transform.
    /// Supports 8-bit tints; the transform reuses the shading function
    /// evaluator (type 2 / stitching of type 2).
    #[allow(clippy::too_many_arguments)]
    fn separation_samples_to_png(
        &self,
        raw: &[u8],
        w: u32,
        h: u32,
        bpc: u32,
        alternate: &SepAlternate,
        tint: &lopdf::Object,
        decode: &[f64; 2],
    ) -> Option<Vec<u8>> {
        use image::codecs::png::PngEncoder;
        use image::ImageEncoder;
        if bpc != 8 {
            return None;
        }
        let px = (w as usize).checked_mul(h as usize)?;
        if px == 0 || px > 16_777_216 || raw.len() < px {
            return None;
        }
        let components = match alternate {
            SepAlternate::Gray => 1,
            SepAlternate::Rgb => 3,
            SepAlternate::Cmyk => 4,
            SepAlternate::Lab(_) => 3,
        };
        let mut rgba = Vec::with_capacity(px * 4);
        for byte in &raw[..px] {
            let t = (decode[0] + *byte as f64 / 255.0 * (decode[1] - decode[0])).clamp(0.0, 1.0);
            let values = self.eval_shading_function(tint, t, components, 0)?;
            let color = match alternate {
                SepAlternate::Rgb => [values[0] as f32, values[1] as f32, values[2] as f32, 1.0],
                SepAlternate::Gray => [values[0] as f32, values[0] as f32, values[0] as f32, 1.0],
                SepAlternate::Cmyk => cmyk_to_rgb(values[0], values[1], values[2], values[3]),
                SepAlternate::Lab(params) => {
                    lab_to_rgb(values[0], values[1], values[2], params.white)
                }
            };
            rgba.extend_from_slice(&[
                (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                255,
            ]);
        }
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
            .ok()?;
        Some(png)
    }

    /// Apply an `/ExtGState` dictionary: fill/stroke alpha, blend mode and
    /// overprint flags. Entries with their own import path (`SMask`, `TR`)
    /// warn once; rendering-intent style entries have no visible effect.
    fn apply_extgstate(
        &mut self,
        resources: &Option<lopdf::Dictionary>,
        name: &str,
        st: &mut GState,
    ) {
        let res = resources.as_ref();
        let dict_opt: Option<lopdf::Dictionary> = res
            .and_then(|r| r.get(b"ExtGState").ok())
            .map(|o| self.resolve(o))
            .and_then(|o| match o {
                lopdf::Object::Dictionary(d) => d
                    .get(name.as_bytes())
                    .ok()
                    .map(|e| self.resolve(e))
                    .and_then(|e| match e {
                        lopdf::Object::Dictionary(d) => Some(d.clone()),
                        _ => None,
                    }),
                _ => None,
            });
        let Some(dict) = dict_opt else {
            self.warn(
                "extgstate",
                format!("グラフィック状態 {name} が見つかりません"),
            );
            return;
        };
        let number = |key: &[u8]| -> Option<f64> {
            dict.get(key).ok().and_then(|o| match self.resolve(o) {
                lopdf::Object::Integer(i) => Some(*i as f64),
                lopdf::Object::Real(f) => Some(*f as f64),
                lopdf::Object::Boolean(true) => Some(1.0),
                lopdf::Object::Boolean(false) => Some(0.0),
                _ => None,
            })
        };
        if let Some(v) = number(b"ca") {
            st.fill_alpha = v.clamp(0.0, 1.0) as f32;
        }
        if let Some(v) = number(b"CA") {
            st.stroke_alpha = v.clamp(0.0, 1.0) as f32;
        }
        if let Some(op) = dict.get(b"OP").ok().map(|o| self.resolve(o)) {
            st.overprint_fill = matches!(op, lopdf::Object::Boolean(true));
        }
        if let Some(op) = dict.get(b"op").ok().map(|o| self.resolve(o)) {
            st.overprint_stroke = matches!(op, lopdf::Object::Boolean(true));
        }
        if let Ok(bm) = dict.get(b"BM") {
            let bm = self.resolve(bm);
            let first = match bm {
                lopdf::Object::Name(_) => name_str(bm),
                lopdf::Object::Array(items) => {
                    items.first().and_then(|e| name_str(self.resolve(e)))
                }
                _ => None,
            };
            match first.as_deref().and_then(blend_mode_from_name) {
                Some(mode) => st.blend = mode,
                None => self.warn(
                    "blend",
                    "未対応のブレンドモードのため通常表示します".to_string(),
                ),
            }
        }
        if dict.get(b"SMask").is_ok() || dict.get(b"TR").is_ok() {
            self.warn(
                "extgstate",
                "一部のグラフィック状態（マスク・トーンカーブ）は未対応です".to_string(),
            );
        }
    }

    /// Whether the current font is Type 3 (cheap subtype check; full glyph
    /// resolution happens in `emit_type3_text`, which falls back loudly).
    fn is_type3_font(&self, resources: &Option<lopdf::Dictionary>, name: &str) -> bool {
        self.font_dict(resources, name)
            .and_then(|fdict| {
                fdict
                    .get(b"Subtype")
                    .ok()
                    .and_then(|o| name_str(self.resolve(o)))
            })
            .as_deref()
            == Some("Type3")
    }

    /// Resolve the current font as Type 3 with `/Differences` encoding.
    /// Named encodings fall back to `None` (legacy WinAnsi text path).
    fn type3_font(&self, resources: &Option<lopdf::Dictionary>, name: &str) -> Option<Type3Data> {
        let fdict = self.font_dict(resources, name)?;
        if fdict
            .get(b"Subtype")
            .ok()
            .and_then(|o| name_str(self.resolve(o)))
            .as_deref()
            != Some("Type3")
        {
            return None;
        }
        let enc = self.resolve(fdict.get(b"Encoding").ok()?);
        let diffs = match enc {
            lopdf::Object::Dictionary(e) => match self.resolve(e.get(b"Differences").ok()?) {
                lopdf::Object::Array(a) => a.clone(),
                _ => return None,
            },
            _ => return None,
        };
        let mut encoding = std::collections::HashMap::new();
        let mut code: i64 = -1;
        for item in &diffs {
            match self.resolve(item) {
                lopdf::Object::Integer(i) => code = *i,
                named if name_str(named).is_some() => {
                    if (0..=255).contains(&code) {
                        encoding.insert(code as u8, name_str(named).unwrap_or_default());
                    }
                    code += 1;
                }
                _ => {}
            }
        }
        let procs = match self.resolve(fdict.get(b"CharProcs").ok()?) {
            lopdf::Object::Dictionary(d) => d.clone(),
            _ => return None,
        };
        let first = match fdict.get(b"FirstChar").ok().map(|o| self.resolve(o)) {
            Some(lopdf::Object::Integer(i)) => (*i).max(0) as usize,
            _ => 0,
        };
        let widths: Vec<f64> = match fdict.get(b"Widths").ok().map(|o| self.resolve(o)) {
            Some(lopdf::Object::Array(a)) => a.iter().map(num).collect(),
            _ => Vec::new(),
        };
        let matrix = match fdict.get(b"Matrix").ok().map(|o| self.resolve(o)) {
            Some(lopdf::Object::Array(a)) if a.len() == 6 => [
                num(&a[0]),
                num(&a[1]),
                num(&a[2]),
                num(&a[3]),
                num(&a[4]),
                num(&a[5]),
            ],
            _ => [0.001, 0.0, 0.0, 0.001, 0.0, 0.0],
        };
        let resources = fdict
            .get(b"Resources")
            .ok()
            .and_then(|o| match self.resolve(o) {
                lopdf::Object::Dictionary(d) => Some(d.clone()),
                _ => None,
            });
        Some(Type3Data {
            matrix,
            first,
            widths,
            encoding,
            procs,
            resources,
        })
    }

    /// Paint Type 3 glyph programs as vector objects, one glyph per byte.
    /// Unresolvable glyphs warn once and advance by a default width instead
    /// of corrupting the run; programs run in an isolated graphics state so
    /// glyph-local clips never leak into the page.
    fn emit_type3_text(
        &mut self,
        bytes: &[u8],
        resources: &Option<lopdf::Dictionary>,
        st: &mut GState,
        suffix: &str,
    ) {
        if !st.in_text || self.glyph_depth > 8 {
            return;
        }
        let font_name = st.font_name.clone();
        let Some(t3) = self.type3_font(resources, &font_name) else {
            // Named encodings and friends: legacy WinAnsi fallback.
            self.warn(
                "type3-encoding",
                "Type3の字形が解決できないため代替表示します".to_string(),
            );
            let s = self.decode_text_bytes(bytes, st);
            self.emit_text(st, &s, suffix);
            return;
        };
        for &code in bytes {
            let glyph = t3.encoding.get(&code).cloned().unwrap_or_default();
            // Pen in device space, same formula as point text.
            let (ux, uy) = (st.tm[4] + st.x_ts * st.tm[0], st.tm[5] + st.x_ts * st.tm[1]);
            if !glyph.is_empty() {
                let content = t3
                    .procs
                    .get(glyph.as_bytes())
                    .ok()
                    .and_then(|o| self.resolve(o).as_stream().ok())
                    .cloned()
                    .and_then(|s| Self::stream_bytes(&s).ok())
                    .and_then(|b| Content::decode(&b).ok());
                match content {
                    Some(content) => {
                        // Glyph space → text space → pen → device, applied
                        // right-to-left (`mat_concat(a, b)` is A×B).
                        let pen_tm = [st.tm[0], st.tm[1], st.tm[2], st.tm[3], ux, uy];
                        let fs_mat = [st.font_size, 0.0, 0.0, st.font_size, 0.0, 0.0];
                        let glyph_ctm = mat_concat(
                            &st.ctm,
                            &mat_concat(&pen_tm, &mat_concat(&fs_mat, &t3.matrix)),
                        );
                        let sub_res = t3.resources.clone().or_else(|| resources.clone());
                        let (saved_ctm, saved_path, saved_pending, saved_clips) = (
                            st.ctm,
                            std::mem::take(&mut st.path),
                            st.pending_clip.take(),
                            st.clip_paths.len(),
                        );
                        st.ctm = glyph_ctm;
                        self.glyph_depth += 1;
                        let r = self.run_ops(&content.operations, &sub_res, st, suffix);
                        self.glyph_depth -= 1;
                        st.ctm = saved_ctm;
                        st.path = saved_path;
                        st.pending_clip = saved_pending;
                        st.clip_paths.truncate(saved_clips);
                        if r.is_err() {
                            self.warn("type3-glyph", format!("Type3グリフ {glyph} をスキップ"));
                        }
                    }
                    None => {
                        self.warn("type3-glyph", format!("Type3グリフ {glyph} をスキップ"));
                    }
                }
            }
            // Advance: glyph-space width through the 1000-unit convention.
            let mut em = t3.width(code) * 0.001 * st.font_size + st.char_space;
            if code == 32 {
                em += st.word_space;
            }
            st.x_ts += em;
        }
    }

    fn font_dict(
        &self,
        resources: &Option<lopdf::Dictionary>,
        name: &str,
    ) -> Option<lopdf::Dictionary> {
        let res = resources.as_ref()?;
        let fonts = self.resolve(res.get(b"Font").ok()?);
        let lopdf::Object::Dictionary(fd) = fonts else {
            return None;
        };
        let fobj = self.resolve(fd.get(name.as_bytes()).ok()?);
        match fobj {
            lopdf::Object::Dictionary(fdict) => Some(fdict.clone()),
            _ => None,
        }
    }

    fn strip_subset_prefix(family: &str) -> String {
        // Strip subset prefixes ("ABCDEF+Helvetica"): exactly six
        // uppercase ASCII letters, otherwise a real family name like
        // "MyFont+Regular" would be mangled into "Regular".
        if let Some(plus) = family.find('+') {
            if plus == 6 && family[..6].chars().all(|c| c.is_ascii_uppercase()) {
                return family[plus + 1..].to_string();
            }
        }
        family.to_string()
    }

    /// Family name plus `/ToUnicode` map and composite flag for `Tf`.
    fn font_details(
        &mut self,
        resources: &Option<lopdf::Dictionary>,
        name: &str,
    ) -> (
        String,
        Option<std::rc::Rc<std::collections::HashMap<u32, String>>>,
        bool,
    ) {
        let Some(fdict) = self.font_dict(resources, name) else {
            return (Self::strip_subset_prefix(name), None, false);
        };
        let mut family = name.to_string();
        if let Ok(base) = fdict.get(b"BaseFont") {
            if let Some(bn) = name_str(self.resolve(base)) {
                family = bn;
            }
        }
        let subtype = fdict
            .get(b"Subtype")
            .ok()
            .and_then(|o| name_str(self.resolve(o)))
            .unwrap_or_default();
        let is_composite = subtype == "Type0" || fdict.get(b"DescendantFonts").is_ok();
        let map = self.tounicode_for_dict(&fdict);
        (Self::strip_subset_prefix(&family), map, is_composite)
    }

    fn tounicode_for_dict(
        &mut self,
        fdict: &lopdf::Dictionary,
    ) -> Option<std::rc::Rc<std::collections::HashMap<u32, String>>> {
        let tu = fdict.get(b"ToUnicode").ok()?;
        let resolved = self.resolve(tu);
        let stream = resolved.as_stream().ok()?.clone();
        let bytes = match Self::stream_bytes(&stream) {
            Ok(b) => b,
            Err(_) => {
                self.warn(
                    "tounicode-decode",
                    "ToUnicodeの展開に失敗したため代替表示します".to_string(),
                );
                return None;
            }
        };
        if bytes.len() > Self::MAX_STREAM_BYTES {
            self.warn(
                "tounicode-decode",
                "ToUnicodeが大きすぎるため代替表示します".to_string(),
            );
            return None;
        }
        let (map, truncated) = parse_tounicode_cmap(&bytes);
        if truncated {
            self.warn(
                "tounicode-large",
                "ToUnicodeが大きいため一部のみ復元しました".to_string(),
            );
        }
        if map.is_empty() {
            return None;
        }
        Some(std::rc::Rc::new(map))
    }

    /// Text bytes → Unicode using the current font's `/ToUnicode` when
    /// present. Unmapped CIDs become U+FFFD with a one-time warning so
    /// missing glyphs stay explicit instead of silently wrong.
    fn decode_text_bytes(&mut self, bytes: &[u8], st: &GState) -> String {
        if let Some(map) = st.font_tounicode.clone() {
            if st.font_is_composite {
                let mut out = String::new();
                let mut i = 0;
                while i < bytes.len() {
                    let code = if i + 1 < bytes.len() {
                        let c = ((bytes[i] as u32) << 8) | (bytes[i + 1] as u32);
                        i += 2;
                        c
                    } else {
                        let c = bytes[i] as u32;
                        i += 1;
                        c
                    };
                    match map.get(&code) {
                        Some(s) => out.push_str(s),
                        None => {
                            self.warn(
                                "cid-missing",
                                "ToUnicodeにないCIDがあるため代替文字で表示します".to_string(),
                            );
                            out.push('\u{FFFD}');
                        }
                    }
                }
                return out;
            }
            let mut out = String::with_capacity(bytes.len());
            for b in bytes {
                match map.get(&(*b as u32)) {
                    Some(s) => out.push_str(s),
                    None => out.push(winansi_byte_to_char(*b)),
                }
            }
            return out;
        }
        if st.font_is_composite {
            self.warn(
                "cid-text",
                "ToUnicodeのないCIDフォントは代替文字で表示します".to_string(),
            );
            // One placeholder per 2-byte CID (plus one for a trailing byte).
            let mut out = String::new();
            let mut i = 0;
            while i < bytes.len() {
                out.push('\u{FFFD}');
                i += if i + 1 < bytes.len() { 2 } else { 1 };
            }
            return out;
        }
        decode_pdf_string(bytes)
    }

    fn emit_xobject(
        &mut self,
        resources: &Option<lopdf::Dictionary>,
        name: &str,
        st: &mut GState,
        suffix: &str,
    ) -> Result<(), String> {
        let res = resources
            .as_ref()
            .ok_or("XObject参照もリソースもありません".to_string())?;
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
            if self.form_depth >= MAX_FORM_DEPTH {
                self.warn(
                    "form_depth",
                    format!("Form再帰が深すぎます（{MAX_FORM_DEPTH}層）ためスキップ"),
                );
                return Ok(());
            }
            // Recurse with concatenated matrix.
            let mat = stream
                .dict
                .get(b"Matrix")
                .ok()
                .and_then(|o| match self.resolve(o) {
                    lopdf::Object::Array(a) if a.len() == 6 => Some([
                        num(&a[0]),
                        num(&a[1]),
                        num(&a[2]),
                        num(&a[3]),
                        num(&a[4]),
                        num(&a[5]),
                    ]),
                    _ => None,
                })
                .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
            let saved = st.ctm;
            let saved_clips = st.clip_paths.clone();
            let saved_pending_clip = st.pending_clip;
            let saved_family = st.font_family.clone();
            let saved_font_name = st.font_name.clone();
            let saved_tounicode = st.font_tounicode.clone();
            let saved_composite = st.font_is_composite;
            let saved_fill_alpha = st.fill_alpha;
            let saved_stroke_alpha = st.stroke_alpha;
            let saved_blend = st.blend;
            let saved_overprint_fill = st.overprint_fill;
            let saved_overprint_stroke = st.overprint_stroke;
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
            let bytes = Self::stream_bytes(&stream).map_err(|e| format!("Form{e}"))?;
            let content = Content::decode(&bytes).map_err(|e| format!("Form解析失敗: {e}"))?;
            self.form_depth += 1;
            let r = self.run_ops(&content.operations, &sub_res, st, suffix);
            self.form_depth -= 1;
            st.ctm = saved;
            st.clip_paths = saved_clips;
            st.pending_clip = saved_pending_clip;
            st.font_family = saved_family;
            st.font_name = saved_font_name;
            st.font_tounicode = saved_tounicode;
            st.font_is_composite = saved_composite;
            st.fill_alpha = saved_fill_alpha;
            st.stroke_alpha = saved_stroke_alpha;
            st.blend = saved_blend;
            st.overprint_fill = saved_overprint_fill;
            st.overprint_stroke = saved_overprint_stroke;
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
        // saturating_mul: untrusted Width×Height must not wrap before the cap.
        let px = w.saturating_mul(h);
        if w == 0 || h == 0 || px > 16_777_216 {
            self.warn(
                "image-size",
                format!("異常な画像サイズ ({w}x{h}) をスキップ"),
            );
            return Ok(());
        }
        if dict.get(b"SMask").is_ok() {
            // Handled below after the base payload decodes; only warn when
            // the mask cannot be applied (fidelity stays explicit).
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
        if has("JBIG2Decode") {
            self.warn("jbig2", format!("JBIG2画像 {name} をスキップ"));
            return Ok(());
        }
        if has("CCITTFaxDecode") {
            let Some(png) = self.decode_ccitt_image(stream, &filters, w, h, name) else {
                // Helper warned specifically; skip this image only.
                return Ok(());
            };
            let placed = match crate::io::raster::decode_placed_image(&png) {
                Ok((_, _, placed)) => placed,
                Err(e) => {
                    self.warn(
                        "image-place",
                        format!("画像配置失敗のためスキップ ({name}): {e}"),
                    );
                    return Ok(());
                }
            };
            self.emit_placed_image(placed, st, suffix);
            return Ok(());
        }
        // Colorspace: direct RGB/Gray/CMYK, indexed-RGB/Gray lookup, Lab,
        // Separation tint.
        enum PixelLayout {
            Direct(u32),
            Indexed {
                base_channels: u32,
                lookup: Vec<u8>,
                hival: usize,
            },
            Lab {
                params: LabParams,
                decode: [f64; 6],
            },
            Separation {
                alternate: SepAlternate,
                tint: lopdf::Object,
                decode: [f64; 2],
            },
        }
        let layout: Option<PixelLayout> = match dict
            .get(b"ColorSpace")
            .ok()
            .map(|o| self.resolve(o))
        {
            Some(lopdf::Object::Name(n)) => {
                let s = String::from_utf8_lossy(n).into_owned();
                if s.contains("DeviceRGB") || s.contains("CalRGB") {
                    Some(PixelLayout::Direct(3))
                } else if s.contains("DeviceGray") || s.contains("CalGray") {
                    Some(PixelLayout::Direct(1))
                } else if s.contains("DeviceCMYK") {
                    Some(PixelLayout::Direct(4))
                } else {
                    None
                }
            }
            Some(lopdf::Object::Array(a)) => {
                let head = a
                    .first()
                    .and_then(|e| name_str(self.resolve(e)))
                    .unwrap_or_default();
                if head.contains("DeviceRGB") || head.contains("CalRGB") {
                    Some(PixelLayout::Direct(3))
                } else if head.contains("DeviceGray") || head.contains("CalGray") {
                    Some(PixelLayout::Direct(1))
                } else if head.contains("DeviceCMYK") {
                    Some(PixelLayout::Direct(4))
                } else if head == "ICCBased" && a.len() >= 2 {
                    // ICC profile: samples stay component-wise; /Alternate
                    // (or /N when missing) tells how to read them. Values
                    // are used as-is (sRGB-adjacent profiles dominate), with
                    // a one-time approximation notice at use.
                    let profile = self.resolve(&a[1]);
                    let (dict_n, dict_alt): (Option<f64>, Option<String>) = match profile {
                        lopdf::Object::Stream(s) => (
                            s.dict.get(b"N").ok().and_then(|o| match self.resolve(o) {
                                lopdf::Object::Integer(i) => Some(*i as f64),
                                lopdf::Object::Real(f) => Some(*f as f64),
                                _ => None,
                            }),
                            s.dict
                                .get(b"Alternate")
                                .ok()
                                .and_then(|o| match self.resolve(o) {
                                    lopdf::Object::Name(n) => {
                                        Some(String::from_utf8_lossy(n).into_owned())
                                    }
                                    lopdf::Object::Array(inner) => {
                                        inner.first().and_then(|e| name_str(self.resolve(e)))
                                    }
                                    _ => None,
                                }),
                        ),
                        _ => (None, None),
                    };
                    let n = dict_n.unwrap_or(0.0) as u32;
                    let alt = dict_alt.unwrap_or_default();
                    let channels = if alt.contains("DeviceRGB") || alt.contains("CalRGB") {
                        3
                    } else if alt.contains("DeviceGray") || alt.contains("CalGray") {
                        1
                    } else if alt.contains("DeviceCMYK") {
                        4
                    } else if n == 1 || n == 3 || n == 4 {
                        n
                    } else {
                        0
                    };
                    match channels {
                        1 | 3 | 4 => {
                            self.warn("icc-approx", "ICCプロファイルを近似表示します".to_string());
                            Some(PixelLayout::Direct(channels))
                        }
                        _ => None,
                    }
                } else if head == "Lab" {
                    match self.resolve(&a[1]) {
                        lopdf::Object::Dictionary(d) => match self.lab_params_from_dict(d) {
                            Some(params) => {
                                self.warn("lab-approx", "Lab画像を近似表示します".to_string());
                                Some(PixelLayout::Lab {
                                    params,
                                    decode: self.image_lab_decode(dict, &params),
                                })
                            }
                            None => None,
                        },
                        _ => None,
                    }
                } else if head == "Separation" && a.len() >= 4 {
                    // [/Separation name alternate tintTransform]: single tint
                    // component mapped through the transform into alternate.
                    let alternate: Option<(SepAlternate, usize)> = match self.resolve(&a[2]) {
                        lopdf::Object::Name(n) => {
                            let s = String::from_utf8_lossy(n).into_owned();
                            if s.contains("DeviceRGB") || s.contains("CalRGB") {
                                Some((SepAlternate::Rgb, 3))
                            } else if s.contains("DeviceGray") || s.contains("CalGray") {
                                Some((SepAlternate::Gray, 1))
                            } else if s.contains("DeviceCMYK") {
                                Some((SepAlternate::Cmyk, 4))
                            } else {
                                None
                            }
                        }
                        lopdf::Object::Array(inner) => {
                            let inner_head = inner
                                .first()
                                .and_then(|e| name_str(self.resolve(e)))
                                .unwrap_or_default();
                            if inner_head == "Lab" {
                                match self.resolve(inner.get(1).unwrap_or(&lopdf::Object::Null)) {
                                    lopdf::Object::Dictionary(d) => self
                                        .lab_params_from_dict(d)
                                        .map(|params| (SepAlternate::Lab(params), 3)),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    let decode = match dict.get(b"Decode").ok().map(|o| self.resolve(o)) {
                        Some(lopdf::Object::Array(d)) if d.len() == 2 => [num(&d[0]), num(&d[1])],
                        _ => [0.0, 1.0],
                    };
                    alternate.map(|(alternate, _)| PixelLayout::Separation {
                        alternate,
                        tint: a[3].clone(),
                        decode,
                    })
                } else if head == "Indexed" && a.len() >= 4 {
                    let base = match self.resolve(&a[1]) {
                        lopdf::Object::Name(n) => String::from_utf8_lossy(n).into_owned(),
                        lopdf::Object::Array(inner) => inner
                            .first()
                            .and_then(|e| name_str(self.resolve(e)))
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    let base_channels = if base.contains("DeviceRGB") || base.contains("CalRGB") {
                        3
                    } else if base.contains("DeviceGray") || base.contains("CalGray") {
                        1
                    } else {
                        String::new().len() as u32 // zero marks unsupported base below
                    };
                    let hival = match self.resolve(&a[2]) {
                        lopdf::Object::Integer(i) => (*i).max(0) as usize,
                        _ => usize::MAX,
                    };
                    let lookup: Option<Vec<u8>> = match self.resolve(&a[3]) {
                        lopdf::Object::String(b, _) => Some(b.clone()),
                        lopdf::Object::Stream(s) => match s.decompressed_content() {
                            Ok(decoded) => Some(decoded),
                            Err(_) => Some(s.content.clone()),
                        },
                        _ => None,
                    };
                    match (base_channels, lookup) {
                        (1 | 3, Some(table))
                            if hival <= 255
                                && table.len() >= (hival + 1) * base_channels as usize =>
                        {
                            Some(PixelLayout::Indexed {
                                base_channels,
                                lookup: table,
                                hival,
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        };
        let Some(layout) = layout else {
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
            // Separation / Lab: light version skips.
            self.warn(
                "colorspace",
                format!("未対応色空間 ({cs}) の画像 {name} をスキップ"),
            );
            return Ok(());
        };
        let channels: u32 = match &layout {
            PixelLayout::Direct(c) => *c,
            PixelLayout::Indexed { .. } => 1,
            PixelLayout::Lab { .. } => 3,
            PixelLayout::Separation { .. } => 1,
        };
        let png_bytes: Vec<u8> = if has("DCTDecode") {
            if filters.len() == 1 {
                if matches!(layout, PixelLayout::Indexed { .. }) {
                    self.warn(
                        "indexed-jpeg",
                        format!("パレットJPEG画像 {name} をスキップ"),
                    );
                    return Ok(());
                }
                if matches!(layout, PixelLayout::Lab { .. }) {
                    self.warn("lab-jpeg", format!("Lab JPEG画像 {name} をスキップ"));
                    return Ok(());
                }
                if matches!(layout, PixelLayout::Separation { .. }) {
                    self.warn("separation-jpeg", format!("特色JPEG画像 {name} をスキップ"));
                    return Ok(());
                }
                // Bare JPEG (lopdf leaves DCT alone). CMYK JPEG decodes via
                // the `image` crate when supported; otherwise warn-skip below
                // at placement instead of aborting the whole import.
                if channels == 4 && image::load_from_memory(&stream.content).is_err() {
                    self.warn("cmyk-jpeg", format!("CMYK JPEG画像 {name} をスキップ"));
                    return Ok(());
                }
                stream.content.clone()
            } else {
                self.warn(
                    "jpeg-chain",
                    format!("複合フィルタのJPEG {name} をスキップ"),
                );
                return Ok(());
            }
        } else {
            // No filter / unknown filter: raw samples, but a broken stream
            // must not abort the whole import — warn and skip this image.
            // (`stream_bytes` returns raw content when no /Filter is set;
            // `decompressed_content` alone would error on those.)
            match Self::stream_bytes(stream) {
                Ok(raw) => {
                    let converted = match &layout {
                        PixelLayout::Direct(4) => cmyk_samples_to_png(&raw, w, h, bpc),
                        PixelLayout::Direct(c) => samples_to_png(&raw, w, h, *c, bpc),
                        PixelLayout::Indexed {
                            base_channels,
                            lookup,
                            hival,
                        } => {
                            indexed_samples_to_png(&raw, w, h, bpc, *base_channels, lookup, *hival)
                        }
                        PixelLayout::Lab { params, decode } => {
                            lab_samples_to_png(&raw, w, h, bpc, params, decode)
                        }
                        PixelLayout::Separation {
                            alternate,
                            tint,
                            decode,
                        } => {
                            self.separation_samples_to_png(&raw, w, h, bpc, alternate, tint, decode)
                        }
                    };
                    match converted {
                        Some(png) => png,
                        None => {
                            self.warn("samples", format!("画像サンプル解釈失敗 ({name})"));
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    self.warn(
                        "image-stream",
                        format!("画像展開失敗のためスキップ ({name}): {e}"),
                    );
                    return Ok(());
                }
            }
        };
        let placed = match self.apply_smask(stream, &png_bytes, w, h, name) {
            Some(applied_png) => match crate::io::raster::decode_placed_image(&applied_png) {
                Ok((_, _, placed)) => placed,
                Err(e) => {
                    self.warn(
                        "image-place",
                        format!("画像配置失敗のためスキップ ({name}): {e}"),
                    );
                    return Ok(());
                }
            },
            None => match crate::io::raster::decode_placed_image(&png_bytes) {
                Ok((_, _, placed)) => placed,
                Err(e) => {
                    self.warn(
                        "image-place",
                        format!("画像配置失敗のためスキップ ({name}): {e}"),
                    );
                    return Ok(());
                }
            },
        };
        // Unit-square corners through the CTM → device rect.
        self.emit_placed_image(placed, st, suffix);
        Ok(())
    }

    /// Place already-decoded PNG bytes as a clipped image object.
    fn emit_placed_image(&mut self, placed: Vec<u8>, st: &GState, suffix: &str) {
        let (x1, y1) = mat_apply(&st.ctm, 0.0, 0.0);
        let (x2, y2) = mat_apply(&st.ctm, 1.0, 1.0);
        let (lx, rx) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
        let (ty, by) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
        self.seq += 1;
        let mut obj = Object::new_image(
            &format!("PDF Image {:03}{suffix}", self.seq),
            lx,
            ty,
            (rx - lx).max(1.0),
            (by - ty).max(1.0),
            placed,
        );
        obj.opacity *= st.fill_alpha;
        obj.blend_mode = st.blend;
        self.emit_clipped_object(obj, st);
    }

    /// Decode a `/CCITTFaxDecode` image to PNG bytes (8-bit gray).
    /// Group 4 (K<0) and Group 3 1D (K=0) decode via the `fax` crate; mixed
    /// 2D (K>0) and byte-aligned lines warn-skip so gaps stay explicit.
    /// Short streams pad white, overlong ones truncate to `/Rows`.
    fn decode_ccitt_image(
        &mut self,
        stream: &lopdf::Stream,
        filters: &[String],
        w: u32,
        h: u32,
        name: &str,
    ) -> Option<Vec<u8>> {
        // Single-filter only; chained filters stay explicit.
        if filters.len() != 1 {
            self.warn(
                "fax-chain",
                format!("複合フィルタのFAX画像 {name} をスキップ"),
            );
            return None;
        }
        let parms_dict: Option<lopdf::Dictionary> = match stream
            .dict
            .get(b"DecodeParms")
            .ok()
            .map(|o| self.resolve(o))
        {
            Some(lopdf::Object::Dictionary(d)) => Some(d.clone()),
            Some(lopdf::Object::Array(a)) if a.len() == 1 => match self.resolve(&a[0]) {
                lopdf::Object::Dictionary(d) => Some(d.clone()),
                _ => None,
            },
            _ => None,
        };
        let num_param = |key: &[u8]| -> Option<f64> {
            parms_dict
                .as_ref()?
                .get(key)
                .ok()
                .and_then(|o| match self.resolve(o) {
                    lopdf::Object::Integer(i) => Some(*i as f64),
                    lopdf::Object::Real(f) => Some(*f as f64),
                    _ => None,
                })
        };
        let bool_param = |key: &[u8]| -> bool {
            parms_dict
                .as_ref()
                .and_then(|d| d.get(key).ok())
                .is_some_and(|o| matches!(self.resolve(o), lopdf::Object::Boolean(true)))
        };
        let k = num_param(b"K").unwrap_or(0.0) as i32;
        let columns = num_param(b"Columns").unwrap_or(w as f64).max(0.0) as u32;
        let rows = num_param(b"Rows").unwrap_or(h as f64).max(0.0) as u32;
        let black_is_1 = bool_param(b"BlackIs1");
        if bool_param(b"EncodedByteAlign") {
            self.warn("fax", format!("バイト整列FAX画像 {name} をスキップ"));
            return None;
        }
        if k > 0 {
            self.warn("fax", format!("2次元混合FAX画像 {name} をスキップ"));
            return None;
        }
        if columns == 0
            || rows == 0
            || columns > 65535
            || rows > 65535
            || columns.saturating_mul(rows) > 16_777_216
            || w == 0
            || h == 0
            || w.saturating_mul(h) > 16_777_216
        {
            self.warn("fax", format!("FAX画像寸法が不正なためスキップ ({name})"));
            return None;
        }
        // Wire-size guard: the generic image path caps streams at
        // MAX_STREAM_BYTES, but the FAX path used the raw content.
        // Cap the decoder input so a malicious strip cannot burn CPU.
        const MAX_FAX_BYTES: usize = 32 * 1024 * 1024;
        if stream.content.len() > MAX_FAX_BYTES {
            self.warn("fax", format!("FAX画像が大きすぎるためスキップ ({name})"));
            return None;
        }
        let data = stream.content.clone();
        let width = columns as u16;
        let mut lines: Vec<Vec<u8>> = Vec::new();
        let ok = if k < 0 {
            fax::decoder::decode_g4(
                data.iter().copied(),
                width,
                Some(rows as u16),
                |transitions| {
                    lines.push(
                        fax::decoder::pels(transitions, width)
                            .map(|c| match c {
                                fax::Color::White => 255u8,
                                fax::Color::Black => 0u8,
                            })
                            .collect(),
                    );
                },
            )
            .is_some()
        } else {
            let mut count = 0u32;
            fax::decoder::decode_g3(data.iter().copied(), |transitions| {
                if count < rows {
                    lines.push(
                        fax::decoder::pels(transitions, width)
                            .map(|c| match c {
                                fax::Color::White => 255u8,
                                fax::Color::Black => 0u8,
                            })
                            .collect(),
                    );
                    count += 1;
                }
            })
            .is_some()
        };
        if !ok || lines.is_empty() {
            self.warn("fax", format!("FAX展開に失敗したためスキップ ({name})"));
            return None;
        }
        while lines.len() < rows as usize {
            lines.push(vec![255u8; columns as usize]);
        }
        lines.truncate(rows as usize);
        let mut gray: Vec<u8> = lines.concat();
        if black_is_1 {
            for v in &mut gray {
                *v = 255 - *v;
            }
        }
        if columns != w || rows != h {
            gray = resize_alpha_nearest(&gray, columns, rows, w, h);
        }
        samples_to_png(&gray, w, h, 1, 8).or_else(|| {
            self.warn("fax", format!("FAX画像変換に失敗したためスキップ ({name})"));
            None
        })
    }

    /// Apply a `/SMask` soft mask to a decoded base payload. Returns the
    /// alpha-composited PNG on success, or `None` (with a one-time warning)
    /// when the mask is missing/unsupported — the caller then keeps the
    /// opaque base instead of failing the whole import.
    fn apply_smask(
        &mut self,
        stream: &lopdf::Stream,
        base_png: &[u8],
        w: u32,
        h: u32,
        name: &str,
    ) -> Option<Vec<u8>> {
        let smask_obj = stream.dict.get(b"SMask").ok()?;
        let resolved = self.resolve(smask_obj);
        let mask_stream = resolved.as_stream().ok()?.clone();
        let mask_dict = &mask_stream.dict;
        let get_num = |key: &[u8]| -> Option<f64> {
            mask_dict.get(key).ok().and_then(|o| match self.resolve(o) {
                lopdf::Object::Integer(i) => Some(*i as f64),
                lopdf::Object::Real(f) => Some(*f as f64),
                _ => None,
            })
        };
        let (mw, mh) = (
            get_num(b"Width").unwrap_or(0.0) as u32,
            get_num(b"Height").unwrap_or(0.0) as u32,
        );
        let bpc = get_num(b"BitsPerComponent").unwrap_or(8.0) as u32;
        if mw == 0 || mh == 0 || mw.saturating_mul(mh) > 16_777_216 {
            self.warn(
                "smask",
                format!("ソフトマスク寸法が不正なため不透明表示 ({name})"),
            );
            return None;
        }
        let mask_filters: Vec<String> = mask_dict
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
        let is_dct = mask_filters
            .iter()
            .any(|x| x == "DCTDecode" || x == "/DCTDecode");
        let alpha_full: Vec<u8> = if is_dct && mask_filters.len() == 1 {
            let img = image::load_from_memory(&mask_stream.content)
                .ok()?
                .to_luma8();
            if img.width() != mw || img.height() != mh {
                return None;
            }
            img.into_raw()
        } else {
            let raw = Self::stream_bytes(&mask_stream).ok()?;
            smask_raw_to_alpha(&raw, mw, mh, bpc)?
        };
        let base = image::load_from_memory(base_png).ok()?.to_rgba8();
        let (bw, bh) = (base.width(), base.height());
        if bw == 0 || bh == 0 {
            return None;
        }
        // Mask and base rarely differ, but viewers scale the mask to the
        // image; nearest-neighbor keeps hard transparency edges exact.
        let alpha = if mw == bw && mh == bh {
            alpha_full
        } else {
            resize_alpha_nearest(&alpha_full, mw, mh, bw, bh)
        };
        if (alpha.len() as u32) != bw.saturating_mul(bh) {
            self.warn(
                "smask",
                format!("ソフトマスク適用に失敗し不透明表示 ({name})"),
            );
            return None;
        }
        // Base images here are opaque, but multiply anyway so a future
        // alpha-bearing base composes instead of being replaced. `w`/`h`
        // are kept for signature stability with the dict dimensions.
        let _ = (w, h);
        apply_alpha_to_png(base_png, &alpha).or_else(|| {
            self.warn(
                "smask",
                format!("ソフトマスク適用に失敗し不透明表示 ({name})"),
            );
            None
        })
    }
}

fn blend_mode_from_name(name: &str) -> Option<BlendMode> {
    match name.strip_prefix('/').unwrap_or(name) {
        "Normal" | "Compatible" => Some(BlendMode::Normal),
        "Multiply" => Some(BlendMode::Multiply),
        "Screen" => Some(BlendMode::Screen),
        "Overlay" => Some(BlendMode::Overlay),
        "Darken" => Some(BlendMode::Darken),
        "Lighten" => Some(BlendMode::Lighten),
        "ColorDodge" => Some(BlendMode::ColorDodge),
        "ColorBurn" => Some(BlendMode::ColorBurn),
        "HardLight" => Some(BlendMode::HardLight),
        "SoftLight" => Some(BlendMode::SoftLight),
        "Difference" => Some(BlendMode::Difference),
        "Exclusion" => Some(BlendMode::Exclusion),
        "Hue" => Some(BlendMode::Hue),
        "Saturation" => Some(BlendMode::Saturation),
        "Color" => Some(BlendMode::Color),
        "Luminosity" => Some(BlendMode::Luminosity),
        _ => None,
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

/// PDF `Lab` parameters: D50 white point and a/b range. `BlackPoint` only
/// scales the destination flare and is ignored by viewers alike.
#[derive(Debug, Clone, Copy)]
struct LabParams {
    white: [f64; 3],
    range: [f64; 4],
}

/// Alternate space behind a `/Separation` tint transform.
/// Output model for shading function values and mesh vertex colors.
enum ShadingSpace {
    Rgb,
    Gray,
    Cmyk,
    Lab(LabParams),
}

/// Function/vertex color values → opaque sRGB for the canvas model.
fn shading_values_to_rgb(values: &[f64], space: &ShadingSpace) -> [f32; 4] {
    let color = match space {
        ShadingSpace::Rgb => [values[0] as f32, values[1] as f32, values[2] as f32, 1.0],
        ShadingSpace::Gray => [values[0] as f32, values[0] as f32, values[0] as f32, 1.0],
        ShadingSpace::Cmyk => cmyk_to_rgb(values[0], values[1], values[2], values[3]),
        ShadingSpace::Lab(params) => lab_to_rgb(values[0], values[1], values[2], params.white),
    };
    [
        color[0].clamp(0.0, 1.0),
        color[1].clamp(0.0, 1.0),
        color[2].clamp(0.0, 1.0),
        1.0,
    ]
}

#[derive(Debug, Clone)]
enum SepAlternate {
    Gray,
    Rgb,
    Cmyk,
    Lab(LabParams),
}

/// A resolvable Type 3 font: glyph names per code, widths and the CharProcs
/// programs themselves (plus the font-level resources).
struct Type3Data {
    matrix: Affine,
    first: usize,
    widths: Vec<f64>,
    encoding: std::collections::HashMap<u8, String>,
    procs: lopdf::Dictionary,
    resources: Option<lopdf::Dictionary>,
}

impl Type3Data {
    /// Glyph advance in glyph-space units (half an em without `/Widths`).
    fn width(&self, code: u8) -> f64 {
        let i = code as usize;
        if i >= self.first && i - self.first < self.widths.len() {
            self.widths[i - self.first]
        } else {
            500.0
        }
    }
}

/// CIE Lab (D50) → sRGB via Bradford-adapted D65. Out-of-gamut channels
/// clamp; the caller decides whether that deserves a notice.
fn lab_to_rgb(l: f64, a: f64, b: f64, white: [f64; 3]) -> [f32; 4] {
    const EPS: f64 = 216.0 / 24389.0;
    const KAPPA: f64 = 24389.0 / 27.0;
    let fy = (l.clamp(0.0, 100.0) + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let finv = |t: f64| {
        let t3 = t * t * t;
        if t3 > EPS {
            t3
        } else {
            (116.0 * t - 16.0) / KAPPA
        }
    };
    let (x, y, z) = (
        white[0] * finv(fx),
        white[1] * finv(fy),
        white[2] * finv(fz),
    );
    // Bradford D50 → D65.
    const M: [[f64; 3]; 3] = [
        [0.9555766, -0.0230393, 0.0631636],
        [-0.0282895, 1.0099416, 0.0210077],
        [0.0122982, -0.0204830, 1.3299098],
    ];
    let (x, y, z) = (
        M[0][0] * x + M[0][1] * y + M[0][2] * z,
        M[1][0] * x + M[1][1] * y + M[1][2] * z,
        M[2][0] * x + M[2][1] * y + M[2][2] * z,
    );
    // Linear sRGB (D65) + gamma.
    let linear = [
        3.2404542 * x - 1.5371385 * y - 0.4985314 * z,
        -0.9692660 * x + 1.8760108 * y + 0.0415560 * z,
        0.0556434 * x - 0.2040259 * y + 1.0572252 * z,
    ];
    let gamma = |u: f64| {
        if u <= 0.0031308 {
            12.92 * u
        } else {
            1.055 * u.powf(1.0 / 2.4) - 0.055
        }
    };
    [
        gamma(linear[0]).clamp(0.0, 1.0) as f32,
        gamma(linear[1]).clamp(0.0, 1.0) as f32,
        gamma(linear[2]).clamp(0.0, 1.0) as f32,
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

fn path_from_segments(segments: &[Seg], ctm: &Affine) -> PathData {
    let mut path = PathData::new();
    for seg in segments {
        match seg {
            Seg::Move(x, y) => {
                let (x, y) = mat_apply(ctm, *x, *y);
                path.push_move_to(x, y);
            }
            Seg::Line(x, y) => {
                let (x, y) = mat_apply(ctm, *x, *y);
                path.push_line_to(x, y);
            }
            Seg::Curve(x1, y1, x2, y2, x3, y3) => {
                let (x1, y1) = mat_apply(ctm, *x1, *y1);
                let (x2, y2) = mat_apply(ctm, *x2, *y2);
                let (x3, y3) = mat_apply(ctm, *x3, *y3);
                path.elements
                    .push(PathElement::CurveTo(crate::core::path::BezierSegment {
                        start: path_current(&path).unwrap_or(AnchorPoint::new(x1, y1)),
                        control1: AnchorPoint::new(x1, y1),
                        control2: AnchorPoint::new(x2, y2),
                        end: AnchorPoint::new(x3, y3),
                    }));
            }
            Seg::Rect(x, y, w, h) => {
                let (x1, y1) = mat_apply(ctm, *x, *y);
                let (x2, y2) = mat_apply(ctm, *x + *w, *y + *h);
                // The transformed opposite corners define the axis-aligned
                // rectangle; arbitrary rotated rectangles remain a known gap.
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
    path
}

/// Raw CMYK samples (8-bit, `DeviceCMYK`) → PNG bytes via the importer's
/// naive CMYK→RGB model (same as `k`/`K` paint operators, so placed images
/// agree with vector fills from the same file).
fn cmyk_samples_to_png(raw: &[u8], w: u32, h: u32, bpc: u32) -> Option<Vec<u8>> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    if bpc != 8 {
        return None;
    }
    let px = (w as usize).checked_mul(h as usize)?;
    if px == 0 || px > 16_777_216 || raw.len() < px * 4 {
        return None;
    }
    let mut rgba = Vec::with_capacity(px * 4);
    for chunk in raw[..px * 4].as_chunks::<4>().0 {
        let [r, g, b, _] = cmyk_to_rgb(
            chunk[0] as f64 / 255.0,
            chunk[1] as f64 / 255.0,
            chunk[2] as f64 / 255.0,
            chunk[3] as f64 / 255.0,
        );
        rgba.extend_from_slice(&[
            (r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (b.clamp(0.0, 1.0) * 255.0).round() as u8,
            255,
        ]);
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(png)
}

/// Lab samples (8-bit) → PNG bytes through `Decode` and D50 Lab.
fn lab_samples_to_png(
    raw: &[u8],
    w: u32,
    h: u32,
    bpc: u32,
    params: &LabParams,
    decode: &[f64; 6],
) -> Option<Vec<u8>> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    if bpc != 8 {
        return None;
    }
    let px = (w as usize).checked_mul(h as usize)?;
    if px == 0 || px > 16_777_216 || raw.len() < px * 3 {
        return None;
    }
    let mut rgba = Vec::with_capacity(px * 4);
    for chunk in raw[..px * 3].as_chunks::<3>().0 {
        let v = [
            chunk[0] as f64 / 255.0,
            chunk[1] as f64 / 255.0,
            chunk[2] as f64 / 255.0,
        ];
        let l = decode[0] + v[0] * (decode[1] - decode[0]);
        let a = decode[2] + v[1] * (decode[3] - decode[2]);
        let b = decode[4] + v[2] * (decode[5] - decode[4]);
        let [r, g, bl, _] = lab_to_rgb(l, a, b, params.white);
        rgba.extend_from_slice(&[
            (r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (bl.clamp(0.0, 1.0) * 255.0).round() as u8,
            255,
        ]);
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(png)
}

/// Indexed samples → PNG bytes via the lookup table. Supports 1/2/4/8-bit
/// indices over RGB or Gray bases; out-of-range indices clamp to `hival`.
fn indexed_samples_to_png(
    raw: &[u8],
    w: u32,
    h: u32,
    bpc: u32,
    base_channels: u32,
    lookup: &[u8],
    hival: usize,
) -> Option<Vec<u8>> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    if !(base_channels == 1 || base_channels == 3) || hival > 255 {
        return None;
    }
    if !matches!(bpc, 1 | 2 | 4 | 8) {
        return None;
    }
    let px = (w as usize).checked_mul(h as usize)?;
    if px == 0 || px > 16_777_216 {
        return None;
    }
    if lookup.len() < (hival + 1) * base_channels as usize {
        return None;
    }
    let per_byte = 8 / bpc as usize;
    let need = if bpc == 8 { px } else { px.div_ceil(per_byte) };
    if raw.len() < need {
        return None;
    }
    let mut rgba = Vec::with_capacity(px * 4);
    for i in 0..px {
        let index = if bpc == 8 {
            raw[i] as usize
        } else {
            let byte = raw[i / per_byte];
            let shift = 8 - bpc * ((i % per_byte) as u32 + 1);
            ((byte >> shift) & ((1 << bpc) - 1)) as usize
        }
        .min(hival);
        let base = index * base_channels as usize;
        let (r, g, b) = if base_channels == 3 {
            (lookup[base], lookup[base + 1], lookup[base + 2])
        } else {
            let v = lookup[base];
            (v, v, v)
        };
        rgba.extend_from_slice(&[r, g, b, 255]);
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(png)
}

/// Raw `/SMask` samples → alpha bytes (0 = transparent). Supports 1/2/4/8-bit
/// gray; other depths return `None` so the caller keeps opaque output.
fn smask_raw_to_alpha(raw: &[u8], w: u32, h: u32, bpc: u32) -> Option<Vec<u8>> {
    let px = (w as usize).checked_mul(h as usize)?;
    if px == 0 || px > 16_777_216 {
        return None;
    }
    match bpc {
        8 if raw.len() >= px => Some(raw[..px].to_vec()),
        1 if raw.len() * 8 >= px => {
            let mut out = Vec::with_capacity(px);
            for i in 0..px {
                out.push(if raw[i / 8] & (0x80 >> (i % 8)) != 0 {
                    255
                } else {
                    0
                });
            }
            Some(out)
        }
        2 | 4 => {
            let max = ((1u16 << bpc) - 1) as f32;
            let per_byte = 8 / bpc as usize;
            let need = px.div_ceil(per_byte);
            if raw.len() < need {
                return None;
            }
            let mut out = Vec::with_capacity(px);
            for i in 0..px {
                let byte = raw[i / per_byte];
                let shift = 8 - bpc * ((i % per_byte) as u32 + 1);
                let v = ((byte >> shift) & ((1 << bpc) - 1)) as f32;
                out.push((v / max * 255.0).round() as u8);
            }
            Some(out)
        }
        _ => None,
    }
}

/// Nearest-neighbor alpha resize (mask dimensions ≠ base dimensions).
fn resize_alpha_nearest(alpha: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return Vec::new();
    }
    if alpha.len() != (sw.saturating_mul(sh)) as usize {
        return Vec::new();
    }
    let mut out = Vec::with_capacity((dw.saturating_mul(dh)) as usize);
    for y in 0..dh {
        let sy = (y as u64 * sh as u64 / dh as u64) as u32;
        for x in 0..dw {
            let sx = (x as u64 * sw as u64 / dw as u64) as u32;
            out.push(alpha[(sy.saturating_mul(sw).saturating_add(sx)) as usize]);
        }
    }
    out
}

/// Multiply `alpha` into PNG bytes, returning fresh PNG bytes.
fn apply_alpha_to_png(base_png: &[u8], alpha: &[u8]) -> Option<Vec<u8>> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    let mut base = image::load_from_memory(base_png).ok()?.to_rgba8();
    if (base.width().saturating_mul(base.height())) as usize != alpha.len() {
        return None;
    }
    for (pixel, mask) in base.pixels_mut().zip(alpha.iter()) {
        let a = (pixel[3] as u16 * (*mask as u16) / 255) as u8;
        pixel[3] = a;
    }
    let (w, h) = (base.width(), base.height());
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(base.as_raw(), w, h, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(out)
}

/// One mesh vertex: shading-space position plus color components.
type MeshVertex = (f64, f64, Vec<f64>);
/// One flat mesh triangle with per-vertex colors.
type MeshTriangle = [MeshVertex; 3];
/// Previous right edge for flag-1 continuation: points B→T plus the two
/// endpoint colors.
type PatchEdge = ([(f64, f64); 4], [Vec<f64>; 2]);

/// MSB-first bit cursor over mesh/shading data streams.
struct MeshBitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl MeshBitReader<'_> {
    fn read(&mut self, n: u32) -> Option<u64> {
        if n == 0 || n > 32 {
            return None;
        }
        let mut value = 0u64;
        for _ in 0..n {
            let byte = *self.data.get(self.bit / 8)?;
            value = (value << 1) | ((byte >> (7 - (self.bit % 8))) & 1) as u64;
            self.bit += 1;
        }
        Some(value)
    }
}

/// Mesh vertex records: `(flag, x, y, colors)` with `Decode`-mapped values.
/// Supports flag widths 2/4/8, coordinates 1–32 bits and components
/// 1/2/4/8/12/16 bits; the vertex count is capped so hostile streams stay
/// bounded.
fn decode_mesh_vertices(
    data: &[u8],
    flag_bits: u32,
    coord_bits: u32,
    comp_bits: u32,
    decode: &[f64],
    components: usize,
    max_vertices: usize,
) -> Vec<(u8, f64, f64, Vec<f64>)> {
    let mut bits = MeshBitReader { data, bit: 0 };
    let map = |raw: u64, bits: u32, lo: f64, hi: f64| -> f64 {
        let denom = ((1u64 << bits) - 1) as f64;
        lo + raw as f64 / denom * (hi - lo)
    };
    let mut out = Vec::new();
    loop {
        if out.len() >= max_vertices {
            break;
        }
        let Some(flag) = bits.read(flag_bits) else {
            break;
        };
        let (Some(rx), Some(ry)) = (bits.read(coord_bits), bits.read(coord_bits)) else {
            break;
        };
        let mut colors = Vec::with_capacity(components);
        let mut ok = true;
        for j in 0..components {
            match bits.read(comp_bits) {
                Some(raw) => {
                    let (lo, hi) = (decode[4 + 2 * j], decode[5 + 2 * j]);
                    colors.push(map(raw, comp_bits, lo, hi));
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            break;
        }
        out.push((
            flag as u8,
            map(rx, coord_bits, decode[0], decode[1]),
            map(ry, coord_bits, decode[2], decode[3]),
            colors,
        ));
    }
    out
}

/// Cubic Bezier point for patch boundaries.
fn cubic_point(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    t: f64,
) -> (f64, f64) {
    let s = 1.0 - t;
    let (a, b, c, d) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
    (
        a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0,
        a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1,
    )
}

/// Cubic Bernstein basis for tensor patches.
fn bernstein(t: f64) -> [f64; 4] {
    let s = 1.0 - t;
    [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t]
}

/// Coons surface point from the four boundary cubics (each ordered from
/// the (0,0) corner outward: bottom L→R, right B→T, top L→R, left B→T).
fn coons_point(
    bottom: &[(f64, f64); 4],
    right: &[(f64, f64); 4],
    top: &[(f64, f64); 4],
    left: &[(f64, f64); 4],
    u: f64,
    v: f64,
) -> (f64, f64) {
    let b = cubic_point(bottom[0], bottom[1], bottom[2], bottom[3], u);
    let t = cubic_point(top[0], top[1], top[2], top[3], u);
    let l = cubic_point(left[0], left[1], left[2], left[3], v);
    let r = cubic_point(right[0], right[1], right[2], right[3], v);
    let (c00, c10, c01, c11) = (bottom[0], bottom[3], top[0], top[3]);
    let blend = |a: f64, b: f64, c: f64, d: f64| {
        (1.0 - u) * (1.0 - v) * a + u * (1.0 - v) * b + (1.0 - u) * v * c + u * v * d
    };
    (
        (1.0 - v) * b.0 + v * t.0 + (1.0 - u) * l.0 + u * r.0 - blend(c00.0, c10.0, c01.0, c11.0),
        (1.0 - v) * b.1 + v * t.1 + (1.0 - u) * l.1 + u * r.1 - blend(c00.1, c10.1, c01.1, c11.1),
    )
}

/// Bilinear corner-color blend matching the Coons geometry blend.
fn bilinear_color(c00: &[f64], c10: &[f64], c01: &[f64], c11: &[f64], u: f64, v: f64) -> Vec<f64> {
    c00.iter()
        .zip(c10)
        .zip(c01)
        .zip(c11)
        .map(|(((a, b), c), d)| {
            (1.0 - u) * (1.0 - v) * a + u * (1.0 - v) * b + (1.0 - u) * v * c + u * v * d
        })
        .collect()
}

/// Raw image samples (post-filter, post-predictor via lopdf) → PNG bytes.
/// Supports 8-bit gray/RGB and 1-bit gray.
fn samples_to_png(raw: &[u8], w: u32, h: u32, channels: u32, bpc: u32) -> Option<Vec<u8>> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    let px = (w as usize).checked_mul(h as usize)?;
    // Callers already cap pixel count; re-check so this helper is safe alone.
    if px > 16_777_216 {
        return None;
    }
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
                let v = if raw[i / 8] & (0x80 >> (i % 8)) != 0 {
                    0u8
                } else {
                    255u8
                };
                out.extend_from_slice(&[v, v, v]);
            }
            out
        }
        _ => return None,
    };
    // Opaque alpha.
    let mut rgba = Vec::with_capacity(px * 4);
    for p in rgb.as_chunks::<3>().0 {
        rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(png)
}

#[cfg(test)]
mod smask_tests {
    use super::*;

    #[test]
    fn cmyk_samples_decode_to_expected_rgb() {
        // White (0,0,0,0), black (0,0,0,255), pure-C, pure-M.
        let raw = [
            0, 0, 0, 0, //
            0, 0, 0, 255, //
            255, 0, 0, 0, //
            0, 255, 0, 0, //
        ];
        let png = cmyk_samples_to_png(&raw, 2, 2, 8).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(&img.get_pixel(0, 0).0[..3], &[255u8, 255u8, 255u8]);
        assert_eq!(&img.get_pixel(1, 0).0[..3], &[0u8, 0u8, 0u8]);
        assert_eq!(&img.get_pixel(0, 1).0[..3], &[0u8, 255u8, 255u8]);
        assert_eq!(&img.get_pixel(1, 1).0[..3], &[255u8, 0u8, 255u8]);
        assert!(cmyk_samples_to_png(&raw, 2, 2, 1).is_none());
        assert!(cmyk_samples_to_png(&raw[..3], 2, 2, 8).is_none());
    }

    #[test]
    fn smask_alpha_decodes_bit_depths() {
        assert_eq!(
            smask_raw_to_alpha(&[0, 128, 255], 3, 1, 8).unwrap(),
            vec![0, 128, 255]
        );
        // 1-bit: set bit = opaque.
        assert_eq!(
            smask_raw_to_alpha(&[0b1010_0000], 4, 1, 1).unwrap(),
            vec![255, 0, 255, 0]
        );
        // 4-bit full white nibbles.
        assert_eq!(
            smask_raw_to_alpha(&[0xFF], 2, 1, 4).unwrap(),
            vec![255, 255]
        );
        assert!(smask_raw_to_alpha(&[0], 0, 1, 8).is_none());
        assert!(smask_raw_to_alpha(&[0], 1, 1, 16).is_none());
    }

    #[test]
    fn lab_to_srgb_matches_known_colors() {
        let white = [0.9642, 1.0, 0.8251];
        let near = |got: [f32; 4], want: [f32; 3]| {
            got[..3]
                .iter()
                .zip(want)
                .all(|(g, w)| (*g - w).abs() < 0.03)
        };
        assert!(near(lab_to_rgb(100.0, 0.0, 0.0, white), [1.0, 1.0, 1.0]));
        assert!(near(lab_to_rgb(0.0, 0.0, 0.0, white), [0.0, 0.0, 0.0]));
        // sRGB red is roughly Lab(54.3, 80.8, 69.9) under D50.
        assert!(near(
            lab_to_rgb(54.29, 80.80, 69.94, white),
            [1.0, 0.0, 0.0]
        ));
    }

    #[test]
    fn lab_samples_apply_decode_and_range() {
        let params = LabParams {
            white: [0.9642, 1.0, 0.8251],
            range: [-100.0, 100.0, -100.0, 100.0],
        };
        let decode = [0.0, 100.0, -100.0, 100.0, -100.0, 100.0];
        // White then black pixels.
        let png =
            lab_samples_to_png(&[255, 128, 128, 0, 128, 128], 2, 1, 8, &params, &decode).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert!(img.get_pixel(0, 0).0[..3].iter().all(|v| *v > 240));
        assert!(img.get_pixel(1, 0).0[..3].iter().all(|v| *v < 15));
        assert!(lab_samples_to_png(&[0, 0, 0], 1, 1, 1, &params, &decode).is_none());
    }

    #[test]
    fn indexed_samples_map_through_lookup() {
        // RGB base: red, green.
        let lookup = [255, 0, 0, 0, 255, 0];
        let png = indexed_samples_to_png(&[0, 1, 1, 0], 2, 2, 8, 3, &lookup, 1).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(&img.get_pixel(0, 0).0[..3], &[255u8, 0u8, 0u8]);
        assert_eq!(&img.get_pixel(1, 0).0[..3], &[0u8, 255u8, 0u8]);
        // Gray base replicates the single channel.
        let gray = indexed_samples_to_png(&[0, 1], 2, 1, 8, 1, &[0, 255], 1).unwrap();
        let img = image::load_from_memory(&gray).unwrap().to_rgba8();
        assert_eq!(&img.get_pixel(0, 0).0[..3], &[0u8, 0u8, 0u8]);
        assert_eq!(&img.get_pixel(1, 0).0[..3], &[255u8, 255u8, 255u8]);
        // 2-bit packing plus hival clamping.
        // 0b01000000 packs 2-bit indices [1, 0, ...] from the top bits.
        let packed = indexed_samples_to_png(&[0b01000000], 2, 1, 2, 3, &lookup, 1).unwrap();
        let img = image::load_from_memory(&packed).unwrap().to_rgba8();
        assert_eq!(&img.get_pixel(0, 0).0[..3], &[0u8, 255u8, 0u8]);
        assert!(indexed_samples_to_png(&[0], 1, 1, 8, 3, &[255, 0], 1).is_none());
        assert!(indexed_samples_to_png(&[0], 1, 1, 16, 3, &lookup, 1).is_none());
    }

    #[test]
    fn smask_alpha_resize_and_composite() {
        // 1x1 → 2x2 nearest neighbor replicates the single value.
        assert_eq!(
            resize_alpha_nearest(&[128], 1, 1, 2, 2),
            vec![128, 128, 128, 128]
        );
        let base = samples_to_png(&[255, 0, 0, 0, 255, 0], 2, 1, 3, 8).unwrap();
        let out = apply_alpha_to_png(&base, &[0, 255]).unwrap();
        let img = image::load_from_memory(&out).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(1, 0)[3], 255);
        assert!(apply_alpha_to_png(&base, &[255]).is_none());
    }
}
