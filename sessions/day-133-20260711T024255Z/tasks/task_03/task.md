Title: Distinguish external-only task scope from genuine source-file mismatches in dashboard
Files: scripts/build_evolution_dashboard.py
Issue: none
Origin: planner

Evidence:
- Trajectory: `task_scope_mismatch_count=1` — the Day 132 task to close issues #89,#91,#92 was reverted because implementation wrote `session_plan/task_01_external_evidence.json` but planned Files was "(none — GitHub issue management only, no source edits)"
- The task was issue-management-only: no source code to edit, no src/ files in scope. The verification gate correctly rejected it (assessment says "the gate worked"), but the dashboard counts it identically to a genuine edit-surface mismatch
- Assessment: "The root issue is that the task picker produced a task whose implementation surface didn't match its planned file list." But this conflates two different failure modes:
  1. Genuine scope violation: task planned src/foo.rs but edited src/bar.rs too — the implementation overreached
  2. External-only task: task planned no source files (issue management, gh CLI only) but harness wrote session_plan artifacts automatically — not the implementation's fault
- The `task_scope_mismatch` function at line 901 of build_evolution_dashboard.py currently treats all non-overlapping file changes equally
- If the dashboard could distinguish these, it would give cleaner pressure signals: genuine scope violations need fixing in task selection/discipline; external-only mismatches need fixing in the verification gate's session_plan exemption or in planner instructions

Edit Surface:
- scripts/build_evolution_dashboard.py — only the `task_scope_mismatch` function and its callers that aggregate `task_scope_mismatch_count`

Verifier:
- python3 -c "import scripts.build_evolution_dashboard as d; print('import OK')"
- python3 -m py_compile scripts/build_evolution_dashboard.py

Fallback:
- If the scope mismatch classification would require changes to evolve.sh (protected), limit the task to dashboard-only changes: add a `task_mismatch_kind` field ("external_only" vs "source_edit") alongside the existing boolean, and update the dashboard HTML to display the distinction. Do not touch evolve.sh.

Objective:
Make the `task_scope_mismatch_count` gnome honest by distinguishing between two different failure modes: (1) genuine implementation overreach where the agent edited source files outside the planned surface, and (2) external-only tasks where the harness auto-wrote session_plan evidence files not declared in the task's Files line. Both are real issues, but they need different interventions, and collapsing them into one number hides that.

Why this matters:
The trajectory's graph-derived pressure says "Align implementation edits with task file scope (task_scope_mismatch_count=1)" and "Raise verified task success rate (task_success_rate=0.5)". But the single scope-mismatch event was an external-only task — closing GitHub issues. Collapsing it with genuine source-file mismatches produces misleading pressure: it tells the system to tighten edit discipline when the real fix is in the planning pipeline (include session_plan files for issue-only tasks) or the verification gate (exempt session_plan writes). Honest classification → better task selection.

Success Criteria:
- The dashboard's `task_scope_mismatch_count` or a sibling metric distinguishes external-only mismatches from source-edit mismatches
- The existing behavior for genuine source-file scope violations is unchanged
- The dashboard HTML renders the distinction clearly (e.g., "1 scope mismatch (external-only)" vs "1 scope mismatch (source edit)")
- Dashboard self-consistency checks (if any) still pass

Verification:
- python3 -m py_compile scripts/build_evolution_dashboard.py
- python3 -c "
import scripts.build_evolution_dashboard as d
# Verify task_scope_mismatch still works on known cases
# Verify new external-only classification path exists
print('OK: module loads and expected functions exist')
"

Expected Evidence:
- Next session's trajectory shows `task_scope_mismatch_count` with a qualifier or a separate `task_external_only_mismatch_count` metric
- Future issue-only tasks that write session_plan files no longer inflate the "genuine scope violation" count
- The graph-derived pressure row "Align implementation edits with task file scope" becomes more specific when scope mismatches are external-only

Implementation Notes:
- Study the `task_scope_mismatch` function at line 901 of build_evolution_dashboard.py to understand the current detection
- The key distinction: if a task's planned Files contains "(none" or "no source edits" or similar external-only markers, AND the only unplanned touched files are under session_plan/, classify as "external_only" instead of a genuine scope mismatch
- Alternatively, if the task's planned files are empty/None and all touched files are session_plan/*, classify as external-only
- Keep backward compatibility: existing callers of `task_scope_mismatch()` should still work
- Consider adding a separate counter `task_external_only_mismatch_count` or a new field `task_mismatch_kind` in the row data
- Update the dashboard HTML section that renders scope-mismatch counts to show the distinction
- Do NOT modify evolve.sh — the enforcement gate stays as-is; this task only improves observability
