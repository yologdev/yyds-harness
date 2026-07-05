Title: Add held-out eval fixture for state event lifecycle pairing (bounded scope)
Files: eval/fixtures/local-smoke/371-state-lifecycle-pairing.json
Issue: #67, #37
Origin: planner

Evidence:
- Day 127 Task 2 attempted this fixture but was auto-reverted after the evaluator timed
  out. The previous fixture used `yyds state lifecycle --limit 1000` which was too slow.
- `yyds state lifecycle --limit 100` (assessed in Phase A1) reports: 182 runs started,
  187 completed, 9 incomplete; 4 incomplete model calls; 3 unmatched completed calls.
  These gaps are real and need a held-out test.
- Issue #37 tracks held-out eval coverage for "state event coverage for key lifecycle
  transitions" — this fixture closes that specific gap.
- Issue #67 tracks the reverted attempt.
- Fixtures #369 (prompt-layout-determinism) and #370 (genome-determinism) established
  the pattern for DeepSeek harness eval fixtures. This follows the same format.
- The eval infrastructure now has per-command timeouts (Task 1 of this session), so
  even if the command takes longer than expected, it won't hang the evaluator.

Edit Surface:
- eval/fixtures/local-smoke/371-state-lifecycle-pairing.json — new fixture file

Verifier:
- cargo run -- eval fixtures run --suite local-smoke --task 371-state-lifecycle-pairing

Fallback:
- If `yyds state lifecycle --limit 500` still takes too long even after the Task 1
  timeout fix (unlikely at 500 events), reduce to --limit 200 and mark the task
  done-with-findings explaining the constraint.
- If the eval framework can't express lifecycle-pairing assertions as executable
  commands, write the fixture as a documentation/spec artifact with a "not yet
  executable" note.

Objective:
Create a held-out eval fixture that validates correct lifecycle event pairing —
RunStarted↔RunCompleted and ModelCallStarted↔ModelCallCompleted — within a bounded
event window of 500 events.

Why this matters:
Lifecycle event pairing is foundational to state integrity. When runs or model calls
complete without a start event (or start without completing), every downstream
diagnostic — trajectory analysis, dashboard metrics, failure classification — operates
on incomplete data. The current 9-incomplete-runs and 3-unmatched-model-calls suggest
this is not a theoretical concern but a live gap.

A held-out eval fixture makes this measurable and prevents regressions. When future
sessions modify the state recording pipeline, this fixture catches lifecycle pairing
breaks before they become invisible failures.

This closes one gap from issue #37 ("state event coverage for key lifecycle
transitions") and redeems the reverted Day 127 Task 2 attempt.

Success Criteria:
- A new fixture file exists at eval/fixtures/local-smoke/371-state-lifecycle-pairing.json
- The fixture has a `tests` array with commands that check lifecycle pairing
- The fixture uses `--limit 500` (not 1000) to stay within the timeout budget
- The fixture follows the format of fixtures 369 and 370
- The fixture passes `cargo run -- eval fixtures run --task 371-state-lifecycle-pairing`
  when run against known-good state data (this session's events)

Verification:
- cargo run -- eval fixtures run --suite local-smoke --task 371-state-lifecycle-pairing
- Verify the fixture reports a clear pass/fail verdict

Expected Evidence:
- A new held-out eval fixture covering lifecycle transitions
- The eval suite has one more guard against state recording regressions
- Issue #37 has "state event coverage for key lifecycle transitions" checked off
- Issue #67 (reverted) is resolved

Implementation Notes:
- Follow the exact JSON format of fixtures 369 and 370. The structure is:
  {
    "task_id": "state-lifecycle-pairing",
    "category": "deepseek/state integrity",
    "repo_fixture": "self",
    "initial_commit": "HEAD",
    "risk_label": "high",
    "goal": "...",
    "tests": ["..."],
    "expected_files": ["eval/fixtures/local-smoke/371-state-lifecycle-pairing.json"],
    "hidden_failure_mode": "..."
  }

- The `tests` array should contain bash commands that verify lifecycle pairing.
  Recommended approach: use `yyds state lifecycle --limit 500` and pipe through
  grep/awk to check that incomplete counts are zero (or at least not growing).

- Example test command:
  `yyds state lifecycle --limit 500 2>&1 | grep -E 'incomplete runs: 0|incomplete model calls: 0'`

  Or more robustly, parse the output and assert:
  ```
  output=$(cargo run -- state lifecycle --limit 500 2>&1)
  incomplete_runs=$(echo "$output" | grep -oP '\d+(?= incomplete runs?)' || echo 0)
  incomplete_calls=$(echo "$output" | grep -oP '\d+(?= incomplete model calls?)' || echo 0)
  [ "$incomplete_runs" -eq 0 ] && [ "$incomplete_calls" -eq 0 ]
  ```

- The `timeout_secs` field (added by Task 1) can be set to 60 to be explicit about the
  budget. If Task 1 hasn't landed yet, the fixture should still complete within the
  default 120s timeout — 500 events is a small enough window.

- The `goal` should state: "Verify that within a bounded event window, every RunStarted
  has a matching RunCompleted and every ModelCallStarted has a matching
  ModelCallCompleted — no orphaned starts, no unmatched completions."

- The `hidden_failure_mode` should describe what silently breaks: "A change to the state
  recording pipeline introduces event ordering bugs (completions arriving before starts)
  or dropped events, causing downstream diagnostics to operate on incomplete lifecycle
  data without any visible error."
