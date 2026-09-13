use crate::core::diff::compute_semantic_diff;
use crate::core::document::Document;
use crate::io::git;
use crate::io::svg::parse_svg_document;
use std::path::{Path, PathBuf};

/// Handle `amata diff` command
pub fn handle_diff(
    target_a: &str,
    target_b: Option<&str>,
    file_opt: Option<&Path>,
    json: bool,
) -> Result<(), String> {
    let (doc_a, doc_b) = load_documents_for_diff(target_a, target_b, file_opt)?;
    let diff = compute_semantic_diff(&doc_a, &doc_b);

    if json {
        let json_str = diff
            .to_json()
            .map_err(|e| format!("JSON serialization error: {e}"))?;
        println!("{json_str}");
    } else {
        print!("{}", diff.format_text());
    }

    Ok(())
}

fn load_documents_for_diff(
    target_a: &str,
    target_b: Option<&str>,
    file_opt: Option<&Path>,
) -> Result<(Document, Document), String> {
    let path_a = Path::new(target_a);

    // Case 1: Two local files: `amata diff old.svg new.svg`
    if let Some(tb) = target_b {
        let path_b = Path::new(tb);
        if path_a.exists() && path_b.exists() {
            let svg_a = std::fs::read_to_string(path_a)
                .map_err(|e| format!("Failed to read {target_a}: {e}"))?;
            let svg_b =
                std::fs::read_to_string(path_b).map_err(|e| format!("Failed to read {tb}: {e}"))?;
            let doc_a = parse_svg_document(&svg_a);
            let doc_b = parse_svg_document(&svg_b);
            return Ok((doc_a, doc_b));
        }
    }

    // Case 2: Git revision comparison
    // e.g. `amata diff HEAD poster.svg` -> target_a: "HEAD", target_b: "poster.svg"
    // e.g. `amata diff HEAD~1 HEAD --file poster.svg`
    let (rev_a, rev_b_or_file) = (target_a, target_b);

    let file_path = if let Some(f) = file_opt {
        f.to_path_buf()
    } else if let Some(tb) = rev_b_or_file {
        let p = PathBuf::from(tb);
        if p.exists() {
            p
        } else {
            return Err(format!(
                "Could not identify target file. Please specify with --file"
            ));
        }
    } else {
        return Err("Missing target file or revision to compare against.".to_string());
    };

    if !git::is_git_repository(&file_path) {
        return Err(format!(
            "'{}' is not part of a Git repository.",
            file_path.display()
        ));
    }

    // If target_b is a revision (like HEAD), fetch from git; if target_b is the file, compare rev_a vs working copy
    let content_a = git::get_file_content_at_rev(&file_path, rev_a)?;
    let content_b = if let Some(f) = file_opt {
        if let Some(tb) = rev_b_or_file {
            // comparing rev_a with rev_b
            git::get_file_content_at_rev(f, tb)?
        } else {
            std::fs::read_to_string(f).map_err(|e| format!("Failed to read file: {e}"))?
        }
    } else {
        // rev_b_or_file was the file, compare rev_a vs current working tree
        std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read {}: {e}", file_path.display()))?
    };

    let doc_a = parse_svg_document(&content_a);
    let doc_b = parse_svg_document(&content_b);

    Ok((doc_a, doc_b))
}

/// Handle `amata history` command
pub fn handle_history(input: &Path, max: usize, json: bool) -> Result<(), String> {
    if !git::is_git_repository(input) {
        return Err(format!(
            "'{}' is not inside a Git repository.",
            input.display()
        ));
    }

    let history = git::get_file_commit_history(input, max)?;
    if history.is_empty() {
        println!("No Git history found for '{}'.", input.display());
        return Ok(());
    }

    if json {
        let json_str = serde_json::to_string_pretty(&history)
            .map_err(|e| format!("Failed to serialize history to JSON: {e}"))?;
        println!("{json_str}");
    } else {
        println!("Version History for '{}':\n", input.display());
        for entry in history {
            println!(
                "  {}  {:18}  {}  ({})",
                entry.short_hash, entry.relative_time, entry.message, entry.author
            );
        }
        println!();
    }

    Ok(())
}

/// Handle `amata checkpoint` command
pub fn handle_checkpoint(input: &Path, message: &str) -> Result<(), String> {
    if !input.exists() {
        return Err(format!("File '{}' not found.", input.display()));
    }

    if !git::is_git_repository(input) {
        let parent = match input.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        println!("Initializing Git repository at '{}'...", parent.display());
        git::init_git_repository(parent)?;
    }

    let result = git::create_checkpoint(input, message)?;
    println!("Checkpoint created successfully for '{}'.", input.display());
    if !result.trim().is_empty() {
        println!("{result}");
    }

    Ok(())
}

/// Handle `amata restore` command
pub fn handle_restore(input: &Path, revision: &str) -> Result<(), String> {
    if !git::is_git_repository(input) {
        return Err(format!(
            "'{}' is not inside a Git repository.",
            input.display()
        ));
    }

    git::restore_file_to_rev(input, revision)?;
    println!(
        "Successfully restored '{}' to revision '{}'.",
        input.display(),
        revision
    );

    Ok(())
}
