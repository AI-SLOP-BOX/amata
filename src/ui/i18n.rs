//! Runtime localization backed by compile-time embedded JSON catalogs.
//!
//! Add a `locales/<BCP-47-tag>.json` file to add a language; `build.rs`
//! discovers and embeds every catalog automatically. Message IDs stay
//! language-neutral, and missing messages follow each catalog's fallback
//! chain before falling back to the message ID itself.

use serde::Deserialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/locale_catalogs.rs"));

#[derive(Debug, Deserialize)]
struct Catalog {
    name: String,
    fallback: Option<String>,
    messages: HashMap<String, String>,
}

static CATALOGS: OnceLock<HashMap<&'static str, Catalog>> = OnceLock::new();

fn catalogs() -> &'static HashMap<&'static str, Catalog> {
    CATALOGS.get_or_init(|| {
        LOCALE_CATALOGS
            .iter()
            .map(|(id, source)| {
                let catalog: Catalog = serde_json::from_str(source)
                    .unwrap_or_else(|error| panic!("invalid locale catalog {id}: {error}"));
                (*id, catalog)
            })
            .collect()
    })
}

/// Available locale tags and native display names, sorted by tag.
pub fn available_locales() -> Vec<(&'static str, &'static str)> {
    let mut locales: Vec<_> = catalogs()
        .iter()
        .map(|(id, catalog)| (*id, catalog.name.as_str()))
        .collect();
    locales.sort_unstable_by_key(|(id, _)| *id);
    locales
}

/// Native name for the currently selected locale (after compatibility
/// aliases and regional-tag resolution).
pub fn locale_name(locale: &str) -> &'static str {
    let resolved = resolve_locale(locale);
    catalogs()
        .get(resolved)
        .map(|catalog| catalog.name.as_str())
        .unwrap_or("日本語")
}

/// Resolve legacy preference values and regional tags to an installed
/// catalog. Existing `Japanese`/`English` preference files remain valid.
pub fn resolve_locale(requested: &str) -> &'static str {
    let lowered = requested.trim().replace('_', "-").to_ascii_lowercase();
    let mut candidate = match lowered.as_str() {
        "japanese" => "ja".to_string(),
        "english" => "en".to_string(),
        _ => lowered,
    };
    let catalogs = catalogs();
    // BCP-47 lookup truncates subtags progressively (e.g. zh-Hant-TW,
    // zh-Hant, zh), and compares tags case-insensitively.
    loop {
        if let Some(id) = catalogs
            .keys()
            .find(|id| id.eq_ignore_ascii_case(&candidate))
        {
            return id;
        }
        let Some((base, _)) = candidate.rsplit_once('-') else {
            break;
        };
        candidate = base.to_string();
    }
    "ja"
}

/// Translate a stable message ID using the requested locale's fallback
/// chain. Unknown IDs return unchanged, making partial catalogs safe.
pub fn text<'a>(locale: &str, message_id: &'a str) -> Cow<'a, str> {
    let catalogs = catalogs();
    // The normal frame-rendering path uses an exact tag and avoids locale
    // normalization/allocation. The catalog-count bound also makes a
    // malformed fallback cycle harmless.
    let mut current = catalogs
        .get_key_value(locale)
        .map(|(id, _)| *id)
        .unwrap_or_else(|| resolve_locale(locale));
    for _ in 0..=catalogs.len() {
        let Some(catalog) = catalogs.get(current) else {
            break;
        };
        if let Some(message) = catalog.messages.get(message_id) {
            return Cow::Borrowed(message);
        }
        let Some(fallback) = catalog.fallback.as_deref() else {
            break;
        };
        current = catalogs
            .get_key_value(fallback)
            .map(|(id, _)| *id)
            .unwrap_or_else(|| resolve_locale(fallback));
    }
    // Catalog values live in the process-wide OnceLock.
    catalogs
        .get("ja")
        .and_then(|catalog| catalog.messages.get(message_id))
        .map(|message| Cow::Borrowed(message.as_str()))
        .unwrap_or_else(|| Cow::Borrowed(message_id))
}

