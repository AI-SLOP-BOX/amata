use fontdb::{Database, Family, Query};
use std::path::Path;
use std::sync::OnceLock;

use super::document::FontStyle;

/// Central Font Registry for Amata.
/// Manages system and bundled fonts, family resolution, fallback, and raw glyph data access.
/// Prevents expensive per-frame font scans by caching the font database at startup.
pub struct FontRegistry {
    db: Database,
    cached_families: Vec<String>,
}

static GLOBAL_REGISTRY: OnceLock<FontRegistry> = OnceLock::new();

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
        families.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

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

        // Try direct family query first
        let query = Query {
            families: &[Family::Name(family), Family::SansSerif],
            weight: weight_val,
            style: style_val,
            stretch: fontdb::Stretch::Normal,
        };

        if let Some(id) = self.db.query(&query) {
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
}
