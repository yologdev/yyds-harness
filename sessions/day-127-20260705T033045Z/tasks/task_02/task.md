Title: Add held-out eval fixture for state event lifecycle pairing
Files: eval/fixtures/local-smoke/371-state-lifecycle-pairing.json
Issue: #37
Origin: planner

Evidence:
- `state lifecycle --limit 1000` reports: 182 runs started, 187 completed, 9 incomplete; 4 incomplete model calls; 3 unmatched completed model calls
- Issue #37 requests held-out eval coverage for "state event coverage for key lifecycle transitions"
- Day 126 Task 2 added fixture #370 (genome determinism); the lifecycle pairing gap remains untested
- The unmatched completed model calls (ModelCallCompleted without ModelCallStarted) suggest events can arrive out of order or be dropped — this is a class of bug that should have a held-out test
- Current eval fixtures don't include any lifecycle-transition-specific tests

Edit Surface:
- eval/fixtures/local-smoke/371-state-lifecycle-pairing.json — new fixture file

Verifier:
- yyds eval run --fixture 371-state-lifecycle-pairing

Fallback:
- If the eval infrastructure can't validate lifecycle pairing within the eval framework's capabilities, write the fixture as a documentation/spec artifact and mark the task done-with-findings with a note about what eval infrastructure would be needed.

Objective:
Create a held-out eval fixture that validates correct lifecycle event pairing — specifically that every RunStarted has a matching RunCompleted and every ModelCallStarted has a matching ModelCallCompleted — within a bounded event window.

Why this matters:
Lifecycle event pairing is foundational to state integrity. When runs or model calls complete without a start event (or start without completing), every downstream diagnostic — trajectory analysis, dashboard metrics, failure classification — operates on incomplete data. The current 9-incomplete-runs and 3-unmatched-model-calls suggest this is not a theoretical concern; it's a live gap.

A held-out eval fixture makes this measurable and prevents regressions. When future sessions modify the state recording pipeline, this fixture will catch lifecycle pairing breaks before they become invisible failures.

Success Criteria:
- A new eval fixture exists at `eval/fixtures/local-smoke/371-state-lifecycle-pairing.json`
- The fixture asserts that within a representative event sample, RunStarted events are paired with RunCompleted events (same run_id) and ModelCallStarted events are paired with ModelCallCompleted events (same call_id)
- The fixture passes when run against known-good state data
- The fixture uses the existing eval format (follow the pattern of fixtures 369 and 370)

Verification:
- yyds eval run --fixture 371-state-lifecycle-pairing

Expected Evidence:
- A new held-out eval fixture covering lifecycle transitions
- The eval suite has one more guard against state recording regressions
- Issue #37 has one more gap closed

Implementation Notes:
- Follow the existing eval fixture format. Fixtures 369 (prompt-layout-determinism) and 370 (genome-determinism) are the closest examples — both were added in the Day 125-126 window.
- The fixture should describe: what event types to scan, what pairing assertions to make (RunStarted↔RunCompleted, ModelCallStarted↔ModelCallCompleted), and how to report mismatches.
- The eval framework may work by describing expected state invariants rather than executing code. If so, the fixture should name the invariant clearly: "Every RunStarted event with run_id X must have a corresponding RunCompleted event with the same run_id."
- If the eval infrastructure requires executable checks (bash commands), use `yyds state lifecycle --limit 1000` and grep for incomplete counts.
- The fixture number 371 is the next available after 370.
