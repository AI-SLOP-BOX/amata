use fontdb::{Database, Family, Query};
use std::path::Path;
use std::sync::OnceLock;

use super::document::{FontStyle, VariationSetting};

/// Central Font Registry for Amata.
/// Manages system and bundled fonts, family resolution, fallback, and raw glyph data access.
/// Prevents expensive per-frame font scans by caching the font database at startup.
pub struct FontRegistry {
    db: Database,
    cached_families: Vec<String>,
}

static GLOBAL_REGISTRY: OnceLock<FontRegistry> = OnceLock::new();

impl Default for FontRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FontRegistry {
    /// Initialize registry with system fonts and bundled assets/fonts
    pub fn new() -> Self {
        let mut db = Database::new();

        // 1. Load bundled fonts first
        let bundled_dirs = ["assets/fonts", "fonts"];
        for dir in bundled_dirs {
            let p = Path::new(dir);
            if p.is_dir() {
                db.load_fonts_dir(p);
            }
        }

        // 2. Load system fonts
        db.load_system_fonts();

        // 3. Extract and deduplicate font families
        let mut families = Vec::new();
        for face in db.faces() {
            for (fam, _) in &face.families {
                if !families.contains(fam) && !fam.is_empty() {
                    families.push(fam.clone());
                }
            }
        }
        families.sort_by_key(|a| a.to_lowercase());

        Self {
            db,
            cached_families: families,
        }
    }

