Title: Add unit tests for tool_wrappers.rs — AutoCheckTool, TruncatingTool, RecoveryHintTool
Files: src/tool_wrappers.rs
Issue: none

## What to do

Expand test coverage for `tool_wrappers.rs`, which is at 2.13% test density (36 tests for 1,688 lines). This is one of the most critical files — every tool invocation flows through these wrappers. Focus on the wrapper types that have the least coverage.

**Tests to add:**

### TruncatingTool / truncate_result
- Test `truncate_result` with output under the limit (should pass through unchanged)
- Test `truncate_result` with output over the limit (should truncate with message)
- Test `truncate_result` with multi-byte UTF-8 characters near the boundary (safety check — no panics)
- Test `truncate_result` with empty input
- Test `with_truncation` wraps a tool correctly and the name/description pass through

### AutoCheckTool
- Test `with_auto_check` wraps a tool and preserves name/description
- Test that `AutoCheckTool` description includes the auto-check hint text
- Test behavior: when inner tool returns success, auto-check should run the watch command (mock via construction)

### RecoveryHintTool
- Test `with_recovery_hints` wraps a tool and preserves name/description
- Test that `RecoveryHintTool` tracks failures and adds hints after threshold
- Test recovery hint escalation: first failure gets diagnostic hint, second failure gets alternative suggestion
- Test that success resets the failure counter
- Test `ToolFailureTracker::new` initializes correctly
- Test tracker records failures and successes correctly

### GuardedTool / ArcGuardedTool
- Test `maybe_guard` with empty deny list (should not wrap)
- Test `maybe_guard` with deny patterns (should wrap)
- Test `maybe_guard_arc` similarly

**Implementation notes:**
- Many of these wrappers call `AgentTool::call()` which is async. For tests that need to call the wrapped tool, create a minimal mock tool struct that implements `AgentTool` with predictable behavior (returns Ok with a known string, or returns Err).
- Tests for `TruncatingTool` can use `truncate_result` directly (it's a standalone function).
- Tests for `RecoveryHintTool` need to test the `ToolFailureTracker` struct directly (it has public methods).
- Keep the mock tool simple — just a struct with a name and a fixed return value.
- Target: add at least 20 new tests.
