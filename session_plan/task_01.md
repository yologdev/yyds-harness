Title: Migrate prompt.rs re-exports to canonical imports (batch 3 — final)
Files: src/agent_builder.rs, src/commands_lint.rs, src/commands_run.rs
Issue: none

## What

Complete the re-export migration that has been running across Days 66-67. Five files still
use `use crate::prompt::*` to access items that actually live in `session.rs`,
`prompt_budget.rs`, `prompt_retry.rs`, `prompt_utils.rs`, and `prompt.rs` itself.

This task handles 3 of the 5 remaining files (task 02 handles the other 2).

### File-by-file changes

**src/agent_builder.rs** (line 21: `use crate::prompt::*`):
- Actually uses: `PromptOutcome`, `is_audit_enabled`, `run_prompt`, `run_prompt_with_content`
- `PromptOutcome`, `run_prompt`, `run_prompt_with_content` → genuinely live in `prompt.rs`
- `is_audit_enabled` → lives in `prompt_budget.rs`
- Replace the wildcard with:
  ```rust
  use crate::prompt::{run_prompt, run_prompt_with_content, PromptOutcome};
  use crate::prompt_budget::is_audit_enabled;
  ```

**src/commands_lint.rs** (line 6: `use crate::prompt::*`):
- Actually uses: `run_prompt`
- `run_prompt` → genuinely lives in `prompt.rs`
- Replace with:
  ```rust
  use crate::prompt::run_prompt;
  ```

**src/commands_run.rs** (line 6: `use crate::prompt::*`):
- Actually uses: `SessionChanges`, `run_prompt_auto_retry`
- `run_prompt_auto_retry` → lives in `prompt.rs`
- `SessionChanges` → lives in `session.rs`
- Replace with:
  ```rust
  use crate::prompt::run_prompt_auto_retry;
  use crate::session::SessionChanges;
  ```

### Verification

After each file change, run `cargo build` to confirm imports resolve. After all three,
run `cargo test` and `cargo clippy --all-targets -- -D warnings`.

### DO NOT

- Remove the `pub use` re-exports from `prompt.rs` yet — the remaining 2 files
  (commands_session.rs, commands_spawn.rs) still depend on them (task 02 handles those).
- Touch any other files.
