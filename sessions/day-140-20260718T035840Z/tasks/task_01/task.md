Title: Deduplicate retroactive ModelCallCompleted events in janitor
Files: scripts/append_terminal_state_events.py, scripts/test_append_terminal_state_events.py
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure #1: "Close yyds state and model lifecycle gaps (deepseek_model_call_incomplete_count=8): Lifecycle causes: model_incomplete/open_after_ModelCallStarted=8; sta..."
- Assessment confirms: "Model call lifecycle gaps (8 incomplete). The janitor now writes retroactive events, but 8 model calls have ModelCallStarted without ModelCallCompleted."
- The janitor (`scripts/append_terminal_state_events.py` lines 658-691) already writes retroactive ModelCallCompleted events for orphaned ModelCallStarted entries, but each invocation may write duplicates because the retroactive event's key doesn't match the orphan-detection key.
- Day 139's fix (same file) deduplicated retroactive FailureObserved events — this is the same class of bug on a parallel lifecycle track. When `model_call_id` is absent from the original ModelCallStarted event, `find_orphaned_model_calls` uses run_id as the key, but the retroactive ModelCallCompleted gets `model_call_id = f"retroactive-{rid}"` — so on the next janitor invocation, the original ModelCallStarted (keyed by run_id) doesn't match the retroactive completed event (keyed by "retroactive-{rid}"), and it's detected as orphaned again.
- The `deepseek_model_call_incomplete_count=8` persists across sessions despite the janitor running, confirming duplicates are being written without closing the gap.
- `scripts/test_append_terminal_state_events.py` already has test coverage for the FailureObserved dedup fix (Day 139); this task adds parallel coverage for ModelCallCompleted dedup.

Edit Surface:
- scripts/append_terminal_state_events.py
- scripts/test_append_terminal_state_events.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events -v

Fallback:
- If the janitor already correctly deduplicates retroactive ModelCallCompleted events (i.e., the retroactive event uses the same key as the orphan detection), mark this task obsolete and document what the actual root cause of `deepseek_model_call_incomplete_count=8` is. The count may be from events predating the janitor that haven't been processed yet, or from a different mechanism entirely.
- If the fix requires more than 3 files or >100 lines of change, narrow the scope to just the dedup logic in `find_orphaned_model_calls` / the retroactive-append loop, and defer any broader refactor.

Objective:
Ensure each orphaned ModelCallStarted event receives exactly one retroactive ModelCallCompleted event, so `deepseek_model_call_incomplete_count` drops to 0 after the janitor runs and stays at 0 on subsequent invocations.

Why this matters:
The trajectory's #1 graph pressure item is model call lifecycle gaps. Each time the janitor runs without proper dedup, it writes another set of retroactive events, cluttering the event log without actually closing the lifecycle gap. The count stays at 8 because the original ModelCallStarted events remain unmatched. This is the same bug class that Day 139 fixed for FailureObserved — the retroactive event's key must match the key used for orphan detection, or the gap looks "still open" on the next scan.

Success Criteria:
- After one janitor invocation, `deepseek_model_call_incomplete_count` drops from 8 to 0 (or to the count of genuinely new incomplete calls, if any).
- A second janitor invocation writes zero additional retroactive ModelCallCompleted events for the same orphaned calls.
- The existing FailureObserved dedup behavior is not regressed.
- All existing `scripts/test_append_terminal_state_events.py` tests pass.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events -v
- Manual check: run the janitor twice against the same events file (with --dry-run or inspection) and verify the second run reports 0 new retroactive ModelCallCompleted events.

Expected Evidence:
- Trajectory graph pressure `deepseek_model_call_incomplete_count` decreases from 8 to 0 (or near 0) in the next session.
- State events show no duplicate retroactive ModelCallCompleted entries after the fix.
- The `orphaned_model_calls_appended` count in janitor diagnostics is 0 on the second invocation against the same event set.

Implementation Notes:
- The root cause is in `find_orphaned_model_calls` (lines 354-403) and the retroactive-append loop (lines 658-691) in `scripts/append_terminal_state_events.py`.
- When the original ModelCallStarted event has no `model_call_id`, `find_orphaned_model_calls` keys it by run_id. But the retroactive ModelCallCompleted at line 681 uses `model_call_id = f"retroactive-{rid}"`, which doesn't match the run_id key. On the next scan, the original ModelCallStarted (still keyed by run_id) isn't found in `model_completed_keys` (which contains "retroactive-{rid}"), so it's reported as orphaned again.
- Fix: when the original ModelCallStarted has no model_call_id, the retroactive ModelCallCompleted should also omit model_call_id (or use None/null), so that `find_orphaned_model_calls` matches it by run_id on the next scan. See how run_id fallback matching works in lines 393-398.
- Alternative fix: scan for existing retroactive ModelCallCompleted events before writing new ones (like Day 139 did for FailureObserved). The FailureObserved fix in `_find_missing_failure_observed` likely uses a set of run_ids that already have FailureObserved events; use the same pattern for ModelCallCompleted.
- Study the Day 139 FailureObserved dedup fix (look for `_find_missing_failure_observed` or similar helper, and the retroactive-append guard) and apply the same pattern to the ModelCallCompleted path.
- Add a test case in `scripts/test_append_terminal_state_events.py` that: creates a ModelCallStarted without model_call_id, runs the janitor, verifies one retroactive ModelCallCompleted is written, runs the janitor again, verifies zero additional retroactive events.
- Keep changes scoped to the two listed files. The state.rs event types don't need changes.
