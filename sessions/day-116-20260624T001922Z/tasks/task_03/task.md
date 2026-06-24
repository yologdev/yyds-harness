Title: Reconcile tool-failure counting between extract_trajectory.py and state CLI
Files: scripts/extract_trajectory.py, src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Trajectory "Graph-derived next-task pressure" row 4: `failed_tool_summary.bash_tool_error=4`
- `yyds state failures tools` output: "no tool failures found"
- These two queries disagree on whether tool failures exist in the current session window
- Assessment: "This mismatch could hide real tool-call problems. If the trajectory extractor sees failures the state CLI doesn't, the diagnostics are incomplete."
- The discrepancy may be: (a) different event windows (trajectory looks at multi-session history, state CLI looks at current run only), (b) different failure definitions (trajectory counts attempted-but-errored, state CLI counts confirmed-failed), or (c) a query bug in one side

Edit Surface:
- scripts/extract_trajectory.py (the `failed_tool_summary` computation path)
- src/commands_state.rs (the `build_tool_failures_report` function and/or `handle_tool_failures`)

Verifier:
- cargo build && cargo test --test integration -- --test-threads=1
- python3 scripts/test_extract_trajectory.py (if tests exist for the failure-counting path)

Fallback:
- If investigation reveals the discrepancy is a legitimate scope difference (trajectory intentionally counts differently from state CLI), document the difference in a code comment and add a note to the trajectory output explaining the scope. Do not force them to agree if they answer different questions.

Objective:
Make tool-failure diagnostics consistent so the trajectory and state CLI agree on whether tool failures exist.

Why this matters:
When the trajectory graph pressure says "bound failing shell commands" but `state why` says "no failures," the implementation agent receives contradictory signals. This confusion wastes session time and undermines trust in both diagnostic surfaces. Reconciling them ensures the next session's trajectory pressure is actionable and the state CLI is trustworthy.

Success Criteria:
- `state failures tools` and trajectory `failed_tool_summary` counts are consistent for the same session window
- If they intentionally count different things, the difference is documented and the trajectory graph-pressure rows quote the right diagnostic surface

Verification:
- cargo build && cargo test --test integration -- --test-threads=1
- python3 scripts/test_extract_trajectory.py (if tests exist)
- After fix: run both queries on the same session window and confirm they report consistent failure counts

Expected Evidence:
- Next trajectory `failed_tool_summary` rows match what `state failures tools` reports
- No more contradictory pressure rows telling the agent to fix failures that the state CLI can't see
- Implementation agent can trust either diagnostic surface

Implementation Notes:
- Start by comparing the failure-counting logic: how does `extract_trajectory.py` define a "tool failure" vs how does `commands_state.rs` define one?
- Check if the scope mismatch is the root cause: does `state failures tools` default to current run while trajectory aggregates across session window?
- If fixing one side to match the other, prefer making the trajectory more conservative (don't count things as failures unless confirmed) over making state CLI more aggressive
- Keep the change small — this is a reconciliation, not a rewrite of either failure-counting system
- Add a test case to `test_extract_trajectory.py` that verifies failure counting against known state events if one doesn't exist
