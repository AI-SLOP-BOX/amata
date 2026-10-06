use crate::core::document::Document;
use std::fs;
use std::path::Path;

pub fn save_project(doc: &Document, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    super::atomic::atomic_write_str(path, &json).map_err(|e| e.to_string())
}

pub fn load_project(path: &Path) -> Result<Document, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut doc: Document = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    doc.normalize();
    Ok(doc)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RecoveryFile {
    original_path: Option<String>,
    saved_at_secs: u64,
    document: Document,
}

fn recovery_paths() -> [std::path::PathBuf; 2] {
    let tmp = std::env::temp_dir();
    [
        tmp.join("amata_autosave_recovery_a.json"),
        tmp.join("amata_autosave_recovery_b.json"),
    ]
}

fn all_recovery_paths() -> Vec<std::path::PathBuf> {
    let mut paths = recovery_paths().to_vec();
    // Legacy single-slot file (pre A/B rotation).
    paths.push(std::env::temp_dir().join("amata_autosave_recovery.json"));
    paths
}

/// Move a corrupt snapshot aside so it can still be inspected by hand.
/// Explicit (never inside a read path): quarantine is a mutation.
pub fn quarantine_recovery(path: &std::path::Path) {
    let _ = fs::rename(path, path.with_extension("json.corrupt"));
}

/// Newest valid slot (by `saved_at_secs`), falling back to the legacy
/// single-slot file from older builds. Pure read: corrupt slots are
/// skipped here and quarantined by `load_recovery`.
fn newest_recovery() -> Option<(std::path::PathBuf, RecoveryFile)> {
    let mut best: Option<(std::path::PathBuf, RecoveryFile)> = None;
    for path in all_recovery_paths() {
        let Ok(data) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(rec) = serde_json::from_str::<RecoveryFile>(&data) else {
            continue;
        };
        let newer = best
            .as_ref()
            .map(|(_, b)| rec.saved_at_secs >= b.saved_at_secs)
            .unwrap_or(true);
        if newer {
            best = Some((path, rec));
        }
    }
    best
}

/// Write an autosave snapshot (always full-fidelity project JSON in the
/// temp dir — never touching the user's file, so the watcher stays quiet).
/// Atomic like everything else, and A/B-rotated: a crash mid-write can only
/// ever take out the slot being written, leaving the previous tick intact.
pub fn save_recovery(doc: &Document, original: Option<&Path>) -> Result<(), String> {
    let rec = RecoveryFile {
        original_path: original.map(|p| p.to_string_lossy().to_string()),
        saved_at_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        document: doc.clone(),
    };
    let json = serde_json::to_string(&rec).map_err(|e| e.to_string())?;
    // Write the older slot so the newest intact snapshot always survives.
    let [a, b] = recovery_paths();
    let a_time = fs::read_to_string(&a)
        .ok()
        .and_then(|d| serde_json::from_str::<RecoveryFile>(&d).ok())
        .map(|r| r.saved_at_secs)
        .unwrap_or(0);
    let b_time = fs::read_to_string(&b)
        .ok()
        .and_then(|d| serde_json::from_str::<RecoveryFile>(&d).ok())
        .map(|r| r.saved_at_secs)
        .unwrap_or(0);
    let target = if a_time <= b_time { a } else { b };
    super::atomic::atomic_write_str(&target, &json).map_err(|e| e.to_string())
}

pub fn load_recovery() -> Option<(Option<std::path::PathBuf>, Document)> {
    // Quarantine corrupt slots on the explicit load path (not inside the
    // pure newest-slot scan), so a half-written snapshot never loads.
    for path in all_recovery_paths() {
        if let Ok(data) = fs::read_to_string(&path) {
            if serde_json::from_str::<RecoveryFile>(&data).is_err() {
                quarantine_recovery(&path);
            }
        }
    }
    let (_path, rec) = newest_recovery()?;
    let mut doc = rec.document;
    doc.normalize();
    Some((rec.original_path.map(std::path::PathBuf::from), doc))
}

pub fn clear_recovery() {
    for path in all_recovery_paths() {
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.corrupt"));
    }
}

pub fn has_recovery() -> bool {
    newest_recovery().is_some()
}
