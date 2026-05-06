Title: Migrate prompt.rs re-exports to canonical imports (batch 4 — final two files)
Files: src/commands_session.rs, src/commands_spawn.rs, src/prompt.rs
Issue: none

## What

Complete the final batch of the re-export migration. After task 01 handles 3 files,
these are the last 2 files still using `use crate::prompt::*`.

### File-by-file changes

**src/commands_session.rs** (line 5: `use crate::prompt::*`):
- Actually uses: `ChangeKind`, `SessionChanges`, `search_messages`, `summarize_message`
- `ChangeKind`, `SessionChanges` → live in `session.rs`
- `search_messages`, `summarize_message` → live in `prompt_utils.rs`
- Replace with:
  ```rust
  use crate::session::{ChangeKind, SessionChanges};
  use crate::prompt_utils::{search_messages, summarize_message};
  ```

**src/commands_spawn.rs** (line 8: `use crate::prompt::*`):
- Actually uses: `run_prompt`, `summarize_message`
- `run_prompt` → lives in `prompt.rs`
- `summarize_message` → lives in `prompt_utils.rs`
- Replace with:
  ```rust
  use crate::prompt::run_prompt;
  use crate::prompt_utils::summarize_message;
  ```

**src/prompt.rs** — After both consumer files are migrated, remove ALL `pub use` re-export
lines (lines 14, 22, 27, 99, 111, and any others). These re-exports exist solely for
backward compatibility; with all consumers migrated to canonical imports, they are dead code.
Keep line 104's comment about `MAX_RETRIES` if it's still relevant, but remove the re-export
line above it.

### Verification

1. After editing commands_session.rs and commands_spawn.rs, run `cargo build`.
2. After removing re-exports from prompt.rs, run `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`.
3. Confirm no remaining `use crate::prompt::*` anywhere: `grep -r "use crate::prompt::\*" src/`

### Important

- This task MUST run after task 01 (depends on those files already being migrated).
- After this task, prompt.rs should have zero `pub use` re-export lines.
