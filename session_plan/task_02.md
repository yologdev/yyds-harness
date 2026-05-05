Title: Migrate prompt.rs re-exports to canonical imports (batch 1: repl.rs, dispatch.rs, conversations.rs)
Files: src/prompt.rs, src/repl.rs, src/dispatch.rs
Issue: none

prompt.rs lines 1-63 are a re-export façade that re-exports symbols from `session`, `watch`,
`prompt_budget`, `prompt_retry`, and `prompt_utils` so that `use crate::prompt::*` keeps working.
This makes the canonical location of symbols unclear and creates coupling.

**This task:** Update 3 callers to import from canonical modules, then remove the re-exports
that are no longer needed. Only remove a re-export if NO remaining caller depends on it via
`crate::prompt::*` or `crate::prompt::<symbol>`.

**Step 1 — Update `src/repl.rs`:**
- Currently has `use crate::prompt::*;` (line 11)
- Check which re-exported symbols repl.rs actually uses (likely: `run_prompt_auto_retry`,
  `run_prompt_with_content_and_changes`, `SessionChanges`, `TurnHistory`, `TurnSnapshot`,
  `format_changes`, `ChangeKind`, `get_watch_command`, `set_watch_command`, `run_watch_after_prompt`,
  `session_budget_exhausted`, etc.)
- Replace the wildcard with explicit imports from canonical modules:
  - `use crate::session::{SessionChanges, TurnHistory, TurnSnapshot, ChangeKind, format_changes};`
  - `use crate::watch::{get_watch_command, set_watch_command, run_watch_after_prompt};`
  - `use crate::prompt::{run_prompt_auto_retry, ...};` (for things that live in prompt.rs itself)
  - `use crate::prompt_budget::session_budget_exhausted;`

**Step 2 — Update `src/dispatch.rs`:**
- Currently has `use crate::prompt::*;` (line 17)
- Same approach: replace wildcard with explicit imports from canonical modules

**Step 3 — Update `src/conversations.rs`:**
- Currently has `use crate::prompt::*;` (line 10)
- Same approach

**Step 4 — Remove re-exports from prompt.rs:**
- After updating these 3 files, check if any remaining `use crate::prompt::*` callers
  still depend on each re-export. Use `cargo build` to verify — if it compiles, the
  re-export was safely removed. If not, keep it for a future batch.
- Be conservative: only remove re-exports where ALL users now import directly.

**Important:** This is a batch-1 incremental cleanup. Don't try to update all 13+ callers
at once. The remaining callers (commands_git.rs, commands_lint.rs, etc.) will be updated
in future sessions.

**Verification:** `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
No behavior changes — pure import path cleanup.
