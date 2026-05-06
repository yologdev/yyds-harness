Title: Migrate prompt.rs re-exports to canonical imports (batch 2: commands_dev, commands_git, commands_git_review, commands_lint)
Files: src/commands_dev.rs, src/commands_git.rs, src/commands_git_review.rs
Issue: none

## Context

Day 66 migrated batch 1 (repl.rs, dispatch.rs, conversations.rs) from importing through
`prompt.rs` re-exports to canonical module imports. 15 files still use `use crate::prompt::*`
or import specific items through prompt.rs that actually live in other modules.

The prompt.rs file currently re-exports:
- `crate::watch::{get_watch_command, run_watch_after_prompt, set_watch_command}`
- `crate::prompt_budget::{audit_log_tool_call, enable_audit_log, is_audit_enabled}`
- `crate::session::{format_changes, ChangeKind, SessionChanges, TurnHistory}`
- `crate::prompt_utils::{search_messages, summarize_message, write_output_file}`
- `crate::prompt_retry::build_retry_prompt`

## What to do

For these 3 files that use `use crate::prompt::*`:
- `src/commands_dev.rs`
- `src/commands_git.rs`
- `src/commands_git_review.rs`

1. Find what each file actually uses from prompt.rs (grep for function/type names from
   prompt.rs and its re-exports).
2. Replace `use crate::prompt::*` with explicit imports from canonical modules:
   - Items defined in prompt.rs itself (like `run_prompt`, `run_prompt_auto_retry`, 
     `run_prompt_with_content`, `PromptOutcome`) → keep as `use crate::prompt::{...}`
   - Items from session.rs → `use crate::session::{...}`
   - Items from prompt_budget.rs → `use crate::prompt_budget::{...}`
   - Items from prompt_utils.rs → `use crate::prompt_utils::{...}`
   - Items from watch.rs → `use crate::watch::{...}`
   - Items from prompt_retry.rs → `use crate::prompt_retry::{...}`
3. Verify with `cargo build && cargo test`.

Do NOT remove the re-exports from prompt.rs yet — other files still depend on them.
Just migrate these 3 consumers to canonical imports.
