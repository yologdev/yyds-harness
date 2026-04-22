Title: Enrich exit summary with tokens, cost, and duration
Files: src/commands_retry.rs, src/repl.rs
Issue: none

## What to do

When a user quits yoyo (Ctrl-C, Ctrl-D, or `/quit`), the exit summary currently shows only file changes:

```
Session: 3 files changed (2 edited, 1 written)
```

Claude Code shows a richer exit summary including tokens used, estimated cost, and session duration. Enhance yoyo's exit summary to match.

### 1. Modify `format_exit_summary` in `commands_retry.rs`

Change the signature to accept additional data:

```rust
pub fn format_exit_summary(
    changes: &SessionChanges,
    session_total: &Usage,
    model: &str,
    session_start: std::time::Instant,
) -> Option<String>
```

The function currently returns `None` if no files were changed. Change it to return `Some(...)` if EITHER files were changed OR tokens were used (i.e., the user actually had a conversation). This means even a pure Q&A session without file changes gets a summary.

Build the summary as a compact multi-line box:

```
  ─── Session Summary ───
  Duration: 4m 32s
  Tokens:   12,450 in / 3,200 out
  Cost:     ~$0.05
  Files:    3 changed (2 edited, 1 written)
  ────────────────────────
```

Use the existing helpers:
- `format_duration` from `format/cost.rs` for duration
- `format_token_count` from `format/cost.rs` for tokens  
- `estimate_cost` and `format_cost` from `format/cost.rs` for cost
- If cost estimation isn't available for the model, omit the cost line

### 2. Update the call site in `repl.rs`

The exit summary is generated around line 1307 in `run_repl`. Pass the additional parameters (`session_total`, model name, and session start time) to `format_exit_summary`.

Track `session_start` — there should already be an `Instant` created near the start of the REPL loop. If not, add `let session_start = std::time::Instant::now();` near the beginning of `run_repl`.

### 3. Update existing tests

Update the test `test_handle_changes_with_entries_does_not_panic` (and any other tests) in `commands_retry.rs` to pass the new parameters. Add a new test:
- `test_exit_summary_with_tokens_no_files` — verify that a session with tokens used but no file changes still produces a summary
- `test_exit_summary_with_files_and_cost` — verify the full summary includes all sections

### Verification

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

Keep the summary compact — no more than 6 lines. Use `DIM` color for the box frame and labels, `GREEN` for the values. Match the visual style of `/profile` output if possible.
