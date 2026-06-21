Title: Add run-ID and timestamp detail to cold-start state diagnostics
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: planner (refined from harness seed)

Evidence:
- Assessment line 55: `state why last-failure` returns "no failures recorded, notes 1 incomplete run (current session), suggests state trace / state crashes" — basic cold-start behavior already works
- The seed task's premise (returning only "no state event found") is stale; the assessment proves cold-start already distinguishes incomplete runs
- BUT the output lacks specific run IDs and timestamps — a user seeing "1 incomplete run" can't immediately run `state trace <run_id>` to investigate
- Graph pressure row 1 (lifecycle gaps) makes run-identification detail directly actionable: when the incomplete run is the very one with the lifecycle gap, the user needs the run ID to trace it

Edit Surface:
- src/commands_state.rs (why last-failure handler), src/state.rs (helper to find the most recent incomplete run with its ID and timestamp)

Verifier:
- cargo test commands_state state
- cargo check

Fallback:
- If the current output already includes run IDs and timestamps, write an obsolete note and do not edit. The seed task's problem statement is stale; only the run-ID detail is genuinely missing.

Objective:
When `yyds state why last-failure` finds no completed failures, include the incomplete/active run's ID and start timestamp in the output, so the user can immediately run `yyds state trace <run_id>` to investigate.

Why this matters:
Cold-start diagnostics that say "1 incomplete run" are friendly but not actionable — the user must guess which run and run another command to find it. Including the run ID and timestamp closes the loop: the diagnostic points directly to the evidence.

Success Criteria:
- `yyds state why last-failure` output includes the incomplete run's ID and start timestamp when an open run exists
- Existing output for "no failures recorded" and "no state events at all" cases is unchanged
- The run ID is copy-pasteable into `yyds state trace <run_id>`

Verification:
- cargo test commands_state state
- cargo check
- Manual check: `cargo run -- state why last-failure` should show run ID + timestamp for an open run

Expected Evidence:
- Future assessment `state why last-failure` output contains a specific run ID (e.g., "run-abc123") and timestamp
- State doctor reports the current run as no longer "incomplete" in the diagnostic sense (the tool now identifies it precisely)

Implementation Notes:
- The harness seed set this as a minimum viable task. The assessment confirms basic cold-start behavior exists. Narrow the implementation to adding the run ID and start timestamp to the existing output — do not rewrite the entire `why last-failure` handler.
- Use `close_orphaned_run_if_needed`'s tail-scanning approach (already in `src/state.rs`) as a reference pattern for finding the most recent open RunStarted event.
- Keep changes under ~50 lines.
