Title: Exclude cancelled runs from log_feedback lifecycle gap counts
Files: scripts/log_feedback.py
Issue: #152
Origin: planner (refined from harness-seed)

Evidence:
- `scripts/append_terminal_state_events.py` already has `_is_run_externally_cancelled()` (lines 227-304) and `find_missing_failure_observed` already excludes cancelled runs (line 344: `if status == "cancelled": cancelled_excluded += 1; continue`). This was landed Day 156.
- `scripts/log_feedback.py` `state_cache_metrics()` (line 1279) computes `state_run_incomplete_count` and `deepseek_model_call_incomplete_count` without filtering out cancelled runs. The `run_lifecycle_event_summary()` helper (line 1259) already captures `status` and `error_detail` from each run's last event — the data IS available but not consulted.
- `is_input_validation_completion()` (line 1268) already exists as a precedent for filtering lifecycle counts by run completion reason. A similar `is_cancelled_completion()` helper would follow the same pattern.
- Trajectory graph pressure: "Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=43)" and "Raise session success rate (session_success_rate=0.0)" — cancelled runs inflate both metrics.
- 3 of 8 recent completed runs were cancelled in the 17:xx UTC time slot (Day 156 cancelled at 2.5hr). These produce retroactive FailureObserved and lifecycle gap counts that are infrastructure noise, not harness bugs.

Edit Surface:
- scripts/log_feedback.py — add `is_cancelled_completion()` helper modeled after `is_input_validation_completion()`, then filter `run_incomplete_ids` and `incomplete_model_runs` to exclude cancelled runs before computing `state_run_incomplete_count` and `deepseek_model_call_incomplete_count`.

Verifier:
- python3 -m unittest scripts.test_task_lineage_feedback
- python3 -c "import scripts.log_feedback; print('imports OK')"

Fallback:
- If `run_last_events` lacks the status field needed to detect cancelled runs consistently (the `run_lifecycle_event_summary` helper might not record RunCompleted status for all event types), document the gap in comments and mark the task blocked — do not add fragile heuristics.

Objective:
Stop log_feedback from counting externally-cancelled runs as harness lifecycle bugs. When a session is killed by SIGTERM or GitHub Actions concurrency, missing terminal events are expected, not bugs.

Why this matters:
Cancelled-session noise distorts `state_run_incomplete_count` and `deepseek_model_call_incomplete_count` in log_feedback, which feeds into trajectory graph pressure and task selection. The dashboard shows `session_success_rate=0.0` partly because cancelled sessions count as failures. Clean this upstream of the dashboard so the numbers reflect actual harness health.

Success Criteria:
- `state_cache_metrics()` excludes runs with `status == "cancelled"` from `state_run_incomplete_count` and `deepseek_model_call_incomplete_count`.
- Runs with `status == "error" and error_detail==""` (the rust_panic-from-SIGTERM pattern) are also excluded — these are externally killed, not buggy.
- Existing tests in `scripts/test_task_lineage_feedback.py` continue to pass.
- `is_cancelled_completion()` checks the `run_last_events` entry for the run: if the last event's `status` is `"cancelled"`, or if the last event is RunCompleted with `status == "error"` and empty `error_detail` (the rust-panic-from-SIGTERM pattern documented in `_is_run_externally_cancelled` signal 2).

Verification:
- python3 -m unittest scripts.test_task_lineage_feedback
- python3 scripts/log_feedback.py --help 2>&1 || true  (verify no import errors)

Expected Evidence:
- Future trajectory shows lower `state_run_incomplete_count` from sessions where the only incomplete runs were cancelled.
- The "state run lifecycle was incomplete" log feedback lesson fires less often on cancelled-session noise.
- Dashboard `session_success_rate` rises toward actual success rate (excluding cancelled).

Implementation Notes:
- Add `is_cancelled_completion(last_event)` helper after `is_input_validation_completion()` (around line 1277). Check: `status == "cancelled"` OR (`kind == "RunCompleted"` AND `status == "error"` AND empty `error_detail`).
- In `state_cache_metrics()`, after computing `run_incomplete_ids` (line 1341), filter out runs where `is_cancelled_completion(run_last_events.get(run_id))` is true. Same for `incomplete_model_runs` computed at line 1329.
- The `run_incomplete_ids` computation at line 1341 uses `run_started - run_completed` — cancelled runs appear here because SIGTERM prevents RunCompleted. Filter after computing the set difference.
- Keep changes minimal: one helper function (~8 lines), two filter expressions (~4 lines each). Do not refactor the broader function.
