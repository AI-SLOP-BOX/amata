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
