//! Project memory system for yoyo.
//!
//! Persists project-specific notes across sessions in `.yoyo/memory.json`.
//! Each memory is a `{note, timestamp}` pair stored as a JSON array.
//! Users can add memories with `/remember`, list with `/memories`, remove with `/forget`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A single project memory entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryEntry {
    pub note: String,
    pub timestamp: String,
}

/// The in-memory store of project memories.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectMemory {
    pub entries: Vec<MemoryEntry>,
}

/// The directory name for yoyo project data.
const YOYO_DIR: &str = ".yoyo";

/// The filename for the memory store within `.yoyo/`.
const MEMORY_FILE: &str = "memory.json";

/// Get the path to the memory file for the current project.
pub fn memory_file_path() -> PathBuf {
    Path::new(YOYO_DIR).join(MEMORY_FILE)
}

/// Load project memories from `.yoyo/memory.json`.
/// Returns an empty `ProjectMemory` if the file doesn't exist or can't be parsed.
pub fn load_memories() -> ProjectMemory {
    load_memories_from(&memory_file_path())
}

/// Load project memories from a specific path (for testing).
pub fn load_memories_from(path: &Path) -> ProjectMemory {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => ProjectMemory::default(),
    }
}

/// Save project memories to `.yoyo/memory.json`.
/// Creates the `.yoyo/` directory if it doesn't exist.
pub fn save_memories(memory: &ProjectMemory) -> Result<(), String> {
    save_memories_to(memory, &memory_file_path())
}

/// Save project memories to a specific path (for testing).
pub fn save_memories_to(memory: &ProjectMemory, path: &Path) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    let json =
        serde_json::to_string_pretty(memory).map_err(|e| format!("Serialization error: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

/// Add a new memory entry with the current timestamp.
pub fn add_memory(memory: &mut ProjectMemory, note: &str) {
    let timestamp = current_timestamp();
    memory.entries.push(MemoryEntry {
        note: note.to_string(),
        timestamp,
    });
}

/// Remove a memory entry by index (0-based).
/// Returns the removed entry, or None if the index is out of bounds.
pub fn remove_memory(memory: &mut ProjectMemory, index: usize) -> Option<MemoryEntry> {
    if index < memory.entries.len() {
        Some(memory.entries.remove(index))
    } else {
        None
    }
}

/// Search memories by case-insensitive substring match.
/// Returns a vec of `(index, &MemoryEntry)` for matching entries.
/// An empty query matches all entries.
pub fn search_memories<'a>(
    memory: &'a ProjectMemory,
    query: &str,
) -> Vec<(usize, &'a MemoryEntry)> {
    let query_lower = query.to_lowercase();
    memory
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.note.to_lowercase().contains(&query_lower))
        .collect()
}

/// Format memories for display in the system prompt.
/// Returns None if there are no memories.
pub fn format_memories_for_prompt(memory: &ProjectMemory) -> Option<String> {
    if memory.entries.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    lines.push("## Project Memories".to_string());
    lines.push(String::new());
    for entry in &memory.entries {
        lines.push(format!("- {} ({})", entry.note, entry.timestamp));
    }
    Some(lines.join("\n"))
}

/// Format an ISO-like timestamp as a human-readable relative time.
///
/// Accepts timestamps in the `YYYY-MM-DD HH:MM` format (as stored by
/// `current_timestamp()`). Returns strings like "just now", "5m ago",
/// "2h ago", "3d ago", "2w ago", "3mo ago".
///
/// On parse failure, returns the original timestamp unchanged.
pub fn format_relative_time(iso_timestamp: &str) -> String {
    format_relative_time_from(iso_timestamp, now_epoch_secs())
}

