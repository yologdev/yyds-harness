Title: Prevent retroactive FailureObserved for deliberate no-op sessions
Files: scripts/append_terminal_state_events.py
Issue: #164
Origin: planner (refined from harness seed)

Evidence:
- Day 160 assessment §Last Failure Analysis: "The 'last-failure' is a retroactive FailureObserved from the Day 159 12:05 journal-only session — the harness inserted it because RunCompleted had error status but no FailureObserved was recorded during the session."
- `yyds state why last-failure` output confirmed: "Found a retroactive FailureObserved from Day 159 (12:05 session), source=unknown, class=unknown"
- Day 160 assessment §Bugs/Friction #2: "[LOW] Retroactive FailureObserved for deliberate no-op sessions — The Day 159 12:05 journal-only session got a retroactive FailureObserved because RunCompleted status=error."
- Day 159 had a journal-only session (12:05) that deliberately produced no tasks — `tasks_attempted==0`, clean tree. The harness's `append_terminal_state_events.py` retroactively inserted FailureObserved, polluting failure metrics.
- `find_missing_failure_observed()` in scripts/append_terminal_state_events.py already excludes input-validation and cancelled runs (lines 341-346). Deliberate no-op runs (0 TaskStarted events, no tool errors) need the same exclusion.

Edit Surface:
- scripts/append_terminal_state_events.py

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
- `find_missing_failure_observed()` does NOT return runs that have zero `TaskStarted` events and zero tool-call `FailureObserved` events — these are deliberate no-ops, not real failures.
- Existing exclusions (input-validation, cancelled) continue to work.
- Existing tests pass. New test case covers the deliberate-no-op scenario.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events
- After the fix, re-running `yyds state why last-failure` on a fresh state rebuild should no longer show deliberate-no-op sessions as failures.

Expected Evidence:
- Next time a journal-only or planning-only session exits without tasks, no retroactive FailureObserved is inserted.
- `failure_observed_diagnostics` in append_terminal_state_events output shows a new `deliberate_no_op_excluded` count.
- Future state snapshots show fewer misleading failure events for deliberate-no-op sessions.

Implementation Notes:
- The fix lives in `find_missing_failure_observed()` (line 307). Add a tracking set for runs that have `TaskStarted` events and another for runs that have `FailureObserved` with tool-call lineage (i.e., non-harness FailureObserved).
- A deliberate no-op run has ALL of: RunCompleted error status, zero TaskStarted events, zero non-harness FailureObserved events, AND is not input-validation/cancelled (already excluded).
- Add a `deliberate_no_op_excluded` counter to the `diagnostics` dict.
- The existing test data may not have deliberate-no-op runs — add a test fixture with RunCompleted error status + no TaskStarted + no tool failures, and verify the run is NOT in `missing`.
- Keep the change minimal: new tracking sets in the event loop, one new exclusion condition before adding to `missing`, one new diagnostics counter.
- Do NOT try to read transcripts, session files, or external state to determine no-op status — rely only on events present in the events list.
