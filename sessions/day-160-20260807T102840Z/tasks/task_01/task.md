Title: Prevent retroactive FailureObserved for deliberate no-op sessions
Files: scripts/append_terminal_state_events.py, scripts/test_append_terminal_state_events.py
Issue: #165
Origin: planner (refined from harness seed + issue #165 evidence)

Evidence:
- Day 160 assessment §Bugs/Friction #1: "[LOW] Retroactive FailureObserved for deliberate no-op sessions — The Day 159 12:05 journal-only session got a retroactive FailureObserved because RunCompleted status=error."
- `yyds state why last-failure` output confirmed: "Found a retroactive FailureObserved from Day 159 (12:05 session), source=unknown, class=unknown"
- Day 160 assessment §Capability Gaps #3: "Retroactive FailureObserved pollution: Deliberate no-op sessions (clean tree, journal-only) get retroactive FailureObserved events because RunCompleted has `status=error` (exit code 1 from no tasks completed). This inflates failure metrics."
- Day 159 had a journal-only session (12:05) that deliberately produced no tasks — `tasks_attempted==0`, clean tree. The harness's `append_terminal_state_events.py` retroactively inserted FailureObserved, polluting failure metrics.
- `find_missing_failure_observed()` in scripts/append_terminal_state_events.py already excludes input-validation and cancelled runs. Deliberate no-op runs (0 TaskStarted events, no tool errors) need the same exclusion.
- Issue #165 was reverted due to evaluator timeout, not code error. The fix is correct — just needs to pass the evaluator this time.

Edit Surface:
- scripts/append_terminal_state_events.py
- scripts/test_append_terminal_state_events.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events

Fallback:
- If the latest state snapshot (from `yyds state tail --limit 100`) shows zero retroactive FailureObserved events from the last 3 sessions, OR if no sessions in the last 5 have both RunCompleted error status AND zero tasks attempted, mark this task obsolete — the problem self-resolved.
- If the fix would require reading full transcripts or external session data (not available in the events stream), narrow scope to only the event-level check and skip deeper session analysis.

Objective:
Prevent `find_missing_failure_observed()` from flagging deliberate no-op sessions (planning-only or journal-only sessions that produced zero tasks with zero errors) as needing retroactive FailureObserved events.

Why this matters:
Deliberate no-op sessions (journal-only, assessment-only, clean-tree planning failure) exit with non-zero status because no tasks were completed — but this is expected behavior, not a harness failure. Retroactively inserting FailureObserved into these sessions inflates failure counts, pollutes the state event ledger with misleading records, and trains feedback systems to treat harmless sessions as problems. The Day 159 build_evolution_dashboard fix already stopped penalizing these in success-rate scoring, but the state events themselves still carry the false FailureObserved records.

Success Criteria:
- `find_missing_failure_observed()` does NOT return runs that have zero `TaskStarted` events AND zero non-harness `FailureObserved` events — these are deliberate no-ops, not real failures.
- Existing exclusions (input-validation, cancelled) continue to work.
- Existing tests pass. New test case covers the deliberate-no-op scenario.
- A `deliberate_no_op_excluded` counter appears in the diagnostics dict.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events

Expected Evidence:
- Next time a journal-only or planning-only session exits without tasks, no retroactive FailureObserved is inserted.
- `failure_observed_diagnostics` in append_terminal_state_events output shows a new `deliberate_no_op_excluded` count.
- Future state snapshots show fewer misleading failure events for deliberate-no-op sessions.

Implementation Notes:
- The fix lives in `find_missing_failure_observed()` (~line 307). The function iterates over events to find runs with RunCompleted error status but no FailureObserved.
- Add a tracking set `runs_with_task_started` — runs that have at least one `TaskStarted` event.
- Add a tracking set `runs_with_tool_failure` — runs that have at least one `FailureObserved` event with a tool-call actor or `source=tool` (not harness/unknown).
- Before adding a run to `missing` (the return set), check: does it have zero TaskStarted events AND zero tool failures? If so, skip it — it's a deliberate no-op.
- Keep the existing exclusions for input-validation and cancelled runs intact (they're checked separately).
- Add `deliberate_no_op_excluded` counter to the `failure_observed_diagnostics` dict.
- Add a test case: a run with RunCompleted error status, zero TaskStarted events, zero FailureObserved events, not input-validation, not cancelled. Assert it is NOT in the returned `missing` set.
- Keep the change minimal: one new tracking set, one new exclusion condition, one new diagnostics counter, one new test.
- Do NOT try to read transcripts, session files, or external state to determine no-op status — rely only on events present in the events list.
