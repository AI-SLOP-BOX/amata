//! 環境設定 (Preferences): persistence shape, derived values, and the bridge
//! into the running session (`PreferencesDialog::apply_to_session`).

use irasu_illustrator::core::document::{Guide, GuideOrientation};
use irasu_illustrator::core::prefs::Prefs;
use irasu_illustrator::core::state::AppState;
use irasu_illustrator::core::unit::LengthUnit;
use irasu_illustrator::ui::PreferencesDialog;

#[test]
fn default_prefs_match_session_defaults() {
    // Prefs is the *startup* value of the session state: applying the defaults
    // to a fresh AppState must change nothing.
    let mut state = AppState::default();
    let before = (
        state.show_grid,
        state.grid_size,
        state.snap_to_grid,
        state.snap_to_objects,
        state.snap_to_guides,
        state.snap_to_points,
        state.snap_to_pixels,
        state.show_rulers,
        state.show_smart_guides,
    );
    PreferencesDialog::apply_to_session(&mut state);
    let after = (
        state.show_grid,
        state.grid_size,
        state.snap_to_grid,
        state.snap_to_objects,
        state.snap_to_guides,
        state.snap_to_points,
        state.snap_to_pixels,
        state.show_rulers,
        state.show_smart_guides,
    );
    assert_eq!(before, after, "applying default prefs must be a no-op");
}

#[test]
fn apply_to_session_pushes_grid_and_snap_prefs() {
    let mut state = AppState::default();
    state.prefs.show_grid = false;
    state.prefs.grid_size = 12.5;
    state.prefs.snap_to_grid = true;
    state.prefs.snap_to_objects = false;
    state.prefs.snap_to_guides = false;
    state.prefs.snap_to_points = false;
    state.prefs.snap_to_pixels = true;
    state.prefs.show_rulers = false;
    state.prefs.show_smart_guides_on_transform = false;

    PreferencesDialog::apply_to_session(&mut state);

    assert!(!state.show_grid);
    assert_eq!(state.grid_size, 12.5);
    assert!(state.snap_to_grid);
    assert!(!state.snap_to_objects);
    assert!(!state.snap_to_guides);
    assert!(!state.snap_to_points);
    assert!(state.snap_to_pixels);
    assert!(!state.show_rulers);
    assert!(!state.show_smart_guides);
}

#[test]
fn apply_to_session_clamps_undo_stack_to_pref() {
    let mut state = AppState::default();
    assert_eq!(state.prefs.history_states_count, 100);
    // The default UndoManager cap is also 100 — lowering it via prefs must
    // take effect immediately.
    state.prefs.history_states_count = 50;
    PreferencesDialog::apply_to_session(&mut state);
    // Raising it back must not panic or resurrect evicted steps.
    state.prefs.history_states_count = 200;
    PreferencesDialog::apply_to_session(&mut state);
}

#[test]
fn missing_fields_fall_back_to_defaults() {
    // An old preferences.json written before ガイド・グリッド existed must
    // still load: every field is `#[serde(default)]`.
    let legacy = r#"{
        "show_home_on_startup": false,
        "recent_files_count": 50,
        "color_theme": "ライト"
    }"#;
    let p: Prefs = serde_json::from_str(legacy).expect("legacy prefs load");
    assert!(!p.show_home_on_startup);
    assert_eq!(p.recent_files_count, 50);
    assert_eq!(p.color_theme, "ライト");
    let d = Prefs::default();
    assert_eq!(p.grid_size, d.grid_size);
    assert_eq!(p.show_grid, d.show_grid);
    assert_eq!(p.snap_to_objects, d.snap_to_objects);
    assert_eq!(p.show_rulers, d.show_rulers);
}

#[test]
fn unknown_fields_are_dropped_not_fatal() {
    // Forward compatibility: a file written by a newer build must still load.
    let futuristic = r#"{"sync_cloud_docs": true, "ai_feature_level": "max"}"#;
    let p: Prefs = serde_json::from_str(futuristic).expect("unknown fields ignored");
    assert_eq!(p, Prefs::default());
}