    /// Access the global singleton instance (initialized lazily once)
    pub fn global() -> &'static FontRegistry {
        GLOBAL_REGISTRY.get_or_init(Self::new)
    }

    /// Get underlying fontdb Database reference (for resvg, etc.)
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// Return all unique font family names available on the system
    pub fn list_families(&self) -> &[String] {
        &self.cached_families
    }

    /// Check if a single font family is installed or bundled
    pub fn is_family_available(&self, family: &str) -> bool {
        let trimmed = family.trim().trim_matches('\'').trim_matches('"');
        self.cached_families
            .iter()
            .any(|f| f.eq_ignore_ascii_case(trimmed))
    }

    /// Check if any family in a comma-separated list is available.
    /// E.g. "Inter, Helvetica, sans-serif" returns true if any is present.
    pub fn is_any_family_available(&self, font_family_spec: &str) -> bool {
        for candidate in font_family_spec.split(',') {
            let name = candidate.trim().trim_matches('\'').trim_matches('"');
            if name.eq_ignore_ascii_case("sans-serif")
                || name.eq_ignore_ascii_case("serif")
                || name.eq_ignore_ascii_case("monospace")
                || name.eq_ignore_ascii_case("cursive")
                || name.eq_ignore_ascii_case("fantasy")
                || name.eq_ignore_ascii_case("system-ui")
                || name.eq_ignore_ascii_case("-apple-system")
            {
                return true;
            }
            if self.is_family_available(name) {
                return true;
            }
        }
        false
    }

    /// Resolve a font family spec (e.g. "CustomFont, Inter, sans-serif") to the first available concrete family name.
    /// If none matches, returns None (caller can fallback safely without mutating document).
    pub fn resolve_family(&self, font_family_spec: &str) -> Option<String> {
        for candidate in font_family_spec.split(',') {
            let name = candidate.trim().trim_matches('\'').trim_matches('"');
            for f in &self.cached_families {
                if f.eq_ignore_ascii_case(name) {
                    return Some(f.clone());
                }
            }
        }
        None
    }

    /// Query raw font file data and face index for a given family, weight, and style.
    /// Useful for ttf-parser outline extraction.
    pub fn query_face_data<R, F: FnOnce(&[u8], u32) -> R>(
        &self,
        family: &str,
        weight: u16,
        style: FontStyle,
        f: F,
    ) -> Option<R> {
        let weight_val = fontdb::Weight(weight);
        let style_val = match style {
            FontStyle::Normal => fontdb::Style::Normal,
            FontStyle::Italic => fontdb::Style::Italic,
            FontStyle::Oblique => fontdb::Style::Oblique,
        };

        // The caller may pass a CSS-like spec ("Inter, sans-serif").
        // fontdb matches Family::Name exactly, so resolve candidates first,
        // then invoke the FnOnce callback exactly once.
        let mut matched_id = None;
        for candidate in family.split(',') {
            let name = candidate.trim().trim_matches('\'').trim_matches('"');
            if name.is_empty() {
                continue;
            }
            if name.eq_ignore_ascii_case("sans-serif")
                || name.eq_ignore_ascii_case("serif")
                || name.eq_ignore_ascii_case("monospace")
                || name.eq_ignore_ascii_case("cursive")
                || name.eq_ignore_ascii_case("fantasy")
                || name.eq_ignore_ascii_case("system-ui")
            {
                continue;
            }
            let query = Query {
                families: &[Family::Name(name), Family::SansSerif],
                weight: weight_val,
                style: style_val,
                stretch: fontdb::Stretch::Normal,
            };
            if let Some(id) = self.db.query(&query) {
                matched_id = Some(id);
                break;
            }
        }

        if let Some(id) = matched_id {
            return self.db.with_face_data(id, f);
        }

        // Fallback to sans-serif
        let fallback_query = Query {
            families: &[Family::SansSerif],
            weight: weight_val,
            style: style_val,
            stretch: fontdb::Stretch::Normal,
        };
        if let Some(id) = self.db.query(&fallback_query) {
            return self.db.with_face_data(id, f);
        }

        None
    }

    /// True when no installed face matches the requested style, i.e. the
    /// style must be *synthesized* (faux italic/oblique) instead of taken
    /// from a real face. fontdb always returns the closest face, so a query
    /// for Italic can silently resolve to an upright face — this detects
    /// exactly that case by comparing the matched face's real style.
    ///
    /// Semantics per candidate kind: a concrete installed family answers
    /// for itself; a missing family forces faux (deterministic across
    /// viewers, whose own fallbacks may or may not have italics); generic
    /// aliases (`sans-serif`, …) resolve through the local sans fallback.
    pub fn needs_synthetic_style(&self, family: &str, weight: u16, style: FontStyle) -> bool {
        let wanted = match style {
            FontStyle::Normal => return false,
            FontStyle::Italic => fontdb::Style::Italic,
            FontStyle::Oblique => fontdb::Style::Oblique,
        };
        let weight_val = fontdb::Weight(weight);
        // CSS font matching accepts italic and oblique interchangeably: any
        // slanted face satisfies either request (and must NOT be sheared on
        // top, or it would double-slant).
        let is_slanted =
            |s: fontdb::Style| matches!(s, fontdb::Style::Italic | fontdb::Style::Oblique);
        let check = |families: &[Family]| -> Option<bool> {
            let id = self.db.query(&Query {
                families,
                weight: weight_val,
                style: wanted,
                stretch: fontdb::Stretch::Normal,
            })?;
            Some(self.db.face(id).map(|face| !is_slanted(face.style)).unwrap_or(true))
        };
        fn is_generic(name: &str) -> bool {
            name.eq_ignore_ascii_case("sans-serif")
                || name.eq_ignore_ascii_case("serif")
                || name.eq_ignore_ascii_case("monospace")
                || name.eq_ignore_ascii_case("cursive")
                || name.eq_ignore_ascii_case("fantasy")
                || name.eq_ignore_ascii_case("system-ui")
        }
        let mut saw_generic = false;
        for candidate in family.split(',') {
            let name = candidate.trim().trim_matches('\'').trim_matches('"');
            if name.is_empty() {
                continue;
            }
            if is_generic(name) {
                saw_generic = true;
                continue;
            }
            if !self.is_family_available(name) {
                continue;
            }
            // Installed concrete family: restrict the query to it so the
            // answer is about this family, not the sans fallback.
            return check(&[Family::Name(name)]).unwrap_or(true);
        }
        if saw_generic {
            return check(&[Family::SansSerif]).unwrap_or(true);
        }
        // No resolvable family at all: synthesize deterministically.
        true
    }

    /// Variation axes exposed by the face that would resolve for this family.
    /// Empty when the face is static or no face resolves.
    ///
    /// `weight`/`style` select the face the same way outline extraction does,
    /// so the axes reported here are the ones `set_variation` will apply to.
    pub fn variation_axes(
        &self,
        family: &str,
        weight: u16,
        style: FontStyle,
    ) -> Vec<AxisDescriptor> {
        self.query_face_data(family, weight, style, |data, index| {
            let Ok(face) = ttf_parser::Face::parse(data, index) else {
                return Vec::new();
            };
            let mut axes = Vec::new();
            for ax in face.variation_axes() {
                // Prefer a Unicode name record for the axis; fall back to the tag.
                let name = face
                    .names()
                    .into_iter()
                    .filter(|n| n.name_id == ax.name_id)
                    .find_map(|n| n.to_string())
                    .unwrap_or_else(|| ax.tag.to_string());
                axes.push(AxisDescriptor {
                    tag: ax.tag.to_string(),
                    name,
                    min: ax.min_value,
                    max: ax.max_value,
                    default: ax.def_value,
                });
            }
            axes
        })
        .unwrap_or_default()
    }

    /// Convenience: axes for a style's family (weight/style from the style).
    pub fn variation_axes_for_style(
        &self,
        family: &str,
        weight: u16,
        style: FontStyle,
    ) -> Vec<AxisDescriptor> {
        self.variation_axes(family, weight, style)
    }

    /// Apply a style's `variations` list to a parsed face (in place).
    /// Unknown tags / static faces are ignored (no error — the outline still
    /// renders at default coordinates).
    pub fn apply_variations(face: &mut ttf_parser::Face<'_>, variations: &[VariationSetting]) {
        for v in variations {
            if v.axis.len() != 4 {
                continue;
            }
            let mut tag = [0u8; 4];
            tag.copy_from_slice(v.axis.as_bytes());
            let _ = face.set_variation(ttf_parser::Tag::from_bytes(&tag), v.value as f32);
        }
    }
}

/// A single `fvar` axis as seen by the UI (tag + human name + range).
#[derive(Debug, Clone, PartialEq)]
pub struct AxisDescriptor {
    /// 4-char OpenType tag (`wght`, `wdth`, `opsz`, …).
    pub tag: String,
    /// Display name from the `name` table (falls back to the tag).
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl AxisDescriptor {
    /// CSS/OpenType-friendly label, e.g. `Weight (wght)`.
    pub fn label(&self) -> String {
        format!("{} ({})", self.name, self.tag)
    }
}
