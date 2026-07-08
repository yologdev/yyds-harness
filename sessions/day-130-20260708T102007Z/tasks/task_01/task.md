Title: Close yyds state and model lifecycle gaps
Files: scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Assessment Day 130 found `deepseek_model_call_unmatched_completed_count=16`, causes:
  `state_unmatched/completion_without_run_start=8` and gaps in partial-session lifecycle closure.
- `state why last-failure` shows 1 stale incomplete run (github-actions-28319290130, started ~10 days
  ago, never completed). This predates the Day 130 terminal-state fix.
- The Day 129 dashboard fix already filters input-validation model calls from lifecycle mismatch
  counts in `scripts/build_evolution_dashboard.py`, but `scripts/log_feedback.py` and
  `scripts/summarize_state_gnomes.py` may still count them in gnome metrics.
- Trajectory graph-derived pressure #1: "Close yyds state and model lifecycle gaps" with
  `deepseek_model_call_unmatched_completed_count=16` as the top-ranked signal.

Edit Surface:
- scripts/append_terminal_state_events.py — retroactively close stale incomplete runs,
  handle edge cases where a run started but never recorded a terminal event
- scripts/log_feedback.py — ensure input-validation model completions are excluded from
  lifecycle gap counts and lessons (matching the dashboard fix from Day 129)
- scripts/summarize_state_gnomes.py — ensure `is_input_validation_completion()` gates
  are applied consistently when computing `deepseek_model_call_unmatched_completed_count`

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback
- python3 scripts/append_terminal_state_events.py --dry-run (if supported)
- python3 scripts/summarize_state_gnomes.py --check (if supported)

Fallback:
- If the stale incomplete run is already closed by the Day 130 fix on next run, reduce scope
  to only the input-validation filtering fix.
- If neither gap is reproducible with current state, write an obsolete-task note explaining
  the evidence and mark complete.

Objective:
Make lifecycle gap detection precise: retroactively close the 1 known stale incomplete run,
and ensure input-validation model completions stay classified separately from non-validation
unmatched completions across all three scripts that compute lifecycle gnomes.

Why this matters:
The assessment shows 16 unmatched model-call completions, but 8 are from missing RunStarted
events (likely stale sessions). The remaining 8 may include input-validation calls that
should be filtered out. Without precise classification, the trajectory cannot distinguish
"provider is unreliable" from "harness lifecycle tracking has gaps" — two very different
problems that need different interventions. Clean lifecycle signals mean the harness can
trust its own provider health metrics.

Success Criteria:
- The 1 stale incomplete run (github-actions-28319290130) is retroactively closed with a
  FailureObserved event, or confirmed already handled by the Day 130 fix.
- `scripts/log_feedback.py` no longer counts input-validation model completions as
  lifecycle gaps (matching the pattern in `scripts/build_evolution_dashboard.py`).
- `scripts/summarize_state_gnomes.py` applies `is_input_validation_completion()` gating
  before incrementing `deepseek_model_call_unmatched_completed_count`.
- Future trajectory reports show `deepseek_model_call_unmatched_completed_count` decreasing
  (ideally to 0 if all gaps are benign input-validation or already-closed stale sessions).

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback
- python3 scripts/summarize_state_gnomes.py (verify output gnomes don't inflate lifecycle counts)
- Check: after fix, `yyds state why last-failure` should not show the stale run

Expected Evidence:
- Future structured state snapshots show lower (or zero) `state_run_incomplete_count`.
- `deepseek_model_call_unmatched_completed_count` drops as input-validation calls are excluded.
- Lifecycle repair tasks stop appearing as top graph-derived pressure in trajectory reports.

Implementation Notes:
- The dashboard fix in `scripts/build_evolution_dashboard.py` already filters input-validation
  completions — use the same `is_input_validation_completion()` function or pattern.
- The Day 130 `append_terminal_state_events.py` fix may have already added the logic to close
  stale runs — verify by running the script and checking if the stale run is resolved.
- If the script already handles it but hasn't been run yet (cron schedule), the task is to
  verify the code is correct and add a test proving it.
- Keep changes minimal: do not refactor the scripts, just add the filtering gating where missing.
