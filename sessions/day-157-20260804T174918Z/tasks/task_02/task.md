Title: Exclude cancelled runs from summarize_state_gnomes lifecycle gnome counts
Files: scripts/summarize_state_gnomes.py
Issue: #152
Origin: planner

Evidence:
- `scripts/append_terminal_state_events.py` already detects and excludes cancelled runs (lines 227-304, 344-346). Landed Day 156.
- `scripts/summarize_state_gnomes.py` `summarize_state_lifecycle()` (line 306) computes lifecycle gnomes including `state_run_incomplete_count` (line 420), `deepseek_model_call_incomplete_count` (line 437). Neither filters out cancelled runs.
- The function already has `is_input_validation_completion()` checks for model calls (lines 396-406) but no equivalent for cancelled runs. `run_last_events` already stores each run's terminal event via `event_lifecycle_summary()` which captures `status` and `error_detail`.
- Gnome values flow into the dashboard and trajectory, so cancelled-session pollution inflates `state_run_incomplete_count` and `deepseek_model_call_incomplete_count` gnomes — making the harness look less healthy than it is.
- Trajectory graph pressure: "Close yyds state and model lifecycle gaps" — the gnome layer is downstream of state events and should be consistent with the state-event-level exclusions already in `append_terminal_state_events.py`.

Edit Surface:
- scripts/summarize_state_gnomes.py — add `is_cancelled_run()` helper (or inline the check), then filter `run_incomplete_ids` and `model_incomplete_ids` to exclude cancelled runs before computing gnome counts.

Verifier:
- python3 -m pytest scripts/test_summarize_state_gnomes.py -x -q 2>&1 || python3 -m unittest scripts.test_summarize_state_gnomes -v 2>&1 || echo "no dedicated test file; verify with python3 -c 'import scripts.summarize_state_gnomes'"

Fallback:
- If `run_last_events` entries don't reliably contain RunCompleted status for cancelled runs (SIGTERM might prevent RunCompleted from being written), check for the rust-panic-without-detail pattern instead: `error == "rust_panic"` in the last event AND no RunCompleted. If neither signal is reliably available, document the gap and mark the task blocked.

Objective:
Stop `summarize_state_gnomes` from counting externally-cancelled runs in lifecycle gnome metrics. Cancelled runs should not inflate `state_run_incomplete_count` or `deepseek_model_call_incomplete_count`.

Why this matters:
Lifecycle gnomes feed into the dashboard and trajectory graph pressure. When cancelled runs inflate "incomplete" counts, the planning agent sees false pressure to "close lifecycle gaps" that aren't real bugs. This creates a feedback loop: cancelled runs → inflated gnomes → planning agent selects lifecycle tasks → session runs long → next session cancels it → more inflated gnomes.

Success Criteria:
- `summarize_state_lifecycle()` excludes runs with cancelled terminal events from `run_incomplete_ids`.
- `model_incomplete_ids` also excludes runs whose last event indicates cancellation (same check).
- `state_run_incomplete_count` and `deepseek_model_call_incomplete_count` gnomes drop for sessions where the only incomplete runs were cancelled.
- Existing gnome tests continue to pass (no regression on healthy-run lifecycle counts).

Verification:
- python3 -c "
import scripts.summarize_state_gnomes as g
# smoke test: ensure module loads and lifecycle_gnomes function exists
assert callable(g.lifecycle_gnomes), 'lifecycle_gnomes not found'
print('import OK, lifecycle_gnomes is callable')
"
- If a test file exists: python3 -m unittest scripts.test_summarize_state_gnomes -v

Expected Evidence:
- Dashboard gnome history shows lower `state_run_incomplete_count` for sessions where cancelled runs were the primary incomplete entries.
- Trajectory "Close yyds state and model lifecycle gaps" pressure drops as gnome counts reflect actual bugs, not infrastructure kills.

Implementation Notes:
- Add a helper `_is_cancelled_run(run_id, run_last_events)` after `is_input_validation_completion` (around line 255 in summarize_state_gnomes.py). Check the last event for: `status == "cancelled"` OR (RunCompleted with `status == "error"` AND empty `error_detail`). The empty-error_detail signal matches `_is_run_externally_cancelled` signal 2 in append_terminal_state_events.py.
- At line 367, after computing `run_incomplete_ids`, add a filter:
  ```python
  cancelled_run_ids = {rid for rid in run_incomplete_ids if _is_cancelled_run(rid, run_last_events)}
  run_incomplete_ids = [rid for rid in run_incomplete_ids if rid not in cancelled_run_ids]
  ```
- At line 393-407, after computing `model_incomplete_ids` and before counting, add the same filter.
- Keep changes minimal: ~10 lines for the helper, ~6 lines for the two filters. Do not restructure the function.
