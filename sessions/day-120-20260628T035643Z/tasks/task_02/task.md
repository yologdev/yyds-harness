Title: Add analysis-only task escape hatch to preseed task selection
Files: scripts/preseed_session_plan.py
Issue: #41
Origin: planner (narrowed from reverted Day 118 attempt)

Evidence:
- Trajectory Day 120 graph-derived pressure: "Force analysis-only attempts into action (task_analysis_only_attempt_count=2): Implementation ended without file progress or terminal evidence; retry with a task whose first verifiable step is small enough to complete in 10 min"
- Assessment: "Six consecutive sessions have landed zero Rust code changes." The preseed picker continues to select analysis tasks when analysis-only pressure is the top signal — creating a self-reinforcing loop.
- Issue #41 (auto-filed Day 118): Previous attempt to fix `choose_task` was reverted because "Evaluator timed out without a verifier verdict" — the change was too broad. This attempt is narrower.
- `ANALYSIS_ONLY_TASK_TITLE` constant exists in preseed_session_plan.py. When this task is selected and analysis-only/no-edit pressure is the top signal, the preseed should prefer a landable alternative task whose Files list names editable source files.
- Commit `5472bc21` added stale/obsolete seed detection but didn't address the analysis-only → landable task preference.

Edit Surface:
- scripts/preseed_session_plan.py: add a post-selection check in `choose_task` (or the caller that invokes it) that, when the selected task matches `ANALYSIS_ONLY_TASK_TITLE` and analysis-only pressure is the dominant signal, falls back to the next landable task in the TASKS list.

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `choose_task` already skips ANALYSIS_ONLY_TASK_TITLE when analysis-only pressure is the top signal and a landable alternative exists, and self-tests prove this path, write an obsolete_already_satisfied note with the test evidence.
- If `ANALYSIS_ONLY_TASK_TITLE` is no longer in the TASKS list, the problem is already resolved — write obsolete_already_satisfied.

Objective:
When the trajectory shows analysis-only/no-edit task pressure (task_analysis_only_attempt_count > 0 or reverted_no_edit in recent sessions), and the preseed picker would select the ANALYSIS_ONLY_TASK_TITLE, prefer the next landable task instead. Break the self-reinforcing loop where analysis pressure selects analysis tasks.

Why this matters:
The preseed pipeline is the harness's last chance to turn diagnostic evidence into a concrete task before the planner runs. When analysis-only pressure selects yet another analysis task, the session is doomed before implementation even starts. This single post-selection filter converts one guaranteed-empty session into a session with at least a chance of landing code.

Success Criteria:
- When `task_analysis_only_attempt_count > 0` or `reverted_no_edit` is present in recent evidence, and the TASKS ranking would pick ANALYSIS_ONLY_TASK_TITLE as the top task, the picker selects the next-best landable task instead.
- The landable fallback task's Files list names at least one editable source file.
- Self-tests cover this escape-hatch path.
- Existing tests continue to pass.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -c "from scripts.preseed_session_plan import ANALYSIS_ONLY_TASK_TITLE; print('OK')"

Expected Evidence:
- Future task manifests for sessions with analysis-only pressure show landable tasks (not ANALYSIS_ONLY_TASK_TITLE).
- Future trajectory shows fewer reverted_no_edit and task_analysis_only_attempt_count outcomes.
- Session outcomes show at least one session with a landable task that wasn't analysis-only.

Implementation Notes:
- This is a NARROWER version of the reverted Day 118 task. Do not refactor `choose_task` broadly. Add ONE check: after `choose_task` returns its selection, if the selected task title equals `ANALYSIS_ONLY_TASK_TITLE` and analysis-only pressure is the top signal in the evidence, iterate through the remaining TASKS (in order) and pick the first one whose title does NOT equal `ANALYSIS_ONLY_TASK_TITLE`.
- If no landable alternative exists in TASKS, keep the ANALYSIS_ONLY_TASK_TITLE selection — an analysis task is better than nothing.
- Look for analysis-only pressure signals: `task_analysis_only_attempt_count > 0`, `reverted_no_edit` in task states, or `task_success_rate < 0.1` in the evidence dict.
- Add one targeted test that constructs evidence with `task_analysis_only_attempt_count=1` and verifies the picker does not return `ANALYSIS_ONLY_TASK_TITLE`.
- The change is scoped to `choose_task` or its immediate caller — do not restructure the TASKS list, constants, or evidence pipeline.
