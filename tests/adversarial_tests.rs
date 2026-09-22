use irasu_illustrator::core::diff::compute_semantic_diff;
use irasu_illustrator::core::document::Document;
use irasu_illustrator::core::history::{Command as HistoryCommand, UndoManager};
use irasu_illustrator::core::watcher::FileWatcher;
use irasu_illustrator::io::git::{
    create_checkpoint, preserve_external_version, restore_file_to_rev,
};
use irasu_illustrator::io::svg::parse_svg_document;
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

fn setup_adversarial_temp_repo(prefix: &str) -> std::path::PathBuf {
    let unique_id = uuid::Uuid::new_v4().to_string();
    let repo_dir = std::env::temp_dir().join(format!("amata_adv_test_{prefix}_{unique_id}"));
    let _ = fs::remove_dir_all(&repo_dir);
    fs::create_dir_all(&repo_dir).expect("Failed to create temp repo dir");

    let status = Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .status()
        .expect("git init");
    assert!(status.success());

    Command::new("git")
        .args(["config", "user.name", "Adversarial Tester"])
        .current_dir(&repo_dir)
        .status()
        .expect("git config user.name");
    Command::new("git")
        .args(["config", "user.email", "adv@amata.local"])
        .current_dir(&repo_dir)
        .status()
        .expect("git config user.email");

    repo_dir
}

// =============================================================================
// 1. STABLE ID ADVERSARIAL TESTS (Breaking sequential ID assumption)
// =============================================================================

#[test]
fn test_adversarial_idless_prepend_element() {
    // Create an SVG with 100 id-less rect elements at distinct coordinates
    let mut original_svg =
        String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="2000">"##);
    for i in 0..100 {
        original_svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="10" height="10" fill="#ff0000"/>"##,
            i * 15,
            i * 15
        ));
    }
    original_svg.push_str("</svg>");

    // Now prepend 1 new element at the very beginning of the SVG
    let mut prepended_svg =
        String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="2000">"##);
    prepended_svg.push_str(r##"<rect x="9999" y="9999" width="10" height="10" fill="#00ff00"/>"##);
    for i in 0..100 {
        prepended_svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="10" height="10" fill="#ff0000"/>"##,
            i * 15,
            i * 15
        ));
    }
    prepended_svg.push_str("</svg>");

    let doc_before = parse_svg_document(&original_svg);
    let doc_after = parse_svg_document(&prepended_svg);

    let diff = compute_semantic_diff(&doc_before, &doc_after);

    println!("\n[Test A: Idless Prepend]");
    println!(
        "  Summary: Added={}, Removed={}, Modified={}, Unchanged={}",
        diff.summary.added_count,
        diff.summary.removed_count,
        diff.summary.modified_count,
        diff.summary.unchanged_count
    );

    // If sequential IDs (rect_1 -> rect_2) are used, ALL 100 items cascade into Modified!
    // A genuinely stable matcher MUST recognise Added=1, and 0 or minimal Modified.
    assert_eq!(
        diff.summary.added_count, 1,
        "Should detect exactly 1 added element"
    );
    assert!(
        diff.summary.modified_count <= 5,
        "Existing 100 elements should NOT all cascade into modified! Got: {}",
        diff.summary.modified_count
    );
}

#[test]
fn test_adversarial_idless_middle_insertion() {
    let mut original_svg =
        String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="2000">"##);
    for i in 0..100 {
        original_svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="10" height="10" fill="#ff0000"/>"##,
            i * 15,
            i * 15
        ));
    }
    original_svg.push_str("</svg>");

    // Insert 1 element right in the middle (after 50th element)
    let mut inserted_svg =
        String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="2000">"##);
    for i in 0..50 {
        inserted_svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="10" height="10" fill="#ff0000"/>"##,
            i * 15,
            i * 15
        ));
    }
    inserted_svg.push_str(r##"<circle cx="555" cy="555" r="20" fill="#0000ff"/>"##);
    for i in 50..100 {
        inserted_svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="10" height="10" fill="#ff0000"/>"##,
            i * 15,
            i * 15
        ));
    }
    inserted_svg.push_str("</svg>");

    let doc_before = parse_svg_document(&original_svg);
    let doc_after = parse_svg_document(&inserted_svg);
    let diff = compute_semantic_diff(&doc_before, &doc_after);

    println!("\n[Test B: Idless Middle Insertion]");
    println!(
        "  Summary: Added={}, Removed={}, Modified={}, Unchanged={}",
        diff.summary.added_count,
        diff.summary.removed_count,
        diff.summary.modified_count,
        diff.summary.unchanged_count
    );

    assert_eq!(diff.summary.added_count, 1);
    assert!(
        diff.summary.modified_count <= 5,
        "Subsequent 50 elements should NOT cascade into modified! Got: {}",
        diff.summary.modified_count
    );
}

