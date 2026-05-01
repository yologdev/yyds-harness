Title: Extract /blame and /review from commands_git.rs into commands_git_review.rs
Files: src/commands_git.rs, src/commands_git_review.rs
Issue: none

## Goal

`commands_git.rs` is 2,602 lines with six distinct command groups: diff, undo, commit, pr,
git subcommands, review, and blame. Extract the `/review` and `/blame` handlers into a new
`commands_git_review.rs` file. These are self-contained code-review-oriented commands that
share no mutable state with the other git commands.

## What to extract

From `commands_git.rs`, move to a new `src/commands_git_review.rs`:

1. `build_review_content` function
2. `build_review_prompt` function  
3. `handle_review` function
4. `BlameArgs` struct
5. `parse_blame_args` function
6. `colorize_blame_line` function
7. `colorize_blame` function
8. `handle_blame` function

## Wiring

- Add `mod commands_git_review;` to `main.rs`
- Make the extracted functions `pub(crate)` as needed
- Update any imports in `commands_git.rs` that reference the moved items
- Update callers in `dispatch.rs` or wherever `/review` and `/blame` are dispatched to
  import from the new module instead

## What to verify

- `cargo build` clean
- `cargo test` passes — all existing tests for review and blame functionality must pass
  in their new home
- `cargo clippy --all-targets -- -D warnings` clean
- `commands_git.rs` should drop by ~500-700 lines

## Why

This is the ongoing consolidation pattern: files above ~2,000 lines get split along
concern boundaries. Review/blame are code-inspection tools, while the rest of commands_git
is about making changes (commit, diff, undo, pr). The split follows the natural read-vs-write
boundary.
