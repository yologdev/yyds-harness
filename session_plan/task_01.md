Title: Extract tool-wrapping infrastructure from tools.rs into src/tool_wrappers.rs
Files: src/tool_wrappers.rs (new), src/tools.rs
Issue: none

## What

Extract the tool-wrapping/decorator types and their helper functions from `tools.rs` into a new `src/tool_wrappers.rs` module. These are a self-contained concern — generic wrappers that add behavior (guarding, truncation, confirmation, Arc-based guarding) around any tool.

## What to extract

The following items form a cohesive "tool decorator" concern:

1. **`GuardedTool`** struct + its `Tool` impl — wraps a tool with a permission check
2. **`TruncatingTool`** struct + its `Tool` impl — wraps a tool to truncate output
3. **`ArcGuardedTool`** struct + its `Tool` impl — like GuardedTool but with Arc<Mutex<>> for shared state
4. **`ConfirmTool`** struct + its `Tool` impl — wraps a tool to ask for confirmation before execution
5. **`maybe_guard`** helper function — conditionally wraps a tool in GuardedTool
6. **`maybe_truncate`** helper function — conditionally wraps a tool in TruncatingTool
7. **`maybe_confirm`** helper function — conditionally wraps a tool in ConfirmTool
8. **`truncate_result`** function — the truncation logic used by TruncatingTool
9. **`describe_file_operation`** and **`confirm_file_operation`** functions — used by ConfirmTool

## What stays in tools.rs

- `StreamingBashTool`, `RenameSymbolTool`, `AskUserTool`, `TodoTool` — these are concrete tool implementations, not wrappers
- `build_tools`, `build_sub_agent_tool` — the builder functions that compose everything
- All tests that test the concrete tools stay; tests for wrapper behavior move

## How

1. Create `src/tool_wrappers.rs` with all the wrapper types, their Tool impls, and the helper functions
2. Add `pub mod tool_wrappers;` to `main.rs`
3. Update `tools.rs` to `use crate::tool_wrappers::*;` (or specific imports) so `build_tools` can still use them
4. Move any tests that specifically test wrapper behavior to the new file
5. Run `cargo build && cargo test` to verify
6. Run `cargo clippy --all-targets -- -D warnings` to verify no warnings

## Sizing

~500 lines of extraction. No logic changes. All existing tests must pass unchanged. The wrapper types are completely self-contained — they depend on `yoagent::Tool` trait and the format/config modules, not on any concrete tool types.

## CLAUDE.md update

Add `tool_wrappers.rs` to the multi-file agent list with description: "Tool decorator types (GuardedTool, TruncatingTool, ConfirmTool, ArcGuardedTool) and helper wrappers"