#[test]
fn test_adversarial_idless_deletion() {
    let mut original_svg =
        String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="2000">"##);
    for i in 0..100 {
        original_svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="10" height="10" fill="#ff0000"/>"##,
            i * 15,
            i * 15
        ));
    }
    original_svg.push_str("</svg>");

    // Delete the very first element (index 0)
    let mut deleted_svg =
        String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="2000">"##);
    for i in 1..100 {
        deleted_svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="10" height="10" fill="#ff0000"/>"##,
            i * 15,
            i * 15
        ));
    }
    deleted_svg.push_str("</svg>");

    let doc_before = parse_svg_document(&original_svg);
    let doc_after = parse_svg_document(&deleted_svg);
    let diff = compute_semantic_diff(&doc_before, &doc_after);

    println!("\n[Test C: Idless Deletion]");
    println!(
        "  Summary: Added={}, Removed={}, Modified={}, Unchanged={}",
        diff.summary.added_count,
        diff.summary.removed_count,
        diff.summary.modified_count,
        diff.summary.unchanged_count
    );

    assert_eq!(
        diff.summary.removed_count, 1,
        "Should detect 1 removed element"
    );
    assert!(
        diff.summary.modified_count <= 5,
        "Remaining 99 elements should NOT cascade into modified! Got: {}",
        diff.summary.modified_count
    );
}

#[test]
fn test_adversarial_idless_reorder() {
    // 5 distinct rects
    let original_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000">
  <rect x="10" y="10" width="50" height="50" fill="#111111"/>
  <rect x="100" y="100" width="50" height="50" fill="#222222"/>
  <rect x="200" y="200" width="50" height="50" fill="#333333"/>
  <rect x="300" y="300" width="50" height="50" fill="#444444"/>
  <rect x="400" y="400" width="50" height="50" fill="#555555"/>
</svg>"##;

    // Reverse order
    let reversed_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000">
  <rect x="400" y="400" width="50" height="50" fill="#555555"/>
  <rect x="300" y="300" width="50" height="50" fill="#444444"/>
  <rect x="200" y="200" width="50" height="50" fill="#333333"/>
  <rect x="100" y="100" width="50" height="50" fill="#222222"/>
  <rect x="10" y="10" width="50" height="50" fill="#111111"/>
</svg>"##;

    let doc_before = parse_svg_document(original_svg);
    let doc_after = parse_svg_document(reversed_svg);
    let diff = compute_semantic_diff(&doc_before, &doc_after);

    println!("\n[Test D: Idless Reorder]");
    println!(
        "  Summary: Added={}, Removed={}, Modified={}, Unchanged={}",
        diff.summary.added_count,
        diff.summary.removed_count,
        diff.summary.modified_count,
        diff.summary.unchanged_count
    );

    assert_eq!(
        diff.summary.added_count, 0,
        "Reordering should not add elements"
    );
    assert_eq!(
        diff.summary.removed_count, 0,
        "Reordering should not remove elements"
    );
}

#[test]
fn test_adversarial_external_id_removal() {
    let before_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500">
  <text id="headline" x="100" y="100" font-size="32">HIRARI FESTIVAL</text>
  <path id="waveform" d="M 0 0 L 100 100" stroke="#ff0000"/>
</svg>"##;

    // External AI cleans SVG and removes all id attributes
    let after_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500">
  <text x="100" y="100" font-size="32">HIRARI FESTIVAL</text>
  <path d="M 0 0 L 100 100" stroke="#ff0000"/>
</svg>"##;

    let doc_before = parse_svg_document(before_svg);
    let doc_after = parse_svg_document(after_svg);
    let diff = compute_semantic_diff(&doc_before, &doc_after);

    println!("\n[Test H: External ID Removal]");
    println!(
        "  Summary: Added={}, Removed={}, Modified={}, Unchanged={}",
        diff.summary.added_count,
        diff.summary.removed_count,
        diff.summary.modified_count,
        diff.summary.unchanged_count
    );

    // It should NOT treat the entire artwork as 2 Removed + 2 Added
    assert_eq!(
        diff.summary.removed_count, 0,
        "Should match elements despite id attribute removal"
    );
    assert_eq!(
        diff.summary.added_count, 0,
        "Should match elements despite id attribute removal"
    );
}

