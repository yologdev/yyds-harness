Title: Make analysis-only task pressure landable
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: harness-seed
validated_against_assessment: true

Evidence:
- Current assessment confirms: `reverted_no_edit=4` in trajectory task states window. These are sessions where tasks produced no file changes — the analysis-only pattern that Day 109 taught the harness to stop retrying on, but the preseed picker can still select tasks that trigger this pattern.
- Latest log feedback score 0.9844, task_success_rate=1.0, task_artifact_coverage=1.0 — the harness is healthy enough to improve task selection precision.

Edit Surface:
- scripts/preseed_session_plan.py — add analysis-only pressure detection and landable task constraints
- scripts/state_graph_tools.py — if pressure signal computation needs a new graph-derived metric, add it here
- scripts/test_state_graph_tools.py — cover the new pressure path

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If the assessment or trajectory shows zero `reverted_no_edit` counts and zero `task_analysis_only_attempt_count`, mark this task obsolete — the failure class is gone.

Objective:
Ensure that when graph-derived pressure detects analysis-only/no-edit task attempts (reverted_no_edit > 0 or task_analysis_only_attempt_count > 0), the preseed picker selects a small, landable follow-up task instead of selecting broad harness work or protected-file changes.

Why this matters:
When a previous session chose a task that produced zero file edits, the next session's task selection must learn from that — pick a smaller, more concrete task, not another broad-planning task. Without this, the harness can loop: pick oversized task → produce no edits → pick another oversized task. Day 109 stopped the retry loop; this task makes the first retry landable.

Success Criteria:
- When `reverted_no_edit` > 0 or `task_analysis_only_attempt_count` > 0 is detected, the preseed picker selects a task whose Files list: (a) has ≤3 files, (b) contains no files in PROTECTED_IMPLEMENTATION_FILES, and (c) is scoped to a specific tool, script, or function rather than a broad subsystem.
- The preseed self-tests (`--test`) exercise the analysis-only pressure path with mock evidence.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Expected Evidence:
- Future task manifests for sessions following a reverted_no_edit session show concrete, small Files entries targeting specific scripts or modules.
- No protected implementation files appear in task Files lists when analysis-only pressure is active.

Implementation Notes:
- This task was seeded by the harness before planner exploration. The assessment does not contradict it (reverted_no_edit=4 is still present in trajectory), though it's lower priority than the reconciliation gap (task_02.md).
- Keep the change scoped to the listed files. The preseed picker's TASKS dict or choose_task() logic is the likely change point. A guard in choose_task() that checks for analysis-only pressure and downgrades task scope is sufficient.
- PROTECTED_IMPLEMENTATION_FILES is already defined in preseed_session_plan.py — reuse it.
- If the implementation cannot be contained in 3 files, narrow the scope to just preseed_session_plan.py with a self-test update.
