use irasu_illustrator::core::document::Object;
use irasu_illustrator::core::shape_builder::{
    decompose_shapes_into_fragments, fragment_to_object, merge_fragments,
};

#[test]
fn test_shape_builder_decomposition() {
    // Two overlapping rectangles
    // R1: (0, 0) to (100, 100)
    // R2: (50, 0) to (150, 100)
    // Intersects at (50, 0) to (100, 100)
    let r1 = Object::new_rect("Rect1", 0.0, 0.0, 100.0, 100.0, 0.0);
    let r2 = Object::new_rect("Rect2", 50.0, 0.0, 100.0, 100.0, 0.0);

    let frags = decompose_shapes_into_fragments(&[&r1, &r2]);
    assert!(!frags.is_empty(), "Should generate shape fragments");
    // Intersecting 2 rectangles should yield at least 3 disjoint regions
    assert!(
        frags.len() >= 3,
        "Expected at least 3 fragments, got {}",
        frags.len()
    );

    // Check that one fragment belongs to both parents (intersection)
    let overlapping = frags.iter().find(|f| f.original_object_indices.len() == 2);
    assert!(
        overlapping.is_some(),
        "Should have a shared intersection fragment"
    );

    // Extract one fragment
    let frag_obj = fragment_to_object(&frags[0], &r1);
    assert!(!frag_obj.name.is_empty());
    assert_eq!(frag_obj.visible, true);
}

#[test]
fn test_shape_builder_merge() {
    let r1 = Object::new_rect("Rect1", 0.0, 0.0, 100.0, 100.0, 0.0);
    let r2 = Object::new_rect("Rect2", 50.0, 0.0, 100.0, 100.0, 0.0);

    let frags = decompose_shapes_into_fragments(&[&r1, &r2]);
    let frag_refs: Vec<_> = frags.iter().collect();
    let merged = merge_fragments(&frag_refs, &r1);

    assert_eq!(merged.visible, true);
    assert!(merged.name.contains("Merged"));
}

#[test]
fn test_shape_builder_cli_run() {
    // Test the CLI shape-builder decomposition flow directly
    let mut doc = irasu_illustrator::core::document::Document::default();
    let r1 = Object::new_rect("R1", 10.0, 10.0, 80.0, 80.0, 0.0);
    let r2 = Object::new_rect("R2", 40.0, 10.0, 80.0, 80.0, 0.0);
    doc.layers[0].objects.push(r1);
    doc.layers[0].objects.push(r2);

    let objs: Vec<&Object> = doc.all_objects().map(|(_, o)| o).collect();
    let frags = decompose_shapes_into_fragments(&objs);
    assert!(frags.len() >= 3);
}
