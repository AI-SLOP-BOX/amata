use irasu_illustrator::core::document::{Document, Layer, Object};

#[test]
fn test_layer_reordering() {
    let mut doc = Document::default();
    doc.layers.push(Layer::new("Layer 2"));
    doc.layers.push(Layer::new("Layer 3"));

    assert_eq!(doc.layers.len(), 3);
    assert_eq!(doc.layers[0].name, "Layer 1");
    assert_eq!(doc.layers[1].name, "Layer 2");

    // Move Layer 1 Up (swap with Layer 2)
    doc.move_layer_up(0);
    assert_eq!(doc.layers[0].name, "Layer 2");
    assert_eq!(doc.layers[1].name, "Layer 1");

    // Move Layer 1 Down (swap back)
    doc.move_layer_down(1);
    assert_eq!(doc.layers[0].name, "Layer 1");
    assert_eq!(doc.layers[1].name, "Layer 2");
}

#[test]
fn test_object_reordering_within_layer() {
    let mut doc = Document::default();
    let obj1 = Object::new_rect("Rect 1", 0.0, 0.0, 100.0, 100.0, 0.0);
    let obj2 = Object::new_rect("Rect 2", 10.0, 10.0, 100.0, 100.0, 0.0);
    doc.layers[0].objects.push(obj1);
    doc.layers[0].objects.push(obj2);

    assert_eq!(doc.layers[0].objects[0].name, "Rect 1");
    assert_eq!(doc.layers[0].objects[1].name, "Rect 2");

    // Move Rect 1 forward (swap with Rect 2)
    doc.move_object_up(0, 0);
    assert_eq!(doc.layers[0].objects[0].name, "Rect 2");
    assert_eq!(doc.layers[0].objects[1].name, "Rect 1");

    // Move Rect 1 backward
    doc.move_object_down(0, 1);
    assert_eq!(doc.layers[0].objects[0].name, "Rect 1");
    assert_eq!(doc.layers[0].objects[1].name, "Rect 2");
}

#[test]
fn test_move_object_between_layers() {
    let mut doc = Document::default();
    doc.layers.push(Layer::new("Layer 2"));
    let obj = Object::new_ellipse("Circle 1", 50.0, 50.0, 30.0, 30.0);
    doc.layers[0].objects.push(obj);

    assert_eq!(doc.layers[0].objects.len(), 1);
    assert_eq!(doc.layers[1].objects.len(), 0);

    // Transfer Circle 1 from Layer 0 to Layer 1
    doc.move_object_to_layer(0, 0, 1);
    assert_eq!(doc.layers[0].objects.len(), 0);
    assert_eq!(doc.layers[1].objects.len(), 1);
    assert_eq!(doc.layers[1].objects[0].name, "Circle 1");
}
