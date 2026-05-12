Title: Show colored diff preview for write_file when overwriting existing files
Files: src/prompt.rs, src/format/diff.rs
Issue: none

## What

Currently, `edit_file` operations display a colored diff showing exactly what changed:
```
  ▶ edit src/main.rs (5 → 7 lines)
    - old line
    + new line
```

But `write_file` just shows:
```
  ▶ write src/main.rs (42 lines)
```

When `write_file` overwrites an existing file, show a diff preview so the user can see what changed. This closes the "multi-file edit visualization" competitive gap — making write operations as transparent as edit operations.

## Changes

### 1. `src/prompt.rs` — `handle_tool_execution_start`

After the existing edit_file diff display block (around line 257), add a similar block for write_file:

```rust
} else if tool_name == "write_file" {
    // Show diff when overwriting an existing file
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let new_content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
    if !path.is_empty() && std::path::Path::new(path).exists() {
        if let Ok(old_content) = std::fs::read_to_string(path) {
            let diff = format_edit_diff(&old_content, new_content);
            if !diff.is_empty() {
                // Truncate long diffs to avoid flooding terminal
                let diff = truncate_diff_preview(&diff, 30);
                println!();
                println!("{diff}");
            }
        }
    }
}
```

Key details:
- Only show diff when the file already exists (new files just show line count)
- Only show diff when content differs (avoid empty diff noise)
- Truncate to 30 lines max — if the diff is longer, show first 28 lines + "... (N more lines)"
- Read the file at ToolExecutionStart (before the tool writes) so we see the BEFORE state
- If file read fails (permissions, binary), silently skip the diff

### 2. `src/format/diff.rs` — add `truncate_diff_preview`

Add a helper function:
```rust
pub fn truncate_diff_preview(diff: &str, max_lines: usize) -> String
```

- Count lines in the diff output
- If ≤ max_lines, return unchanged
- If > max_lines, take first (max_lines - 2) lines and append a dimmed "... (N more lines)" summary
- Use `safe_truncate` pattern for any string slicing to avoid UTF-8 panics

Add tests:
- Short diff passes through unchanged
- Long diff gets truncated with correct count
- Empty diff returns empty string

### 3. Tests in `src/prompt.rs`

Add a test verifying that the write_file diff preview path doesn't panic:
- Test with a temp file that exists, verify diff is generated
- Test with a non-existent path, verify no diff (graceful skip)

## Rules
- Import `format_edit_diff` is already in scope in prompt.rs (line 261 uses it)
- Use `std::fs::read_to_string` for simplicity — this is a display path, not performance-critical
- The diff is purely cosmetic (display only) — it doesn't affect tool execution
- Don't block tool execution if file read fails
- Run `cargo test` to verify all pass
- Run `cargo clippy --all-targets -- -D warnings` to verify no warnings
