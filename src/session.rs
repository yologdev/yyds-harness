//! Session tracking types — file changes, turn snapshots, and undo history.
//!
//! Extracted from `prompt.rs` (Day 54) to keep session-state types separate
//! from prompt execution logic.

use crate::format::pluralize;
use crate::sync_util::lock_or_recover;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Tracks files modified during a session via write_file and edit_file tool calls.
/// Thread-safe via Arc<Mutex<...>> so it can be shared across async tasks.
#[derive(Debug, Clone)]
pub struct SessionChanges {
    inner: Arc<Mutex<Vec<FileChange>>>,
}

/// A single file modification event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub kind: ChangeKind,
}

/// The kind of file modification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Write,
    Edit,
}

impl std::fmt::Display for ChangeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeKind::Write => write!(f, "write"),
            ChangeKind::Edit => write!(f, "edit"),
        }
    }
}

impl SessionChanges {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record a file modification.
    pub fn record(&self, path: &str, kind: ChangeKind) {
        let mut changes = lock_or_recover(&self.inner);
        // Update existing entry if same path, or add new
        if let Some(existing) = changes.iter_mut().find(|c| c.path == path) {
            existing.kind = kind;
        } else {
            changes.push(FileChange {
                path: path.to_string(),
                kind,
            });
        }
    }

    /// Get a snapshot of all changes, in order of first modification.
    pub fn snapshot(&self) -> Vec<FileChange> {
        lock_or_recover(&self.inner).clone()
    }

    /// Clear all tracked changes.
    pub fn clear(&self) {
        lock_or_recover(&self.inner).clear();
    }

    /// Return a JSON summary of file changes for `--json` output.
    ///
    /// Format:
    /// ```json
    /// { "files_changed": 2, "changes": [
    ///     { "path": "src/main.rs", "kind": "write" },
    ///     { "path": "src/cli.rs", "kind": "edit" }
    /// ]}
    /// ```
    pub fn to_json_summary(&self) -> serde_json::Value {
        let changes = lock_or_recover(&self.inner);
        let entries: Vec<serde_json::Value> = changes
            .iter()
            .map(|c| {
                serde_json::json!({
                    "path": c.path,
                    "kind": c.kind.to_string(),
                })
            })
            .collect();
        serde_json::json!({
            "files_changed": entries.len(),
            "changes": entries,
        })
    }
}

#[cfg(test)]
impl SessionChanges {
    /// Return the number of unique files changed.
    pub fn len(&self) -> usize {
        lock_or_recover(&self.inner).len()
    }

    /// Return true if no files have been changed.
    pub fn is_empty(&self) -> bool {
        lock_or_recover(&self.inner).is_empty()
    }
}

/// A snapshot of file state before a single agent turn.
///
/// Stores the original content of files that existed before the turn,
/// and tracks paths of files that were newly created during the turn.
/// Used by `/undo` to revert only the most recent turn's changes.
#[derive(Debug, Clone)]
pub struct TurnSnapshot {
    /// Files that existed before the turn: path → original content.
    pub originals: HashMap<String, String>,
    /// Files that were created during the turn (didn't exist before).
    pub created: Vec<String>,
}

impl TurnSnapshot {
    /// Create a new empty snapshot.
    pub fn new() -> Self {
        Self {
            originals: HashMap::new(),
            created: Vec::new(),
        }
    }

    /// Snapshot the current content of a file. If the file exists, stores its
    /// content in `originals`. Does nothing if already snapshotted.
    pub fn snapshot_file(&mut self, path: &str) {
        if self.originals.contains_key(path) {
            return; // Already snapshotted
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            self.originals.insert(path.to_string(), content);
        }
        // If file doesn't exist, we'll track it as created when we see it appear
    }

