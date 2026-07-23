Title: Add harness-internal discriminator to FailureObserved events to fix state-transcript reconciliation
Files: src/state.rs, scripts/build_evolution_dashboard.py
Issue: none
Origin: planner

Evidence:
- Trajectory: state_only_failed_tool_count=41 — 41 state events contain failed tool actions with no matching transcript entry
- Trajectory: transcript_only_failed_tool_count=1 — only 1 transcript failure missing from state (asymmetric)
- The 41:1 ratio strongly suggests harness-internal FailureObserved events (panic hook at state.rs:60, orphaned-run auto-close at state.rs:459) are being counted as tool failures in the reconciliation query
- `scripts/build_evolution_dashboard.py:2713` computes `state_only_failed_tool_count` as `unique_delta_count(state_failed_tools_all, transcript_failed_tools_all)` — it treats ALL FailureObserved events as tool failures
- The panic hook (state.rs:32-60) emits `FailureObserved` with `Actor::Harness` — these are infrastructure failures, not agent tool failures
- The orphaned-run closer (state.rs:412-459) also emits `FailureObserved` — same category
- Existing `Actor` enum already has `Harness` vs `Agent` — the reconciliation just doesn't filter by actor

Edit Surface:
- src/state.rs
- scripts/build_evolution_dashboard.py

Verifier:
- cargo test state
- python3 scripts/build_evolution_dashboard.py 2>&1 | head -5 (smoke test)

Fallback:
- If Actor filtering is already implemented in the reconciliation but the count is still 41, the issue is in how transcripts classify tool failures — mark this task obsolete and write a new task targeting transcript parsing in scripts/log_feedback.py.
- If the Actor field is not consistently set on existing FailureObserved events (some lack it), add a default Actor::Agent for backward compatibility and filter on explicit Actor::Harness.

Objective:
Stop counting harness-internal FailureObserved events as "state-only tool failures" by filtering the reconciliation query in build_evolution_dashboard.py to exclude events where Actor is Harness.

Why this matters:
The `state_only_failed_tool_count=41` metric is the #4 graph-derived pressure in the trajectory. It inflates the apparent state-transcript gap and makes the dashboard suggest reconciliation work that isn't needed. The 41:1 asymmetry (41 state-only vs 1 transcript-only) points to a systemic category error, not 41 individual bugs. Filtering harness-internal events from the tool-failure reconciliation makes the metric trustworthy.

Success Criteria:
- `state_only_failed_tool_count` drops significantly (harness-internal events excluded)
- `transcript_only_failed_tool_count` is unchanged (still ~1)
- `cargo test state` passes with no regressions
- The dashboard reconciliation query filters on `actor != "Harness"` or equivalent

Verification:
- cargo test state
- cargo build
- python3 -c "import scripts.build_evolution_dashboard as m; print('import ok')"

Expected Evidence:
- Next trajectory shows `state_only_failed_tool_count` reduced to a single-digit number
- The remaining count represents actual state-transcript gaps (not harness infrastructure)
- Dashboard state reconciliation panel no longer shows inflated mismatch

Implementation Notes:
- The fix is in `scripts/build_evolution_dashboard.py` around line 2713 where `state_only_failed_tool_count` is computed
- The `state_failed_tools_all` set is populated from state events — add an actor filter there (or in the delta computation) to exclude `Actor::Harness` events
- Check how `state_failed_tools_all` is populated (search for the variable name) and add the actor filter at the source
- In `src/state.rs`, verify that the panic hook at line 60 uses `Actor::Harness` consistently (it already does based on the code at line 60)
- The orphaned-run closer at line 459 should also use `Actor::Harness` — verify and fix if needed
- Keep the change minimal: add one filter condition in the dashboard script, and verify actor consistency in state.rs
- Do NOT change the event schema — Actor is already part of the StateEvent struct
