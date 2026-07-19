Title: Improve fallback planning when assessment is missing — surface trajectory gnomes in seed tasks
Files: scripts/preseed_session_plan.py
Issue: none
Origin: harness-seed (refined by planner)

Evidence:
- Day 141 (16:58) assessment phase: exit_code=0, no provider error, but assessment.md was not written (see session_plan/assessment_missing.md).
- Trajectory shows task_obsolete_count=1 — the planner selected a task that was already completed, indicating the seed task picker doesn't cross-reference against recently landed work.
- Day 141 (11:03) session: 1/2 tasks strict-verified, with one task obsolete_already_satisfied — the seed task picker re-created a task (#124 unbounded-command) that Task 1 had already landed.

Edit Surface:
- scripts/preseed_session_plan.py (the fallback task seeding logic that runs when assessment.md is absent)

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If the assessment_missing path already produces good seed tasks (verified by manual run), write an obsolete note instead.
- Do not modify protected files (scripts/evolve.sh, .github/workflows/).

Objective:
When the assessment phase produces no assessment.md, the fallback seed task in preseed_session_plan.py should include trajectory gnome evidence (task_success_rate, task_obsolete_count, etc.) so the planning agent can select tasks that address live pressure rather than generic repairs.

Why this matters:
The harness reached planning with no assessment artifact. The seed task picker created a generic "fix the assessment pipeline" task without trajectory evidence. With trajectory gnomes in the seed task evidence, the planner can select higher-impact work (e.g., contradiction detector, success-rate-aware scoping) even when assessment is absent.

Success Criteria:
- When assessment.md is missing, the seed task's Evidence section includes trajectory-derived gnome metrics (task_success_rate, task_verification_rate, task_obsolete_count, etc.).
- The seed task picker can detect recently-landed work (via git log) and avoid re-creating tasks that were already completed.
- python3 scripts/preseed_session_plan.py --test passes.

Verification:
- python3 scripts/preseed_session_plan.py --test
- Manual: simulate missing assessment and check seed task evidence quality.

Expected Evidence:
- Next session with missing assessment produces a task_01.md with trajectory gnomes in Evidence.
- Future trajectory: task_obsolete_count decreases (fewer stale tasks selected).

Implementation Notes:
- In preseed_session_plan.py, the `choose_task` function or the harness seed path should pull from available trajectory data when assessment.md is absent.
- The trajectory extractor (`scripts/extract_trajectory.py`) already produces structured gnome data; the seed task picker should consume at minimum: task_success_rate, task_verification_rate, task_obsolete_count, and the top graph-pressure row.
- Keep the change minimal — add trajectory evidence injection to the seed task rendering path, not a whole new task selection system.
- Do NOT modify evolve.sh or protected workflow files.
