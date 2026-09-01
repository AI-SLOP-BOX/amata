use eframe::egui::{self, Color32, CornerRadius, Margin, Stroke, Vec2, Visuals};

/// Apply the iconic Adobe CC Charcoal Dark Theme to egui
pub fn apply_adobe_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // Adobe Creative Cloud Charcoal Palette
    visuals.panel_fill = Color32::from_rgb(50, 50, 50);
    visuals.window_fill = Color32::from_rgb(43, 43, 43);
    visuals.faint_bg_color = Color32::from_rgb(36, 36, 36);
    visuals.extreme_bg_color = Color32::from_rgb(26, 26, 26);
    visuals.code_bg_color = Color32::from_rgb(22, 22, 22);

    // Adobe Classic Selection Blue
    visuals.selection.bg_fill = Color32::from_rgb(20, 115, 230);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.hyperlink_color = Color32::from_rgb(38, 128, 235);

    // Sharp modern 2.0px corner rounding
    visuals.window_corner_radius = CornerRadius::same(3);
    visuals.menu_corner_radius = CornerRadius::same(2);

    // Widgets Styling
    let active_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 115, 230));
    let default_stroke = Stroke::new(1.0_f32, Color32::from_rgb(68, 68, 68));

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(58, 58, 58);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(2);
    visuals.widgets.inactive.bg_stroke = default_stroke;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(225, 225, 225));

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(72, 72, 72);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(2);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(90, 90, 90));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    visuals.widgets.active.bg_fill = Color32::from_rgb(20, 115, 230);
    visuals.widgets.active.corner_radius = CornerRadius::same(2);
    visuals.widgets.active.bg_stroke = active_stroke;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    visuals.widgets.open.bg_fill = Color32::from_rgb(45, 45, 45);
    visuals.widgets.open.corner_radius = CornerRadius::same(2);

    ctx.set_visuals(visuals);

    // Style Spacing
    ctx.style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(6.0, 5.0);
        style.spacing.button_padding = Vec2::new(8.0, 4.0);
        style.spacing.window_margin = Margin::same(6);
    });
}
