Title: Integration test for sub-agent SharedState round-trip
Files: tests/integration.rs
Issue: #347

## What to do

Add an integration test that validates the SharedState wiring introduced in the Day 58 Session 3
commits (7d2be5a). The test should exercise the **dry-run path** — creating a sub-agent tool
with SharedState, writing a value from the parent side, and verifying the SharedState is
accessible and the tool names include `shared_state`.

### Specific steps

1. In `tests/integration.rs`, add a test function `test_shared_state_sub_agent_roundtrip`.

2. The test should:
   - Call `build_sub_agent_tool` (from `src/tools.rs`) and receive back both the `SubAgentTool` and the `SharedState`.
   - Write a key-value pair to the `SharedState` from the parent side (e.g., `state.set("test.key", "test_value")`).
   - Read back the value and assert it matches.
   - Verify that `BUILTIN_TOOL_NAMES` contains `"shared_state"`.

3. This is a **unit-level integration test** — it does NOT need to actually run an agent or make API calls. It validates the wiring: that `build_sub_agent_tool` returns a working SharedState, and that the parent can read/write to it.

4. Check how existing tests in `tests/integration.rs` are structured and follow the same patterns (imports, test attributes, etc.).

5. Also check the existing SharedState tests in `src/tools.rs` (lines ~1446-1490) to avoid duplicating what's already tested there. The integration test should test something the unit tests don't — specifically the round-trip through `build_sub_agent_tool`'s public API as called from outside the module.

### Acceptance criteria
- `cargo test test_shared_state` passes
- `cargo build && cargo test && cargo clippy --all-targets -- -D warnings` all green
- No modifications to production code — test-only change
