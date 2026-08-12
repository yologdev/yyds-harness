Title: Add source/class fields to retroactive FailureObserved events in append_terminal_state_events.py
Files: scripts/append_terminal_state_events.py
Issue: #179
Origin: planner (corrected edit surface from blocked-task evidence)

Evidence:
- `yyds state why last-failure` shows retroactive FailureObserved from day-164 16:57 session with source=unknown, class=unknown.
- The retroactive FailureObserved payload at lines 751-754 of `scripts/append_terminal_state_events.py` sets only `reason` and `retroactive` — no `source` or `class` field.
- The `status` variable (from the RunCompleted event, e.g. "reverted", "timeout", "cancelled") is available at line 743 but is NOT used to populate classification fields.
- `src/commands_state.rs` line ~2038 reads `source` from `payload["source"]` or `payload["operation"]`, falls back to "-". Since the Python script sets neither field, retroactive FailureObserved shows `source="-"`.
- Previous attempt (#179 Day 165 Task 1) was reverted because the task assigned `src/state.rs` as edit surface — the actual fix is in `scripts/append_terminal_state_events.py`, not `src/state.rs`. The blocked-task evidence already corrected this.
- Trajectory graph pressure #5: "Close yyds state and model lifecycle gaps" — retroactive FailureObserved with unknown source feeds false lifecycle pressure.

Edit Surface:
- scripts/append_terminal_state_events.py

Verifier:
- python3 -c "import ast; ast.parse(open('scripts/append_terminal_state_events.py').read()); print('syntax ok')"
- python3 scripts/append_terminal_state_events.py --help 2>&1 | head -5

Fallback:
- If the `status` variable in `find_missing_failure_observed` always arrives as empty string or None, the mapping produces no source/class — still better than before (the fields exist but are empty), but check whether the RunCompleted events actually carry meaningful status values before claiming success.
- If `scripts/append_terminal_state_events.py` has been significantly refactored since the blocked-task evidence was written (lines 738-758 don't match), narrow scope to finding the new location of `payload_fo` and applying the same mapping there.

Objective:
Make retroactive FailureObserved events carry a meaningful `source` and `class` derived from the RunCompleted event's `status`, so the state doctor's failure detection produces actionable classification instead of "unknown/unknown."

Why this matters:
The trajectory shows `task_unlanded_source_count=1` and `reverted_unlanded_source_edits` across multiple sessions. When the harness retroactively classifies a failure as "unknown," it can't distinguish infrastructure timeouts from real code problems from cancelled CI runs. This feeds false pressure into the trajectory graph and wastes task slots on misdiagnosed failures. A ~15 line change that maps RunCompleted status to FailureObserved source/class makes the failure signal actionable.

Success Criteria:
- When the state doctor retroactively adds a FailureObserved for a RunCompleted with status containing a recognizable keyword ("reverted", "timeout", "cancelled", "build_failure", "test_failure"), the FailureObserved's `source` and `class` fields reflect that classification.
- Unknown statuses still produce no source/class (backward compatible — the renderer's fallback to "-" still works).
- The change is under 20 lines.

Verification:
- python3 -c "import ast; ast.parse(open('scripts/append_terminal_state_events.py').read()); print('syntax ok')"
- python3 scripts/append_terminal_state_events.py --help 2>&1 | head -5
- grep -n 'source\|class' scripts/append_terminal_state_events.py | grep -i 'failure\|fo\|observed' — verify the new fields exist near the payload_fo construction

Expected Evidence:
- Next `yyds state why last-failure` shows source="task_revert" or similar non-unknown value for retroactive FailureObserved events.
- `task_unlanded_source_count` gnome becomes more precise (distinguishes reverted from timeout from cancelled).
- Lifecycle graph pressure stops inflating from unknown-source FailureObserved events.

Implementation Notes:
- The fix is in `scripts/append_terminal_state_events.py`, in the loop that creates retroactive FailureObserved events (~line 740-758).
- The `status` variable from the RunCompleted event is already available (line 743: `status = entry.get("status", "")`).
- Add this mapping before the `payload_fo` dict construction (~line 751):

```python
_status_source_map = {
    "reverted": ("task_revert", "verification"),
    "timeout": ("timeout", "infrastructure"),
    "cancelled": ("cancelled", "external"),
    "build_failure": ("build", "code"),
    "test_failure": ("test", "code"),
}
_source_class = _status_source_map.get(status.lower(), (None, None))
```

- Add to `payload_fo` dict (lines 751-754):

```python
if _source_class[0]:
    payload_fo["source"] = _source_class[0]
if _source_class[1]:
    payload_fo["class"] = _source_class[1]
```

- Do NOT modify any other function or file. Keep the change surgical.
- The `src/commands_state.rs` renderer already handles `source` and `class` fields when present — no changes needed there.
- Total change: ~10 lines inserted near line 750, before the payload_fo construction.
