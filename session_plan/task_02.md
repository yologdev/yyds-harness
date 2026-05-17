Title: Consolidate per-language symbol extractors to table-driven LanguagePatterns — C#, PHP, Kotlin
Files: src/commands_map.rs
Issue: none

## Goal

Reduce `commands_map.rs` line count and improve maintainability by converting the three
newest per-language extraction functions (`extract_csharp_symbols`, `extract_php_symbols`,
`extract_kotlin_symbols`) to use the existing `LanguagePatterns` table-driven approach
(already used by Go, Ruby, Shell).

## Current state

The file has a `LanguagePatterns` struct:
```rust
struct LanguagePatterns {
    // regex patterns for different symbol kinds
}
fn extract_symbols_from_patterns(code: &str, patterns: &LanguagePatterns) -> Vec<Symbol>
```

Go, Ruby, and Shell use this. But C#, PHP, and Kotlin each have their own
~60-100 line dedicated functions with inline regex patterns. These functions follow
the same pattern: compile regexes, iterate lines, match, push Symbol.

## Implementation

1. Create `static CSHARP_PATTERNS: LanguagePatterns`, `static PHP_PATTERNS: LanguagePatterns`,
   and `static KOTLIN_PATTERNS: LanguagePatterns` — each initialized with the same regex
   patterns currently used in their respective `extract_*_symbols` functions.

2. Update the `extract_symbols` match to route these three languages through
   `extract_symbols_from_patterns(code, &*_PATTERNS)` instead of their dedicated functions.

3. Delete the now-unused `extract_csharp_symbols`, `extract_php_symbols`, and
   `extract_kotlin_symbols` functions.

4. Verify that existing tests for these languages still pass (the test expectations
   should be identical since the patterns are the same).

## Constraints

- Only modify `src/commands_map.rs`
- Existing tests for C#/PHP/Kotlin symbol extraction MUST continue to pass unchanged
- If any language has extraction logic that doesn't fit cleanly into `LanguagePatterns`
  (e.g., special multi-line handling), leave that language as-is and document why
- Must pass `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
- Net line reduction should be 100+ lines (each function is ~60-100 lines, replacing
  with ~15-20 line static declaration)

## Tests

No new tests needed — existing tests cover symbol extraction for these languages.
Run existing tests to verify no regression.
