use crate::core::document::Document;
use std::fs;
use std::path::Path;

pub fn save_project(doc: &Document, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    super::atomic::atomic_write_str(path, &json).map_err(|e| e.to_string())
}

/// Max project file size (DoS guard: nested groups deserialize
/// recursively, and embedded images inflate memory far past file size).
pub const MAX_PROJECT_BYTES: usize = 256 * 1024 * 1024;

pub fn load_project(path: &Path) -> Result<Document, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_PROJECT_BYTES as u64 {
        return Err(format!(
            "プロジェクトが大きすぎます（上限{}MB）",
            MAX_PROJECT_BYTES / 1024 / 1024
        ));
    }
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    check_json_depth(&data)?;
    let mut doc: Document = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    sanitize_document(&mut doc);
    doc.normalize();
    Ok(doc)
}

/// Reject deeply-nested JSON before serde recursion (stack overflow).
/// String literals and escapes are skipped so `{"text":"[[["}` is safe.
pub(crate) fn check_json_depth(data: &str) -> Result<(), String> {
    const MAX_DEPTH: usize = 200;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escape = false;
    for b in data.bytes() {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err("プロジェクトのネストが深すぎます".into());
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Clamp adversarial dimensions/counts after load: serde recursion on
/// hostile nesting is bounded by the file cap above, but absurd-but-valid
/// values (NaN transforms, gigapixel canvases, million-object layers)
/// must not reach the renderer/canvas.
pub(crate) fn sanitize_document(doc: &mut Document) {
    fn finite_or(v: f64, fallback: f64) -> f64 {
        if v.is_finite() {
            v
        } else {
            fallback
        }
    }
    doc.width = finite_or(doc.width, 1920.0).clamp(1.0, 16384.0);
    doc.height = finite_or(doc.height, 1080.0).clamp(1.0, 16384.0);
    doc.bleed = finite_or(doc.bleed, 0.0).clamp(0.0, 144.0);
    const MAX_OBJECTS: usize = 200_000;
    let mut count = 0usize;
    fn cap_objects(objs: &mut Vec<crate::core::document::Object>, count: &mut usize) {
        objs.retain(|_| {
            *count += 1;
            *count <= MAX_OBJECTS
        });
        for o in objs.iter_mut() {
            // Scrub non-finite transforms (NaN/Inf poison bbox math and
            // hit-testing all the way down the pipeline).
            let t = &mut o.transform;
            t.x = finite_or(t.x, 0.0);
            t.y = finite_or(t.y, 0.0);
            t.rotation = finite_or(t.rotation, 0.0);
            t.scale_x = finite_or(t.scale_x, 1.0);
            t.scale_y = finite_or(t.scale_y, 1.0);
            t.skew_x = finite_or(t.skew_x, 0.0);
            t.skew_y = finite_or(t.skew_y, 0.0);
            match &mut o.object_type {
                crate::core::document::ObjectType::Group(children)
                | crate::core::document::ObjectType::ClippingMask { children } => {
                    cap_objects(children, count);
                }
                crate::core::document::ObjectType::Image { png_bytes, .. }
                    if png_bytes.len() > 32 * 1024 * 1024 =>
                {
                    // Oversized embedded rasters blow up renderer memory
                    // and project JSON alike: drop the bytes, keep the box.
                    png_bytes.clear();
                }
                _ => {}
            }
        }
    }
    for layer in &mut doc.layers {
        cap_objects(&mut layer.objects, &mut count);
    }
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
