#![allow(clippy::field_reassign_with_default)]
use irasu_illustrator::core::diff::compute_semantic_diff;
use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::watcher::FileWatcher;
use irasu_illustrator::io::git::{create_checkpoint, get_file_commit_history, restore_file_to_rev};
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};
use std::fs;
use std::process::Command;
use std::time::Instant;

fn setup_temp_repo(prefix: &str) -> std::path::PathBuf {
    let unique_id = uuid::Uuid::new_v4().to_string();
    let repo_dir = std::env::temp_dir().join(format!("amata_ai_test_{prefix}_{unique_id}"));
    let _ = fs::remove_dir_all(&repo_dir);
    fs::create_dir_all(&repo_dir).expect("Failed to create temp repo dir");

    let status = Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .status()
        .expect("git init");
    assert!(status.success());

    Command::new("git")
        .args(["config", "user.name", "Amata AI Tester"])
        .current_dir(&repo_dir)
        .status()
        .expect("git config user.name");
    Command::new("git")
        .args(["config", "user.email", "ai_tester@amata.local"])
        .current_dir(&repo_dir)
        .status()
        .expect("git config user.email");

    repo_dir
}

// -----------------------------------------------------------------------------
// 1. Clean External Edit Detected
// -----------------------------------------------------------------------------
#[test]
fn test_clean_external_edit_detected() {
    let repo_dir = setup_temp_repo("clean_edit");
    let file_path = repo_dir.join("poster.svg");

    let mut doc = Document::default();
    doc.name = "Poster".to_string();
    let text = Object::new_text("headline", "HIRARI SOUND 2026", 100.0, 100.0, 48.0);
    doc.add_object(text);
    let original_svg = export_svg(&doc);
    fs::write(&file_path, &original_svg).unwrap();

    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(&original_svg);

    // Initial check -> no external modification
    assert!(watcher.check_for_external_content().is_none());

    // External process edits SVG
    let modified_svg = original_svg.replace("48", "58");
    fs::write(&file_path, &modified_svg).unwrap();

    // Watcher detects the external change
    let detected = watcher.check_for_external_content();
    assert!(detected.is_some());
    assert_eq!(detected.unwrap(), modified_svg);
}

// -----------------------------------------------------------------------------
// 2. Same-Size External Edit Detected (Content Hash verification)
// -----------------------------------------------------------------------------
#[test]
fn test_same_size_external_edit_detected() {
    let repo_dir = setup_temp_repo("same_size");
    let file_path = repo_dir.join("poster.svg");

    // Two SVGs with exactly identical byte lengths but different content
    let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect id="r1" fill="#111111"/></svg>"##;
    let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect id="r1" fill="#999999"/></svg>"##;
    assert_eq!(
        svg1.len(),
        svg2.len(),
        "Byte lengths must be identical for this test"
    );

    fs::write(&file_path, svg1).unwrap();
    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(svg1);

    assert!(watcher.check_for_external_content().is_none());

    // Write same-size alternative content
    fs::write(&file_path, svg2).unwrap();

    let detected = watcher.check_for_external_content();
    assert!(
        detected.is_some(),
        "Content hash must detect same-size changes"
    );
    assert_eq!(detected.unwrap(), svg2);
}

// -----------------------------------------------------------------------------
// 3. External Edit -> Semantic Diff
// -----------------------------------------------------------------------------
#[test]
fn test_external_edit_semantic_diff() {
    let svg_before = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000">
  <text id="headline" x="100" y="200" font-size="96">HIRARI SOUND 2026</text>
  <path id="waveform" d="M 0 500 Q 250 300 500 500" stroke="#00ffff" />
</svg>"##;

    // AI increases font size by 20% (96 * 1.2 = 115.2) without touching waveform
    let svg_after = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000">
  <text id="headline" x="100" y="200" font-size="115.2">HIRARI SOUND 2026</text>
  <path id="waveform" d="M 0 500 Q 250 300 500 500" stroke="#00ffff" />
</svg>"##;

    let doc_before = parse_svg_document(svg_before);
    let doc_after = parse_svg_document(svg_after);

    let diff = compute_semantic_diff(&doc_before, &doc_after);
    assert_eq!(diff.summary.modified_count, 1);
    assert_eq!(diff.summary.added_count, 0);
    assert_eq!(diff.summary.removed_count, 0);

    let changed = &diff.objects[0];
    assert_eq!(changed.id, "headline");
    assert_eq!(diff.count_text_changes(), 1);
    assert_eq!(diff.count_geometry_changes(), 0);
}

