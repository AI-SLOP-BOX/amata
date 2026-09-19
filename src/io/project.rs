use crate::core::document::Document;
use std::fs;
use std::path::Path;

pub fn save_project(doc: &Document, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
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

fn recovery_path() -> std::path::PathBuf {
    std::env::temp_dir().join("amata_autosave_recovery.json")
}

/// Write an autosave snapshot (always full-fidelity project JSON in the
/// temp dir — never touching the user's file, so the watcher stays quiet).
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
    fs::write(recovery_path(), json).map_err(|e| e.to_string())
}

pub fn load_recovery() -> Option<(Option<std::path::PathBuf>, Document)> {
    let data = fs::read_to_string(recovery_path()).ok()?;
    let rec: RecoveryFile = serde_json::from_str(&data).ok()?;
    let mut doc = rec.document;
    doc.normalize();
    Some((rec.original_path.map(std::path::PathBuf::from), doc))
}

pub fn clear_recovery() {
    let _ = fs::remove_file(recovery_path());
}

pub fn has_recovery() -> bool {
    recovery_path().exists()
}
