//! テンプレート・スウォッチなど「自作アセット」を読み込む仕組み。
//!
//! アセットは `assets/` 以下にコンパイル時埋め込み（`include_str!`）:
//! 実行時にファイルシステムを触らないので、アプリ単体で配布でき、
//! パス解決も不要。テンプレートは `.amata`（プロジェクトJSON）そのまま。

use crate::core::document::Document;
use crate::core::print::SpotColor;

/// A bundled swatch (ink) library.
#[derive(Debug, Clone, PartialEq)]
pub struct SwatchLibrary {
    /// Library name shown in the UI.
    pub name: String,
    /// `(slug, cmyk)` pairs, in file order.
    pub colors: Vec<(String, [f32; 4])>,
}

impl SwatchLibrary {
    /// Convert to document spots, skipping names already registered.
    pub fn to_spots(&self) -> Vec<SpotColor> {
        self.colors
            .iter()
            .map(|(slug, cmyk)| SpotColor::new(slug.clone(), *cmyk))
            .collect()
    }
}

/// Raw on-disk shape of a swatch library file.
#[derive(Debug, Clone, serde::Deserialize)]
struct SwatchFile {
    name: String,
    colors: Vec<SwatchEntry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SwatchEntry {
    slug: String,
    cmyk: [f32; 4],
}

/// A bundled template project.
#[derive(Debug, Clone)]
pub struct Template {
    /// Stable id (file stem).
    pub id: &'static str,
    /// Display name (the project's own name).
    pub title: String,
    /// Artboard size in px, for the picker's thumbnail text.
    pub size: (f64, f64),
    /// Parsed project; callers clone before editing.
    pub document: Document,
}

const SWATCH_TRADITIONAL_JP: &str = include_str!("../../assets/swatches/traditional-jp.json");
const SWATCH_DIC_APPROX: &str = include_str!("../../assets/swatches/dic-approx.json");
const SWATCH_NEUTRAL_GRAYS: &str = include_str!("../../assets/swatches/neutral-grays.json");
const SWATCH_UI_FLAT: &str = include_str!("../../assets/swatches/ui-flat.json");
const TEMPLATE_A4_POSTER: &str = include_str!("../../assets/templates/a4-poster-vertical.amata");
const TEMPLATE_BUSINESS_CARD: &str = include_str!("../../assets/templates/business-card.amata");
const TEMPLATE_FLYER_2COL: &str = include_str!("../../assets/templates/flyer-2col.amata");
const TEMPLATE_INVOICE_A4: &str = include_str!("../../assets/templates/invoice-a4.amata");
const TEMPLATE_NEWYEAR_CARD: &str = include_str!("../../assets/templates/newyear-card.amata");
const TEMPLATE_STORY_9X16: &str = include_str!("../../assets/templates/story-9x16.amata");
const TEMPLATE_NOTEBOOK_GRID: &str = include_str!("../../assets/templates/notebook-grid.amata");

/// Every bundled swatch library, in menu order.
pub fn built_in_swatches() -> Vec<SwatchLibrary> {
    [
        SWATCH_TRADITIONAL_JP,
        SWATCH_DIC_APPROX,
        SWATCH_NEUTRAL_GRAYS,
        SWATCH_UI_FLAT,
    ]
    .into_iter()
    .filter_map(parse_swatch_file)
    .collect()
}

fn parse_swatch_file(json: &'static str) -> Option<SwatchLibrary> {
    let file: SwatchFile = match serde_json::from_str(json) {
        Ok(f) => f,
        Err(e) => {
            log::error!("bundled swatch library failed to parse: {e}");
            return None;
        }
    };
    let colors = file.colors.into_iter().map(|c| (c.slug, c.cmyk)).collect();
    Some(SwatchLibrary {
        name: file.name,
        colors,
    })
}

/// Every bundled template, in picker order.
pub fn built_in_templates() -> Vec<Template> {
    [
        ("a4-poster-vertical", TEMPLATE_A4_POSTER),
        ("business-card", TEMPLATE_BUSINESS_CARD),
        ("flyer-2col", TEMPLATE_FLYER_2COL),
        ("invoice-a4", TEMPLATE_INVOICE_A4),
        ("newyear-card", TEMPLATE_NEWYEAR_CARD),
        ("story-9x16", TEMPLATE_STORY_9X16),
        ("notebook-grid", TEMPLATE_NOTEBOOK_GRID),
    ]
    .into_iter()
    .filter_map(|(id, json)| parse_template(id, json))
    .collect()
}

fn parse_template(id: &'static str, json: &'static str) -> Option<Template> {
    let doc: Document = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(e) => {
            log::error!("bundled template {id} failed to parse: {e}");
            return None;
        }
    };
    let title = doc.name.clone();
    let size = (doc.width, doc.height);
    Some(Template {
        id,
        title,
        size,
        document: doc,
    })
}

/// Look up a template by id.
pub fn template_by_id(id: &str) -> Option<Template> {
    built_in_templates().into_iter().find(|t| t.id == id)
}
