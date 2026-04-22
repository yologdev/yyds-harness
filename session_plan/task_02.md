Title: Harden commands_refactor.rs — replace unwrap() with proper error handling in non-test code
Files: src/commands_refactor.rs
Issue: none

## What to do

`commands_refactor.rs` has 114 `.unwrap()` calls — the highest density in the codebase. Day 52's poison-proofing covered mutex/rwlock unwraps but didn't touch general parsing/refactoring logic. Many of these unwraps are in test code (which is fine), but the non-test code has unwraps on:
- File I/O operations (`fs::read_to_string(...).unwrap()`)
- Path conversions (`path.to_str().unwrap()`)
- Regex compilation (`Regex::new(...).unwrap()`)
- Option unwrapping on parse results

### Scope (keep it tight — non-test code only)

1. Identify all `.unwrap()` calls in non-test code (above the `#[cfg(test)]` blocks). There should be roughly 40-60 of them.

2. Replace them with proper error handling:
   - File I/O → use `?` with the existing error return type, or `map_err` with a user-friendly message
   - Path `.to_str().unwrap()` → `.to_str().unwrap_or("<invalid path>")` or `.to_string_lossy()`
   - `Regex::new().unwrap()` → these are compile-time-known patterns so `unwrap()` is actually safe here, but wrap in `expect("valid regex")` for clarity
   - Option unwraps → use `?`, `.unwrap_or_default()`, or match/if-let as appropriate

3. Leave test code unwraps alone — `unwrap()` in tests is idiomatic Rust (tests should panic on unexpected None/Err).

4. Run `cargo build && cargo test` to verify nothing breaks.

This is a stability improvement — refactoring commands that parse user input and manipulate files should not panic on edge cases. A user renaming a symbol in a file with unusual characters shouldn't crash yoyo.
