#![allow(clippy::field_reassign_with_default)]
use irasu_illustrator::core::document::{Document, Object, ObjectType, Symbol};
use irasu_illustrator::core::path::{FillStyle, FillType, GradientStop, LinearGradient};
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};

#[test]
fn test_one_symbol_one_use() {
    let mut doc = Document::default();
    let mut sym_obj = Object::new_rect("CardMaster", 0.0, 0.0, 150.0, 100.0, 6.0);
    sym_obj.id = "card-master".to_string();

    doc.add_symbol(Symbol {
        id: "card-component".to_string(),
        name: "Card".to_string(),
        object: sym_obj,
        use_count: 1,
    });

    let use_inst = Object::new_use(
        "Card1",
        "card-component",
        50.0,
        50.0,
        Some(150.0),
        Some(100.0),
    );
    doc.add_object(use_inst);

    let svg = export_svg(&doc);
    assert!(svg.contains("<symbol id=\"card-component\">"));
    assert!(svg.contains("href=\"#card-component\""));

    let reimported = parse_svg_document(&svg);
    assert_eq!(reimported.symbols.len(), 1);
    assert_eq!(reimported.symbols[0].id, "card-component");
    assert_eq!(reimported.layers[0].objects.len(), 1);

    if let ObjectType::Use { href, .. } = &reimported.layers[0].objects[0].object_type {
        assert_eq!(href, "card-component");
    } else {
        panic!("Expected Use object");
    }
}

#[test]
fn test_one_symbol_many_use_and_transforms() {
    let mut doc = Document::default();
    let mut star_master = Object::new_star("StarMaster", 0.0, 0.0, 5, 20.0, 50.0);
    star_master.id = "star-master".to_string();

    doc.add_symbol(Symbol {
        id: "star-sym".to_string(),
        name: "Star".to_string(),
        object: star_master,
        use_count: 3,
    });

    let mut u1 = Object::new_use("Star 1", "star-sym", 100.0, 100.0, None, None);
    u1.transform.scale_x = 1.0;
    u1.transform.scale_y = 1.0;

    let mut u2 = Object::new_use("Star 2", "star-sym", 250.0, 100.0, None, None);
    u2.transform.scale_x = 1.5;
    u2.transform.scale_y = 1.5;

    let mut u3 = Object::new_use("Star 3", "star-sym", 400.0, 100.0, None, None);
    u3.transform.rotation = 45.0f64.to_radians();

    doc.add_object(u1);
    doc.add_object(u2);
    doc.add_object(u3);

    let svg = export_svg(&doc);
    assert!(svg.contains("<symbol id=\"star-sym\">"));
    // Identity instance keeps plain x/y; transformed instances must carry a matrix
    // so scale/rotation survive the round-trip (previously they were dropped).
    assert!(svg.contains("href=\"#star-sym\" x=\"100\" y=\"100\""));
    assert!(svg.contains("transform=\"matrix("));

    let reimported = parse_svg_document(&svg);
    assert_eq!(reimported.symbols.len(), 1);
    assert_eq!(reimported.layers[0].objects.len(), 3);
    let objs = &reimported.layers[0].objects;
    assert!((objs[0].transform.x - 100.0).abs() < 1e-6);
    assert!((objs[1].transform.scale_x - 1.5).abs() < 1e-6);
    assert!((objs[1].transform.x - 250.0).abs() < 1e-6);
    assert!((objs[2].transform.rotation - 45.0f64.to_radians()).abs() < 1e-6);
    assert!((objs[2].transform.x - 400.0).abs() < 1e-6);
}

