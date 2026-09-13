use irasu_illustrator::cli::{run_cli, Cli, Commands};
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::core::effects::{
    AppearanceStack, BlurEffect, ColorAdjustEffect, DropShadow, GlowEffect, VectorEffect,
};

#[test]
fn test_appearance_stack_composition() {
    let mut stack = AppearanceStack::new();
    assert!(stack.is_empty());

    // 1. Add Drop Shadow
    stack.push(VectorEffect::DropShadow(DropShadow {
        offset_x: 10.0,
        offset_y: 10.0,
        blur_radius: 12.0,
        color: [0.0, 0.0, 0.0, 1.0],
        opacity: 0.6,
    }));
    assert!(stack.has_shadow().is_some());
    assert_eq!(stack.has_shadow().unwrap().offset_x, 10.0);

    // 2. Add Glow
    stack.push(VectorEffect::Glow(GlowEffect {
        radius: 20.0,
        color: [0.0, 1.0, 0.5, 1.0],
        intensity: 0.8,
    }));
    assert!(stack.has_glow().is_some());

    // 3. Add Blur
    stack.push(VectorEffect::Blur(BlurEffect { radius: 8.0 }));
    assert_eq!(stack.has_blur(), Some(8.0));

    // 4. Add Color Adjust
    stack.push(VectorEffect::ColorAdjust(ColorAdjustEffect {
        brightness: 0.1,
        contrast: 1.2,
        saturation: 1.5,
        hue_rotate: 45.0,
    }));
    assert!(stack.has_color_adjust().is_some());
    assert_eq!(stack.effects.len(), 4);
}

#[test]
fn test_svg_export_with_appearance_filters() {
    let mut doc = Document::default();
    let mut rect = Object::new_rect("CardWithEffects", 50.0, 50.0, 200.0, 150.0, 8.0);
    rect.shadow = Some(DropShadow {
        offset_x: 6.0,
        offset_y: 6.0,
        blur_radius: 10.0,
        color: [0.0, 0.0, 0.0, 1.0],
        opacity: 0.5,
    });
    rect.glow = Some(GlowEffect {
        radius: 15.0,
        color: [1.0, 0.2, 0.8, 1.0],
        intensity: 0.7,
    });
    doc.add_object(rect);

    let svg = irasu_illustrator::io::svg::export_svg(&doc);

    // Verify SVG Filter elements are properly output
    assert!(svg.contains("<defs>"));
    assert!(svg.contains("<filter id=\"filter_1\""));
    assert!(svg.contains("<feDropShadow"));
    assert!(svg.contains("<feGaussianBlur"));
    assert!(svg.contains("<feFlood"));
    assert!(svg.contains("filter=\"url(#filter_1)\""));
}

#[test]
fn test_cli_phase12_effect_subcommand() {
    let temp_dir = std::env::temp_dir();
    let in_svg = temp_dir.join("p12_input.svg");
    let out_svg = temp_dir.join("p12_effects.svg");

    let mut doc = Document::default();
    doc.add_object(Object::new_rect("Button", 100.0, 100.0, 160.0, 60.0, 10.0));
    let content = irasu_illustrator::io::svg::export_svg(&doc);
    std::fs::write(&in_svg, &content).unwrap();

    let cli = Cli {
        command: Some(Commands::Effect {
            input: in_svg.clone(),
            shadow: true,
            shadow_x: 12.0,
            shadow_y: 12.0,
            shadow_blur: 16.0,
            glow: true,
            glow_radius: 20.0,
            output: out_svg.clone(),
        }),
    };
    assert!(run_cli(cli).is_ok());
    assert!(out_svg.exists());

    let result_svg = std::fs::read_to_string(&out_svg).unwrap();
    assert!(result_svg.contains("<filter id=\"filter_1\""));
    assert!(result_svg.contains("feDropShadow dx=\"12\" dy=\"12\" stdDeviation=\"16\""));

    let _ = std::fs::remove_file(in_svg);
    let _ = std::fs::remove_file(out_svg);
}
