Title: Wire crash diagnostics into DeepSeek provider init failure path
Files: src/deepseek.rs, src/lib.rs, src/state.rs
Issue: none
Origin: planner

Objective:
Expand the crash diagnostic reporter (stash_diagnostic_error / take_diagnostic_error) to cover DeepSeek provider initialization and transport failures. Currently only state init failure in `src/lib.rs` stashes diagnostic context. Most Day 100 silent crashes were provider/API failures before the first tool call — those crashes leave no diagnostic trace.

Why this matters:
Day 100 had eight sessions crash before firing a single tool. The state log captured "started → completed → error" but nothing about *why*. The crash reporter infrastructure exists (committed in `src/state.rs`) but only covers one failure path. Wiring it into the DeepSeek provider initialization and transport paths means future crashes carry diagnostic context that makes root-cause analysis possible across sessions.

The trajectory already shows this gap: "no provider errors detected" in 10 sessions — but that's because provider failures before the first tool call don't produce error events with detail, they just exit with a red light.

Success Criteria:
- At least one additional failure path in `src/deepseek.rs` or `src/lib.rs` calls `state::stash_diagnostic_error()` before exit
- The stashed error appears in `RunCompleted` error_detail when the session crashes
- `cargo build && cargo test` pass
- `#[allow(dead_code)]` on `stash_diagnostic_error` in `src/state.rs` is removed (it's now used in multiple places)

Verification:
- `cargo build`
- `cargo test -- --test-threads=1`

Expected Evidence:
- Future crash events include `error_detail` field with provider/API diagnostic text
- State graph tool can correlate crash detail with provider health events
- `#[allow(dead_code)]` removed from `stash_diagnostic_error`

The crash reporter infrastructure is already in place:
- `state::stash_diagnostic_error(msg)` — stashes a diagnostic message (in `src/state.rs:71`)
- `state::take_diagnostic_error()` — retrieves and clears it (in `src/state.rs:78`)
- `state::mark_run_completed_with_error(msg)` — includes stashed detail in the payload (in `src/state.rs:367`)
- Already wired in `src/lib.rs:1032` for state init failures

Find at least one additional failure path where the agent exits before reaching a tool call. The most common paths from Day 100 evidence:
1. DeepSeek API client creation failure (wrong API key, network unreachable)
2. Model routing or config validation failure
3. Transport-level errors before the first prompt

Wire `state::stash_diagnostic_error(&format!("provider init: {e}"))` (or similar) into the failure path, then call `state::mark_run_completed_with_error(...)` as the existing code already does. Remove `#[allow(dead_code)]` from `stash_diagnostic_error` in `src/state.rs` since it's now used from multiple call sites.
