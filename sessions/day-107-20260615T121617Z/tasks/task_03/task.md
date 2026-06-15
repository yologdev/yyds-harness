Title: Validate seeded tasks against fresh assessment evidence
Files: scripts/evolve.sh, scripts/preseed_session_plan.py
Issue: none
Origin: planner

Objective:
Prevent stale seeded tasks from reaching implementation when the current assessment contradicts their stated problem, reducing seed task contradictions from the current count of 1+ to 0.

Why this matters:
The trajectory reports `task_seed_contradiction_count=1` — a seeded task was contradicted by assessment evidence but still reached implementation. This wastes an implementation slot on work that may already be done or may not address the current state. The seed task pipeline (`preseed_session_plan.py`) writes task_01.md before the planner runs, but doesn't validate against the fresh assessment. When the seeded task's stated problem has already been fixed in a recent session, the implementation agent works on a solved problem.

Success Criteria:
- `preseed_session_plan.py` reads the current assessment (session_plan/assessment.md) before writing seed tasks
- If the assessment explicitly states that the seed task's problem is already addressed, the seed task is NOT written (or is written with an `obsolete` marker)
- The planner receives clear signals about whether the seed task is fresh or contradicted
- task_seed_contradiction_count drops to 0 in future sessions

Verification:
- python3 scripts/preseed_session_plan.py (dry run with sample assessment)
- cargo check (no Rust changes expected, but verify no regressions)
- Manual: Create a scenario where assessment.md says "X is already fixed" and verify the seed task for X is suppressed

Expected Evidence:
- task_seed_contradiction_count reaches 0 in future trajectory reports
- Seed tasks include a `validated_against_assessment: true/false` field
- When contradicted, seed task is either suppressed or annotated with the contradiction

Implementation Notes:
The seed pipeline is `scripts/preseed_session_plan.py`. It currently writes `session_plan/task_01.md` based on recent state/trajectory evidence but doesn't cross-reference the assessment.

The fix should:
1. In `preseed_session_plan.py`: Before writing a seed task, scan `session_plan/assessment.md` for evidence that the proposed problem is already resolved. Look for:
   - The assessment's "Recent Changes" section mentioning a fix for the same area
   - Self-test results showing the feature already works
   - Explicit statements like "already addressed" or "correctly reports"
2. If contradicted: either skip the seed task entirely, or write it with a header annotation like `Validity: CONTRAVERTED — assessment says [reason]` so the planner can see and handle it.
3. In `scripts/evolve.sh`: After the planning phase, verify that no task file contains a contradiction marker without the planner having explicitly resolved it.

Keep the change scoped to the Python script — no Rust changes unless a state event type is needed. The assessment.md format is stable enough for regex/keyword matching.
