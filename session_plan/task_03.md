Title: Always inject project-type conventions into system prompt context
Files: src/context.rs
Issue: none

## Problem

Currently, `project_type_hints()` conventions (e.g., "Build: `cargo build`, Test: `cargo test`") are only injected into the system prompt when NO explicit context file (YOYO.md, CLAUDE.md, etc.) is found. This means that in any project with a YOYO.md, the agent loses the auto-detected build/test/lint commands — even if the YOYO.md doesn't mention them.

The logic is in `load_project_context()` in `src/context.rs`:
```rust
// Append project-type conventions if no explicit context file was found
if found.is_empty() {
    let project_type = detect_project_type(std::path::Path::new("."));
    if let Some(hints) = project_type_hints(&project_type) {
        ...
    }
}
```

## Implementation

Change the condition so project-type conventions are ALWAYS injected when detected, regardless of whether a context file exists. The conventions complement the context file — they don't replace it.

Specifically in `src/context.rs`, in `load_project_context()`:

1. Remove the `if found.is_empty()` guard around the project-type conventions injection.
2. Keep the conventions section header as "## Development Conventions" for clarity.
3. Inject conventions AFTER the context file content but BEFORE memories — this way the context file's instructions take precedence (they come first in the prompt), and conventions provide fallback guidance.
4. Track whether conventions were injected (the `conventions_injected` variable already exists) for the status message.

The change is small — essentially just removing the `if found.is_empty()` condition and un-indenting the block inside it.

**Tests to add/update:**
- Add a test that verifies `load_project_context()` includes conventions even when a context file exists. Create a temp directory with both a `YOYO.md` file and a `Cargo.toml` file, call `load_project_context()` from that dir, and verify the output contains both the YOYO.md content AND "cargo build" or "cargo test" hints.
- Verify existing tests still pass — the test `test_conventions_injected_when_no_context_file` (if it exists) should still work since conventions are now always injected (a superset of the previous behavior).

Run `cargo test` and `cargo clippy --all-targets -- -D warnings` to verify.