#[test]
fn test_adversarial_duplicate_identical_objects() {
    // 50 identical rects at exactly same position without ID
    let mut svg1 =
        String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000">"##);
    for _ in 0..50 {
        svg1.push_str(r##"<rect x="100" y="100" width="50" height="50" fill="#ff0000"/>"##);
    }
    svg1.push_str("</svg>");

    // Move only 1 of them to x=500
    let mut svg2 =
        String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000">"##);
    for _ in 0..49 {
        svg2.push_str(r##"<rect x="100" y="100" width="50" height="50" fill="#ff0000"/>"##);
    }
    svg2.push_str(r##"<rect x="500" y="100" width="50" height="50" fill="#ff0000"/>"##);
    svg2.push_str("</svg>");

    let doc1 = parse_svg_document(&svg1);
    let doc2 = parse_svg_document(&svg2);
    let diff = compute_semantic_diff(&doc1, &doc2);

    println!("\n[Test: Duplicate Similar Objects]");
    println!(
        "  Summary: Added={}, Removed={}, Modified={}, Unchanged={}",
        diff.summary.added_count,
        diff.summary.removed_count,
        diff.summary.modified_count,
        diff.summary.unchanged_count
    );

    // Exactly 49 should remain unchanged, and 1 modified (or 1 removed + 1 added)
    assert_eq!(diff.summary.unchanged_count, 49);
}

// =============================================================================
// 2. SEMANTIC DIFF ROBUSTNESS (Syntactic variation vs Semantic equality)
// =============================================================================

#[test]
fn test_adversarial_semantic_diff_attribute_order() {
    let svg_a = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r1" x="10" y="20" width="100" height="50" fill="#ff0000"/></svg>"##;
    let svg_b = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect fill="#ff0000" height="50" width="100" y="20" x="10" id="r1"/></svg>"##;

    let doc_a = parse_svg_document(svg_a);
    let doc_b = parse_svg_document(svg_b);
    let diff = compute_semantic_diff(&doc_a, &doc_b);

    assert_eq!(
        diff.summary.modified_count, 0,
        "Attribute order alone must not create diff"
    );
    assert_eq!(diff.summary.unchanged_count, 1);
}

#[test]
fn test_adversarial_semantic_diff_whitespace_and_minification() {
    let svg_pretty = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500">
  <g id="group1">
    <text
      id="t1"
      x="100"
      y="200"
      font-size="24"
    >
      HELLO WORLD
    </text>
  </g>
</svg>"##;

    let svg_minified = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500"><g id="group1"><text id="t1" x="100" y="200" font-size="24">HELLO WORLD</text></g></svg>"##;

    let doc_a = parse_svg_document(svg_pretty);
    let doc_b = parse_svg_document(svg_minified);
    let diff = compute_semantic_diff(&doc_a, &doc_b);

    assert_eq!(
        diff.summary.modified_count, 0,
        "Whitespace and minification differences must be semantically zero"
    );
}

#[test]
fn test_adversarial_path_spacing_formatting() {
    let svg_spaced = r##"<svg xmlns="http://www.w3.org/2000/svg"><path id="p1" d="M 0 0 L 10 10 L 20 20 Z" /></svg>"##;
    let svg_compact =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><path id="p1" d="M0,0L10,10L20,20Z" /></svg>"##;

    let doc_a = parse_svg_document(svg_spaced);
    let doc_b = parse_svg_document(svg_compact);
    let diff = compute_semantic_diff(&doc_a, &doc_b);

    assert_eq!(
        diff.summary.modified_count, 0,
        "Compact vs spaced path data must be semantically identical"
    );
}

#[test]
fn test_adversarial_path_relative_vs_absolute() {
    let svg_abs =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><path id="p1" d="M 10 10 L 30 40" /></svg>"##;
    let svg_rel =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><path id="p1" d="M 10 10 l 20 30" /></svg>"##;

    let doc_a = parse_svg_document(svg_abs);
    let doc_b = parse_svg_document(svg_rel);
    let diff = compute_semantic_diff(&doc_a, &doc_b);

    assert_eq!(
        diff.summary.modified_count, 0,
        "Relative vs absolute path with identical geometry must be semantically unchanged"
    );
}

// =============================================================================
// 3. LARGE SVG STRESS TEST (1k, 10k objects & Diff Performance)
// =============================================================================

#[test]
fn test_adversarial_stress_1k_objects() {
    let count = 1_000;
    let mut svg1 = String::with_capacity(count * 150);
    svg1.push_str(r##"<svg xmlns="http://www.w3.org/2000/svg" width="5000" height="5000">"##);
    for i in 0..count {
        svg1.push_str(&format!(
            r##"<rect id="r_{i}" x="{}" y="{}" width="20" height="20" fill="#123456"/>"##,
            i % 100 * 30,
            (i / 100) * 30
        ));
    }
    svg1.push_str("</svg>");

    // Modify exactly 1 element in the middle
    let svg2 = svg1.replace(
        r##"id="r_500" x="0" y="150" width="20" height="20" fill="#123456""##,
        r##"id="r_500" x="0" y="150" width="40" height="40" fill="#ff0000""##,
    );

    let t0 = Instant::now();
    let doc1 = parse_svg_document(&svg1);
    let parse_time = t0.elapsed();

    let doc2 = parse_svg_document(&svg2);

    let t1 = Instant::now();
    let diff = compute_semantic_diff(&doc1, &doc2);
    let diff_time = t1.elapsed();

    println!("\n[Stress 1k Objects]");
    println!("  Parse Time: {:>8.3?}", parse_time);
    println!("  Diff Time:  {:>8.3?}", diff_time);
    println!(
        "  Modified: {}, Unchanged: {}",
        diff.summary.modified_count, diff.summary.unchanged_count
    );

    assert_eq!(diff.summary.modified_count, 1);
    assert_eq!(diff.summary.unchanged_count, count - 1);
    assert!(
        diff_time.as_millis() < 150,
        "1k objects diff should be under 150ms in debug mode"
    );
}

#[test]
fn test_adversarial_stress_10k_objects() {
    let count = 10_000;
    let mut svg1 = String::with_capacity(count * 150);
    svg1.push_str(r##"<svg xmlns="http://www.w3.org/2000/svg" width="10000" height="10000">"##);
    for i in 0..count {
        svg1.push_str(&format!(
            r##"<rect id="r_{i}" x="{}" y="{}" width="10" height="10" fill="#aabbcc"/>"##,
            i % 200 * 20,
            (i / 200) * 20
        ));
    }
    svg1.push_str("</svg>");

    let svg2 = svg1.replace(
        r##"id="r_5000" x="0" y="500" width="10" height="10" fill="#aabbcc""##,
        r##"id="r_5000" x="0" y="500" width="25" height="25" fill="#ff0000""##,
    );

    let t0 = Instant::now();
    let doc1 = parse_svg_document(&svg1);
    let parse_time = t0.elapsed();

    let doc2 = parse_svg_document(&svg2);

    let t1 = Instant::now();
    let diff = compute_semantic_diff(&doc1, &doc2);
    let diff_time = t1.elapsed();

    println!("\n[Stress 10k Objects]");
    println!("  Parse Time: {:>8.3?}", parse_time);
    println!("  Diff Time:  {:>8.3?}", diff_time);
    println!(
        "  Modified: {}, Unchanged: {}",
        diff.summary.modified_count, diff.summary.unchanged_count
    );

    assert_eq!(diff.summary.modified_count, 1);
    assert_eq!(diff.summary.unchanged_count, count - 1);
    assert!(
        diff_time.as_millis() < 600,
        "10k objects diff should be under 600ms in debug mode"
    );
}

// =============================================================================
// 4. TWO-WINDOW SAME-FILE EDIT (Process-isolated watcher instances)
// =============================================================================

#[test]
fn test_adversarial_two_window_same_file_edit() {
    let repo_dir = setup_adversarial_temp_repo("two_windows");
    let file_path = repo_dir.join("shared.svg");

    let initial_content =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r1" width="10"/></svg>"##;
    fs::write(&file_path, initial_content).unwrap();

    // Window A and Window B open the same file
    let mut watcher_a = FileWatcher::new(file_path.clone());
    watcher_a.mark_saved(initial_content);

    let mut watcher_b = FileWatcher::new(file_path.clone());
    watcher_b.mark_saved(initial_content);

    // Window A edits and saves
    let window_a_edit =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r1" width="20"/></svg>"##;
    fs::write(&file_path, window_a_edit).unwrap();
    watcher_a.mark_saved(window_a_edit); // Window A suppresses self-write loop

    // Window A check must be None (no loop)
    assert!(watcher_a.check_for_external_content().is_none());

    // Window B MUST detect Window A's save as an external change!
    let b_detected = watcher_b.check_for_external_content();
    assert!(
        b_detected.is_some(),
        "Window B must detect Window A's save!"
    );
    assert_eq!(b_detected.unwrap(), window_a_edit);
}

// =============================================================================
// 5. GIT RESTORE WITH UNRELATED DIRTY FILES
// =============================================================================

#[test]
fn test_adversarial_git_restore_with_unrelated_dirty_files() {
    let repo_dir = setup_adversarial_temp_repo("unrelated_dirty");
    let poster_file = repo_dir.join("poster.svg");
    let other_file = repo_dir.join("notes.txt");
    let untracked_file = repo_dir.join("untracked_scratch.svg");

    // Commit baseline for poster and notes
    fs::write(&poster_file, r##"<svg><text id="t">V1</text></svg>"##).unwrap();
    fs::write(&other_file, "Original Notes").unwrap();
    create_checkpoint(&poster_file, "Baseline").unwrap();
    // Commit other_file too
    Command::new("git")
        .args(["add", "notes.txt"])
        .current_dir(&repo_dir)
        .status()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Add notes"])
        .current_dir(&repo_dir)
        .status()
        .unwrap();

    // Now make dirty changes in other_file and create an untracked file
    fs::write(&other_file, "DIRTY UNSAVED NOTES - DO NOT TOUCH").unwrap();
    fs::write(&untracked_file, "<svg>IMPORTANT UNTRACKED FILE</svg>").unwrap();

    // External edit to poster.svg
    fs::write(
        &poster_file,
        r##"<svg><text id="t">V2 Broken</text></svg>"##,
    )
    .unwrap();

    // Restore ONLY poster.svg to HEAD
    restore_file_to_rev(&poster_file, "HEAD").expect("restore poster");

    // Verify poster was restored
    let poster_content = fs::read_to_string(&poster_file).unwrap();
    assert_eq!(poster_content, r##"<svg><text id="t">V1</text></svg>"##);

    // Verify other_file dirty changes were NOT touched or reverted!
    let other_content = fs::read_to_string(&other_file).unwrap();
    assert_eq!(
        other_content, "DIRTY UNSAVED NOTES - DO NOT TOUCH",
        "Unrelated dirty files must NOT be overwritten!"
    );

    // Verify untracked file was NOT deleted!
    assert!(
        untracked_file.exists(),
        "Untracked files must NOT be deleted!"
    );
}

// =============================================================================
// 6. DIRTY STATE RIGOR (Save, Undo back to save, Redo, Past-save)
// =============================================================================

struct MockEditCommand(&'static str);
impl HistoryCommand for MockEditCommand {
    fn execute(&mut self, _doc: &mut Document) {}
    fn undo(&mut self, _doc: &mut Document) {}
    fn name(&self) -> &str {
        self.0
    }
}

#[test]
fn test_adversarial_dirty_state_rigor() {
    let mut undo_manager = UndoManager::new();
    let mut doc = Document::default();

    // A. Initial state: clean
    assert!(
        !undo_manager.is_dirty(),
        "Initially document should be clean"
    );
    assert!(!undo_manager.can_undo());

    // B. First edit -> dirty
    undo_manager.execute(Box::new(MockEditCommand("Edit 1")), &mut doc);
    assert!(
        undo_manager.is_dirty(),
        "After edit, document must be dirty"
    );
    assert!(undo_manager.can_undo());

    // C. Save -> clean (Undo history is preserved, NOT cleared!)
    undo_manager.mark_saved();
    assert!(
        !undo_manager.is_dirty(),
        "After mark_saved, document must NOT be dirty"
    );
    assert!(
        undo_manager.can_undo(),
        "Undo history must be retained after save!"
    );

    // D. Second edit -> dirty
    undo_manager.execute(Box::new(MockEditCommand("Edit 2")), &mut doc);
    assert!(
        undo_manager.is_dirty(),
        "After second edit, document must be dirty"
    );

    // E. Undo back to saved state -> MUST BE CLEAN!
    let undid = undo_manager.undo(&mut doc);
    assert_eq!(undid.as_deref(), Some("Edit 2"));
    assert!(
        !undo_manager.is_dirty(),
        "Undoing back to the save-point must make the document clean!"
    );

    // F. Redo away from saved state -> MUST BE DIRTY AGAIN!
    let redid = undo_manager.redo(&mut doc);
    assert_eq!(redid.as_deref(), Some("Edit 2"));
    assert!(
        undo_manager.is_dirty(),
        "Redoing back away from saved point must make the document dirty!"
    );

    // G. Undo back to saved state -> Clean
    undo_manager.undo(&mut doc);
    assert!(!undo_manager.is_dirty());

    // H. Undo past saved state -> MUST BE DIRTY (document in memory now differs from saved disk state)!
    let undid_past = undo_manager.undo(&mut doc);
    assert_eq!(undid_past.as_deref(), Some("Edit 1"));
    assert!(
        undo_manager.is_dirty(),
        "Undoing prior to saved state makes document differ from disk, so must be dirty!"
    );

    // I. Redo back to saved state -> Clean again!
    undo_manager.redo(&mut doc);
    assert!(
        !undo_manager.is_dirty(),
        "Redoing back up to saved state must restore clean status!"
    );
}

// =============================================================================
// 7. CORRUPTED / INCOMPLETE SVG WRITE TIMEOUT
// =============================================================================

#[test]
fn test_adversarial_corrupted_svg_timeout() {
    let repo_dir = setup_adversarial_temp_repo("corrupted_timeout");
    let file_path = repo_dir.join("broken.svg");

    let initial_valid = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r1"/></svg>"##;
    fs::write(&file_path, initial_valid).unwrap();

    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(initial_valid);
    // Set a small partial write timeout for test speed
    watcher.set_partial_write_timeout(Duration::from_millis(50));

    // External tool writes an incomplete / corrupted SVG (missing closing tag, broken syntax)
    let corrupted_svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="broken" "##;
    // Sleep briefly to ensure filesystem mtime updates
    std::thread::sleep(Duration::from_millis(15));
    fs::write(&file_path, corrupted_svg).unwrap();

    // 1. Immediate check: Should return None because it grants a grace period for in-flight atomic writes
    let first_check = watcher.check_for_external_content();
    assert!(
        first_check.is_none(),
        "First check should defer incomplete write to allow atomic finish"
    );

    // 2. Wait for partial write timeout to expire
    std::thread::sleep(Duration::from_millis(70));

    // 3. Subsequent check: MUST NOT HANG OR SILENTLY IGNORE FOREVER!
    let timed_out_check = watcher.check_for_external_content();
    assert!(
        timed_out_check.is_some(),
        "After timeout expires, corrupted file must be reported to the editor!"
    );
    assert_eq!(timed_out_check.unwrap(), corrupted_svg);
}

// =============================================================================
// 8. RAPID SAVE BURST (30 saves/second)
// =============================================================================

#[test]
fn test_adversarial_rapid_save_burst_30_per_sec() {
    let repo_dir = setup_adversarial_temp_repo("rapid_burst");
    let file_path = repo_dir.join("burst.svg");

    let initial = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r" width="0"/></svg>"##;
    fs::write(&file_path, initial).unwrap();

    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(initial);

    let start = Instant::now();
    for i in 1..=30 {
        let content = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r" width="{}"/></svg>"##,
            i
        );
        fs::write(&file_path, &content).unwrap();
        watcher.mark_saved(&content);

        // Immediate check must never trigger a false-positive self-write loop
        assert!(
            watcher.check_for_external_content().is_none(),
            "Burst save {} falsely triggered external change loop",
            i
        );
    }
    let elapsed = start.elapsed();
    println!("\n[Rapid Save Burst 30x] Completed in {:?}", elapsed);
    assert!(
        elapsed < Duration::from_millis(1500),
        "30 saves burst took too long: {:?}",
        elapsed
    );
}

// =============================================================================
// 9. LARGE SVG STRESS TEST (50,000 objects)
// =============================================================================

#[test]
fn test_adversarial_stress_50k_objects() {
    let count = 50_000;
    println!("\n[Stress Test: Generating {} objects]", count);
    let gen_start = Instant::now();
    let mut doc_a = Document::default();
    let mut doc_b = Document::default();

    for i in 0..count {
        let id = format!("elem_{}", i);
        let obj_a = irasu_illustrator::core::document::Object {
            id: id.clone(),
            name: format!("Object {}", i),
            object_type: irasu_illustrator::core::document::ObjectType::Rectangle {
                width: 10.0,
                height: 10.0,
                corner_radius: 0.0,
            },
            transform: irasu_illustrator::core::document::Transform::default(),
            fill: None,
            stroke: None,
            shadow: None,
            glow: None,
            appearance: Default::default(),
            opacity: 1.0,
            blend_mode: irasu_illustrator::core::document::BlendMode::Normal,
            width_profile: None,
            visible: true,
            locked: false,
            auto_layout: None,
        };
        let mut obj_b = obj_a.clone();
        if i == count / 2 {
            // Modify exactly one object in the middle
            obj_b.opacity = 0.5;
        }
        doc_a.add_object(obj_a);
        doc_b.add_object(obj_b);
    }
    println!("  Generated 50k objects in {:?}", gen_start.elapsed());

    let diff_start = Instant::now();
    let diff = compute_semantic_diff(&doc_a, &doc_b);
    let diff_time = diff_start.elapsed();

    println!("  Semantic diff of 50,000 objects took: {:?}", diff_time);
    println!(
        "  Summary: Added={}, Removed={}, Modified={}, Unchanged={}",
        diff.summary.added_count,
        diff.summary.removed_count,
        diff.summary.modified_count,
        diff.summary.unchanged_count
    );

    assert_eq!(
        diff.summary.modified_count, 1,
        "Must detect exactly 1 modified object"
    );
    assert_eq!(diff.summary.unchanged_count, count - 1);
    // With O(N) lookup, 50k should take ~300-800ms, definitely < 3000ms.
    assert!(
        diff_time.as_millis() < 3000,
        "50k objects diff took {:?}, exceeding 3000ms limit! O(N^2) regression detected!",
        diff_time
    );
}

// =============================================================================
// 10. WATCHER WORKLOAD BENCHMARK (20KB, 1MB, 10MB, 50MB)
// =============================================================================

#[test]
fn test_adversarial_watcher_benchmark() {
    let repo_dir = setup_adversarial_temp_repo("watcher_bench");

    let sizes = [
        ("20KB", 20 * 1024),
        ("1MB", 1024 * 1024),
        ("10MB", 10 * 1024 * 1024),
        ("50MB", 50 * 1024 * 1024),
    ];

    println!("\n=== Watcher Polling Workload Benchmarks ===");
    for (label, size_bytes) in sizes {
        let file_path = repo_dir.join(format!("bench_{label}.svg"));
        let mut content = String::with_capacity(size_bytes + 200);
        content.push_str(r##"<svg xmlns="http://www.w3.org/2000/svg">"##);
        while content.len() < size_bytes {
            content.push_str(r##"<rect x="0" y="0" width="10" height="10"/>"##);
        }
        content.push_str("</svg>");
        fs::write(&file_path, &content).unwrap();

        let mut watcher = FileWatcher::new(file_path.clone());
        watcher.mark_saved(&content);

        // 1. Benchmark unchanged poll (Metadata fast-path)
        let iter_unchanged = 1000;
        let t_unchanged_start = Instant::now();
        for _ in 0..iter_unchanged {
            let res = watcher.check_for_external_content();
            assert!(res.is_none());
        }
        let dur_unchanged = t_unchanged_start.elapsed() / iter_unchanged;

        // 2. Benchmark change detection (Disk read + Hashing)
        // Simulate an external modification
        std::thread::sleep(Duration::from_millis(15));
        let mut modified_content = content.clone();
        modified_content.push_str("<!-- modified -->");
        fs::write(&file_path, &modified_content).unwrap();

        let t_changed_start = Instant::now();
        let res = watcher.check_for_external_content();
        let dur_changed = t_changed_start.elapsed();
        assert!(res.is_some());

        println!(
            "  Size {:>5}: Unchanged poll = {:>8.3?}/check | Changed poll (read+hash) = {:>8.3?}",
            label, dur_unchanged, dur_changed
        );
    }
}

// =============================================================================
// 11. STAT-FAST-PATH EDGE CASE: SAME SIZE + SAME MTIME CONTENT CHANGE
// =============================================================================

#[test]
fn test_adversarial_same_size_same_mtime_periodic_verify() {
    let repo_dir = setup_adversarial_temp_repo("same_size_mtime");
    let file_path = repo_dir.join("color_replace.svg");

    // Test A: Exact same byte length string substitution
    let initial =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r1" fill="#112233"/></svg>"##;
    let modified =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r1" fill="#445566"/></svg>"##;
    assert_eq!(
        initial.len(),
        modified.len(),
        "Byte lengths must match exactly"
    );

    fs::write(&file_path, initial).unwrap();
    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(initial);
    // Configure a short periodic verification interval for testing (50ms)
    watcher.set_periodic_verify_interval(Duration::from_millis(50));

    // Overwrite content with modified (same byte length)
    fs::write(&file_path, modified).unwrap();

    // Test B & C: Simulate identical mtime collision by syncing watcher's cached mtime with current disk mtime
    let disk_mtime = fs::metadata(&file_path).unwrap().modified().ok();
    watcher.last_modified = disk_mtime;
    watcher.last_file_size = modified.len() as u64;

    // Immediately after write, if periodic interval hasn't elapsed, stat fast-path sees identical mtime & size:
    // When periodic verify interval elapses, watcher MUST verify content hash and detect change!
    std::thread::sleep(Duration::from_millis(60));
    let detected = watcher.check_for_external_content();
    assert!(
        detected.is_some(),
        "Periodic verification must catch content change even when mtime and size collide!"
    );
    assert_eq!(detected.unwrap(), modified);
}

// =============================================================================
// 12. SLOW WRITING SVG IDLE TIMEOUT SEMANTICS
// =============================================================================

#[test]
fn test_adversarial_slow_writing_svg_idle_timeout() {
    let repo_dir = setup_adversarial_temp_repo("slow_write_idle");
    let file_path = repo_dir.join("slow.svg");

    let initial = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="init"/></svg>"##;
    fs::write(&file_path, initial).unwrap();

    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(initial);
    watcher.set_partial_write_timeout(Duration::from_millis(200));

    // Case A: External tool writes slowly in 4 chunks across 240ms (80ms interval < 200ms idle timeout)
    // 0ms: chunk 1
    fs::write(&file_path, r##"<svg xmlns="http://www.w3.org/2000/svg">"##).unwrap();
    assert!(
        watcher.check_for_external_content().is_none(),
        "Chunk 1 in-flight should defer"
    );

    std::thread::sleep(Duration::from_millis(80));
    // 80ms: chunk 2
    fs::write(
        &file_path,
        r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="c1" "##,
    )
    .unwrap();
    assert!(
        watcher.check_for_external_content().is_none(),
        "Chunk 2 actively changing should defer"
    );

    std::thread::sleep(Duration::from_millis(80));
    // 160ms: chunk 3
    fs::write(
        &file_path,
        r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="c1" width="10"/>"##,
    )
    .unwrap();
    assert!(
        watcher.check_for_external_content().is_none(),
        "Chunk 3 actively changing should defer"
    );

    std::thread::sleep(Duration::from_millis(80));
    // 240ms: chunk 4 completes valid SVG
    let complete_svg =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="c1" width="10"/></svg>"##;
    fs::write(&file_path, complete_svg).unwrap();

    // Now check: must be recognized as valid external content without premature broken error!
    let detected = watcher.check_for_external_content();
    assert_eq!(
        detected,
        Some(complete_svg.to_string()),
        "Slow-writing valid SVG must be accepted cleanly"
    );

    // Case B: External tool writes an incomplete chunk and STOPS
    watcher.mark_saved(complete_svg);
    fs::write(&file_path, r##"<svg><path d="M 0 0"##).unwrap();
    assert!(
        watcher.check_for_external_content().is_none(),
        "First tick should defer incomplete write"
    );

    // Wait past idle timeout (200ms timeout + buffer)
    std::thread::sleep(Duration::from_millis(250));
    let broken = watcher.check_for_external_content();
    assert!(
        broken.is_some(),
        "After file is idle for >200ms and still incomplete, error must be emitted"
    );
    assert_eq!(broken.unwrap(), r##"<svg><path d="M 0 0"##);

    // Case C: Broken file recovers to valid
    let fixed_svg = r##"<svg><path d="M 0 0"/></svg>"##;
    fs::write(&file_path, fixed_svg).unwrap();
    let recovered = watcher.check_for_external_content();
    assert_eq!(
        recovered,
        Some(fixed_svg.to_string()),
        "Recovered valid SVG must be accepted"
    );
}

// =============================================================================
// 13. KEEP LOCAL PRESERVES EXTERNAL VERSION (No silent data loss)
// =============================================================================

#[test]
fn test_adversarial_keep_local_preserves_external_version() {
    let repo_dir = setup_adversarial_temp_repo("keep_local_preservation");
    let file_path = repo_dir.join("poster.svg");

    let baseline_v1 =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><text id="t">V1 Baseline</text></svg>"##;
    fs::write(&file_path, baseline_v1).unwrap();
    create_checkpoint(&file_path, "Initial commit").unwrap();

    // External AI writes V_ext to disk
    let ext_svg =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><text id="t">V_EXT from AI</text></svg>"##;
    fs::write(&file_path, ext_svg).unwrap();

    // User chose "Keep Local": Before local overwrite, external version MUST be preserved
    let backup_result = preserve_external_version(&file_path, ext_svg);
    assert!(
        backup_result.is_ok(),
        "External version backup must succeed"
    );
    let backup_path = backup_result.unwrap();

    // Verify backup file exists and holds external content
    assert!(backup_path.exists());
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, ext_svg);

    // Now simulate local overwrite
    let local_svg =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><text id="t">V_LOCAL Unsaved</text></svg>"##;
    fs::write(&file_path, local_svg).unwrap();

    // Verify disk has local SVG, but external SVG remains safe and recoverable
    assert_eq!(fs::read_to_string(&file_path).unwrap(), local_svg);
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        ext_svg,
        "External version must be recoverable from backup!"
    );
}

// =============================================================================
// 14. TRUE EXTERNAL RAPID SAVE TEST (Threaded external writer)
// =============================================================================

#[test]
fn test_adversarial_true_external_rapid_save_threaded() {
    let repo_dir = setup_adversarial_temp_repo("true_rapid_save");
    let file_path = repo_dir.join("live.svg");

    let initial = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r" width="0"/></svg>"##;
    fs::write(&file_path, initial).unwrap();

    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(initial);

    // Spawn a true independent external writer thread
    let target_path = file_path.clone();
    let handle = std::thread::spawn(move || {
        for i in 1..=30 {
            let content = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r" width="{}"/></svg>"##,
                i
            );
            fs::write(&target_path, &content).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
    });

    // Wait for external writer to finish
    handle.join().unwrap();

    // Final state on disk is v30
    let final_expected =
        r##"<svg xmlns="http://www.w3.org/2000/svg"><rect id="r" width="30"/></svg>"##;

    // Watcher check MUST reliably detect external modification and yield the final state v30
    let detected = watcher.check_for_external_content();
    assert!(
        detected.is_some(),
        "Watcher must detect external rapid writes"
    );
    assert_eq!(
        detected.unwrap(),
        final_expected,
        "Watcher must observe the latest final state, never stale intermediate"
    );
}

// =============================================================================
// 15. AMBIGUOUS NEAR-MATCH SCENARIO TEST
// =============================================================================

#[test]
fn test_adversarial_ambiguous_near_matches() {
    let doc_a_svg = r##"<svg xmlns="http://www.w3.org/2000/svg">
        <rect x="10" y="10" width="10" height="10" fill="#f00"/>
        <rect x="10" y="12" width="10" height="10" fill="#0f0"/>
    </svg>"##;

    let doc_b_svg = r##"<svg xmlns="http://www.w3.org/2000/svg">
        <rect x="10" y="11" width="10" height="10" fill="#f00"/>
        <rect x="10" y="13" width="10" height="10" fill="#0f0"/>
    </svg>"##;

    let doc_a = parse_svg_document(doc_a_svg);
    let doc_b = parse_svg_document(doc_b_svg);

    let diff = compute_semantic_diff(&doc_a, &doc_b);
    println!("\n[Ambiguous Near Matches]");
    println!(
        "  Summary: Added={}, Removed={}, Modified={}, Unchanged={}",
        diff.summary.added_count,
        diff.summary.removed_count,
        diff.summary.modified_count,
        diff.summary.unchanged_count
    );

    // Both should be matched as modified (each moved by 1px) with 0 orphaned additions/deletions
    assert_eq!(diff.summary.added_count, 0);
    assert_eq!(diff.summary.removed_count, 0);
    assert_eq!(diff.summary.modified_count, 2);
}
