use irasu_illustrator::core::diff::compute_semantic_diff;
use irasu_illustrator::core::document::{Document, Object};
use irasu_illustrator::io::git::{
    create_checkpoint, get_file_commit_history, get_file_content_at_rev, is_git_repository,
    restore_file_to_rev,
};
use irasu_illustrator::io::svg::{export_svg, parse_svg_document};
use std::fs;
use std::process::Command;

fn setup_temp_git_repo(prefix: &str) -> std::path::PathBuf {
    let unique_id = uuid::Uuid::new_v4().to_string();
    let repo_dir = std::env::temp_dir().join(format!("amata_git_test_{prefix}_{unique_id}"));
    let _ = fs::remove_dir_all(&repo_dir);
    fs::create_dir_all(&repo_dir).expect("Failed to create temp repo dir");

    // Initialize git
    let status = Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .status()
        .expect("Failed to run git init");
    assert!(status.success());

    // Configure user name and email locally for CI portability
    Command::new("git")
        .args(["config", "user.name", "Amata CI Tester"])
        .current_dir(&repo_dir)
        .status()
        .expect("git config user.name");
    Command::new("git")
        .args(["config", "user.email", "test@amata.local"])
        .current_dir(&repo_dir)
        .status()
        .expect("git config user.email");

    repo_dir
}

#[test]
fn test_git_checkpoint_history_restore_workflow() {
    let repo_dir = setup_temp_git_repo("workflow");
    assert!(is_git_repository(&repo_dir));

    let file_path = repo_dir.join("poster.svg");

    // 1. Create initial SVG
    let mut doc_v1 = Document::default();
    doc_v1.name = "Poster".to_string();
    let mut text_v1 = Object::new_text("Title", "HIRARI SOUND", 100.0, 100.0, 48.0);
    text_v1.id = "headline".to_string();
    doc_v1.add_object(text_v1);

    let svg_v1 = export_svg(&doc_v1);
    fs::write(&file_path, &svg_v1).expect("write v1");

    // 2. Initial checkpoint
    let cp1_hash = create_checkpoint(&file_path, "Initial draft").expect("checkpoint 1");
    assert!(!cp1_hash.is_empty());

    // 3. Modify SVG in Amata
    let mut doc_v2 = parse_svg_document(&svg_v1);
    // Change title and add artist
    for (_, obj) in doc_v2.all_objects_mut() {
        if obj.id == "headline" {
            if let irasu_illustrator::core::document::ObjectType::Text {
                text, font_size, ..
            } = &mut obj.object_type
            {
                *text = "HIRARI SOUND 2026".to_string();
                *font_size = 64.0;
            }
        }
    }
    let mut artist_obj = Object::new_text("Artist", "Special Guest", 100.0, 300.0, 24.0);
    artist_obj.id = "guest-1".to_string();
    doc_v2.add_object(artist_obj);

    let svg_v2 = export_svg(&doc_v2);
    fs::write(&file_path, &svg_v2).expect("write v2");

    // 4. Semantic diff check between HEAD and current disk file
    let head_content = get_file_content_at_rev(&file_path, "HEAD").expect("get HEAD content");
    let doc_head = parse_svg_document(&head_content);
    let diff = compute_semantic_diff(&doc_head, &doc_v2);

    assert_eq!(diff.summary.modified_count, 1, "headline was modified");
    assert_eq!(diff.summary.added_count, 1, "guest-1 was added");

    // 5. Second checkpoint
    let cp2_hash =
        create_checkpoint(&file_path, "Updated title and added guest").expect("checkpoint 2");
    assert!(!cp2_hash.is_empty());

    // 6. Inspect history
    let history = get_file_commit_history(&file_path, 10).expect("get history");
    assert_eq!(history.len(), 2, "Expected 2 commits in history");
    assert_eq!(history[0].message, "Updated title and added guest");
    assert_eq!(history[1].message, "Initial draft");

    // 7. Restore to initial draft
    restore_file_to_rev(&file_path, &history[1].hash).expect("restore to v1");

    // 8. Verify restored content matches v1
    let restored_svg = fs::read_to_string(&file_path).expect("read restored");
    let restored_doc = parse_svg_document(&restored_svg);
    let diff_after_restore = compute_semantic_diff(&doc_v1, &restored_doc);
    assert!(
        diff_after_restore.is_empty(),
        "Restored doc should match doc_v1 identically"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&repo_dir);
}
