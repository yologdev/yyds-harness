Title: Verify lifecycle gap cleanup after retroactive terminal events and close Issue #87
Files: scripts/append_terminal_state_events.py, scripts/build_evolution_dashboard.py
Issue: #87
Origin: planner (refined from harness-seed)

Evidence:
- Assessment Day 132 03:25: the terminal-state script fired 271 retroactive FailureObserved events after the SessionStarted fix (d5a4e22a). These retroactively closed historical runs that had no terminal event.
- Trajectory snapshot still shows `state_run_unmatched_non_validation_completed_count=25` — some runs remain unmatched even after the cleanup. Need to identify which ones and why.
- Issue #87 was reverted (evaluator timeout) but the underlying code fix (SessionStarted recognition) is already landed. This task verifies the fix works end-to-end and closes the issue.
- Dashboard `lifecycle_imbalance_causes` may misclassify input-validation exits (pre-agent sanity checks) as real lifecycle gaps, creating false-positive lifecycle warnings.

Edit Surface:
- scripts/append_terminal_state_events.py: run against current state to count remaining open runs; fix any edge cases the 03:25 cleanup missed
- scripts/build_evolution_dashboard.py: ensure `is_input_validation_completion()` is called in `lifecycle_imbalance_causes` so input-validation exits are classified separately from real lifecycle gaps

Verifier:
- python3 scripts/append_terminal_state_events.py --dry-run (or equivalent) shows remaining open runs <= 5 (only current in-flight sessions)
- python3 -c "from scripts.build_evolution_dashboard import lifecycle_imbalance_causes; print(lifecycle_imbalance_causes(...))" shows input-validation exits separated from real gaps

Fallback:
- If remaining open runs are already <= 5 and input-validation classification already works, close Issue #87 and report success without code changes.
- If append_terminal_state_events.py has no --dry-run flag, add one or use a read-only path to count remaining open runs.

Objective:
Verify the 03:25 session's retroactive cleanup actually closed historical lifecycle gaps, close any remaining gaps, and ensure the dashboard correctly distinguishes real lifecycle problems from benign input-validation exits.

Why this matters:
The trajectory shows lifecycle gaps as the #2 graph-derived pressure signal (`state_run_unmatched_non_validation_completed_count=25`). The 03:25 session already fired 271 retroactive events — this task verifies that work actually resolved the problem. Open runs with no terminal events undermine state feedback accuracy and cause the dashboard to emit lifecycle warnings that may not reflect real problems. Closing Issue #87 removes a reverted task from the backlog and confirms the SessionStarted fix (d5a4e22a) works end-to-end.

Success Criteria:
- Running append_terminal_state_events.py closes remaining historical open runs (target: <= 5 remaining, only current in-flight sessions)
- build_evolution_dashboard.py lifecycle_imbalance_causes separates input-validation exits from real lifecycle gaps
- Issue #87 is closed with a comment summarizing what was verified
- Future structured state snapshots show `state_run_incomplete_count` approaching zero

Verification:
- python3 scripts/append_terminal_state_events.py (dry-run or actual) — count remaining open runs
- python3 scripts/build_evolution_dashboard.py — (read-only check) verify input-validation classification
- python3 -c "import scripts.append_terminal_state_events; import scripts.build_evolution_dashboard; print('imports OK')"

Expected Evidence:
- state_run_incomplete_count drops from current 25 to <= 5
- state_run_unmatched_non_validation_completed_count drops toward zero
- Lifecycle tasks stop appearing as top graph-pressure signals
- Issue #87 closed

Implementation Notes:
- The SessionStarted fix in d5a4e22a added SessionStarted to the lifecycle-start recognition. Run the script against `.yoyo/state/events.jsonl` to verify all edge cases.
- For dashboard: check if `is_input_validation_completion()` is already called in `lifecycle_imbalance_causes()`. If not, add the filter.
- Do NOT modify src/state.rs — the lifecycle recording is already correct. This is cleanup of historical artifacts and dashboard classification.
- If the 03:25 session already handled most gaps, the remaining count may be small and the task is verification-only.
