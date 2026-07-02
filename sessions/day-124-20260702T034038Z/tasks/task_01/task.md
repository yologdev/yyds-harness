Title: Fix `yyds state why last-failure` timeout — add event sampling cap
Files: src/commands_state.rs
Issue: #51
Origin: planner (refined from harness-seed, replaces quiet-journal seed)

Evidence:
- Assessment §Self-Test: `yyds state why last-failure` timed out at 15s on Day 122
- Same read-everything pattern was already fixed in `state doctor` (Day 117) and `state crashes` (Day 122)
- `build_why_report` at src/commands_state.rs:3112 scans all events to find a single failure event
- Graph pressure: bash_tool_error=10 from trajectory, calling for bounded shell commands
- This is issue #51 — reverted because evaluator timed out; implementation was correct but unverified

Edit Surface:
- src/commands_state.rs (build_why_report at ~line 3112 and its caller; follow the sampling pattern from state crashes)

Verifier:
- timeout 15 cargo run -- yyds state why last-failure
- cargo test --lib commands_state

Fallback:
- If no failure exists in the sampled window, print "No failure found in recent events (sampled last N events). Use --full to scan all history."
- If the timeout is caused by per-event computation (not volume), add early termination after a fixed event window.
- If the command was already fixed by a concurrent session, verify and write an obsolete note.

Objective:
Make `yyds state why last-failure` complete within 15 seconds by reading only the most recent N events (e.g., 5000) instead of the full 63K-line events file.

Why this matters:
`state why` is yyds's primary self-diagnostic command. When it times out, the agent loses visibility into why sessions fail — which blocks trajectory feedback, graph pressure computation, and self-directed improvement. The fix follows the established pattern from state doctor and state crashes: add a sampling cap. This is a fitness gate: state_capture_coverage is blocked until diagnostics work.

Success Criteria:
- `timeout 15 cargo run -- yyds state why last-failure` completes and prints diagnostic output
- When no failure exists in sampled events, prints a clear message (not a cryptic error)
- `cargo build && cargo test --lib commands_state` passes
- `yyds state why <specific-event-id>` still works by reading the full event stream for a concrete ID

Verification:
- cargo build
- cargo test --lib commands_state
- timeout 15 cargo run -- yyds state why last-failure
- cargo run -- yyds state why evt-promote  (specific event, should still work)

Expected Evidence:
- `yyds state why last-failure` completes within 15 seconds
- Task lineage shows file edits in src/commands_state.rs
- Future structured state snapshots show `bash_tool_error` pressure decreasing
- Self-test report shows zero timeouts for `state why last-failure`

Implementation:
1. In `src/commands_state.rs`, find where `build_why_report` is called. The caller decides which events to pass. Add sampling: when the query is "last-failure" (not a specific event ID), read only the most recent 5000 events from the events file.
2. The events file is append-only — the most recent events are at the end. Read the file backwards or use `tail` semantics.
3. Use the same sampling pattern from `state crashes` (Day 122) — look at how that command caps its event reads and replicate.
4. When no failure event exists in the sampled window, output: "No failure found in the most recent N events. Use --full to scan all history."
5. Keep `yyds state why <event-id>` working unchanged for specific event lookups.
6. Keep the change to ~30 lines or fewer. This is a focused fix, not an exploration.
