Title: Wire RecoveryHintTool into build_tools so tool errors include recovery advice
Files: src/tools.rs
Issue: none

## Context

The `RecoveryHintTool` wrapper was built on Day 75 (this morning) in `tool_wrappers.rs` along with `ToolFailureTracker`. It's fully tested (7+ test functions). However, it is NOT wired into the actual toolbox — `build_tools()` in `src/tools.rs` never calls `with_recovery_hints()`. The wrapper exists but does nothing in production.

This directly addresses Gap #2 in the competitive priority queue: "Full graceful degradation on partial tool failures."

## What to do

1. In `src/tools.rs`, add `with_recovery_hints` to the existing import from `tool_wrappers`:
   ```rust
   use crate::tool_wrappers::{
       maybe_confirm, maybe_guard, maybe_guard_arc, with_auto_check, with_recovery_hints,
       with_truncation, ToolFailureTracker,
   };
   ```

2. In `build_tools()`, create a single shared `ToolFailureTracker` instance at the top (before building individual tools):
   ```rust
   let failure_tracker = ToolFailureTracker::new();
   ```

3. Wrap each tool with `with_recovery_hints()` as the OUTERMOST wrapper (outside `with_truncation`, which is outside `with_auto_check`). The wrapping order matters — recovery hints should see the final error after truncation, not raw output:
   - `bash` tool: `maybe_hook(with_recovery_hints(with_truncation(Box::new(bash), max_tool_output), &failure_tracker), &hooks)`
   - `read_file` tool: wrap with `with_recovery_hints(..., &failure_tracker)`
   - `write_file` tool: wrap the `with_truncation(with_auto_check(write_tool), ...)` result with `with_recovery_hints`
   - `edit_file` tool: same pattern as write_file
   - `search` tool: wrap with `with_recovery_hints`
   - `list_files` tool: wrap with `with_recovery_hints`
   - `rename_symbol` tool: wrap with `with_recovery_hints`
   
   Do NOT wrap `ask_user`, `todo`, or `sub_agent` — these are interactive/meta tools where recovery hints don't apply.

4. Verify the existing tests still pass: `cargo test`

5. Verify no clippy warnings: `cargo clippy --all-targets -- -D warnings`

## Key constraint

The `ToolFailureTracker` uses `Arc<Mutex<HashMap>>` internally, so it can be cloned/shared across all tools safely. Each `with_recovery_hints` call clones the tracker's inner Arc. A single tracker instance means cross-tool failure counts are shared — if `edit_file` fails twice and the user tries `search`, the search tool still starts at attempt 0 (counts are per tool name).

## Verification

After wiring, tool errors in production will include escalating advice:
- 1st failure: diagnostic hint (e.g., "check the path", "simplify the pattern")
- 2nd+ failure: concrete alternative (e.g., "try write_file instead of edit_file")

This is entirely transparent to the user — the hints appear in the tool error message that the LLM sees, helping it self-correct without human intervention.
