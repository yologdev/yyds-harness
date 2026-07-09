Title: Make preseed fallback produce actionable tasks when assessment is missing
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner (trajectory evidence)

Evidence:
- Day 131 planning phase: assessment phase produced a transcript (exit code 0, no provider error) but didn't write `session_plan/assessment.md` (confirmed by `session_plan/assessment_missing.md`)
- The preseed's `choose_task()` fallback (lines 847-885) produces a generic "Repair evidence-backed planning after no-task sessions" task with `files = "scripts/preseed_session_plan.py"` when `assessment_was_missing=True`
- This generic task doesn't tell the implementation agent what to actually change — it opens preseed_session_plan.py, doesn't know what to fix, and either makes no changes or bad changes
- The seed task_01.md from Day 131 was this exact generic fallback — the planner had to refine it into the concrete lifecycle gap task
- The `assessment_missing.md` file already contains structured failure diagnostics (exit code, provider_error flag, transcript path) but the fallback ignores them

Edit Surface:
- scripts/preseed_session_plan.py: `choose_task()` function (assessment_was_missing branch, lines 877-885) and the fallback dict (lines 847-875)

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If the assessment_missing.md file format has changed or the assessment failure class is already handled by other pipeline improvements, mark the task obsolete.

Objective:
When the assessment phase fails to produce assessment.md, the preseed fallback should produce a task that names the specific failure mode (timeout, exit code, provider error, missing file) and points the implementation agent at the transcript for diagnosis, rather than always producing the generic "Repair evidence-backed planning" task.

Why this matters:
The preseed is the last line of defense before the planner sees an empty task slate. When it produces a vague "fix yourself" task, it wastes an implementation slot. A task that says "Assessment failed with exit code 0 and no provider error — check transcripts/assess.log for why assessment.md wasn't written" gives the implementation agent a concrete starting point for diagnosis, which is more likely to produce a useful fix.

Success Criteria:
- When assessment_missing.md indicates exit_code=0 and no provider error, the fallback task describes the specific failure (assessment ran but didn't produce output) and points at the transcript
- When assessment_missing.md indicates a timeout, the fallback task addresses timeout-specific causes
- When assessment_missing.md indicates a provider error, the fallback task addresses provider reliability
- The generic "Repair evidence-backed planning" fallback is preserved for cases where assessment_missing.md is unavailable or unparseable
- Existing task candidate selection (lifecycle gap, search friction, etc.) is unchanged

Verification:
- python3 scripts/preseed_session_plan.py --test
- Manual test: create a minimal assessment_missing.md and verify the fallback produces a task with the specific failure mode in its title or objective

Expected Evidence:
- Future no-assessment sessions produce fallback tasks with titles that reflect the specific assessment failure mode (e.g., "Diagnose assessment phase silent failure (exit 0, no output)") rather than the generic "Repair evidence-backed planning"
- Implementation agents spend less time trying to interpret vague fallback tasks

Implementation Notes:
The change is in `choose_task()` in `scripts/preseed_session_plan.py`:

1. When `assessment_was_missing=True` (line 877), instead of immediately returning the generic fallback, parse `assessment_missing.md` for structured fields:
   - `assessment_exit_code` (int)
   - `assessment_timeout_seconds` (int)
   - `provider_error_detected` (bool)
   - `transcript` path

2. Based on the parsed fields, produce a more specific fallback task:
   - Exit code 0 + no provider error → "Assessment agent ran successfully but didn't write assessment.md. Check the transcript for why the output file wasn't produced."
   - Timeout → "Assessment phase timed out after N seconds. Check if the assessment prompt is too broad or if the model is slow."
   - Provider error → "Assessment phase hit a provider/API error. Check API key validity and rate limits."
   - Unknown/parse failure → fall back to existing generic task

3. The assessment_missing.md format is:
```markdown
# Assessment Missing - Day N (HH:MM)
...
- status: assessment_missing
- assessment_exit_code: N
- assessment_timeout_seconds: N
- provider_error_detected: true/false
- required_artifact: session_plan/assessment.md
- transcript: transcripts/assess.log
```

Parse these fields with simple regex or line-based matching. The file is small (~15 lines).

Keep the change minimal — modify only the `assessment_was_missing` branch and maybe add a small helper to parse the assessment_missing.md file.