// -----------------------------------------------------------------------------
// 4. External Edit -> Accept Workflow
// -----------------------------------------------------------------------------
#[test]
fn test_external_edit_accept() {
    let repo_dir = setup_temp_repo("accept_flow");
    let file_path = repo_dir.join("poster.svg");

    let initial_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500"><text id="title">Version 1</text></svg>"##;
    fs::write(&file_path, initial_svg).unwrap();

    let mut current_doc = parse_svg_document(initial_svg);
    assert!(current_doc.all_objects().any(|(_, o)| o.id == "title"));
    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(initial_svg);

    // AI makes an edit
    let ai_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500"><text id="title">Version 2 (AI Edited)</text></svg>"##;
    fs::write(&file_path, ai_svg).unwrap();

    let detected = watcher
        .check_for_external_content()
        .expect("must detect AI edit");
    let external_doc = parse_svg_document(&detected);

    // Simulate [ Accept ]
    current_doc = external_doc;
    watcher.mark_saved(&detected);

    // Verify state
    let (_, title_obj) = current_doc
        .all_objects()
        .find(|(_, o)| o.id == "title")
        .unwrap();
    if let ObjectType::Text { ref text, .. } = title_obj.object_type {
        assert_eq!(text, "Version 2 (AI Edited)");
    } else {
        panic!("expected text object");
    }

    // Subsequent tick has no false-positive triggers
    assert!(watcher.check_for_external_content().is_none());
}

// -----------------------------------------------------------------------------
// 5. External Edit -> Revert Workflow
// -----------------------------------------------------------------------------
#[test]
fn test_external_edit_revert() {
    let repo_dir = setup_temp_repo("revert_flow");
    let file_path = repo_dir.join("poster.svg");

    let original_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500"><text id="title">Original Master</text></svg>"##;
    fs::write(&file_path, original_svg).unwrap();

    let mut current_doc = parse_svg_document(original_svg);
    assert!(current_doc.all_objects().any(|(_, o)| o.id == "title"));
    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(original_svg);
    let pre_edit_svg = original_svg.to_string();

    // AI edits the file
    let bad_ai_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500"><text id="title">Unwanted AI Modification</text></svg>"##;
    fs::write(&file_path, bad_ai_svg).unwrap();

    let _ = watcher
        .check_for_external_content()
        .expect("detected bad edit");

    // Simulate [ Revert ]
    fs::write(&file_path, &pre_edit_svg).expect("restore disk file");
    current_doc = parse_svg_document(&pre_edit_svg);
    watcher.mark_saved(&pre_edit_svg);

    // Verify disk content is byte-for-byte restored
    let disk_now = fs::read_to_string(&file_path).unwrap();
    assert_eq!(disk_now, original_svg);

    // Verify doc is restored
    let (_, title_obj) = current_doc
        .all_objects()
        .find(|(_, o)| o.id == "title")
        .unwrap();
    if let ObjectType::Text { ref text, .. } = title_obj.object_type {
        assert_eq!(text, "Original Master");
    }

    // Verify watcher loop prevention on revert
    assert!(watcher.check_for_external_content().is_none());
}

// -----------------------------------------------------------------------------
// 6. Watcher Self-Write Loop Prevention
// -----------------------------------------------------------------------------
#[test]
fn test_watcher_self_write_loop_prevention() {
    let repo_dir = setup_temp_repo("loop_prevention");
    let file_path = repo_dir.join("poster.svg");

    let doc = Document::default();
    let svg = export_svg(&doc);
    fs::write(&file_path, &svg).unwrap();

    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(&svg);

    // Internal save 1
    let svg_v2 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"></svg>"##;
    fs::write(&file_path, svg_v2).unwrap();
    watcher.mark_saved(svg_v2);

    // Check must return None
    assert!(watcher.check_for_external_content().is_none());

    // Internal save 2
    let svg_v3 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="300"></svg>"##;
    fs::write(&file_path, svg_v3).unwrap();
    watcher.mark_saved(svg_v3);

    assert!(watcher.check_for_external_content().is_none());
}

