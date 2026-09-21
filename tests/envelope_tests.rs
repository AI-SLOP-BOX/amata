//! Live envelopes: wrap/edit/release, deform math, export paths.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::envelope::EnvelopeKind;

fn rect() -> Object {
    let mut r = Object::new_rect("R", 0.0, 0.0, 100.0, 50.0, 0.0);
    r.fill = Some(irasu_illustrator::core::path::FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
    r
}

#[test]
fn test_wrap_rejects_non_vector() {
    let text = Object::new_text("T", "hi", 0.0, 0.0, 12.0);
    assert!(Object::wrap_envelope("W", &text, EnvelopeKind::Bulge, 0.5).is_none());
    let img = Object::new_image("I", 0.0, 0.0, 4.0, 4.0, vec![0u8; 64]);
    assert!(Object::wrap_envelope("W", &img, EnvelopeKind::Bulge, 0.5).is_none());
    assert!(Object::wrap_envelope("W", &rect(), EnvelopeKind::Bulge, 0.5).is_some());
}

#[test]
fn test_zero_amount_is_identity() {
    let env = Object::wrap_envelope("W", &rect(), EnvelopeKind::Bulge, 0.0).unwrap();
    let a = env.to_path_data().bounding_box().unwrap();
    let b = rect().to_path_data().bounding_box().unwrap();
    assert!((a.0.x - b.0.x).abs() < 1e-6 && (a.1.x - b.1.x).abs() < 1e-6);
    assert!((a.0.y - b.0.y).abs() < 1e-6 && (a.1.y - b.1.y).abs() < 1e-6);
}

#[test]
fn test_bulge_expands_and_params_matter() {
    let base = Object::wrap_envelope("W", &rect(), EnvelopeKind::Bulge, 0.5).unwrap();
    let (mn, mx) = base.to_path_data().bounding_box().unwrap();
    // Bulge pushes the middle outward: area grows asymmetrically.
    assert!(mx.x - mn.x > 100.0, "bulge widens");
    let pinch = Object::wrap_envelope("W", &rect(), EnvelopeKind::Pinch, 0.5).unwrap();
    let (pn, px) = pinch.to_path_data().bounding_box().unwrap();
    assert!(px.x - pn.x < mx.x - mn.x, "pinch is narrower than bulge");
}

#[test]
fn test_envelope_svg_and_proxy() {
    let env = Object::wrap_envelope("W", &rect(), EnvelopeKind::Wave, 0.8).unwrap();
    // Proxy carries paint + deformed geometry as a plain path.
    let proxy = env.envelope_proxy().expect("proxy");
    assert!(matches!(proxy.object_type, ObjectType::Path(_)));
    assert!(proxy.fill.is_some(), "paint preserved");
    let mut doc = Document::default();
    doc.add_object(env);
    let svg = irasu_illustrator::io::svg::export_svg(&doc);
    assert!(svg.contains("<path"), "envelope renders, got:\n{svg}");
}

#[test]
fn test_envelope_serialization_round_trip() {
    let mut doc = Document::default();
    doc.add_object(Object::wrap_envelope("W", &rect(), EnvelopeKind::Twist, -0.4).unwrap());
    let json = serde_json::to_string(&doc).unwrap();
    let back: Document = serde_json::from_str(&json).unwrap();
    let found = back
        .all_objects()
        .map(|(_, o)| o)
        .find(|o| matches!(o.object_type, ObjectType::Envelope { .. }))
        .expect("envelope survives JSON");
    if let ObjectType::Envelope { kind, amount, .. } = &found.object_type {
        assert_eq!(*kind, EnvelopeKind::Twist);
        assert!((amount + 0.4).abs() < 1e-9);
    }
    // Still renders after reload.
    let svg = irasu_illustrator::io::svg::export_svg(&back);
    assert!(svg.contains("<path"));
}
