Title: Add self-written code percentage to /version -v and /status
Files: src/commands_info.rs, src/cli.rs
Issue: none

## Motivation

Aider prominently advertises "88% of our code is self-written by Aider" as a credibility signal.
yoyo is actually **100% self-written** (every line in `src/` is authored by `yoyo-evolve[bot]` 
per `git blame`). This is a powerful differentiator that should be visible to users.

## Implementation

### 1. Add a `compute_self_written_pct()` function in `commands_info.rs`

```rust
/// Compute the percentage of source code lines written by the bot (yoyo-evolve).
/// Returns (self_written_lines, total_lines, percentage) or None if git blame fails.
pub fn compute_self_written_pct() -> Option<(usize, usize, f64)> {
    // Run: git blame --line-porcelain src/*.rs src/format/*.rs | grep "^author "
    // Count lines by "yoyo-evolve" vs total
    // This is O(codebase) so only run on explicit request, not in hot paths
}
```

Use `std::process::Command` to run `git blame --line-porcelain` across all `src/**/*.rs` files,
count lines where author contains "yoyo" or the bot username, compute percentage.

Handle errors gracefully: if git blame fails (not a git repo, no git installed), return None.

### 2. Show in `/version -v` (handle_version_verbose in commands_info.rs)

After the existing version/provider/model/yoagent lines, add:
```
  self-written: 100.0% (62,886 / 62,886 lines)
```

Only show if `compute_self_written_pct()` returns Some. This keeps the command fast for
non-git contexts.

### 3. Show in `/status` (handle_status in commands_info.rs)

Add a `self-written` line after the existing status fields, same format.
Cache the result for the session duration so repeated `/status` calls don't re-run git blame.

### 4. Add test

Test `compute_self_written_pct()` in a temp git repo with known authorship.
Test that the function returns None when not in a git repo.

## Notes
- Do NOT add this to the startup banner — it's a git blame operation and would slow startup
- Do NOT compute at build time — it needs to work in any git repo, not just yoyo's
- Cache using a `LazyLock<Option<(usize, usize, f64)>>` or similar so it's computed once per session
