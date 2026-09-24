//! User preferences — the single source of truth behind the 環境設定 dialog.
//!
//! The dialog edits [`AppState::prefs`](super::state::AppState::prefs) directly
//! (no mirrored form fields), the renderer reads the values while drawing, and
//! [`IrasuApp`](crate::app::IrasuApp) applies the startup-only ones once at
//! launch. Persisted as JSON in the user's config directory.

use super::unit::LengthUnit;
use serde::{Deserialize, Serialize};

/// User-facing preferences (Illustrator-style *Preferences ▸ General*).
///
/// `#[serde(default)]` keeps old `preferences.json` files loadable: unknown
/// fields are dropped and missing ones fall back to [`Default`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    // ── 一般: startup & files ────────────────────────────────────────────
    /// Show the home screen on launch.
    pub show_home_on_startup: bool,
    /// Reopen the most recent document when launching without a file.
    /// Only consulted when [`show_home_on_startup`](Self::show_home_on_startup)
    /// is off (the home screen wins otherwise).
    pub open_last_doc: bool,
    /// Show the new-document dialog; when off, *新規* creates a document from
    /// the current (default or preset) settings straight away.
    pub show_new_doc_dialog: bool,
    /// How many entries to keep in the recent-files list.
    pub recent_files_count: usize,
    /// Undo stack depth (取り消しの回数).
    pub history_states_count: usize,

    // ── 一般: selection & display ────────────────────────────────────────
    /// Draw the selection bounding box and its resize/rotate handles.
    pub show_bounding_box: bool,
    /// Draw anchor points (node edit).
    pub show_anchor_points: bool,
    /// Show tool hints (tool tooltips, menu access-key hints).
    pub show_tool_hints: bool,
    /// Default for `AppState::show_smart_guides` — that flag is session state
    /// toggled live by *View ▸ Smart Guides*, the panels and `U`; this is only
    /// its startup value and its control inside the dialog.
    pub show_smart_guides_on_transform: bool,
    /// Anchor point size in px (node edit).
    pub anchor_point_size: f32,
    /// Bounding-box resize handle size in px.
    pub handle_size: f32,
    /// Selection outline width in px.
    pub selection_line_width: f32,
    /// `point_handle_color_mode`: `"デフォルト"` or `"ハイコントラスト"`.
    pub point_handle_color_mode: String,
    /// `color_theme`: `"ダーク"`, `"ミディアムダーク"` or `"ライト"`.
    pub color_theme: String,
    /// `ui_scale`: `"100%"`, `"125%"` or `"150%"`.
    pub ui_scale: String,
    /// `artboard_bg_mode`: `"ホワイト"` or `"透明グリッド"`.
    pub artboard_bg_mode: String,
    /// Draw the outline around each artboard.
    pub show_boundary_lines: bool,
    /// Show `name (W × H px)` on the artboard header.
    pub show_dimension_labels: bool,

    // ── ガイド・グリッド ──────────────────────────────────────────────────
    ///
    /// The grid/snap/ruler toggles are *session* state (`AppState::show_grid`
    /// and friends, changed live by *View* and the panels). These fields are
    /// their startup default and their control inside the dialog: the dialog
    /// mirrors session → prefs while it is open and pushes prefs → session
    /// on every frame and again when it closes (see
    /// [`PreferencesDialog`](crate::ui::PreferencesDialog)).
    pub show_grid: bool,
    /// Grid spacing in document units (`AppState::grid_size`).
    pub grid_size: f64,
    pub snap_to_grid: bool,
    pub snap_to_objects: bool,
    pub snap_to_guides: bool,
    pub snap_to_points: bool,
    pub snap_to_pixels: bool,
    pub show_rulers: bool,

    // ── 単位 ─────────────────────────────────────────────────────────────
    ///
    /// Geometry is always *stored* in document px at 96 dpi —
    /// `crate::core::unit::LengthUnit` is that conversion table. These fields
    /// only pick what the ruler labels, the artboard dimension tag and the
    /// numeric fields *show*; changing one re-reads the same numbers on the
    /// next frame, so nothing on disk moves.
    ///
    /// Defaults are px, which is what every field printed before units were
    /// configurable — including the two that were printing the wrong unit
    /// over a px value (the property panel's hardcoded ` mm`, the stroke
    /// fields' ` pt`). `default_doc_unit` is the one exception: the
    /// new-document dialog has offered millimetres since day one.
    ///
    /// Unit for rulers, coordinates, sizes and the artboard dimension label.
    pub ruler_unit: LengthUnit,
    /// Unit for stroke widths.
    pub stroke_unit: LengthUnit,
    /// Unit preselected in the new-document dialog.
    pub default_doc_unit: LengthUnit,

    // ── 通知 ─────────────────────────────────────────────────────────────
    /// Report skipped constructs when importing a (PDF) file.
    pub notify_file_compat: bool,
    /// Report UI font substitutions at startup.
    pub notify_font_substitute: bool,
    /// Report plugins as they are registered.
    pub notify_plugin_load: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            show_home_on_startup: true,
            open_last_doc: true,
            show_new_doc_dialog: true,
            recent_files_count: 20,
            history_states_count: 100,
            show_bounding_box: true,
            show_anchor_points: true,
            show_tool_hints: true,
            show_smart_guides_on_transform: true,
            // Sizes match the values the renderer used before they were
            // configurable, so enabling the settings changes nothing.
            anchor_point_size: 6.0,
            handle_size: 8.0,
            selection_line_width: 1.0,
            point_handle_color_mode: "デフォルト".to_string(),
            color_theme: "ダーク".to_string(),
            ui_scale: "100%".to_string(),
            // Matches the checkerboard artboards have always been drawn with.
            artboard_bg_mode: "透明グリッド".to_string(),
            show_boundary_lines: true,
            show_dimension_labels: true,
            // Defaults mirror `AppState`'s so applying them at startup
            // changes nothing.
            show_grid: true,
            grid_size: 50.0,
            snap_to_grid: false,
            snap_to_objects: true,
            snap_to_guides: true,
            snap_to_points: true,
            snap_to_pixels: false,
            show_rulers: true,
            // Document px: identical to what every field printed before
            // units were configurable.
            ruler_unit: LengthUnit::Px,
            stroke_unit: LengthUnit::Px,
            // Matches `NewDocModal`'s long-standing default.
            default_doc_unit: LengthUnit::Mm,
            notify_file_compat: true,
            notify_font_substitute: true,
            notify_plugin_load: true,
        }
    }
}

