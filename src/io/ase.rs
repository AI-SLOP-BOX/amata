//! Adobe Swatch Exchange (`.ase`) colour swatch import.
//!
//! This is a deliberately small, *read-only* subset: spot colours land
//! in the document's spot library ([`crate::core::document::Document`]'s
//! `spots`) so fills can reference them and the press exporter can emit
//! a Separation colour space per plate. There is no `.ase` writer —
//! swatch round-tripping through third-party libraries is not a target
//! (see `docs/COMPATIBILITY.md`), and colour groups are flattened.
//!
//! Colour models:
//! - `RGB `  → naive-UCR CMYK (same model as the UI pickers)
//! - `CMYK` → taken as-is
//! - `Gray` → K only
//! - `LAB ` → D50 Lab → XYZ → sRGB → naive-UCR CMYK
//!
//! ASE LAB is a D50 colour; the conversion below uses the standard
//! Bradford D50→D65 adaptation so previews match what Photoshop shows
//! on a normal display. Values are process fallbacks, not ink
//! measurements — same contract as the Pantone kit.

use crate::core::print::{rgb_to_cmyk_ink, SpotColor};
use std::path::Path;

/// Why an `.ase` file could not be read.
#[derive(Debug, Clone, PartialEq)]
pub enum AseError {
    /// File is not an ASE container (bad signature/version).
    NotAse,
    /// File ends in the middle of a block.
    Truncated,
    /// I/O failure (missing file, permissions, …).
    Io(String),
}

impl std::fmt::Display for AseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AseError::NotAse => "not an Adobe Swatch Exchange file",
            AseError::Truncated => "swatch data is truncated",
            AseError::Io(e) => e.as_str(),
        })
    }
}

impl std::error::Error for AseError {}

/// Import an `.ase` file into spot colours (document order).
///
/// Group blocks (`c001`/`c002`) are containers only: their colours join
/// the flat list. Unknown colour models are skipped rather than
/// failing the whole file.
pub fn import_ase(path: &Path) -> Result<Vec<SpotColor>, AseError> {
    let bytes = std::fs::read(path).map_err(|e| AseError::Io(e.to_string()))?;
    import_ase_bytes(&bytes)
}

/// Byte-slice form of [`import_ase`] (used by tests and the API server).
pub fn import_ase_bytes(bytes: &[u8]) -> Result<Vec<SpotColor>, AseError> {
    let mut r = Reader::new(bytes);
    if r.take(4) != Some(b"ASEF".as_slice()) {
        return Err(AseError::NotAse);
    }
    // Version: 1.0 = 0x0001, 2.0 = 0x0002 (both are read the same way).
    let version = r.u16_be()?;
    if version != 1 && version != 2 {
        return Err(AseError::NotAse);
    }
    let blocks = r.u32_be()? as usize;
    // A sane ceiling: a hand-edited count must not pre-allocate wildly.
    let mut out: Vec<SpotColor> = Vec::new();
    for _ in 0..blocks.min(100_000) {
        let block_type = match r.u16_be() {
            Ok(v) => v,
            Err(AseError::Truncated) if out.is_empty() => return Err(AseError::Truncated),
            // Trailing junk after the last complete block: keep what we got.
            Err(AseError::Truncated) => break,
            Err(e) => return Err(e),
        };
        let size = r.u32_be()? as usize;
        let end = r.pos + size;
        if end > bytes.len() {
            return Err(AseError::Truncated);
        }
        let body = &bytes[r.pos..end];
        r.pos = end;
        // 0xC001 group start / 0xC002 group end: structure only.
        if block_type == 0x0001 {
            if let Some(spot) = parse_color_entry(body) {
                push_unique(&mut out, spot);
            }
        }
    }
    Ok(out)
}

/// Append a spot, replacing any earlier entry with the same name
/// (last-write-wins, like Photoshop's merge on import).
fn push_unique(out: &mut Vec<SpotColor>, spot: SpotColor) {
    if let Some(slot) = out.iter_mut().find(|s| s.name == spot.name) {
        *slot = spot;
    } else {
        out.push(spot);
    }
}