// -----------------------------------------------------------------------------
// 7. Unsaved Local Edit + External Edit Conflict
// -----------------------------------------------------------------------------
#[test]
fn test_unsaved_local_edit_conflict() {
    let local_edited = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500"><text id="t1">Local Work In Progress</text></svg>"##;
    let disk_external = r##"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500"><text id="t1">AI Parallel Edit</text></svg>"##;

    let doc_local = parse_svg_document(local_edited);
    let doc_disk = parse_svg_document(disk_external);

    // Semantic diff between Local Document vs Disk Version
    let conflict_diff = compute_semantic_diff(&doc_local, &doc_disk);
    assert_eq!(conflict_diff.summary.modified_count, 1);
    assert_eq!(conflict_diff.objects[0].id, "t1");
}

// -----------------------------------------------------------------------------
// 8. Invalid Temporary SVG -> Graceful Recovery
// -----------------------------------------------------------------------------
#[test]
fn test_invalid_temporary_svg_recovery() {
    let repo_dir = setup_temp_repo("invalid_temp");
    let file_path = repo_dir.join("poster.svg");

    let initial = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"></svg>"##;
    fs::write(&file_path, initial).unwrap();

    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(initial);

    // External process begins non-atomic partial write (unclosed SVG tag)
    fs::write(
        &file_path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">"##,
    )
    .unwrap();

    // Watcher must defer / return None instead of panic or emitting partial error
    let in_flight = watcher.check_for_external_content();
    assert!(in_flight.is_none(), "Incomplete write must be deferred");

    // External process completes writing
    let full_valid = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"><rect id="r1"/></svg>"##;
    fs::write(&file_path, full_valid).unwrap();

    // Now watcher detects valid content
    let completed = watcher.check_for_external_content();
    assert!(completed.is_some());
    assert_eq!(completed.unwrap(), full_valid);
}

// -----------------------------------------------------------------------------
// 9. Atomic Save (Temp File -> Rename)
// -----------------------------------------------------------------------------
#[test]
fn test_atomic_save_detection() {
    let repo_dir = setup_temp_repo("atomic_save");
    let target_file = repo_dir.join("poster.svg");
    let temp_file = repo_dir.join("poster.svg.tmp");

    let v1 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"></svg>"##;
    fs::write(&target_file, v1).unwrap();

    let mut watcher = FileWatcher::new(target_file.clone());
    watcher.mark_saved(v1);

    // External tool writes to temp file then renames atomically over target
    let v2 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><circle id="c1" r="50"/></svg>"##;
    fs::write(&temp_file, v2).unwrap();
    fs::rename(&temp_file, &target_file).expect("atomic rename");

    let detected = watcher.check_for_external_content();
    assert!(detected.is_some(), "Watcher must detect atomic replacement");
    assert_eq!(detected.unwrap(), v2);
}

// -----------------------------------------------------------------------------
// 10. Rapid Repeated External Edits
// -----------------------------------------------------------------------------
#[test]
fn test_rapid_repeated_external_edits() {
    let repo_dir = setup_temp_repo("rapid_edits");
    let file_path = repo_dir.join("poster.svg");

    let initial = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"></svg>"##;
    fs::write(&file_path, initial).unwrap();

    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(initial);

    for i in 1..=5 {
        let content = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><text id="counter">{i}</text></svg>"##
        );
        fs::write(&file_path, &content).unwrap();

        let detected = watcher
            .check_for_external_content()
            .expect("must detect rapid edit");
        assert_eq!(detected, content);
        watcher.mark_saved(&content);
    }
}

// -----------------------------------------------------------------------------
// 11. Delete and Recreate
// -----------------------------------------------------------------------------
#[test]
fn test_delete_and_recreate() {
    let repo_dir = setup_temp_repo("delete_recreate");
    let file_path = repo_dir.join("poster.svg");

    let v1 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"></svg>"##;
    fs::write(&file_path, v1).unwrap();

    let mut watcher = FileWatcher::new(file_path.clone());
    watcher.mark_saved(v1);

    // Delete file
    fs::remove_file(&file_path).unwrap();
    assert!(
        watcher.check_for_external_content().is_none(),
        "Should not error when file missing"
    );

    // Recreate file with new content
    let v2 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"><rect id="recreated"/></svg>"##;
    fs::write(&file_path, v2).unwrap();

    let detected = watcher.check_for_external_content();
    assert!(detected.is_some(), "Must detect recreated file");
    assert_eq!(detected.unwrap(), v2);
}

