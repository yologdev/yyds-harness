Title: Make preseed fallback use assessment_missing artifact when assessment phase fails
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Day 125 (17:27) assessment phase produced `session_plan/assessment_missing.md` (exit_code=0, no provider errors, but no assessment.md written).
- The preseed script (`scripts/preseed_session_plan.py`) has no explicit handling for the "assessment missing" case — it uses assessment text when available but doesn't detect or react to `assessment_missing.md`.
- The trajectory shows `task_analysis_only_attempt_count=3` — sessions that analyze but don't implement. When assessment is missing, the fallback task selection should be more conservative (prefer smaller, concrete tasks over diagnostic investigation).
- The current seed task was generic ("Repair evidence-backed planning after no-task sessions"); this refinement makes it concrete: detect assessment_missing.md and adjust fallback task selection accordingly.

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test
- Simulate: touch session_plan/assessment_missing.md && python3 scripts/preseed_session_plan.py (verify it detects the file and adjusts fallback)

Fallback:
- If the preseed script already handles this case (check first), mark this task obsolete with a note about what already exists.
- If the fix requires changes to evolve.sh or protected files, narrow to preseed-only.

Objective:
When the assessment phase fails to produce assessment.md, the preseed fallback task selection should detect `assessment_missing.md` and prefer smaller, concrete implementation tasks over broad diagnostic investigations.

Why this matters:
The assessment phase failed this session. The preseed script ran anyway and produced a generic planning-repair task. Making the fallback aware of the `assessment_missing.md` artifact means future assessment failures produce better-calibrated tasks — smaller scope, concrete files, verifiable outcomes — instead of broad "fix the planning pipeline" tasks that are self-referential and hard to verify.

Success Criteria:
- When `session_plan/assessment_missing.md` exists, the preseed fallback task includes a note that assessment was missing and adjusts task scope accordingly.
- The fallback task's Files: entry names at most 2 concrete source files (not "the whole planning pipeline").
- `python3 scripts/preseed_session_plan.py --test` passes.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -c "import scripts.preseed_session_plan" (syntax check)

Expected Evidence:
- After this fix, future assessment-missing sessions get preseed tasks with narrower scope.
- The preseed script's `--test` output shows the assessment_missing detection path works.

Implementation:
1. In `scripts/preseed_session_plan.py`, near where the assessment text is loaded, add a check for `session_plan/assessment_missing.md`.
2. When found, set a flag (e.g., `assessment_was_missing = True`) that influences task selection.
3. When `assessment_was_missing` is true, prefer tasks from `TASKS` that:
   - Touch at most 2 source files
   - Are concrete implementation tasks (not diagnostic/analysis tasks)
   - Have a clear verifier (not "investigate why X")
4. Update the `--test` path to cover this case.
5. Keep the change minimal — under 30 lines.
