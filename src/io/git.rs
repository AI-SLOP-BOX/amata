use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitEntry {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub relative_time: String,
    pub timestamp: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub is_tracked: bool,
    pub is_modified: bool,
    pub is_staged: bool,
}

fn get_dir_for_file(path: &Path) -> &Path {
    if path.is_file() {
        match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        }
    } else if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    }
}

/// Helper to run a git command inside a specific working directory
fn run_git_cmd(dir: &Path, args: &[&str]) -> Result<String, String> {
    let target_dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let output = Command::new("git")
        .current_dir(target_dir)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute git command: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(err.trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Check if a directory or file belongs to a Git repository
pub fn is_git_repository(path: &Path) -> bool {
    let dir = get_dir_for_file(path);

    match run_git_cmd(dir, &["rev-parse", "--is-inside-work-tree"]) {
        Ok(out) => out.trim() == "true",
        Err(_) => false,
    }
}

/// Initialize a new git repository if one doesn't exist
pub fn init_git_repository(dir: &Path) -> Result<String, String> {
    let target = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    run_git_cmd(target, &["init"])
}

/// Get commit history for a specific file
pub fn get_file_commit_history(
    file_path: &Path,
    max_count: usize,
) -> Result<Vec<GitCommitEntry>, String> {
    let parent = get_dir_for_file(file_path);
    let filename = file_path
        .file_name()
        .ok_or("Invalid file path")?
        .to_string_lossy();

    let max_str = max_count.to_string();
    let format_str = "%H|%h|%an|%cr|%cI|%s";
    let args = [
        "log",
        &format!("-n{max_str}"),
        &format!("--pretty=format:{format_str}"),
        "--follow",
        "--",
        &filename,
    ];

    let output = run_git_cmd(parent, &args)?;
    let mut entries = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(6, '|').collect();
        if parts.len() == 6 {
            entries.push(GitCommitEntry {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                author: parts[2].to_string(),
                relative_time: parts[3].to_string(),
                timestamp: parts[4].to_string(),
                message: parts[5].to_string(),
            });
        }
    }

    Ok(entries)
}

/// Get status of a specific file
pub fn get_file_status(file_path: &Path) -> Result<GitFileStatus, String> {
    let parent = get_dir_for_file(file_path);
    let filename = file_path
        .file_name()
        .ok_or("Invalid file path")?
        .to_string_lossy();

    let output = run_git_cmd(parent, &["status", "--porcelain", "--", &filename])?;
    if output.trim().is_empty() {
        // File is tracked and clean
        Ok(GitFileStatus {
            is_tracked: true,
            is_modified: false,
            is_staged: false,
        })
    } else {
        let first_line = output.lines().next().unwrap_or("");
        let index_status = first_line.chars().next().unwrap_or(' ');
        let work_status = first_line.chars().nth(1).unwrap_or(' ');

        let is_untracked = index_status == '?' && work_status == '?';
        let is_staged = index_status != ' ' && index_status != '?';
        let is_modified = work_status == 'M' || index_status == 'M';

        Ok(GitFileStatus {
            is_tracked: !is_untracked,
            is_modified,
            is_staged,
        })
    }
}

/// Create a checkpoint (Git commit) for a specific file
pub fn create_checkpoint(file_path: &Path, message: &str) -> Result<String, String> {
    let parent = get_dir_for_file(file_path);
    let filename = file_path
        .file_name()
        .ok_or("Invalid file path")?
        .to_string_lossy();

    // Stage file
    run_git_cmd(parent, &["add", &filename])?;

    // Commit
    // Using -c user.name and user.email fallback in case user hasn't set global git config
    let args = [
        "-c",
        "user.name=Amata Vector Studio",
        "-c",
        "user.email=amata@local",
        "commit",
        "-m",
        message,
        "--",
        &filename,
    ];

    run_git_cmd(parent, &args)
}

/// Retrieve content of a file at a specific revision (e.g. HEAD, HEAD~1, a commit hash)
pub fn get_file_content_at_rev(file_path: &Path, revision: &str) -> Result<String, String> {
    let parent = get_dir_for_file(file_path);
    let filename = file_path
        .file_name()
        .ok_or("Invalid file path")?
        .to_string_lossy();

    let target = format!("{}:./{}", revision, filename);
    run_git_cmd(parent, &["show", &target])
}

/// Restore a file to a specific revision
pub fn restore_file_to_rev(file_path: &Path, revision: &str) -> Result<String, String> {
    let parent = get_dir_for_file(file_path);
    let filename = file_path
        .file_name()
        .ok_or("Invalid file path")?
        .to_string_lossy();

    let args = ["checkout", revision, "--", &filename];
    run_git_cmd(parent, &args)
}

/// Safely preserve external version before a local overwrite (Keep Local).
/// Attempts Git checkpoint first, and creates a backup file snapshot.
pub fn preserve_external_version(
    file_path: &Path,
    external_svg: &str,
) -> Result<std::path::PathBuf, String> {
    let mut git_saved = false;
    if is_git_repository(file_path)
        && create_checkpoint(
            file_path,
            "[Amata Snapshot] External version before local overwrite",
        ).is_ok() {
            git_saved = true;
        }

    let backup_path = file_path.with_extension("external_backup.svg");
    let file_saved = std::fs::write(&backup_path, external_svg).is_ok();

    if git_saved || file_saved {
        Ok(backup_path)
    } else {
        Err("Failed to create either Git checkpoint or snapshot file".to_string())
    }
}
