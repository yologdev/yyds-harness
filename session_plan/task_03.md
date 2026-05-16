Title: Data-driven language extractor table in commands_map.rs — Phase 1: consolidate regex patterns
Files: src/commands_map.rs
Issue: none

## Problem

`commands_map.rs` is the largest source file at 3,291 lines and still growing (5 languages were added this session alone). The 16 language extractors in `extract_symbols` share an identical structure: compile a set of regex patterns, iterate over lines, match patterns, push `Symbol` structs. The per-language logic differs only in the regex patterns and which `SymbolKind` they map to.

## Solution — Phase 1 only

This is a large refactor. This task handles ONLY the first step: create a data-driven `LanguagePatterns` struct and convert 3-4 of the simplest languages to use it (e.g., Go, Ruby, Shell — languages with few special cases).

### Specific steps:

1. Define a struct:
```rust
struct PatternRule {
    regex: &'static str,
    kind: SymbolKind,
    // which capture group is the name
    name_group: usize,
}

struct LanguagePatterns {
    patterns: &'static [PatternRule],
}
```

2. Add a helper function `extract_symbols_from_patterns(content: &str, patterns: &LanguagePatterns) -> Vec<Symbol>` that does the generic line-by-line extraction.

3. Convert Go, Ruby, and Shell extractors to use this table-driven approach. These are good candidates because they have straightforward patterns with no special nesting logic.

4. Leave Rust, Python, JS/TS, Java, C/C++, and the newly-added languages unchanged for now — they have more complex extraction logic (nesting, decorators, namespaces).

5. Add tests that verify the table-driven extraction produces the same results as the old code for Go, Ruby, and Shell.

### What NOT to do:
- Don't convert all 16 languages in one task
- Don't change the public API (`extract_symbols`, `build_repo_map`, etc.)
- Don't split into a separate file yet — that's a future Phase 2

## Verification

```bash
cargo test --lib commands_map
cargo test  # full suite  
cargo clippy --all-targets -- -D warnings
```

Verify that existing `/map` tests still pass unchanged — the output should be identical for Go, Ruby, and Shell files.
