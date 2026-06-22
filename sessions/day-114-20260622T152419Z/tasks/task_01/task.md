Title: Close yyds state and model lifecycle gaps
Files: scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py
Issue: none
Origin: planner

Evidence:
- Trajectory lifecycle aggregate: run_incomplete=117, model_incomplete=54 across 92 sessions. These are persistent gaps that erode state feedback quality.
- 4 recent non-proven claims (model_lifecycle=2 observed, run_lifecycle=2 missing) — the harness reports lifecycle events as observed without completing the lifecycle pair (RunStarted→RunCompleted, ModelCallStarted→ModelCallCompleted).
- Pre-agent input-validation exits create false incomplete signals. The assessment confirms: "Pre-agent input-validation exits stay classified separately from non-validation unmatched completions" is a known need.
- The trajectory shows "Corrected top lessons: failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards" — lifecycle gaps compound with tool-failure reconciliation because both rely on terminal event pairing accuracy.

Edit Surface:
- scripts/append_terminal_state_events.py — terminal event appending logic
- scripts/log_feedback.py — lifecycle scoring and lesson derivation (run_lifecycle_event_summary, score_assessment, build_assessment)
- scripts/summarize_state_gnomes.py — gnome metric extraction from lifecycle state

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback

Fallback:
- If `yyds state doctor` shows run_incomplete_count < 50 (indicating a prior session already fixed this) or model_incomplete_count < 20, write an obsolete-task note with the concrete numbers instead of editing. A 50%+ reduction from current counts means the gap is closing organically.

Objective:
Reduce false lifecycle-incomplete signals so state-driven task selection, scoring, and gnome metrics reflect real gaps instead of classification artifacts.

Why this matters:
The assessment found incomplete run/model-call lifecycle gnomes (117 run, 54 model). These signals affect state feedback, assessment trust, and future task selection: a planner that sees "incomplete" when the work actually completed will over-prioritize lifecycle repair at the expense of real harness improvements.

Success Criteria:
- One verified lifecycle gap classification is fixed or downgraded with precise evidence in the listed files.
- Pre-agent input-validation exits are classified separately from non-validation unmatched completions in at least one of the three listed files.
- Log feedback emits lifecycle lessons only for real incomplete or non-validation unmatched paths, not for validation-only model calls.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback
- bash -n scripts/evolve.sh  # ensure no syntax errors in the evolution loop

Expected Evidence:
- Future structured state snapshots show lower `state_run_incomplete_count` and `deepseek_model_call_incomplete_count` because false positives are reclassified.
- Lifecycle repair tasks are selected from current assessment evidence instead of stale dashboard-only symptoms.
- Test coverage confirms input-validation model calls are excluded from incomplete counts.

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 114 (15:24).
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
- The most impactful single change is likely in `log_feedback.py`'s `run_lifecycle_event_summary` or its callers: distinguish input-validation model calls (which are expected to not complete a full lifecycle) from real unmatched calls.
- If the test files don't cover the specific classification path being fixed, add a focused test case that exercises the new distinction.