impl Prefs {
    fn path() -> Option<std::path::PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA")
                .ok()
                .map(std::path::PathBuf::from)
                .map(|p| p.join("Amata"))
                .or_else(|| {
                    std::env::var("USERPROFILE")
                        .ok()
                        .map(|h| std::path::PathBuf::from(h).join(".amata"))
                })
                .map(|d| d.join("preferences.json"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| {
                    std::path::PathBuf::from(h)
                        .join(".config")
                        .join("amata")
                        .join("preferences.json")
                })
        }
    }

    /// Read the saved preferences, falling back to [`Default`] when the file
    /// is missing or unreadable (a corrupt file must not block startup).
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, json);
            }
        }
    }

    /// Multiplier for [`ui_scale`](Self::ui_scale) (`"125%"` → `1.25`).
    pub fn ui_scale_factor(&self) -> f32 {
        self.ui_scale
            .trim_end_matches('%')
            .trim()
            .parse::<f32>()
            .map(|pct| (pct / 100.0).clamp(0.5, 3.0))
            .unwrap_or(1.0)
    }

    /// True when artboards are painted opaque white instead of the
    /// transparency checkerboard.
    pub fn artboard_is_white(&self) -> bool {
        self.artboard_bg_mode == "ホワイト"
    }

    /// True when points and handles use the high-contrast palette.
    pub fn high_contrast_handles(&self) -> bool {
        self.point_handle_color_mode == "ハイコントラスト"
    }
}
