Title: Add --stat flag to /diff for compact diffstat view
Files: src/commands_git.rs, src/commands.rs
Issue: none

## What to do

`/diff` currently shows the full diff output (or `--name-only` for just filenames). Add a `--stat` flag that shows a compact diffstat summary — the same format as `git diff --stat` but with yoyo's colorized formatting.

The infrastructure already exists: `parse_diff_stat()` and `format_diff_stat()` are already implemented in `commands_git.rs` and are currently used internally for staged diff summaries. This task just wires them up as a user-facing option.

### 1. Add `stat_only` to `DiffOptions` (commands_git.rs)

Add a `stat_only: bool` field to the `DiffOptions` struct. Parse `--stat` in `parse_diff_args`:

```rust
"--stat" => stat_only = true,
```

### 2. Handle `--stat` in `handle_diff` (commands_git.rs)

When `stat_only` is true, run `git diff --stat` (or `git diff --cached --stat` if `--staged` is also set) and format the output using `format_diff_stat(parse_diff_stat(&output))`.

If a specific file is provided, pass it to the git command as well.

The flow should be:
1. If `stat_only` → run `git diff --stat [--cached] [-- file]`, parse and format, return
2. Otherwise → existing behavior (full diff or name-only)

### 3. Add `--stat` to DIFF_FLAGS (commands.rs)

Add `"--stat"` to the `DIFF_FLAGS` constant so tab-completion works:

```rust
pub const DIFF_FLAGS: &[&str] = &["--staged", "--cached", "--name-only", "--stat"];
```

### 4. Add tests

Add tests in the `#[cfg(test)]` module of `commands_git.rs`:

- `test_parse_diff_args_stat` — verify `--stat` sets `stat_only: true`
- `test_parse_diff_args_staged_stat` — verify `--staged --stat` sets both flags
- `test_parse_diff_stat_parsing` — if not already tested, verify `parse_diff_stat` handles typical `git diff --stat` output correctly (there are likely existing tests for this)

### Verification

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

This is a small, focused feature that makes `/diff` more practical for quick change reviews — you can see "3 files changed, 15 insertions, 7 deletions" without scrolling through the full diff.
