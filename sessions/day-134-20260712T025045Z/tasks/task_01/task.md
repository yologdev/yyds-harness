Title: Investigate and reduce the 35 unmatched lifecycle completions
Files: scripts/append_terminal_state_events.py, scripts/build_evolution_dashboard.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Trajectory Day 134: `state_run_unmatched_non_validation_completed_count=35` — top graph-pressure item.
  Causes: `state_unmatched/open_after_FailureObserved=7`, remainder untagged.
- Dashboard at scripts/build_evolution_dashboard.py:2435 computes this as
  `len(run_unmatched_non_validation_completed)` — runs with Completion+error events
  but no matching Start events, excluding input-validation completions.
- Day 132 (10:55 session) had a similar dashboard field-name bug where
  `unmatched_completed_details` was wired instead of `unmatched_non_validation_completed_details`.
  That was fixed. The 35 remaining appear to be genuine lifecycle gaps, not a
  wiring bug — the computation at lines 2395-2440 looks correct.
- Day 130 partially addressed the "incomplete" side; the "unmatched completed"
  side still has 35 remaining. Some may be from the current active session
  (normal — runs haven't finished yet so their Start events haven't been paired).
- Assessment: "35 model calls completed without matching start events" — this
  may be mislabeled by the trajectory extractor. The actual metric is about
  **runs**, not model calls.

Edit Surface:
- scripts/append_terminal_state_events.py — may need to handle additional edge cases
  for runs that completed but lack Start events (currently handles FailureObserved
  for error-status runs, but may miss runs that completed without any start record).
- scripts/build_evolution_dashboard.py — may need to exclude runs from the current
  active session (which legitimately haven't finished yet) from the unmatched count.

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events -q 2>&1
- python3 scripts/build_evolution_dashboard.py --help 2>&1 | head -5

Fallback:
- If `yyds state lifecycle --limit 200000` shows the 35 are all from the current
  session or are all input-validation runs misclassified, note this as a
  dashboard filter bug and fix the filter. Do not modify event files.
- If the investigation reveals a deeper problem requiring src/ changes, write
  findings to session_plan/task_01_findings.md and stop — do not exceed scope.

Objective:
Reduce the `state_run_unmatched_non_validation_completed_count` by identifying
and fixing the root cause of the 35 unmatched runs. If they are current-session
artifacts, filter them from the count. If they are genuine orphaned runs, fix
the terminal event cleanup to close them.

Why this matters:
This is the #1 graph-pressure item. The unmatched lifecycle count feeds into
evo-readiness assessment — when it's high, it creates noise that can mask real
problems. Every session adds more runs, and if the cleanup isn't keeping up,
the number compounds. A session that can't trust its lifecycle metrics can't
trust its own health assessment.

Success Criteria:
- `state_run_unmatched_non_validation_completed_count` drops by at least 5
  (either by fixing genuine gaps or by filtering active-session noise).
- The fix is in the listed files only — no src/ changes needed.
- `python3 -m unittest scripts.test_append_terminal_state_events` still passes.
- The fix survives: running the same computation twice produces the same (lower) count.

Verification:
- Run: `yyds state lifecycle --limit 200000 2>&1 | python3 -c "import sys,json; d=json.load(sys.stdin); runs=d.get('runs',{}); print(f'unmatched_non_validation_completed={runs.get(\"unmatched_non_validation_completed\",\"?\")}')"`
- Confirm the count is lower than 35.
- Run: `python3 scripts/build_evolution_dashboard.py 2>&1 | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('state_lifecycle',{}).get('runs',{}).get('unmatched_non_validation_completed','?'))"` 

Expected Evidence:
- Next trajectory shows `state_run_unmatched_non_validation_completed_count` < 35.
- If the fix filtered active-session runs, the count should drop by the number
  of runs in the current session.
- If the fix closed genuine orphaned runs via terminal event cleanup, the event
  file should have new FailureObserved events for those runs.

Implementation Notes:
1. First diagnostic: run `yyds state lifecycle --limit 200000` and inspect the
   `runs.unmatched_non_validation_completed_details` list. Look at run_ids and
   last_event types.
2. Check if any run_ids match the current session (newer than 1 hour). If so,
   those are active-session runs — they should be excluded from the count in
   the dashboard at line 2435. Add a filter: skip runs whose run_id starts with
   the current session's timestamp prefix or whose session_started flag is true.
3. For runs that are genuinely orphaned (old, no start event, completed with
   error): `append_terminal_state_events.py` should catch and close them.
   Check if there's a filtering gap — e.g., it only scans recent events or
   requires specific event patterns that these runs don't match.
4. Keep changes small. A one-line filter addition in the dashboard is preferred
   over a complex rewrite of the terminal event script.
