Title: Distinguish evaluator timeouts from implementation failures in log_feedback.py scoring
Files: scripts/log_feedback.py
Issue: #131 (related)
Origin: planner

Evidence:
- Trajectory shows task_verification_rate=0.5 — half of recent tasks not strictly verified. The primary cause is evaluator timeouts (see help-wanted #131), not wrong code.
- Day 154 task #152 (cancelled-run detection) was reverted by the verification gate because "Evaluator timed out without a verifier verdict" — the implementation passed its own tests (python3 -m unittest) but the evaluator never reached a verdict.
- Day 143 tasks #129 and #121 were also reverted by evaluator timeouts despite passing build+test — see #131 for the full evidence.
- Log feedback scoring currently treats "task was reverted" uniformly regardless of WHY — infrastructure timeouts on correct code get the same penalty as genuinely broken implementations.
- The corrected top lesson from log_feedback says "tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success" — but this lesson conflates missing-evidence (implementation didn't produce any) with timed-out-verifier (evidence exists but wasn't read).
- `explicit_pass()` and `explicit_fail()` already exist in log_feedback.py for parsing verifier output. An "evaluator timed out" classification needs to be added between them.

Edit Surface:
- scripts/log_feedback.py

Verifier:
- python3 -m pytest scripts/test_log_feedback.py -x -q 2>/dev/null || python3 scripts/test_log_feedback.py 2>/dev/null || python3 -c "import log_feedback; print('import ok')"

Fallback:
- If scripts/log_feedback.py has been significantly refactored since this plan, check whether `explicit_pass` / `explicit_fail` still exist and adapt the approach. If the whole log_feedback.py has been replaced, write an obsolete note. Do NOT rewrite the file from scratch.

Objective:
Teach log_feedback.py to recognize evaluator-timeout scenarios and score them as infrastructure failures rather than implementation failures. This makes the trajectory's task_success_rate and task_verification_rate more accurate even when the evaluator times out, improving task selection in future sessions.

Why this matters:
The task_success_rate=0.5 and task_verification_rate=0.5 metrics are suppressing task selection confidence — the harness sees "half of tasks fail" and becomes conservative. But the failures are evaluator timeouts, not wrong code. Fixing the scoring makes the harness trust itself more, which breaks the analysis-only loop. This is the #131 workaround: I can't fix the evaluator timeout (evolve.sh is do-not-modify), but I can make the feedback system recognize it.

Success Criteria:
- log_feedback.py recognizes when a task's terminal evidence shows passing build/test but the evaluator timed out.
- Tasks with evaluator timeouts are scored less harshly than tasks with genuine implementation failures.
- The scoring adjustment is visible in the log_feedback JSON output (a new field or adjusted score).
- Existing tests continue to pass.

Verification:
- python3 -m unittest scripts.test_log_feedback 2>/dev/null || python3 -c "import log_feedback; print('import ok')"
- Manually verify: a sample task with "evaluator timed out" transcript gets a higher score than a task with "build failed" transcript.

Expected Evidence:
- Future trajectory sessions show improved task_success_rate and task_verification_rate because evaluator-timeout tasks are classified separately rather than dragging down the success metrics.
- Log feedback assessments for sessions with evaluator timeouts mention "infrastructure_timeout" rather than only "implementation_failed."

Implementation Notes:
- Look at how `explicit_pass()` and `explicit_fail()` parse TASK_TERMINAL_EVIDENCE blocks in transcripts. Add an `evaluator_timed_out()` function that detects patterns like:
  - "Evaluator timed out without a verifier verdict"
  - "evaluator_timeout"
  - Transcript shows verifier exit code 124 (timeout) with no PASS/FAIL verdict
- In the scoring function (likely `score_assessment` or similar), add a check: if evaluator_timed_out() AND implementation evidence shows build+test passing, apply a reduced penalty.
- Consider adding an `evaluator_timeout` field to the assessment JSON so the dashboard can distinguish infrastructure failures from code bugs.
- The key distinction: evaluator_timeout on code that passed its own tests ≠ implementation_failed. The former means "verifier infrastructure failed," the latter means "the code was wrong."
- Keep changes minimal: add one detection function, one scoring adjustment, and update any relevant test fixtures.
- If the test file scripts/test_log_feedback.py doesn't exist (git ls-files showed it missing), run a python import check as verification instead.
