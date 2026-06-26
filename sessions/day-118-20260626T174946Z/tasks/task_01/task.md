Title: Make analysis-only task pressure landable
Files: scripts/preseed_session_plan.py, scripts/test_state_graph_tools.py
Issue: none
Origin: harness-seed (refined by planner)

Evidence:
- Day 118 trajectory confirms task-success pressure is live: session day-118 (03:50) had `obsolete_already_satisfied=1`, day-117 had `reverted_no_edit=1` and `reverted_unlanded_source_edits=1`. These are analysis-only/no-edit task outcomes that the preseed picker should detect and convert into landable follow-up tasks.
- Assessment: "Five consecutive sessions working on diagnostic/self-observation infrastructure. Zero source-code Rust changes that affect agent behavior." — the preseed picker is selecting analysis-friendly tasks over landable implementation tasks.
- Current `scripts/preseed_session_plan.py` has `ANALYSIS_ONLY_TASK_TITLE` and `ACTIONABLE_LIFECYCLE_METRICS` constants but the `choose_task` function may not reliably select landable tasks when analysis-only pressure is high.

Edit Surface:
- scripts/preseed_session_plan.py (choose_task logic, analysis-only detection)
- scripts/test_state_graph_tools.py (or add test_preseed_session_plan.py if needed for unit coverage)

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `choose_task` already correctly prefers landable tasks when analysis-only evidence is present and the self-tests already cover this path, mark this task obsolete_already_satisfied with evidence of the existing coverage.

Objective:
Ensure that when trajectory evidence shows analysis-only/no-edit task outcomes (obsolete_already_satisfied, reverted_no_edit, reverted_unlanded_source_edits), the preseed picker selects a concrete, landable follow-up task whose Files list contains no protected implementation files (PROTECTED_IMPLEMENTATION_FILES).

Why this matters:
The preseed pipeline is the harness's last chance to turn diagnostic evidence into a concrete task before the planner runs. When it selects analysis-only or protected-file tasks, the implementation agent either does nothing or gets reverted — wasting a full session. The Day 118 assessment shows this is still happening: 5 consecutive sessions of diagnostic-only work with no Rust source changes affecting agent behavior.

Success Criteria:
- `choose_task` returns a task with Files containing no entries from PROTECTED_IMPLEMENTATION_FILES when analysis-only pressure is the dominant signal.
- The selected task Files list names at most 3 editable source files (not scripts/, not .github/).
- Self-tests cover the path where ANALYSIS_ONLY_TASK_TITLE would have been selected but a landable alternative is chosen instead.
- Existing tests continue to pass.

Verification:
- python3 scripts/preseed_session_plan.py --test
- grep -n 'ANALYSIS_ONLY' scripts/preseed_session_plan.py | head -20  (confirm constants are used in choose_task)
- python3 -c "from scripts.preseed_session_plan import PROTECTED_IMPLEMENTATION_FILES, ANALYSIS_ONLY_TASK_TITLE; print('OK')"

Expected Evidence:
- Future task manifests show landable Files entries when task-success repair pressure fires.
- Future trajectory pressure leads with implementation targets rather than analysis-only seeds.
- Session outcomes show fewer reverted_no_edit and reverted_unlanded_source_edits task states.

Implementation Notes:
- The `choose_task` function in preseed_session_plan.py selects from TASKS based on evidence. When analysis-only pressure is the top signal, it should either pick the next-best landable task or return a narrowed version of the analysis task that targets a specific editable file.
- The ANALYSIS_ONLY_TASK_TITLE is "Analyze and improve DeepSeek harness" — this title signals the task is investigation, not implementation. When it's selected, ensure the Files list is filtered to exclude protected files.
- PROTECTED_IMPLEMENTATION_FILES includes scripts/evolve.sh, .github/workflows/*, IDENTITY.md, PERSONALITY.md, ECONOMICS.md, and similar files. Do not add to this list — the fix is in task selection, not protection expansion.
- After editing, run `python3 scripts/preseed_session_plan.py --test` and confirm all tests pass.
