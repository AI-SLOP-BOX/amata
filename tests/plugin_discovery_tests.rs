//! `amata plugins` used to answer with a "not yet implemented" placeholder.
//! These tests cover the discovery scan, header metadata parsing, and the CLI
//! reporting path.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::cli::{run_cli, Cli, Commands};
use irasu_illustrator::plugin::discovery::{discover_in, find_plugin, parse_metadata};
use std::path::PathBuf;

const PLUGIN_WITH_METADATA: &str = r#"// @name: Poster Grid
// @id: poster-grid
// @version: 1.2.0
// @author: Amata Contributors
// @description: Repeats the selection on a grid
let g = grid(2, 2, 10.0, 10.0, 1.0, 1.0);
#{ width: 100.0, height: 100.0, objects: [] }
"#;

const PLUGIN_WITHOUT_METADATA: &str = r#"// A bare script with no header block.
#{ width: 100.0, height: 100.0, objects: [] }
"#;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_parse_metadata_header_and_defaults() {
    let meta = parse_metadata(PLUGIN_WITH_METADATA, "ignored-stem");
    assert_eq!(meta.id, "poster-grid");
    assert_eq!(meta.name, "Poster Grid");
    assert_eq!(meta.version, "1.2.0");
    assert_eq!(meta.author, "Amata Contributors");
    assert_eq!(meta.description, "Repeats the selection on a grid");

    // No header: fall back to the file stem and placeholder version/author.
    let bare = parse_metadata(PLUGIN_WITHOUT_METADATA, "bare-plugin");
    assert_eq!(bare.id, "bare-plugin");
    assert_eq!(bare.name, "bare-plugin");
    assert_eq!(bare.version, "0.0.0");
    assert_eq!(bare.author, "unknown");
    assert!(bare.description.is_empty());

    // Only the leading comment block counts; a later `@id:` must be ignored.
    let trailing = parse_metadata(
        "// @id: real-id\nlet x = 1;\n// @id: fake-id\n",
        "stem",
    );
    assert_eq!(trailing.id, "real-id");
}

#[test]
fn test_discover_in_scans_rhai_files_only() {
    let dir = scratch_dir("amata_plugin_discovery");
    std::fs::write(dir.join("poster.rhai"), PLUGIN_WITH_METADATA).unwrap();
    std::fs::write(dir.join("bare.rhai"), PLUGIN_WITHOUT_METADATA).unwrap();
    std::fs::write(dir.join("notes.txt"), "not a plugin").unwrap();
    std::fs::create_dir_all(dir.join("nested.rhai")).unwrap();

    let missing_dir = dir.join("does-not-exist");
    // Same directory passed twice must not duplicate results.
    let found = discover_in(&[dir.clone(), missing_dir, dir.clone()]);

    assert_eq!(found.len(), 2, "found: {:?}", found.iter().map(|p| &p.id).collect::<Vec<_>>());
    // Sorted by id for stable CLI output; the bare script's id is its file stem.
    assert_eq!(found[0].id, "bare");
    assert_eq!(found[0].name, "bare");
    assert_eq!(found[1].id, "poster-grid");
    assert_eq!(found[1].version, "1.2.0");
    assert!(found[1].line_count >= 6);
    assert!(found[1].preview(2).contains("// @name: Poster Grid"));
    assert!(found[0].preview(1).starts_with("// A bare script"));

    // Lookup by id, display name, and file stem (case-insensitive).
    assert!(find_plugin(&found, "POSTER-GRID").is_some());
    assert!(find_plugin(&found, "Poster Grid").is_some());
    assert!(find_plugin(&found, "bare").is_some());
    assert!(find_plugin(&found, "nope").is_none());
    assert!(find_plugin(&found, "  ").is_none());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_plugins_cli_reports_discovered_plugins() {
    let dir = scratch_dir("amata_plugin_cli");
    std::fs::write(dir.join("poster.rhai"), PLUGIN_WITH_METADATA).unwrap();

    // `AMATA_PLUGIN_DIR` is the documented override, and it keeps this test
    // independent of the developer's ~/.irasu directory.
    std::env::set_var("AMATA_PLUGIN_DIR", &dir);

    let list = run_cli(Cli {
        command: Some(Commands::Plugins { info: None }),
    });
    assert!(list.is_ok(), "listing plugins must succeed: {list:?}");

    let known = run_cli(Cli {
        command: Some(Commands::Plugins {
            info: Some("poster-grid".to_string()),
        }),
    });
    assert!(known.is_ok(), "info for a discovered plugin must succeed");

    let unknown = run_cli(Cli {
        command: Some(Commands::Plugins {
            info: Some("definitely-not-installed".to_string()),
        }),
    });
    assert!(unknown.is_err(), "unknown plugin ids must fail loudly");

    std::env::remove_var("AMATA_PLUGIN_DIR");
    let _ = std::fs::remove_dir_all(&dir);
}

