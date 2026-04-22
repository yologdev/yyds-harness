//! Diff rendering: LCS-based line diff and colored unified diff output.

use super::{DIM, GREEN, RED, RESET};

/// Maximum number of diff lines to display before truncating.
const MAX_DIFF_LINES: usize = 20;

/// Number of context lines to show around each change hunk.
const DIFF_CONTEXT_LINES: usize = 3;

/// Operations produced by the LCS diff algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DiffOp<'a> {
    Keep(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

/// Compute a line-level diff between two texts using LCS (Longest Common Subsequence).
///
/// Returns a sequence of `DiffOp`s representing keeps, deletions, and insertions.
fn compute_line_diff<'a>(old_lines: &[&'a str], new_lines: &[&'a str]) -> Vec<DiffOp<'a>> {
    let m = old_lines.len();
    let n = new_lines.len();

    // Build LCS table
    // dp[i][j] = length of LCS of old_lines[..i] and new_lines[..j]
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if old_lines[i - 1] == new_lines[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to produce diff ops
    let mut ops = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            ops.push(DiffOp::Keep(old_lines[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            ops.push(DiffOp::Insert(new_lines[j - 1]));
            j -= 1;
        } else {
            ops.push(DiffOp::Delete(old_lines[i - 1]));
            i -= 1;
        }
    }

    ops.reverse();
    ops
}

/// Format a colored unified diff between old_text and new_text.
///
/// Uses LCS-based line diffing to produce proper unified-style output with context lines.
/// Context lines (unchanged) are shown dimmed, removed lines in red with `- ` prefix,
/// added lines in green with `+ ` prefix. Non-adjacent hunks are separated by `···`.
/// If the diff exceeds `MAX_DIFF_LINES`, it is truncated with an ellipsis note.
pub fn format_edit_diff(old_text: &str, new_text: &str) -> String {
    // Handle both-empty case
    if old_text.is_empty() && new_text.is_empty() {
        return String::new();
    }

    let old_lines: Vec<&str> = if old_text.is_empty() {
        Vec::new()
    } else {
        old_text.lines().collect()
    };
    let new_lines: Vec<&str> = if new_text.is_empty() {
        Vec::new()
    } else {
        new_text.lines().collect()
    };

    let ops = compute_line_diff(&old_lines, &new_lines);

    // If everything is Keep, texts are identical
    if ops.iter().all(|op| matches!(op, DiffOp::Keep(_))) {
        return String::new();
    }

    // Assign indices and mark which ops are changes (Delete or Insert)
    let is_change: Vec<bool> = ops
        .iter()
        .map(|op| !matches!(op, DiffOp::Keep(_)))
        .collect();

    // For each op, determine if it should be shown (is a change, or within
    // DIFF_CONTEXT_LINES of a change)
    let len = ops.len();
    let mut visible = vec![false; len];
    for (idx, &changed) in is_change.iter().enumerate() {
        if changed {
            // Mark the change itself and surrounding context
            let start = idx.saturating_sub(DIFF_CONTEXT_LINES);
            let end = (idx + DIFF_CONTEXT_LINES + 1).min(len);
            for v in &mut visible[start..end] {
                *v = true;
            }
        }
    }

    // Build output lines, inserting hunk separators where there are gaps
    let mut output: Vec<String> = Vec::new();
    let mut last_visible: Option<usize> = None;

    for (idx, op) in ops.iter().enumerate() {
        if !visible[idx] {
            continue;
        }

        // Insert hunk separator if there's a gap
        if let Some(prev) = last_visible {
            if idx > prev + 1 {
                output.push(format!("{DIM}  ···{RESET}"));
            }
        }
        last_visible = Some(idx);

        match op {
            DiffOp::Keep(line) => {
                output.push(format!("{DIM}    {line}{RESET}"));
            }
            DiffOp::Delete(line) => {
                output.push(format!("{RED}  - {line}{RESET}"));
            }
            DiffOp::Insert(line) => {
                output.push(format!("{GREEN}  + {line}{RESET}"));
            }
        }
    }

    if output.is_empty() {
        return String::new();
    }

    // Truncate if too many lines
    if output.len() > MAX_DIFF_LINES {
        let remaining = output.len() - MAX_DIFF_LINES;
        output.truncate(MAX_DIFF_LINES);
        output.push(format!("{DIM}  ... ({remaining} more lines){RESET}"));
    }

    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_edit_diff_single_line_change() {
        let diff = format_edit_diff("old line", "new line");
        assert!(diff.contains("- old line"));
        assert!(diff.contains("+ new line"));
        // Should have red for removed, green for added
        assert!(diff.contains(&format!("{RED}")));
        assert!(diff.contains(&format!("{GREEN}")));
    }

    #[test]
    fn test_format_edit_diff_multi_line_change() {
        let old = "line 1\nline 2\nline 3";
        let new = "line A\nline B";
        let diff = format_edit_diff(old, new);
        assert!(diff.contains("- line 1"));
        assert!(diff.contains("- line 2"));
        assert!(diff.contains("- line 3"));
        assert!(diff.contains("+ line A"));
        assert!(diff.contains("+ line B"));
    }

    #[test]
    fn test_format_edit_diff_addition_only() {
        let diff = format_edit_diff("", "new content\nmore content");
        // No removed lines
        assert!(!diff.contains("- "));
        // Added lines present
        assert!(diff.contains("+ new content"));
        assert!(diff.contains("+ more content"));
    }

    #[test]
    fn test_format_edit_diff_deletion_only() {
        let diff = format_edit_diff("old content\nmore old", "");
        // Removed lines present
        assert!(diff.contains("- old content"));
        assert!(diff.contains("- more old"));
        // No added lines
        assert!(!diff.contains("+ "));
    }

    #[test]
    fn test_format_edit_diff_long_diff_truncation() {
        // Generate a diff with more than MAX_DIFF_LINES lines
        let old_lines: Vec<&str> = (0..15).map(|_| "old").collect();
        let new_lines: Vec<&str> = (0..15).map(|_| "new").collect();
        let old = old_lines.join("\n");
        let new = new_lines.join("\n");
        let diff = format_edit_diff(&old, &new);
        // Should be truncated — total would be 30 lines, max is 20
        assert!(diff.contains("more lines)"));
    }

    #[test]
    fn test_format_edit_diff_empty_both() {
        let diff = format_edit_diff("", "");
        assert!(diff.is_empty());
    }

    #[test]
    fn test_format_edit_diff_empty_old_text_new_file_section() {
        // Simulates adding new content to a file (old_text is empty)
        let diff = format_edit_diff("", "fn new_function() {\n    println!(\"hello\");\n}");
        assert!(!diff.contains("- "));
        assert!(diff.contains("+ fn new_function()"));
        assert!(diff.contains("+ }"));
    }

    #[test]
    fn test_format_edit_diff_short_diff_not_truncated() {
        let diff = format_edit_diff("a", "b");
        assert!(!diff.contains("more lines"));
    }

    #[test]
    fn test_format_edit_diff_context_lines_around_change() {
        // Change one line in the middle of a block — context lines should appear
        let old = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9";
        let new = "line 1\nline 2\nline 3\nline 4\nLINE FIVE\nline 6\nline 7\nline 8\nline 9";
        let diff = format_edit_diff(old, new);
        // The changed lines should be present
        assert!(diff.contains("- line 5"));
        assert!(diff.contains("+ LINE FIVE"));
        // Context lines around the change should be present (dimmed)
        assert!(diff.contains("line 3") || diff.contains("line 4"));
        assert!(diff.contains("line 6") || diff.contains("line 7"));
        // Lines far from the change should NOT appear
        assert!(!diff.contains("line 1"));
        assert!(!diff.contains("line 9"));
    }

    #[test]
    fn test_format_edit_diff_adjacent_changes_grouped() {
        // Two consecutive changed lines should appear in one hunk without separator
        let old = "keep 1\nold A\nold B\nkeep 2";
        let new = "keep 1\nnew A\nnew B\nkeep 2";
        let diff = format_edit_diff(old, new);
        assert!(diff.contains("- old A"));
        assert!(diff.contains("- old B"));
        assert!(diff.contains("+ new A"));
        assert!(diff.contains("+ new B"));
        // No hunk separator between adjacent changes
        assert!(!diff.contains("···"));
    }

    #[test]
    fn test_format_edit_diff_nonadjacent_changes_get_separator() {
        // Two changes separated by many unchanged lines should get a hunk separator
        let old = "line 1\nold A\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\nold B\nline 12";
        let new = "line 1\nnew A\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\nnew B\nline 12";
        let diff = format_edit_diff(old, new);
        assert!(diff.contains("- old A"));
        assert!(diff.contains("+ new A"));
        assert!(diff.contains("- old B"));
        assert!(diff.contains("+ new B"));
        // Should have a hunk separator between the two distant changes
        assert!(diff.contains("···"));
    }

    #[test]
    fn test_format_edit_diff_single_line_change_with_context() {
        // A single line changed, surrounded by context
        let old = "before\ntarget\nafter";
        let new = "before\nreplacement\nafter";
        let diff = format_edit_diff(old, new);
        assert!(diff.contains("- target"));
        assert!(diff.contains("+ replacement"));
        // Context should include surrounding lines
        assert!(diff.contains("before"));
        assert!(diff.contains("after"));
    }

    #[test]
    fn test_format_edit_diff_identical_texts() {
        let diff = format_edit_diff("same\ncontent\nhere", "same\ncontent\nhere");
        assert!(diff.is_empty());
    }
}
