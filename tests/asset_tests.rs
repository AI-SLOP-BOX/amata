//! 自作アセット（テンプレート・スウォッチ）の配布テスト。
//! 埋め込みアセットがパースでき、プロジェクトとして開けることを保証する。

use irasu_illustrator::io::library::{built_in_swatches, built_in_templates, template_by_id};

#[test]
fn swatch_libraries_parse_and_expose_inks() {
    let libs = built_in_swatches();
    assert!(!libs.is_empty(), "at least one bundled library");
    assert_eq!(libs.len(), 4, "four bundled libraries: {}", libs.len());
    let jp = libs
        .iter()
        .find(|l| l.name.contains("伝統色"))
        .expect("traditional library");
    assert_eq!(jp.colors.len(), 18, "十八色: {:?}", jp.colors.len());
    for (slug, cmyk) in &jp.colors {
        assert!(!slug.is_empty());
        assert!(
            cmyk.iter().all(|c| (0.0..=1.0).contains(c)),
            "{slug} out of range: {cmyk:?}"
        );
    }
    // 藍 (indigo) and 墨 (ink) are the extremes of the set.
    // DIC近似: 18 inks, all in range, ink names carry the DIC number.
    let dic = libs.iter().find(|l| l.name.contains("DIC")).expect("dic");
    assert_eq!(dic.colors.len(), 18, "DIC kit: {}", dic.colors.len());
    for (slug, cmyk) in &dic.colors {
        assert!(cmyk.iter().all(|c| (0.0..=1.0).contains(c)), "{slug}");
        assert!(slug.starts_with("DIC "), "{slug}");
    }
    // 無彩色: grays ramp K only; リッチ黒 adds CMY under 100% K.
    let gray = libs
        .iter()
        .find(|l| l.name.contains("無彩色"))
        .expect("grays");
    assert_eq!(gray.colors.len(), 12);
    let rich = gray
        .colors
        .iter()
        .find(|(s, _)| s == "リッチ黒")
        .expect("rich black");
    assert!(
        rich.1[3] > 0.9 && rich.1[0] > 0.5,
        "rich black: {:?}",
        rich.1
    );
    // UIフラット: 12 colors, ink 900 is the darkest.
    let ui = libs
        .iter()
        .find(|l| l.name.contains("UI"))
        .expect("ui flat");
    assert_eq!(ui.colors.len(), 12);
    let ai = jp.colors.iter().find(|(s, _)| s == "藍").expect("藍");
    assert!(ai.1[0] > 0.7, "indigo is cyan-heavy: {:?}", ai.1);
    let sumi = jp.colors.iter().find(|(s, _)| s == "墨").expect("墨");
    assert_eq!(sumi.1, [0.0, 0.0, 0.0, 0.9]);
    // Spots carry the library ink straight through.
    let spots = jp.to_spots();
    assert_eq!(spots.len(), jp.colors.len());
    assert_eq!(spots[0].cmyk, ai.1);
}

#[test]
fn templates_load_as_valid_projects() {
    let templates = built_in_templates();
    assert_eq!(templates.len(), 7, "seven bundled templates");
    let poster = templates
        .iter()
        .find(|t| t.id == "a4-poster-vertical")
        .expect("poster");
    // A4 portrait ratio (1240×1754 ≈ 1:1.414).
    assert!(
        (poster.size.1 / poster.size.0 - 1.414).abs() < 0.01,
        "A4 ratio: {:?}",
        poster.size
    );
    assert!(!poster.title.is_empty());
    // Each template carries real content: layers with objects.
    let objects: usize = poster.document.layers.iter().map(|l| l.objects.len()).sum();
    assert!(objects >= 4, "poster has art + text: {objects}");
    // Vertical writing is set on the body text.
    let vertical = poster
        .document
        .layers
        .iter()
        .flat_map(|l| &l.objects)
        .any(|o| {
            matches!(&o.object_type, irasu_illustrator::core::document::ObjectType::Text { style, .. } if style.vertical)
        });
    assert!(vertical, "the poster body is vertical");
    // Artboards match the canvas size (templates are print-ready).
    let card = templates
        .iter()
        .find(|t| t.id == "business-card")
        .expect("card");
    assert_eq!(card.document.artboards.len(), 1);
    assert_eq!((card.size.0, card.size.1), (1075.0, 650.0));
    // Id lookup works (used by the picker).
    assert!(template_by_id("flyer-2col").is_some());
    assert!(template_by_id("nope").is_none());
    // Every template is non-empty, artboard-matched and id-addressable.
    for t in &templates {
        let objects: usize = t.document.layers.iter().map(|l| l.objects.len()).sum();
        assert!(objects >= 1, "{} has artwork: {objects}", t.id);
        assert!(
            !t.document.artboards.is_empty(),
            "{} carries an artboard",
            t.id
        );
        assert!(
            (t.size.0 - t.document.artboards[0].width).abs() < 1e-6,
            "{} canvas matches its artboard",
            t.id
        );
    }
    // Print-oriented templates carry a spot library.
    let invoice = template_by_id("invoice-a4").expect("invoice");
    assert!(
        !invoice.document.spots.is_empty(),
        "the invoice registers spot inks"
    );
    let story = template_by_id("story-9x16").expect("story");
    assert_eq!((story.size.0, story.size.1), (1080.0, 1920.0), "9:16");
}
