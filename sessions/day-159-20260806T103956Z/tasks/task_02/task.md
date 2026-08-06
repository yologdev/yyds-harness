Title: Don't penalize session_success_rate for deliberate planning no-ops
Files: scripts/build_evolution_dashboard.py
Issue: none
Origin: planner

Evidence:
- Trajectory shows `session_success_rate=0.0` for day-159 04:38 which had 0 tasks attempted with `planner_no_task_count=1`. The planner produced no task files.
- At `build_evolution_dashboard.py` line 3176-3180, when `manifest.get("planning_failed")` is true, `session_success_rate` is unconditionally set to `0.0`.
- But a planning failure with `attempted=0` means the planner found nothing to do (healthy codebase, no actionable issues) — it's a deliberate no-op, not a crash.
- The empty-session classifier (`extract_trajectory.py` line 902) already distinguishes `assessment_empty` from `implementation_failed`. This data exists but isn't used in the gnome computation.
- The `attempted` variable is in scope at line 3176 (checked at line 3161 in the `elif attempted:` branch), so the fix is a one-line conditional guard.
- Assessment confirms: "Session success rate metric is noisy. A session that deliberately found nothing to do should not drag down the success rate."

Edit Surface:
- scripts/build_evolution_dashboard.py (line 3176-3180, the `planning_failed` override block)

Verifier:
- python3 -c "import scripts.build_evolution_dashboard; print('import ok')"
- python3 scripts/build_evolution_dashboard.py 2>&1 | head -5  (syntax check)

Fallback:
- If the dashboard script has changed significantly since this assessment, narrow the fix to only guard `session_success_rate = 0.0` behind `if attempted:`.
- If the fix breaks any dashboard test assertions, adjust the test to expect the new behavior.

Objective:
When the planner correctly decides no work is needed (writes `planning_failure.md` but attempted zero tasks), `session_success_rate` should not be hard-coded to `0.0`. The session was successful at its job (assessment), it just found nothing to fix.

Why this matters:
Conflating "healthy codebase, nothing to fix" with "planning crashed" makes the trajectory less trustworthy. When every no-op session drags down the success rate, the metric can't tell you whether sessions are failing or the codebase is just stable.

Success Criteria:
- A session with `planning_failed=True` and `attempted=0` (zero tasks attempted) no longer gets `session_success_rate=0.0`.
- A session with `planning_failed=True` and `attempted>0` (tasks attempted but planning failed) still gets `session_success_rate=0.0`.

Verification:
- python3 scripts/build_evolution_dashboard.py 2>&1 | head -5
- Manual review: the `planning_failed` block at line 3176 should only set `session_success_rate = 0.0` when `attempted` is truthy.

Expected Evidence:
- Future trajectory blocks for no-op sessions show `session_success_rate` as absent/None instead of `0.0`.
- The "Raise session success rate" graph pressure stops firing for genuine no-op sessions.

Implementation Notes:
- The fix is a one-line guard at line 3178: change `gnomes["session_success_rate"] = 0.0` to only execute when `attempted` is truthy.
- The surrounding block (lines 3176-3180) should become:
  ```python
  if manifest.get("planning_failed"):
      gnomes["planner_no_task_count"] = max(int(gnomes.get("planner_no_task_count") or 0), 1)
      if attempted:
          gnomes["session_success_rate"] = 0.0
      gnomes["task_artifact_coverage"] = 0.0
      recalc_score = True
  ```
- Do NOT change anything else in the gnome computation. This is a ~3-character addition.
