//! Display units for lengths (定規, 座標, 測定, ストローク).
//!
//! A document's geometry is stored in **pixels at 96 dpi** — the same base
//! unit [`NewDocModal`](crate::ui::NewDocModal) converts from when a preset
//! says *210 × 297 mm*. This module is that conversion table in one place:
//! the UI converts to the user's chosen unit on the way out and back to px on
//! the way in, so what is stored on disk never changes.

use serde::{Deserialize, Serialize};

/// A length display unit. Round-trips through `preferences.json` as its
/// Japanese label (the same strings `NewDocModal` uses for its selector).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LengthUnit {
    #[default]
    Px,
    Mm,
    Pt,
    In,
}

impl LengthUnit {
    /// Every unit, in the order the selectors list them.
    pub const ALL: [LengthUnit; 4] = [Self::Px, Self::Mm, Self::Pt, Self::In];

    /// Document px in one of this unit — the factor `dims_to_px` multiplies
    /// by. `96 dpi` base: 1 in = 96 px, 1 pt = 96/72 px, 1 mm = 96/25.4 px.
    pub fn px_per_unit(self) -> f64 {
        match self {
            Self::Px => 1.0,
            Self::Mm => 96.0 / 25.4,
            Self::Pt => 96.0 / 72.0,
            Self::In => 96.0,
        }
    }

    /// Short suffix shown next to numeric fields (`px`, `mm`, `pt`, `in`).
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Px => "px",
            Self::Mm => "mm",
            Self::Pt => "pt",
            Self::In => "in",
        }
    }

    /// Label used by the selectors and stored in `preferences.json`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Px => "ピクセル",
            Self::Mm => "ミリメートル",
            Self::Pt => "ポイント",
            Self::In => "インチ",
        }
    }

    /// Accept a stored/selected label, falling back to [`Self::default`]
    /// (px) so a hand-edited or corrupt `preferences.json` cannot break the
    /// UI.
    pub fn parse(label: &str) -> Self {
        match label {
            "ミリメートル" => Self::Mm,
            "ポイント" => Self::Pt,
            "インチ" => Self::In,
            _ => Self::Px,
        }
    }

    /// Document px → this unit.
    ///
    /// Clippy's `from_*` rule assumes the *target* of a conversion; here the
    /// unit itself is the input, so `self` is exactly what's wanted.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_px(self, px: f64) -> f64 {
        px / self.px_per_unit()
    }

    /// This unit → document px.
    pub fn to_px(self, value: f64) -> f64 {
        value * self.px_per_unit()
    }

    /// Round to the nearest `1 / 2 / 5 × 10ⁿ` step *in this unit*, so ruler
    /// labels land on readable numbers (5, 10, 20, 50 mm …) while the tick
    /// density stays within ~1.6× of the rough spacing the zoom ladder asked
    /// for. Exact for the steps the px ruler already used (20, 50, 100,
    /// 500), so the default unit keeps the look it has today.
    pub fn nice_step(self, rough_px: f64) -> f64 {
        let in_unit = self.from_px(rough_px);
        if !in_unit.is_finite() || in_unit <= 0.0 {
            return self.to_px(1.0);
        }
        // The 1-2-5 ladder spanning one rung below and above `in_unit`, so
        // the nearest rung always exists. Compared in log space: the rungs
        // are geometric, and this bounds the result to within 10^(398/2000)
        // ≈ 1.58× either way.
        let base = 10f64.powf(in_unit.log10().floor());
        let log_v = in_unit.log10();
        let step_unit = [0.5, 1.0, 2.0, 5.0, 10.0]
            .iter()
            .map(|m| m * base)
            .min_by(|a, b| {
                (a.log10() - log_v)
                    .abs()
                    .partial_cmp(&(b.log10() - log_v).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(in_unit);
        self.to_px(step_unit)
    }

    /// Convert document px for display: whole numbers without a decimal tail,
    /// one decimal place otherwise (matches the measurement badges).
    pub fn format(self, px: f64) -> String {
        let v = self.from_px(px);
        if (v - v.round()).abs() < 0.05 {
            format!("{}", v.round() as i64)
        } else {
            format!("{v:.1}")
        }
    }

    /// The `" mm"`-style suffix for a numeric field (leading space included).
    pub fn suffix_label(self) -> String {
        format!(" {}", self.suffix())
    }

    /// Decimal places a coordinate/size field should keep: one is plenty for
    /// px (half a pixel is already a visible step) while inch values need
    /// two before they stop snapping (0.1 in ≈ 9.6 px).
    pub fn field_decimals(self) -> usize {
        match self {
            Self::In => 2,
            _ => 1,
        }
    }
}

/// Serde helpers so `Prefs` can store a `LengthUnit` directly while keeping
/// the on-disk shape (a Japanese label) unchanged from the other selectors.
impl Serialize for LengthUnit {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.label())
    }
}

