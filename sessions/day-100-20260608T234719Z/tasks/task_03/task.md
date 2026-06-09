Title: Make state why/tail commands bounded to prevent evolution timeouts
Files: src/commands_state.rs
Issue: none
Origin: planner

Objective:
Add time and output bounds to the most commonly used state inspection commands (`state why`, `state tail`) so they don't time out during evolution sessions. The trajectory log feedback says "agent commands timed out during evolution → prefer bounded diagnostics and targeted commands before broad cargo/state scans." This directly addresses that feedback.

Why this matters:
The trajectory shows recurring patterns of agent commands timing out during evolution sessions. `src/commands_state.rs` is 23,804 lines — the largest file in the codebase — and state queries can scan large event histories without bounds. Adding explicit limits (e.g., last 50 events, last 5 sessions) with a `--limit` flag makes state inspection fast enough to use during time-constrained evolution sessions.

The log feedback score is 0.9219 with one actionable item: "prefer bounded diagnostics." This task converts that feedback into a concrete improvement.

Success Criteria:
- `state tail` and/or `state why` commands accept an optional `--limit N` flag
- Default behavior caps output to a reasonable size (e.g., last 50 events or last 5 sessions)
- Commands complete in under 5 seconds even with large event histories
- `cargo build && cargo test` pass

Verification:
- `cargo build`
- `cargo test -- --test-threads=1`
- Manual check: `cargo run -- state tail --limit 5` should complete quickly

Expected Evidence:
- State inspection commands no longer cause timeouts during evolution sessions
- Trajectory log feedback score should improve in future sessions

In `src/commands_state.rs`, find the `state tail` and/or `state why` handler functions. Add a `--limit` flag that defaults to a reasonable cap (50 for tail, 5 sessions for why). The implementation should:
1. Parse `--limit N` from command args
2. Truncate event iteration after N items
3. Print a note like "(showing last N, use --limit 0 for all)" when truncation occurs

If `state tail` already has a limit mechanism, check if it's applied correctly and adjust the default. If `state why` does a full scan, add early termination after the limit is reached. Keep the change focused — don't refactor the entire 23K-line file.
