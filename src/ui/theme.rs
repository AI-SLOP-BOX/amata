use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin, Stroke, Vec2,
    Visuals,
};
use std::sync::Arc;

/// UI font bundle assembled without an egui context (testable).
pub struct UiFontSet {
    pub defs: FontDefinitions,
    /// Name of the Latin font at the head of the cascade (`None` if missing).
    pub latin: Option<String>,
    /// Name of the CJK font at the head of the cascade (`None` if missing).
    pub japanese: Option<String>,
    /// Non-fatal problems (logged as warnings by the caller).
    pub warnings: Vec<String>,
}

/// Build the UI font cascade: Inter (Latin) → Japanese CJK → egui defaults.
///
/// Every candidate is *verified* before use (parses as a font AND covers a
/// probe glyph). This matters because the old code accepted `.ttc`
/// collections, which egui/ab_glyph cannot parse — the file read succeeded
/// but every Japanese glyph came out as tofu on machines without a lucky
/// single-file CJK font. `.ttc` candidates are therefore skipped outright.
///
/// Priority: user/project files → bundled `assets/fonts` (guaranteed) →
/// egui embedded defaults (last resort, likely tofu for CJK).
pub fn build_ui_font_definitions() -> UiFontSet {
    let mut fonts = FontDefinitions::default();
    let mut warnings = Vec::new();

    // 1. Latin: project file first, bundled Inter as the guarantee.
    let inter_candidates = [
        "assets/fonts/Inter.ttf",
        "/System/Library/Fonts/Supplemental/Inter.ttf",
        "/Library/Fonts/Inter.ttf",
        "C:\\Windows\\Fonts\\Inter-Regular.ttf",
        "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
    ];
    let mut inter_bytes: Option<Vec<u8>> =
        inter_candidates.iter().find_map(|p| std::fs::read(p).ok());
    if inter_bytes.is_none() {
        inter_bytes = Some(include_bytes!("../../assets/fonts/Inter.ttf").to_vec());
    }
    let latin = match inter_bytes {
        Some(bytes) if font_covers(&bytes, 'A') => {
            fonts
                .font_data
                .insert("inter".to_owned(), Arc::new(FontData::from_owned(bytes)));
            log::info!("UI Latin font: Inter");
            Some("inter".to_owned())
        }
        _ => {
            warnings.push("Inter failed verification; Latin falls back to egui defaults".into());
            None
        }
    };

    // 2. Japanese CJK: user fonts first (LINE Seed JP, Noto), bundled
    // NotoSansJP-Regular as the guarantee. Single-file fonts only.
    let mut jp_candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let home_p = std::path::PathBuf::from(home);
        jp_candidates.push(home_p.join("Library/Fonts/LINESeedJP_OTF_Rg.otf"));
        jp_candidates.push(home_p.join("Library/Fonts/LINESeedJP_TTF_Rg.ttf"));
        jp_candidates.push(home_p.join("Library/Fonts/LINESeedJP-Regular.otf"));
        jp_candidates.push(home_p.join(".local/share/fonts/LINESeedJP_OTF_Rg.otf"));
        jp_candidates.push(home_p.join("Library/Fonts/NotoSansJP-Regular.ttf"));
        jp_candidates.push(home_p.join("Library/Fonts/NotoSansJP-Regular.otf"));
        jp_candidates.push(home_p.join(".local/share/fonts/NotoSansJP-Regular.ttf"));
        jp_candidates.push(home_p.join(".fonts/NotoSansJP-Regular.ttf"));
    }
    jp_candidates.push(std::path::PathBuf::from("assets/fonts/NotoSansJP-Regular.ttf"));
    jp_candidates.push(std::path::PathBuf::from("/Library/Fonts/LINESeedJP_OTF_Rg.otf"));
    jp_candidates.push(std::path::PathBuf::from("/usr/share/fonts/NotoSansJP-Regular.ttf"));

    let mut jp_bytes: Option<(Vec<u8>, String)> = jp_candidates.iter().find_map(|p| {
        // Collections need a face index egui can't address reliably;
        // skip instead of installing tofu.
        if p.extension().and_then(|e| e.to_str()) == Some("ttc") {
            return None;
        }
        let bytes = std::fs::read(p).ok()?;
        if !font_covers(&bytes, 'あ') {
            return None;
        }
        log::info!("UI Japanese font candidate: {}", p.display());
        Some((bytes, p.display().to_string()))
    });
    if jp_bytes.is_none() {
        let bytes = include_bytes!("../../assets/fonts/NotoSansJP-Regular.ttf").to_vec();
        if font_covers(&bytes, 'あ') {
            jp_bytes = Some((bytes, "bundled assets/fonts/NotoSansJP-Regular.ttf".into()));
        }
    }
    let japanese = match jp_bytes {
        Some((bytes, source)) => {
            fonts.font_data.insert(
                "jp_ui_font".to_owned(),
                Arc::new(FontData::from_owned(bytes)),
            );
            log::info!("UI Japanese font: {source}");
            Some("jp_ui_font".to_owned())
        }
        None => {
            warnings.push("No usable CJK font found; Japanese UI may show tofu".into());
            None
        }
    };

    // 3. Assemble the cascade, keeping egui's embedded fonts as last resort.
    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        if japanese.is_some() {
            prop.insert(0, "jp_ui_font".to_owned());
        }
        if latin.is_some() {
            prop.insert(0, "inter".to_owned());
        }
    }
    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        if japanese.is_some() {
            mono.insert(0, "jp_ui_font".to_owned());
        }
        if latin.is_some() {
            mono.insert(0, "inter".to_owned());
        }
    }

    UiFontSet {
        defs: fonts,
        latin,
        japanese,
        warnings,
    }
}

