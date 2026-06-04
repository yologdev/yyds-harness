Title: Error-driven file suggestions — parse error output to suggest relevant files
Files: src/watch.rs, src/commands_file.rs
Issue: none

## What

When watch mode detects failures, the error output often references specific files (e.g., `src/foo.rs:42: error[E0308]`). Currently yoyo just shows the error and tries to fix it. This task adds a small utility that extracts file paths from error output and could suggest them for context — and more immediately, uses them to enrich the fix prompt so the agent knows which files to focus on.

## Why

"Richer context loading" is the #3 actionable gap in the assessment. This is the simplest, most practical form: when errors tell you exactly which files are involved, use that information. This makes the watch-fix loop more effective by directing the agent's attention to the right files.

## How

### In `src/commands_file.rs`:
1. Add a function `extract_file_paths_from_output(output: &str) -> Vec<String>` that:
   - Uses a regex to find patterns like `path/to/file.ext:LINE` or `path/to/file.ext(LINE)` or `--> path/to/file.ext:LINE:COL`
   - Filters to only files that actually exist on disk (`Path::new(f).exists()`)
   - Deduplicates
   - Returns sorted, unique file paths
   - Handles common compiler output formats: Rust (`--> src/foo.rs:42:10`), TypeScript (`src/foo.ts(42,10)`), Python (`File "src/foo.py", line 42`), Go (`./foo.go:42:10`)

2. Add tests for `extract_file_paths_from_output`:
   - Rust error format
   - TypeScript error format  
   - Python traceback format
   - Deduplication
   - Non-existent files are filtered out

### In `src/watch.rs`:
3. In `build_watch_fix_prompt`, after building the fix prompt, call `extract_file_paths_from_output` on the error output. If files are found, append a line to the prompt like:
   `"\n\nFiles referenced in errors: {paths_joined_by_comma}. Focus your fixes on these files."`

4. This is a 3-5 line change in `build_watch_fix_prompt`.

## Constraints
- Only touch `src/watch.rs` and `src/commands_file.rs`
- The regex should be simple and correct, not exhaustive — we can add formats later
- Do NOT auto-load file contents (that would bloat context) — just mention the paths
- Tests for path extraction should NOT depend on files existing on disk (test the regex parsing separately, test the filtering with a note about it)
