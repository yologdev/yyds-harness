Title: Harden assessment prompt to guarantee artifact is written
Files: scripts/evolve.sh
Issue: none
Origin: planner

Objective:
Make the assessment phase reliably produce `session_plan/assessment.md` by restructuring the prompt so the file is created first, then populated — eliminating the "agent ran but didn't write the file" failure mode.

Why this matters:
Day 105's assessment agent ran with exit code 0 but did not write `session_plan/assessment.md`. The harness gracefully degraded to `assessment_missing.md` and fallback planning, but this pattern (successful run, missing artifact) wastes an entire assessment phase. The trajectory shows this isn't new — past sessions also hit no-task planning failures that may share the same root cause. A 10-line prompt restructuring can eliminate this class of failure.

Success Criteria:
- The assessment prompt instructs the agent to write `session_plan/assessment.md` as its FIRST action (with a placeholder), not as step 10 of 10.
- The harness-side completion watch (`^# Assessment\b`) still fires correctly on the partial file.
- The assessment phase continues to produce full assessments in the normal case.

Verification:
- Read the assessment prompt block in scripts/evolve.sh and confirm step ordering.
- Check that `session_plan/assessment.md` is referenced before research/self-test steps.
- `bash -n scripts/evolve.sh` passes.

Expected Evidence:
- Fewer `assessment_missing.md` artifacts in future sessions.
- Dashboard shows assessment artifacts present in sessions that previously had none.

Implementation Notes:
- In `scripts/evolve.sh`, around line 1064-1176 (the ASSESSEOF heredoc), restructure the steps:
  1. Move the file-writing instruction (currently step 10) to step 1 or a preamble before step 1.
  2. Tell the agent: "First, write a placeholder to `session_plan/assessment.md` with the header `# Assessment — Day N`. You will fill in each section as you complete the corresponding step."
  3. Keep the existing format template but add a note that sections can be filled incrementally.
- Keep all existing steps (0-9) but renumber them.
- Do not change the completion watch pattern or the harness guard logic.
- This is a prompt-only change; no script logic changes needed.
