Title: Classify planning failures by cause so future empty sessions are diagnosable
Files: scripts/log_feedback.py
Issue: none
Origin: planner

Evidence:
- Day 159 trajectory: `planner_no_task_count=1` — the 04:01 session produced no task files. Graph pressure: "Make planning failure actionable."
- Day 159 assessment: "The 04:01 session assessment completed but the planner emitted planning_failed with no task files. Three FailureObserved events in that session suggest something in the planning phase broke. Root cause needs diagnosis — was it a model error, a tool failure, or a prompt issue?"
- `scripts/log_feedback.py` already detects planning failures (line 892: `PLANNER_NO_TASK_RE`) and distinguishes provider-blocked from non-provider-blocked (lines 2081-2092). But it does not classify WHY a non-provider planning failure occurred — model API error, tool execution failure, or silent no-op (planner chose not to produce tasks). Without classification, every planning failure looks the same and produces no actionable feedback.
- The 04:01 session had 3 `FailureObserved` events during planning — if those were correlated with the planning failure, future sessions could learn that `FailureObserved` during Phase A means "replan with shorter prompt" rather than "try again with same parameters."

Edit Surface:
- scripts/log_feedback.py

Verifier:
- python3 -m unittest scripts.test_task_lineage_feedback

Fallback:
- If the latest trajectory shows `planner_no_task_count=0` and no planning failures in the last 5 sessions, mark this task obsolete — the problem may have self-resolved.
- If the classification would require reading full planning transcripts (which log_feedback doesn't currently do), narrow scope to inferring cause from already-parsed event metrics (FailureObserved count, tool failure count, model error count during planning phase).

Objective:
Add planning-failure cause classification to `log_feedback.py` so that when a future session's planner produces zero tasks, the feedback system can distinguish: (a) model API error/timeout, (b) tool execution failure (bash/edit_file crashed), (c) silent no-op (planner ran but chose not to produce tasks), or (d) unknown. Each cause suggests a different intervention.

Why this matters:
The trajectory's top graph-derived pressure is "Make planning failure actionable." Right now, `planner_no_task_count=1` tells us a session was wasted but not why. Classifying the cause converts an opaque count into actionable feedback: model errors suggest retry/backoff, tool failures suggest fix the tool, silent no-ops suggest prompt improvements. Without classification, the harness can't learn from planning failures and keeps repeating them.

Success Criteria:
- `log_feedback.py` emits a `planning_failure_cause` field (one of: `model_error`, `tool_failure`, `silent_no_op`, `provider_blocked`, `unknown`) when `planner_no_task_count > 0`.
- The cause is inferred from already-parsed per-phase event metrics (FailureObserved during planning, model errors during planning, tool failures during planning) — no new transcript parsing needed.
- Existing tests pass and new test cases cover at least model_error and silent_no_op causes.
- The classification appears in log_feedback output so trajectory extraction and dashboard can surface it.

Verification:
- python3 -m unittest scripts.test_task_lineage_feedback
- python3 scripts/log_feedback.py --help (confirm new output field appears)

Expected Evidence:
- Next time a planning failure occurs, the dashboard and trajectory show a specific cause (e.g., `planning_failure_cause: tool_failure`) instead of just `planner_no_task_count=1`.
- The `Graph-derived next-task pressure` section in future trajectories includes the cause in its suggestion text.
- Planning failure recurrence decreases because feedback is now actionable.

Implementation Notes:
- The detection already happens in `parse_log()` (line 892): `PLANNER_NO_TASK_RE` matches "Planning agent produced 0 tasks".
- After detection, check surrounding event metrics for the same run: were there `FailureObserved` events? Model errors (transport timeouts, API errors)? Tool execution failures?
- The `is_provider_blocked_before_tasks()` function already handles one cause — extend the classification to cover the other three.
- Keep the change minimal: a single new field in the metrics dict and the classification logic. No new file parsing, no new regexes.
- Test with the existing test data at line 3217 which already has a planning_failed fixture — add FailureObserved and tool_failure variants.
