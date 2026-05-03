Title: Extract side/quick/extended conversation handlers from repl.rs into src/conversations.rs
Files: src/repl.rs, src/conversations.rs, src/main.rs
Issue: none

## What

`repl.rs` is 2,107 lines — the second-largest non-format file. Lines 960-1319 contain three self-contained conversation handlers (`handle_side`, `handle_quick`, `handle_extended`) plus `build_add_content_blocks`. These are independent features that can live in their own module, reducing repl.rs by ~460 lines.

## Steps

1. **Create `src/conversations.rs`** — move these functions from `repl.rs`:
   - `build_add_content_blocks` (line ~861, helper for building content from /add results)
   - `handle_side` (line ~960, side conversation with separate agent)
   - `handle_quick` (line ~1062, quick one-shot prompt)
   - `handle_extended` (line ~1221, extended multi-turn conversation)

2. **Update imports in `conversations.rs`** — the handlers use:
   - `agent_builder::{AgentConfig, build_side_agent}`
   - `prompt::{run_prompt, run_prompt_with_content}`
   - `format` utilities (Color, cost display, etc.)
   - `commands::AddResult`
   - `yoagent::types::Content`
   
   Add all necessary `use` statements. Functions should be `pub(crate)`.

3. **Update `src/repl.rs`** — remove the moved functions, add `use crate::conversations::*` where needed. The `run_repl` function calls these handlers, so it needs the import.

4. **Update `src/main.rs`** — add `mod conversations;` declaration.

## Important

- Do NOT move any tests. Tests that reference the moved functions should stay in repl.rs and use `crate::conversations::` imports, OR move only if they test the moved functions in isolation.
- Actually, check if any tests in repl.rs test these specific functions. If so, move those tests too into conversations.rs.
- Keep `complete_file_path`, `needs_continuation`, `collect_multiline_rl`, and `run_repl` in repl.rs — they're core REPL infrastructure.

## Verify

`cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