/// Testable inner implementation — takes an explicit "now" epoch.
fn format_relative_time_from(iso_timestamp: &str, now_secs: i64) -> String {
    let ts_secs = match parse_timestamp_to_epoch(iso_timestamp.trim()) {
        Some(s) => s,
        None => return iso_timestamp.to_string(),
    };
    let diff = now_secs.saturating_sub(ts_secs);
    if diff < 0 {
        return iso_timestamp.to_string();
    }
    let diff = diff as u64;

    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    const WEEK: u64 = 7 * DAY;
    const MONTH: u64 = 30 * DAY;

    if diff < MINUTE {
        "just now".to_string()
    } else if diff < HOUR {
        format!("{}m ago", diff / MINUTE)
    } else if diff < DAY {
        format!("{}h ago", diff / HOUR)
    } else if diff < WEEK {
        format!("{}d ago", diff / DAY)
    } else if diff < MONTH {
        format!("{}w ago", diff / WEEK)
    } else {
        format!("{}mo ago", diff / MONTH)
    }
}

/// Parse `YYYY-MM-DD HH:MM` (local time) into epoch seconds.
/// Returns `None` on any parse failure.
fn parse_timestamp_to_epoch(s: &str) -> Option<i64> {
    // Expected format: "2026-03-15 08:32"
    if s.len() < 16 {
        return None;
    }
    let year: i32 = s.get(0..4)?.parse().ok()?;
    if s.as_bytes().get(4)? != &b'-' {
        return None;
    }
    let month: u32 = s.get(5..7)?.parse().ok()?;
    if s.as_bytes().get(7)? != &b'-' {
        return None;
    }
    let day: u32 = s.get(8..10)?.parse().ok()?;
    if s.as_bytes().get(10)? != &b' ' {
        return None;
    }
    let hour: u32 = s.get(11..13)?.parse().ok()?;
    if s.as_bytes().get(13)? != &b':' {
        return None;
    }
    let min: u32 = s.get(14..16)?.parse().ok()?;

    // Validate ranges
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if hour >= 24 || min >= 60 {
        return None;
    }

    // Convert to epoch using the same local-time assumption as `date`.
    // We shell out to `date -d` (Linux) or `date -j -f` (macOS) for
    // portability — but that's fragile. Instead, compute a UTC-ish epoch
    // inline and accept the same local-time offset error that `date +%Y-%m-%d`
    // introduces. Since both the stored timestamp and "now" share the offset,
    // the *difference* is correct (which is all we need for relative display).

    // Days from year 0 to start of `year`, then add month/day.
    let epoch = simple_local_epoch(year, month, day, hour, min);
    Some(epoch)
}

/// A lightweight local-time epoch (seconds since 1970-01-01 00:00 local).
/// Does NOT account for timezone — but since both stored timestamps and
/// `now_epoch_secs` share the same offset, the difference is correct.
fn simple_local_epoch(year: i32, month: u32, day: u32, hour: u32, min: u32) -> i64 {
    // Days in each month (non-leap)
    const MONTH_DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    fn is_leap(y: i32) -> bool {
        (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
    }

    // Days from 1970-01-01 to start of `year`
    let mut days: i64 = 0;
    if year >= 1970 {
        for y in 1970..year {
            days += if is_leap(y) { 366 } else { 365 };
        }
    } else {
        for y in year..1970 {
            days -= if is_leap(y) { 366 } else { 365 };
        }
    }

    // Add months
    for (m, &md) in MONTH_DAYS.iter().enumerate().take((month - 1) as usize) {
        days += md as i64;
        if m == 1 && is_leap(year) {
            days += 1; // Feb in leap year
        }
    }

    // Add days (1-indexed)
    days += (day - 1) as i64;

    days * 86400 + hour as i64 * 3600 + min as i64 * 60
}

/// Current local time as epoch seconds (same basis as `current_timestamp`).
fn now_epoch_secs() -> i64 {
    // Parse the output of `date +%s` for a local-consistent epoch.
    // Falls back to UNIX_EPOCH-based SystemTime if shell fails.
    std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8(o.stdout).ok()?;
                parse_timestamp_to_epoch(s.trim())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        })
}

