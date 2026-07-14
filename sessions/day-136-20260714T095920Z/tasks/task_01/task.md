Title: Close yyds state lifecycle gaps — open_after_FailureObserved runs
Files: scripts/append_terminal_state_events.py, scripts/test_append_terminal_state_events.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Assessment Day 136: `state_unmatched/open_after_FailureObserved=8` — 8 runs have FailureObserved recorded but no RunCompleted event. These are runs where the harness crashed or was cancelled after recording the failure but before closing the run lifecycle.
- Assessment Day 136: `deepseek_model_call_unmatched_completed_count=3` — 3 model calls completed without matching start events, indicating incomplete lifecycle tracking.
- Day 136 02:33 session already added retroactive FailureObserved detection to the terminal-state janitor; the remaining gap is closing runs that HAVE FailureObserved but lack RunCompleted.
- Trajectory graph pressure row 5: "Close yyds state and model lifecycle gaps" with `state_unmatched/open_after_FailureObserved=8`.

Edit Surface:
- scripts/append_terminal_state_events.py — add logic to detect runs with FailureObserved but no RunCompleted, and append a retroactive RunCompleted(status=error)
- scripts/test_append_terminal_state_events.py — add test cases for open_after_FailureObserved detection

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events

Fallback:
- If the current events.jsonl shows 0 open_after_FailureObserved runs (gap already closed by prior fix or event churn), write an obsolete-task note explaining the evidence and mark complete.

Objective:
Close runs that have FailureObserved but no RunCompleted by teaching the terminal-state janitor to append retroactive RunCompleted(status=error) events for these orphaned runs.

Why this matters:
Incomplete run lifecycles block accurate state feedback. When runs are open-ended after a failure, the state graph can't compute correct session outcomes, and the trajectory extractor sees phantom incomplete runs. This directly affects task selection quality in future sessions.

Success Criteria:
- `scripts/append_terminal_state_events.py` detects runs where FailureObserved exists but RunCompleted does not
- Appends retroactive `RunCompleted(status=error)` events for those runs
- Test coverage for the open_after_FailureObserved detection path
- `python3 -m unittest scripts.test_append_terminal_state_events` passes

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events
- python3 scripts/append_terminal_state_events.py --dry-run 2>&1 | head -20 (verify it detects the gap)

Expected Evidence:
- After running the janitor, `yyds state lifecycle --limit 20` shows fewer unmatched/open runs
- Future structured state snapshots show lower `state_unmatched` counts
- The trajectory `deepseek_model_call_unmatched_completed_count` decreases

Implementation Notes:
- The Day 136 02:33 session already added `find_stale_orphaned_runs()` and `find_runs_with_failure_observed_no_completion()` stubs. This task should complete the `find_runs_with_failure_observed_no_completion()` implementation and its corresponding `append_terminal_events()` handler.
- Use the same pattern as the existing retroactive FailureObserved append: scan events, group by run_id, detect orphans, and append RunCompleted with retroactive=true in the payload.
- Keep the change scoped to the listed 2 files. The 02:33 session already expanded the test file.
