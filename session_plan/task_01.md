Title: /outline accepts file paths — show symbols for a specific file
Files: src/commands_search.rs
Issue: none

## Problem

Running `yoyo outline src/main.rs` prints "No symbols matching 'src/main.rs' found" because
`handle_outline` treats the argument purely as a symbol-name search query. The name "outline"
strongly implies "show the structure of this file" when given a file path. This was caught in
the Day 58 self-test (9/10 — this is the failing test).

## What to do

In `handle_outline()` in `src/commands_search.rs`:

1. Before the existing symbol-name search logic, check if the query looks like a file path
   (contains `/` or `.` with a known extension like `.rs`, `.py`, `.ts`, `.js`, `.go`, etc.).
2. If it looks like a file path AND the file exists on disk, filter the repo map to only that
   file and display ALL symbols from it (no fuzzy matching needed — show everything).
3. If it looks like a file path but the file doesn't exist, fall through to the existing
   symbol-name search behavior (maybe the user meant a symbol with dots in it).
4. The file-path mode should use the same `format_outline_match` display as existing output.

## Implementation sketch

```
let query = ...; // existing parse
if (query.contains('/') || query.contains('.')) && std::path::Path::new(query).exists() {
    // File path mode: show all symbols from this specific file
    let entries = build_repo_map(...);
    let file_entries: Vec<_> = entries.iter()
        .filter(|e| e.path == query || e.path.ends_with(query))
        .collect();
    if !file_entries.is_empty() {
        // Display all symbols from matching files
        // ... format and print
        return;
    }
}
// Fall through to existing symbol search
```

## Tests

Add a test that calls `handle_outline("/outline src/main.rs")` or the underlying function
with a known file path and verifies it returns symbols (not "no symbols found"). Also test
that a non-existent path falls through to symbol search.

## Verification

`cargo build && cargo test` must pass. After implementation, `yoyo outline src/main.rs`
should list the functions/structs in main.rs.
