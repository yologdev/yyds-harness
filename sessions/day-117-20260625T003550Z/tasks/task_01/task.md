Title: Make analysis-only task pressure landable via preseed logic
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: harness-seed (refined by planner)

Evidence:
- Trajectory: `task_analysis_only_attempt_count=2`, `reverted_no_edit=2` — two recent tasks consumed session budget without producing file changes
- Trajectory: `task_success_rate=0.0`, `task_verification_rate=0.0` — no tasks landed verified code
- Trajectory graph-derived pressure #1: "Force analysis-only attempts into action — Implementation ended without file progress or terminal evidence; retry with action-first checkpoint or block"
- Assessment: Yuanhao's evolve.sh change (63c43f2) adds runtime retry for analysis-only attempts — this is a complementary runtime-level mitigation, not a planning-level fix
- Assessment: "The root cause may be in the implementation prompt itself" — but the preseed layer selects what task gets implemented; a landable task is half the battle

Edit Surface:
- scripts/preseed_session_plan.py — add logic so analysis-only/reverted_no_edit pressure selects small, concrete, non-protected tasks
- scripts/state_graph_tools.py — expose task_analysis_only_attempt_count and reverted_no_edit counts for preseed consumption
- scripts/test_state_graph_tools.py — cover the new analysis-only pressure path

Verifier:
- python3 scripts/preseed_session_plan.py --test
- python3 -m pytest scripts/test_state_graph_tools.py -q

Fallback:
- If the current assessment shows analysis-only failure class is already fully addressed by Yuanhao's evolve.sh change and no longer live, write session_plan/task_01_obsolete.md with the exact evidence and stop.

Objective:
When the state graph detects analysis-only/no-edit task failures, the preseed layer produces a small, landable task targeting concrete code (not broad analysis or protected evolution files) so the next implementation session can actually ship verified code.

Why this matters:
Analysis-only implementation attempts consume DeepSeek API budget (~$3-8/session) without producing code. The evolve.sh retry (Yuanhao's fix) gives implementation agents a second chance, but if the preselected task itself is too broad or targets protected files, no amount of retry will help. Fixing the preseed layer ensures that when the graph says "we're stuck on analysis-only," the next task is genuinely implementable.

Success Criteria:
- When `task_analysis_only_attempt_count >= 1` or `reverted_no_edit >= 1`, preseed selects a task whose Files list is non-empty and contains no PROTECTED_IMPLEMENTATION_FILES entries
- The selected task targets at most 2 source files (not scripts/evolve.sh, IDENTITY.md, etc.)
- Preseed self-tests exercise the analysis-only/no-edit pressure path and verify protected-file exclusion
- Existing tests in test_preseed_session_plan and test_state_graph_tools continue to pass

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m pytest scripts/test_state_graph_tools.py -q
- Manual: inspect generated task_01.md from a preseed run with analysis-only evidence — verify Files list is concrete and non-protected

Expected Evidence:
- Future task manifests show landable Files entries when analysis-only pressure exists
- Future trajectory: `task_success_rate > 0` and `task_verification_rate > 0` within 2 sessions of this landing
- `reverted_no_edit` count drops to 0 in subsequent sessions

Implementation Notes:
- This task was seeded by the harness before planner exploration. The planner validated it against fresh assessment evidence and found no contradiction.
- Yuanhao's evolve.sh change (63c43f2, retry analysis-only once) is complementary — this task fixes the planning layer, evolve.sh fixed the runtime layer.
- PROTECTED_IMPLEMENTATION_FILES is defined in preseed_session_plan.py (line ~15). Ensure the analysis-only pressure path respects this constant.
- The CONSTANT `TASKS` dict in preseed_session_plan.py maps condition functions to task templates. Add or modify an entry that fires when analysis-only metrics are elevated.
- Keep the change scoped to the listed files. If state_graph_tools needs new metrics, add them minimally (one or two helper functions).
- Do NOT touch scripts/evolve.sh, task_manifest.py, or other protected files.
