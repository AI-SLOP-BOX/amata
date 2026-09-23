use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A recently opened project/document entry (real paths, persisted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    pub path: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub last_opened_secs: u64,
}

/// How many recent files to keep. Driven by `Prefs::recent_files_count`
/// (see [`set_recent_limit`]); the default only applies before preferences
/// are loaded (CLI runs and tests).
static RECENT_LIMIT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(8);

/// Update the recent-files cap (`0` is clamped to 1 so the list never
/// becomes unreachable).
pub fn set_recent_limit(count: usize) {
    RECENT_LIMIT.store(count.max(1), std::sync::atomic::Ordering::Relaxed);
}

fn recent_limit() -> usize {
    RECENT_LIMIT.load(std::sync::atomic::Ordering::Relaxed)
}

fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .map(|p| p.join("Amata"))
            .or_else(|| {
                std::env::var("USERPROFILE")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".amata"))
            })
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".config").join("amata"))
    }
}

fn recents_file() -> Option<PathBuf> {
    config_dir().map(|d| d.join("recent.json"))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Humanize a timestamp for the home screen ("3日前", "昨日", …).
pub fn friendly_age(secs: u64) -> String {
    let now = now_secs();
    let dt = now.saturating_sub(secs);
    if dt < 60 {
        "たった今".to_string()
    } else if dt < 3600 {
        format!("{}分前", dt / 60)
    } else if dt < 86400 {
        let h = dt / 3600;
        if h <= 3 {
            format!("{h}時間前")
        } else {
            "今日".to_string()
        }
    } else if dt < 2 * 86400 {
        "昨日".to_string()
    } else if dt < 7 * 86400 {
        format!("{}日前", dt / 86400)
    } else {
        format!("{}週間前", dt / (7 * 86400))
    }
}

/// Stable accent colors derived from the path so cards look distinct
/// without thumbnails.
pub fn entry_colors(path: &str) -> ([u8; 3], [u8; 3]) {
    let mut h: u64 = 1469598103934665603;
    for b in path.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    let hue = (h % 360) as f32;
    let (r, g, b) = hsv(hue, 0.55, 0.75);
    let (r2, g2, b2) = hsv((hue + 40.0) % 360.0, 0.45, 0.9);
    ([r, g, b], [r2, g2, b2])
}

fn hsv(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

pub fn load_recents() -> Vec<RecentEntry> {
    let Some(path) = recents_file() else {
        return Vec::new();
    };
    let Ok(data) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut entries: Vec<RecentEntry> = serde_json::from_str(&data).unwrap_or_default();
    // Drop entries whose files vanished (stale cards are worse than none).
    entries.retain(|e| Path::new(&e.path).exists());
    entries.truncate(recent_limit());
    entries
}

fn save_recents(entries: &[RecentEntry]) {
    let Some(path) = recents_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(path, json);
    }
}

/// Record an opened/saved document (deduplicated, most-recent-first).
pub fn push_recent(path: &Path, width: f64, height: f64) {
    let path_str = path.to_string_lossy().to_string();
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();
    let mut entries = load_recents();
    entries.retain(|e| e.path != path_str);
    entries.insert(
        0,
        RecentEntry {
            path: path_str,
            name,
            width,
            height,
            last_opened_secs: now_secs(),
        },
    );
    entries.truncate(recent_limit());
    save_recents(&entries);
}