impl<'de> Deserialize<'de> for LengthUnit {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Ok(Self::parse(&raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_matches_new_doc_modal_table() {
        // The factors `NewDocModal::dims_to_px` has always used.
        assert_eq!(LengthUnit::Px.px_per_unit(), 1.0);
        assert!((LengthUnit::Mm.px_per_unit() - 96.0 / 25.4).abs() < 1e-12);
        assert!((LengthUnit::Pt.px_per_unit() - 96.0 / 72.0).abs() < 1e-12);
        assert_eq!(LengthUnit::In.px_per_unit(), 96.0);
    }

    #[test]
    fn a4_preset_is_794px_wide() {
        // 210 mm at 96 dpi — what the A4 preset creates today.
        let px = LengthUnit::Mm.to_px(210.0);
        assert!((px - 793.7).abs() < 0.1, "got {px}");
    }

    #[test]
    fn round_trip_is_lossless() {
        for unit in LengthUnit::ALL {
            for px in [0.0, 1.0, 37.5, 1000.0, -256.0] {
                let back = unit.to_px(unit.from_px(px));
                assert!((back - px).abs() < 1e-9, "{unit:?}: {px} -> {back}");
            }
        }
    }

    #[test]
    fn parse_is_total_and_falls_back_to_px() {
        assert_eq!(LengthUnit::parse("ミリメートル"), LengthUnit::Mm);
        assert_eq!(LengthUnit::parse("ポイント"), LengthUnit::Pt);
        assert_eq!(LengthUnit::parse("インチ"), LengthUnit::In);
        assert_eq!(LengthUnit::parse("ピクセル"), LengthUnit::Px);
        assert_eq!(LengthUnit::parse(""), LengthUnit::Px);
        assert_eq!(LengthUnit::parse("観光"), LengthUnit::Px);
    }

    #[test]
    fn parse_sees_every_label() {
        for unit in LengthUnit::ALL {
            assert_eq!(LengthUnit::parse(unit.label()), unit);
        }
    }

    #[test]
    fn nice_step_keeps_the_px_rulers_current_steps() {
        // The px ruler has always stepped 20 / 50 / 100 / 500 at these
        // zooms — `nice_step` must be the identity on them.
        for px in [20.0, 50.0, 100.0, 500.0] {
            assert_eq!(LengthUnit::Px.nice_step(px), px, "{px}");
        }
    }

    #[test]
    fn nice_step_picks_the_nearest_readable_mm() {
        let mm = LengthUnit::Mm;
        // A rough px step from the zoom ladder → the closest round mm rung.
        assert_eq!(mm.nice_step(mm.to_px(3.0)), mm.to_px(2.0));
        assert_eq!(mm.nice_step(mm.to_px(7.0)), mm.to_px(5.0));
        assert_eq!(mm.nice_step(mm.to_px(30.0)), mm.to_px(20.0));
        assert_eq!(mm.nice_step(mm.to_px(60.0)), mm.to_px(50.0));
    }

    #[test]
    fn nice_step_stays_close_to_the_requested_spacing_in_every_unit() {
        // The ruler must not suddenly double or halve its tick density just
        // because the unit changed: the picked rung is bounded to within
        // ~1.6× of whatever the zoom ladder asked for.
        for unit in LengthUnit::ALL {
            for rough in [20.0, 50.0, 100.0, 500.0] {
                let got = unit.nice_step(rough);
                let ratio = got / rough;
                assert!(
                    (0.6..=1.6).contains(&ratio),
                    "{unit:?}: rough {rough} px -> {got} px (×{ratio:.2})"
                );
            }
        }
    }

    #[test]
    fn nice_step_survives_degenerate_input() {
        for unit in LengthUnit::ALL {
            for v in [0.0, -1.0, f64::NAN] {
                let s = unit.nice_step(v);
                assert!(s > 0.0 && s.is_finite(), "{unit:?} {v} -> {s}");
            }
        }
    }

    #[test]
    fn format_drops_whole_number_decimals() {
        let mm = LengthUnit::Mm;
        assert_eq!(mm.format(mm.to_px(210.0)), "210");
        assert_eq!(mm.format(mm.to_px(0.5)), "0.5");
        assert_eq!(LengthUnit::Px.format(64.0), "64");
        assert_eq!(LengthUnit::Px.format(64.4), "64.4");
    }

    #[test]
    fn serde_keeps_the_japanese_label_on_disk() {
        let json = serde_json::to_string(&LengthUnit::Mm).unwrap();
        assert_eq!(json, "\"ミリメートル\"");
        let back: LengthUnit = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LengthUnit::Mm);
        // A corrupt value falls back to px rather than failing to load.
        let fallback: LengthUnit = serde_json::from_str("\"ノンユニット\"").unwrap();
        assert_eq!(fallback, LengthUnit::Px);
    }
}