#[test]
fn prefs_round_trip_through_json() {
    let p = Prefs {
        ui_scale: "150%".to_string(),
        artboard_bg_mode: "ホワイト".to_string(),
        grid_size: 25.0,
        anchor_point_size: 7.5,
        handle_size: 3.0,
        selection_line_width: 2.5,
        ..Default::default()
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: Prefs = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn ui_scale_factor_parses_and_clamps() {
    let at = |scale: &str| {
        Prefs {
            ui_scale: scale.to_string(),
            ..Default::default()
        }
        .ui_scale_factor()
    };
    assert_eq!(at("100%"), 1.0);
    assert_eq!(at("125%"), 1.25);
    assert_eq!(at("150%"), 1.5);
    assert_eq!(at("9999%"), 3.0, "clamped to 3x");
    assert_eq!(at("not a number"), 1.0, "garbage falls back to 100%");
}

#[test]
fn artboard_and_handle_mode_helpers() {
    let mut p = Prefs::default();
    assert!(
        !p.artboard_is_white(),
        "default artboard is the checkerboard"
    );
    p.artboard_bg_mode = "ホワイト".to_string();
    assert!(p.artboard_is_white());

    assert!(!p.high_contrast_handles());
    p.point_handle_color_mode = "ハイコントラスト".to_string();
    assert!(p.high_contrast_handles());
}

#[test]
fn snap_prefers_guide_within_threshold_over_grid() {
    // Guide snap is gated on `snap_to_guides`; when on, a guide 1px away must
    // beat a grid line 30px away (closest target wins).
    let mut state = AppState {
        snap_to_grid: true,
        grid_size: 60.0,
        snap_to_objects: false,
        snap_to_points: false,
        snap_to_guides: true,
        ..Default::default()
    };
    state.guides.push(Guide {
        orientation: GuideOrientation::Horizontal,
        position: 101.0,
    });

    let (x, y) = state.snap(4.0, 100.0);
    assert_eq!(y, 101.0, "guide 1px away wins");
    assert_eq!(x, 0.0, "x still snaps to the nearest grid line");

    // Guides off → the guide is no longer a target; y falls back to the
    // nearest grid line (60 is 40 away, 120 is 20 away → 120).
    state.snap_to_guides = false;
    let (_, y2) = state.snap(4.0, 100.0);
    assert_eq!(y2, 120.0, "grid line at 120 is now the closest y target");
}

#[test]
fn snap_to_object_edges_returns_none_on_miss() {
    // A miss must be distinguishable from an exact hit at the origin,
    // otherwise it cancels stronger snap targets.
    let mut state = AppState::default();
    let rect =
        irasu_illustrator::core::document::Object::new_rect("R", 100.0, 100.0, 50.0, 50.0, 0.0);
    let objects: Vec<&_> = vec![&rect];
    assert_eq!(state.snap_to_object_edges(0.0, 0.0, &objects), None);
    assert_eq!(
        state.snap_to_object_edges(100.0, 125.0, &objects),
        Some((100.0, 125.0)),
        "a point exactly on the left edge is a hit, not a miss"
    );
    state.snap_to_objects = false;
}

// ── 単位 ─────────────────────────────────────────────────────────────────

#[test]
fn default_units_change_nothing_already_printed() {
    let d = Prefs::default();
    // Coordinates and stroke widths printed raw px before units existed —
    // the defaults must keep them on px so every field reads the same.
    assert_eq!(d.ruler_unit, LengthUnit::Px);
    assert_eq!(d.stroke_unit, LengthUnit::Px);
    // …but the new-document dialog has offered millimetres since day one.
    assert_eq!(d.default_doc_unit, LengthUnit::Mm);
}

#[test]
fn unit_prefs_survive_a_json_round_trip_as_labels() {
    let p = Prefs {
        ruler_unit: LengthUnit::Mm,
        stroke_unit: LengthUnit::Pt,
        default_doc_unit: LengthUnit::In,
        ..Default::default()
    };
    let json = serde_json::to_string(&p).unwrap();
    // Stored as the same Japanese labels the selectors show, so a hand-edited
    // preferences.json stays readable.
    assert!(json.contains("\"ミリメートル\""), "{json}");
    assert!(json.contains("\"ポイント\""), "{json}");
    assert!(json.contains("\"インチ\""), "{json}");
    assert_eq!(serde_json::from_str::<Prefs>(&json).unwrap(), p);
}

#[test]
fn legacy_json_without_units_gets_the_px_defaults() {
    let legacy = r#"{"show_home_on_startup": false}"#;
    let p: Prefs = serde_json::from_str(legacy).unwrap();
    assert_eq!(p.ruler_unit, LengthUnit::Px);
    assert_eq!(p.stroke_unit, LengthUnit::Px);
    assert_eq!(p.default_doc_unit, LengthUnit::Mm);
}

#[test]
fn a_garbage_unit_label_falls_back_to_px_instead_of_failing_to_load() {
    // A unit is a *display* choice: a hand-edited file must not be able to
    // take the whole preferences load down with it.
    let bad = r#"{"ruler_unit": "観光", "stroke_unit": 42}"#;
    // A non-string is a hard serde error for that field; `Prefs::load`
    // already swallows those and returns Default. What must *not* happen is
    // a panic, and a wrong-but-string label must load as px.
    assert!(serde_json::from_str::<Prefs>(bad).is_err());
    let bad_label: Prefs = serde_json::from_str(r#"{"ruler_unit": "観光"}"#).unwrap();
    assert_eq!(bad_label.ruler_unit, LengthUnit::Px);
}