// -----------------------------------------------------------------------------
// 12. Git Checkpoint Before External Edit & Clean History
// -----------------------------------------------------------------------------
#[test]
fn test_git_checkpoint_before_ai_edit() {
    let repo_dir = setup_temp_repo("git_checkpoint_hygiene");
    let file_path = repo_dir.join("poster.svg");

    let master_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="1200">
  <text id="headline" font-size="72">HIRARI SOUND</text>
</svg>"##;
    fs::write(&file_path, master_svg).unwrap();

    // 1. User / GUI creates pre-AI checkpoint
    let cp_hash =
        create_checkpoint(&file_path, "🛡️ Before AI typography edit").expect("checkpoint creation");
    assert!(!cp_hash.is_empty());

    // 2. AI edits file
    let ai_svg = master_svg.replace("72", "86.4");
    fs::write(&file_path, &ai_svg).unwrap();

    // 3. Verify history hygiene: exactly 1 commit recorded, not spammed
    let history = get_file_commit_history(&file_path, 10).expect("history");
    assert_eq!(history.len(), 1);
    assert!(history[0].message.contains("Before AI typography edit"));

    // 4. Restore to pre-AI checkpoint
    restore_file_to_rev(&file_path, "HEAD").expect("restore to checkpoint");
    let restored = fs::read_to_string(&file_path).unwrap();
    assert_eq!(
        restored, master_svg,
        "Byte-for-byte exact restoration from Git checkpoint"
    );
}

