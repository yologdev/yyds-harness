Title: Add unit tests for dispatch.rs — command routing coverage
Files: src/dispatch.rs
Issue: none

## Goal

Increase test coverage for `dispatch.rs` which currently has 24 tests for 1,296 lines
(54 lines/test ratio — second-worst in the codebase). The routing logic is critical:
every `/command` goes through `route_command` and `dispatch_command`.

## What to test

Focus on `route_command` — it's a pure function that takes a command string and returns
a `CommandRoute` enum variant. This is highly testable without any mocking.

Add tests for:

1. **Route classification** (15+ tests):
   - All major commands route correctly: `/help`, `/map`, `/grep`, `/find`, `/add`,
     `/diff`, `/commit`, `/pr`, `/undo`, `/review`, `/blame`, `/lint`, `/test`,
     `/health`, `/fix`, `/doctor`, `/spawn`, `/bg`, `/fork`, `/checkpoint`
   - Unknown commands route to the appropriate variant
   - Empty input handling
   - Commands with arguments pass through correctly
   - Case sensitivity (commands should be case-sensitive)

2. **Edge cases** (5+ tests):
   - Commands with leading/trailing whitespace
   - Commands that are prefixes of other commands (e.g., `/map` vs `/mark`)
   - Multi-word commands where the first word matches

3. **CommandResult variants** (5+ tests):
   - Verify that different CommandResult variants serialize/display correctly
   - Test the `dispatch_command` return types for known commands

## Implementation notes

- All tests go in the existing `#[cfg(test)] mod tests` block at the bottom of `dispatch.rs`
- Use `route_command` directly — it's a pure function, no agent/context needed
- For `dispatch_command` tests that need a `DispatchContext`, create minimal mock contexts
  (if feasible) or test only the routing layer
- Target: 20+ new tests bringing the total from 24 to 44+ (ratio from 54 to ~29 lines/test)

## Constraints

- Only modify `src/dispatch.rs`
- Don't delete or modify existing tests
- Must pass `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