    /// Record a file as newly created during this turn.
    /// Only records if not already in originals (i.e., it truly didn't exist before).
    pub fn record_created(&mut self, path: &str) {
        if !self.originals.contains_key(path) && !self.created.contains(&path.to_string()) {
            self.created.push(path.to_string());
        }
    }

    /// Return true if no files were affected.
    pub fn is_empty(&self) -> bool {
        self.originals.is_empty() && self.created.is_empty()
    }

    /// Restore all files to their pre-turn state:
    /// - Overwrite modified files with their original content
    /// - Delete files that were created during the turn
    ///
    /// Returns a list of actions taken (for display).
    pub fn restore(&self) -> Vec<String> {
        let mut actions = Vec::new();

        // Restore modified files
        for (path, content) in &self.originals {
            if std::fs::write(path, content).is_ok() {
                actions.push(format!("restored {path}"));
            } else {
                actions.push(format!("failed to restore {path}"));
            }
        }

        // Delete newly created files
        for path in &self.created {
            if std::fs::remove_file(path).is_ok() {
                actions.push(format!("deleted {path}"));
            } else {
                actions.push(format!("failed to delete {path}"));
            }
        }

        actions
    }
}

#[cfg(test)]
impl TurnSnapshot {
    /// Return the number of files affected (modified + created).
    pub fn file_count(&self) -> usize {
        self.originals.len() + self.created.len()
    }
}

/// A stack of turn snapshots for multi-level undo.
///
/// Each completed agent turn pushes a snapshot. `/undo` pops the most recent.
/// `/undo N` pops the last N turns.
#[derive(Debug, Clone)]
pub struct TurnHistory {
    turns: Vec<TurnSnapshot>,
}

impl TurnHistory {
    /// Create a new empty history.
    pub fn new() -> Self {
        Self { turns: Vec::new() }
    }

    /// Push a completed turn's snapshot onto the stack.
    /// Skips empty snapshots (turns that didn't modify any files).
    pub fn push(&mut self, snapshot: TurnSnapshot) {
        if !snapshot.is_empty() {
            self.turns.push(snapshot);
        }
    }

    /// Return the number of undoable turns.
    pub fn len(&self) -> usize {
        self.turns.len()
    }

    /// Return true if there are no undoable turns.
    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }

    /// Undo the last N turns by popping and restoring each.
    /// Returns a list of all actions taken.
    pub fn undo_last(&mut self, n: usize) -> Vec<String> {
        let mut all_actions = Vec::new();
        let count = n.min(self.turns.len());
        for _ in 0..count {
            if let Some(snapshot) = self.turns.pop() {
                all_actions.extend(snapshot.restore());
            }
        }
        all_actions
    }

    /// Clear the entire history (used after /clear or /undo --all).
    pub fn clear(&mut self) {
        self.turns.clear();
    }
}

#[cfg(test)]
impl TurnHistory {
    /// Pop the most recent turn snapshot.
    pub fn pop(&mut self) -> Option<TurnSnapshot> {
        self.turns.pop()
    }
}