/// True when `bytes` parse as a single font containing `probe`.
fn font_covers(bytes: &[u8], probe: char) -> bool {
    match ttf_parser::Face::parse(bytes, 0) {
        Ok(face) => face.glyph_index(probe).is_some(),
        Err(_) => false,
    }
}

/// Setup refined high-legibility UI typography: Inter (Latin) + Japanese CJK.
/// Prioritized cascade: Inter → CJK → egui defaults. See
/// [`build_ui_font_definitions`].
pub fn setup_custom_fonts(ctx: &egui::Context) {
    let set = build_ui_font_definitions();
    for w in &set.warnings {
        log::warn!("{w}");
    }
    ctx.set_fonts(set.defs);
}

/// Apply refined Creative Cloud Charcoal Theme — precisely matched to Illustrator CC reference.
///
/// Palette:
///   #1e1e1e — panel fills (sidebars, top/bottom bars)
///   #262626 — modal/popup window fills
///   #323232 — widget idle bg
///   #3e3e3e — hover bg
///   #1473e6 — accent blue
pub fn apply_adobe_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // Base surfaces
    visuals.panel_fill = Color32::from_rgb(30, 30, 30); // #1e1e1e
    visuals.window_fill = Color32::from_rgb(38, 38, 38); // #262626
    visuals.faint_bg_color = Color32::from_rgb(26, 26, 26);
    visuals.extreme_bg_color = Color32::from_rgb(20, 20, 20);
    visuals.code_bg_color = Color32::from_rgb(18, 18, 18);

    // macOS-style window shadow
    visuals.window_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };

    // Selection / accent
    visuals.selection.bg_fill = Color32::from_rgb(20, 115, 230);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.hyperlink_color = Color32::from_rgb(64, 156, 255);

    visuals.window_corner_radius = CornerRadius::same(6);
    visuals.menu_corner_radius = CornerRadius::same(4);

    // Inactive
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(50, 50, 50);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(3);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(62, 62, 62));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(212, 212, 212));
    visuals.widgets.inactive.expansion = 0.0;

    // Hovered
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(62, 62, 62);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(3);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(88, 88, 88));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.expansion = 0.0;

    // Active (pressed)
    visuals.widgets.active.bg_fill = Color32::from_rgb(20, 115, 230);
    visuals.widgets.active.corner_radius = CornerRadius::same(3);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230));
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.expansion = 0.0;

    // Open (ComboBox / menu)
    visuals.widgets.open.bg_fill = Color32::from_rgb(42, 42, 42);
    visuals.widgets.open.corner_radius = CornerRadius::same(3);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230));
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    // Noninteractive (labels, static frames)
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(30, 30, 30);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(48, 48, 48));
    visuals.widgets.noninteractive.fg_stroke =
        Stroke::new(1.0_f32, Color32::from_rgb(175, 175, 175));

    visuals.window_stroke = Stroke::new(1.0_f32, Color32::from_rgb(52, 52, 52));

    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        // Tight professional density matching reference screenshots
        style.spacing.item_spacing = Vec2::new(4.0, 3.0);
        style.spacing.button_padding = Vec2::new(6.0, 3.0);
        style.spacing.window_margin = Margin::same(8);
        style.spacing.menu_margin = Margin::symmetric(6, 3);
        style.spacing.interact_size = Vec2::new(16.0, 18.0);
        style.spacing.indent = 14.0;
        style.spacing.scroll.bar_width = 6.0;
        style.spacing.combo_height = 220.0;

        use egui::{FontId, TextStyle};
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(13.0, FontFamily::Proportional),
        );
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(11.5, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(11.5, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(10.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(10.5, FontFamily::Monospace),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_cover_their_scripts() {
        let inter = include_bytes!("../../assets/fonts/Inter.ttf");
        assert!(font_covers(inter, 'A'), "Inter must cover Latin");
        assert!(!font_covers(inter, 'あ'), "Inter has no kana (fallback required)");
        let noto = include_bytes!("../../assets/fonts/NotoSansJP-Regular.ttf");
        assert!(font_covers(noto, 'A'), "Noto JP must cover Latin");
        assert!(font_covers(noto, 'あ'), "Noto JP must cover hiragana");
        assert!(font_covers(noto, '漢'), "Noto JP must cover ideographs");
    }

    #[test]
    fn cascade_orders_latin_before_cjk_before_defaults() {
        // Bundled fonts guarantee both slots regardless of machine fonts.
        let set = build_ui_font_definitions();
        assert_eq!(set.latin.as_deref(), Some("inter"));
        assert_eq!(set.japanese.as_deref(), Some("jp_ui_font"));
        let prop = &set.defs.families[&FontFamily::Proportional];
        assert!(prop.len() >= 3, "cascade keeps egui defaults: {prop:?}");
        assert_eq!(prop[0], "inter");
        assert_eq!(prop[1], "jp_ui_font");
        let mono = &set.defs.families[&FontFamily::Monospace];
        assert!(mono.contains(&"jp_ui_font".to_owned()), "mono needs CJK too: {mono:?}");
    }

    #[test]
    fn ttc_bytes_fail_verification() {
        // A collection is not a font: verification must reject it so we
        // never install tofu (regression guard for the old .ttc candidates).
        let fake_ttc = b"ttcf\x00\x01\x00\x00\x00\x00\x00\x02garbage";
        assert!(!font_covers(fake_ttc, 'あ'));
        assert!(!font_covers(b"not a font", 'A'));
    }
}
