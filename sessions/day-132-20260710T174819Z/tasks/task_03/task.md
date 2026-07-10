Title: Add recent-window counts to action_evidence_summary_for_sessions (smaller retry of #89)
Files: scripts/build_evolution_dashboard.py
Issue: #89
Origin: planner

Evidence:
- Trajectory shows `state_only_failed_tool_count=37` as graph-derived pressure, but this aggregates ALL sessions including the Day 114-119 cascade period (42+ rapid crashes). The number includes historical noise that's already resolved — making the signal non-actionable.
- Assessment confirms: "The state-only count (37) is substantially larger than transcript-only (2)... may be cumulative historical data from the cascade failure period."
- Issue #89 was reverted due to evaluator timeout, not code failure. The task premise is still valid.
- `action_evidence_summary_for_sessions()` at build_evolution_dashboard.py:5057 currently returns only all-time aggregate counts.
- `extract_trajectory.py` at lines ~1636-1639 reads the all-time aggregate — but this task scopes the trajectory renderer change OUT to keep the retry smaller. Only the dashboard data source changes this time.

Edit Surface:
- scripts/build_evolution_dashboard.py (add recent-window subset counts to action_evidence_summary_for_sessions output)

Verifier:
- python3 -c "import scripts.build_evolution_dashboard as b; print('import ok')"

Fallback:
- If the dashboard test suite reveals these counts are already time-bounded or the recent-window calculation is already available (proving this task's premise wrong), write an obsolete note instead.
- If the function is significantly more complex than expected, scope down to just adding the keys without changing the trajectory renderer.

Objective:
Add `recent_state_only_failed_tool_count` and `recent_transcript_only_failed_tool_count` keys to `action_evidence_summary_for_sessions()` output, computed from only the last 5 sessions, so the dashboard and future trajectory renderers can distinguish live tool-failure mismatches from historical cascade-period noise.

Why this matters:
The graph-derived pressure signal says `state_only_failed_tool_count=37` but most of that is from resolved cascade failures. A planner seeing "37" can't tell if the problem is live or historical. Adding a recent-window count (last 5 sessions) converts this from a warning that gets ignored into a signal that drives action. This is the first half of the #89 fix — the trajectory renderer half comes later.

Success Criteria:
- `action_evidence_summary_for_sessions()` returns `recent_state_only_failed_tool_count` and `recent_transcript_only_failed_tool_count` keys
- Recent-window counts are computed from the last 5 sessions (matching existing `recent_window_size` pattern if available, hardcoded 5 otherwise)
- Existing tests pass (no regression in dashboard computation)
- The new keys don't break downstream consumers (they're additive)

Verification:
- python3 -c "import scripts.build_evolution_dashboard; print('module loads')"
- python3 scripts/test_build_evolution_dashboard.py TestActionEvidenceSummary -v 2>&1 | tail -20 (if test exists)
- Manual sanity: run dashboard build and check for new keys in output

Expected Evidence:
- Dashboard output includes `recent_state_only_failed_tool_count` with a value significantly smaller than `state_only_failed_tool_count` (e.g., 1-3 vs 37)
- Future trajectory output (once the renderer half lands) shows `state_only_failed_tools=37 (recent: 2)`
- Assessment notes that the tool failure reconciliation signal is now time-aware

Implementation Notes:
- In `action_evidence_summary_for_sessions()` (~line 5057 in build_evolution_dashboard.py), after computing the all-time aggregate, compute a second set of totals using only the last `recent_window_size` sessions.
- Use `recent_window_size = 5` (match the existing pattern if it exists in the module, hardcode otherwise).
- Add keys `recent_state_only_failed_tool_count` and `recent_transcript_only_failed_tool_count` to the returned dict.
- Do NOT change `extract_trajectory.py` in this task — scope is dashboard data source only. The trajectory renderer change is a follow-up task.
- Keep the change minimal: add computation and keys, don't restructure the function.
- The recent subset should use the same `session_sort_key` ordering as the existing dashboard code.
