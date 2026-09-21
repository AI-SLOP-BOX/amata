use irasu_illustrator::core::document::{
    Document, FontStyle, Object, ObjectType, TextAnchor, TextStyle,
};
use irasu_illustrator::core::font::FontRegistry;
use irasu_illustrator::core::text_path::text_to_outline_path_with_style;
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};

#[test]
fn test_font_model_backward_compatibility() {
    // Simulate legacy JSON without 'style' field
    let legacy_json = r#"{
        "Text": {
            "text": "Legacy Header",
            "font_size": 36.0
        }
    }"#;

    let parsed: ObjectType = serde_json::from_str(legacy_json).expect("Should parse legacy JSON");
    if let ObjectType::Text {
        text,
        font_size,
        style,
        ..
    } = parsed
    {
        assert_eq!(text, "Legacy Header");
        assert_eq!(font_size, 36.0);
        // Default style safely populated
        assert_eq!(style.font_family, "Inter, sans-serif");
        assert_eq!(style.font_weight, 400);
        assert_eq!(style.font_style, FontStyle::Normal);
        assert_eq!(style.letter_spacing, 0.0);
        assert_eq!(style.text_anchor, TextAnchor::Start);
    } else {
        panic!("Parsed object should be ObjectType::Text");
    }
}

#[test]
fn test_svg_import_export_font_attributes_round_trip() {
    let svg_input = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600">
  <text x="100" y="200" font-size="48" font-family="Inter, Helvetica, sans-serif" font-weight="700" font-style="italic" letter-spacing="2.5" text-anchor="middle" fill="#FF5500">Amata Typography</text>
</svg>"##;

    let doc = parse_svg_document(svg_input);
    let objects: Vec<_> = doc.all_objects().collect();
    assert_eq!(objects.len(), 1);

    let (_, obj) = &objects[0];
    if let ObjectType::Text {
        text,
        font_size,
        style,
        ..
    } = &obj.object_type
    {
        assert_eq!(text, "Amata Typography");
        assert_eq!(*font_size, 48.0);
        assert_eq!(style.font_family, "Inter, Helvetica, sans-serif");
        assert_eq!(style.font_weight, 700);
        assert_eq!(style.font_style, FontStyle::Italic);
        assert_eq!(style.letter_spacing, 2.5);
        assert_eq!(style.text_anchor, TextAnchor::Middle);
    } else {
        panic!("Parsed object should be ObjectType::Text");
    }

    // Export back to SVG and verify preservation of font attributes
    let svg_output = export_svg(&doc);
    assert!(
        svg_output.contains("font-family=\"Inter, Helvetica, sans-serif\""),
        "SVG output must contain original font-family"
    );
    assert!(
        svg_output.contains("font-weight=\"700\""),
        "SVG output must contain font-weight=700"
    );
    assert!(
        svg_output.contains("font-style=\"italic\""),
        "SVG output must contain font-style=italic"
    );
    assert!(
        svg_output.contains("letter-spacing=\"2.5\""),
        "SVG output must contain letter-spacing=2.5"
    );
    assert!(
        svg_output.contains("text-anchor=\"middle\""),
        "SVG output must contain text-anchor=middle"
    );
    assert!(
        !svg_output.contains("system-ui, -apple-system, sans-serif"),
        "SVG output must NOT force system-ui"
    );
}

#[test]
fn test_missing_font_handling() {
    let svg_input = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 600">
  <text x="50" y="100" font-size="28" font-family="DefinitelyMissingFont12345" fill="#333333">Missing Font Test</text>
</svg>"##;

    // Must not panic on import
    let doc = parse_svg_document(svg_input);
    let objects: Vec<_> = doc.all_objects().collect();
    let (_, obj) = &objects[0];

    if let ObjectType::Text { style, .. } = &obj.object_type {
        // Document must preserve the requested font name, NOT overwrite it
        assert_eq!(style.font_family, "DefinitelyMissingFont12345");

        // Registry should identify it as missing
        let registry = FontRegistry::global();
        assert!(
            !registry.is_family_available(&style.font_family),
            "Non-existent font should be flagged as unavailable"
        );
    } else {
        panic!("Expected Text object");
    }

    // Export must still preserve the missing font name
    let svg_output = export_svg(&doc);
    assert!(
        svg_output.contains("font-family=\"DefinitelyMissingFont12345\""),
        "Export must keep missing font name"
    );
}

#[test]
fn test_multilanguage_text_fidelity() {
    let multi_lang_samples = [
        ("English", "Amata Professional Vector Editor"),
        ("Japanese", "デザイン 美しい文字 数多"),
        ("Korean", "벡터 그래픽 디자인 스튜디오"),
        ("Chinese", "数多 专业的矢量图形编辑器"),
        (
            "Spanish",
            "Diseño gráfico vectorial moderno con acentos: ¡España, año 2026!",
        ),
        (
            "French",
            "Éditeur vectoriel où la créativité s'exprime en fluidité",
        ),
        (
            "German",
            "Präzise Vektorgrafiken & flüssige Übergänge für Künstler",
        ),
    ];

    for (lang, sample) in multi_lang_samples {
        let mut doc = Document::default();
        let style = TextStyle {
            font_family: "Inter, LINE Seed JP, sans-serif".to_string(),
            font_size: 32.0,
            font_weight: 400,
            font_style: FontStyle::Normal,
            letter_spacing: 0.5,
            text_anchor: TextAnchor::Start,
            ..Default::default()
        };
        let obj = Object::new_text_with_style(lang, sample, 50.0, 100.0, style);
        doc.add_object(obj);

        let svg = export_svg(&doc);
        assert!(!svg.is_empty(), "Export must succeed for {lang}");

        let reloaded = parse_svg_document(&svg);
        let objects: Vec<_> = reloaded.all_objects().collect();
        let (_, reloaded_obj) = &objects[0];
        if let ObjectType::Text { text, .. } = &reloaded_obj.object_type {
            assert_eq!(text, sample, "Round trip text fidelity failed for {lang}");
        }
    }
}

#[test]
fn test_text_to_outline_real_and_mock_fallback() {
    let style_inter = TextStyle {
        font_family: "Inter".to_string(),
        font_size: 40.0,
        font_weight: 400,
        font_style: FontStyle::Normal,
        letter_spacing: 1.0,
        text_anchor: TextAnchor::Start,
        ..Default::default()
    };

    // Extract outline for Latin text with bundled Inter
    let outline_inter = text_to_outline_path_with_style("AMATA 2026", &style_inter);
    assert!(
        !outline_inter.elements.is_empty(),
        "Outline elements should be generated for AMATA 2026"
    );

    // Fallback test: obscure font
    let style_fallback = TextStyle {
        font_family: "ObscureUninstalledFontNameXYZ".to_string(),
        font_size: 30.0,
        font_weight: 400,
        font_style: FontStyle::Normal,
        letter_spacing: 0.0,
        text_anchor: TextAnchor::Start,
        ..Default::default()
    };
    let outline_fallback = text_to_outline_path_with_style("FallbackText", &style_fallback);
    assert!(
        !outline_fallback.elements.is_empty(),
        "Fallback outline generator should safely produce elements without panic"
    );
}

#[test]
fn test_font_registry_enumeration_and_caching() {
    let registry = FontRegistry::global();
    let families = registry.list_families();

    assert!(
        !families.is_empty(),
        "Font registry must discover installed/bundled font families"
    );

    // Calling global multiple times should return the exact same instance without re-scan
    let reg2 = FontRegistry::global();
    assert_eq!(
        registry.list_families().len(),
        reg2.list_families().len(),
        "Cached registry should maintain identical font list"
    );
}