/// Parse one colour entry body: name (UTF-16BE, NUL-terminated) + model
/// + values. `name_len` counts UTF-16 code units *including* the NUL.
fn parse_color_entry(body: &[u8]) -> Option<SpotColor> {
    let mut r = Reader::new(body);
    let name_len = r.u16_be().ok()? as usize;
    // Name buffer: name_len code units, capped so a bogus length cannot
    // make us read far past the block.
    let name_units = name_len.min(body.len() / 2);
    let name_bytes = r.take(name_units * 2)?;
    let name = decode_utf16be(name_bytes)?;
    let name = name.trim().trim_end_matches('\0').to_string();
    if name.is_empty() {
        return None;
    }
    let model = r.take(4)?;
    let cmyk = match model {
        b"RGB " => {
            let v = [r.f32_be().ok()?, r.f32_be().ok()?, r.f32_be().ok()?];
            if !v.iter().all(|c| (0.0..=1.5).contains(c)) {
                return None;
            }
            rgb_to_cmyk_ink(v[0], v[1], v[2])
        }
        b"CMYK" => {
            let v = [
                r.f32_be().ok()?,
                r.f32_be().ok()?,
                r.f32_be().ok()?,
                r.f32_be().ok()?,
            ];
            if !v.iter().all(|c| (0.0..=1.5).contains(c)) {
                return None;
            }
            v
        }
        b"Gray" => {
            let g = r.f32_be().ok()?;
            if !(0.0..=1.5).contains(&g) {
                return None;
            }
            [0.0, 0.0, 0.0, g]
        }
        b"LAB " => {
            let l = r.f32_be().ok()?;
            let a = r.f32_be().ok()?;
            let b = r.f32_be().ok()?;
            if !(0.0..=100.0).contains(&l) || !(-128.0..=128.0).contains(&a) {
                return None;
            }
            let (r_, g_, b_) = lab_d50_to_srgb(l, a, b);
            rgb_to_cmyk_ink(r_, g_, b_)
        }
        // Unknown models: skip the swatch, keep the rest of the file.
        _ => return None,
    };
    Some(SpotColor::new(name, cmyk))
}

/// Minimal big-endian cursor over a byte slice.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        if end > self.bytes.len() {
            return None;
        }
        let s = &self.bytes[self.pos..end];
        self.pos = end;
        Some(s)
    }

    fn u16_be(&mut self) -> Result<u16, AseError> {
        let s = self.take(2).ok_or(AseError::Truncated)?;
        Ok(u16::from_be_bytes([s[0], s[1]]))
    }

    fn u32_be(&mut self) -> Result<u32, AseError> {
        let s = self.take(4).ok_or(AseError::Truncated)?;
        Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn f32_be(&mut self) -> Result<f32, AseError> {
        let v = self.u32_be()?;
        let f = f32::from_bits(v);
        if f.is_finite() {
            Ok(f)
        } else {
            Err(AseError::Truncated)
        }
    }
}

fn decode_utf16be(bytes: &[u8]) -> Option<String> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    // Trailing NUL is the ASE terminator, not part of the name.
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    let units = &units[..end];
    char::decode_utf16(units.iter().copied())
        .collect::<Result<String, _>>()
        .ok()
}

/// CIE L\*a\*b\* (D50) → sRGB in 0..=1, using the standard Bradford
/// D50→D65 chromatic adaptation.
fn lab_d50_to_srgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let finv = |t: f32| -> f32 {
        let d = 6.0f32 / 29.0;
        if t > d {
            t * t * t
        } else {
            3.0 * d * d * (t - 4.0 / 29.0)
        }
    };
    let fy = (l + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    // D50 white point.
    let x = 0.9642 * finv(fx);
    let y = 1.0000 * finv(fy);
    let z = 0.8249 * finv(fz);
    // Bradford D50 → D65.
    let [x, y, z] = [
        0.955_576_6 * x - 0.023_039_3 * y + 0.063_163_6 * z,
        -0.028_289_5 * x + 1.009_941_6 * y + 0.021_007_7 * z,
        0.012_298_2 * x - 0.020_483 * y + 1.329_909_8 * z,
    ];
    // XYZ (D65) → linear sRGB.
    let r = 3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z;
    let g = -0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z;
    let b = 0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z;
    let gam = |c: f32| -> f32 {
        let c = c.clamp(0.0, 1.0);
        if c <= 0.003_130_8 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    };
    (gam(r), gam(g), gam(b))
}
