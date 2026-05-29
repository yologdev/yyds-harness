Title: Fix byte-index safety violation in test code and add safe_truncate to format prelude
Files: src/commands_project.rs, src/format/mod.rs
Issue: none

The assessment found a byte-index safety violation in test code at `commands_project.rs:1717`:
```rust
&content[..200.min(content.len())]
```

While this is currently safe (test content is ASCII), it violates the project's own safety
rule documented in CLAUDE.md. Fix it to use `safe_truncate` from `format/mod.rs`.

Specific changes:

1. In `src/commands_project.rs` test `test_generate_init_content_rust_project` (~line 1717):
   - Replace `&content[..200.min(content.len())]` with a call to `safe_truncate(&content, 200)`
   - Add `use crate::format::safe_truncate;` in the test module if not already imported

2. Search for any other `&content[..` or `&s[..` byte-slice patterns in test code across
   the codebase and fix any that don't use `is_char_boundary()` or `safe_truncate()`.
   Only fix patterns in test code (inside `#[cfg(test)]` modules) — don't touch production
   code in this task.

3. In `src/format/mod.rs`, verify that `safe_truncate` is `pub` and accessible from other
   modules. It should already be — just confirm.

Run `cargo test` after changes to verify nothing breaks. Run `cargo clippy --all-targets -- -D warnings`
to check for any new warnings.
