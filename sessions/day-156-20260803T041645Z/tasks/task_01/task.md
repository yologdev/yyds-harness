Title: Validate task Files entries against git-tracked paths in preseed_session_plan.py
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner

Evidence:
- Trajectory Day 156 shows task_scope_mismatch_count=1 with recommendation: "tighten task
  files and implementation prompts so planned Files entries match the intended edit surface."
- Trajectory shows reverted_unlanded_source_edits=3 in last 6 sessions — some of these
  reverts are from implementation touching files outside the planned edit surface.
- The assessment confirms: "Implementation agents touch files outside the planned task
  surface. This wastes implementation budget on non-scoped work that gets reverted."
- Preseed task generation currently writes Files: entries without validating they exist
  as git-tracked paths. A typo or stale path in a task template produces a task that
  implementation will inevitably fail (edit surface doesn't match reality).

Edit Surface:
- scripts/preseed_session_plan.py — add a validation helper that checks Files: entries
  against `git ls-files` output, and either corrects or drops candidates with invalid paths.

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If git ls-files is not available or the validation would require restructuring the
  task generation loop beyond a reasonable scope, add a warning comment instead and
  document what's needed. Do not force a large refactor.

Objective:
Prevent task files from being generated with Files: entries that don't correspond to
existing git-tracked paths, reducing scope-mismatch reverts.

Why this matters:
Task scope mismatch wastes implementation budget — the agent spends 20 minutes editing
files, only to be reverted because it touched something outside the planned surface.
Catching invalid Files: entries at generation time is cheaper than catching them at
verification time. This directly addresses trajectory pressure #4 and should reduce
reverted_unlanded_source_edits in future sessions.

Success Criteria:
- When preseed_session_plan.py generates a task candidate, each file in its Files: entry
  is checked against `git ls-files` (or the repo's tracked file list).
- Candidates with invalid Files: entries are either corrected (if the fix is obvious) or
  dropped with a logged reason.
- Existing preseed tests continue to pass. No regression in task generation throughput
  (the check should be a fast set lookup, not a per-file stat call).

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_task_manifest -k "test_" 2>/dev/null || true
  (spot-check: no unrelated test regressions)

Expected Evidence:
- Future trajectory shows reduced task_scope_mismatch_count.
- Task manifests show Files: entries that all resolve to git-tracked paths.
- No task candidates with phantom file paths reach implementation.

Implementation Notes:
- Use `git ls-files` to build a set of tracked paths. Cache the result — it doesn't
  change during planning.
- The check should run after task candidates are assembled but before they're written
  to session_plan/. Validate each path in the Files: field.
- If a path doesn't exist as tracked, try common corrections: strip leading `./`,
  check case-insensitive match, check if it's a directory that should be a file.
  If no correction works, log a warning and skip the candidate.
- Keep the change under 50 lines. This is a focused validation gate, not a refactor
  of the task generation pipeline.
- The existing `PROTECTED_IMPLEMENTATION_FILES` constant already tracks some file
  knowledge — the validation can live near it.
