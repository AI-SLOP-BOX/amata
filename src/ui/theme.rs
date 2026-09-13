use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin, Stroke, Vec2,
    Visuals,
};
use std::sync::Arc;

/// Setup refined high-legibility UI typography: Inter (Latin, Digits, Symbols) + Noto Sans JP (CJK)
pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // 1. Primary Latin & Number Font: Inter
    let inter_candidates = [
        "assets/fonts/Inter.ttf",
        "/System/Library/Fonts/Supplemental/Inter.ttf",
        "/Library/Fonts/Inter.ttf",
    ];

    let mut inter_loaded = false;
    for path in inter_candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("inter".to_owned(), Arc::new(FontData::from_owned(bytes)));
            inter_loaded = true;
            log::info!("Loaded primary Latin UI font (Inter): {}", path);
            break;
        }
    }

    // 2. Primary Japanese Font: Noto Sans JP
    let mut jp_candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let home_p = std::path::PathBuf::from(home);
        jp_candidates.push(home_p.join("Library/Fonts/NotoSansJP-Medium.ttf"));
        jp_candidates.push(home_p.join("Library/Fonts/NotoSansJP-Regular.ttf"));
    }
    jp_candidates.push(std::path::PathBuf::from("/System/Library/Fonts/Hiragino Sans GB.ttc"));
    jp_candidates.push(std::path::PathBuf::from("/Library/Fonts/Arial Unicode.ttf"));

    let mut jp_loaded = false;
    for path in &jp_candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "noto_sans_jp".to_owned(),
                Arc::new(FontData::from_owned(bytes)),
            );
            jp_loaded = true;
            log::info!("Loaded Japanese UI font (Noto Sans JP): {}", path.display());
            break;
        }
    }

    // Assemble font cascade: Inter -> Noto Sans JP -> egui default fallbacks
    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        if jp_loaded {
            prop.insert(0, "noto_sans_jp".to_owned());
        }
        if inter_loaded {
            prop.insert(0, "inter".to_owned());
        }
    }

    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        if inter_loaded {
            mono.insert(0, "inter".to_owned());
        }
    }

    ctx.set_fonts(fonts);
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
