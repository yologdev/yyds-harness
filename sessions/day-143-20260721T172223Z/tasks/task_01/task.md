Title: Add evaluator-timeout-with-evidence detection to log_feedback.py
Files: scripts/log_feedback.py
Issue: #132
Origin: planner

Evidence:
- Assessment: "Evaluator timeout reliability: The #1 cause of false task reverts. When the evaluator times out, tasks that passed build+test get the same treatment as tasks that broke the build."
- Trajectory graph pressure: "Make evaluator timeouts resumable or cheaper (evaluator_timeout_count=3): Evaluator timeout friction still appears in action logs."
- Day 143 Task 1 (cbc4211b): 263 lines of correct code in src/state.rs, cargo test passed, but evaluator timed out → task reverted
- Day 143 Task 2 (038d468c): 38 lines in preseed_session_plan.py, python3 tests passed, but evaluator timed out → task reverted
- log_feedback.py already has EVALUATOR_TIMEOUT_RE (line 49), EVALUATOR_UNVERIFIED_RE (line 50), evaluator_timeout counting (line 835-836), evaluator_timeout_with_verdict_count in GNOME_KEYS (line 241) and SCORE_FAILURE_WEIGHTS (line 296)
- Current scoring treats "evaluator timed out + code was wrong" identically to "evaluator timed out + code was correct"

Edit Surface:
- scripts/log_feedback.py

Verifier:
- python3 -c "import sys; sys.path.insert(0, 'scripts'); import log_feedback; print('import OK')"
- python3 scripts/log_feedback.py --test

Fallback:
- If the implementation transcript format doesn't reliably expose cargo build/test exit codes, scope down to detecting any build/test success markers and adding a lower-confidence note.
- If log_feedback.py has no existing transcript-parsing infrastructure to build on, add a single focused function that scans for common success markers ("test result: ok", "cargo build" with exit 0) and returns a boolean.
- If the change would require modifying the score_assessment function signature in a breaking way, add the new metric as an optional field that existing callers ignore.

Objective:
When the evaluator times out but the implementation transcript contains evidence of passing cargo build and cargo test, log_feedback.py should produce a distinct, less-severe score category so trajectory and future task selection can distinguish infrastructure timeouts from real implementation failures.

Why this matters:
The #1 cause of false task reverts in recent sessions is evaluator timeouts on correct implementations. When scoring treats infrastructure failure the same as implementation failure, the system loses the signal needed to retry good code vs. abandon bad code. A distinct "timeout but code passed" category lets the trajectory extractor and task picker make better decisions — e.g., re-attempting the same task with a longer evaluator timeout rather than picking a new task.

Success Criteria:
- A new function `_implementation_passed_build_and_test(transcript_text)` scans implementation transcript text for cargo build exit 0 and cargo test "test result: ok" markers
- When an evaluator timeout is detected AND implementation evidence is found, the feedback includes a new metric `evaluator_timeout_with_passing_impl_count` with lower severity weight (1.0) than a bare timeout (2.0)
- When no implementation evidence is found, existing evaluator timeout behavior is unchanged
- The new detection doesn't introduce false positives (e.g., cargo build output from a DIFFERENT command, or test failure output containing "ok" in a filename)

Verification:
- python3 -c "import sys; sys.path.insert(0, 'scripts'); import log_feedback; print('import OK')"
- python3 scripts/log_feedback.py --test

Expected Evidence:
- After landing, future log_feedback output shows `evaluator_timeout_with_passing_impl_count` when evaluator times out on passing code
- Trajectory scoring reflects the distinction, reducing false-negative pressure on tasks that were correct but timed out
- Over multiple sessions, the task picker can use this signal to retry tasks that timed out with evidence rather than abandoning them

Implementation Notes:
- log_feedback.py line 49-51 already defines EVALUATOR_TIMEOUT_RE and EVALUATOR_UNVERIFIED_RE
- The score_assessment function (around line 1770) counts evaluator_timeouts and evaluator_unverified via SCORE_FAILURE_WEIGHTS
- The parse_log function (around line 835-836) increments evaluator_timeouts when EVALUATOR_TIMEOUT_RE matches
- The task artifact loop (around line 1406) iterates task directories and reads eval_attempt_*.json files
- Add a helper function that takes transcript text (or log lines) and scans for build/test success markers
- The implementation transcript is available in the task transcript files — look for lines containing "cargo build" or "cargo test" followed by success indicators
- Success markers to detect:
  - "cargo build" line followed by no error output (exit code 0 implied by lack of "error:" lines within a few lines)
  - "cargo test" line followed by "test result: ok" within a few lines
  - "python3 scripts/preseed_session_plan.py --test" followed by "All tests passed" or no error output
- Add the new metric `evaluator_timeout_with_passing_impl_count` to GNOME_KEYS (after line 241, near existing evaluator_timeout_with_verdict_count) and SCORE_FAILURE_WEIGHTS (after line 296)
- Suggested weight: 1.0 (half of evaluator_timeout_with_verdict_count at 2.0) — reduced severity because the code was correct
- The detection should happen in the task artifact loop (around line 1406) where eval_attempt_*.json files are read and evaluator timeouts are already counted
- For each task where evaluator timeout occurred AND the implementation transcript shows build/test passing, increment the new counter
- Keep the change scoped to log_feedback.py only