// -----------------------------------------------------------------------------
// 20. HIRARI SOUND REGRESSION BENCHMARK SUITE
// -----------------------------------------------------------------------------
#[test]
fn test_hirari_sound_benchmark_regression() {
    let fixture_path = std::path::Path::new("benchmark/hirari_direct_v1.svg");
    let original_svg = fs::read_to_string(fixture_path).expect("hirari_direct_v1.svg must exist");
    let original_doc = parse_svg_document(&original_svg);

    // --- TEST A: Title Only +20% ---
    // Change HIRARI font-size="132" to "158.4"
    let title_edited_svg = original_svg.replace(r#"font-size="132""#, r#"font-size="158.4""#);
    assert_ne!(title_edited_svg, original_svg);

    let doc_title_edit = parse_svg_document(&title_edited_svg);
    let diff_a = compute_semantic_diff(&original_doc, &doc_title_edit);

    assert_eq!(
        diff_a.summary.modified_count, 1,
        "Expected ONLY title to be modified"
    );
    assert_eq!(diff_a.summary.added_count, 0);
    assert_eq!(diff_a.summary.removed_count, 0);
    assert_eq!(diff_a.count_text_changes(), 1);
    assert_eq!(diff_a.count_geometry_changes(), 0);

    // --- TEST B: Title + Accidental Waveform Edit (Negative Test) ---
    // AI accidentally changes both title and a path in ambient soundwaves
    let accidental_svg =
        title_edited_svg.replace("C 240,520 400,920 595.5,720", "C 240,320 400,920 595.5,720");
    assert_ne!(
        accidental_svg, title_edited_svg,
        "Accidental waveform replacement must succeed"
    );
    let doc_accidental = parse_svg_document(&accidental_svg);
    let diff_b = compute_semantic_diff(&original_doc, &doc_accidental);

    assert_eq!(
        diff_b.summary.modified_count, 2,
        "Semantic diff MUST catch accidental waveform modification"
    );
    assert_eq!(diff_b.count_text_changes(), 1);
    assert_eq!(diff_b.count_geometry_changes(), 1);

    // --- TEST C: Revert -> Original SVG Exact Recovery ---
    let reverted_doc = parse_svg_document(&original_svg);
    let diff_c = compute_semantic_diff(&original_doc, &reverted_doc);
    assert_eq!(diff_c.summary.modified_count, 0);
    assert_eq!(diff_c.summary.added_count, 0);
    assert_eq!(diff_c.summary.removed_count, 0);

    // --- TEST D: Add 2 Artists ---
    let mut added_artists_svg = original_svg.clone();
    let insert_pos = added_artists_svg.find("</svg>").unwrap();
    let new_artists = r##"
    <text id="artist-new-1" x="200" y="900" font-size="18">DJ SOLARIS</text>
    <text id="artist-new-2" x="200" y="930" font-size="18">ECHO DRIFT</text>
"##;
    added_artists_svg.insert_str(insert_pos, new_artists);

    let doc_artists = parse_svg_document(&added_artists_svg);
    let diff_d = compute_semantic_diff(&original_doc, &doc_artists);
    assert_eq!(
        diff_d.summary.added_count, 2,
        "Must detect exactly 2 added artists"
    );

    // --- TEST E: Palette Change ---
    // Change radial gradient stop color
    let palette_svg =
        original_svg.replace(r##"stop-color="#00f5ff""##, r##"stop-color="#ff00aa""##);
    let doc_palette = parse_svg_document(&palette_svg);
    let diff_e = compute_semantic_diff(&original_doc, &doc_palette);
    // Gradient definitions changed without geometry modifications
    assert_eq!(diff_e.count_geometry_changes(), 0);
}

// -----------------------------------------------------------------------------
// 21. PERFORMANCE MEASUREMENTS
// -----------------------------------------------------------------------------
#[test]
fn test_hirari_sound_performance_metrics() {
    let fixture_path = std::path::Path::new("benchmark/hirari_direct_v1.svg");
    let original_svg = fs::read_to_string(fixture_path).expect("hirari_direct_v1.svg must exist");

    // 1. Reload Time
    let t0 = Instant::now();
    let doc1 = parse_svg_document(&original_svg);
    let reload_duration = t0.elapsed();

    // 2. Modified SVG
    let modified_svg = original_svg.replace(r#"font-size="132""#, r#"font-size="158.4""#);
    let doc2 = parse_svg_document(&modified_svg);

    // 3. Semantic Diff Time
    let t1 = Instant::now();
    let diff = compute_semantic_diff(&doc1, &doc2);
    let diff_duration = t1.elapsed();

    // 4. File Watcher Latency (Hashing & Check)
    let temp_dir = setup_temp_repo("perf");
    let test_file = temp_dir.join("perf.svg");
    fs::write(&test_file, &original_svg).unwrap();
    let mut watcher = FileWatcher::new(test_file.clone());
    watcher.mark_saved(&original_svg);

    fs::write(&test_file, &modified_svg).unwrap();
    let t2 = Instant::now();
    let detected = watcher.check_for_external_content();
    let watcher_duration = t2.elapsed();
    assert!(detected.is_some());

    // 5. Revert Time (write + parse + hash update)
    let t3 = Instant::now();
    fs::write(&test_file, &original_svg).unwrap();
    let _restored_doc = parse_svg_document(&original_svg);
    watcher.mark_saved(&original_svg);
    let revert_duration = t3.elapsed();

    println!("\n=======================================================");
    println!("⏱️  AMATA AI EDITING WORKFLOW PERFORMANCE REPORT");
    println!("=======================================================");
    println!("• Document Reload Time:   {:>8.3?}", reload_duration);
    println!("• Semantic Diff Time:     {:>8.3?}", diff_duration);
    println!("• External Watcher Check: {:>8.3?}", watcher_duration);
    println!("• Full Revert Time:       {:>8.3?}", revert_duration);
    println!(
        "• Total Changes Detected: Modified={}",
        diff.summary.modified_count
    );
    println!("=======================================================\n");

    // Strict performance assertions
    assert!(
        reload_duration.as_millis() < 50,
        "Document reload should be under 50ms"
    );
    assert!(
        diff_duration.as_millis() < 30,
        "Semantic diff should be under 30ms"
    );
    assert!(
        watcher_duration.as_millis() < 10,
        "Watcher check should be under 10ms"
    );
    assert!(
        revert_duration.as_millis() < 50,
        "Full revert should be under 50ms"
    );
}
