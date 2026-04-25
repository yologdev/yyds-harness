Title: Smart /add truncation for large files
Files: src/commands_file.rs, src/format/output.rs
Issue: none

## What

When `/add` is used on a large file without a line range, intelligently truncate the content with head+tail preservation and a clear omission marker instead of injecting the entire file into context. This closes the "intelligent file truncation" gap identified in the assessment.

## Why

Currently `/add bigfile.rs` reads the entire file and injects it as-is into the conversation. For a 5,000+ line file, this wastes context tokens and can push the agent toward compaction. Claude Code has smart truncation for large files in context. We already have `truncate_tool_output` for tool results, but `/add` bypasses that since it injects content directly as a user message.

## Implementation

### 1. Add `smart_truncate_for_context` to `src/format/output.rs`
Create a file-aware truncation function that's smarter than the generic tool output truncator:

```rust
/// Maximum lines to include when auto-truncating a large file for /add.
const ADD_MAX_LINES: usize = 500;
const ADD_HEAD_LINES: usize = 200;
const ADD_TAIL_LINES: usize = 100;

/// Truncate file content for context injection. Preserves head and tail
/// with a clear omission marker showing what was skipped.
/// Returns (truncated_content, was_truncated, original_line_count).
pub fn smart_truncate_for_context(content: &str, max_lines: usize) -> (String, bool, usize) {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    
    if total <= max_lines {
        return (content.to_string(), false, total);
    }
    
    let head_lines = (max_lines * 2) / 5;  // 40% for head
    let tail_lines = max_lines / 5;         // 20% for tail
    let omitted = total - head_lines - tail_lines;
    
    let mut result = String::new();
    for line in &lines[..head_lines] {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str(&format!(
        "\n[... {} lines omitted ({} total) — use /add file:START-END for specific sections ...]\n\n",
        omitted, total
    ));
    for (i, line) in lines[total - tail_lines..].iter().enumerate() {
        result.push_str(line);
        if i < tail_lines - 1 {
            result.push('\n');
        }
    }
    
    (result, true, total)
}
```

### 2. Use in `src/commands_file.rs` `handle_add`
In `handle_add`, after reading the file content (when no line range was specified), apply `smart_truncate_for_context`:

```rust
// In the None range branch of read_file_for_add or in handle_add after reading:
let (content, was_truncated, total_lines) = smart_truncate_for_context(&content, ADD_MAX_LINES);
if was_truncated {
    println!("  {DIM}(truncated from {total_lines} lines — use /add {path}:START-END for specific sections){RESET}");
}
```

When a line range IS specified (e.g., `/add file.rs:100-200`), skip truncation — the user explicitly chose what they want.

### 3. Print feedback
When truncation happens, show the user:
```
  📎 Added bigfile.rs (truncated: 200 head + 100 tail of 5,432 lines)
     Use /add bigfile.rs:100-300 to add specific sections
```

This teaches users about the line-range feature while being transparent about what was included.

### Tests
- Test `smart_truncate_for_context` with content under limit (no truncation)
- Test with content over limit (truncated, correct head/tail sizes)
- Test omission marker contains correct line counts
- Test edge cases: empty content, exactly at limit, 1 line over
- Test that line-range `/add` still works without truncation

### Docs
- Update docs/src/usage/commands.md if it documents /add behavior
