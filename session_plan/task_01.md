Title: Extract SmartEditTool into src/smart_edit.rs
Files: src/smart_edit.rs (new), src/tool_wrappers.rs, src/main.rs
Issue: none

## What

Extract `SmartEditTool` and its helper functions (`find_nearest_match`, `extract_matched_text`) from `tool_wrappers.rs` into a new `src/smart_edit.rs` module. This addresses the assessment's explicit callout that `tool_wrappers.rs` at 3,397 lines is the largest non-data file.

## Why

`tool_wrappers.rs` contains 8 different tool decorator types. `SmartEditTool` is the most complex — it has fuzzy matching logic, whitespace-only auto-fix retry, and multi-strategy match finding. Extracting it makes the auto-fix logic easier to find, understand, and extend independently.

## How

1. Create `src/smart_edit.rs` with the following items moved from `tool_wrappers.rs`:
   - `struct SmartEditTool` (line ~660)
   - `fn find_nearest_match` (line ~666) 
   - `fn extract_matched_text` (line ~756)
   - `pub(crate) fn with_smart_edit` (line ~770)
   - `impl AgentTool for SmartEditTool` (line ~775)
   - `impl SmartEditTool` (line ~813) — the `try_auto_fix` and related methods
   - All associated tests for SmartEditTool from the `#[cfg(test)]` section

2. Add `mod smart_edit;` to `src/main.rs`

3. In `tool_wrappers.rs`, replace the moved code with `pub(crate) use smart_edit::{with_smart_edit, SmartEditTool};` or adjust imports as needed. The `with_smart_edit` function is called from `tools.rs` via `tool_wrappers::with_smart_edit`.

4. Make sure all imports in the new file are correct — SmartEditTool uses `AgentTool` from yoagent, `serde_json`, and format utilities from `crate::format`.

5. Run `cargo build && cargo test` to verify.

## Size estimate

~270 lines of code + associated tests moved. Straightforward extraction — no logic changes.
