Title: Improve cold-start state failure diagnostics
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: harness-seed

Objective:
Make `yyds state why last-failure` useful when there are no completed failed sessions yet by reporting nearby startup errors, incomplete runs, or missing diagnostic evidence.

Why this matters:
The assessment found `state why last-failure` returning only `no state event found` during fresh-state sessions. That leaves yyds unable to explain the earliest failures that block evolution.

Success Criteria:
- Cold-start `why last-failure` output gives actionable next evidence to inspect.
- Existing behavior for completed failed sessions remains unchanged.
- Output distinguishes no history from missing diagnostics and from active/incomplete runs.

Verification:
- cargo test commands_state state
- cargo check

Expected Evidence:
- Future assessment logs can cite concrete cold-start diagnostics instead of an empty result.
- State/dashboard blockers become easier to trace to run/session ids.

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 104 (04:05); refine it if the planner has stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
