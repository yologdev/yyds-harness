Title: Make analysis-only task pressure landable
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: harness-seed (refined by planner)

Evidence:
- Day 115 sessions: 3 of 6 had `reverted_no_edit` or `no-touched-files` outcomes
- Trajectory graph pressure row: "Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence"
- Latest evo readiness: task_success_rate=0.667, task_verification_rate=0.667 — dragged down by no-edit tasks that never attempted real code changes
- Day 114 preseed changes added analysis-only pressure triggers but the task still sometimes selects protected-file or unfinishable work

Edit Surface:
- scripts/preseed_session_plan.py (task selection logic, analysis-only pressure handling)
- scripts/state_graph_tools.py (analysis-only detection helpers)
- scripts/test_state_graph_tools.py (coverage for the new pressure path)

Verifier:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Fallback:
- If `preseed_session_plan.py --test` already passes and no new test case for analysis-only pressure is needed (i.e., current tests already cover the path), write an obsolete note explaining coverage already exists. Do not add tests for already-covered behavior.

Objective:
Ensure graph-derived analysis-only/no-edit task pressure produces a small, verifiable, landable seed task whose Files list contains no protected implementation files.

Why this matters:
`task_success_rate` and `task_verification_rate` are the primary fitness gnomes. When analysis-only pressure selects tasks that can't land code changes, sessions burn tokens achieving nothing — dragging both gnomes down and wasting $3-8/session. The preseed picker should translate "you didn't change files last time" into a concrete, small, non-protected follow-up.

Success Criteria:
- When `task_analysis_only_attempt_count >= 1`, the preseed picker selects a concrete task whose Files list excludes protected implementation files (those in PROTECTED_IMPLEMENTATION_FILES)
- The selected task is small enough to complete in one implementation session (≤2 source files)
- Self-tests cover the analysis-only pressure → landable-task translation path

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Expected Evidence:
- Future task manifests from sessions following analysis-only attempts show landable Files entries
- task_success_rate gnome rises (or at minimum, stops being dragged down by unfinishable task selections)
- No more sessions with `reverted_no_edit` caused by protected-file selections from analysis-only pressure

Implementation Notes:
- The analysis-only pressure trigger should prefer a small, non-protected src/ change over lifecycle/diagnostic tasks
- If there's genuinely nothing actionable, the fallback should be clear in the task file (stating "no landable changes found") rather than selecting an unfinishable task
- PROTECTED_IMPLEMENTATION_FILES is already defined in preseed_session_plan.py — ensure the new path filters against it
