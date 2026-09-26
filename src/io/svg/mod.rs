//! SVG import/export.
//!
//! Split by concern:
//! - [`parse`]: path-data / color / full-document parsing, gradients
//! - [`tokenize`]: raw tag tokenizer
//! - [`attrs`]: attribute extraction, points, affine transforms
//! - [`export`]: `Document` → SVG serialisation
//! - [`util`]: base64, color formatting, XML escaping
//!
//! The public API is re-exported here so existing
//! `irasu_illustrator::io::svg::*` call sites keep working unchanged.

mod attrs;
mod export;
mod parse;
mod tokenize;
mod util;

// Public API re-exports.  The `amata` binary compiles these modules
// independently of the library and may not consume every item, hence the
// allow on the re-export block.
#[allow(unused_imports)]
pub use export::{export_svg, export_svg_with_options, export_svg_with_profile};
#[allow(unused_imports)]
pub use parse::{parse_svg_color, parse_svg_document, parse_svg_path_data, try_parse_svg_document};
#[allow(unused_imports)]
pub use util::xml_unescape;

// Every submodule sees the full sibling surface via the parent glob, which
// keeps the split purely mechanical (no signature changes).