#[test]
fn test_nested_groups_and_gradient_and_text_in_symbol() {
    let mut doc = Document::default();

    let mut bg = Object::new_rect("BadgeBG", 0.0, 0.0, 200.0, 80.0, 8.0);
    let mut grad = LinearGradient::default();
    grad.stops = vec![
        GradientStop {
            offset: 0.0,
            color: [1.0, 0.5, 0.0, 1.0],
        },
        GradientStop {
            offset: 1.0,
            color: [1.0, 0.0, 0.5, 1.0],
        },
    ];
    bg.fill = Some(FillStyle {
        color: [0.0, 0.0, 0.0, 1.0],
        fill_type: FillType::Linear(grad),
        rule: irasu_illustrator::core::path::FillRule::NonZero,
        overprint: false,
        spot: None,
    });

    let label = Object::new_text("BadgeLabel", "VIP ACCESS", 20.0, 50.0, 22.0);

    let badge_group = Object::new_group("BadgeGroup", vec![bg, label]);

    doc.add_symbol(Symbol {
        id: "badge-symbol".to_string(),
        name: "Badge".to_string(),
        object: badge_group,
        use_count: 2,
    });

    let inst1 = Object::new_use("VIP 1", "badge-symbol", 50.0, 100.0, None, None);
    let inst2 = Object::new_use("VIP 2", "badge-symbol", 50.0, 220.0, None, None);
    doc.add_object(inst1);
    doc.add_object(inst2);

    let svg = export_svg(&doc);
    assert!(svg.contains("<symbol id=\"badge-symbol\">"));
    assert!(svg.contains("VIP ACCESS"));
    assert!(svg.contains("<linearGradient"));

    let reimported = parse_svg_document(&svg);
    assert_eq!(reimported.symbols.len(), 1);
    assert_eq!(reimported.symbols[0].id, "badge-symbol");
    assert_eq!(reimported.layers[0].objects.len(), 2);
}

#[test]
fn test_nested_use_and_round_trip() {
    let mut doc = Document::default();

    // 1. Icon symbol
    let icon = Object::new_ellipse("Dot", 10.0, 10.0, 5.0, 5.0);
    doc.add_symbol(Symbol {
        id: "dot-icon".to_string(),
        name: "Dot".to_string(),
        object: icon,
        use_count: 1,
    });

    // 2. Button symbol containing a use instance of Dot
    let btn_bg = Object::new_rect("BtnBG", 0.0, 0.0, 120.0, 36.0, 4.0);
    let dot_inst = Object::new_use("DotInBtn", "dot-icon", 15.0, 18.0, None, None);
    let btn_compound = Object::new_group("BtnCompound", vec![btn_bg, dot_inst]);

    doc.add_symbol(Symbol {
        id: "button-comp".to_string(),
        name: "Button".to_string(),
        object: btn_compound,
        use_count: 1,
    });

    // 3. Document instance of Button
    let btn_instance = Object::new_use("BtnInst", "button-comp", 300.0, 400.0, None, None);
    doc.add_object(btn_instance);

    let svg = export_svg(&doc);
    assert!(svg.contains("<symbol id=\"dot-icon\">"));
    assert!(svg.contains("<symbol id=\"button-comp\">"));
    assert!(svg.contains("href=\"#button-comp\""));

    let reimported = parse_svg_document(&svg);
    assert_eq!(reimported.symbols.len(), 2);
    assert_eq!(reimported.layers[0].objects.len(), 1);
}

#[test]
fn test_edit_master_updates_instances() {
    let mut doc = Document::default();

    let master_rect = Object::new_rect("MasterRect", 0.0, 0.0, 100.0, 50.0, 0.0);
    doc.add_symbol(Symbol {
        id: "box-sym".to_string(),
        name: "Box".to_string(),
        object: master_rect,
        use_count: 2,
    });

    let u1 = Object::new_use("U1", "box-sym", 10.0, 10.0, None, None);
    let u2 = Object::new_use("U2", "box-sym", 200.0, 10.0, None, None);
    doc.add_object(u1);
    doc.add_object(u2);

    // Edit master symbol geometry/color
    if let Some(sym) = doc.symbol_by_id_mut("box-sym") {
        sym.object.fill = Some(FillStyle::solid([0.0, 1.0, 0.0, 1.0]));
        if let ObjectType::Rectangle {
            width,
            height,
            corner_radius,
        } = &mut sym.object.object_type
        {
            *width = 180.0;
            *height = 90.0;
            *corner_radius = 12.0;
        }
    }

    // Both instances reference "box-sym", so exporting document renders the updated master in <symbol>
    let svg = export_svg(&doc);
    assert!(svg.contains("width=\"180\" height=\"90\" rx=\"12\""));
    assert!(svg.contains("fill=\"#00ff00\""));

    let reimported = parse_svg_document(&svg);
    let sym = reimported.symbol_by_id("box-sym").expect("find symbol");
    if let ObjectType::Rectangle {
        width,
        height,
        corner_radius,
    } = sym.object.object_type
    {
        assert_eq!(width, 180.0);
        assert_eq!(height, 90.0);
        assert_eq!(corner_radius, 12.0);
    } else {
        panic!("Expected rectangle");
    }
}