/// Format a human-readable summary of session changes.
pub fn format_changes(changes: &SessionChanges) -> String {
    let snapshot = changes.snapshot();
    if snapshot.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(&format!(
        "  {} {} modified this session:\n",
        snapshot.len(),
        pluralize(snapshot.len(), "file", "files")
    ));
    for change in &snapshot {
        let icon = match change.kind {
            ChangeKind::Write => "✏",
            ChangeKind::Edit => "🔧",
        };
        out.push_str(&format!("    {icon} {} ({})\n", change.path, change.kind));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- SessionChanges tests ---

    #[test]
    fn test_session_changes_new_is_empty() {
        let changes = SessionChanges::new();
        assert!(changes.is_empty());
        assert_eq!(changes.len(), 0);
        assert!(changes.snapshot().is_empty());
    }

    #[test]
    fn test_session_changes_record_write() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        assert_eq!(changes.len(), 1);
        assert!(!changes.is_empty());
        let snapshot = changes.snapshot();
        assert_eq!(snapshot[0].path, "src/main.rs");
        assert_eq!(snapshot[0].kind, ChangeKind::Write);
    }

    #[test]
    fn test_session_changes_record_edit() {
        let changes = SessionChanges::new();
        changes.record("src/cli.rs", ChangeKind::Edit);
        assert_eq!(changes.len(), 1);
        let snapshot = changes.snapshot();
        assert_eq!(snapshot[0].path, "src/cli.rs");
        assert_eq!(snapshot[0].kind, ChangeKind::Edit);
    }

    #[test]
    fn test_session_changes_deduplicates_same_path() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        changes.record("src/main.rs", ChangeKind::Edit);
        // Should still be 1 entry, updated to Edit
        assert_eq!(changes.len(), 1);
        let snapshot = changes.snapshot();
        assert_eq!(snapshot[0].kind, ChangeKind::Edit);
    }

    #[test]
    fn test_session_changes_multiple_files() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        changes.record("src/cli.rs", ChangeKind::Edit);
        changes.record("README.md", ChangeKind::Write);
        assert_eq!(changes.len(), 3);
        let snapshot = changes.snapshot();
        assert_eq!(snapshot[0].path, "src/main.rs");
        assert_eq!(snapshot[1].path, "src/cli.rs");
        assert_eq!(snapshot[2].path, "README.md");
    }

    #[test]
    fn test_session_changes_clear() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        changes.record("src/cli.rs", ChangeKind::Edit);
        assert_eq!(changes.len(), 2);
        changes.clear();
        assert!(changes.is_empty());
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn test_session_changes_clone_is_independent() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        let cloned = changes.clone();
        // They share the same inner Arc, so they should be linked
        changes.record("src/cli.rs", ChangeKind::Edit);
        assert_eq!(cloned.len(), 2);
    }

    #[test]
    fn test_change_kind_display() {
        assert_eq!(format!("{}", ChangeKind::Write), "write");
        assert_eq!(format!("{}", ChangeKind::Edit), "edit");
    }

    #[test]
    fn test_format_changes_empty() {
        let changes = SessionChanges::new();
        let output = format_changes(&changes);
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_changes_single_write() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        let output = format_changes(&changes);
        assert!(output.contains("1 file modified"));
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("write"));
        assert!(output.contains("✏"));
    }

    #[test]
    fn test_format_changes_multiple_files() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        changes.record("src/cli.rs", ChangeKind::Edit);
        let output = format_changes(&changes);
        assert!(output.contains("2 files modified"));
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("src/cli.rs"));
        assert!(output.contains("write"));
        assert!(output.contains("edit"));
        assert!(output.contains("🔧"));
    }

    #[test]
    fn test_session_changes_shared_across_content_prompts() {
        // Verifies that SessionChanges can be used across multiple prompt styles.
        // When @file mention prompts use the same SessionChanges as regular prompts,
        // all changes should be tracked together.
        let changes = SessionChanges::new();

        // Simulate a regular prompt recording a write
        changes.record("src/main.rs", ChangeKind::Write);

        // Simulate an @file mention prompt recording an edit
        changes.record("src/cli.rs", ChangeKind::Edit);

        // Both should be visible in the snapshot
        let snapshot = changes.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].path, "src/main.rs");
        assert_eq!(snapshot[0].kind, ChangeKind::Write);
        assert_eq!(snapshot[1].path, "src/cli.rs");
        assert_eq!(snapshot[1].kind, ChangeKind::Edit);

        // format_changes should show both
        let output = format_changes(&changes);
        assert!(output.contains("2 files"));
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("src/cli.rs"));
    }

    // --- TurnSnapshot tests ---

    #[test]
    fn test_turn_snapshot_new_is_empty() {
        let snap = TurnSnapshot::new();
        assert!(snap.is_empty());
        assert_eq!(snap.file_count(), 0);
    }

    #[test]
    fn test_turn_snapshot_save_and_restore() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "original content").unwrap();
        let path_str = path.to_str().unwrap();

        let mut snap = TurnSnapshot::new();
        snap.snapshot_file(path_str);

        assert!(!snap.is_empty());
        assert_eq!(snap.file_count(), 1);
        assert_eq!(snap.originals.get(path_str).unwrap(), "original content");

        // Simulate agent modifying the file
        std::fs::write(&path, "modified content").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "modified content");

        // Restore should revert to original
        let actions = snap.restore();
        assert_eq!(actions.len(), 1);
        assert!(actions[0].contains("restored"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original content");
    }

    #[test]
    fn test_turn_snapshot_created_files_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_file.txt");
        let path_str = path.to_str().unwrap();

        let mut snap = TurnSnapshot::new();
        // File doesn't exist yet — record as created
        snap.record_created(path_str);

        assert!(!snap.is_empty());
        assert_eq!(snap.file_count(), 1);

        // Simulate agent creating the file
        std::fs::write(&path, "new content").unwrap();
        assert!(path.exists());

        // Restore should delete it
        let actions = snap.restore();
        assert_eq!(actions.len(), 1);
        assert!(actions[0].contains("deleted"));
        assert!(!path.exists());
    }

    #[test]
    fn test_turn_snapshot_no_duplicate_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "v1").unwrap();
        let path_str = path.to_str().unwrap();

        let mut snap = TurnSnapshot::new();
        snap.snapshot_file(path_str);

        // Modify file, then snapshot again — should keep original
        std::fs::write(&path, "v2").unwrap();
        snap.snapshot_file(path_str);

        assert_eq!(snap.originals.get(path_str).unwrap(), "v1");
    }

    #[test]
    fn test_turn_snapshot_nonexistent_file() {
        let mut snap = TurnSnapshot::new();
        snap.snapshot_file("/nonexistent/path/to/file.txt");
        // Should not add to originals since file doesn't exist
        assert!(snap.originals.is_empty());
    }

    #[test]
    fn test_turn_snapshot_created_not_duplicated() {
        let mut snap = TurnSnapshot::new();
        snap.record_created("new.txt");
        snap.record_created("new.txt");
        assert_eq!(snap.created.len(), 1);
    }

    #[test]
    fn test_turn_snapshot_created_ignores_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "content").unwrap();
        let path_str = path.to_str().unwrap();

        let mut snap = TurnSnapshot::new();
        snap.snapshot_file(path_str);
        // Should not add to created since it was already snapshotted
        snap.record_created(path_str);
        assert!(snap.created.is_empty());
    }

    // --- TurnHistory tests ---

    #[test]
    fn test_turn_history_new_is_empty() {
        let hist = TurnHistory::new();
        assert!(hist.is_empty());
        assert_eq!(hist.len(), 0);
    }

    #[test]
    fn test_turn_history_push_pop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "original").unwrap();

        let mut hist = TurnHistory::new();

        let mut snap = TurnSnapshot::new();
        snap.snapshot_file(path.to_str().unwrap());
        hist.push(snap);

        assert_eq!(hist.len(), 1);

        let popped = hist.pop();
        assert!(popped.is_some());
        assert_eq!(hist.len(), 0);
    }

    #[test]
    fn test_turn_history_skips_empty_snapshots() {
        let mut hist = TurnHistory::new();
        hist.push(TurnSnapshot::new()); // empty — should be skipped
        assert!(hist.is_empty());
    }

    #[test]
    fn test_turn_history_undo_last_n() {
        let dir = tempfile::tempdir().unwrap();

        // Turn 1: modify a.txt
        let path_a = dir.path().join("a.txt");
        std::fs::write(&path_a, "a_original").unwrap();
        let mut snap1 = TurnSnapshot::new();
        snap1.snapshot_file(path_a.to_str().unwrap());

        // Turn 2: modify b.txt
        let path_b = dir.path().join("b.txt");
        std::fs::write(&path_b, "b_original").unwrap();
        let mut snap2 = TurnSnapshot::new();
        snap2.snapshot_file(path_b.to_str().unwrap());

        let mut hist = TurnHistory::new();
        hist.push(snap1);
        hist.push(snap2);
        assert_eq!(hist.len(), 2);

        // Simulate modifications
        std::fs::write(&path_a, "a_modified").unwrap();
        std::fs::write(&path_b, "b_modified").unwrap();

        // Undo last 1 — only b.txt should be restored
        let actions = hist.undo_last(1);
        assert!(!actions.is_empty());
        assert_eq!(std::fs::read_to_string(&path_b).unwrap(), "b_original");
        assert_eq!(std::fs::read_to_string(&path_a).unwrap(), "a_modified");
        assert_eq!(hist.len(), 1);

        // Undo last 1 — now a.txt should be restored
        let actions = hist.undo_last(1);
        assert!(!actions.is_empty());
        assert_eq!(std::fs::read_to_string(&path_a).unwrap(), "a_original");
        assert!(hist.is_empty());
    }

    #[test]
    fn test_turn_history_undo_more_than_available() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.txt");
        std::fs::write(&path, "orig").unwrap();

        let mut snap = TurnSnapshot::new();
        snap.snapshot_file(path.to_str().unwrap());

        let mut hist = TurnHistory::new();
        hist.push(snap);

        // Undo 5 when only 1 exists — should undo 1 without panic
        std::fs::write(&path, "changed").unwrap();
        let actions = hist.undo_last(5);
        assert!(!actions.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "orig");
        assert!(hist.is_empty());
    }

    #[test]
    fn test_turn_history_clear() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.txt");
        std::fs::write(&path, "content").unwrap();

        let mut snap = TurnSnapshot::new();
        snap.snapshot_file(path.to_str().unwrap());

        let mut hist = TurnHistory::new();
        hist.push(snap);
        assert_eq!(hist.len(), 1);

        hist.clear();
        assert!(hist.is_empty());
    }

    #[test]
    fn test_to_json_summary_empty() {
        let changes = SessionChanges::new();
        let summary = changes.to_json_summary();
        assert_eq!(summary["files_changed"], 0);
        assert!(summary["changes"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_to_json_summary_mixed_changes() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        changes.record("src/cli.rs", ChangeKind::Edit);
        changes.record("src/new.rs", ChangeKind::Write);

        let summary = changes.to_json_summary();
        assert_eq!(summary["files_changed"], 3);

        let arr = summary["changes"].as_array().unwrap();
        assert_eq!(arr.len(), 3);

        assert_eq!(arr[0]["path"], "src/main.rs");
        assert_eq!(arr[0]["kind"], "write");

        assert_eq!(arr[1]["path"], "src/cli.rs");
        assert_eq!(arr[1]["kind"], "edit");

        assert_eq!(arr[2]["path"], "src/new.rs");
        assert_eq!(arr[2]["kind"], "write");
    }

    #[test]
    fn test_to_json_summary_is_valid_json() {
        let changes = SessionChanges::new();
        changes.record("a.rs", ChangeKind::Write);
        let summary = changes.to_json_summary();
        // Round-trip through serde to verify it's valid JSON
        let serialized = serde_json::to_string(&summary).unwrap();
        let _: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    }

    #[test]
    fn test_to_json_summary_deduplicates_paths() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        changes.record("src/main.rs", ChangeKind::Edit); // overwrites

        let summary = changes.to_json_summary();
        assert_eq!(summary["files_changed"], 1);

        let arr = summary["changes"].as_array().unwrap();
        assert_eq!(arr[0]["path"], "src/main.rs");
        assert_eq!(arr[0]["kind"], "edit"); // latest kind wins
    }
}