/// Format a catalog message with named placeholders (`{count}`, `{name}`).
/// Named arguments let translators reorder values for their grammar.
pub fn format(locale: &str, message_id: &str, args: &[(&str, &str)]) -> String {
    let mut message = text(locale, message_id).into_owned();
    for (name, value) in args {
        message = message.replace(&format!("{{{name}}}"), value);
    }
    message
}

/// Resolve an internal history action name to a stable catalog key.
/// Unknown names stay visible rather than disappearing from undo/redo.
pub fn history_name<'a>(locale: &str, name: &'a str) -> Cow<'a, str> {
    let key = match name {
        "Bring to Front" => "history.bring_to_front",
        "Bring Forward" => "history.bring_forward",
        "Send Backward" => "history.send_backward",
        "Send to Back" => "history.send_to_back",
        "Reorder Object" => "history.reorder_object",
        "Move Object in Tree" => "history.move_object_in_tree",
        "Group" => "history.group",
        "Ungroup" => "history.ungroup",
        "Make Compound Path" => "history.make_compound_path",
        "Release Compound" => "history.release_compound",
        "Clipping Mask" => "history.clipping_mask",
        "Create Outlines" => "history.create_outlines",
        "Simplify Path" => "history.simplify_path",
        "Pathfinder" => "history.pathfinder",
        "Edit Path Nodes" => "history.edit_path_nodes",
        "Align Left" => "history.align_left",
        "Align Center H" => "history.align_center_h",
        "Align Right" => "history.align_right",
        "Align Top" => "history.align_top",
        "Align Center V" => "history.align_center_v",
        "Align Bottom" => "history.align_bottom",
        "Distribute H" => "history.distribute_h",
        "Distribute V" => "history.distribute_v",
        "Add Object" => "history.add_object",
        "Remove Object" => "history.remove_object",
        "Edit Object" => "history.edit_object",
        "Move Object" => "history.move_object",
        "Edit Transform" => "history.edit_transform",
        "Change Typography" => "history.change_typography",
        "Cut Objects" => "history.cut_objects",
        "Paste Objects" => "history.paste_objects",
        "Duplicate Objects" => "history.duplicate_objects",
        "Alt Duplicate" => "history.alt_duplicate",
        "Delete Objects" => "history.delete_objects",
        "Move Objects" => "history.move_objects",
        "Move Multi" => "history.move_multi",
        "Nudge" => "history.nudge",
        "Add Layer" => "history.add_layer",
        "Delete Layer" => "history.delete_layer",
        "Edit Layer" => "history.edit_layer",
        "Reorder Layers" => "history.reorder_layers",
        "Neon Glow" => "history.neon_glow",
        "Grid Repeat" => "history.grid_repeat",
        "Radial Repeat" => "history.radial_repeat",
        "Add Auto Layout" => "history.add_auto_layout",
        _ => return Cow::Borrowed(name),
    };
    text(locale, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_are_discovered_and_fallbacks_work() {
        assert_eq!(text("en", "menu.edit"), "Edit");
        assert_eq!(text("ja", "menu.edit"), "編集");
        assert_eq!(text("en", "prefs.language.label"), "Language");
        assert_eq!(text("ja", "missing.id"), "missing.id");
        assert_eq!(
            format("en", "components.created", &[("id", "demo")]),
            "Created component <symbol id=\"demo\">"
        );
        assert_eq!(resolve_locale("en-US"), "en");
        assert_eq!(resolve_locale("English"), "en");
        assert_eq!(resolve_locale("unsupported"), "ja");
    }

    #[test]
    fn every_catalog_has_metadata_and_unique_locale_tags() {
        let locales = available_locales();
        assert!(locales.iter().any(|(id, _)| *id == "ja"));
        assert!(locales.iter().any(|(id, _)| *id == "en"));
        assert!(locales.iter().all(|(_, name)| !name.is_empty()));
    }

    #[test]
    fn locale_preference_is_open_ended_and_old_values_resolve() {
        let prefs = crate::core::prefs::Prefs {
            language: "pt-BR".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let loaded: crate::core::prefs::Prefs = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.language, "pt-BR");
        assert_eq!(resolve_locale("English"), "en");
        assert_eq!(resolve_locale("Japanese"), "ja");
        assert_eq!(crate::core::prefs::Prefs::default().language, "ja");
    }
}
