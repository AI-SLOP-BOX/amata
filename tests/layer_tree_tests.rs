use irasu_illustrator::core::document::{Document, Layer, Object};
use irasu_illustrator::core::history::{Command, ReparentObjectCommand, UndoManager};
use irasu_illustrator::core::state::AppState;

fn rect(name: &str) -> Object {
    Object::new_rect(name, 0.0, 0.0, 10.0, 10.0, 0.0)
}

#[test]
fn reparent_between_layers_execute_undo() {
    let mut doc = Document::default();
    doc.layers.push(Layer::new("Layer 2"));
    let obj = rect("Rect 1");
    let id = obj.id.clone();
    doc.layers[0].objects.push(obj);

    let mut cmd = ReparentObjectCommand {
        object_id: id.clone(),
        old_parent: None,
        old_layer: 0,
        old_index: 0,
        new_parent: None,
        new_layer: 1,
        new_index: 0,
    };
    cmd.execute(&mut doc);
    assert!(doc.layers[0].objects.is_empty());
    assert_eq!(doc.layers[1].objects.len(), 1);
    assert_eq!(doc.layers[1].objects[0].id, id);
    assert_eq!(cmd.name(), "Move Object in Tree");

    cmd.undo(&mut doc);
    assert_eq!(doc.layers[0].objects.len(), 1);
    assert!(doc.layers[1].objects.is_empty());
    assert_eq!(doc.layers[0].objects[0].id, id);
}

#[test]
fn reparent_into_group_execute_undo() {
    let mut doc = Document::default();
    let a = rect("A");
    let a_id = a.id.clone();
    let group = Object::new_group("G", vec![rect("Child")]);
    let g_id = group.id.clone();
    doc.layers[0].objects.push(a);
    doc.layers[0].objects.push(group);

    let mut cmd = ReparentObjectCommand {
        object_id: a_id.clone(),
        old_parent: None,
        old_layer: 0,
        old_index: 0,
        new_parent: Some(g_id.clone()),
        new_layer: 0,
        new_index: 1,
    };
    cmd.execute(&mut doc);
    assert_eq!(doc.layers[0].objects.len(), 1);
    let g = doc.find_object(&g_id).unwrap();
    let children = Document::children_of(g);
    assert_eq!(children.len(), 2);
    assert_eq!(children[1].id, a_id);

    cmd.undo(&mut doc);
    assert_eq!(doc.layers[0].objects.len(), 2);
    assert_eq!(doc.layers[0].objects[0].id, a_id);
    let g = doc.find_object(&g_id).unwrap();
    assert_eq!(Document::children_of(g).len(), 1);
}

#[test]
fn reparent_same_container_reorder_execute_undo() {
    let mut doc = Document::default();
    doc.layers[0].objects.push(rect("A"));
    doc.layers[0].objects.push(rect("B"));
    doc.layers[0].objects.push(rect("C"));
    let a_id = doc.layers[0].objects[0].id.clone();

    // A before C: after removing A, B is 0 / C is 1, so insertion index 1.
    let mut cmd = ReparentObjectCommand {
        object_id: a_id.clone(),
        old_parent: None,
        old_layer: 0,
        old_index: 0,
        new_parent: None,
        new_layer: 0,
        new_index: 1,
    };
    cmd.execute(&mut doc);
    let names: Vec<&str> = doc.layers[0]
        .objects
        .iter()
        .map(|o| o.name.as_str())
        .collect();
    assert_eq!(names, ["B", "A", "C"]);

    cmd.undo(&mut doc);
    let names: Vec<&str> = doc.layers[0]
        .objects
        .iter()
        .map(|o| o.name.as_str())
        .collect();
    assert_eq!(names, ["A", "B", "C"]);
}

#[test]
fn reparent_via_undo_manager_is_one_step() {
    let mut doc = Document::default();
    doc.layers.push(Layer::new("Layer 2"));
    let obj = rect("Rect 1");
    let id = obj.id.clone();
    doc.layers[0].objects.push(obj);

    let mut undo = UndoManager::new();
    undo.execute(
        Box::new(ReparentObjectCommand {
            object_id: id.clone(),
            old_parent: None,
            old_layer: 0,
            old_index: 0,
            new_parent: None,
            new_layer: 1,
            new_index: 0,
        }),
        &mut doc,
    );
    assert_eq!(doc.layers[1].objects.len(), 1);
    assert_eq!(undo.undo_name(), Some("Move Object in Tree"));

    undo.undo(&mut doc);
    assert_eq!(doc.layers[0].objects.len(), 1);
    assert!(doc.layers[1].objects.is_empty());
}

#[test]
fn is_descendant_of_strict_and_recursive() {
    let mut doc = Document::default();
    let leaf = rect("Leaf");
    let leaf_id = leaf.id.clone();
    let inner = Object::new_group("Inner", vec![leaf]);
    let inner_id = inner.id.clone();
    let outer = Object::new_group("Outer", vec![inner]);
    let outer_id = outer.id.clone();
    doc.layers[0].objects.push(outer);

    assert!(doc.is_descendant_of(&leaf_id, &outer_id));
    assert!(doc.is_descendant_of(&inner_id, &outer_id));
    assert!(!doc.is_descendant_of(&outer_id, &outer_id));
    assert!(!doc.is_descendant_of(&outer_id, &leaf_id));
    assert!(!doc.is_descendant_of("missing", &outer_id));
}

#[test]
fn insert_object_at_clamps_and_falls_back() {
    let mut doc = Document::default();
    let g = Object::new_group("G", vec![rect("Child")]);
    let g_id = g.id.clone();
    doc.layers[0].objects.push(g);

    let obj = rect("Inserted");
    let obj_id = obj.id.clone();
    doc.insert_object_at(Some(&g_id), 0, 99, obj);
    let g = doc.find_object(&g_id).unwrap();
    assert_eq!(Document::children_of(g).len(), 2);
    assert_eq!(Document::children_of(g)[1].id, obj_id);

    // Dangling parent falls back to the layer instead of losing the object.
    let obj2 = rect("Fallback");
    let obj2_id = obj2.id.clone();
    doc.insert_object_at(Some("missing-id"), 0, 0, obj2);
    assert_eq!(doc.layers[0].objects[0].id, obj2_id);
}

#[test]
fn children_of_empty_for_leaf() {
    let leaf = rect("Leaf");
    assert!(Document::children_of(&leaf).is_empty());
    let g = Object::new_group("G", vec![rect("C")]);
    assert_eq!(Document::children_of(&g).len(), 1);
}

#[test]
fn rename_via_snapshot_commit_is_undoable() {
    let mut state = AppState::default();
    let obj = rect("Before");
    let id = obj.id.clone();
    state.document.layers[0].objects.push(obj);

    state.ensure_object_snapshot(&id);
    state.document.find_object_mut(&id).unwrap().name = "After".into();
    state.commit_object_edits("Rename Object");

    assert_eq!(state.document.find_object(&id).unwrap().name, "After");
    // ObjectCommand::name() is hardcoded; label args are informational only.
    assert_eq!(state.undo_manager.undo_name(), Some("Edit Object"));

    state.undo_manager.undo(&mut state.document);
    assert_eq!(state.document.find_object(&id).unwrap().name, "Before");
}

#[test]
fn rename_commit_without_change_records_nothing() {
    let mut state = AppState::default();
    let obj = rect("Same");
    let id = obj.id.clone();
    state.document.layers[0].objects.push(obj);

    state.ensure_object_snapshot(&id);
    state.commit_object_edits("Rename Object");
    assert_eq!(state.undo_manager.undo_name(), None);
}
