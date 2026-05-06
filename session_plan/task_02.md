Title: Migrate commands.rs re-exports batch 1: auto_compact_if_needed to canonical imports
Files: src/commands_dev.rs, src/commands_git_review.rs, src/commands_lint.rs
Issue: none

## Context

`commands.rs` has 25 `pub use` re-exports acting as a middleman. Five files import
`auto_compact_if_needed` through `commands.rs` when it actually lives in `commands_session.rs`.
This batch migrates 3 of those 5 consumers (the other 2 — commands_plan.rs and commands_run.rs
— will be a future batch to stay within the 3-file rule).

## What to do

In these 3 files:
- `src/commands_dev.rs` — change `use crate::commands::auto_compact_if_needed;` to `use crate::commands_session::auto_compact_if_needed;`
- `src/commands_git_review.rs` — change `use crate::commands::auto_compact_if_needed;` to `use crate::commands_session::auto_compact_if_needed;`
- `src/commands_lint.rs` — change `use crate::commands::auto_compact_if_needed;` to `use crate::commands_session::auto_compact_if_needed;`

Do NOT remove the re-export from commands.rs yet — commands_plan.rs and commands_run.rs still
depend on it. That will be cleaned up in a future batch.

Verify with `cargo build && cargo test`.
