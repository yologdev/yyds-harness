Title: Don't penalize recovered tool failures in session scoring
Files: scripts/log_feedback.py
Issue: none
Origin: planner

Evidence:
- Trajectory graph-derived pressure: "Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dominant failure class and add prompt/tool guards for the failure class."
- Trajectory corrected top lessons: "failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class"
- Current scoring in score_assessment() (line 1735) uses SCORE_FAILURE_WEIGHTS["tool_error_count"]=1.0 in failure_pressure calculation, but does not check whether tool errors were later recovered by retry.
- The build_assessment() function (line 1755) computes tool_error_count from session evidence but doesn't cross-reference transcript recovery evidence.
- A session where the agent hit one tool error, retried, and succeeded gets the same tool_error_count=1 penalty as a session where the error was unrecovered — this is unfair scoring.

Edit Surface:
- scripts/log_feedback.py

Verifier:
- python3 scripts/log_feedback.py (self-tests must pass)
- Manual: check that score_assessment with tool_error_count=1 but recovered=true gives a higher (better) score than unrecovered

Fallback:
- If the tool_error_count scoring path has already been refactored to check recovery, or if build_assessment already cross-references transcript evidence for tool errors, mark this task obsolete.
- If the recovery signal isn't available in the data that build_assessment receives, mark this task blocked and write why in a planning_failure.md note.

Objective:
When a tool error was recovered (the agent retried and the tool succeeded in a later attempt), don't count it against the session score — or at minimum, weight it lower than truly unrecovered errors.

Why this matters:
Fair scoring is essential for evolution reliability. When yyds hits a tool error, retries with a recovery hint, and succeeds, that's good behavior — not a failure. Penalizing recovered errors the same as unrecovered ones creates noise in the scoring signal and makes it harder for the harness to distinguish healthy sessions from broken ones. This directly addresses the graph-derived pressure to "recover failed tool actions before scoring."

Success Criteria:
- Sessions where tool errors were recovered in subsequent attempts score higher than identical sessions where the same errors were unrecovered.
- The self-tests in log_feedback.py continue to pass.
- Existing scoring behavior for sessions with no tool errors is unchanged.

Verification:
- python3 scripts/log_feedback.py (runs self-tests)
- Spot-check: run against a recent session that had tool_error_count=1 and verify the adjusted score.

Expected Evidence:
- Future trajectory reports should show tool_error_count remaining at 0 for sessions where all tool errors were recovered.
- The corrected top lessons should stop flagging "recover failed tool actions before scoring" as an active pressure item.
- Score fairness improves: sessions that recover from errors aren't misclassified as lower-quality.

Implementation Notes:
- Focus on the build_assessment() function (line 1755) and its callers. This is where tool_error_count is computed.
- The simplest approach: when tool_error_count > 0, scan the session transcript files for evidence that the errored tool succeeded in a later attempt within the same task. If evidence of recovery exists, either:
  a) Subtract recovered errors from tool_error_count before scoring, OR
  b) Add a separate "recovered_tool_error_count" metric and adjust SCORE_FAILURE_WEIGHTS.
- Approach (a) is simpler and preferred.
- Use the existing transcript-parsing infrastructure already in log_feedback.py (TRANSCRIPT_ACTION_RE, clean_transcript_action, etc.).
- If transcript files aren't available (log_available=False), keep the unrecovered tool_error_count — don't assume recovery without evidence.
- Update the self-test metrics dict (around line 2559) to include a test case with recovered tool errors.