/// Get the current timestamp in a human-readable format.
fn current_timestamp() -> String {
    // Use a simple approach: shell out to date command for portability
    std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_memory_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yoyo_test_memory_{}", name));
        let _ = fs::create_dir_all(&dir);
        dir.join(MEMORY_FILE)
    }

    fn cleanup(path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn test_memory_entry_serialize_deserialize() {
        let entry = MemoryEntry {
            note: "uses sqlx for database access".to_string(),
            timestamp: "2026-03-15 08:32".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, entry);
    }

    #[test]
    fn test_project_memory_serialize_deserialize() {
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "tests require docker running".to_string(),
                    timestamp: "2026-03-15 08:00".to_string(),
                },
                MemoryEntry {
                    note: "use pnpm not npm".to_string(),
                    timestamp: "2026-03-15 09:00".to_string(),
                },
            ],
        };
        let json = serde_json::to_string_pretty(&memory).unwrap();
        let parsed: ProjectMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].note, "tests require docker running");
        assert_eq!(parsed.entries[1].note, "use pnpm not npm");
    }

    #[test]
    fn test_add_memory() {
        let mut memory = ProjectMemory::default();
        assert!(memory.entries.is_empty());

        add_memory(&mut memory, "this project uses sqlx");
        assert_eq!(memory.entries.len(), 1);
        assert_eq!(memory.entries[0].note, "this project uses sqlx");
        assert!(!memory.entries[0].timestamp.is_empty());

        add_memory(&mut memory, "tests need docker");
        assert_eq!(memory.entries.len(), 2);
        assert_eq!(memory.entries[1].note, "tests need docker");
    }

    #[test]
    fn test_remove_memory_valid_index() {
        let mut memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "note 0".to_string(),
                    timestamp: "t0".to_string(),
                },
                MemoryEntry {
                    note: "note 1".to_string(),
                    timestamp: "t1".to_string(),
                },
                MemoryEntry {
                    note: "note 2".to_string(),
                    timestamp: "t2".to_string(),
                },
            ],
        };

        let removed = remove_memory(&mut memory, 1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().note, "note 1");
        assert_eq!(memory.entries.len(), 2);
        assert_eq!(memory.entries[0].note, "note 0");
        assert_eq!(memory.entries[1].note, "note 2");
    }

    #[test]
    fn test_remove_memory_invalid_index() {
        let mut memory = ProjectMemory {
            entries: vec![MemoryEntry {
                note: "only one".to_string(),
                timestamp: "t0".to_string(),
            }],
        };

        let removed = remove_memory(&mut memory, 5);
        assert!(removed.is_none());
        assert_eq!(memory.entries.len(), 1);
    }

    #[test]
    fn test_remove_memory_empty() {
        let mut memory = ProjectMemory::default();
        let removed = remove_memory(&mut memory, 0);
        assert!(removed.is_none());
    }

    #[test]
    fn test_save_and_load_memories() {
        let path = temp_memory_path("save_load");
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "first note".to_string(),
                    timestamp: "2026-03-15 08:00".to_string(),
                },
                MemoryEntry {
                    note: "second note".to_string(),
                    timestamp: "2026-03-15 09:00".to_string(),
                },
            ],
        };

        let result = save_memories_to(&memory, &path);
        assert!(result.is_ok(), "Save should succeed: {:?}", result);

        let loaded = load_memories_from(&path);
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].note, "first note");
        assert_eq!(loaded.entries[1].note, "second note");

        cleanup(&path);
    }

    #[test]
    fn test_load_memories_nonexistent_file() {
        let path = Path::new("/tmp/yoyo_test_nonexistent_12345/memory.json");
        let memory = load_memories_from(path);
        assert!(memory.entries.is_empty());
    }

    #[test]
    fn test_load_memories_invalid_json() {
        let path = temp_memory_path("invalid_json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not valid json at all {{{").unwrap();

        let memory = load_memories_from(&path);
        assert!(
            memory.entries.is_empty(),
            "Invalid JSON should return empty memory"
        );

        cleanup(&path);
    }

    #[test]
    fn test_save_creates_directory() {
        let dir = std::env::temp_dir().join("yoyo_test_memory_create_dir");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("subdir").join(MEMORY_FILE);

        let memory = ProjectMemory {
            entries: vec![MemoryEntry {
                note: "test".to_string(),
                timestamp: "now".to_string(),
            }],
        };

        let result = save_memories_to(&memory, &path);
        assert!(
            result.is_ok(),
            "Save should create parent dirs: {:?}",
            result
        );
        assert!(path.exists(), "File should exist after save");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_format_memories_for_prompt_empty() {
        let memory = ProjectMemory::default();
        assert!(format_memories_for_prompt(&memory).is_none());
    }

    #[test]
    fn test_format_memories_for_prompt_with_entries() {
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "uses sqlx".to_string(),
                    timestamp: "2026-03-15 08:00".to_string(),
                },
                MemoryEntry {
                    note: "docker needed for tests".to_string(),
                    timestamp: "2026-03-15 09:00".to_string(),
                },
            ],
        };

        let prompt = format_memories_for_prompt(&memory).unwrap();
        assert!(prompt.contains("## Project Memories"));
        assert!(prompt.contains("uses sqlx"));
        assert!(prompt.contains("docker needed for tests"));
        assert!(prompt.contains("2026-03-15 08:00"));
    }

    #[test]
    fn test_memory_file_path() {
        let path = memory_file_path();
        assert!(path.to_string_lossy().contains(".yoyo"));
        assert!(path.to_string_lossy().contains("memory.json"));
    }

    #[test]
    fn test_full_crud_workflow() {
        let path = temp_memory_path("crud_workflow");

        // Start fresh
        let mut memory = load_memories_from(&path);
        assert!(memory.entries.is_empty());

        // Add entries
        add_memory(&mut memory, "first");
        add_memory(&mut memory, "second");
        add_memory(&mut memory, "third");
        assert_eq!(memory.entries.len(), 3);

        // Save
        save_memories_to(&memory, &path).unwrap();

        // Reload
        let mut loaded = load_memories_from(&path);
        assert_eq!(loaded.entries.len(), 3);
        assert_eq!(loaded.entries[0].note, "first");

        // Remove middle entry
        let removed = remove_memory(&mut loaded, 1);
        assert_eq!(removed.unwrap().note, "second");
        assert_eq!(loaded.entries.len(), 2);

        // Save and reload again
        save_memories_to(&loaded, &path).unwrap();
        let final_load = load_memories_from(&path);
        assert_eq!(final_load.entries.len(), 2);
        assert_eq!(final_load.entries[0].note, "first");
        assert_eq!(final_load.entries[1].note, "third");

        cleanup(&path);
    }

    #[test]
    fn test_search_memories_basic() {
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "uses sqlx for database".to_string(),
                    timestamp: "t0".to_string(),
                },
                MemoryEntry {
                    note: "docker needed for tests".to_string(),
                    timestamp: "t1".to_string(),
                },
                MemoryEntry {
                    note: "always run cargo fmt".to_string(),
                    timestamp: "t2".to_string(),
                },
            ],
        };

        let results = search_memories(&memory, "docker");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1); // index 1
        assert_eq!(results[0].1.note, "docker needed for tests");
    }

    #[test]
    fn test_search_memories_case_insensitive() {
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "Uses SQLx for Database".to_string(),
                    timestamp: "t0".to_string(),
                },
                MemoryEntry {
                    note: "docker NEEDED".to_string(),
                    timestamp: "t1".to_string(),
                },
            ],
        };

        let results = search_memories(&memory, "SQLX");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.note, "Uses SQLx for Database");

        let results = search_memories(&memory, "needed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[test]
    fn test_search_memories_no_match() {
        let memory = ProjectMemory {
            entries: vec![MemoryEntry {
                note: "uses sqlx".to_string(),
                timestamp: "t0".to_string(),
            }],
        };

        let results = search_memories(&memory, "python");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_memories_empty_query() {
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "first".to_string(),
                    timestamp: "t0".to_string(),
                },
                MemoryEntry {
                    note: "second".to_string(),
                    timestamp: "t1".to_string(),
                },
            ],
        };

        let results = search_memories(&memory, "");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_memories_multiple_matches() {
        let memory = ProjectMemory {
            entries: vec![
                MemoryEntry {
                    note: "cargo build first".to_string(),
                    timestamp: "t0".to_string(),
                },
                MemoryEntry {
                    note: "docker needed".to_string(),
                    timestamp: "t1".to_string(),
                },
                MemoryEntry {
                    note: "cargo fmt before commit".to_string(),
                    timestamp: "t2".to_string(),
                },
            ],
        };

        let results = search_memories(&memory, "cargo");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 0);
        assert_eq!(results[1].0, 2);
    }

    // --- format_relative_time tests ---

    fn fixed_now() -> i64 {
        // 2026-05-24 12:00 as local epoch
        simple_local_epoch(2026, 5, 24, 12, 0)
    }

    #[test]
    fn test_relative_time_just_now() {
        let now = fixed_now();
        // 30 seconds ago — "just now"
        let ts = "2026-05-24 12:00"; // same minute
        assert_eq!(format_relative_time_from(ts, now), "just now");
    }

    #[test]
    fn test_relative_time_minutes_ago() {
        let now = fixed_now();
        // 5 minutes ago → 2026-05-24 11:55
        let ts = "2026-05-24 11:55";
        assert_eq!(format_relative_time_from(ts, now), "5m ago");
    }

    #[test]
    fn test_relative_time_hours_ago() {
        let now = fixed_now();
        // 2 hours ago → 2026-05-24 10:00
        let ts = "2026-05-24 10:00";
        let result = format_relative_time_from(ts, now);
        assert_eq!(result, "2h ago");
    }

    #[test]
    fn test_relative_time_days_ago() {
        let now = fixed_now();
        // 3 days ago → 2026-05-21 12:00
        let ts = "2026-05-21 12:00";
        assert_eq!(format_relative_time_from(ts, now), "3d ago");
    }

    #[test]
    fn test_relative_time_weeks_ago() {
        let now = fixed_now();
        // 14 days ago → 2026-05-10 12:00
        let ts = "2026-05-10 12:00";
        assert_eq!(format_relative_time_from(ts, now), "2w ago");
    }

    #[test]
    fn test_relative_time_months_ago() {
        let now = fixed_now();
        // ~90 days ago → 2026-02-23 12:00
        let ts = "2026-02-23 12:00";
        assert_eq!(format_relative_time_from(ts, now), "3mo ago");
    }

    #[test]
    fn test_relative_time_malformed_returns_original() {
        let now = fixed_now();
        let ts = "not-a-timestamp";
        assert_eq!(format_relative_time_from(ts, now), "not-a-timestamp");
    }

    #[test]
    fn test_relative_time_exactly_one_hour() {
        let now = fixed_now();
        // Exactly 1 hour ago → 2026-05-24 11:00
        let ts = "2026-05-24 11:00";
        assert_eq!(format_relative_time_from(ts, now), "1h ago");
    }

    #[test]
    fn test_parse_timestamp_to_epoch_valid() {
        let epoch = parse_timestamp_to_epoch("2026-05-24 12:00");
        assert!(epoch.is_some());
    }

    #[test]
    fn test_parse_timestamp_to_epoch_invalid() {
        assert!(parse_timestamp_to_epoch("garbage").is_none());
        assert!(parse_timestamp_to_epoch("2026-13-01 00:00").is_none()); // bad month
        assert!(parse_timestamp_to_epoch("2026-05-32 00:00").is_none()); // bad day
        assert!(parse_timestamp_to_epoch("2026-05-24 25:00").is_none()); // bad hour
    }
}
