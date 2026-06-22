Title: Make analysis-only task pressure landable
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Day 114 trajectory shows reverted_no_edit=1, task_analysis_only_attempt_count=1, task_success_rate=0.5
- Day 114 12:45 session: 0/1 tasks verified — planner produced no actionable tasks, burned a session
- Day 114 19:29 session: 1/2 tasks verified — one task was analysis-only with no file progress
- State snapshot confirms "recent task issues: reverted_no_edit=1"
- Graph-derived pressure: "Force analysis-only attempts into action" and "Raise verified task success rate (0.5)"
- Recent defensive work (task_manifest.py analysis-only rejection) catches the symptom but doesn't prevent it at source

Edit Surface:
- scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If preseed already handles analysis-only pressure correctly (i.e., the gap is in the planner, not the picker), write session_plan/task_01_obsolete.md with the exact evidence showing the preseed code produces a landable task for the current analysis-only pressure and the problem is upstream.

Objective:
Ensure that when graph-derived pressure includes analysis-only/no-edit task failure evidence, the preseed task picker selects a concrete, landable, small-scope task rather than falling through to no-op or broad protected-file work.

Why this matters:
Two recent sessions produced 0 or 1 verified tasks because the planner couldn't convert analysis-only pressure into a landable implementation task. The trajectory's 0.5 task success rate directly reduces the harness's evolution throughput. The task_manifest now rejects analysis-only escape hatches (defensive), but the preseed picker should produce a task that a DeepSeek implementation agent can actually complete (offensive).

Success Criteria:
- When graph-derived metrics show task_analysis_only_attempt_count > 0 or reverted_no_edit > 0, preseed selects a small, concrete task with Files entries that are not protected implementation surfaces
- Preseed self-tests cover the analysis-only/no-edit pressure path with a landable task output
- The selected task touches at most 3 source files and has a runnable verifier

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Expected Evidence:
- Future dashboard/trajectory runs show task_success_rate improving above 0.5
- task_analysis_only_attempt_count and reverted_no_edit decrease in subsequent sessions
- Task manifest no longer flags analysis-only escape hatches because preseed produces landable tasks

Implementation Notes:
- The preseed picker already has `_has_analysis_only_pressure()` and `analysis_only_active` logic. This task should ensure that when analysis-only pressure is detected, the picker selects from concrete, small-scope task templates rather than broad protected-file work.
- The picker's `TASKS` constant (or equivalent task templates) may need a new small-scope entry specifically for analysis-only pressure repair.
- Keep the change scoped to the listed files. The task_manifest.py analysis-only rejection (session 19:29) is complementary — this task fixes the prevention side.
